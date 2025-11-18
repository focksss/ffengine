use crate::math::Vector;

#[derive(Debug, Copy, Clone)]
pub struct Sphere {
    pub center: Vector,
    pub radius: f32,
}