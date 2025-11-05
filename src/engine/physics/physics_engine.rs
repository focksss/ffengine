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
                        half_extents: (max - min) * 0.5,
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
            if body.is_static { continue; }

            let fg = self.gravity * body.mass;
        }
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

    pub fn colliding_with(&self, other: &RigidBody) -> bool {
        false
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
pub enum Hitbox {
    OBB(Obb),
}
#[derive(Debug)]
pub struct Obb {
    pub center: Vector,
    pub half_extents: Vector,
    pub orientation: Vector, // quaternion
}