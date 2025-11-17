use crate::math::Vector;

#[derive(Debug, Copy, Clone)]
pub struct BoundingBox {
    pub center: Vector,
    pub half_extents: Vector,
}
impl BoundingBox {
    pub fn from_min_max(min: Vector, max: Vector) -> Self {
        BoundingBox {
            center: (min + max) * 0.5,
            half_extents: (max - min) * 0.5,
        }
    }
}