use crate::math::Vector;

pub struct Quad {
    color: Vector,
    corner_radius: f32,
}
impl Default for Quad {
    fn default() -> Self {
        Self {
            color: Vector::fill(1.0),
            corner_radius: 5.0,
        }
    }
}