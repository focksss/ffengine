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


    pub fn tick(&mut self, delta_time: f32, world: &mut Scene) {
        for body in &mut self.rigid_bodies {
            if body.is_static {
                body.update_this_according_to_coupled(world);
            } else {
                body.update_coupled_according_to_this(world);
            }
        }
        // apply gravity
        for body in &mut self.rigid_bodies {
            if body.is_static {
                continue;
            } else {
                body.apply_impulse(self.gravity * delta_time * body.mass, body.get_center_of_mass_world_space());
            }
        }
        // collision detection
        let num_bodies = self.rigid_bodies.len();
        let mut contacts = Vec::new();
        for i in 0..num_bodies {
            for j in i + 1..num_bodies {
                let (a, b) = self.rigid_bodies.split_at_mut(j);
                let body_a = &mut a[i];
                let body_b = &mut b[0];

                if body_a.is_static && body_b.is_static {
                    continue;
                }

                if let Some(contact) = body_a.will_collide_with(body_b, delta_time) {
                    if !contact.contact_points.is_empty() {
                        contacts.push((contact, (i, j)));
                    }
                }
            }
        }
        contacts.sort_by(|a, b| a.0.time_of_impact.partial_cmp(&b.0.time_of_impact).unwrap());
        let mut accum_time = 0.0;
        for contact in &contacts {
            let (i, j) = contact.1;

            let collision = &contact.0;
            let dt = collision.time_of_impact - accum_time;

            for body in &mut self.rigid_bodies {
                if !body.is_static {
                    body.update(dt);
                }
            }

            let (first_idx, second_idx) = if i < j { (i, j) } else { (j, i) };
            let (left, right) = self.rigid_bodies.split_at_mut(second_idx);
            let body_a = &mut left[first_idx];
            let body_b = &mut right[0];


            let normal = collision.normal;
            let deepest = collision.contact_points.iter().max_by(
                |a_point, b_point|
                    a_point.penetration.partial_cmp(&b_point.penetration).unwrap()
            ).unwrap();
            let depth = deepest.penetration;
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

            if collision.time_of_impact == 0.0 {
                let t_a = im_a / s_im;
                let t_b = im_b / s_im;

                let ds = pt_on_b - pt_on_a;
                body_a.position += ds * t_a;
                body_b.position -= ds * t_b;
            }
            accum_time += dt;
        }
        // apply velocity
        for body in &mut self.rigid_bodies {
            if !body.is_static {
                body.update(delta_time - accum_time);
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
            let mut contacts = Vec::new();
            {
                let rigid_body_index = self.players[player_index].rigid_body_pointer.index;
                let (head, tail) = self.rigid_bodies.split_at_mut(rigid_body_index);
                let (rb_slice, after) = tail.split_at_mut(1);
                let rigid_body = &mut rb_slice[0];
                for other_body in head.iter_mut().chain(after.iter_mut()) {
                    if let Some(new_collisions) =
                        rigid_body.will_collide_with(other_body, delta_time)
                    {
                        contacts.push(new_collisions);
                    }
                }
            }
            let player_body = &self.rigid_bodies[self.players[player_index].rigid_body_pointer.index];
            if contacts.is_empty() {
                break;
            }

            let deepest = contacts.into_iter().max_by(|a, b| {
                let max_a = a.contact_points
                    .iter()
                    .map(|p| p.penetration)
                    .fold(0.0, f32::max);
                let max_b = b.contact_points
                    .iter()
                    .map(|p| p.penetration)
                    .fold(0.0, f32::max);
                max_a.partial_cmp(&max_b).unwrap()
            }).unwrap();

            let mut normal = deepest.normal.normalize_3d();
            if normal.dot3(&player_body.velocity) > 0.0 {
                normal = normal * -1.0;
            }

            let player = &mut self.players[player_index];
            let rigid_body = &mut self.rigid_bodies[player.rigid_body_pointer.index];
            if normal.dot3(&self.gravity.normalize_3d()) < -0.35 {
                player.grounded = true;
            }

            let depenetration = normal * (deepest.contact_points[0].penetration);
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
pub enum AxisType {
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
    pub time_of_impact: f32,
    pub normal: Vector,
}
impl ContactInformation {
    pub fn flip(mut self) -> ContactInformation {
        for point in &mut self.contact_points { point.flip(); }
        self.normal = -self.normal;
        self
    }
}
#[derive(Debug)]
pub struct ContactPoint {
    pub point_on_a: Vector,
    pub point_on_b: Vector,

    pub penetration: f32,
}
impl ContactPoint {
    fn flip(&mut self) {
        let temp = self.point_on_b;
        self.point_on_b = self.point_on_a;
        self.point_on_a = temp;
    }
}