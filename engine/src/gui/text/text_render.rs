use std::mem;
use std::slice;
use std::cell::RefCell;
use std::ffi::c_void;
use ash::vk;
use std::ptr::null_mut;
use std::sync::Arc;
use ash::vk::{CommandBuffer, DescriptorType, DeviceMemory, Format, Sampler, ShaderStageFlags};
use crate::gui::text::{font::Font, glyph::{Glyph, GlyphQuadVertex}};
use crate::math::Vector;
use crate::offset_of;
use crate::render::render::MAX_FRAMES_IN_FLIGHT;
use crate::render::render_helper::{Descriptor, DescriptorCreateInfo, DescriptorSetCreateInfo, Pass, PassCreateInfo, PipelineCreateInfo, Renderpass, RenderpassCreateInfo, TextureCreateInfo};
use crate::render::vulkan_base::{copy_buffer_synchronous, copy_data_to_memory, Context};

const OUTPUT_DIR: &str = "resources\\fonts\\generated";
const MAX_FONTS: usize = 10;

pub struct TextRenderer {
    context: Arc<Context>,

    pub renderpass: Renderpass,

    pub default_font: Arc<Font>,
    sampler: Sampler,
}
impl TextRenderer {
    pub fn new(context: &Arc<Context>, pass_ref: Option<Arc<RefCell<Pass>>>) -> TextRenderer { unsafe {
        let sampler = context.device.create_sampler(&vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            ..Default::default()
        }, None).unwrap();
        let default_font = Arc::new(Font::new(context, "engine\\resources\\fonts\\Oxygen-Regular.ttf", Some(32), Some(2.0)));
        TextRenderer {
            context: context.clone(),
            renderpass: Self::create_text_renderpass(context, default_font.clone(), pass_ref),
            default_font: default_font.clone(),
            sampler,
        }
    } }
    pub fn destroy(&mut self) { unsafe {
        self.renderpass.destroy();
        self.context.device.destroy_sampler(self.sampler, None);
        self.default_font.destroy();
    } }

    pub fn create_text_renderpass(context: &Arc<Context>, default_font: Arc<Font>, pass_ref: Option<Arc<RefCell<Pass>>>) -> Renderpass {
        let color_tex_create_info = TextureCreateInfo::new(context).format(Format::R8G8B8A8_UNORM);
        let pass_create_info = PassCreateInfo::new(context)
            .frames_in_flight(MAX_FRAMES_IN_FLIGHT)
            .add_color_attachment_info(color_tex_create_info.add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC));
        //<editor-fold desc = "descriptor set">
        let image_infos: Vec<vk::DescriptorImageInfo> = vec![vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view: default_font.texture.image_view,
            sampler: default_font.sampler,
            ..Default::default()
        }; MAX_FONTS];
        let atlas_samplers_create_info = DescriptorCreateInfo::new(context)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT)
            .dynamic(true)
            .image_infos(image_infos.clone());

        let descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
            .frames_in_flight(MAX_FRAMES_IN_FLIGHT)
            .add_descriptor(Descriptor::new(&atlas_samplers_create_info));
        //</editor-fold>

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

        let mut renderpass_create_info = RenderpassCreateInfo::new(context)
            .pass_create_info(pass_create_info)
            .descriptor_set_create_info(descriptor_set_create_info)
            .add_push_constant_range(push_constant_range)
            .add_pipeline_create_info(PipelineCreateInfo::new()
                .pipeline_input_assembly_state(vertex_input_assembly_state_info)
                .pipeline_vertex_input_state(vertex_input_state_info)
                .pipeline_color_blend_state_create_info(color_blend_state)
                .vertex_shader_uri(String::from("gui\\text\\text.vert.spv"))
                .fragment_shader_uri(String::from("gui\\text\\text.frag.spv")));
        if pass_ref.is_some() {
            renderpass_create_info = renderpass_create_info.pass_ref(pass_ref.unwrap());
        }
        Renderpass::new(renderpass_create_info)
    }

    pub fn update_font_atlases_all_frames(&self, fonts: Vec<Arc<Font>>) {
        for frame in 0..MAX_FRAMES_IN_FLIGHT {
            self.update_font_atlases(&fonts, frame);
        }
    }
    pub fn update_font_atlases(&self, fonts: &Vec<Arc<Font>>, frame: usize) { unsafe {
        let mut image_infos: Vec<vk::DescriptorImageInfo> = Vec::with_capacity(MAX_FONTS);
        for font in fonts {
            image_infos.push(vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image_view: font.texture.image_view,
                sampler: font.sampler,
                ..Default::default()
            });
        }
        let missing = MAX_FONTS - image_infos.len();
        let default_device_texture = &self.default_font.texture;
        for _ in 0..missing {
            image_infos.push(vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image_view: default_device_texture.image_view,
                sampler: self.default_font.sampler,
                ..Default::default()
            });
        }
        let image_infos = image_infos.as_slice().as_ptr();

        let descriptor_write = vk::WriteDescriptorSet {
            s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
            dst_set: self.renderpass.descriptor_set.borrow().descriptor_sets[frame],
            dst_binding: 0,
            dst_array_element: 0,
            descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: MAX_FONTS as u32,
            p_image_info: image_infos,
            ..Default::default()
        };
        self.context.device.update_descriptor_sets(&[descriptor_write], &[]);
    }}

    /**
    * Font index of each TexInformation must be the index of the correct font in the most recently bound font_atlas vector.

    * This function will not begin the renderpass, it assumes that the pass is already being recorded for.
    */
    pub fn draw_gui_text(
        &self,
        frame: usize,
        text_info: &TextInformation,
        position: Vector,
        scale: Vector,
        clip_min: Vector,
        clip_max: Vector,
    ) { unsafe {
        let frame_command_buffer = self.context.draw_command_buffers[frame];
        let device = &self.context.device;

        device.cmd_bind_pipeline(
            frame_command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.renderpass.pipelines[0].vulkan_pipeline,
        );
        device.cmd_set_viewport(frame_command_buffer, 0, &[self.renderpass.viewport]);
        device.cmd_set_scissor(frame_command_buffer, 0, &[self.renderpass.scissor]);
        device.cmd_bind_descriptor_sets(
            frame_command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.renderpass.pipeline_layout,
            0,
            &[self.renderpass.descriptor_set.borrow().descriptor_sets[frame]],
            &[],
        );
        device.cmd_push_constants(frame_command_buffer, self.renderpass.pipeline_layout, ShaderStageFlags::ALL_GRAPHICS, 0, slice::from_raw_parts(
            &TextPushConstants {
                clip_min: clip_min.to_array2(),
                clip_max: clip_max.to_array2(),
                position: position.to_array2(),
                resolution: [self.renderpass.viewport.width as i32, self.renderpass.viewport.height as i32],
                glyph_size: text_info.font.glyph_size,
                distance_range: text_info.font.distance_range,
                font_index: text_info.font_index.unwrap_or(0),
                align_shift: match text_info.alignment {
                    BaselineAlignment::Top => text_info.font.ascent,
                    BaselineAlignment::Center => (text_info.font.ascent + text_info.font.descent) / 2.0,
                    BaselineAlignment::Bottom => -text_info.font.descent,
                },
                font_size: text_info.font_size,
                _pad: [0; 3]
            } as *const TextPushConstants as *const u8,
            size_of::<TextPushConstants>(),
        ));
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
    } }
}
pub struct TextInformation {
    context: Arc<Context>,

    font: Arc<Font>,
    font_index: Option<u32>,
    pub text: String,
    pub font_size: f32,
    pub scale_vector: Vector,
    color: Vector,
    pub auto_wrap_distance: f32,
    pub bold: bool,
    pub italic: bool,

    pub alignment: BaselineAlignment,

    pub glyph_count: u32,
    pub vertex_buffer: Vec<(vk::Buffer, DeviceMemory)>,
    pub vertex_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
    vertex_buffer_size: u64,
    pub index_buffer: Vec<(vk::Buffer, DeviceMemory)>,
    pub index_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
    index_buffer_size: u64,
}
#[derive(Debug)]
pub enum BaselineAlignment {
    Top,
    Center,
    Bottom,
}
impl TextInformation {
    pub fn new(font: Arc<Font>) -> TextInformation {
        TextInformation {
            context: font.context.clone(),
            font,
            font_index: None,

            text: String::new(),
            font_size: 0.1,
            scale_vector: Vector::fill(1.0),
            color: Vector::fill(1.0),
            auto_wrap_distance: 20.0,
            bold: false,
            italic: false,

            alignment: BaselineAlignment::Top,

            glyph_count: 0,
            vertex_buffer: vec![(vk::Buffer::null(), DeviceMemory::null()); MAX_FRAMES_IN_FLIGHT],
            vertex_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
            vertex_buffer_size: 0,
            index_buffer: vec![(vk::Buffer::null(), DeviceMemory::null()); MAX_FRAMES_IN_FLIGHT],
            index_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
            index_buffer_size: 0,
        }
    }
    pub fn destroy(&self) { unsafe {
        for buffer in self.vertex_buffer.iter() {
            self.context.device.destroy_buffer(buffer.0, None);
            self.context.device.free_memory(buffer.1, None);
        }
        for buffer in self.index_buffer.iter() {
            self.context.device.destroy_buffer(buffer.0, None);
            self.context.device.free_memory(buffer.1, None);
        }
        self.context.device.destroy_buffer(self.index_staging_buffer.0, None);
        self.context.device.free_memory(self.index_staging_buffer.1, None);
        self.context.device.destroy_buffer(self.vertex_staging_buffer.0, None);
        self.context.device.free_memory(self.vertex_staging_buffer.1, None);
    } }
    /** To get the quad for a glyph:
          * Let P = ( x: Σ(prior advances) + baseline x, y: baseline y )
          * Let min = P + glyph.plane_min(), with UV of glyph.uv_min()
          * Let max = P + glyph.plane_max(), with UV of glyph.uv_max()
          * Increase Σ(prior advances) by glyph.advance()
    */
    fn get_vertex_and_index_data(&mut self) -> (Vec<GlyphQuadVertex>, Vec<u32>) {
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
                let p = Vector::new2(advance_sum, line_shift);
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
    pub fn build_set_buffers(mut self) -> Self {
        self.set_buffers();
        self
    }
    pub fn set_buffers(&mut self) {
        let (vertices, indices) = self.get_vertex_and_index_data();
        let vertex_buffer_size = (size_of::<GlyphQuadVertex>() * vertices.len()) as u64 + 5000;
        self.vertex_buffer_size = vertex_buffer_size;
        let index_buffer_size = (size_of::<u32>() * indices.len()) as u64 + 2000;
        self.index_buffer_size = index_buffer_size;
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            if i == 0 {
                (self.vertex_buffer[i], self.vertex_staging_buffer) =
                    self.context.create_device_and_staging_buffer(
                        vertex_buffer_size,
                        &vertices,
                        vk::BufferUsageFlags::VERTEX_BUFFER, false, true, true
                    );
                (self.index_buffer[i], self.index_staging_buffer) =
                    self.context.create_device_and_staging_buffer(
                        index_buffer_size,
                        &indices,
                        vk::BufferUsageFlags::INDEX_BUFFER, false, true, true
                    );
            } else {
                self.vertex_buffer[i] = self.context.create_device_and_staging_buffer(
                    vertex_buffer_size,
                    &vertices,
                    vk::BufferUsageFlags::VERTEX_BUFFER, true, false, true
                ).0;
                self.index_buffer[i] = self.context.create_device_and_staging_buffer(
                    index_buffer_size,
                    &indices,
                    vk::BufferUsageFlags::INDEX_BUFFER, true, false, true
                ).0
            }
        }
    }

    pub fn update_buffers(&mut self, command_buffer: CommandBuffer, frame: usize) {
        let (vertices, indices) = self.get_vertex_and_index_data();
        let vertex_buffer_size = size_of::<GlyphQuadVertex>() * vertices.len();
        let index_buffer_size = size_of::<u32>() * indices.len();
        copy_data_to_memory(self.vertex_staging_buffer.2, &vertices);
        copy_data_to_memory(self.index_staging_buffer.2, &indices);
        copy_buffer_synchronous(
            &self.context.device,
            command_buffer,
            &self.vertex_staging_buffer.0,
            &self.vertex_buffer[frame].0,
            None,
            &(vertex_buffer_size as u64)
        );
        copy_buffer_synchronous(
            &self.context.device,
            command_buffer,
            &self.index_staging_buffer.0,
            &self.index_buffer[frame].0,
            None,
            &(index_buffer_size as u64)
        );
    }
    pub fn update_buffers_all_frames(&mut self, command_buffer: CommandBuffer) {
        for frame in 0..self.vertex_buffer.len() {
            self.update_buffers(command_buffer, frame);
        }
    }

    pub fn update_text(&mut self, text: &str) {
        self.text = text.to_string();
    }

    pub fn text(mut self, text: &str) -> Self {
        self.text = text.to_string();
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
struct TextPushConstants {
    clip_min: [f32; 2],
    clip_max: [f32; 2],

    position: [f32; 2],
    resolution: [i32; 2],

    glyph_size: u32,
    distance_range: f32,
    font_index: u32,
    align_shift: f32,

    font_size: f32,
    _pad: [u32; 3],
}


