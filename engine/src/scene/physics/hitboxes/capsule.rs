use crate::math::Vector;

#[derive(Debug, Copy, Clone)]
pub struct Capsule {
    pub a: Vector,
    pub b: Vector,
    pub radius: f32,
}