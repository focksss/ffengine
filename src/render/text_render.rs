use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use ash::vk;
use ash::vk::{DeviceMemory, Image, ImageView, Sampler};
use serde_json::Value;
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
    pub unsafe fn new(base: &mut VkBase, path: &str) -> Self { unsafe {
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

        let atlas = base.create_2d_texture_image(&PathBuf::from(atlas_path_str), false);
        let atlas_texture = Texture {
            image: atlas.1.0,
            image_view: atlas.0.0,
            device_memory: atlas.1.1,
            clear_value: vk::ClearValue::default(),
            format: vk::Format::R8G8B8A8_UNORM,
            resolution: vk::Extent3D::default(),
            array_layers: 1,
            samples: base.msaa_samples,
            is_depth: false,
        };

        let file = std::fs::File::open(json_path_str).unwrap();
        let json: Value = serde_json::from_reader(file).unwrap();

        let atlas_w = json["atlas"]["width"].as_f64().unwrap() as f32;
        let atlas_h = json["atlas"]["height"].as_f64().unwrap() as f32;

        let metrics = &json["metrics"];
        let ascent = metrics["ascender"].as_f64().unwrap() as f32;
        let descent = metrics["descender"].as_f64().unwrap() as f32;
        let line_gap = metrics["lineHeight"].as_f64().unwrap() as f32 - (ascent - descent);

        let mut glyphs = HashMap::new();
        for glyph in json["glyphs"].as_array().unwrap() {
            let unicode = glyph["unicode"].as_u64().unwrap();
            let ch = std::char::from_u32(unicode as u32).unwrap_or('?');
            let advance = glyph["advance"].as_f64().unwrap_or(0.0) as f32;

            let (plane_min, plane_max) = if let Some(bounds) = glyph.get("planeBounds") {
                (
                    Vector::new_vec2(bounds["left"].as_f64().unwrap_or(0.0) as f32, bounds["bottom"].as_f64().unwrap_or(0.0) as f32),
                    Vector::new_vec2(bounds["right"].as_f64().unwrap_or(0.0) as f32, bounds["top"].as_f64().unwrap_or(0.0) as f32)
                )
            } else {
                (Vector::new_vec2(0.0, 0.0), Vector::new_vec2(0.0, 0.0))
            };

            let (uv_min, uv_max) = if let Some(bounds) = glyph.get("atlasBounds") {
                let l = bounds["left"].as_f64().unwrap_or(0.0) as f32 / atlas_w;
                let r = bounds["right"].as_f64().unwrap_or(0.0) as f32 / atlas_w;
                let b = bounds["bottom"].as_f64().unwrap_or(0.0) as f32 / atlas_h;
                let t = bounds["top"].as_f64().unwrap_or(0.0) as f32 / atlas_h;
                (Vector::new_vec2(l, b), Vector::new_vec2(r, t))
            } else {
                (Vector::new_vec2(0.0, 0.0), Vector::new_vec2(0.0, 0.0))
            };

            glyphs.insert(ch, Glyph { uv_min, uv_max, plane_min, plane_max, advance });
        }

        Font {
            texture: atlas_texture,
            sampler: atlas.0.1,
            glyphs,
            atlas_size: Vector::new_vec2(atlas_w, atlas_h),
            ascent,
            descent,
            line_gap,
        }
    } }
}
struct Glyph {
    pub uv_min: Vector,
    pub uv_max: Vector,
    pub plane_min: Vector,
    pub plane_max: Vector,
    pub advance: f32,
}



