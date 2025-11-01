use std::sync::Arc;
use ash::vk;
use ash::vk::{DescriptorType, Format, ShaderStageFlags};
use crate::math::Vector;
use crate::render::{Descriptor, DescriptorCreateInfo, DescriptorSetCreateInfo, Font, Pass, PassCreateInfo, Renderpass, RenderpassCreateInfo, TextInformation, TextRenderer, TextureCreateInfo, VkBase};

pub struct GUI {
    device: ash::Device,

    pub pass: Arc<Pass>,
    pub text_renderer: TextRenderer,
    pub quad_renderpass: Renderpass,

    pub gui_nodes: Vec<GUINode>,

    pub fonts: Vec<Arc<Font>>,
}
impl GUI {
    pub unsafe fn new(base: &VkBase) -> GUI { unsafe {
        let pass_create_info = PassCreateInfo::new(base)
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R16G16B16A16_SFLOAT).add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC));
        let pass_ref = Arc::new(Pass::new(pass_create_info));

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
            .push_constant_range(vk::PushConstantRange {
                stage_flags: ShaderStageFlags::FRAGMENT,
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

            fonts: vec![default_font.clone()],
        }
    } }
    pub unsafe fn set_fonts(&mut self, fonts: &Vec<Arc<Font>>) {
        self.fonts = fonts.clone();
        self.text_renderer.update_font_atlases_all_frames(fonts.clone());
    }

    pub unsafe fn draw(&mut self, current_frame: usize, command_buffer: vk::CommandBuffer,) { unsafe {
        let device = &self.device;
        device.cmd_begin_render_pass(
            command_buffer,
            &self.pass.get_pass_begin_info(current_frame, None, self.text_renderer.renderpass.scissor),
            vk::SubpassContents::INLINE,
        );
        for node in &self.gui_nodes {

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
    pub clip_min: Vector,
    pub clip_max: Vector,
    pub children_indices: Vec<usize>,

    pub text: Option<TextInformation>,
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