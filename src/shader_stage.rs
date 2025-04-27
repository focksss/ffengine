use std::io::Cursor;
use ash::prelude::VkResult;
use ash::util::read_spv;
use ash::vk;
use ash::vk::{PipelineShaderStageCreateInfo, ShaderModule};
use crate::vk_initializer::VkBase;

pub struct Shader {
    pub vertex_module: vk::ShaderModule,
    pub fragment_module: vk::ShaderModule,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
}

impl Shader {
    unsafe fn new<'a>(&mut self, base: &VkBase, vs_source: &str, fs_source: &str) -> [PipelineShaderStageCreateInfo<'a>; 2] { unsafe {
        let mut vertex_spv_file = Cursor::new(&include_bytes!("../src/shaders/hello_triangle/vert.spv")[..]);
        let mut frag_spv_file = Cursor::new(&include_bytes!("../src/shaders/hello_triangle/frag.spv")[..]);
        let vertex_code =
            read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
        let vertex_shader_info = vk::ShaderModuleCreateInfo::default().code(&vertex_code);

        let frag_code =
            read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
        let frag_shader_info = vk::ShaderModuleCreateInfo::default().code(&frag_code);

        self.vertex_shader_module = Ok(base
            .device
            .create_shader_module(&vertex_shader_info, None)
            .expect("Vertex shader module error"));

        self.fragment_shader_module = Ok(base
            .device
            .create_shader_module(&frag_shader_info, None)
            .expect("Fragment shader module error"));

        let shader_entry_name = c"main";

        [
             PipelineShaderStageCreateInfo {
                 module: self.vertex_shader_module.expect("Cascaded vertex error"),
                 p_name: shader_entry_name.as_ptr(),
                 stage: vk::ShaderStageFlags::VERTEX,
                 ..Default::default()
             },
             PipelineShaderStageCreateInfo {
                 s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                 module: self.fragment_shader_module.expect("Cascaded fragment error"),
                 p_name: shader_entry_name.as_ptr(),
                 stage: vk::ShaderStageFlags::FRAGMENT,
                 ..Default::default()
             },
        ]
    }}
}