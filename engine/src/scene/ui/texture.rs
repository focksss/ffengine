use crate::math::Vector;
use crate::render::render_helper::Texture as RenderTexture;

pub struct Texture {
    pub texture_set: Vec<RenderTexture>,
    pub index: usize,

    pub additive_tint: Vector,
    pub multiplicative_tint: Vector,
    pub corner_radius: f32,
    pub aspect_ratio: Option<f32>,
}