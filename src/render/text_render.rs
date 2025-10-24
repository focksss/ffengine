use crate::{mem, CameraMatrixUniformData};
use std::collections::HashMap;
use std::{fs, slice};
use std::ffi::c_void;
use std::ops::Add;
use std::path::PathBuf;
use std::process::Command;
use ash::vk;
use std::ptr::null_mut;
use ash::vk::{CommandBuffer, DescriptorType, DeviceMemory, Extent2D, Format, Image, ImageView, Offset2D, RenderPass, SampleCountFlags, Sampler, ShaderStageFlags};
use serde_json::{Value};
use crate::{offset_of, MAX_FRAMES_IN_FLIGHT};
use crate::math::Vector;
use crate::render::*;

const OUTPUT_DIR: &str = "resources\\fonts\\generated";

pub struct TextRenderer<'a> {
    base: &'a VkBase,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set: DescriptorSet,
    pub renderpass: Pass,
    sampler: Sampler,
    shader: Shader,
}
impl<'a> TextRenderer<'a> {
    pub unsafe fn new(base: &VkBase) -> TextRenderer { unsafe {
        //<editor-fold desc = "pass">
        let color_tex_create_info = TextureCreateInfo::new(base).format(Format::R16G16B16A16_SFLOAT);
        let pass_create_info = PassCreateInfo::new(base)
            .frames_in_flight(MAX_FRAMES_IN_FLIGHT)
            .add_color_attachment_info(color_tex_create_info)
            .depth_attachment_info(TextureCreateInfo::new(base).format(Format::D16_UNORM).is_depth(true).clear_value([1.0, 0.0, 0.0, 0.0])); // depth
        let renderpass = Pass::new(pass_create_info);
        //</editor-fold>
        //<editor-fold desc = "descriptor set">
        let texture_sampler_create_info = DescriptorCreateInfo::new(base)
            .frames_in_flight(MAX_FRAMES_IN_FLIGHT)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT);
        let descriptor_set_create_info = DescriptorSetCreateInfo::new(base)
            .frames_in_flight(MAX_FRAMES_IN_FLIGHT)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info));
        let descriptor_set = DescriptorSet::new(descriptor_set_create_info);
        //</editor-fold>
        //<editor-fold desc = "shader">
        let shader = Shader::new(base, "text\\text.vert.spv", "text\\text.frag.spv", None);
        //</editor-fold>
        //<editor-fold desc = "graphics pipeline initiation">
        let push_constant_range = vk::PushConstantRange {
            stage_flags: ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: size_of::<TextPushConstants>() as _,
        };
        let pipeline_layout = base
            .device
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo {
                    s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                    set_layout_count: 1,
                    p_set_layouts: &descriptor_set.descriptor_set_layout,
                    p_push_constant_ranges: &push_constant_range,
                    push_constant_range_count: 1,
                    ..Default::default()
                }, None
            ).unwrap();

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

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: base.surface_resolution.width as f32,
            height: base.surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [base.surface_resolution.into()];
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
            .scissors(&scissors)
            .viewports(&viewports);
        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_state);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            cull_mode: vk::CullModeFlags::NONE,
            line_width: 1.0,
            polygon_mode: vk::PolygonMode::FILL,
            ..Default::default()
        };

        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        let noop_stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            ..Default::default()
        };
        let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: 1,
            depth_write_enable: 1,
            depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
            front: noop_stencil_state,
            back: noop_stencil_state,
            max_depth_bounds: 1.0,
            ..Default::default()
        };
        let null_blend_attachment = vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::FALSE,  // Disable blending
            src_color_blend_factor: vk::BlendFactor::ONE,
            dst_color_blend_factor: vk::BlendFactor::ZERO,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        };
        let null_blend_states = [null_blend_attachment; 1];
        let null_blend_state = vk::PipelineColorBlendStateCreateInfo::default().attachments(&null_blend_states);

        let shader_create_info = shader.generate_shader_stage_create_infos();

        let base_pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .dynamic_state(&dynamic_state_info);
        let pipeline_info = base_pipeline_info
            .stages(&shader_create_info)
            .vertex_input_state(&vertex_input_state_info)
            .multisample_state(&multisample_state_info)
            .render_pass(renderpass.renderpass)
            .color_blend_state(&null_blend_state)
            .layout(pipeline_layout)
            .depth_stencil_state(&depth_state_info);

        let graphics_pipelines = base
            .device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
            .expect("Unable to create graphics pipeline");
        let pipeline = graphics_pipelines[0];
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
            base,
            pipeline,
            pipeline_layout,
            descriptor_set,
            renderpass,
            sampler,
            shader,
        }
    } }
    pub unsafe fn destroy(self) { unsafe {
        self.base.device.destroy_pipeline(self.pipeline, None);
        self.base.device.destroy_pipeline_layout(self.pipeline_layout, None);
        self.descriptor_set.destroy(&self.base);
        self.renderpass.destroy(self.base);
        self.base.device.destroy_sampler(self.sampler, None);
        self.shader.destroy(self.base);
    } }

    pub unsafe fn render_text(&self, frame: usize, frame_command_buffer: CommandBuffer, text_info: &TextInformation) { unsafe {
        let font = text_info.font;
        let base = self.base;
        let device = &base.device;
        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: base.surface_resolution.width as f32,
            height: base.surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [base.surface_resolution.into()];
        // <editor-fold desc = "descriptor updates">
        for current_frame in 0..MAX_FRAMES_IN_FLIGHT {
            let image_info = vk::DescriptorImageInfo {
                sampler: self.sampler,
                image_view: font.texture.image_view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            };
            let descriptor_write = vk::WriteDescriptorSet::default()
                .dst_set(self.descriptor_set.descriptor_sets[current_frame])
                .dst_binding(0u32)
                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(slice::from_ref(&image_info));
            base.device.update_descriptor_sets(&[descriptor_write], &[]);
        }
        //</editor-fold>
        let pass_pass_begin_info = vk::RenderPassBeginInfo::default()
            .render_pass(self.renderpass.renderpass)
            .framebuffer(self.renderpass.framebuffers[frame])
            .render_area(base.surface_resolution.into())
            .clear_values(&self.renderpass.clear_values);

        device.cmd_begin_render_pass(
            frame_command_buffer,
            &pass_pass_begin_info,
            vk::SubpassContents::INLINE,
        );
        device.cmd_bind_pipeline(
            frame_command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline,
        );
        /*
        device.cmd_push_constants(frame_command_buffer, self.pipeline_layout, ShaderStageFlags::FRAGMENT, 0, slice::from_raw_parts(
            &camera_constants as *const CameraMatrixUniformData as *const u8,
            size_of::<CameraMatrixUniformData>(),
        ));
         */
        device.cmd_set_viewport(frame_command_buffer, 0, &viewports);
        device.cmd_set_scissor(frame_command_buffer, 0, &scissors);
        device.cmd_bind_descriptor_sets(
            frame_command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline_layout,
            0,
            &[self.descriptor_set.descriptor_sets[frame]],
            &[],
        );
        device.cmd_bind_vertex_buffers(
            frame_command_buffer,
            0,
            &[text_info.vertex_buffer.0],
            &[0],
        );
        device.cmd_bind_index_buffer(
            frame_command_buffer,
            text_info.index_buffer.0,
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
        );


        device.cmd_end_render_pass(frame_command_buffer);
        //</editor-fold>
        self.renderpass.transition_to_readable(base, frame_command_buffer, frame);
    } }
}
pub struct TextInformation<'a> {
    font: &'a Font<'a>,
    text: String,
    position: Vector,
    font_size: f32,
    scale_vector: Vector,
    color: Vector,
    newline_distance: f32,
    bold: bool,
    italic: bool,

    base: &'a VkBase,
    pub glyph_count: u32,
    pub vertex_buffer: (vk::Buffer, DeviceMemory),
    pub vertex_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
    pub index_buffer: (vk::Buffer, DeviceMemory),
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
            newline_distance: 20.0,
            bold: false,
            italic: false,

            base: font.base,
            glyph_count: 0,
            vertex_buffer: (vk::Buffer::null(), DeviceMemory::null()),
            vertex_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
            index_buffer: (vk::Buffer::null(), DeviceMemory::null()),
            index_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
        }
    }
    pub fn destroy(&mut self) { unsafe {
        let base = self.base;
        base.device.destroy_buffer(self.index_buffer.0, None);
        base.device.free_memory(self.index_buffer.1, None);
        base.device.destroy_buffer(self.index_staging_buffer.0, None);
        base.device.free_memory(self.index_staging_buffer.1, None);
        base.device.destroy_buffer(self.vertex_buffer.0, None);
        base.device.free_memory(self.vertex_buffer.1, None);
        base.device.destroy_buffer(self.vertex_staging_buffer.0, None);
        base.device.free_memory(self.vertex_staging_buffer.1, None);
    } }
    /** To get the quad for a glyph:
          * Let P = ( x: Σ(prior advances) + baseline x, y: baseline y )
          * Let min = P + glyph.plane_min(), with UV of glyph.uv_min()
          * Let max = P + glyph.plane_max(), with UV of glyph.uv_max()
          * Increase Σ(prior advances) by glyph.advance()
    */
    pub fn update_buffers(mut self) -> Self {
        let font = &self.font;
        let per_line_shift = (font.ascent - font.descent) + font.line_gap;
        let scale_factor = self.scale_vector * self.font_size;

        self.glyph_count = 0;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut advance_sum = 0.0;
        let line_shift = 0.0;

        for character in self.text.chars() {
            if let Some(glyph_pattern) = font.glyphs.get_key_value(&character) {
                self.glyph_count += 1;
                let glyph = glyph_pattern.1;
                let position_extent = (glyph.plane_max - glyph.plane_min) * scale_factor;
                let uv_extent = glyph.uv_max - glyph.uv_min;

                let p = self.position + (Vector::new_vec2(advance_sum, line_shift) * scale_factor);
                let bl = GlyphQuadVertex::new( // min
                    p + (glyph.plane_min * scale_factor),
                    glyph.uv_min,
                    self.color
                );
                let tl = GlyphQuadVertex::new(
                    p + (glyph.plane_min * scale_factor) + Vector::new_vec2(0.0, position_extent.y),
                    glyph.uv_min + Vector::new_vec2(0.0, uv_extent.y),
                    self.color
                );
                let tr = GlyphQuadVertex::new( // max
                    p + (glyph.plane_max * scale_factor),
                    glyph.uv_max,
                    self.color
                );
                let br = GlyphQuadVertex::new(
                    p + (glyph.plane_min * scale_factor) + Vector::new_vec2(position_extent.x, 0.0),
                    glyph.uv_min + Vector::new_vec2(uv_extent.x, 0.0),
                    self.color
                );
                let v = vertices.len() as u32;
                vertices.push(bl); vertices.push(tl); vertices.push(tr); vertices.push(br);
                indices.push(v); indices.push(v + 1); indices.push(v + 2);
                indices.push(v); indices.push(v + 2); indices.push(v + 3);

                advance_sum += glyph.advance;
            } // else the character is not included in the font atlas, and will be skipped.
        }
        unsafe {
            (self.vertex_buffer, self.vertex_staging_buffer) =
                self.base.create_device_and_staging_buffer(
                    (size_of::<GlyphQuadVertex>() * vertices.len()) as u64,
                    &vertices,
                    vk::BufferUsageFlags::VERTEX_BUFFER, false, false, true
                );
            (self.index_buffer, self.index_staging_buffer) =
                self.base.create_device_and_staging_buffer(
                    (size_of::<u32>() * indices.len()) as u64,
                    &indices,
                    vk::BufferUsageFlags::INDEX_BUFFER, false, false, true
                );
        }
        self
    }

    pub fn text(mut self, text: &str) -> Self {
        self.text = text.to_string();
        self
    }
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
        self.newline_distance = distance;
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
impl Drop for TextInformation<'_> {
    fn drop(&mut self) {
        unsafe {
            self.destroy()
        }
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
}

pub struct Font<'a> {
    pub base: &'a VkBase,
    pub texture: Texture,
    pub sampler: Sampler,
    pub glyphs: HashMap<char, Glyph>,
    pub atlas_size: Vector,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}
impl<'a> Font<'a> {
    pub fn new(base: &'a VkBase, path: &str) -> Self { unsafe {
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
            format: Format::R8G8B8A8_UNORM,
            resolution: vk::Extent3D::default(),
            array_layers: 1,
            samples: base.msaa_samples,
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
                (Vector::new_vec2(l, b), Vector::new_vec2(r, t))
            } else {
                (Vector::new_vec2(0.0, 0.0), Vector::new_vec2(0.0, 0.0))
            };

            glyphs.insert(ch, Glyph { uv_min, uv_max, plane_min, plane_max, advance });
        }

        Font {
            base,
            texture: atlas_texture,
            sampler: atlas.0.1,
            glyphs,
            atlas_size: Vector::new_vec2(atlas_w, atlas_h),
            ascent,
            descent,
            line_gap,
        }
    } }
    pub unsafe fn destroy(&self) { unsafe {
        self.texture.destroy(self.base);
        self.base.device.destroy_sampler(self.sampler, None);
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


