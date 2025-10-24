use crate::mem;
use std::collections::HashMap;
use std::{fs, slice};
use std::ops::Add;
use std::path::PathBuf;
use std::process::Command;
use ash::vk;
use ash::vk::{DescriptorType, DeviceMemory, Extent2D, Format, Image, ImageView, Offset2D, RenderPass, SampleCountFlags, Sampler, ShaderStageFlags};
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
    renderpass: Pass,
    shader: Shader,
}
impl TextRenderer<'_> {
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
        TextRenderer {
            base,
            pipeline,
            pipeline_layout,
            descriptor_set,
            renderpass,
            shader,
        }
    } }
    pub unsafe fn destroy(self) { unsafe {
        self.base.device.destroy_pipeline(self.pipeline, None);
        self.base.device.destroy_pipeline_layout(self.pipeline_layout, None);
        self.descriptor_set.destroy(&self.base);
        self.renderpass.destroy(self.base);
        self.shader.destroy(self.base);
    } }
    /** To get the quad for a glyph:
       * Let P = ( x: Σ(prior advances), y: baseline y )
       * Let min = P + glyph.plane_min(), with UV of glyph.uv_min()
       * Let max = P + glyph.plane_max(), with UV of glyph.uv_max()
       * Increase Σ(prior advances) by glyph.advance()
    */
    pub unsafe fn render_text(&self, base: &VkBase, font: &Font, text_info: &TextInformation) { unsafe {
        // <editor-fold desc = "descriptor updates">
        let sampler = base.device.create_sampler(&vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            ..Default::default()
        }, None).unwrap();
        for current_frame in 0..MAX_FRAMES_IN_FLIGHT {
            let image_info = vk::DescriptorImageInfo {
                sampler,
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
    } }
}
pub struct TextInformation {
    text: String,
    position: Vector,
    font_size: f32,
    scale_vector: Vector,
    color: Vector,
    newline_distance: f32,
    bold: bool,
    italic: bool,
}
impl Default for TextInformation {
    fn default() -> TextInformation {
        TextInformation {
            text: String::new(),
            position: Vector::new_empty(),
            font_size: 0.1,
            scale_vector: Vector::new_vec(1.0),
            color: Vector::new_vec(1.0),
            newline_distance: 20.0,
            bold: false,
            italic: false,
        }
    }
}
struct GlyphQuadVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}
struct TextPushConstants {
    clip_min: [f32; 2],
    clip_max: [f32; 2],
}

pub struct Font {
    pub texture: Texture,
    pub sampler: Sampler,
    pub glyphs: HashMap<char, Glyph>,
    pub atlas_size: Vector,
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}
impl Font {
    pub unsafe fn new(base: &mut VkBase, path: &str) -> Self { unsafe {
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
            format: vk::Format::R8G8B8A8_UNORM,
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
            texture: atlas_texture,
            sampler: atlas.0.1,
            glyphs,
            atlas_size: Vector::new_vec2(atlas_w, atlas_h),
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


