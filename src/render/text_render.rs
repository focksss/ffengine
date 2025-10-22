use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use ash::vk;
use crate::engine::scene::Texture;
use crate::math::Vector;
use crate::render::*;

const OUTPUT_DIR: &str = "resources\\fonts\\generated";

pub struct Font {
    pub texture: Texture,
    pub sampler: vk::Sampler,
    pub glyphs: HashMap<char, Glyph>,
    pub atlas_size: Vector,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}
impl Font {
    pub fn new(base: &mut VkBase, path: &str) {
        let generated_path = path.replace("fonts", "fonts\\generated");
        let atlas_path_str = generated_path.replace(".ttf", ".png");
        let json_path_str = generated_path.replace(".ttf", ".json");

        Command::new("resources\\msdf-atlas-gen.exe") // https://github.com/Chlumsky/msdf-atlas-gen/tree/v1.3
            .args(&[
                "-font", path,
                "-imageout", &atlas_path_str,
                "-json", &json_path_str,
                "-size", "64" // base pixel size for glyphs
            ])
            .status()
            .expect("Failed to run msdfgen");
    }
}
struct Glyph {
    pub uv_min: Vector,
    pub uv_max: Vector,
    pub plane_min: Vector,
    pub plane_max: Vector,
    pub advance: f32,
}



