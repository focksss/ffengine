use std::cell::RefCell;
use std::slice;
use std::sync::Arc;
use ash::vk;
use ash::vk::{DescriptorType, Format, ShaderStageFlags};
use crate::math::Vector;
use crate::render::{Descriptor, DescriptorCreateInfo, DescriptorSetCreateInfo, Font, Pass, PassCreateInfo, Renderpass, RenderpassCreateInfo, TextInformation, TextRenderer, TextureCreateInfo, VkBase};

pub struct GUI {
    device: ash::Device,

    pub pass: Arc<RefCell<Pass>>,
    pub text_renderer: TextRenderer,
    pub quad_renderpass: Renderpass,

    pub gui_nodes: Vec<GUINode>,
    pub gui_root_node_indices: Vec<usize>,

    pub fonts: Vec<Arc<Font>>,
}
impl GUI {
    pub unsafe fn new(base: &VkBase) -> GUI { unsafe {
        let pass_create_info = PassCreateInfo::new(base)
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R16G16B16A16_SFLOAT).add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC));
        let pass_ref = Arc::new(RefCell::new(Pass::new(pass_create_info)));

        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
            blend_enable: 0,
            src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ZERO,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        }];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);

        let texture_sampler_create_info = DescriptorCreateInfo::new(base)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT);
        let quad_descriptor_set_create_info = DescriptorSetCreateInfo::new(base)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info));
        let quad_renderpass_create_info = { RenderpassCreateInfo::new(base)
            .pass_ref(pass_ref.clone())
            .descriptor_set_create_info(quad_descriptor_set_create_info)
            .vertex_shader_uri(String::from("gui\\quad\\quad.vert.spv"))
            .fragment_shader_uri(String::from("gui\\quad\\quad.frag.spv"))
            .pipeline_color_blend_state_create_info(color_blend_state)
            .push_constant_range(vk::PushConstantRange {
                stage_flags: ShaderStageFlags::ALL_GRAPHICS,
                offset: 0,
                size: size_of::<GUIQuadSendable>() as _,
            }) };
        let quad_renderpass = Renderpass::new(quad_renderpass_create_info);

        let default_font = Arc::new(Font::new(&base, "resources\\fonts\\Oxygen-Regular.ttf", Some(32), Some(2.0)));
        GUI {
            device: base.device.clone(),

            pass: pass_ref.clone(),
            text_renderer: TextRenderer::new(base, Some(pass_ref.clone())),
            quad_renderpass,

            gui_nodes: Vec::new(),
            gui_root_node_indices: Vec::new(),

            fonts: vec![default_font.clone()],
        }
    } }
    pub unsafe fn set_fonts(&mut self, fonts: &Vec<Arc<Font>>) {
        self.fonts = fonts.clone();
        self.text_renderer.update_font_atlases_all_frames(fonts.clone());
    }

    /**
    * Uses custom JSON .gui files
    * * Refer to default.gui in resources/gui
    */
    pub fn load_from_file(&mut self, path: &str) {

    }

    pub unsafe fn draw(&self, current_frame: usize, command_buffer: vk::CommandBuffer,) { unsafe {
        let device = &self.device;
        device.cmd_begin_render_pass(
            command_buffer,
            &self.pass.borrow().get_pass_begin_info(current_frame, None, self.text_renderer.renderpass.scissor),
            vk::SubpassContents::INLINE,
        );

        for node_index in &self.gui_root_node_indices {
            self.draw_node(*node_index, current_frame, command_buffer, Vector::new_vec(0.0), Vector::new_vec2(1920.0, 1080.0));
        }

        device.cmd_end_render_pass(command_buffer);
        self.pass.borrow().transition_to_readable(command_buffer, current_frame);
    } }
    unsafe fn draw_node(
        &self,
        node_index: usize,
        current_frame: usize,
        command_buffer: vk::CommandBuffer,
        parent_position: Vector,
        parent_scale: Vector,
    ) { unsafe {
        let node = &self.gui_nodes[node_index];
        let position = parent_position + if node.absolute_position { node.position } else { node.position * parent_scale };
        let scale = if node.absolute_scale { node.scale } else { parent_scale * node.scale };
        
        if let Some(quad) = &node.quad {
            self.draw_quad(quad, current_frame, command_buffer, position, scale);
        }
        if let Some(text) = &node.text {
            self.draw_text(text, current_frame, command_buffer, position, scale);
        }

        for child in &node.children_indices {
            self.draw_node(*child, current_frame, command_buffer, position, scale);
        }
    } }
    unsafe fn draw_quad(
        &self,
        quad: &GUIQuad,
        current_frame: usize,
        command_buffer: vk::CommandBuffer,
        position: Vector,
        scale: Vector,
    ) { unsafe {
        let clip_min = position + scale * quad.clip_min; // + if quad.absolute_clip_min { quad.clip_min } else { scale * quad.clip_min }
        let clip_max = position + scale * quad.clip_max; // + if quad.absolute_clip_max { quad.clip_max } else { scale * quad.clip_max }
        let position = position + if quad.absolute_position { quad.position } else { quad.position * scale };
        let scale = if quad.absolute_scale { quad.scale } else { scale * quad.scale };

        let quad_constants = GUIQuadSendable {
            color: quad.color.to_array4(),
            resolution: [self.quad_renderpass.viewport.width as i32, self.quad_renderpass.viewport.height as i32],
            clip_min: clip_min.to_array2(),
            clip_max: clip_max.to_array2(),
            position: position.to_array2(),
            scale: scale.to_array2(),
            _pad: [0.0; 2],
        };

        let device = self.device.clone();
        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.quad_renderpass.pipeline,
        );
        device.cmd_set_viewport(command_buffer, 0, &[self.quad_renderpass.viewport]);
        device.cmd_set_scissor(command_buffer, 0, &[self.quad_renderpass.scissor]);
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.quad_renderpass.pipeline_layout,
            0,
            &[self.quad_renderpass.descriptor_set.descriptor_sets[current_frame]],
            &[],
        );
        device.cmd_push_constants(command_buffer, self.quad_renderpass.pipeline_layout, ShaderStageFlags::ALL_GRAPHICS, 0, slice::from_raw_parts(
            &quad_constants as *const GUIQuadSendable as *const u8,
            size_of::<GUIQuadSendable>(),
        ));
        device.cmd_draw(command_buffer, 6, 1, 0, 0);
    } }
    unsafe fn draw_text(
        &self,
        text: &GUIText,
        current_frame: usize,
        command_buffer: vk::CommandBuffer,
        position: Vector,
        scale: Vector,
    ) { unsafe {
        let clip_min = position + scale * text.clip_min; // + if quad.absolute_clip_min { quad.clip_min } else { scale * quad.clip_min }
        let clip_max = position + scale * text.clip_max; // + if quad.absolute_clip_max { quad.clip_max } else { scale * quad.clip_max }
        let position = position + if text.absolute_position { text.position } else { text.position * scale };
        let scale = if text.absolute_scale { text.scale } else { scale * text.scale };

        self.text_renderer.draw_gui_text(current_frame, &text.text_information, position, scale, clip_min, clip_max);
    } }

    pub fn update_text_of_node(&mut self, node_index: usize, text: &str, command_buffer: vk::CommandBuffer) {
        let node_text = self.gui_nodes[node_index].text.as_mut().expect("node does not have text or index is out of bounds");
        node_text.text_information.update_text(text);
        node_text.text_information.update_buffers_all_frames(command_buffer);
    }

    pub unsafe fn destroy(&mut self) { unsafe {
        self.text_renderer.destroy();
        self.quad_renderpass.destroy();
        for node in &self.gui_nodes {
            if let Some(text) = &node.text {
                text.text_information.destroy();
            }
        }
    } }
}

/**
* Position and scale are relative and normalized.
*/
pub struct GUINode {
    pub name: String,
    pub position: Vector,
    pub scale: Vector,
    pub children_indices: Vec<usize>,
    pub absolute_position: bool,
    pub absolute_scale: bool,

    pub text: Option<GUIText>,
    pub quad: Option<GUIQuad>
}
/**
* Position and scale are relative and normalized.
*/
pub struct GUIQuad {
    pub position: Vector,
    pub scale: Vector,
    pub clip_min: Vector,
    pub clip_max: Vector,
    pub absolute_position: bool,
    pub absolute_scale: bool,

    pub color: Vector,
}
pub struct GUIText {
    pub position: Vector,
    pub scale: Vector,
    pub clip_min: Vector,
    pub clip_max: Vector,
    pub absolute_position: bool,
    pub absolute_scale: bool,

    pub text_information: TextInformation,

    pub color: Vector,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct GUIQuadSendable {
    pub color: [f32; 4],

    pub resolution: [i32; 2],

    pub clip_min: [f32; 2],
    pub clip_max: [f32; 2],

    pub position: [f32; 2],

    pub scale: [f32; 2],

    pub _pad: [f32; 2],
}