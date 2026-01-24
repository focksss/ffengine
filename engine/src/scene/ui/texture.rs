use crate::math::Vector;

pub struct Texture {
    texture_set: Vec<Texture>,
    pub(crate) index: usize,

    pub(crate) additive_tint: Vector,
    pub(crate) multiplicative_tint: Vector,
    pub(crate) corner_radius: f32,
    pub(crate) aspect_ratio: Option<f32>,
}