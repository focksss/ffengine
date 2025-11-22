use std::cell::RefCell;
use std::sync::Arc;
use crate::math::matrix::Matrix;
use crate::math::Vector;
use crate::physics::hitboxes::bounding_box::BoundingBox;
use crate::physics::hitboxes::capsule::Capsule;
use crate::physics::hitboxes::hitbox::Hitbox;
use crate::physics::hitboxes::mesh::{Bvh, MeshCollider};
use crate::physics::player::{MovementMode, Player};
pub(crate) use crate::physics::rigid_body::RigidBody;
use crate::world::scene::{Node, Scene, Vertex};

const MAX_ITERATIONS: usize = 5;
const MIN_MOVE_THRESHOLD: f32 = 0.001;

pub struct PhysicsEngine {
    pub gravity: Vector,
    pub air_resistance_coefficient: f32,
    pub player_horiz_const_resistance: f32,

    pub rigid_bodies: Vec<RigidBody>,
    pub players: Vec<Player>
}

impl PhysicsEngine {
    pub fn new(gravity: Vector, air_resistance_coefficient: f32, player_horiz_const_resistance: f32) -> Self {
        let mut rigid_bodies = Vec::new();
        Self {
            gravity,
            air_resistance_coefficient,
            player_horiz_const_resistance,
            rigid_bodies,
            players: Vec::new(),
        }
    }
    /// For hitbox type:
    /// * 0 = OBB
    /// * 1 = Mesh
    /// * 2 = Capsule
    /// * 3 = Sphere
    pub fn add_all_nodes_from_model(&mut self, world: &Scene, model_index: usize, hitbox_type: usize) {
        assert!(hitbox_type < 4);
        let model = &world.models[model_index];
        for (node_index, node) in model.nodes.iter().enumerate() {
            if let Some(hitbox) = Hitbox::get_hitbox_from_node(node, hitbox_type) {
                let mut rb = RigidBody::new_from_node(node, Some((model_index, node_index)), hitbox);
                rb.update_this_according_to_coupled(world);
                self.rigid_bodies.push(rb);
            }
        }
    }
    pub fn add_player(&mut self, player: Player) {
        self.players.push(player);
    }

    fn get_world_contacts(&self, body:& RigidBody) -> Option<Vec<ContactInformation>> {
        let mut collisions = Vec::new();
        for other_body in self.rigid_bodies.iter() {
            if let Some(new_collisions) = body.colliding_with(other_body) {
                collisions.push(new_collisions);
            }
        }
        if collisions.len() > 0 { return Some(collisions) }
        None
    }
    fn get_deepest_of_contacts(contacts: &Vec<ContactInformation>) -> &ContactInformation {
        let mut deepest = &contacts[0];
        for i in 1..contacts.len() {
            if contacts[i].penetration_depth > deepest.penetration_depth { deepest = &contacts[i]; }
        }
        deepest
    }

    fn body_cast(
        &self,
        body: &mut RigidBody,
        start_offset: &Vector,
        direction: &Vector,
        max_distance: f32
    ) -> Option<CastInformation> {
        let dir = direction.normalize_3d();
        let original_position = body.position;

        let start_pos = body.position + start_offset;
        body.position = start_pos;
        let original_contacts = self.get_world_contacts(&body);
        if let Some(collisions) = original_contacts {
            return Some(CastInformation {
                distance: 0.0,
                contacts: collisions,
            });
        }

        let mut t_min = 0.0;
        let mut t_max = max_distance;
        let epsilon = 0.001;

        while t_max - t_min > epsilon {
            let t_mid = (t_min + t_max) / 2.0;

            let test_offset = dir * t_mid;

            body.position = start_pos + test_offset;

            let mut hit = false;
            for other_body in self.rigid_bodies.iter() {
                if body.colliding_with(other_body).is_some() {
                    hit = true;
                    break;
                }
            }
            if hit {
                t_max = t_mid;
            } else {
                t_min = t_mid;
            }
        }

        let final_offset = dir * t_max;

        body.position = start_pos + final_offset;
        let final_contacts = self.get_world_contacts(&body);
        body.position = original_position;
        if let Some(collisions) = final_contacts {
            Some(CastInformation {
                distance: t_max,
                contacts: collisions,
            })
        } else {
            None
        }
    }


    pub fn tick(&mut self, delta_time: f32, world: &mut Scene) {
        // apply gravity
        for body in &mut self.rigid_bodies {
            if body.is_static {
                continue;
            } else {
                body.apply_impulse(self.gravity * delta_time * body.mass, body.get_center_of_mass_world_space());
            }
        }
        // collision
        for i in 0..self.rigid_bodies.len() {
            let (a, b) = self.rigid_bodies.split_at_mut(i + 1);
            let body_a = &mut a[i];
            for body_b in b {
                if body_a.is_static && body_b.is_static {
                    continue;
                }
                if let Some(collision) = body_a.colliding_with(body_b) {
                    if collision.contact_points.is_empty() { continue; }
                    let normal = collision.normal;
                    let depth = collision.penetration_depth;
                    let im_a = body_a.inv_mass; let im_b = body_b.inv_mass;
                    let s_im = im_a + im_b;
                    let restitution = body_a.restitution_coefficient * body_b.restitution_coefficient;
                    let inv_inertia_a = body_a.get_inverse_inertia_tensor_world_space();
                    let inv_inertia_b = body_b.get_inverse_inertia_tensor_world_space();

                    let pt_on_a = collision.contact_points[0].point_on_a;
                    let pt_on_b = collision.contact_points[0].point_on_b;
                    let ra = pt_on_a - body_a.get_center_of_mass_world_space();
                    let rb = pt_on_b - body_b.get_center_of_mass_world_space();

                    let angular_j_a = (inv_inertia_a * ra.cross(&normal)).cross(&ra);
                    let angular_j_b = (inv_inertia_b * rb.cross(&normal)).cross(&rb);
                    let angular_factor = (angular_j_a + angular_j_b).dot3(&normal);

                    let vel_a = body_a.velocity + body_a.angular_velocity.cross(&ra);
                    let vel_b = body_b.velocity + body_b.angular_velocity.cross(&rb);

                    let v_diff = vel_a - vel_b;

                    let j = normal * (1.0 + restitution) * v_diff.dot3(&normal) / (s_im + angular_factor);
                    body_a.apply_impulse(-j, pt_on_a);
                    body_b.apply_impulse(j, pt_on_b);

                    let friction = body_a.friction_coefficient * body_b.friction_coefficient;
                    let velocity_normal = normal * normal.dot3(&v_diff);
                    let velocity_tangent = v_diff - velocity_normal;

                    let relative_tangent_vel = velocity_tangent.normalize_3d();
                    let inertia_a = (inv_inertia_a * ra.cross(&relative_tangent_vel)).cross(&ra);
                    let inertia_b = (inv_inertia_b * rb.cross(&relative_tangent_vel)).cross(&rb);
                    let inv_inertia = (inertia_a + inertia_b).dot3(&relative_tangent_vel);

                    let mass_reduc = 1.0 / (s_im + inv_inertia);
                    let friction_impulse = velocity_tangent * mass_reduc * friction;

                    body_a.apply_impulse(-friction_impulse, pt_on_a);
                    body_b.apply_impulse(friction_impulse, pt_on_b);

                    let t_a = im_a / s_im;
                    let t_b = im_b / s_im;

                    let ds = normal * depth;

                    body_a.position -= ds * t_a;
                    body_b.position += ds * t_b;
                }
            }
        }
        // apply velocity
        for body in &mut self.rigid_bodies {
            if body.is_static {
                body.update_this_according_to_coupled(world);
            } else {
                body.update(delta_time);

                body.update_coupled_according_to_this(world);
            }
        }
        for i in 0..self.players.len() {
            if let MovementMode::PHYSICS = self.players[i].movement_mode {
                self.move_player_with_collision(i, delta_time);
            }
        }
    }


    ///* New plan: Implement full physics tick, then make movement force based
    ///            https://www.youtube.com/watch?v=qdskE8PJy6Q
    fn move_player_with_collision(&mut self, player_index: usize, delta_time: f32) {
        {
            let player = &mut self.players[player_index];
            let rigid_body = &mut self.rigid_bodies[player.rigid_body_pointer.index];
            let air_resist = self.air_resistance_coefficient.powf(delta_time);
            let lerp_f = 1.0 - 0.001_f32.powf(delta_time / self.player_horiz_const_resistance);
            rigid_body.velocity = rigid_body.velocity * air_resist.powf(delta_time);
            let lerp = |a: f32, b: f32, t: f32| -> f32 {
                a + t * (b - a)
            };
            rigid_body.velocity.x = lerp(rigid_body.velocity.x, 0.0, lerp_f);
            rigid_body.velocity.z = lerp(rigid_body.velocity.z, 0.0, lerp_f);


            rigid_body.velocity = rigid_body.velocity + self.gravity * delta_time;
            /*
           let original_hitbox = player.rigid_body.hitbox.clone();
           match &mut player.rigid_body.hitbox {
               Hitbox::OBB(obb) => {
                   obb.half_extents = obb.half_extents - Vector::new_vec(player.skin_width)
               }
               Hitbox::CAPSULE(capsule) => {
                   capsule.radius -= player.skin_width;
               }
               Hitbox::MESH(_) => panic!("player mesh colliders not yet implemented")
           }

           self.collide(player, 0, delta_time);
           player.rigid_body.hitbox = original_hitbox;
            */

            let displacement = rigid_body.velocity * delta_time;
            rigid_body.position += displacement;
            player.grounded = false;
        }
        let mut iteration = 0;
        while iteration < MAX_ITERATIONS {
            let mut normal = Vector::new_empty();
            let mut deepest = ContactInformation { contact_points: Vec::new(), normal, penetration_depth: 0.0 };
            {
                let player = &self.players[player_index];
                let rigid_body = &self.rigid_bodies[player.rigid_body_pointer.index];
                let contacts = self.get_world_contacts(rigid_body);

                if contacts.is_none() || contacts.as_ref().unwrap().is_empty() {
                    break;
                }

                deepest = contacts.unwrap().into_iter()
                    .max_by(|a, b| a.penetration_depth.partial_cmp(&b.penetration_depth).unwrap())
                    .unwrap();

                normal = deepest.normal.normalize_3d();
                if normal.dot3(&rigid_body.velocity) > 0.0 {
                    normal = normal * -1.0;
                }
            }

            let player = &mut self.players[player_index];
            let rigid_body = &mut self.rigid_bodies[player.rigid_body_pointer.index];
            if normal.dot3(&self.gravity.normalize_3d()) < -0.35 {
                player.grounded = true;
            }

            let depenetration = normal * (deepest.penetration_depth);
            rigid_body.position += depenetration;

            rigid_body.velocity = rigid_body.velocity
                .project_onto_plane(&normal);

            if depenetration.magnitude_3d() < MIN_MOVE_THRESHOLD {
                break;
            }

            iteration += 1;
        }
        if iteration >= MAX_ITERATIONS {
            println!("Resolution exceeded max iterations - stuck in geometry");
            // Optional: Teleport player to last known good position
        }
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
#[derive(Clone, Copy)]
pub(crate) enum AxisType {
    FaceA(usize),
    FaceB(usize),
    Edge(usize, usize),
}

struct CastInformation {
    distance: f32,
    contacts: Vec<ContactInformation>,
}
pub struct ContactInformation {
    pub contact_points: Vec<ContactPoint>,
    pub normal: Vector,
    pub penetration_depth: f32,
}
impl ContactInformation {
    pub fn flip(mut self) -> ContactInformation {
        for point in &mut self.contact_points { point.flip(); }
        self.normal = -self.normal;
        self
    }
}
#[derive(Debug)]
pub(crate) struct ContactPoint {
    pub(crate) point_on_a: Vector,
    pub(crate) point_on_b: Vector,

    pub(crate) penetration: f32,
}
impl ContactPoint {
    fn flip(&mut self) {
        let temp = self.point_on_b;
        self.point_on_b = self.point_on_a;
        self.point_on_a = temp;
    }
}

pub(crate) struct ColliderInfo<'a> {
    pub(crate) hitbox: &'a Hitbox,
    pub(crate) position: &'a Vector,
    pub(crate) orientation: &'a Vector,
}
impl ColliderInfo<'_> {
    pub(crate) fn intersects_sphere(&self, other: &ColliderInfo) -> Option<ContactInformation> {
        if let Hitbox::SPHERE(sphere) = other.hitbox {
            return match self.hitbox {
                Hitbox::OBB(obb) => {
                    let obb_position = self.position;
                    let sphere_position = other.position;
                    let obb_orientation = self.orientation;
                    let sphere_orientation = other.orientation;
                    let rot = Matrix::new_rotate_quaternion_vec4(obb_orientation);

                    let sphere_center = sphere.center.rotate_by_quat(sphere_orientation) + sphere_position;
                    let obb_center = (rot * obb.center.with('w', 1.0)) + obb_position;

                    let axes = [
                        rot * Vector::new_vec4(1.0, 0.0, 0.0, 1.0),
                        rot * Vector::new_vec4(0.0, 1.0, 0.0, 1.0),
                        rot * Vector::new_vec4(0.0, 0.0, 1.0, 1.0),
                    ];

                    let delta = sphere_center - obb_center;
                    let local_sphere_center = Vector::new_vec3(delta.dot3(&axes[0]), delta.dot3(&axes[1]), delta.dot3(&axes[2]));

                    let closest_local = local_sphere_center.clamp3(&(-obb.half_extents), &obb.half_extents);
                    let closest_world = obb_center + (axes[0] * closest_local.x) + (axes[1] * closest_local.y) + (axes[2] * closest_local.z);

                    let diff = sphere_center - closest_world;
                    let dist_sq = diff.dot3(&diff);
                    let radius_sq = sphere.radius * sphere.radius;
                    if dist_sq > radius_sq {
                        return None
                    }
                    let dist = dist_sq.sqrt();

                    let (normal, penetration, point_on_obb) = if dist < 1e-6 {
                        let penetrations = [
                            (obb.half_extents.x - local_sphere_center.x.abs(), 0, local_sphere_center.x.signum()),
                            (obb.half_extents.y - local_sphere_center.y.abs(), 1, local_sphere_center.y.signum()),
                            (obb.half_extents.z - local_sphere_center.z.abs(), 2, local_sphere_center.z.signum()),
                        ];
                        let mut min_pen = penetrations[0];
                        for &pen in &penetrations[1..] {
                            if pen.0 < min_pen.0 {
                                min_pen = pen;
                            }
                        }

                        let axis_idx = min_pen.1;
                        let sign = min_pen.2;
                        let normal = axes[axis_idx] * sign;
                        let penetration = min_pen.0 + sphere.radius;

                        let mut local_point = local_sphere_center;
                        match axis_idx {
                            0 => local_point.x = obb.half_extents.x * sign,
                            1 => local_point.y = obb.half_extents.y * sign,
                            2 => local_point.z = obb.half_extents.z * sign,
                            _ => unreachable!(),
                        }

                        let point_on_obb = obb_center + (
                            axes[0] * local_point.x + axes[1] * local_point.y + axes[2] * local_point.z
                        );

                        (normal, penetration, point_on_obb)
                    } else {
                        let normal = diff * (1.0 / dist);
                        let penetration = sphere.radius - dist;
                        (normal, penetration, closest_world)
                    };

                    let point_on_sphere = sphere_center - (normal * sphere.radius);

                    let tolerance = 1e-4;
                    let mut contact_points = vec![ContactPoint {
                        point_on_a: point_on_obb,
                        point_on_b: point_on_sphere,
                        penetration,
                    }];

                    let on_face_x = (local_sphere_center.x.abs() - obb.half_extents.x).abs() < tolerance;
                    let on_face_y = (local_sphere_center.y.abs() - obb.half_extents.y).abs() < tolerance;
                    let on_face_z = (local_sphere_center.z.abs() - obb.half_extents.z).abs() < tolerance;

                    let faces_count = on_face_x as u8 + on_face_y as u8 + on_face_z as u8;

                    // additional contact points for edge/corner
                    if faces_count >= 2 && dist > 1e-6 {
                        let tangent1 = if normal.x.abs() < 0.9 {
                            Vector::new_vec3(1.0, 0.0, 0.0).cross(&normal).normalize_3d()
                        } else {
                            Vector::new_vec3(0.0, 1.0, 0.0).cross(&normal).normalize_3d()
                        };
                        let tangent2 = normal.cross(&tangent1).normalize_3d();

                        // contact points in a circle around the main contact
                        let num_additional = if faces_count == 3 { 3 } else { 2 }; // corner/edge
                        for i in 1..=num_additional {
                            let angle = (i as f32) * std::f32::consts::PI * 2.0 / (num_additional + 1) as f32;
                            let offset = tangent1 * (angle.cos() * 0.01) + tangent2 * (angle.sin() * 0.01);

                            contact_points.push(ContactPoint {
                                point_on_a: point_on_obb + offset,
                                point_on_b: point_on_sphere + offset,
                                penetration,
                            });
                        }
                    }

                    Some(ContactInformation {
                        contact_points,
                        normal,
                        penetration_depth: penetration,
                    })
                }
                Hitbox::SPHERE(a) => {
                    let p_a = a.center.rotate_by_quat(&self.orientation) + self.position;
                    let p_b = sphere.center.rotate_by_quat(&other.orientation) + other.position;

                    let d = p_b - p_a;
                    let d_m = d.magnitude_3d();
                    let n = if d_m > 1e-6 { d / d_m } else { Vector::new_vec3(0.0, 1.0, 0.0) };

                    if d_m > a.radius + sphere.radius { return None }

                    let point_on_a = p_a + n * a.radius;
                    let point_on_b = p_b - n * sphere.radius;

                    let penetration = a.radius + sphere.radius - d_m;

                    Some(ContactInformation {
                        contact_points: vec![ContactPoint{ point_on_a, point_on_b, penetration }],
                        normal: n,
                        penetration_depth: penetration
                    })
                }
                _ => { None }
            }
        }
        None
    }
    pub(crate) fn intersects_obb(&self, other: &ColliderInfo) -> Option<ContactInformation> {
        if let Hitbox::OBB(obb) = other.hitbox {
            return match self.hitbox {
                Hitbox::OBB(this_obb) => {
                    let (a, b) = (this_obb, obb);

                    let a_center = a.center.rotate_by_quat(&self.orientation) + self.position;
                    let b_center = b.center.rotate_by_quat(&other.orientation) + other.position;

                    let a_quat = self.orientation;
                    let b_quat = other.orientation;

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
                    let mut best_axis_type = AxisType::FaceA(0);
                    for i in 0..3 {
                        { // a_normals
                            let axis = a_axes[i];
                            let penetration = Self::test_axis(&axis, &t, &a.half_extents, &b.half_extents, &a_axes, &b_axes, true);
                            if penetration < 0.0 {
                                return None
                            }
                            if penetration < min_penetration {
                                min_penetration = penetration;
                                collision_normal = axis;
                                best_axis_type = AxisType::FaceA(i);
                            }
                        }
                        { // b_normals
                            let axis = b_axes[i];
                            let penetration = Self::test_axis(&axis, &t, &a.half_extents, &b.half_extents, &a_axes, &b_axes, true);
                            if penetration < 0.0 {
                                return None
                            }
                            if penetration < min_penetration {
                                min_penetration = penetration;
                                collision_normal = axis;
                                best_axis_type = AxisType::FaceB(i);
                            }
                        }
                        for j in 0..3 { // edge-edge cross-products
                            let axis = a_axes[i].cross(&b_axes[j]);
                            let axis_length = axis.magnitude_3d();

                            if axis_length < 1e-6 {
                                continue;
                            }

                            let axis_normalized = axis / axis_length;
                            let penetration = Self::test_axis_cross(&axis_normalized, &t, &a.half_extents, &b.half_extents, &a_axes, &b_axes);

                            if penetration < 0.0 {
                                return None;
                            }

                            if penetration < min_penetration {
                                min_penetration = penetration;
                                collision_normal = axis_normalized;
                                best_axis_type = AxisType::Edge(i, j);
                            }
                        }
                    }

                    // normal points from A to B
                    if collision_normal.dot(&t) < 0.0 {
                        collision_normal = -collision_normal;
                    }

                    //TODO correct contact points
                    Some(ContactInformation {
                        contact_points: vec![ContactPoint {
                            point_on_a: a_center + t.normalize_3d() * a.half_extents.magnitude_3d(),
                            point_on_b: b_center - t.normalize_3d() * b.half_extents.magnitude_3d(),
                            penetration: min_penetration
                        }],
                        normal: collision_normal,
                        penetration_depth: min_penetration,
                    })
                }
                Hitbox::SPHERE(_) => {
                    if let Some(contact) = other.intersects_sphere(self) {
                        Some(contact.flip())
                    } else { None }
                }
                _ => None
            }
        }
        None
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