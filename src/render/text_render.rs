use crate::{mem};
use std::collections::HashMap;
use std::{fs, slice};
use std::ffi::c_void;
use std::ops::Add;
use std::path::PathBuf;
use std::process::Command;
use ash::vk;
use std::ptr::null_mut;
use ash::vk::{CommandBuffer, DescriptorType, DeviceMemory, Format, Handle, SampleCountFlags, Sampler, ShaderStageFlags};
use serde_json::{Value};
use crate::{offset_of, MAX_FRAMES_IN_FLIGHT};
use crate::math::Vector;
use crate::render::*;

const OUTPUT_DIR: &str = "resources\\fonts\\generated";

pub struct TextRenderer {
    pub renderpass: Renderpass,
    sampler: Sampler,
}
impl TextRenderer {
    pub unsafe fn new(base: &VkBase) -> TextRenderer { unsafe {
        //<editor-fold desc = "pass">
        let color_tex_create_info = TextureCreateInfo::new(base).format(Format::R8G8B8A8_UNORM);
        let pass_create_info = PassCreateInfo::new(base)
            .frames_in_flight(MAX_FRAMES_IN_FLIGHT)
            .add_color_attachment_info(color_tex_create_info.add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC));
        //</editor-fold>
        //<editor-fold desc = "descriptor set">
        let texture_sampler_create_info = DescriptorCreateInfo::new(base)
            .frames_in_flight(MAX_FRAMES_IN_FLIGHT)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT)
            .binding_flags(vk::DescriptorBindingFlags::UPDATE_AFTER_BIND);
        let descriptor_set_create_info = DescriptorSetCreateInfo::new(base)
            .frames_in_flight(MAX_FRAMES_IN_FLIGHT)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info));
        //</editor-fold>
        //<editor-fold desc = "graphics pipeline initiation">
        let push_constant_range = vk::PushConstantRange {
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            offset: 0,
            size: size_of::<TextPushConstants>() as _,
        };
        let vertex_input_binding_descriptions = [
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: size_of::<GlyphQuadVertex>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            }, // vertex
        ];
        let vertex_input_attribute_descriptions = [
            // vertex
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: Format::R32G32B32_SFLOAT,
                offset: offset_of!(GlyphQuadVertex, position) as u32,
            }, // position
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: Format::R32G32_SFLOAT,
                offset: offset_of!(GlyphQuadVertex, uv) as u32,
            }, // uv
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(GlyphQuadVertex, color) as u32,
            }, // color
        ];
        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);
        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };
        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::TRUE,
            src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        }];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(&color_blend_attachment_states);

        let renderpass_create_info = RenderpassCreateInfo::new(base)
            .pass_create_info(pass_create_info)
            .descriptor_set_create_info(descriptor_set_create_info)
            .vertex_shader_uri(String::from("text\\text.vert.spv"))
            .fragment_shader_uri(String::from("text\\text.frag.spv"))
            .push_constant_range(push_constant_range)
            .pipeline_input_assembly_state(vertex_input_assembly_state_info)
            .pipeline_vertex_input_state(vertex_input_state_info)
            .pipeline_color_blend_state_create_info(color_blend_state);
        let renderpass = Renderpass::new(renderpass_create_info);

        //</editor-fold>
        let sampler = base.device.create_sampler(&vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            ..Default::default()
        }, None).unwrap();
        TextRenderer {
            renderpass,
            sampler,
        }
    } }
    pub unsafe fn destroy(&self, base: &VkBase) { unsafe {
        self.renderpass.destroy(base);
        base.device.destroy_sampler(self.sampler, None);
    } }

    // TODO() store refs to base device and command buffers, instead of sending base ref in all method calls.
    pub unsafe fn render_text(&self, base: &VkBase, frame: usize, text_info: &TextInformation) { unsafe {
        let font = text_info.font;
        let frame_command_buffer = base.draw_command_buffers[frame];
        let device = &base.device;
        // <editor-fold desc = "descriptor updates">
        for current_frame in 0..MAX_FRAMES_IN_FLIGHT {
            let image_info = vk::DescriptorImageInfo {
                sampler: self.sampler,
                image_view: font.texture.image_view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            };
            let descriptor_write = vk::WriteDescriptorSet::default()
                .dst_set(self.renderpass.descriptor_set.descriptor_sets[current_frame])
                .dst_binding(0u32)
                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(slice::from_ref(&image_info));
            base.device.update_descriptor_sets(&[descriptor_write], &[]);
        }
        //</editor-fold>
        
        self.renderpass.do_renderpass(base, current_frame, frame_command_buffer, Some(|| {
            device.cmd_push_constants(frame_command_buffer, self.renderpass.pipeline_layout, ShaderStageFlags::ALL_GRAPHICS, 0, slice::from_raw_parts(
                &TextPushConstants {
                    clip_min: Vector::new_vec(0.0).to_array2(),
                    clip_max: Vector::new_vec(1.0).to_array2(),
                    position: text_info.position.to_array2(),
                    resolution: [base.surface_resolution.width as i32, base.surface_resolution.height as i32],
                    glyph_size: font.glyph_size,
                    distance_range: font.distance_range,
                    _pad: [0; 2]
                } as *const TextPushConstants as *const u8,
                size_of::<TextPushConstants>(),
            ))
        }), Some(|| {
            device.cmd_bind_vertex_buffers(
                frame_command_buffer,
                0,
                &[text_info.vertex_buffer[frame].0],
                &[0],
            );
            device.cmd_bind_index_buffer(
                frame_command_buffer,
                text_info.index_buffer[frame].0,
                0,
                vk::IndexType::UINT32,
            );
            device.cmd_draw_indexed(
                frame_command_buffer,
                text_info.glyph_count * 6u32,
                1,
                0u32,
                0,
                0,
            )
        }))
    } }
}
pub struct TextInformation<'a> {
    font: &'a Font,
    text: String,
    position: Vector,
    font_size: f32,
    scale_vector: Vector,
    color: Vector,
    auto_wrap_distance: f32,
    bold: bool,
    italic: bool,

    pub glyph_count: u32,
    pub vertex_buffer: Vec<(vk::Buffer, DeviceMemory)>,
    pub vertex_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
    pub index_buffer: Vec<(vk::Buffer, DeviceMemory)>,
    pub index_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
}
impl<'a> TextInformation<'a> {
    pub fn new(font: &'a Font) -> TextInformation<'a> {
        TextInformation {
            font,
            text: String::new(),
            position: Vector::new_empty(),
            font_size: 0.1,
            scale_vector: Vector::new_vec(1.0),
            color: Vector::new_vec(1.0),
            auto_wrap_distance: 20.0,
            bold: false,
            italic: false,

            glyph_count: 0,
            vertex_buffer: vec![(vk::Buffer::null(), DeviceMemory::null()); MAX_FRAMES_IN_FLIGHT],
            vertex_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
            index_buffer: vec![(vk::Buffer::null(), DeviceMemory::null()); MAX_FRAMES_IN_FLIGHT],
            index_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
        }
    }
    pub fn destroy(&mut self, base: &VkBase) { unsafe {
        for buffer in self.vertex_buffer.iter() {
            base.device.destroy_buffer(buffer.0, None);
            base.device.free_memory(buffer.1, None);
        }
        for buffer in self.index_buffer.iter() {
            base.device.destroy_buffer(buffer.0, None);
            base.device.free_memory(buffer.1, None);
        }
        base.device.destroy_buffer(self.index_staging_buffer.0, None);
        base.device.free_memory(self.index_staging_buffer.1, None);
        base.device.destroy_buffer(self.vertex_staging_buffer.0, None);
        base.device.free_memory(self.vertex_staging_buffer.1, None);
    } }
    /** To get the quad for a glyph:
          * Let P = ( x: Σ(prior advances) + baseline x, y: baseline y )
          * Let min = P + glyph.plane_min(), with UV of glyph.uv_min()
          * Let max = P + glyph.plane_max(), with UV of glyph.uv_max()
          * Increase Σ(prior advances) by glyph.advance()
    */
    fn get_vertex_and_index_data(&mut self, base: &VkBase) -> (Vec<GlyphQuadVertex>, Vec<u32>) {
        let font = &self.font;
        let per_line_shift = (font.ascent - font.descent) + font.line_gap;
        let scale_factor = self.scale_vector * self.font_size;
        let space_advance = font.glyphs.get_key_value(&' ').unwrap().1.advance;
        let auto_wrap_distance = self.auto_wrap_distance / self.font_size;

        self.glyph_count = 0;
        let mut vertices: Vec<GlyphQuadVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        let mut words = Vec::new();
        let mut advances = Vec::new();

        let mut advance = 0.0;
        {
            let mut current_word = Vec::new();
            for character in self.text.chars() {
                if let Some(glyph_pattern) = font.glyphs.get_key_value(&character) {
                    let glyph = glyph_pattern.1;
                    current_word.push(glyph);
                    advance += glyph.advance;
                    if character == ' ' {
                        words.push(current_word.clone());
                        advances.push(advance);
                        current_word.clear();
                        advance = 0.0;
                    }
                } // else the character is not included in the font atlas, and will be skipped.
            }
            if !current_word.is_empty() {
                words.push(current_word);
                advances.push(advance);
            }
        }
        let mut advance_sum = 0.0;
        let mut line_shift = 0.0;
        let word_advance = |word: &Vec<&Glyph>| -> f32 {
            word.iter().map(|g| g.advance).sum()
        };
        for (i, word) in words.iter().enumerate() {
            let w_advance = word_advance(word);

            if advance_sum > 0.0 && (advance_sum + w_advance) > auto_wrap_distance {
                line_shift -= per_line_shift;
                advance_sum = 0.0;
            }

            for glyph in word.iter() {
                if advance_sum > 0.0 && (advance_sum + glyph.advance) > auto_wrap_distance {
                    line_shift -= per_line_shift;
                    advance_sum = 0.0;
                }
                let p = Vector::new_vec2(advance_sum, line_shift);
                glyph.push_to_buffers(&mut vertices, &mut indices, p, &scale_factor, &self.color);
                self.glyph_count += 1;
                advance_sum += glyph.advance;
            }

            if i != words.len() - 1 {
                advance_sum += space_advance;
            }
        }
        (vertices, indices)
    }
    pub fn set_buffers(mut self, base: &VkBase) -> Self {
        let (vertices, indices) = self.get_vertex_and_index_data(base);
        unsafe {
            for i in 0..MAX_FRAMES_IN_FLIGHT {
                if i == 0 {
                    (self.vertex_buffer[i], self.vertex_staging_buffer) =
                        base.create_device_and_staging_buffer(
                            0 as vk::DeviceSize,
                            &vertices,
                            vk::BufferUsageFlags::VERTEX_BUFFER, false, true, true
                        );
                    (self.index_buffer[i], self.index_staging_buffer) =
                        base.create_device_and_staging_buffer(
                            0 as vk::DeviceSize,
                            &indices,
                            vk::BufferUsageFlags::INDEX_BUFFER, false, true, true
                        );
                } else {
                    self.vertex_buffer[i] = base.create_device_and_staging_buffer(
                        0 as vk::DeviceSize,
                        &vertices,
                        vk::BufferUsageFlags::VERTEX_BUFFER, true, false, true
                    ).0;
                    self.index_buffer[i] = base.create_device_and_staging_buffer(
                        0 as vk::DeviceSize,
                        &indices,
                        vk::BufferUsageFlags::INDEX_BUFFER, true, false, true
                    ).0
                }
            }
        }
        self
    }
    pub fn update_buffers(&mut self, base: &VkBase, command_buffer: CommandBuffer, frame: usize) { unsafe {
        let (vertices, indices) = self.get_vertex_and_index_data(base);
        let vertex_buffer_size = size_of::<GlyphQuadVertex>() * vertices.len();
        let index_buffer_size = size_of::<u32>() * indices.len();
        copy_data_to_memory(self.vertex_staging_buffer.2, &vertices);
        copy_data_to_memory(self.index_staging_buffer.2, &indices);
        base.copy_buffer_synchronous(
            command_buffer,
            &self.vertex_staging_buffer.0,
            &self.vertex_buffer[frame].0,
            None,
            &(vertex_buffer_size as u64)
        );
        base.copy_buffer_synchronous(
            command_buffer,
            &self.index_staging_buffer.0,
            &self.index_buffer[frame].0,
            None,
            &(index_buffer_size as u64)
        );
    } }
    pub fn update_buffers_all_frames(&mut self, base: &VkBase, command_buffer: CommandBuffer) {
        for frame in 0..self.vertex_buffer.len() {
            self.update_buffers(base, command_buffer, frame);
        }
    }

    pub fn update_text(&mut self, text: &str) {
        self.text = text.to_string();
    }

    pub fn text(mut self, text: &str) -> Self {
        self.text = text.to_string();
        self
    }
    /**
     * (0, 0) = bottom left (implemented in vertex shader)
     */
    pub fn position(mut self, position: Vector) -> Self {
        self.position = position;
        self
    }
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }
    pub fn scale_vector(mut self, scale_vector: Vector) -> Self {
        self.scale_vector = scale_vector;
        self
    }
    pub fn color(mut self, color: Vector) -> Self {
        self.color = color;
        self
    }
    pub fn newline_distance(mut self, distance: f32) -> Self {
        self.auto_wrap_distance = distance;
        self
    }
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }
    pub fn italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct GlyphQuadVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}
impl GlyphQuadVertex {
    pub fn new(position: Vector, uv: Vector, color: Vector) -> GlyphQuadVertex {
        GlyphQuadVertex {
            position: position.to_array2(),
            uv: uv.to_array2(),
            color: color.to_array4()
        }
    }
}
#[repr(C)]
struct TextPushConstants {
    clip_min: [f32; 2],
    clip_max: [f32; 2],
    position: [f32; 2],
    resolution: [i32; 2],
    glyph_size: u32,
    distance_range: f32,
    _pad: [u32; 2],
}

pub struct Font {
    pub texture: Texture,
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

        Command::new("resources\\msdf-atlas-gen.exe") // https://github.com/Chlumsky/msdf-atlas-gen/tree/v1.3
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
        let atlas_texture = Texture {
            image: atlas.1.0,
            image_view: atlas.0.0,
            device_memory: atlas.1.1,
            clear_value: vk::ClearValue::default(),
            format: Format::R8G8B8A8_UNORM,
            resolution: vk::Extent3D::default(),
            array_layers: 1,
            samples: SampleCountFlags::TYPE_1,
            is_depth: false,
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
                (Vector::new_vec2(l, 1.0 - b), Vector::new_vec2(r, 1.0 - t))
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
            glyph_size: glyph_size_final,
            distance_range: distance_range_final,
            ascent,
            descent,
            line_gap,
        }
    } }
    pub unsafe fn destroy(&self, base: &VkBase) { unsafe {
        self.texture.destroy(base);
        base.device.destroy_sampler(self.sampler, None);
    } }
}

#[derive(Debug)]
pub struct Glyph {
    pub uv_min: Vector,
    pub uv_max: Vector,
    pub plane_min: Vector,
    pub plane_max: Vector,
    pub advance: f32,
}
impl Glyph {
    pub fn get_quad(&self, position: Vector, scale_factor: &Vector, color: &Vector) -> [GlyphQuadVertex; 4] {
        let position_extent = (self.plane_max - self.plane_min) * scale_factor;
        let uv_extent = self.uv_max - self.uv_min;

        let p = position * scale_factor;
        let bl = GlyphQuadVertex::new( // min
            p + (self.plane_min * scale_factor),
            self.uv_min,
            color.clone()
        );
        let tl = GlyphQuadVertex::new(
            p + (self.plane_min * scale_factor) + Vector::new_vec2(0.0, position_extent.y),
            self.uv_min + Vector::new_vec2(0.0, uv_extent.y),
            color.clone()
        );
        let tr = GlyphQuadVertex::new( // max
            p + (self.plane_max * scale_factor),
            self.uv_max,
            color.clone()
        );
        let br = GlyphQuadVertex::new(
            p + (self.plane_min * scale_factor) + Vector::new_vec2(position_extent.x, 0.0),
            self.uv_min + Vector::new_vec2(uv_extent.x, 0.0),
            color.clone()
        );
        [bl, tl, tr, br]
    }
    pub fn push_to_buffers(&self, vertex_buffer: &mut Vec<GlyphQuadVertex>, index_buffer: &mut Vec<u32>, position: Vector, scale_factor: &Vector, color: &Vector) {
        let v = vertex_buffer.len() as u32;
        let [bl, tl, tr, br] = self.get_quad(position, &scale_factor, &color);
        vertex_buffer.push(bl); vertex_buffer.push(tl); vertex_buffer.push(tr); vertex_buffer.push(br);
        index_buffer.push(v); index_buffer.push(v + 1); index_buffer.push(v + 2);
        index_buffer.push(v); index_buffer.push(v + 2); index_buffer.push(v + 3);
    }
}

