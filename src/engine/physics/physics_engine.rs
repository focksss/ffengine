use std::cell::RefCell;
use std::ops::Deref;
use std::sync::Arc;
use crate::engine::physics::physics_engine::Hitbox::OBB;
use crate::engine::physics::player::Player;
use crate::engine::world::scene::{Mesh, Node, Scene};
use crate::math::*;
use crate::math::matrix::Matrix;

pub struct PhysicsEngine {
    pub gravity: Vector,

    pub rigid_bodies: Vec<RigidBody>,
    pub players: Vec<Arc<RefCell<Player>>>
}
impl PhysicsEngine {
    pub fn new(world: &Scene, gravity: Vector) -> Self {
        let mut rigid_bodies = Vec::new();
        for model in &world.models {
            for node in &model.nodes {
                if let Some(mesh) = &node.mesh {
                    let (min, max) = mesh.borrow().get_min_max();
                    let obb = Obb {
                        center: (min + max) * 0.5,
                        half_extents: (max - min) * 0.5 * node.scale,
                        orientation: node.world_transform.extract_quaternion(),
                    };
                    rigid_bodies.push(RigidBody::new_from_node(node, OBB(obb)));
                }
            }
        }
        Self {
            gravity,
            rigid_bodies,
            players: Vec::new(),
        }
    }
    pub fn add_player(&mut self, player: Arc<RefCell<Player>>) {
        self.players.push(player);
    }

    pub fn tick(&mut self, delta_time: f32) {
        for body in &mut self.rigid_bodies {
            if !body.is_static {
                body.velocity = body.velocity + self.gravity * delta_time;
            }
        }
        for player_ref in &self.players {
            let mut player = player_ref.borrow_mut();
            player.rigid_body.position = player.camera.position;

            player.rigid_body.velocity = player.rigid_body.velocity + self.gravity * delta_time;

            let velocity_step = player.rigid_body.velocity * delta_time;

            self.move_player_with_collision(&mut player, velocity_step);
        }
        for body in &mut self.rigid_bodies {
            if !body.is_static {
                let displacement = body.velocity * delta_time;
                body.position = body.position + displacement;

                if body.angular_velocity.magnitude_3d() > 1e-6 {
                    let angle = body.angular_velocity.magnitude_3d() * delta_time;
                    let axis = body.angular_velocity.normalize_3d();
                    let rotation_quat = PhysicsEngine::axis_angle_to_quat(&axis, angle);
                    body.orientation = body.orientation.combine(&rotation_quat).normalize_4d();
                }
            }
        }
    }
    fn move_player_with_collision(&self, player: &mut Player, intended_step: Vector) {
        const MAX_ITERATIONS: usize = 4;
        const COLLISION_EPSILON: f32 = 0.001;

        let mut remaining_step = intended_step;
        let mut iteration = 0;

        while iteration < MAX_ITERATIONS && remaining_step.magnitude_3d() > COLLISION_EPSILON {
            let test_position = player.rigid_body.position + remaining_step;
            player.rigid_body.position = test_position;

            let mut closest_collision: Option<(ContactInformation, usize)> = None;
            let mut min_distance = f32::MAX;
            for (i, static_body) in self.rigid_bodies.iter().enumerate() {
                if !static_body.is_static {
                    continue;
                }
                if let Some(contact) = player.rigid_body.colliding_with_info(static_body) {
                    let distance = contact.penetration_depth;
                    if distance < min_distance {
                        min_distance = distance;
                        closest_collision = Some((contact, i));
                    }
                }
            }
            if let Some((contact, _body_index)) = closest_collision {
                let separation = contact.normal * (contact.penetration_depth + COLLISION_EPSILON);
                let safe_position = test_position + separation;

                let normal_component = remaining_step.dot3(&contact.normal);
                let tangent_step = remaining_step - contact.normal * normal_component;

                let friction = player.rigid_body.friction_coefficient;
                let adjusted_tangent = tangent_step * (1.0 - friction);

                player.step(safe_position - player.camera.position);
                let velocity_normal = player.rigid_body.velocity.dot3(&contact.normal);
                if velocity_normal < 0.0 {
                    let restitution = player.rigid_body.restitution_coefficient;
                    player.rigid_body.velocity = player.rigid_body.velocity - contact.normal * (velocity_normal * (1.0 + restitution));
                    if contact.normal.y > 0.7 {
                        player.rigid_body.velocity.y = 0.0;
                    }
                }
                remaining_step = adjusted_tangent;
            } else {
                player.step(remaining_step);
                break;
            }
            iteration += 1;
        }
        player.rigid_body.position = player.camera.position;
    }
    fn axis_angle_to_quat(axis: &Vector, angle: f32) -> Vector {
        let half_angle = angle * 0.5;
        let s = half_angle.sin();
        Vector::new_vec4(
            axis.x * s,
            axis.y * s,
            axis.z * s,
            half_angle.cos()
        )
    }
}

pub struct RigidBody {
    pub hitbox: Hitbox,
    pub is_static: bool,
    pub restitution_coefficient: f32,
    pub friction_coefficient: f32,
    pub mass: f32,
    pub inv_mass: f32,

    pub force: Vector,
    pub torque: Vector,

    pub position: Vector,
    pub velocity: Vector,
    pub orientation: Vector, // quaternion
    pub angular_velocity: Vector,

    pub inertia_tensor: Matrix, // 3x3
    pub inv_inertia_tensor: Matrix,
}

impl RigidBody {
    //TODO
    // * SHOULD CONSTRUCT MIN AND MAX FROM THE WORLD TRANSFORM, OR MAYBE COPY WHAT THE FRUSTUM CULLING FUNCTION DOES
    pub fn new_from_node(node: &Node, hitbox: Hitbox) -> Self {
        let mut body = RigidBody::default();
        body.position = node.world_transform * Vector::new_vec4(0.0, 0.0, 0.0, 1.0);
        body.hitbox = hitbox;
        body
    }

    pub fn colliding_with_info(&self, other: &RigidBody) -> Option<ContactInformation> {
        match (&self.hitbox, &other.hitbox) {
            (OBB(_), OBB(_)) => {
                self.obb_intersects_obb(other)
            }
        }
    }

    pub fn get_min_max(&self) -> (Vector, Vector) {
        // return min and max of hitbox in world space for debug/drawing purposes
        match &self.hitbox {
            OBB(obb) => {
                let min = (obb.center - obb.half_extents).rotate_by_quat(&obb.orientation) + self.position;
                let max = (obb.center + obb.half_extents).rotate_by_quat(&obb.orientation) + self.position;

                let corners = [
                    Vector::new_vec3(min.x, min.y, min.z),
                    Vector::new_vec3(min.x, max.y, min.z),
                    Vector::new_vec3(max.x, min.y, min.z),
                    Vector::new_vec3(max.x, max.y, min.z),
                    Vector::new_vec3(min.x, min.y, max.z),
                    Vector::new_vec3(min.x, max.y, max.z),
                    Vector::new_vec3(max.x, min.y, max.z),
                    Vector::new_vec3(max.x, max.y, max.z),
                ];
                let mut min = corners[0];
                let mut max = corners[0];
                for corner in &corners {
                    min = Vector::min(&min, corner);
                    max = Vector::max(&max, corner);
                }

                (min, max)
            }
            _ => (Vector::new_vec(0.0), Vector::new_vec(0.0))
        }
    }

    pub fn obb_intersects_obb(&self, other: &RigidBody) -> Option<ContactInformation> {
        let (a, b) = match (&self.hitbox, &other.hitbox) {
            (OBB(a), OBB(b)) => (a, b),
            _ => return None,
        };

        // OBB centers in world space
        let a_center = a.center.rotate_by_quat(&a.orientation) + self.position;
        let b_center = b.center.rotate_by_quat(&b.orientation) + other.position;

        // net quaternions
        let a_quat = a.orientation;
        let b_quat = b.orientation;

        // local axes
        let a_axes = [
            Vector::new_vec3(1.0, 0.0, 0.0).rotate_by_quat(&a_quat),
            Vector::new_vec3(0.0, 1.0, 0.0).rotate_by_quat(&a_quat),
            Vector::new_vec3(0.0, 0.0, 1.0).rotate_by_quat(&a_quat),
        ];
        let b_axes = [
            Vector::new_vec3(1.0, 0.0, 0.0).rotate_by_quat(&b_quat),
            Vector::new_vec3(0.0, 1.0, 0.0).rotate_by_quat(&b_quat),
            Vector::new_vec3(0.0, 0.0, 1.0).rotate_by_quat(&b_quat),
        ];

        let t = b_center - a_center;

        let mut min_penetration = f32::MAX;
        let mut collision_normal = Vector::new_empty();
        for i in 0..3 {
            { // a_normals
                let axis = a_axes[i];

                let penetration = RigidBody::test_axis(&axis, &t, &a.half_extents, &b.half_extents, &a_axes, &b_axes, true);
                if penetration < 0.0 {
                    return None
                }
                if penetration < min_penetration {
                    min_penetration = penetration;
                    collision_normal = axis;
                }
            }
            { // b_normals
                let axis = b_axes[i];

                let penetration = RigidBody::test_axis(&axis, &t, &a.half_extents, &b.half_extents, &a_axes, &b_axes, true);
                if penetration < 0.0 {
                    return None
                }
                if penetration < min_penetration {
                    min_penetration = penetration;
                    collision_normal = axis;
                }
            }
            for j in 0..3 { // edge-edge cross-products
                let axis = a_axes[i].cross(&b_axes[j]);
                let axis_length = axis.magnitude_3d();

                // skip near-parallel
                if axis_length < 1e-6 {
                    continue;
                }

                let axis_normalized = axis / axis_length;
                let penetration = RigidBody::test_axis_cross(&axis_normalized, &t, &a.half_extents, &b.half_extents, &a_axes, &b_axes);

                if penetration < 0.0 {
                    return None;
                }

                if penetration < min_penetration {
                    min_penetration = penetration;
                    collision_normal = axis_normalized;
                }
            }
        }

        // bad normal?


        let contact_point = a_center + collision_normal * (min_penetration * 0.5);

        Some(ContactInformation {
            point: contact_point,
            normal: -1.0 * collision_normal,
            penetration_depth: min_penetration,
        })
    }
    fn test_axis(
        axis: &Vector,
        t: &Vector,
        half_a: &Vector,
        half_b: &Vector,
        axes_a: &[Vector; 3],
        axes_b: &[Vector; 3],
        is_a: bool,
    ) -> f32 {
        let ra = if is_a {
            half_a.x * axes_a[0].dot3(axis).abs() +
                half_a.y * axes_a[1].dot3(axis).abs() +
                half_a.z * axes_a[2].dot3(axis).abs()
        } else {
            half_a.x * axes_a[0].dot3(axis).abs() +
                half_a.y * axes_a[1].dot3(axis).abs() +
                half_a.z * axes_a[2].dot3(axis).abs()
        };
        let rb = half_b.x * axes_b[0].dot3(axis).abs() +
            half_b.y * axes_b[1].dot3(axis).abs() +
            half_b.z * axes_b[2].dot3(axis).abs();
        let distance = t.dot3(axis).abs();
        ra + rb - distance
    }
    fn test_axis_cross(
        axis: &Vector,
        t: &Vector,
        half_a: &Vector,
        half_b: &Vector,
        axes_a: &[Vector; 3],
        axes_b: &[Vector; 3],
    ) -> f32 {
        let ra = half_a.x * axes_a[0].dot3(axis).abs() +
            half_a.y * axes_a[1].dot3(axis).abs() +
            half_a.z * axes_a[2].dot3(axis).abs();

        let rb = half_b.x * axes_b[0].dot3(axis).abs() +
            half_b.y * axes_b[1].dot3(axis).abs() +
            half_b.z * axes_b[2].dot3(axis).abs();

        let distance = t.dot3(axis).abs();

        ra + rb - distance
    }
}
impl Default for RigidBody {
    fn default() -> Self {
        Self {
            hitbox: Hitbox::OBB(Obb {
                center: Vector::new_vec(0.0),
                half_extents: Vector::new_vec(1.0),
                orientation: Vector::new_vec4(0.0, 0.0, 0.0, 1.0),
            }),
            is_static: true,
            restitution_coefficient: 0.0,
            friction_coefficient: 0.0,
            mass: 0.0,
            inv_mass: 0.0,
            force: Default::default(),
            torque: Default::default(),
            position: Default::default(),
            velocity: Default::default(),
            orientation: Default::default(),
            angular_velocity: Default::default(),
            inertia_tensor: Matrix::new(),
            inv_inertia_tensor: Matrix::new(),
        }
    }
}

#[derive(Debug)]
pub struct ContactInformation {
    point: Vector,
    normal: Vector,
    penetration_depth: f32,
}

#[derive(Debug)]
pub enum Hitbox {
    OBB(Obb),
}
#[derive(Debug)]
pub struct Obb {
    pub center: Vector,
    pub half_extents: Vector,
    pub orientation: Vector, // quaternion
}