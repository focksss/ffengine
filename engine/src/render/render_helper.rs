use std::cell::RefCell;
use std::ffi::c_void;
use std::io::{BufWriter, Cursor};
use std::sync::Arc;
use ash::{vk, Device, Instance};
use ash::util::read_spv;
use ash::vk::{Buffer, ClearColorValue, ClearDepthStencilValue, ClearValue, DescriptorBindingFlags, DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateFlags, DescriptorPoolSize, DescriptorSetLayout, DescriptorSetLayoutCreateFlags, DescriptorType, DeviceMemory, DeviceSize, DynamicState, Extent3D, Format, GraphicsPipelineCreateInfo, ImageAspectFlags, ImageSubresourceRange, ImageUsageFlags, ImageView, MemoryPropertyFlags, Offset2D, PhysicalDevice, PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo, PipelineDepthStencilStateCreateInfo, PipelineInputAssemblyStateCreateInfo, PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo, PipelineShaderStageCreateInfo, PipelineTessellationStateCreateInfo, PipelineVertexInputStateCreateInfo, PushConstantRange, SampleCountFlags, ShaderModule, ShaderModuleCreateInfo, ShaderStageFlags, StencilOpState};
use crate::engine::get_command_buffer;
use crate::render::render::MAX_FRAMES_IN_FLIGHT;
use crate::render::vulkan_base::{find_memorytype_index, load_file, Context, VkBase};

const SHADER_PATH: &str = "engine\\resources\\shaders\\spv\\";

pub struct Renderpass {
    context: Arc<Context>,

    pub pass: Arc<RefCell<Pass>>,
    pub descriptor_set: Arc<RefCell<DescriptorSet>>,
    pub pipelines: Vec<Pipeline>,
    pub pipeline_layout: vk::PipelineLayout,
    pub viewport: vk::Viewport,
    pub scissor: vk::Rect2D,
}
impl Renderpass {
    pub unsafe fn new(create_info: RenderpassCreateInfo) -> Renderpass { unsafe {
        let context = create_info.context.clone();
        let pass = if let Some(pass_create_info) = create_info.pass_create_info {
            Arc::new(RefCell::new(Pass::new(pass_create_info)))
        } else {
            create_info.pass_ref.expect("Renderpass builder did not contain a pass_ref or a pass_create_info")
        };
        let descriptor_set = if let Some(descriptor_set_create_info) = create_info.descriptor_set_create_info {
            Arc::new(RefCell::new(DescriptorSet::new(descriptor_set_create_info)))
        } else {
            create_info.descriptor_set_ref.expect("Renderpass builder did not contain a pass_ref or a pass_create_info")
        };
        let pipeline_layout = context
            .device
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo {
                    s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                    set_layout_count: 1,
                    p_set_layouts: &descriptor_set.borrow().descriptor_set_layout,
                    p_push_constant_ranges: &create_info.push_constant_info.unwrap_or_default(),
                    push_constant_range_count: if create_info.push_constant_info.is_some() { 1 } else { 0 },
                    ..Default::default()
                }, None
            ).unwrap();
        let viewports = [create_info.viewport];
        let scissors = [create_info.scissor];
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(&viewports)
            .scissors(&scissors);
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&create_info.dynamic_state);

        let shaders = create_info.pipeline_create_infos.iter().map(|info| {
            Shader::new(
                &context,
                info.vertex_shader_uri.as_ref().expect("Must have a vertex shader per pipeline"),
                info.fragment_shader_uri.as_ref().expect("Must have a fragment shader per pipeline"),
                info.geometry_shader_uri.as_ref().map(|s| s.as_str()),
            )
        }).collect::<Vec<Shader>>();
        let shader_stage_create_infos = shaders.iter().map(|shader| {
            shader.generate_shader_stage_create_infos()
        }).collect::<Vec<Vec<PipelineShaderStageCreateInfo>>>();

        let pipeline_create_infos = create_info
            .pipeline_create_infos
            .iter().enumerate()
            .map(|(i, info)| {
                GraphicsPipelineCreateInfo::default()
                    .stages(&shader_stage_create_infos[i])
                    .vertex_input_state(&info.pipeline_vertex_input_state_create_info)
                    .input_assembly_state(&info.pipeline_input_assembly_state_create_info)
                    .tessellation_state(&info.pipeline_tess_state_create_info)
                    .viewport_state(&viewport_state)
                    .rasterization_state(&info.pipeline_rasterization_state_create_info)
                    .multisample_state(&info.pipeline_multisample_state_create_info)
                    .depth_stencil_state(&info.pipeline_depth_stencil_state_create_info)
                    .color_blend_state(&info.pipeline_color_blend_state_create_info)
                    .dynamic_state(&dynamic_state_info)
                    .layout(pipeline_layout)
                    .render_pass(pass.borrow().renderpass)
            }).collect::<Vec<GraphicsPipelineCreateInfo>>();
        let vulkan_pipelines = context.device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            &pipeline_create_infos,
            None
        ).expect("Failed to create pipeline");
        let pipelines: Vec<Pipeline> = shaders
            .into_iter()
            .zip(vulkan_pipelines.into_iter())
            .map(|(shader, vk_pipeline)| Pipeline {
                shader,
                vulkan_pipeline: vk_pipeline,
            })
            .collect();
        Renderpass {
            context,

            pass,
            descriptor_set,
            pipelines,
            pipeline_layout,
            viewport: create_info.viewport,
            scissor: create_info.scissor,
        }
    } }
    pub unsafe fn new_present_renderpass(base: &VkBase) -> Renderpass { unsafe {
        let context = base.context.clone();
        let mut create_info = RenderpassCreateInfo::new(&context);
        let create_info = create_info.add_pipeline_create_info(
            PipelineCreateInfo::new()
                .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
                .fragment_shader_uri(String::from("quad\\quad.frag.spv"))
        );

        let pass = Pass::new_present_pass(base);
        let descriptor_set = DescriptorSet::new(DescriptorSetCreateInfo::new(&context)
            .add_descriptor(Descriptor::new(&DescriptorCreateInfo::new(&context)
                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                .shader_stages(ShaderStageFlags::FRAGMENT)
            ))
        );
        let pipeline_layout = context
            .device
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo {
                    s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                    set_layout_count: 1,
                    p_set_layouts: &descriptor_set.descriptor_set_layout,
                    ..Default::default()
                }, None
            ).unwrap();
        let viewports = [create_info.viewport];
        let scissors = [create_info.scissor];
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(&viewports)
            .scissors(&scissors);
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&create_info.dynamic_state);

        let shaders = create_info.pipeline_create_infos.iter().map(|info| {
            Shader::new(
                &context,
                info.vertex_shader_uri.as_ref().expect("Must have a vertex shader per pipeline"),
                info.fragment_shader_uri.as_ref().expect("Must have a fragment shader per pipeline"),
                info.geometry_shader_uri.as_ref().map(|s| s.as_str()),
            )
        }).collect::<Vec<Shader>>();
        let shader_stage_create_infos = shaders.iter().map(|shader| {
            shader.generate_shader_stage_create_infos()
        }).collect::<Vec<Vec<PipelineShaderStageCreateInfo>>>();

        let pipeline_create_infos = create_info
            .pipeline_create_infos
            .iter().enumerate()
            .map(|(i, info)| {
                GraphicsPipelineCreateInfo::default()
                    .stages(&shader_stage_create_infos[i])
                    .vertex_input_state(&info.pipeline_vertex_input_state_create_info)
                    .input_assembly_state(&info.pipeline_input_assembly_state_create_info)
                    .tessellation_state(&info.pipeline_tess_state_create_info)
                    .viewport_state(&viewport_state)
                    .rasterization_state(&info.pipeline_rasterization_state_create_info)
                    .multisample_state(&info.pipeline_multisample_state_create_info)
                    .depth_stencil_state(&info.pipeline_depth_stencil_state_create_info)
                    .color_blend_state(&info.pipeline_color_blend_state_create_info)
                    .dynamic_state(&dynamic_state_info)
                    .layout(pipeline_layout)
                    .render_pass(pass.renderpass)
            }).collect::<Vec<GraphicsPipelineCreateInfo>>();
        let vulkan_pipelines = context.device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            &pipeline_create_infos,
            None
        ).expect("Failed to create pipeline");
        let pipelines: Vec<Pipeline> = shaders
            .into_iter()
            .zip(vulkan_pipelines.into_iter())
            .map(|(shader, vk_pipeline)| Pipeline {
                shader,
                vulkan_pipeline: vk_pipeline,
            })
            .collect();
        Renderpass {
            context,

            pass: Arc::new(RefCell::new(pass)),
            descriptor_set: Arc::new(RefCell::new(descriptor_set)),
            pipelines,
            pipeline_layout,
            viewport: create_info.viewport,
            scissor: create_info.scissor,
        }
    } }

    /**
    * Defaults to rendering a fullscreen quad with no push constant
    */
    pub unsafe fn do_renderpass<F1: FnOnce(), F2: FnOnce()>(
        &self,
        current_frame: usize,
        command_buffer: vk::CommandBuffer,
        push_constant_action: Option<F1>,
        draw_action: Option<F2>,
        framebuffer_index: Option<usize>,
        do_transition: bool,
    ) { unsafe {
        let device = &self.context.device;
        self.begin_renderpass(current_frame, command_buffer, framebuffer_index);
        if let Some(push_constant_action) = push_constant_action { push_constant_action() };
        if let Some(draw_action) = draw_action { draw_action() } else {
            device.cmd_draw(command_buffer, 6, 1, 0, 0);
        };
        device.cmd_end_render_pass(command_buffer);
        if do_transition {
            self.pass.borrow().transition_to_readable(command_buffer, current_frame);
        }
    } }
    pub unsafe fn begin_renderpass(&self, current_frame: usize, command_buffer: vk::CommandBuffer, framebuffer_index: Option<usize>) { unsafe {
        let device = &self.context.device;
        device.cmd_begin_render_pass(
            command_buffer,
            &self.pass.borrow().get_pass_begin_info(current_frame, framebuffer_index, self.scissor),
            vk::SubpassContents::INLINE,
        );
        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipelines[0].vulkan_pipeline,
        );
        device.cmd_set_viewport(command_buffer, 0, &[self.viewport]);
        device.cmd_set_scissor(command_buffer, 0, &[self.scissor]);
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline_layout,
            0,
            &[self.descriptor_set.borrow().descriptor_sets[current_frame]],
            &[],
        );
    }}

    pub unsafe fn destroy(&mut self) { unsafe {
        for pipeline in self.pipelines.iter() {
            pipeline.shader.destroy();
            self.context.device.destroy_pipeline(pipeline.vulkan_pipeline, None);
        }
        let mut pass = self.pass.borrow_mut();
        if !pass.destroyed { pass.destroy() };
        self.context.device.destroy_pipeline_layout(self.pipeline_layout, None);
        self.descriptor_set.borrow_mut().destroy();
    } }
}
pub struct RenderpassCreateInfo<'a> {
    pub context: Arc<Context>,
    pub pass_create_info: Option<PassCreateInfo>,
    pub pass_ref: Option<Arc<RefCell<Pass>>>,
    pub descriptor_set_create_info: Option<DescriptorSetCreateInfo>,
    pub descriptor_set_ref: Option<Arc<RefCell<DescriptorSet>>>,
    pub viewport: vk::Viewport,
    pub scissor: vk::Rect2D,

    pub push_constant_info: Option<PushConstantRange>,

    pub pipeline_create_infos: Vec<PipelineCreateInfo<'a>>,

    pub dynamic_state: Vec<DynamicState>,

    has_manual_pipelines: bool,
}
impl<'a> RenderpassCreateInfo<'a> {
    /**
    * Defaults to pipeline create info intended for a fullscreen quad pass without blending or depth testing.
    */
    pub fn new(context: &Arc<Context>) -> Self {
        RenderpassCreateInfo {
            context: context.clone(),
            pass_create_info: Some(PassCreateInfo::new(context)),
            pass_ref: None,
            descriptor_set_create_info: Some(DescriptorSetCreateInfo::new(context)),
            descriptor_set_ref: None,
            viewport: vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: context.window.inner_size().width as f32,
                height: context.window.inner_size().height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            },
            scissor: vk::Rect2D {
                extent: vk::Extent2D { width: context.window.inner_size().width, height: context.window.inner_size().height },
                ..Default::default()
            },

            push_constant_info: None,

            pipeline_create_infos: vec![PipelineCreateInfo::new()],

            dynamic_state: vec![DynamicState::VIEWPORT, DynamicState::SCISSOR],

            has_manual_pipelines: false,
        }
    }

    pub fn resolution(mut self, resolution: vk::Extent2D) -> Self {
        self.viewport.width = resolution.width as f32;
        self.viewport.height = resolution.height as f32;
        self.scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: resolution,
        };
        self
    }
    pub fn pass_create_info(mut self, pass_create_info: PassCreateInfo) -> Self {
        self.pass_create_info = Some(pass_create_info);
        self.pass_ref = None;
        self
    }
    pub fn pass_ref(mut self, pass_ref: Arc<RefCell<Pass>>) -> Self {
        self.pass_ref = Some(pass_ref.clone());
        self.pass_create_info = None;
        self
    }
    pub fn add_pipeline_create_info(mut self, pipeline: PipelineCreateInfo<'a>) -> Self {
        if !self.has_manual_pipelines {
            self.pipeline_create_infos.clear();
            self.has_manual_pipelines = true
        }
        self.pipeline_create_infos.push(pipeline);
        self
    }
    /**
    * All descriptor set layouts must be identical per Renderpass
    */
    pub fn descriptor_set_create_info(mut self, descriptor_set_create_info: DescriptorSetCreateInfo) -> Self {
        self.descriptor_set_create_info = Some(descriptor_set_create_info);
        self
    }
    pub fn descriptor_set_ref(mut self, descriptor_set_ref: Arc<RefCell<DescriptorSet>>) -> Self {
        self.descriptor_set_ref = Some(descriptor_set_ref.clone());
        self.descriptor_set_create_info = None;
        self
    }
    pub fn viewport(mut self, viewport: vk::Viewport) -> Self {
        self.viewport = viewport;
        self
    }
    pub fn scissor(mut self, scissor: vk::Rect2D) -> Self {
        self.scissor = scissor;
        self
    }
    pub fn push_constant_range(mut self, push_constant_range: PushConstantRange) -> Self {
        self.push_constant_info = Some(push_constant_range);
        self
    }
    pub fn dynamic_state(mut self, dynamic_state: Vec<DynamicState>) -> Self {
        self.dynamic_state = dynamic_state;
        self
    }
}

pub struct Pipeline {
    shader: Shader,
    pub vulkan_pipeline: vk::Pipeline,
}
pub struct PipelineCreateInfo<'a> {
    pub vertex_shader_uri: Option<String>,
    pub geometry_shader_uri: Option<String>,
    pub fragment_shader_uri: Option<String>,

    pub pipeline_vertex_input_state_create_info: PipelineVertexInputStateCreateInfo<'a>,
    pub pipeline_input_assembly_state_create_info: PipelineInputAssemblyStateCreateInfo<'a>,
    pub pipeline_tess_state_create_info: PipelineTessellationStateCreateInfo<'a>,
    pub pipeline_rasterization_state_create_info: PipelineRasterizationStateCreateInfo<'a>,
    pub pipeline_multisample_state_create_info: PipelineMultisampleStateCreateInfo<'a>,
    pub pipeline_depth_stencil_state_create_info: PipelineDepthStencilStateCreateInfo<'a>,
    pub pipeline_color_blend_state_create_info: PipelineColorBlendStateCreateInfo<'a>,
}
impl<'a> PipelineCreateInfo<'a> {
    pub fn new() -> Self {
        let noop_stencil_state = StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            ..Default::default()
        };
        let null_blend_state = &PipelineColorBlendAttachmentState {
            blend_enable: vk::FALSE,  // Disable blending
            src_color_blend_factor: vk::BlendFactor::ONE,
            dst_color_blend_factor: vk::BlendFactor::ZERO,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        } as *const _ as *const _;
        PipelineCreateInfo {
            vertex_shader_uri: None,
            geometry_shader_uri: None,
            fragment_shader_uri: None,

            pipeline_vertex_input_state_create_info: PipelineVertexInputStateCreateInfo::default(),
            pipeline_input_assembly_state_create_info: PipelineInputAssemblyStateCreateInfo {
                topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                primitive_restart_enable: vk::FALSE,
                ..Default::default()
            },
            pipeline_tess_state_create_info: Default::default(),
            pipeline_rasterization_state_create_info: PipelineRasterizationStateCreateInfo {
                front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                cull_mode: vk::CullModeFlags::NONE,
                line_width: 1.0,
                polygon_mode: vk::PolygonMode::FILL,
                ..Default::default()
            },
            pipeline_multisample_state_create_info: PipelineMultisampleStateCreateInfo {
                rasterization_samples: SampleCountFlags::TYPE_1,
                ..Default::default()
            },
            pipeline_depth_stencil_state_create_info: PipelineDepthStencilStateCreateInfo {
                depth_test_enable: 0,
                depth_write_enable: 0,
                depth_compare_op: vk::CompareOp::NEVER,
                front: noop_stencil_state,
                back: noop_stencil_state,
                max_depth_bounds: 0.0,
                ..Default::default()
            },
            pipeline_color_blend_state_create_info: PipelineColorBlendStateCreateInfo {
                p_attachments: null_blend_state,
                attachment_count: 1,
                ..Default::default()
            },
        }
    }

    pub fn vertex_shader_uri(mut self, vertex_shader_uri: String) -> Self {
        self.vertex_shader_uri = Some(vertex_shader_uri);
        self
    }
    pub fn geometry_shader_uri(mut self, geometry_shader_uri: String) -> Self {
        self.geometry_shader_uri = Some(geometry_shader_uri);
        self
    }
    pub fn fragment_shader_uri(mut self, fragment_shader_uri: String) -> Self {
        self.fragment_shader_uri = Some(fragment_shader_uri);
        self
    }

    pub fn pipeline_vertex_input_state(mut self, pipeline_vertex_input_state: PipelineVertexInputStateCreateInfo<'a>) -> Self {
        self.pipeline_vertex_input_state_create_info = pipeline_vertex_input_state;
        self
    }
    pub fn pipeline_input_assembly_state(mut self, pipeline_input_assembly_state: PipelineInputAssemblyStateCreateInfo<'a>) -> Self {
        self.pipeline_input_assembly_state_create_info = pipeline_input_assembly_state;
        self
    }
    pub fn pipeline_tess_state(mut self, pipeline_tess_state: PipelineTessellationStateCreateInfo<'a>) -> Self {
        self.pipeline_tess_state_create_info = pipeline_tess_state;
        self
    }
    pub fn pipeline_rasterization_state(mut self, pipeline_rasterization_state: PipelineRasterizationStateCreateInfo<'a>) -> Self {
        self.pipeline_rasterization_state_create_info = pipeline_rasterization_state;
        self
    }
    pub fn pipeline_multisample_state(mut self, pipeline_multisample_state: PipelineMultisampleStateCreateInfo<'a>) -> Self {
        self.pipeline_multisample_state_create_info = pipeline_multisample_state;
        self
    }
    pub fn pipeline_depth_stencil_state(mut self, pipeline_depth_stencil_state: PipelineDepthStencilStateCreateInfo<'a>) -> Self {
        self.pipeline_depth_stencil_state_create_info = pipeline_depth_stencil_state;
        self
    }
    pub fn pipeline_color_blend_state_create_info(mut self, pipeline_color_blend_state_create_info: PipelineColorBlendStateCreateInfo<'a>) -> Self {
        self.pipeline_color_blend_state_create_info = pipeline_color_blend_state_create_info;
        self
    }
}

pub struct Pass {
    context: Arc<Context>,

    pub renderpass: vk::RenderPass,
    pub textures: Vec<Vec<Texture>>,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub clear_values: Vec<ClearValue>,

    pub destroyed: bool,
}
impl Pass {
    pub unsafe fn new(create_info: PassCreateInfo) -> Self { unsafe {
        let mut textures = Vec::new();
        let context = create_info.context.clone();
        let mut has_cubemap = false;
        let has_depth = create_info.depth_attachment_create_info.is_some();
        for frame in 0..create_info.frames_in_flight {
            let mut frame_textures = Vec::new();
            for texture in 0..create_info.color_attachment_create_infos.len() {
                let create_info = &create_info.color_attachment_create_infos[texture][frame];
                if create_info.is_cubemap { has_cubemap = true };
                frame_textures.push(Texture::new(create_info));
            }
            if has_depth { frame_textures.push(Texture::new(&create_info.depth_attachment_create_info.as_ref().unwrap()[frame])) };
            textures.push(frame_textures);
        }

        let mut attachments_vec = Vec::new();
        let mut color_attachment_refs_vec = Vec::new();
        let mut depth_attachment_index = None;
        for (i, texture) in textures[0].iter().enumerate() {
                attachments_vec.push(
                    vk::AttachmentDescription {
                        format: texture.format,
                        samples: texture.samples,
                        load_op: texture.load_op,
                        store_op: vk::AttachmentStoreOp::STORE,
                        stencil_load_op: vk::AttachmentLoadOp::CLEAR,
                        stencil_store_op: vk::AttachmentStoreOp::STORE,
                        initial_layout: texture.initial_layout,
                        final_layout: if texture.is_depth { vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL } else { vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL },
                        ..Default::default()
                    }
                );
                if !texture.is_depth {
                    color_attachment_refs_vec.push(vk::AttachmentReference {
                        attachment: i as u32,
                        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    })
                } else {
                    depth_attachment_index = Some(i as u32);
                }
            }

        let attachments = attachments_vec.as_slice();
        let color_attachment_refs = color_attachment_refs_vec.as_slice();
        let depth_attachment_ref = if has_depth { Some(vk::AttachmentReference {
            attachment: depth_attachment_index.unwrap(),
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        }) } else { None };

        let dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ..Default::default()
        }];

        let mut subpass = vk::SubpassDescription::default()
            .color_attachments(&color_attachment_refs)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);
        let depth_ref = depth_attachment_ref.unwrap_or_default();
        if has_depth {
            subpass = subpass.depth_stencil_attachment(&depth_ref);
        }

        let mut renderpass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies);
        let cubemap_view_mask: u32 = 0b0011_1111; // 6 views
        let mut cubemap_multiview_info = vk::RenderPassMultiviewCreateInfo::default()
            .view_masks(std::slice::from_ref(&cubemap_view_mask))
            .correlation_masks(std::slice::from_ref(&cubemap_view_mask));
        if has_cubemap {
            renderpass_create_info = renderpass_create_info.push_next(&mut cubemap_multiview_info);
        }

        let renderpass = context
            .device
            .create_render_pass(&renderpass_create_info, None)
            .unwrap();

        let resolution_info = &create_info
            .depth_attachment_create_info.as_ref()
            .unwrap_or_else(|| &create_info.color_attachment_create_infos[0])[0];
        let width = resolution_info.width;
        let height = resolution_info.height;
        let array_layers = if has_cubemap { 1 } else { resolution_info.array_layers };

        let clear_values;
        let framebuffers: Vec<vk::Framebuffer> = {
            clear_values = textures[0].iter().map(|texture| {texture.clear_value}).collect();
            let framebuffers = (0..create_info.frames_in_flight)
                .map(|i| {
                    let framebuffer_attachments_vec: Vec<ImageView> = textures[i].iter().map(|texture| { texture.device_texture.borrow().image_view }).collect();
                    let framebuffer_attachments = framebuffer_attachments_vec.as_slice();

                    let framebuffer_create_info = vk::FramebufferCreateInfo::default()
                        .render_pass(renderpass)
                        .attachments(&framebuffer_attachments)
                        .width(width)
                        .height(height)
                        .layers(array_layers);

                    let fb = context
                        .device
                        .create_framebuffer(&framebuffer_create_info, None)
                        .expect("Failed to create framebuffer");
                    //println!("Created framebuffer[{}]: 0x{:x}", i, fb.as_raw());
                    fb
                })
                .collect();
            framebuffers
        };

        Self {
            context,

            renderpass,
            textures,
            framebuffers,
            clear_values,

            destroyed: false
        }
    } }
    pub unsafe fn new_present_pass(base: &VkBase) -> Self { unsafe {
        let context = &base.context;
        let mut textures = Vec::new();

        let mut attachments_vec = Vec::new();
        let mut color_attachment_refs_vec = Vec::new();

        attachments_vec.push(vk::AttachmentDescription {
            format: context.surface_format.format,
            samples: SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        });
        color_attachment_refs_vec.push(vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        });

        let attachments = attachments_vec.as_slice();
        let color_attachment_refs = color_attachment_refs_vec.as_slice();

        let dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ..Default::default()
        }];

        let subpass = vk::SubpassDescription::default()
            .color_attachments(&color_attachment_refs)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

        let renderpass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies);

        let renderpass = context
            .device
            .create_render_pass(&renderpass_create_info, None)
            .unwrap();

        let mut width = context.window.inner_size().width;
        let mut height = context.window.inner_size().height;
        let mut array_layers= 1;

        let clear_values;
        let framebuffers: Vec<vk::Framebuffer> = {
            clear_values = vec![
                ClearValue {
                    color: ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 0.0],
                    },
                },
            ];
            let present_framebuffers: Vec<vk::Framebuffer> = base
                .present_image_views
                .iter()
                .map(|&present_image_view| {
                    let framebuffer_attachments = [present_image_view];
                    let framebuffer_create_info = vk::FramebufferCreateInfo::default()
                        .render_pass(renderpass)
                        .attachments(&framebuffer_attachments)
                        .width(context.window.inner_size().width)
                        .height(context.window.inner_size().height)
                        .layers(1);
                    let fb = base
                        .device
                        .create_framebuffer(&framebuffer_create_info, None)
                        .expect("Failed to create framebuffer");
                    //println!("Created present framebuffer: 0x{:x}", fb.as_raw());
                    fb
                })
                .collect();

            present_framebuffers
        };

        Self {
            context: context.clone(),

            renderpass,
            textures,
            framebuffers,
            clear_values,

            destroyed: false
        }
    } }

    pub fn get_pass_begin_info(&self, current_frame: usize, framebuffer_index: Option<usize>, scissor: vk::Rect2D) -> vk::RenderPassBeginInfo {
        vk::RenderPassBeginInfo::default()
            .render_pass(self.renderpass)
            .framebuffer(self.framebuffers[framebuffer_index.unwrap_or(current_frame)])
            .render_area(scissor)
            .clear_values(&self.clear_values)
    }

    pub unsafe fn transition_to_readable(&self, command_buffer: vk::CommandBuffer, frame: usize) { unsafe {
        for tex_idx in 0..self.textures[frame].len() {
            let tex = &self.textures[frame][tex_idx];
            let (old_layout, new_layout, src_access_mask, aspect_mask, stage) = if tex.is_depth {
                (
                    vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                    vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                    vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                    if tex.has_stencil { ImageAspectFlags::DEPTH | ImageAspectFlags::STENCIL } else { ImageAspectFlags::DEPTH },
                    vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                )
            } else {
                (
                    vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    ImageAspectFlags::COLOR,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                )
            };

            let barrier = vk::ImageMemoryBarrier {
                s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                old_layout,
                new_layout,
                src_access_mask,
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: tex.device_texture.borrow().image,
                subresource_range: ImageSubresourceRange {
                    aspect_mask,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: tex.array_layers,
                },
                ..Default::default()
            };

            self.context.device.cmd_pipeline_barrier(
                command_buffer,
                stage,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    } }

    pub unsafe fn destroy(&mut self) { unsafe {
        if !self.destroyed {
            for framebuffer in &self.framebuffers {
                self.context.device.destroy_framebuffer(*framebuffer, None);
            }
            for frame_textures in &mut self.textures {
                for texture in frame_textures {
                    texture.destroy();
                }
            }
            self.context.device.destroy_render_pass(self.renderpass, None);
            self.destroyed = true;
        }
    }}
}
pub struct PassCreateInfo {
    context: Arc<Context>,
    pub frames_in_flight: usize,
    pub color_attachment_create_infos: Vec<Vec<TextureCreateInfo>>,
    pub depth_attachment_create_info: Option<Vec<TextureCreateInfo>>,
}
impl PassCreateInfo {
    pub fn new(context: &Arc<Context>) -> PassCreateInfo {
        PassCreateInfo {
            context: context.clone(),
            frames_in_flight: MAX_FRAMES_IN_FLIGHT,
            color_attachment_create_infos: Vec::new(),
            depth_attachment_create_info: None,
        }
    }
    pub fn frames_in_flight(mut self, frames_in_flight: usize) -> Self {
        self.frames_in_flight = frames_in_flight;
        self
    }
    pub fn add_color_attachment_info(mut self, color_attachment_create_info: TextureCreateInfo) -> Self {
        self.color_attachment_create_infos.push(vec![color_attachment_create_info; self.frames_in_flight]);
        self
    }
    pub fn grab_attachment(mut self, other: &Pass, attachment_index: usize, load_op: vk::AttachmentLoadOp, initial_layout: vk::ImageLayout) -> Self {
        let mut create_infos = Vec::new();
        for frame in 0..other.textures.len() {
            let texture = &other.textures[frame][attachment_index];
            create_infos.push(
                TextureCreateInfo::new(&self.context)
                    .base_on_preexisting(texture)
                    .load_op(load_op)
                    .initial_layout(initial_layout)
            );
        }
        self.color_attachment_create_infos.push(create_infos);
        self
    }
    pub fn grab_depth_attachment(mut self, other: &Pass, attachment_index: usize, load_op: vk::AttachmentLoadOp, initial_layout: vk::ImageLayout) -> Self {
        let mut create_infos = Vec::new();
        for frame in 0..other.textures.len() {
            let texture = &other.textures[frame][attachment_index];
            create_infos.push(
                TextureCreateInfo::new(&self.context)
                    .base_on_preexisting(texture)
                    .load_op(load_op)
                    .initial_layout(initial_layout)
            );
        }
        self.depth_attachment_create_info = Some(create_infos);
        self
    }
    pub fn depth_attachment_info(mut self, depth_attachment_create_info: TextureCreateInfo) -> Self {
        self.depth_attachment_create_info = Some(vec![depth_attachment_create_info; self.frames_in_flight]);
        self
    }
}

#[derive(Clone)]
pub struct Texture {
    context: Arc<Context>,

    pub device_texture: Arc<RefCell<DeviceTexture>>,

    pub clear_value: ClearValue,
    pub format: Format,
    pub resolution: Extent3D,
    pub array_layers: u32,
    pub samples: SampleCountFlags,
    pub is_depth: bool,
    pub has_stencil: bool,

    pub load_op: vk::AttachmentLoadOp,
    pub initial_layout: vk::ImageLayout,
}
impl Texture {
    pub unsafe fn new(create_info: &TextureCreateInfo) -> Self { unsafe {
        if let Some(preexisting) = &create_info.preexisting {
            println!("{}, {}", preexisting.resolution.width, preexisting.resolution.height);
            let mut new = preexisting.clone();
            new.load_op = create_info.load_op;
            new.initial_layout = create_info.initial_layout;
            new
        } else {
            if create_info.depth < 1 { panic!("texture depth is too low"); }
            let mut usage = create_info.usage_flags;
            usage = usage | if create_info.is_depth || create_info.has_stencil { ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT } else { ImageUsageFlags::COLOR_ATTACHMENT };
            let mut flags = vk::ImageCreateFlags::empty();
            if create_info.is_cubemap { flags |= vk::ImageCreateFlags::CUBE_COMPATIBLE };
            let layer_count = if create_info.is_cubemap { 6 } else { create_info.array_layers };
            let color_image_create_info = vk::ImageCreateInfo {
                s_type: vk::StructureType::IMAGE_CREATE_INFO,
                image_type: if create_info.depth > 1 { vk::ImageType::TYPE_3D } else if create_info.height > 1 { vk::ImageType::TYPE_2D } else { vk::ImageType::TYPE_1D },
                extent: Extent3D { width: create_info.width, height: create_info.height, depth: create_info.depth },
                mip_levels: 1,
                array_layers: layer_count,
                format: create_info.format,
                tiling: vk::ImageTiling::OPTIMAL,
                initial_layout: create_info.initial_layout,
                usage,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                samples: create_info.samples,
                flags,
                ..Default::default()
            };

            let context = create_info.context.clone();

            let image = context.device.create_image(&color_image_create_info, None).expect("Failed to create image");
            let image_memory_req = context.device.get_image_memory_requirements(image);
            let image_alloc_info = vk::MemoryAllocateInfo {
                s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
                allocation_size: image_memory_req.size,
                memory_type_index: find_memorytype_index(
                    &image_memory_req,
                    &context.instance.get_physical_device_memory_properties(context.pdevice),
                    MemoryPropertyFlags::DEVICE_LOCAL,
                ).expect("unable to get mem type index for texture image"),
                ..Default::default()
            };
            // println!("this texture requires {}", image_memory_req.size);
            let image_memory = context.device.allocate_memory(&image_alloc_info, None).expect("Failed to allocate image memory");
            context.device.bind_image_memory(image, image_memory, 0).expect("Failed to bind image memory");
            let image_view_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                image,
                view_type: if create_info.is_cubemap {
                    vk::ImageViewType::CUBE
                } else { if create_info.array_layers > 1 {
                    if create_info.height > 1 {
                        vk::ImageViewType::TYPE_2D_ARRAY
                    } else {
                        vk::ImageViewType::TYPE_1D_ARRAY
                    }
                } else if create_info.depth > 1 {
                    vk::ImageViewType::TYPE_3D
                } else if create_info.height > 1 {
                    vk::ImageViewType::TYPE_2D
                } else {
                    vk::ImageViewType::TYPE_1D
                } },
                format: create_info.format,
                subresource_range: ImageSubresourceRange {
                    aspect_mask: if create_info.is_depth {
                        ImageAspectFlags::DEPTH
                    } else {
                        ImageAspectFlags::COLOR
                    },
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count,
                    ..Default::default()
                },
                ..Default::default()
            };
            let stencil_image_view = if create_info.has_stencil {
                Some(context.device.create_image_view(&vk::ImageViewCreateInfo {
                    s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                    image,
                    view_type: if create_info.array_layers > 1 {
                        if create_info.height > 1 {
                            vk::ImageViewType::TYPE_2D_ARRAY
                        } else {
                            vk::ImageViewType::TYPE_1D_ARRAY
                        }
                    } else if create_info.depth > 1 {
                        vk::ImageViewType::TYPE_3D
                    } else if create_info.height > 1 {
                        vk::ImageViewType::TYPE_2D
                    } else {
                        vk::ImageViewType::TYPE_1D
                    },
                    format: create_info.format,
                    subresource_range: ImageSubresourceRange {
                        aspect_mask: ImageAspectFlags::STENCIL,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: create_info.array_layers,
                        ..Default::default()
                    },
                    ..Default::default()
                }, None).expect("failed to create stencil image view"))
            } else { None };

            Self {
                context,

                device_texture: Arc::new(RefCell::new(DeviceTexture {
                    image,
                    image_view: create_info.context.device.create_image_view(&image_view_info, None).expect("failed to create image view"),
                    stencil_image_view,
                    device_memory: image_memory,

                    destroyed: false
                })),

                clear_value: Self::clear_value_for_format(create_info.format, create_info.clear_value),
                format: create_info.format,
                resolution: Extent3D::default().width(create_info.width).height(create_info.height).depth(create_info.depth),
                array_layers: create_info.array_layers,
                samples: create_info.samples,
                is_depth: create_info.is_depth,
                has_stencil: create_info.has_stencil,

                load_op: create_info.load_op,
                initial_layout: create_info.initial_layout,
            }
        }
    } }
    pub unsafe fn sample(&self, x: i32, y: i32, z: i32) -> Box<[f32]> { unsafe {
        let context = &self.context;

        let pixel_size = match self.format {
            Format::R8G8B8A8_UNORM => 4,
            Format::R32_SFLOAT => 4,
            Format::R32G32B32A32_SFLOAT => 16,
            Format::R16G16B16A16_SFLOAT => 8,
            Format::R16_UINT => 2,
            Format::R16G16_UINT => 4,
            Format::R32_UINT => 4,
            Format::D32_SFLOAT => 4,
            _ => panic!("Unsupported format {:?}", self.format),
        };

        let image = self.device_texture.borrow().image;

        let buffer_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            size: pixel_size as u64,
            usage: vk::BufferUsageFlags::TRANSFER_DST,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let staging_buffer = context.device.create_buffer(&buffer_info, None).unwrap();

        let req = context.device.get_buffer_memory_requirements(staging_buffer);

        let mem_index = context
            .instance
            .get_physical_device_memory_properties(context.pdevice)
            .memory_types
            .iter()
            .enumerate()
            .find(|(i, mt)| {
                (req.memory_type_bits & (1u32 << i)) != 0 &&
                    mt.property_flags.contains(
                        MemoryPropertyFlags::HOST_VISIBLE |
                            MemoryPropertyFlags::HOST_COHERENT
                    )
            })
            .map(|(i, _)| i)
            .unwrap() as u32;

        let alloc_info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
            allocation_size: req.size,
            memory_type_index: mem_index,
            ..Default::default()
        };
        let staging_memory = context.device.allocate_memory(&alloc_info, None).unwrap();
        context.device.bind_buffer_memory(staging_buffer, staging_memory, 0).unwrap();

        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,

            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: if self.is_depth {
                    ImageAspectFlags::DEPTH
                } else {
                    ImageAspectFlags::COLOR
                },
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },

            image_offset: vk::Offset3D { x, y, z },
            image_extent: Extent3D { width: 1, height: 1, depth: 1 },
        };

        let subresource_range = ImageSubresourceRange {
            aspect_mask: if self.is_depth {
                ImageAspectFlags::DEPTH
            } else {
                ImageAspectFlags::COLOR
            },
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        context.transition_image_layout(
            image,
            subresource_range,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        );

        {
            let cmd = context.begin_single_time_commands(1)[0];

            context.device.cmd_copy_image_to_buffer(
                cmd,
                image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                staging_buffer,
                &[region],
            );

            context.end_single_time_commands(vec![cmd]);
        }

        let ptr = self.context.device.map_memory(
            staging_memory,
            0,
            pixel_size as u64,
            vk::MemoryMapFlags::empty(),
        ).unwrap();

        let slice = std::slice::from_raw_parts(ptr as *const u8, pixel_size);

        let result: Box<[f32]> = match self.format {
            Format::R8G8B8A8_UNORM => {
                Box::new([
                    slice[0] as f32 / 255.0,
                    slice[1] as f32 / 255.0,
                    slice[2] as f32 / 255.0,
                    slice[3] as f32 / 255.0,
                ])
            }

            Format::R32_SFLOAT => {
                Box::new([f32::from_ne_bytes(slice.try_into().unwrap())])
            }

            Format::R32G32B32A32_SFLOAT => {
                let mut f = [0.0; 4];
                for i in 0..4 {
                    f[i] = f32::from_ne_bytes(slice[i*4..i*4+4].try_into().unwrap());
                }
                Box::new(f)
            }

            Format::R16G16B16A16_SFLOAT => {
                use half::f16;
                let mut f = [0.0f32; 4];
                for i in 0..4 {
                    let bits: u16 = u16::from_ne_bytes(slice[i * 2..i * 2 + 2].try_into().unwrap());
                    f[i] = f16::from_bits(bits).to_f32();
                }
                Box::new(f)
            }

            Format::R16_UINT => {
                Box::new([u16::from_ne_bytes(slice.try_into().unwrap()) as f32])
            }

            Format::R16G16_UINT => {
                Box::new([
                    u16::from_ne_bytes(slice[0..2].try_into().unwrap()) as f32,
                    u16::from_ne_bytes(slice[2..4].try_into().unwrap()) as f32,
                ])
            }

            Format::R32_UINT => {
                Box::new([u32::from_ne_bytes(slice.try_into().unwrap()) as f32])
            }

            Format::D32_SFLOAT => {
                Box::new([f32::from_ne_bytes(slice.try_into().unwrap())])
            }

            _ => panic!("ehlp"),
        };
        self.context.device.unmap_memory(staging_memory);

        self.context.device.destroy_buffer(staging_buffer, None);
        self.context.device.free_memory(staging_memory, None);

        context.transition_image_layout(
            image,
            subresource_range,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        result
    } }
    fn sizeof_texel(format: Format) -> usize {
        match format {
            Format::R8_UNORM => 1,
            Format::R8G8B8A8_UNORM => 4,
            Format::R16_UINT | Format::R16_SFLOAT => 2,
            Format::R32_SFLOAT => 4,
            Format::R32G32B32A32_SFLOAT => 16,
            _ => unimplemented!("format {:?} not supported", format),
        }
    }
    fn decode_texel(format: Format, bytes: &[u8]) -> Box<[f32]> {
        match format {
            Format::R8_UNORM =>
                Box::new([bytes[0] as f32 / 255.0]),

            Format::R8G8B8A8_UNORM =>
                Box::new([
                    bytes[0] as f32 / 255.0,
                    bytes[1] as f32 / 255.0,
                    bytes[2] as f32 / 255.0,
                    bytes[3] as f32 / 255.0,
                ]),

            Format::R16_SFLOAT => {
                let v = u16::from_ne_bytes([bytes[0], bytes[1]]);
                Box::new([half::f16::from_bits(v).to_f32()])
            }

            Format::R32_SFLOAT =>
                Box::new([f32::from_ne_bytes(bytes.try_into().unwrap())]),

            Format::R32G32B32A32_SFLOAT => {
                let r = f32::from_ne_bytes(bytes[0..4].try_into().unwrap());
                let g = f32::from_ne_bytes(bytes[4..8].try_into().unwrap());
                let b = f32::from_ne_bytes(bytes[8..12].try_into().unwrap());
                let a = f32::from_ne_bytes(bytes[12..16].try_into().unwrap());
                Box::new([r, g, b, a])
            }

            _ => unimplemented!("decode for {:?}", format),
        }
    }
    pub unsafe fn destroy(&mut self) { {
        self.device_texture.borrow_mut().destroy(&self.context);
    } }

    fn clear_value_for_format(format: Format, clear: [f32; 4]) -> ClearValue {
        match format {
            // Depth/stencil formats
            Format::D16_UNORM
            | Format::D32_SFLOAT
            | Format::D24_UNORM_S8_UINT
            | Format::D32_SFLOAT_S8_UINT => {
                ClearValue {
                    depth_stencil: ClearDepthStencilValue {
                        depth: clear[0],
                        stencil: clear[1] as u32,
                    },
                }
            }

            // Float color formats
            Format::R16_SFLOAT
            | Format::R16G16_SFLOAT
            | Format::R16G16B16A16_SFLOAT
            | Format::R32_SFLOAT
            | Format::R32G32_SFLOAT
            | Format::R32G32B32A32_SFLOAT
            | Format::R8_UNORM
            | Format::R16_UNORM
            | Format::R8G8B8A8_UNORM
            | Format::B8G8R8A8_UNORM => {
                ClearValue {
                    color: ClearColorValue { float32: clear },
                }
            }

            // Unsigned integer formats
            Format::R8_UINT
            | Format::R16_UINT
            | Format::R32_UINT
            | Format::R16G16_UINT
            | Format::R8G8B8A8_UINT
            | Format::R16G16B16A16_UINT
            | Format::R32G32B32A32_UINT => {
                ClearValue {
                    color: ClearColorValue {
                        uint32: [
                            clear[0] as u32,
                            clear[1] as u32,
                            clear[2] as u32,
                            clear[3] as u32,
                        ],
                    },
                }
            }

            // Signed integer formats
            Format::R8_SINT
            | Format::R16_SINT
            | Format::R32_SINT
            | Format::R8G8B8A8_SINT
            | Format::R16G16B16A16_SINT
            | Format::R32G32B32A32_SINT => {
                ClearValue {
                    color: ClearColorValue {
                        int32: [
                            clear[0] as i32,
                            clear[1] as i32,
                            clear[2] as i32,
                            clear[3] as i32,
                        ],
                    },
                }
            }

            _ => panic!("Unsupported format {:?} for clear value", format),
        }
    }
}
#[derive(Clone)]
pub struct DeviceTexture {
    pub image: vk::Image,
    pub image_view: ImageView,
    pub stencil_image_view: Option<ImageView>,
    pub device_memory: DeviceMemory,

    pub destroyed: bool,
}
impl DeviceTexture {
    pub unsafe fn destroy(&mut self, context: &Arc<Context>) {
        if !self.destroyed { unsafe {
            self.destroyed = true;
            context.device.destroy_image(self.image, None);
            context.device.destroy_image_view(self.image_view, None);
            context.device.free_memory(self.device_memory, None);
            if let Some(stencil_view) = self.stencil_image_view {
                context.device.destroy_image_view(stencil_view, None);
            }
        }
    } }
}
#[derive(Clone)]
pub struct TextureCreateInfo {
    context: Arc<Context>,
    pub preexisting: Option<Texture>,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub samples: SampleCountFlags,
    pub format: Format,
    pub is_depth: bool,
    pub is_cubemap: bool,
    pub has_stencil: bool,
    pub usage_flags: ImageUsageFlags,
    pub array_layers: u32,
    pub clear_value: [f32; 4],
    pub load_op: vk::AttachmentLoadOp,
    pub initial_layout: vk::ImageLayout,
}
impl TextureCreateInfo {
    pub fn new(context: &Arc<Context>) -> TextureCreateInfo {
        TextureCreateInfo {
            context: context.clone(),
            preexisting: None,
            width: context.window.inner_size().width,
            height: context.window.inner_size().height,
            depth: 1,
            samples: SampleCountFlags::TYPE_1,
            format: Format::R16G16B16A16_SFLOAT,
            is_depth: false,
            is_cubemap: false,
            has_stencil: false,
            usage_flags: ImageUsageFlags::SAMPLED,
            array_layers: 1,
            clear_value: [0.0; 4],
            load_op: vk::AttachmentLoadOp::CLEAR,
            initial_layout: vk::ImageLayout::UNDEFINED,
        }
    }
    pub fn width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }
    pub fn height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }
    pub fn depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }
    pub fn samples(mut self, samples: SampleCountFlags) -> Self {
        self.samples = samples;
        self
    }
    pub fn format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }
    pub fn is_depth(mut self, is_depth: bool) -> Self {
        self.is_depth = is_depth;
        self
    }
    pub fn is_cubemap(mut self, is_cubemap: bool) -> Self {
        self.is_cubemap = is_cubemap;
        self
    }
    pub fn usage_flags(mut self, usage_flags: ImageUsageFlags) -> Self {
        self.usage_flags = usage_flags;
        self
    }
    pub fn add_usage_flag(mut self, usage_flag: ImageUsageFlags) -> Self {
        self.usage_flags = self.usage_flags | usage_flag;
        self
    }
    pub fn load_op(mut self, load_op: vk::AttachmentLoadOp) -> Self {
        self.load_op = load_op;
        self
    }
    pub fn array_layers(mut self, array_layers: u32) -> Self {
        self.array_layers = array_layers;
        self
    }
    pub fn clear_value(mut self, clear_value: [f32; 4]) -> Self {
        self.clear_value = clear_value;
        self
    }
    pub fn resolution_denominator(mut self, denominator: u32) -> Self {
        self.width = (self.width / denominator).max(1);
        self.height = (self.height / denominator).max(1);
        self.depth = (self.depth / denominator).max(1);
        self
    }
    pub fn has_stencil(mut self, has_stencil: bool) -> Self {
        self.has_stencil = has_stencil;
        self
    }
    pub fn base_on_preexisting(mut self, preexisting: &Texture) -> Self {
        self.preexisting = Some(preexisting.clone());
        self
    }
    pub fn initial_layout(mut self, initial_layout: vk::ImageLayout) -> Self {
        self.initial_layout = initial_layout;
        self
    }
}

pub struct DescriptorSet {
    context: Arc<Context>,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub descriptor_set_layout: DescriptorSetLayout,
    pub descriptor_pool: DescriptorPool,
    pub descriptors: Vec<Descriptor>,

    pub destroyed: bool,
}
impl DescriptorSet {
    pub unsafe fn new(create_info: DescriptorSetCreateInfo) -> DescriptorSet { unsafe {
        let context = create_info.context.clone();
        let mut has_dynamic = false;
        let mut has_update_after_bind = false;
        let mut variable_descriptor_count = 0;
        let descriptor_pool_sizes = create_info.descriptors.iter().map(|d| {
            if d.is_dynamic {has_dynamic = true; variable_descriptor_count = d.descriptor_count;}
            if d.binding_flags.is_some() {
                if d.binding_flags.unwrap().contains(DescriptorBindingFlags::UPDATE_AFTER_BIND) {
                    has_update_after_bind = true
                }
            }
            DescriptorPoolSize {
                ty: d.descriptor_type,
                descriptor_count: create_info.frames_in_flight as u32 * d.descriptor_count,
            }
        }).collect::<Vec<DescriptorPoolSize>>();
        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            pool_size_count: descriptor_pool_sizes.len() as u32,
            p_pool_sizes: descriptor_pool_sizes.as_ptr(),
            max_sets: create_info.frames_in_flight as u32,
            flags: if has_dynamic || has_update_after_bind {DescriptorPoolCreateFlags::UPDATE_AFTER_BIND} else {DescriptorPoolCreateFlags::empty()},
            ..Default::default()
        };
        let descriptor_pool = context.device.create_descriptor_pool(&descriptor_pool_create_info, None).expect("failed to create descriptor pool");

        let mut i = 0;

        let bindings = create_info.descriptors.iter().map(|d| {
            let ret = vk::DescriptorSetLayoutBinding {
                binding: i as u32,
                descriptor_type: d.descriptor_type,
                descriptor_count: d.descriptor_count,
                stage_flags: d.shader_stages,
                ..Default::default()
            };
            i = i + 1;
            ret
        }).collect::<Vec<vk::DescriptorSetLayoutBinding>>();
        let binding_flags = create_info.descriptors.iter().map(|d| {
            if d.is_dynamic {
                DescriptorBindingFlags::PARTIALLY_BOUND |
                DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT |
                DescriptorBindingFlags::UPDATE_AFTER_BIND
            } else {
                d.binding_flags.unwrap_or_default()
            }
        }).collect::<Vec<DescriptorBindingFlags>>();
        let binding_flags_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_BINDING_FLAGS_CREATE_INFO,
            binding_count: binding_flags.len() as u32,
            p_binding_flags: binding_flags.as_ptr(),
            ..Default::default()
        };
        let descriptor_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            p_next: &binding_flags_info as *const _ as *const c_void,
            binding_count: bindings.len() as u32,
            p_bindings: bindings.as_ptr(),
            flags: if has_dynamic || has_update_after_bind {DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL} else {DescriptorSetLayoutCreateFlags::empty()},
            ..Default::default()
        };
        let descriptor_set_layout = context.device.create_descriptor_set_layout(&descriptor_layout_create_info, None).expect("failed to create descriptor set layout");
        let descriptor_set_layouts = vec![descriptor_set_layout; create_info.frames_in_flight];

        let variable_counts = vec![variable_descriptor_count; create_info.frames_in_flight];
        let variable_count_info = vk::DescriptorSetVariableDescriptorCountAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_VARIABLE_DESCRIPTOR_COUNT_ALLOCATE_INFO,
            descriptor_set_count: create_info.frames_in_flight as u32,
            p_descriptor_counts: variable_counts.as_ptr(),
            ..Default::default()
        };
        let alloc_info = vk::DescriptorSetAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next: if has_dynamic {&variable_count_info as *const _ as *const c_void} else {std::ptr::null()},
            descriptor_pool: descriptor_pool,
            descriptor_set_count: create_info.frames_in_flight as u32,
            p_set_layouts: descriptor_set_layouts.as_ptr(),
            ..Default::default()
        };
        let descriptor_sets = context.device.allocate_descriptor_sets(&alloc_info)
            .expect("failed to allocate descriptor sets");

        let mut binding = 0;
        let infos = create_info.descriptors.iter().map(|d| {
            binding = binding + 1;
            match d.descriptor_type {
                DescriptorType::UNIFORM_BUFFER => {
                    Some((DescriptorType::UNIFORM_BUFFER, binding - 1, d))
                }
                DescriptorType::STORAGE_BUFFER => {
                    Some((DescriptorType::STORAGE_BUFFER, binding - 1, d))
                }
                DescriptorType::COMBINED_IMAGE_SAMPLER => {
                    if d.is_dynamic { Some((DescriptorType::COMBINED_IMAGE_SAMPLER, binding - 1, d)) } else { None }
                }
                _ => { 
                    None
                }
            }
        }).collect::<Vec<Option<(DescriptorType, u32, &Descriptor)>>>();
        for i in 0..create_info.frames_in_flight {
            let mut buffer_infos = Vec::new();
            let descriptor_writes = infos.iter().filter_map(|maybe_info| {
                let info = maybe_info.as_ref()?;
                let buffer_info_idx = match info.0 {
                    DescriptorType::UNIFORM_BUFFER => {
                        buffer_infos.push(vk::DescriptorBufferInfo {
                            buffer: info.2.owned_buffers.0[i],
                            offset: info.2.offset.unwrap(),
                            range: info.2.range.unwrap(),
                        });
                        Some(buffer_infos.len() - 1)
                    }
                    DescriptorType::STORAGE_BUFFER => {
                        buffer_infos.push(vk::DescriptorBufferInfo {
                            buffer: info.2.buffer_refs[i],
                            offset: info.2.offset.unwrap(),
                            range: info.2.range.unwrap(),
                        });
                        Some(buffer_infos.len() - 1)
                    }
                    _ => None,
                };
                let image_infos = info.2.image_infos;
                Some(vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: descriptor_sets[i],
                    dst_binding: info.1,
                    dst_array_element: 0,
                    descriptor_type: info.0,
                    descriptor_count: info.2.descriptor_count,
                    p_buffer_info: buffer_info_idx
                        .map_or(std::ptr::null(), |idx| &buffer_infos[idx]),
                    p_image_info: image_infos
                        .map_or(std::ptr::null(), |imgs| imgs),
                    ..Default::default()
                })
            }).collect::<Vec<_>>();
            context.device.update_descriptor_sets(&descriptor_writes, &[]);
        }
        DescriptorSet {
            context,
            descriptor_sets,
            descriptor_set_layout,
            descriptor_pool,
            descriptors: create_info.descriptors,

            destroyed: false,
        }
    } }
    pub unsafe fn destroy(&mut self) { unsafe {
        if !self.destroyed {
            self.context.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.context.device.destroy_descriptor_pool(self.descriptor_pool, None);
            for descriptor in &self.descriptors {
                descriptor.destroy();
            }
            self.destroyed = true;
        }
    } }
}
pub struct DescriptorSetCreateInfo {
    context: Arc<Context>,
    pub descriptors: Vec<Descriptor>,
    pub frames_in_flight: usize,
}
impl DescriptorSetCreateInfo {
    pub fn new(context: &Arc<Context>) -> DescriptorSetCreateInfo {
        DescriptorSetCreateInfo {
            context: context.clone(),
            descriptors: Vec::new(),
            frames_in_flight: MAX_FRAMES_IN_FLIGHT,
        }
    }
    pub fn add_descriptor(mut self, descriptor: Descriptor) -> Self {
        self.descriptors.push(descriptor);
        self
    }
    pub fn frames_in_flight(mut self, frames_in_flight: usize) -> Self {
        self.frames_in_flight = frames_in_flight;
        self
    }
}

pub struct Descriptor {
    context: Arc<Context>,

    pub descriptor_type: DescriptorType,
    pub shader_stages: ShaderStageFlags,
    pub is_dynamic: bool,
    pub offset: Option<DeviceSize>,
    pub range: Option<DeviceSize>,
    pub owned_buffers: (Vec<Buffer>, Vec<DeviceMemory>, Vec<*mut c_void>),
    pub buffer_refs: Vec<Buffer>,
    pub image_infos: Option<*const DescriptorImageInfo>,
    pub descriptor_count: u32,
    pub binding_flags: Option<DescriptorBindingFlags>
}
impl Descriptor {
    pub unsafe fn new(create_info: &DescriptorCreateInfo) -> Self { unsafe {
        let context = &create_info.context;
        match create_info.descriptor_type {
            DescriptorType::UNIFORM_BUFFER => {
                let mut uniform_buffers = Vec::new();
                let mut uniform_buffers_memory = Vec::new();
                let mut uniform_buffers_mapped = Vec::new();
                for i in 0..create_info.frames_in_flight {
                    uniform_buffers.push(Buffer::null());
                    uniform_buffers_memory.push(DeviceMemory::null());
                    context.create_buffer(
                        create_info.size.expect("DescriptorCreateInfo of type UNIFORM_BUFFER does not contain buffer size"),
                        vk::BufferUsageFlags::UNIFORM_BUFFER,
                        create_info.memory_property_flags,
                        &mut uniform_buffers[i],
                        &mut uniform_buffers_memory[i],
                    );
                    uniform_buffers_mapped.push(context.device.map_memory(
                        uniform_buffers_memory[i],
                        0,
                        create_info.size.expect("DescriptorCreateInfo of type UNIFORM_BUFFER does not contain buffer size"),
                        vk::MemoryMapFlags::empty()
                    ).expect("failed to map uniform buffer"));
                }
                Descriptor {
                    context: context.clone(),
                    descriptor_type: DescriptorType::UNIFORM_BUFFER,
                    shader_stages: create_info.shader_stages,
                    is_dynamic: create_info.dynamic,
                    offset: Some(create_info.offset),
                    range: Some(create_info.range),
                    descriptor_count: 1,
                    owned_buffers: (uniform_buffers, uniform_buffers_memory, uniform_buffers_mapped),
                    buffer_refs: Vec::new(),
                    image_infos: None,
                    binding_flags: create_info.binding_flags,
                }
            }
            DescriptorType::STORAGE_BUFFER => {
                Descriptor {
                    context: context.clone(),
                    descriptor_type: DescriptorType::STORAGE_BUFFER,
                    shader_stages: create_info.shader_stages,
                    is_dynamic: create_info.dynamic,
                    offset: Some(create_info.offset),
                    range: Some(create_info.range),
                    descriptor_count: 1,
                    owned_buffers: (Vec::new(), Vec::new(), Vec::new()),
                    buffer_refs: create_info.buffers.clone().unwrap(),
                    image_infos: None,
                    binding_flags: create_info.binding_flags,
                }
            }
            DescriptorType::COMBINED_IMAGE_SAMPLER => {
                Descriptor {
                    context: context.clone(),
                    descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
                    shader_stages: create_info.shader_stages,
                    is_dynamic: create_info.dynamic,
                    offset: None,
                    range: None,
                    descriptor_count: create_info.image_infos.as_ref().map_or(1, |v| v.len()) as u32,
                    owned_buffers: (Vec::new(), Vec::new(), Vec::new()),
                    buffer_refs: create_info.buffers.clone().as_ref().map_or(Vec::new(), |v| v.to_vec()),
                    image_infos: create_info.image_infos.as_ref().map_or(None, |i| Some(i.as_ptr())),
                    binding_flags: create_info.binding_flags
                }
            }
            _ => {
                Descriptor {
                    context: context.clone(),
                    descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
                    shader_stages: ShaderStageFlags::FRAGMENT,
                    is_dynamic: false,
                    offset: None,
                    range: None,
                    owned_buffers: (Vec::new(), Vec::new(), Vec::new()),
                    buffer_refs: Vec::new(),
                    image_infos: None,
                    descriptor_count: 0,
                    binding_flags: None
                }
            }
        }
    } }

    pub unsafe fn destroy(&self) { unsafe {
        if self.descriptor_type == DescriptorType::UNIFORM_BUFFER || self.descriptor_type == DescriptorType::STORAGE_BUFFER {
            for i in 0..self.owned_buffers.0.len() {
                self.context.device.destroy_buffer(self.owned_buffers.0[i], None);
                self.context.device.free_memory(self.owned_buffers.1[i], None);
            }
        }
    } }
}
pub struct DescriptorCreateInfo {
    pub context: Arc<Context>,
    pub frames_in_flight: usize,
    pub descriptor_type: DescriptorType,
    pub size: Option<u64>,
    pub shader_stages: ShaderStageFlags,
    pub offset: DeviceSize,
    pub range: DeviceSize,
    pub buffers: Option<Vec<Buffer>>,
    pub image_infos: Option<Vec<DescriptorImageInfo>>,
    pub memory_property_flags: MemoryPropertyFlags,
    pub dynamic: bool,
    pub binding_flags: Option<DescriptorBindingFlags>,
}
impl DescriptorCreateInfo {
    pub fn new(context: &Arc<Context>) -> DescriptorCreateInfo {
        DescriptorCreateInfo {
            context: context.clone(),
            frames_in_flight: MAX_FRAMES_IN_FLIGHT,
            descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
            size: None,
            shader_stages: ShaderStageFlags::FRAGMENT,
            offset: 0,
            range: vk::WHOLE_SIZE,
            buffers: None,
            image_infos: None,
            memory_property_flags: MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            dynamic: false,
            binding_flags: None,
        }
    }
    pub fn frames_in_flight(mut self, frames_in_flight: usize) -> Self {
        self.frames_in_flight = frames_in_flight;
        self
    }
    pub fn descriptor_type(mut self, descriptor_type: DescriptorType) -> Self {
        self.descriptor_type = descriptor_type;
        self
    }
    pub fn size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }
    pub fn shader_stages(mut self, shader_stages: ShaderStageFlags) -> Self {
        self.shader_stages = shader_stages;
        self
    }
    pub fn memory_property_flags(mut self, memory_property_flags: MemoryPropertyFlags) -> Self {
        self.memory_property_flags = memory_property_flags;
        self
    }
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }
    pub fn range(mut self, range: DeviceSize) -> Self {
        self.range = range;
        self
    }
    pub fn buffers(mut self, buffers: Vec<Buffer>) -> Self {
        self.buffers = Some(buffers);
        self
    }
    pub fn image_infos(mut self, image_infos: Vec<DescriptorImageInfo>) -> Self {
        self.image_infos = Some(image_infos);
        self
    }
    pub fn dynamic(mut self, dynamic: bool) -> Self {
        self.dynamic = dynamic;
        self
    }
    pub fn binding_flags(mut self, binding_flags: DescriptorBindingFlags) -> Self {
        self.binding_flags = Some(binding_flags);
        self
    }
}

pub struct Shader {
    context: Arc<Context>,

    pub vertex_module: ShaderModule,
    pub geometry_module: Option<ShaderModule>,
    pub fragment_module: ShaderModule,
}
impl Shader {
    pub unsafe fn new(context: &Arc<Context>, vert_path: &str, frag_path: &str, geometry_path: Option<&str>) -> Self { unsafe {
        let mut vertex_spv_file = Cursor::new(load_file(&(SHADER_PATH.to_owned() + vert_path)).unwrap());
        let mut frag_spv_file = Cursor::new(load_file(&(SHADER_PATH.to_owned() + frag_path)).unwrap());
        let geometry_spv_file: Option<Cursor<Vec<u8>>> = if geometry_path.is_some() {
            Some(Cursor::new(load_file(&(SHADER_PATH.to_owned() + geometry_path.unwrap())).unwrap()))
        } else {
            None
        };

        let vertex_code = read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
        let vertex_shader_info = ShaderModuleCreateInfo::default().code(&vertex_code);

        let frag_code = read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
        let frag_shader_info = ShaderModuleCreateInfo::default().code(&frag_code);

        let geometry_code: Option<Vec<u32>> = if let Some(mut geo_file) = geometry_spv_file {
            Some(read_spv(&mut geo_file).expect("Failed to read geometry shader spv file"))
        } else {
            None
        };
        let geometry_shader_info: Option<ShaderModuleCreateInfo> = geometry_code
            .as_ref()
            .map(|code| ShaderModuleCreateInfo::default().code(code));

        let vertex_shader_module = context
            .device
            .create_shader_module(&vertex_shader_info, None)
            .expect("Vertex shader module error");
        let geometry_shader_module: Option<ShaderModule> = if geometry_path.is_some() {
            Some(context
                .device
                .create_shader_module(&geometry_shader_info.unwrap(), None)
                .expect("Geometry shader module error"))
        } else {
            None
        };
        let fragment_shader_module = context
            .device
            .create_shader_module(&frag_shader_info, None)
            .expect("Fragment shader module error");
        Shader {
            context: context.clone(),
            vertex_module: vertex_shader_module,
            geometry_module: geometry_shader_module,
            fragment_module: fragment_shader_module,
        }
    } }

    pub fn generate_shader_stage_create_infos(&self) -> Vec<PipelineShaderStageCreateInfo<'_>> {
        let shader_entry_name = c"main";
        if self.geometry_module.is_some() {
            vec![
                PipelineShaderStageCreateInfo {
                    module: self.vertex_module,
                    p_name: shader_entry_name.as_ptr(),
                    stage: ShaderStageFlags::VERTEX,
                    ..Default::default()
                },
                PipelineShaderStageCreateInfo {
                    module: self.geometry_module.unwrap(),
                    p_name: shader_entry_name.as_ptr(),
                    stage: ShaderStageFlags::GEOMETRY,
                    ..Default::default()
                },
                PipelineShaderStageCreateInfo {
                    s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                    module: self.fragment_module,
                    p_name: shader_entry_name.as_ptr(),
                    stage: ShaderStageFlags::FRAGMENT,
                    ..Default::default()
                },
            ]
        } else {
            vec![
                PipelineShaderStageCreateInfo {
                    module: self.vertex_module,
                    p_name: shader_entry_name.as_ptr(),
                    stage: ShaderStageFlags::VERTEX,
                    ..Default::default()
                },
                PipelineShaderStageCreateInfo {
                    s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                    module: self.fragment_module,
                    p_name: shader_entry_name.as_ptr(),
                    stage: ShaderStageFlags::FRAGMENT,
                    ..Default::default()
                },
            ]
        }
    }

    pub fn destroy(&self) { unsafe {
        self.context.device.destroy_shader_module(self.vertex_module, None);
        if self.geometry_module.is_some() {
            self.context.device.destroy_shader_module(self.geometry_module.unwrap(), None);
        }
        self.context.device.destroy_shader_module(self.fragment_module, None);
    } }
}