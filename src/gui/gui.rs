use std::sync::Arc;
use ash::vk;
use ash::vk::{Format};
use crate::math::Vector;
use crate::render::{Font, Pass, PassCreateInfo, Renderpass, TextInformation, TextRenderer, TextureCreateInfo, VkBase};

pub struct GUI {
    pub pass: Arc<Pass>,
    pub text_renderpass: Renderpass,

    // pub renderpass: RenderPass,

    pub fonts: Vec<Arc<Font>>,
}
impl GUI {
    pub unsafe fn new(base: &VkBase) -> GUI { unsafe {
        let pass_create_info = PassCreateInfo::new(base)
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R16G16B16A16_SFLOAT).add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC));

        let default_font = Font::new(&base, "resources\\fonts\\Oxygen-Regular.ttf", Some(32), Some(2.0));

        let pass_ref = Arc::new(Pass::new(pass_create_info));

        GUI {
            pass: pass_ref.clone(),
            text_renderpass: TextRenderer::create_text_renderpass(base, Some(pass_ref.clone())),

            fonts: vec![Arc::new(default_font)],
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
}
/**
* Position and scale are relative and normalized.
*/
pub struct GUIRectangle {
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