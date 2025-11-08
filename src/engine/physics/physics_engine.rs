use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use crate::engine::physics::player::{MovementMode, Player};
use crate::engine::world::scene::{Mesh, Node, Scene, Vertex};
use crate::math::*;
use crate::math::matrix::Matrix;
use crate::render::render::HitboxPushConstantSendable;

pub struct PhysicsEngine {
    pub gravity: Vector,
    pub air_resistance_coefficient: f32,
    pub player_horiz_const_resistance: f32,

    pub rigid_bodies: Vec<RigidBody>,
    pub players: Vec<Arc<RefCell<Player>>>
}

impl PhysicsEngine {
    pub fn new(world: &Scene, gravity: Vector, air_resistance_coefficient: f32, player_horiz_const_resistance: f32) -> Self {
        let mut rigid_bodies = Vec::new();
        for (model_index, model) in world.models.iter().enumerate() {
            for (node_index, node) in model.nodes.iter().enumerate() {
                if let Some(mesh) = &node.mesh {
                    let (min, max) = mesh.borrow().get_min_max();
                    let half_extent = (max - min) * 0.5 * node.scale;

                    let hitbox;
                    if (
                        if half_extent.x > 1.0 {1} else {0} +
                        if half_extent.y > 1.0 {1} else {0} +
                        if half_extent.z > 1.0 {1} else {0}
                    ) >- 3 {
                        let mesh_collider = MeshCollider {
                            mesh: mesh.clone(),
                            current_scale_factor: node.scale,
                            bvh: Bvh::new(mesh.clone()),
                        };
                        hitbox = Hitbox::MESH(mesh_collider);
                    } else {
                        hitbox = Hitbox::OBB(BoundingBox {
                            center: (min + max) * 0.5,
                            half_extents: half_extent,
                        });
                    }

                    rigid_bodies.push(RigidBody::new_from_node(node, Some((model_index, node_index, (min, max))), hitbox));
                }
            }
        }
        Self {
            gravity,
            air_resistance_coefficient,
            player_horiz_const_resistance,
            rigid_bodies,
            players: Vec::new(),
        }
    }
    pub fn add_player(&mut self, player: Arc<RefCell<Player>>) {
        self.players.push(player);
    }

    pub fn tick(&mut self, delta_time: f32, world: &Scene) {
        for body in &mut self.rigid_bodies {
            body.update_according_to_coupled(world);

            if !body.is_static {
                body.velocity = body.velocity + self.gravity * delta_time;
            }
        }
        for player_ref in &self.players {
            let mut player = player_ref.borrow_mut();
            match player.movement_mode {
                MovementMode::GHOST => { continue }
                MovementMode::PHYSICS => {
                    player.rigid_body.position = player.camera.position;

                    player.rigid_body.velocity = player.rigid_body.velocity + self.gravity * delta_time;

                    let velocity_step = player.rigid_body.velocity * delta_time;

                    self.move_player_with_collision(&mut player, velocity_step);
                }
            }
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
        player.step(intended_step);
        player.grounded = false;
        for rigid_body in &self.rigid_bodies {
            if let Some(contacts) = &mut rigid_body.colliding_with_info(&player.rigid_body) {
                for contact in contacts.iter_mut() {
                    if contact.normal.dot3(&player.rigid_body.velocity) < 0.0 {
                        contact.normal = -1.0 * contact.normal;
                    }
                    player.step(-1.0 * contact.normal * contact.penetration_depth);

                    if contact.normal.dot3(&self.gravity) > 0.9 { // threshold for jumping on walls
                        player.grounded = true;
                    }

                    player.rigid_body.velocity = (player.rigid_body.velocity - player.rigid_body.velocity.project_onto(&contact.normal))
                }
            }
        }

        if player.rigid_body.position.y < -20.0 {
            player.camera.position.y = 20.0;
            player.rigid_body.velocity = Vector::new_empty();
        }

        player.rigid_body.velocity = Vector::new_vec3(
            player.rigid_body.velocity.x * (1.0 - self.player_horiz_const_resistance) * (1.0 - self.air_resistance_coefficient),
            player.rigid_body.velocity.y * (1.0 - self.air_resistance_coefficient),
            player.rigid_body.velocity.z * (1.0 - self.player_horiz_const_resistance) * (1.0 - self.air_resistance_coefficient),
        );
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
    pub coupled_with_scene_object: Option<(usize, usize, (Vector, Vector))>, // model index, node index, (min, max)

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
    pub fn new_from_node(node: &Node, couple_with_node: Option<(usize, usize, (Vector, Vector))>, hitbox: Hitbox) -> Self {
        let mut body = RigidBody::default();
        body.coupled_with_scene_object = couple_with_node;
        body.position = node.world_transform * Vector::new_vec4(0.0, 0.0, 0.0, 1.0);
        body.orientation = node.world_transform.extract_quaternion();
        body.hitbox = hitbox;
        body
    }
    pub fn update_according_to_coupled(&mut self, world: &Scene) {
        if let Some(coupled_object) = self.coupled_with_scene_object {
            let node = &world.models[coupled_object.0].nodes[coupled_object.1];

            let scale = node.world_transform.extract_scale();
            self.position = node.world_transform * Vector::new_vec4(0.0, 0.0, 0.0, 1.0);
            self.orientation = node.world_transform.extract_quaternion();
            match &mut self.hitbox {
                Hitbox::OBB(_) => {
                    let local_center = (coupled_object.2.0 + coupled_object.2.1) * 0.5;
                    let obb = BoundingBox {
                        center: local_center * scale,
                        half_extents: (coupled_object.2.1 - coupled_object.2.0) * 0.5 * scale,
                    };
                    self.hitbox = Hitbox::OBB(obb);
                }
                Hitbox::MESH(mesh) => {
                    self.orientation = node.world_transform.extract_quaternion();
                    mesh.current_scale_factor = scale;
                    self.position = node.world_transform * Vector::new_vec4(0.0, 0.0, 0.0, 1.0);
                }
            }
        }
    }

    pub fn colliding_with_info(&self, other: &RigidBody) -> Option<Vec<ContactInformation>> {
        match (&self.hitbox, &other.hitbox) {
            (Hitbox::OBB(a), Hitbox::OBB(b)) => {
                let info = Self::obb_intersects_obb(
                    &a, &b,
                    &self.position, &other.position,
                    &self.orientation, &other.orientation,
                );
                if info.is_some() {
                    Some(vec![info.unwrap()])
                } else { None }
            }
            (Hitbox::OBB(_), Hitbox::MESH(_)) => {
                self.obb_intersects_mesh(other)
            }
            (Hitbox::MESH(_), Hitbox::OBB(_)) => {
                other.obb_intersects_mesh(self)
            }
            _ => None,
        }
    }
    pub fn obb_intersects_mesh(&self, other: &RigidBody) -> Option<Vec<ContactInformation>> {
        let obb = if let Hitbox::OBB(ref obb) = self.hitbox {
            obb
        } else {
            return None;
        };
        let mesh_collider = if let Hitbox::MESH(ref mesh) = other.hitbox {
            mesh
        } else {
            return None;
        };

        let rot_mat = Matrix::new_rotate_quaternion_vec4(&other.orientation);
        let rot_mat_inv = rot_mat.inverse();

        let obb_mesh_space = BoundingBox {
            center: obb.center / mesh_collider.current_scale_factor,
            half_extents: obb.half_extents / mesh_collider.current_scale_factor,
        };
        let obb_position_mesh_space = rot_mat_inv * ((self.position - &other.position) / mesh_collider.current_scale_factor).unitize_w();
        let obb_orientation_mesh_space = self.orientation.combine(&other.orientation.conjugate());


        let mut contact_infos = Self::split_into_bvh(
            &obb_mesh_space,
            &obb_position_mesh_space,
            &obb_orientation_mesh_space,
            &mesh_collider,
            &mesh_collider.bvh
        );


        if let Some(infos) = &mut contact_infos {
            for info in infos.iter_mut() {
                info.normal = rot_mat * (info.normal * mesh_collider.current_scale_factor).unitize_w();
            }
            return contact_infos;
        }

        None
    }
    fn split_into_bvh(
        obb: &BoundingBox,
        obb_position: &Vector,
        obb_orientation: &Vector,
        mesh_collider: &MeshCollider,
        bvh: &Bvh,
    ) -> Option<Vec<ContactInformation>> {
        let intersection = Self::obb_intersects_obb(
            &obb, &bvh.bounds,
            &obb_position, &Vector::new_empty_quat(),
            &obb_orientation, &Vector::new_empty_quat()
        );
        if intersection.is_some() {
            if let Some(triangle_indices) = &bvh.triangle_indices {
                //TODO() Take ONLY from triangles. do not use anything from OBB-OBB results.
                // this is for debugging
                return Some(vec![intersection.unwrap()]);
            };
            let mut left_intersections = if let Some(left) = &bvh.left_child {
                Self::split_into_bvh(
                    obb,
                    obb_position,
                    obb_orientation,
                    mesh_collider,
                    left
                ).unwrap_or(vec![])
            } else { vec![] };
            let mut right_intersections = if let Some(right) = &bvh.right_child {
                Self::split_into_bvh(
                    obb,
                    obb_position,
                    obb_orientation,
                    mesh_collider,
                    right
                ).unwrap_or(vec![])
            } else { vec![] };
            left_intersections.append(&mut right_intersections);
            return Some(left_intersections);
        }
        None
    }
    fn obb_intersects_obb(
        a_bounds: &BoundingBox, b_bounds: &BoundingBox,
        a_position: &Vector, b_position: &Vector,
        a_orientation: &Vector, b_orientation: &Vector,
    ) -> Option<ContactInformation> {
        let (a, b) = (a_bounds, b_bounds);

        let a_center = a.center.rotate_by_quat(&a_orientation) + a_position;
        let b_center = b.center.rotate_by_quat(&b_orientation) + b_position;

        let a_quat = a_orientation;
        let b_quat = b_orientation;

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
            coupled_with_scene_object: None,
            hitbox: Hitbox::OBB(BoundingBox {
                center: Vector::new_vec(0.0),
                half_extents: Vector::new_vec(1.0),
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
            orientation: Vector::new_empty_quat(),
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

pub enum Hitbox {
    OBB(BoundingBox),
    MESH(MeshCollider),
}
pub struct BoundingBox {
    pub center: Vector,
    pub half_extents: Vector,
}
impl BoundingBox {
    pub fn from_min_max(min: &Vector, max: &Vector) -> Self {
        BoundingBox {
            center: (min + max) * 0.5,
            half_extents: (max - min) * 0.5,
        }
    }
}
pub struct MeshCollider {
    pub mesh: Rc<RefCell<Mesh>>,
    pub current_scale_factor: Vector,
    pub bvh: Bvh
}
pub struct Bvh {
    bounds: BoundingBox,
    left_child: Option<Box<Bvh>>,
    right_child: Option<Box<Bvh>>,
    triangle_indices: Option<Vec<usize>>,
}

impl Bvh {
    pub fn get_bounds_info(&self) -> Vec<(Vector, Vector)> { // centers, half extents
        let mut constants = Vec::new();
        self.bounds_stack_add_bvh(&self, &mut constants);
        constants
    }
    pub fn bounds_stack_add_bvh(&self, bvh: &Bvh, constants: &mut Vec<(Vector, Vector)>) {
        if true {
            constants.push((
                bvh.bounds.center.clone(),
                bvh.bounds.half_extents.clone()
            ));
        }
        if let Some(left_child) = &bvh.left_child {
            self.bounds_stack_add_bvh(left_child, constants);
        }
        if let Some(right_child) = &bvh.right_child {
            self.bounds_stack_add_bvh(right_child, constants);
        }
    }

    pub fn new(mesh: Rc<RefCell<Mesh>>) -> Bvh {
        let mesh_ref = mesh.borrow();
        let mut triangles = Vec::new();

        for primitive in &mesh_ref.primitives {
            let indices: Vec<u32> = if primitive.index_data_u8.len() > 0 {
                primitive.index_data_u8.iter().map(|i| *i as u32).collect()
            } else if primitive.index_data_u16.len() > 0 {
                primitive.index_data_u16.iter().map(|i| *i as u32).collect()
            } else if primitive.index_data_u32.len() > 0 {
                primitive.index_data_u32.clone()
            } else {
                panic!("mesh does not have indices")
            };

            for i in (0..indices.len()).step_by(3) {
                let v0 = &primitive.vertex_data[indices[i] as usize];
                let v1 = &primitive.vertex_data[indices[i + 1] as usize];
                let v2 = &primitive.vertex_data[indices[i + 2] as usize];

                let centroid = Self::centroid(v0, v1, v2);
                triangles.push((i / 3, centroid));
            }
        }

        drop(mesh_ref);
        let num_tris = triangles.len();
        Self::split(mesh, &mut triangles, 0, num_tris)
    }

    fn split(
        mesh: Rc<RefCell<Mesh>>,
        triangles: &mut [(usize, Vector)],
        start: usize,
        end: usize
    ) -> Bvh {
        let (min, max) = Self::min_max(&mesh, triangles, start, end);
        let num_triangles = end - start;

        const MAX_LEAF_SIZE: usize = 4;

        if num_triangles <= MAX_LEAF_SIZE {
            let triangle_indices: Vec<usize> = triangles[start..end]
                .iter()
                .map(|(idx, _)| *idx)
                .collect();

            return Bvh {
                bounds: BoundingBox::from_min_max(&min, &max),
                left_child: None,
                right_child: None,
                triangle_indices: Some(triangle_indices),
            };
        }

        let extent = max - min;
        let axis = if extent.x > extent.y && extent.x > extent.z {
            'x'
        } else if extent.y > extent.z {
            'y'
        } else {
            'z'
        };

        Self::sort_triangles_by_axis(&mut triangles[start..end], axis);

        let mid = start + num_triangles / 2;

        let left_child = Some(Box::new(Self::split(
            mesh.clone(),
            triangles,
            start,
            mid
        )));

        let right_child = Some(Box::new(Self::split(
            mesh.clone(),
            triangles,
            mid,
            end
        )));

        Bvh {
            bounds: BoundingBox::from_min_max(&min, &max),
            left_child,
            right_child,
            triangle_indices: None,
        }
    }

    fn min_max(
        mesh: &Rc<RefCell<Mesh>>,
        triangles: &[(usize, Vector)],
        start: usize,
        end: usize
    ) -> (Vector, Vector) {
        let mut min = Vector::new_vec(f32::MAX);
        let mut max = Vector::new_vec(f32::MIN);

        let mesh_borrow = mesh.borrow();

        for (triangle_idx, _) in &triangles[start..end] {
            let (v0, v1, v2) = Self::get_triangle_vertices(&mesh_borrow, *triangle_idx);

            min = Vector::min(&min, &Vector::new_from_array(&v0.position));
            min = Vector::min(&min, &Vector::new_from_array(&v1.position));
            min = Vector::min(&min, &Vector::new_from_array(&v2.position));

            max = Vector::max(&max, &Vector::new_from_array(&v0.position));
            max = Vector::max(&max, &Vector::new_from_array(&v1.position));
            max = Vector::max(&max, &Vector::new_from_array(&v2.position));
        }

        (min, max)
    }

    fn get_triangle_vertices(mesh: &Mesh, triangle_index: usize) -> (Vertex, Vertex, Vertex) {
        let primitive = &mesh.primitives[0];

        let idx0;
        let idx1;
        let idx2;

        if primitive.index_data_u8.len() > 0 {
            idx0 = primitive.index_data_u8[3 * triangle_index] as usize;
            idx1 = primitive.index_data_u8[3 * triangle_index + 1] as usize;
            idx2 = primitive.index_data_u8[3 * triangle_index + 2] as usize;
        } else if primitive.index_data_u16.len() > 0 {
            idx0 = primitive.index_data_u16[3 * triangle_index] as usize;
            idx1 = primitive.index_data_u16[3 * triangle_index + 1] as usize;
            idx2 = primitive.index_data_u16[3 * triangle_index + 2] as usize;
        } else if primitive.index_data_u32.len() > 0 {
            idx0 = primitive.index_data_u32[3 * triangle_index] as usize;
            idx1 = primitive.index_data_u32[3 * triangle_index + 1] as usize;
            idx2 = primitive.index_data_u32[3 * triangle_index + 2] as usize;
        } else {
            panic!("mesh does not have indices")
        }

        (
            primitive.vertex_data[idx0].clone(),
            primitive.vertex_data[idx1].clone(),
            primitive.vertex_data[idx2].clone()
        )
    }

    fn sort_triangles_by_axis(triangles: &mut [(usize, Vector)], axis: char) {
        match axis {
            'x' => triangles.sort_by(|a, b| a.1.x.partial_cmp(&b.1.x).unwrap()),
            'y' => triangles.sort_by(|a, b| a.1.y.partial_cmp(&b.1.y).unwrap()),
            'z' => triangles.sort_by(|a, b| a.1.z.partial_cmp(&b.1.z).unwrap()),
            _ => panic!("Unknown axis"),
        }
    }

    fn centroid(a: &Vertex, b: &Vertex, c: &Vertex) -> Vector {
        (Vector::new_from_array(&a.position)
            + Vector::new_from_array(&b.position)
            + Vector::new_from_array(&c.position)) / 3.0
    }
}