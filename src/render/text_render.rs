use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use ash::vk;
use ash::vk::{DeviceMemory, Image, ImageView, Sampler};
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

        let atlas: ((ImageView, Sampler), (Image, DeviceMemory), u32) = base.create_2d_texture_image(&PathBuf::from(atlas_path_str), false);
        let atlas_texture = Texture {
            image: atlas.1.0,
            image_view: atlas.0.0,
            device_memory: atlas.1.1,
            clear_value: vk::ClearValue::default(),
            format: vk::Format::R16G16B16A16_SFLOAT,
            resolution: vk::Extent3D::default(),
            array_layers: 1,
            samples: base.msaa_samples,
            is_depth: false,
        };

        let glyph_data: HashMap<char, Glyph> = {
            let file = std::fs::File::open(json_path_str).unwrap();
            let json: serde_json::Value = serde_json::from_reader(file).unwrap();
            let mut map = HashMap::new();
            for (c, g) in json["glyphs"].as_object().unwrap() {
                let ch = c.chars().next().unwrap();
                map.insert(ch, Glyph {
                    uv_min: Vector::new_vec2(g["uv_min"][0].as_f64().unwrap() as f32, g["uv_min"][1].as_f64().unwrap() as f32),
                    uv_max: Vector::new_vec2(g["uv_max"][0].as_f64().unwrap() as f32, g["uv_max"][1].as_f64().unwrap() as f32),
                    plane_min: Vector::new_vec2(g["plane_min"][0].as_f64().unwrap() as f32, g["plane_min"][1].as_f64().unwrap() as f32),
                    plane_max: Vector::new_vec2(g["plane_max"][0].as_f64().unwrap() as f32, g["plane_max"][1].as_f64().unwrap() as f32),
                    advance: g["advance"].as_f64().unwrap() as f32,
                });
            }
            map
        };
        Font {
            texture: atlas_texture,
            sampler: atlas.0.1,
            glyphs: glyph_data,
            atlas_size: Vector::new_vec(0.0),
            ascent: 0.0,
            descent: 0.0,
            line_gap: 0.0,
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



