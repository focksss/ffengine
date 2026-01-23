use crate::math::Vector;

pub struct Texture {
    texture_set: Vec<Texture>,
    index: usize,

    additive_tint: Vector,
    multiplicative_tint: Vector,
    corner_radius: f32,
    aspect_ratio: Option<f32>,
}