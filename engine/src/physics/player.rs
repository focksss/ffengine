use std::cell::RefCell;
use std::sync::Arc;
use crate::world::camera::{Camera, CameraPointer};
use crate::math::Vector;
use crate::physics::hitboxes::bounding_box::BoundingBox;
use crate::physics::hitboxes::convex_hull::ConvexHull;
use crate::physics::hitboxes::hitbox::Hitbox;
use crate::physics::hitboxes::sphere::Sphere;
use crate::physics::physics_engine::{PhysicsEngine, RigidBody};
use crate::physics::rigid_body::RigidBodyPointer;
use crate::world::scene::World;

#[derive(Copy, Clone, Debug)]
pub enum MovementMode {
    GHOST,
    PHYSICS,
    EDITOR,
}
#[derive(Clone)]
pub struct PlayerPointer {
    pub physics_engine: Arc<RefCell<PhysicsEngine>>,
    pub index: usize,
}
pub struct Player {
    pub movement_mode: MovementMode,
    pub move_power: f32,
    pub jump_power: f32,
    pub skin_width: f32,
    pub grounded: bool,
    pub fly_speed: f32,
    pub rigid_body_pointer: RigidBodyPointer,
    pub camera_pointer: CameraPointer,
}
impl Player {
    pub fn new(physics_engine: Arc<RefCell<PhysicsEngine>>, world: Arc<RefCell<World>>, camera: Camera, eye_to_foot: Vector, eye_to_head: Vector, movement_mode: MovementMode, move_power: f32, jump_power: f32, skin_width: f32) -> Self {
        let mut rigid_body = RigidBody::default();
        rigid_body.owned_by_player = true;
        let max = camera.position + eye_to_head;
        let min = camera.position + eye_to_foot;
        let hitbox_height = max.y - min.y;
        let radius = (Vector::new3(max.x, 0.0, max.z) - Vector::new3(min.x, 0.0, min.z)).magnitude3();
        /*
        rigid_body.hitbox = Hitbox::OBB(BoundingBox {
             half_extents: (max - min) * 0.5,
             center: Vector::new3(0.0, -hitbox_height * 0.5 + eye_to_head.y, 0.0),
        }, ConvexHull {
            points: Vec::new(),
            triangle_vert_indices: Vec::new(),
            min_max: (min, max),
        });
        */
        // /*
        rigid_body.hitbox = Hitbox::Sphere(Sphere {
            center: Vector::new3(0.0, -hitbox_height * 0.5 + eye_to_head.y, 0.0),
            radius,
        });
        // */
        /*
        rigid_body.hitbox = Hitbox::CAPSULE(Capsule {
             a: Vector::new_vec3(0.0, eye_to_foot.y, 0.0),
             b: Vector::new_vec3(0.0, eye_to_head.y, 0.0),
             radius,
        });
        */
        rigid_body.position = camera.position;
        rigid_body.friction_coefficient = 0.0;

        let rb_index = physics_engine.borrow().rigid_bodies.len();
        physics_engine.borrow_mut().rigid_bodies.push(rigid_body);
        let cam_index = world.borrow().cameras.len();
        world.borrow_mut().add_camera(camera);

        Player {
            movement_mode,
            move_power,
            jump_power,
            skin_width,
            grounded: false,
            fly_speed: 1.0,
            rigid_body_pointer: RigidBodyPointer {
                physics_engine,
                index: rb_index,
            },
            camera_pointer: CameraPointer {
                world,
                index: cam_index
            },
        }
    }
}