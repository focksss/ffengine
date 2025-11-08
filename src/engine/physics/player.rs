use crate::engine::physics::physics_engine::{BoundingBox, Hitbox, RigidBody};
use crate::engine::world::camera::Camera;
use crate::math::Vector;

#[derive(Copy, Clone)]
pub enum MovementMode {
    GHOST,
    PHYSICS,
}

pub struct Player {
    pub movement_mode: MovementMode,
    pub grounded: bool,
    pub rigid_body: RigidBody,
    pub camera: Camera,
}
impl Player {
    pub fn new(camera: Camera, eye_to_foot: Vector, eye_to_head: Vector, movement_mode: MovementMode) -> Self {
        let mut rigid_body = RigidBody::default();
        let max = camera.position + eye_to_head;
        let min = camera.position + eye_to_foot;
        let hitbox_height = max.y - min.y;
        rigid_body.hitbox = Hitbox::OBB(BoundingBox {
            half_extents: (max - min) * 0.5,
            center: Vector::new_vec3(0.0, -hitbox_height * 0.5 + eye_to_head.y, 0.0),
        });
        rigid_body.position = camera.position;
        rigid_body.friction_coefficient = 0.0;

        Player {
            movement_mode,
            grounded: false,
            rigid_body,
            camera,
        }
    }
    pub fn step(&mut self, step: Vector) {
        self.camera.position = self.camera.position + step;
        self.rigid_body.position = self.camera.position;
    }
}