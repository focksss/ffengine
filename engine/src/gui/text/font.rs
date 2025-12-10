use std::collections::HashMap;
use std::fs;
use std::ops::Add;
use std::path::PathBuf;
use std::process::Command;
use ash::vk;
use ash::vk::{Format, SampleCountFlags, Sampler};
use serde_json::Value;
use crate::gui::text::glyph::Glyph;
use crate::math::Vector;
use crate::render::render_helper::{DeviceTexture, Texture};
use crate::render::vulkan_base::VkBase;

pub struct Font {
    pub device: ash::Device,

    pub texture: DeviceTexture,
    pub sampler: Sampler,
    pub glyphs: HashMap<char, Glyph>,
    pub atlas_size: Vector,
    pub glyph_size: u32,
    pub distance_range: f32,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}
impl Font {
    pub fn new(base: &VkBase, path: &str, glyph_size: Option<u32>, distance_range: Option<f32>) -> Self { unsafe {
        let glyph_size_final = glyph_size.unwrap_or(64);
        let distance_range_final = distance_range.unwrap_or(2.0);
        let font_name = PathBuf::from(path).file_name()
            .expect("Font path must be to a named file")
            .to_str().unwrap().to_string()
            .replace(".ttf", "");
        let generated_path = path.replace("fonts", &*("fonts\\generated\\".to_string().add(font_name.as_str())));
        if let Some(parent_dir) = PathBuf::from(&generated_path).parent() {
            if let Err(e) = fs::create_dir_all(parent_dir) {
                eprintln!("Failed to create directory {:?}: {}", parent_dir, e);
            }
        }

        let atlas_path_str = generated_path.replace(".ttf", ".png");
        let json_path_str = generated_path.replace(".ttf", ".json");

        Command::new("engine\\resources\\msdf-atlas-gen.exe") // https://github.com/Chlumsky/msdf-atlas-gen/tree/v1.3
            .args(&[
                "-font", path,
                "-imageout", &atlas_path_str,
                "-json", &json_path_str,
                "-size", glyph_size_final.to_string().as_str(), // base pixel size for glyphs
                "-pxrange", distance_range_final.to_string().as_str() // width of SDF distance range in output pixels
            ])
            .status()
            .expect("Failed to run msdfgen");

        let atlas = base.create_2d_texture_image(&PathBuf::from(atlas_path_str), false);
        let atlas_texture = DeviceTexture {
            image: atlas.1.0,
            image_view: atlas.0.0,
            stencil_image_view: None,
            device_memory: atlas.1.1,

            destroyed: false
        };

        let file = fs::File::open(json_path_str).unwrap();
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
                    Vector::new2(bounds["left"].as_f64().unwrap_or(0.0) as f32, bounds["bottom"].as_f64().unwrap_or(0.0) as f32),
                    Vector::new2(bounds["right"].as_f64().unwrap_or(0.0) as f32, bounds["top"].as_f64().unwrap_or(0.0) as f32)
                )
            } else {
                (Vector::new2(0.0, 0.0), Vector::new2(0.0, 0.0))
            };

            let (uv_min, uv_max) = if let Some(bounds) = glyph.get("atlasBounds") {
                let l = bounds["left"].as_f64().unwrap_or(0.0) as f32 / atlas_w;
                let r = bounds["right"].as_f64().unwrap_or(0.0) as f32 / atlas_w;
                let b = bounds["bottom"].as_f64().unwrap_or(0.0) as f32 / atlas_h;
                let t = bounds["top"].as_f64().unwrap_or(0.0) as f32 / atlas_h;
                (Vector::new2(l, 1.0 - b), Vector::new2(r, 1.0 - t))
            } else {
                (Vector::new2(0.0, 0.0), Vector::new2(0.0, 0.0))
            };

            glyphs.insert(ch, Glyph { uv_min, uv_max, plane_min, plane_max, advance });
        }

        Font {
            device: base.device.clone(),

            texture: atlas_texture,
            sampler: atlas.0.1,
            glyphs,
            atlas_size: Vector::new2(atlas_w, atlas_h),
            glyph_size: glyph_size_final,
            distance_range: distance_range_final,
            ascent,
            descent,
            line_gap,
        }
    } }
    pub unsafe fn destroy(&self) { unsafe {
        self.device.destroy_image(self.texture.image, None);
        self.device.free_memory(self.texture.device_memory, None);
        self.device.destroy_image_view(self.texture.image_view, None);

        self.device.destroy_sampler(self.sampler, None);
    } }
}