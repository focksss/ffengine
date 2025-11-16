use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;
use image::imageops::sample_nearest;
use crate::engine::physics::player::{MovementMode, Player};
use crate::engine::world::scene::{Mesh, Model, Node, Scene, Vertex};
use crate::math::*;
use crate::math::matrix::Matrix;
use crate::render::render::HitboxPushConstantSendable;

const MAX_ITERATIONS: usize = 5;
const MIN_MOVE_THRESHOLD: f32 = 0.001;

pub struct PhysicsEngine {
    pub gravity: Vector,
    pub air_resistance_coefficient: f32,
    pub player_horiz_const_resistance: f32,

    pub rigid_bodies: Vec<RigidBody>,
    pub players: Vec<Arc<RefCell<Player>>>
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
    pub fn add_all_nodes_from_model(&mut self, world: &Scene, model_index: usize, gen_mesh_hitbox: bool) {
        let model = &world.models[model_index];
        for (node_index, node) in model.nodes.iter().enumerate() {
            if let Some(mesh) = &node.mesh {
                let (min, max) = mesh.borrow().get_min_max();
                let half_extent = (max - min) * 0.5 * node.scale;

                let hitbox;
                if gen_mesh_hitbox {
                    let mesh_collider = MeshCollider::new(mesh.clone(), node.scale);
                    hitbox = Hitbox::MESH(mesh_collider);
                } else {
                    hitbox = Hitbox::OBB(BoundingBox {
                        center: (min + max) * 0.5,
                        half_extents: half_extent,
                    });
                }

                let rb = RigidBody::new_from_node(node, Some((model_index, node_index, (min, max))), hitbox);
                self.rigid_bodies.push(rb);
            }
        }
    }
    pub fn add_player(&mut self, player: Arc<RefCell<Player>>) {
        self.players.push(player);
    }

    fn get_world_contacts(&self, body:& RigidBody) -> Option<Vec<ContactInformation>> {
        let mut collisions = Vec::new();
        for other_body in self.rigid_bodies.iter() {
            if let Some(new_collisions) = body.colliding_with_info(other_body) {
                collisions.extend(new_collisions);
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
                if body.colliding_with_info(other_body).is_some() {
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
                if let Some(collisions) = body_a.colliding_with_info(body_b) {
                    if collisions.is_empty() { continue; }
                    let deepest_collision = Self::get_deepest_of_contacts(&collisions);
                    let normal = deepest_collision.normal;
                    let depth = deepest_collision.penetration_depth;
                    let im_a = body_a.inv_mass; let im_b = body_b.inv_mass;
                    let s_im = im_a + im_b;
                    let restitution = body_a.restitution_coefficient * body_b.restitution_coefficient;
                    let inv_inertia_a = body_a.get_inverse_inertia_tensor_world_space();
                    let inv_inertia_b = body_b.get_inverse_inertia_tensor_world_space();

                    let pt_on_a = deepest_collision.points[0].0;
                    let pt_on_b = deepest_collision.points[0].1;
                    let ra = pt_on_a - body_a.get_center_of_mass_world_space();
                    let rb = pt_on_b - body_b.get_center_of_mass_world_space();

                    let angular_j_a = (inv_inertia_a * ra.cross(&normal)).cross(&ra);
                    let angular_j_b = (inv_inertia_b * rb.cross(&normal)).cross(&rb);
                    let angular_factor = (angular_j_a + angular_j_b).dot3(&normal);

                    let vel_a = body_a.velocity + body_a.angular_velocity.cross(&ra);
                    let vel_b = body_b.velocity + body_b.angular_velocity.cross(&rb);

                    let v_diff = vel_a - vel_b;

                    let j = normal * (1.0 + restitution) * v_diff.dot3(&normal) / (s_im + angular_factor);
                    body_a.apply_impulse(-1.0 * j, pt_on_a);
                    body_b.apply_impulse(j, pt_on_b);

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
        for player_ref in &self.players {
            let mut player = player_ref.borrow_mut();
            match player.movement_mode {
                MovementMode::GHOST => {
                    // println!("{:?}", self.body_cast(&mut player.rigid_body, &Vector::new_empty_quat(), &Vector::new_vec3(0.0, 0.0, -1.0), 1.0));
                    ///*
                    // let all_original_collisions = self.get_world_contacts(&player.rigid_body);
                    //println!("{:?}", all_original_collisions);
                    //*/

                    continue
                }
                MovementMode::PHYSICS => {
                    player.rigid_body.position = player.camera.position;

                    self.move_player_with_collision(&mut player, delta_time);
                }
            }
        }
    }


    ///* New plan: Implement full physics tick, then make movement force based
    ///            https://www.youtube.com/watch?v=qdskE8PJy6Q
    fn move_player_with_collision(&self, player: &mut Player, delta_time: f32) {
        let air_resist = self.air_resistance_coefficient.powf(delta_time);
        let lerp_f = 1.0 - 0.001_f32.powf(delta_time / self.player_horiz_const_resistance);
        player.rigid_body.velocity = player.rigid_body.velocity * air_resist.powf(delta_time);
        let lerp = |a: f32, b: f32, t: f32| -> f32 {
            a + t * (b - a)
        };
        player.rigid_body.velocity.x = lerp(player.rigid_body.velocity.x, 0.0, lerp_f);
        player.rigid_body.velocity.z = lerp(player.rigid_body.velocity.z, 0.0, lerp_f);


        player.rigid_body.velocity = player.rigid_body.velocity + self.gravity * delta_time;
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

        // /*
        let mut iteration = 0;
        let displacement = player.rigid_body.velocity * delta_time;
        player.step(&displacement);
        player.grounded = false;

        while iteration < MAX_ITERATIONS {
            let contacts = self.get_world_contacts(&player.rigid_body);

            if contacts.is_none() || contacts.as_ref().unwrap().is_empty() {
                break;
            }

            let deepest = contacts.unwrap().into_iter()
                .max_by(|a, b| a.penetration_depth.partial_cmp(&b.penetration_depth).unwrap())
                .unwrap();

            let mut normal = deepest.normal.normalize_3d();
            if normal.dot3(&player.rigid_body.velocity) > 0.0 {
                normal = normal * -1.0;
            }

            if normal.dot3(&self.gravity.normalize_3d()) < -0.35 {
                player.grounded = true;
            }

            let depenetration = normal * (deepest.penetration_depth);
            player.step(&depenetration);

            player.rigid_body.velocity = player.rigid_body.velocity
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
        // */


        if player.rigid_body.position.y < -20.0 {
            player.camera.position.y = 20.0;
            player.rigid_body.velocity = Vector::new_empty();
        }
    }
    fn collide(&self, player: &mut Player, iteration_depth: i32, delta_time: f32) {
        if iteration_depth == 0 {
            player.grounded = false;
            // println!("starting velocity: {:?}", player.rigid_body.velocity);
            player.rigid_body.velocity = player.rigid_body.velocity * delta_time; // use velocity field as storage for remaining displacement in step
        }
        if iteration_depth > 10 {
            player.rigid_body.velocity = Vector::new_empty();
            return // no need to convert remaining displacement to velocity, there is none.
        }

        let displacement = player.rigid_body.velocity; // use velocity field as storage for remaining displacement in step
        // println!("iteration: {}", iteration_depth);
        // println!("    remaining displacement: {:?}", displacement);

        let max_dist = displacement.magnitude_3d() + player.skin_width;

        if let Some(hits) = self.body_cast(&mut player.rigid_body, &Vector::new_empty(), &displacement, max_dist) {
            for contact in hits.contacts {
                let mut normal = contact.normal.normalize_3d();
                if normal.dot3(&displacement) > 0.0 {
                    normal = normal * -1.0;
                }
                if normal.dot3(&self.gravity.normalize_3d()) < -0.35 {
                    player.grounded = true;
                }
                println!("    normal: {:?}", normal);
                println!("    depth: {:?}", contact.penetration_depth);

                let displacement_direction = displacement.normalize_3d();
                let mut surface_snap_displacement = displacement_direction * (hits.distance - player.skin_width * 0.5);
                if surface_snap_displacement.magnitude_3d() >= player.skin_width {
                    player.step(&surface_snap_displacement);
                } else {
                    surface_snap_displacement = Vector::new_empty();
                }

                let remaining_displacement = displacement - surface_snap_displacement;
                // println!("    remaining displacement: {:?}", remaining_displacement);
                let remaining_displacement_rotated_onto_normal = remaining_displacement.project_onto_plane(&normal);
                //TODO: do or don't do this?: .normalize_3d() * remaining_displacement.magnitude_3d();
                // println!("    remaining projected: {:?}", remaining_displacement_rotated_onto_normal);
                player.rigid_body.velocity = remaining_displacement_rotated_onto_normal; // use velocity field as storage for remaining displacement in next step
            }
            self.collide(player, iteration_depth + 1, delta_time)
        } else {
            // println!("    terminating with step of {:?}", displacement);
            player.step(&displacement);
            player.rigid_body.velocity = displacement / delta_time; // convert remaining displacement to velocity
            // println!("    terminating with final velocity {:?}", player.rigid_body.velocity);
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

pub struct RigidBody {
    pub coupled_with_scene_object: Option<(usize, usize, (Vector, Vector))>, // model index, node index, (min, max)

    pub hitbox: Hitbox,
    is_static: bool,
    pub restitution_coefficient: f32,
    pub friction_coefficient: f32,
    mass: f32,
    inv_mass: f32,

    pub force: Vector,
    pub torque: Vector,

    pub position: Vector,
    pub velocity: Vector,
    pub orientation: Vector, // quaternion
    pub angular_velocity: Vector,
    center_of_mass: Vector,

    inertia_tensor: Matrix, // 3x3
    inv_inertia_tensor: Matrix,
}

impl RigidBody {
    pub fn new_from_node(node: &Node, couple_with_node: Option<(usize, usize, (Vector, Vector))>, hitbox: Hitbox) -> Self {
        let mut body = RigidBody::default();
        body.coupled_with_scene_object = couple_with_node;
        body.position = node.world_transform * Vector::new_vec4(0.0, 0.0, 0.0, 1.0);
        body.orientation = node.world_transform.extract_quaternion();
        body.hitbox = hitbox;
        body.update_shape_properties();
        body
    }
    pub fn update_this_according_to_coupled(&mut self, world: &Scene)  {
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
                    mesh.rescale_bvh(scale);
                    self.position = node.world_transform * Vector::new_vec4(0.0, 0.0, 0.0, 1.0);
                }
                Hitbox::CAPSULE(_) => {
                    let local_center = (coupled_object.2.0 + coupled_object.2.1) * 0.5;
                    let half_extents = (coupled_object.2.1 - coupled_object.2.0) * 0.5 * scale;
                    let radius = Vector::new_vec3(half_extents.x, 0.0, half_extents.z).magnitude_3d();
                    let capsule = Capsule {
                        a: Vector::new_vec3(local_center.x, coupled_object.2.0.y + radius, local_center.z),
                        b: Vector::new_vec3(local_center.x, coupled_object.2.1.y - radius, local_center.z),
                        radius
                    };
                    self.hitbox = Hitbox::CAPSULE(capsule);
                }
            }
            self.update_shape_properties();
        }
    }
    pub fn update_coupled_according_to_this(&self, world: &mut Scene) {
        if let Some(coupled_object) = self.coupled_with_scene_object {
            let node = &mut world.models[coupled_object.0].nodes[coupled_object.1];

            node.translation = self.position;
            node.rotation = self.orientation;
            node.needs_update = true;
        }
    }
    pub fn set_mass(&mut self, mass: f32) {
        self.mass = mass;
        self.inv_mass = 1.0 / mass;
        self.update_shape_properties();
    }
    pub fn set_static(&mut self, is_static: bool) {
        self.is_static = is_static;
        if is_static {
            self.inv_mass = 0.0;
            self.mass = 999.0
        } else {
            self.inv_mass = 1.0 / self.mass;
        }
        self.update_shape_properties();
    }

    pub fn update_shape_properties(&mut self) {
        match self.hitbox {
            Hitbox::OBB(obb) => {
                let a = obb.half_extents.x * 2.0;
                let b = obb.half_extents.y * 2.0;
                let c = obb.half_extents.z * 2.0;

                self.inertia_tensor = Matrix::new();
                self.inertia_tensor.set(0, 0, (1.0 / 12.0) * (b * b + c * c));
                self.inertia_tensor.set(1, 1, (1.0 / 12.0) * (a * a + c * c));
                self.inertia_tensor.set(2, 2, (1.0 / 12.0) * (a * a + b * b));

                self.center_of_mass = obb.center
            }
            Hitbox::MESH(_) => {
                //TODO inertia tensor and center of mass
            }
            Hitbox::CAPSULE(capsule) => {
                //TODO inertia tensor

                self.center_of_mass = (capsule.a + capsule.b) * 0.5
            }
        }
        self.inv_inertia_tensor = self.inertia_tensor.inverse3().mul_float_into3(self.inv_mass);
    }
    fn get_inverse_inertia_tensor_world_space(&self) -> Matrix {
        let rot = Matrix::new_rotate_quaternion_vec4(&self.orientation);
        rot * self.inv_inertia_tensor * rot.transpose3()
    }
    fn get_center_of_mass_world_space(&self) -> Vector {
        self.position + Matrix::new_rotate_quaternion_vec4(&self.orientation) * self.center_of_mass
    }

    pub fn update(&mut self, delta_time: f32) {
        self.position += self.velocity * delta_time;

        let c = self.get_center_of_mass_world_space();
        let c_to_pos = self.position - c;

        let rot = Matrix::new_rotate_quaternion_vec4(&self.orientation);
        let inertia_world = rot * self.inertia_tensor * rot.transpose3();
        let inv_inertia_world = rot * self.inv_inertia_tensor * rot.transpose3();
        let torque = self.angular_velocity.cross(&(inertia_world * self.angular_velocity));
        let alpha = inv_inertia_world * torque;
        self.angular_velocity += alpha * delta_time;

        let d_theta = self.angular_velocity * delta_time;
        let dq = Vector::axis_angle_quat(&d_theta, d_theta.magnitude_3d());
        self.orientation = dq.combine(&self.orientation).normalize_4d();

        println!("orientation: {:?}, dq: {:?}, dtheta {:?}", self.orientation, dq, d_theta);

        self.position = c + Matrix::new_rotate_quaternion_vec4(&dq) * c_to_pos;
    }

    pub fn apply_impulse(&mut self, impulse: Vector, point: Vector) {
        if self.inv_mass == 0.0 { return }

        self.velocity += impulse * self.inv_mass;

        let c = self.get_center_of_mass_world_space();
        let r = point - c;
        let dl = r.cross(&impulse);
        self.apply_angular_impulse(dl);
    }
    pub fn apply_angular_impulse(&mut self, impulse: Vector) {
        if self.inv_mass == 0.0 { return }
        self.angular_velocity += self.get_inverse_inertia_tensor_world_space() * impulse;

        const MAX_ANGULAR_SPEED: f32 = 30.0;
        if self.angular_velocity.magnitude_3d() > MAX_ANGULAR_SPEED {
            self.angular_velocity = self.angular_velocity.normalize_3d() * MAX_ANGULAR_SPEED;
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
                self.intersects_mesh(other)
            }
            (Hitbox::OBB(a), Hitbox::CAPSULE(b)) => {
                let info = Self::obb_intersects_capsule(
                    &a, &self.orientation, &self.position,
                    &b.a, &b.b, b.radius, &other.position, &other.orientation
                );
                if info.is_some() {
                    Some(vec![info.unwrap()])
                } else { None }
            }
            (Hitbox::MESH(_), Hitbox::MESH(_)) => {
                other.intersects_mesh(self)
            }
            (Hitbox::MESH(_), Hitbox::OBB(_)) => {
                other.intersects_mesh(self)
            }
            (Hitbox::MESH(_), Hitbox::CAPSULE(_)) => {
                other.intersects_mesh(self)
            }
            (Hitbox::CAPSULE(_), Hitbox::CAPSULE(_)) => {
                panic!("capsule-capsule intersections not yet implemented");
            }
            (Hitbox::CAPSULE(a), Hitbox::OBB(b)) => {
                let info = Self::obb_intersects_capsule(
                    &b, &other.orientation, &other.position,
                    &a.a, &a.b, a.radius, &self.position, &self.orientation
                );
                if info.is_some() {
                    Some(vec![info.unwrap()])
                } else { None }
            }
            (Hitbox::CAPSULE(_), Hitbox::MESH(_)) => {
                self.intersects_mesh(other)
            }
        }
    }
    pub fn obb_intersects_capsule(
        a_bounds: &BoundingBox, a_orientation: &Vector, a_position: &Vector,
        b_point_a: &Vector, b_point_b: &Vector, b_radius: f32, b_position: &Vector, b_orientation: &Vector,
    ) -> Option<ContactInformation> {
        let rot_mat = Matrix::new_rotate_quaternion_vec4(a_orientation);
        let inv_rot_mat = rot_mat.inverse4();

        let capsule_rot_mat = Matrix::new_rotate_quaternion_vec4(&b_orientation);

        let world_a = capsule_rot_mat * (b_point_a.unitize_w()) + b_position;
        let world_b = capsule_rot_mat * (b_point_b.unitize_w()) + b_position;

        let obb_center = a_position + a_bounds.center.rotate_by_quat(a_orientation);

        let p0_local = inv_rot_mat * ((world_a - obb_center).unitize_w());
        let p1_local = inv_rot_mat * ((world_b - obb_center).unitize_w());

        let closest_point_on_obb_to_segment = |
            bounds: &BoundingBox,
            p0: &Vector,
            p1: &Vector,
        | -> Vector {
            let clamped_p0 = p0.clamp3(
                &(-1.0 * bounds.half_extents.unitize_w()),
                &bounds.half_extents.unitize_w()
            );
            let clamped_p1 = p1.clamp3(
                &(-1.0 * bounds.half_extents.unitize_w()),
                &bounds.half_extents.unitize_w()
            );
            let segment = p1 - p0;
            let to_p0 = clamped_p0 - p0;

            let t = (to_p0.dot3(&segment) / segment.dot3(&segment)).clamp(0.0, 1.0);
            let point_on_segment = p0 + (segment * t);

            point_on_segment.clamp3(
                &(-1.0 * bounds.half_extents.unitize_w()),
                &bounds.half_extents.unitize_w()
            )
        };

        let closest_on_box = closest_point_on_obb_to_segment(
            a_bounds,
            &p0_local,
            &p1_local
        );

        let segment = p1_local - p0_local;
        let t = ((closest_on_box - p0_local).dot3(&segment) / segment.dot3(&segment))
            .clamp(0.0, 1.0);
        let closest_on_segment = p0_local + (segment * t);

        let diff = closest_on_segment - closest_on_box;
        let dist = diff.magnitude_3d();

        if dist > b_radius {
            return None;
        }

        let normal_local = if dist > 1e-6 {
            diff / dist
        } else {
            (closest_on_segment - Vector::new_vec3(0.0, 0.0, 0.0)).normalize_3d()
        };

        let penetration = b_radius - dist;

        let normal_world = (rot_mat * normal_local.unitize_w()).normalize_3d();
        let contact_point_local = closest_on_box + normal_local * (dist * 0.5);
        let contact_point_world = obb_center + (rot_mat * contact_point_local.unitize_w());

        Some(ContactInformation {
            normal: normal_world,
            points: vec![],
            penetration_depth: penetration,
        })
    }
    fn intersects_mesh(&self, mesh_body: &RigidBody) -> Option<Vec<ContactInformation>> {
        let mesh_collider = if let Hitbox::MESH(ref mesh) = mesh_body.hitbox {
            mesh
        } else {
            return None;
        };

        let rot_mat = Matrix::new_rotate_quaternion_vec4(&mesh_body.orientation);
        let rot_mat_inv = rot_mat.inverse4();

        let mut center_offset = Vector::new_vec3(0.0, 0.0, 0.0);
        let body_hitbox_mesh_space = match self.hitbox {
            Hitbox::MESH(ref mesh) => panic!("mesh-mesh collision not implemented"),
            Hitbox::OBB(ref obb) => {
                center_offset = obb.center;
                Hitbox::OBB(BoundingBox {
                center: Vector::new_vec3(0.0, 0.0, 0.0),
                half_extents: obb.half_extents / mesh_collider.current_scale_multiplier,
            })},
            Hitbox::CAPSULE(ref capsule) => Hitbox::CAPSULE(Capsule {
                a: capsule.a / mesh_collider.current_scale_multiplier,
                b: capsule.b / mesh_collider.current_scale_multiplier,
                radius: capsule.radius / mesh_collider.current_scale_multiplier,
            }),
        };
        let body_position_mesh_space = rot_mat_inv * ((self.position + center_offset - &mesh_body.position) / mesh_collider.current_scale_multiplier).unitize_w();
        let body_orientation_mesh_space = self.orientation.combine(&mesh_body.orientation.conjugate());


        let mut contact_infos = Self::split_into_bvh(
            &body_hitbox_mesh_space,
            &body_position_mesh_space,
            &body_orientation_mesh_space,
            &mesh_collider,
            &mesh_collider.bvh.borrow()
        );

        if let Some(infos) = &mut contact_infos {
            for info in infos.iter_mut() {
                info.normal = rot_mat * info.normal.unitize_w();
            }
            return contact_infos;
        }

        None
    }
    fn split_into_bvh(
        body_hitbox: &Hitbox,
        body_position: &Vector,
        body_orientation: &Vector,
        mesh_collider: &MeshCollider,
        bvh: &Bvh,
    ) -> Option<Vec<ContactInformation>> {
        let intersection = match body_hitbox {
            Hitbox::CAPSULE(capsule) => { Self::obb_intersects_capsule(
                &bvh.bounds, &Vector::new_empty_quat(), &Vector::new_empty_quat(),
                &capsule.a, &capsule.b, capsule.radius, &body_position, &body_orientation
            ) }
            Hitbox::MESH(mesh) => { panic!("mesh-obb collision not implemented"); }
            Hitbox::OBB(obb) => { Self::obb_intersects_obb(
                &obb, &bvh.bounds,
                &body_position, &Vector::new_empty_quat(),
                &body_orientation, &Vector::new_empty_quat()
            ) }
        };
        if intersection.is_some() {
            if let Some(triangle_indices) = &bvh.triangle_indices {
                let mut triangle_intersections = Vec::new();
                match body_hitbox {
                    Hitbox::OBB(obb) => {
                        for triangle_index in triangle_indices {
                            let triangle_intersection = Self::obb_intersects_triangle(
                                obb,
                                body_position,
                                body_orientation,
                                Bvh::get_triangle_vertices(&*mesh_collider.mesh.borrow(), *triangle_index, Some(mesh_collider.current_scale_factor))
                            );
                            if triangle_intersection.is_some() {
                                triangle_intersections.push(triangle_intersection.unwrap());
                            }
                        }
                    }
                    Hitbox::CAPSULE(capsule) => { panic!("capsule-triangle collision not implemented"); }
                    Hitbox::MESH(mesh) => { panic!("mesh-triangle collision not implemented"); }
                }

                return Some(triangle_intersections);
            };
            let mut left_intersections = if let Some(left) = &bvh.left_child {
                Self::split_into_bvh(
                    body_hitbox,
                    body_position,
                    body_orientation,
                    mesh_collider,
                    &left.borrow()
                ).unwrap_or(vec![])
            } else { vec![] };
            let mut right_intersections = if let Some(right) = &bvh.right_child {
                Self::split_into_bvh(
                    body_hitbox,
                    body_position,
                    body_orientation,
                    mesh_collider,
                    &right.borrow()
                ).unwrap_or(vec![])
            } else { vec![] };
            left_intersections.append(&mut right_intersections);
            return Some(left_intersections);
        }
        None
    }
    fn obb_intersects_triangle(
        obb: &BoundingBox,
        obb_position: &Vector,
        obb_orientation: &Vector,
        vertices: (Vertex, Vertex, Vertex),
    ) -> Option<ContactInformation> {
        let rot_mat = Matrix::new_rotate_quaternion_vec4(obb_orientation);

        let v0 = rot_mat.inverse4() * ((Vector::new_from_array(&vertices.0.position) - obb_position).unitize_w());
        let v1 = rot_mat.inverse4() * ((Vector::new_from_array(&vertices.1.position) - obb_position).unitize_w());
        let v2 = rot_mat.inverse4() * ((Vector::new_from_array(&vertices.2.position) - obb_position).unitize_w());

        let e0 = v1 - v0;
        let e1 = v2 - v1;
        let e2 = v0 - v2;

        let axes = [
            Vector::new_vec3(1.0, 0.0, 0.0),
            Vector::new_vec3(0.0, 1.0, 0.0),
            Vector::new_vec3(0.0, 0.0, 1.0),
        ];

        let normal = e0.cross(&e1).normalize_3d();

        let project = |axis: &Vector| -> (f32, f32) {
            let p0 = v0.dot3(axis);
            let p1 = v1.dot3(axis);
            let p2 = v2.dot3(axis);
            (p0.min(p1.min(p2)), p0.max(p1.max(p2)))
        };
        let project_bounds = |axis: &Vector| -> (f32, f32) {
            let r = obb.half_extents.x * axis.x.abs() + obb.half_extents.y * axis.y.abs() + obb.half_extents.z * axis.z.abs();
            (-r, r)
        };
        let test_axis = |axis: &Vector, min_penetration: &mut f32, best_axis: &mut Vector| -> bool {
            if axis.magnitude_3d() < 1e-6 { return true; }
            let axis = axis.normalize_3d();
            let (t_min, t_max) = project(&axis);
            let (b_min, b_max) = project_bounds(&axis);
            if t_max < b_min || t_min > b_max {
                return false;
            }
            let penetration = if t_min < b_min {
                (t_max - b_min).min(b_max - t_min)
            } else {
                (b_max - t_min).min(t_max - b_min)
            };
            if penetration < *min_penetration {
                *min_penetration = penetration;
                *best_axis = axis;
            }
            true
        };

        let mut min_penetration = f32::MAX;
        let mut best_axis = normal;
        for axis in &axes {
            if !test_axis(axis, &mut min_penetration, &mut best_axis) {
                return None;
            }
        }
        if !test_axis(&normal, &mut min_penetration, &mut best_axis) {
            return None;
        }
        for edge in &[e0, e1, e2] {
            for axis in &axes {
                let cross_axis = edge.cross(axis);
                if !test_axis(&cross_axis, &mut min_penetration, &mut best_axis) {
                    return None;
                }
            }
        }
        let tri_center = (v0 + v1 + v2) / 3.0;
        let closest_point = tri_center.clamp3(
            &(-1.0 * obb.half_extents),
            &obb.half_extents,
        );

        Some(ContactInformation {
            points: vec![],
            normal: rot_mat * (best_axis.unitize_w()),
            penetration_depth: min_penetration,
        })
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

        let point_on_b = a_center + collision_normal * (min_penetration * 0.5);
        let point_on_a = b_center - collision_normal * (min_penetration * 0.5);

        // TODO Multiple contact points
        Some(ContactInformation {
            points: vec![(point_on_a, point_on_b, collision_normal, min_penetration)],
            normal: collision_normal,
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
            mass: 999.0,
            inv_mass: 0.0,
            force: Default::default(),
            torque: Default::default(),
            position: Default::default(),
            velocity: Default::default(),
            orientation: Vector::new_empty_quat(),
            angular_velocity: Default::default(),
            center_of_mass: Vector::new_empty_quat(),
            inertia_tensor: Matrix::new(),
            inv_inertia_tensor: Matrix::new(),
        }
    }
}

struct CastInformation {
    distance: f32,
    contacts: Vec<ContactInformation>,
}
pub struct ContactInformation {
    points: Vec<(Vector, Vector, Vector, f32)>, // point on A, point on B, normal, depth
    normal: Vector,
    penetration_depth: f32,
}

pub enum Hitbox {
    OBB(BoundingBox),
    MESH(MeshCollider),
    CAPSULE(Capsule),
}
impl Clone for Hitbox {
    fn clone(&self) -> Self {
        match self {
            Hitbox::OBB(bounding_box) => Hitbox::OBB(bounding_box.clone()),
            Hitbox::MESH(collider) => Hitbox::MESH(collider.clone()),
            Hitbox::CAPSULE(capsule) => Hitbox::CAPSULE(capsule.clone()),
        }
    }
}
#[derive(Debug, Copy, Clone)]
pub struct Capsule {
    pub a: Vector,
    pub b: Vector,
    pub radius: f32,
}
#[derive(Debug, Copy, Clone)]
pub struct BoundingBox {
    pub center: Vector,
    pub half_extents: Vector,
}
impl BoundingBox {
    pub fn from_min_max(min: Vector, max: Vector) -> Self {
        BoundingBox {
            center: (min + max) * 0.5,
            half_extents: (max - min) * 0.5,
        }
    }
}
///* Does not support non-uniform scaling, as is the standard(?) with physics engines. Must call .rescale() to rescale the bvh to be nonuniform.
pub struct MeshCollider {
    pub mesh: Rc<RefCell<Mesh>>,
    pub current_scale_multiplier: f32,
    current_scale_factor: Vector,
    pub bvh: Rc<RefCell<Bvh>>,
}
impl MeshCollider {
    pub fn new(mesh: Rc<RefCell<Mesh>>, scale: Vector) -> Self {
        MeshCollider {
            mesh: mesh.clone(),
            current_scale_factor: scale,
            current_scale_multiplier: 1.0,
            bvh: Rc::new(RefCell::new(Bvh::new(mesh.clone(), scale))),
        }
    }

    pub fn rescale_bvh(&mut self, new_scale: Vector) {
        if self.bvh.borrow().active_scale_factor.equals(&new_scale, 1e-6) { return }
        self.bvh.borrow_mut().rescale_bvh_bounds(&new_scale, &self.current_scale_factor);
        self.current_scale_factor = new_scale;
    }
}
impl Clone for MeshCollider {
    fn clone(&self) -> Self {
        Self {
            mesh: self.mesh.clone(),
            current_scale_factor: self.current_scale_factor.clone(),
            current_scale_multiplier: self.current_scale_multiplier,
            bvh: self.bvh.clone()
        }
    }
}
pub struct Bvh {
    pub active_scale_factor: Vector,
    bounds: BoundingBox,
    left_child: Option<Rc<RefCell<Bvh>>>,
    right_child: Option<Rc<RefCell<Bvh>>>,
    triangle_indices: Option<Vec<usize>>,
}

impl Bvh {
    pub fn get_bounds_info(bvh: &Rc<RefCell<Bvh>>) -> Vec<(Vector, Vector)> { // centers, half extents
        let mut constants = Vec::new();
        Bvh::bounds_stack_add_bvh(&bvh, &mut constants);
        constants
    }
    fn bounds_stack_add_bvh(bvh: &Rc<RefCell<Bvh>>, constants: &mut Vec<(Vector, Vector)>) {
        let bvh = bvh.borrow();
        constants.push((
            bvh.bounds.center.clone(),
            bvh.bounds.half_extents.clone()
        ));
        if let Some(left_child) = &bvh.left_child {
            Bvh::bounds_stack_add_bvh(left_child, constants);
        }
        if let Some(right_child) = &bvh.right_child {
            Bvh::bounds_stack_add_bvh(right_child, constants);
        }
    }

    pub fn rescale_bvh_bounds(&mut self, new_scale: &Vector, old_scale: &Vector) {
        self.bounds.half_extents = (self.bounds.half_extents / old_scale) * new_scale;
        self.bounds.center = (self.bounds.center / old_scale) * new_scale;
        if let Some(left_child) = self.left_child.clone() {
            left_child.borrow_mut().rescale_bvh_bounds(&new_scale, &old_scale);
        }
        if let Some(right_child) = self.right_child.clone() {
            right_child.borrow_mut().rescale_bvh_bounds(&new_scale, &old_scale);
        }
        self.active_scale_factor = new_scale.clone();
    }

    pub fn new(mesh: Rc<RefCell<Mesh>>, scale: Vector) -> Bvh {
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
        Self::split(mesh, &mut triangles, 0, num_tris, &scale)
    }

    fn split(
        mesh: Rc<RefCell<Mesh>>,
        triangles: &mut [(usize, Vector)],
        start: usize,
        end: usize,
        scale: &Vector,
    ) -> Bvh {
        let (mut min, mut max) = Self::min_max(&mesh, triangles, start, end);
        min = min * scale; max = max * scale;
        let num_triangles = end - start;

        const MAX_LEAF_SIZE: usize = 4;

        if num_triangles <= MAX_LEAF_SIZE {
            let triangle_indices: Vec<usize> = triangles[start..end]
                .iter()
                .map(|(idx, _)| *idx)
                .collect();

            return Bvh {
                active_scale_factor: scale.clone(),
                bounds: BoundingBox::from_min_max(min, max),
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

        let left_child = Some(Rc::new(RefCell::new(Self::split(
            mesh.clone(),
            triangles,
            start,
            mid,
            scale
        ))));

        let right_child = Some(Rc::new(RefCell::new(Self::split(
            mesh.clone(),
            triangles,
            mid,
            end,
            scale
        ))));

        Bvh {
            active_scale_factor: scale.clone(),
            bounds: BoundingBox::from_min_max(min, max),
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
            let (v0, v1, v2) = Self::get_triangle_vertices(&mesh_borrow, *triangle_idx, None);

            min = Vector::min(&min, &Vector::new_from_array(&v0.position));
            min = Vector::min(&min, &Vector::new_from_array(&v1.position));
            min = Vector::min(&min, &Vector::new_from_array(&v2.position));

            max = Vector::max(&max, &Vector::new_from_array(&v0.position));
            max = Vector::max(&max, &Vector::new_from_array(&v1.position));
            max = Vector::max(&max, &Vector::new_from_array(&v2.position));
        }

        (min, max)
    }

    fn get_triangle_vertices(mesh: &Mesh, triangle_index: usize, scale_factor: Option<Vector>) -> (Vertex, Vertex, Vertex) {
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

        let mut v0 = primitive.vertex_data[idx0].clone();
        let mut v1 = primitive.vertex_data[idx1].clone();
        let mut v2 = primitive.vertex_data[idx2].clone();

        if let Some(scale) = scale_factor {
            v0.position = (Vector::new_from_array(&v0.position) * scale).to_array3();
            v0.normal = (Vector::new_from_array(&v0.normal) * scale).normalize_3d().to_array3();

            v1.position = (Vector::new_from_array(&v1.position) * scale).to_array3();
            v1.normal = (Vector::new_from_array(&v1.normal) * scale).normalize_3d().to_array3();

            v2.position = (Vector::new_from_array(&v2.position) * scale).to_array3();
            v2.normal = (Vector::new_from_array(&v2.normal) * scale).normalize_3d().to_array3();
        }

        (v0, v1, v2)
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
impl Clone for Bvh {
    fn clone(&self) -> Self {
        Self {
            active_scale_factor: self.active_scale_factor.clone(),
            bounds: self.bounds.clone(),
            left_child: self.left_child.clone(),
            right_child: self.right_child.clone(),
            triangle_indices: self.triangle_indices.clone(),
        }
    }
}