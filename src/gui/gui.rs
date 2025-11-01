use std::sync::Arc;
use ash::vk::RenderPass;
use crate::render::Pass;

pub struct GUI {
    pub renderpass: RenderPass,
    pub pass: Pass,
}