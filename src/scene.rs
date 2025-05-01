use crate::vector::Vector;

pub struct Camera {
    position: Vector,
    target: Vector,
    rotation: Vector,
    fov_y: f32,
    targeting_mode: bool,
}
impl Camera {
    pub fn new(position: &Vector, target: &Vector, rotation: &Vector, fov_y: f32) -> Self {
        Self {
            position: *position,
            target: *target,
            rotation: *rotation,
            fov_y,
            targeting_mode: !target.null,
        }
    }
}