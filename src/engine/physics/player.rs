use crate::engine::physics::physics_engine::{Obb, Hitbox, RigidBody};
use crate::engine::world::camera::Camera;
use crate::math::Vector;

pub struct Player {
    pub rigid_body: RigidBody,
    pub camera: Camera,
}
impl Player {
    pub fn new(camera: Camera, eye_to_foot: Vector, eye_to_head: Vector) -> Self {
        let mut rigid_body = RigidBody::default();
        let max = camera.position + eye_to_head;
        let min = camera.position + eye_to_foot;
        let hitbox_height = max.y - min.y;
        rigid_body.hitbox = Hitbox::OBB(Obb {
            half_extents: (max - min) * 0.5,
            orientation: Vector::new_empty_quat(),
            center: Vector::new_vec3(camera.position.x, camera.position.y - hitbox_height * 0.5 + eye_to_head.y, camera.position.z),
        });
        rigid_body.position = camera.position;

        Player {
            rigid_body,
            camera,
        }
    }
    pub fn step(&mut self, step: Vector) {
        self.camera.position = self.camera.position + step;
        self.rigid_body.position = self.camera.position;
    }
}