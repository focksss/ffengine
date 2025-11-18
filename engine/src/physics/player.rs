use crate::world::camera::Camera;
use crate::math::Vector;
use crate::physics::hitboxes::bounding_box::BoundingBox;
use crate::physics::hitboxes::hitbox::Hitbox;
use crate::physics::physics_engine::RigidBody;

#[derive(Copy, Clone, Debug)]
pub enum MovementMode {
    GHOST,
    PHYSICS,
}

pub struct Player {
    pub movement_mode: MovementMode,
    pub move_power: f32,
    pub jump_power: f32,
    pub skin_width: f32,
    pub grounded: bool,
    pub rigid_body: RigidBody,
    pub camera: Camera,
}
impl Player {
    pub fn new(camera: Camera, eye_to_foot: Vector, eye_to_head: Vector, movement_mode: MovementMode, move_power: f32, jump_power: f32, skin_width: f32) -> Self {
        let mut rigid_body = RigidBody::default();
        let max = camera.position + eye_to_head;
        let min = camera.position + eye_to_foot;
        let hitbox_height = max.y - min.y;
        let radius = (Vector::new_vec3(max.x, 0.0, max.z) - Vector::new_vec3(min.x, 0.0, min.z)).magnitude_3d();
        // /*
        rigid_body.hitbox = Hitbox::OBB(BoundingBox {
             half_extents: (max - min) * 0.5,
             center: Vector::new_vec3(0.0, -hitbox_height * 0.5 + eye_to_head.y, 0.0),
        });
        // */
        /*
        rigid_body.hitbox = Hitbox::SPHERE(Sphere {
            center: Vector::new_vec3(0.0, -hitbox_height * 0.5 + eye_to_head.y, 0.0),
            radius,
        });
         */
        /*
        rigid_body.hitbox = Hitbox::CAPSULE(Capsule {
             a: Vector::new_vec3(0.0, eye_to_foot.y, 0.0),
             b: Vector::new_vec3(0.0, eye_to_head.y, 0.0),
             radius,
        });
        */
        rigid_body.position = camera.position;
        rigid_body.friction_coefficient = 0.0;

        Player {
            movement_mode,
            move_power,
            jump_power,
            skin_width,
            grounded: false,
            rigid_body,
            camera,
        }
    }
    pub fn step(&mut self, step: &Vector) {
        self.camera.position = self.camera.position + step;
        self.rigid_body.position = self.camera.position;
    }
}