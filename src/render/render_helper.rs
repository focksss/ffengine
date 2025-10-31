use std::ffi::c_void;
use std::fs::File;
use std::io::{BufWriter, Cursor};
use std::path::Path;
use ash::{vk, Device, Instance};
use ash::util::read_spv;
use ash::vk::{Buffer, ClearColorValue, ClearDepthStencilValue, ClearValue, DescriptorBindingFlags, DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateFlags, DescriptorPoolSize, DescriptorSetLayout, DescriptorSetLayoutCreateFlags, DescriptorType, DeviceMemory, DeviceSize, DynamicState, Extent3D, Format, GraphicsPipelineCreateInfo, ImageAspectFlags, ImageSubresourceRange, ImageUsageFlags, MemoryPropertyFlags, PhysicalDevice, PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo, PipelineDepthStencilStateCreateInfo, PipelineInputAssemblyStateCreateInfo, PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo, PipelineShaderStageCreateInfo, PipelineTessellationStateCreateInfo, PipelineVertexInputStateCreateInfo, PushConstantRange, SampleCountFlags, ShaderModule, ShaderModuleCreateInfo, ShaderStageFlags, StencilOpState};
use crate::MAX_FRAMES_IN_FLIGHT;
use crate::render::*;

const SHADER_PATH: &str = "resources\\shaders\\spv\\";

pub struct Renderpass {
    pub device: Device,
    pub draw_command_buffers: Vec<vk::CommandBuffer>,

    pub pass: Pass,
    pub descriptor_set: DescriptorSet,
    pub shader: Shader,
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    pub viewport: vk::Viewport,
    pub scissor: vk::Rect2D,
}
impl Renderpass {
    pub unsafe fn new(create_info: RenderpassCreateInfo) -> Renderpass { unsafe {
        let base = create_info.base;
        let pass = Pass::new(create_info.pass_create_info);
        let descriptor_set = DescriptorSet::new(create_info.descriptor_set_create_info);
        let shader = Shader::new(
            base,
            &*create_info.vertex_shader_uri.expect("Must have a vertex shader per pipeline"),
            &*create_info.fragment_shader_uri.expect("Must have a fragment shader per pipeline"),
            create_info.geometry_shader_uri.as_ref().map(|s| s.as_str()),
        );
        let shader_stages_create_infos = shader.generate_shader_stage_create_infos();
        let pipeline_layout = base
            .device
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo {
                    s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                    set_layout_count: 1,
                    p_set_layouts: &descriptor_set.descriptor_set_layout,
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
        let pipeline_create_info = GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages_create_infos)
            .vertex_input_state(&create_info.pipeline_vertex_input_state_create_info)
            .input_assembly_state(&create_info.pipeline_input_assembly_state_create_info)
            .tessellation_state(&create_info.pipeline_tess_state_create_info)
            .viewport_state(&viewport_state)
            .rasterization_state(&create_info.pipeline_rasterization_state_create_info)
            .multisample_state(&create_info.pipeline_multisample_state_create_info)
            .depth_stencil_state(&create_info.pipeline_depth_stencil_state_create_info)
            .color_blend_state(&create_info.pipeline_color_blend_state_create_info)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(pass.renderpass);
        let pipeline = base.device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            &[pipeline_create_info],
            None
        ).expect("Failed to create pipeline")[0];
        Renderpass {
            device: base.device.clone(),
            draw_command_buffers: base.draw_command_buffers.clone(),

            pass,
            descriptor_set,
            shader,
            pipeline,
            pipeline_layout,
            viewport: create_info.viewport,
            scissor: create_info.scissor,
        }
    } }

    pub fn get_pass_begin_info(&self, current_frame: usize, framebuffer_index: Option<usize>) -> vk::RenderPassBeginInfo {
        vk::RenderPassBeginInfo::default()
            .render_pass(self.pass.renderpass)
            .framebuffer(self.pass.framebuffers[framebuffer_index.unwrap_or(current_frame)])
            .render_area(self.scissor)
            .clear_values(&self.pass.clear_values)
    }
    /**
    * Defaults to rendering a fullscreen quad with no push constant
    */
    pub unsafe fn do_renderpass<F1: FnOnce(), F2: FnOnce()>(
        &self,
        current_frame: usize,
        command_buffer: vk::CommandBuffer,
        push_constant_action: Option<F1>,
        draw_action: Option<F2>,
        framebuffer_index: Option<usize>,)
    { unsafe {
        let device = &self.device;
        self.begin_renderpass(current_frame, command_buffer, framebuffer_index);
        if let Some(push_constant_action) = push_constant_action { push_constant_action() };
        if let Some(draw_action) = draw_action { draw_action() } else {
            device.cmd_draw(command_buffer, 6, 1, 0, 0);
        };
        device.cmd_end_render_pass(command_buffer);
        self.pass.transition_to_readable(command_buffer, current_frame);
    } }
    pub unsafe fn begin_renderpass(&self, current_frame: usize, command_buffer: vk::CommandBuffer, framebuffer_index: Option<usize>) { unsafe {
        let device = &self.device;
        device.cmd_begin_render_pass(
            command_buffer,
            &self.get_pass_begin_info(current_frame, framebuffer_index),
            vk::SubpassContents::INLINE,
        );
        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline,
        );
        device.cmd_set_viewport(command_buffer, 0, &[self.viewport]);
        device.cmd_set_scissor(command_buffer, 0, &[self.scissor]);
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.pipeline_layout,
            0,
            &[self.descriptor_set.descriptor_sets[current_frame]],
            &[],
        );
    }}

    pub unsafe fn destroy(&self) { unsafe {
        self.shader.destroy();
        self.pass.destroy();
        self.device.destroy_pipeline_layout(self.pipeline_layout, None);
        self.device.destroy_pipeline(self.pipeline, None);
        self.descriptor_set.destroy();
    } }
}
pub struct RenderpassCreateInfo<'a> {
    pub base: &'a VkBase,
    pub pass_create_info: PassCreateInfo<'a>,
    pub descriptor_set_create_info: DescriptorSetCreateInfo<'a>,
    pub vertex_shader_uri: Option<String>,
    pub geometry_shader_uri: Option<String>,
    pub fragment_shader_uri: Option<String>,
    pub viewport: vk::Viewport,
    pub scissor: vk::Rect2D,

    pub push_constant_info: Option<PushConstantRange>,

    pub pipeline_vertex_input_state_create_info: PipelineVertexInputStateCreateInfo<'a>,
    pub pipeline_input_assembly_state_create_info: PipelineInputAssemblyStateCreateInfo<'a>,
    pub pipeline_tess_state_create_info: PipelineTessellationStateCreateInfo<'a>,
    pub pipeline_rasterization_state_create_info: PipelineRasterizationStateCreateInfo<'a>,
    pub pipeline_multisample_state_create_info: PipelineMultisampleStateCreateInfo<'a>,
    pub pipeline_depth_stencil_state_create_info: PipelineDepthStencilStateCreateInfo<'a>,
    pub pipeline_color_blend_state_create_info: PipelineColorBlendStateCreateInfo<'a>,
    pub dynamic_state: Vec<DynamicState>,

}
impl<'a> RenderpassCreateInfo<'a> {
    /**
    * Defaults to pipeline create info intended for a fullscreen quad pass without blending or depth testing.
    */
    pub fn new(base: &'a VkBase) -> Self {
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
        RenderpassCreateInfo {
            base,
            pass_create_info: PassCreateInfo::new(base),
            descriptor_set_create_info: DescriptorSetCreateInfo::new(base),
            vertex_shader_uri: None,
            geometry_shader_uri: None,
            fragment_shader_uri: None,
            viewport: vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: base.surface_resolution.width as f32,
                height: base.surface_resolution.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            },
            scissor: base.surface_resolution.into(),

            push_constant_info: None,

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
            dynamic_state: vec![DynamicState::VIEWPORT, DynamicState::SCISSOR],
        }
    }

    pub fn pass_create_info(mut self, pass_create_info: PassCreateInfo<'a>) -> Self {
        self.pass_create_info = pass_create_info;
        self
    }
    /**
    * All descriptor set layouts must be identical per Renderpass
    */
    pub fn descriptor_set_create_info(mut self, descriptor_set_create_info: DescriptorSetCreateInfo<'a>) -> Self {
        self.descriptor_set_create_info = descriptor_set_create_info;
        self
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
    pub fn dynamic_state(mut self, dynamic_state: Vec<DynamicState>) -> Self {
        self.dynamic_state = dynamic_state;
        self
    }
}

pub struct Pass {
    pub device: Device,

    pub renderpass: vk::RenderPass,
    pub textures: Vec<Vec<Texture>>,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub clear_values: Vec<ClearValue>,
}
impl Pass {
    pub unsafe fn new(create_info: PassCreateInfo) -> Self { unsafe {
        let mut textures = Vec::new();
        let base = create_info.base;
        let has_depth = create_info.depth_attachment_create_info.is_some();
        if !create_info.is_present_pass {
            for _ in 0..create_info.frames_in_flight {
                let mut frame_textures = Vec::new();
                for texture in 0..create_info.color_attachment_create_infos.len() {
                    frame_textures.push(Texture::new(&create_info.color_attachment_create_infos[texture]));
                }
                if has_depth { frame_textures.push(Texture::new(&create_info.depth_attachment_create_info.unwrap())) };
                textures.push(frame_textures);
            }
        }

        let mut attachments_vec = Vec::new();
        let mut color_attachment_refs_vec = Vec::new();
        let mut depth_attachment_index = None;
        if !create_info.is_present_pass {
            for (i, texture) in textures[0].iter().enumerate() {
                attachments_vec.push(
                    vk::AttachmentDescription {
                        format: texture.format,
                        samples: texture.samples,
                        load_op: vk::AttachmentLoadOp::CLEAR,
                        store_op: vk::AttachmentStoreOp::STORE,
                        initial_layout: vk::ImageLayout::UNDEFINED,
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
        } else {
            attachments_vec.push(vk::AttachmentDescription {
                format: base.surface_format.format,
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

        let renderpass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies);

        let renderpass = base
            .device
            .create_render_pass(&renderpass_create_info, None)
            .unwrap();


        let mut width = base.surface_resolution.width;
        let mut height = base.surface_resolution.height;
        let mut array_layers= 1;
        if !create_info.is_present_pass {
            let resolution_info = create_info
                .depth_attachment_create_info
                .unwrap_or_else(|| create_info.color_attachment_create_infos[0]);
            width = resolution_info.width;
            height = resolution_info.height;
            array_layers = resolution_info.array_layers;
        }

        let clear_values;
        let framebuffers: Vec<vk::Framebuffer> = if !create_info.is_present_pass {
            clear_values = textures[0].iter().map(|texture| {texture.clear_value}).collect();
            let framebuffers = (0..create_info.frames_in_flight)
                .map(|i| {
                    let framebuffer_attachments_vec: Vec<vk::ImageView> = textures[i].iter().map(|texture| { texture.image_view }).collect();
                    let framebuffer_attachments = framebuffer_attachments_vec.as_slice();

                    let framebuffer_create_info = vk::FramebufferCreateInfo::default()
                        .render_pass(renderpass)
                        .attachments(&framebuffer_attachments)
                        .width(width)
                        .height(height)
                        .layers(array_layers);

                    let fb = base
                        .device
                        .create_framebuffer(&framebuffer_create_info, None)
                        .expect("Failed to create framebuffer");
                    //println!("Created framebuffer[{}]: 0x{:x}", i, fb.as_raw());
                    fb
                })
                .collect();
            framebuffers
        } else {
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
                        .width(base.surface_resolution.width)
                        .height(base.surface_resolution.height)
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
            device: base.device.clone(),

            renderpass,
            textures,
            framebuffers,
            clear_values
        }
    } }

    pub unsafe fn transition_to_readable(&self, command_buffer: vk::CommandBuffer, frame: usize) { unsafe {
        for tex_idx in 0..self.textures[frame].len() {
            let tex = &self.textures[frame][tex_idx];
            let (old_layout, new_layout, src_access_mask, aspect_mask, stage) = if tex.is_depth {
                (
                    vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                    vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                    vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                    ImageAspectFlags::DEPTH,
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
                image: tex.image,
                subresource_range: ImageSubresourceRange {
                    aspect_mask,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: tex.array_layers,
                },
                ..Default::default()
            };

            self.device.cmd_pipeline_barrier(
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

    pub unsafe fn destroy(&self) { unsafe {
        for framebuffer in &self.framebuffers {
            self.device.destroy_framebuffer(*framebuffer, None);
        }
        for frame_textures in &self.textures { for texture in frame_textures {
            texture.destroy();
        }}
        self.device.destroy_render_pass(self.renderpass, None);
    }}
}
pub struct PassCreateInfo<'a> {
    pub base: &'a VkBase,
    pub frames_in_flight: usize,
    pub color_attachment_create_infos: Vec<TextureCreateInfo<'a>>,
    pub depth_attachment_create_info: Option<TextureCreateInfo<'a>>,
    pub is_present_pass: bool,
}
impl<'a> PassCreateInfo<'a> {
    pub fn new(base: &'a VkBase) -> PassCreateInfo<'a> {
        PassCreateInfo {
            base,
            frames_in_flight: MAX_FRAMES_IN_FLIGHT,
            color_attachment_create_infos: Vec::new(),
            depth_attachment_create_info: None,
            is_present_pass: false,
        }
    }
    pub fn frames_in_flight(mut self, frames_in_flight: usize) -> Self {
        self.frames_in_flight = frames_in_flight;
        self
    }
    pub fn add_color_attachment_info(mut self, color_attachment_create_info: TextureCreateInfo<'a>) -> Self {
        self.color_attachment_create_infos.push(color_attachment_create_info);
        self
    }
    pub fn depth_attachment_info(mut self, depth_attachment_create_info: TextureCreateInfo<'a>) -> Self {
        self.depth_attachment_create_info = Some(depth_attachment_create_info);
        self
    }
    pub fn set_is_present_pass(mut self, is_present_pass: bool) -> Self {
        self.is_present_pass = is_present_pass;
        self
    }
}

pub struct Texture {
    pub device: Device,

    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub device_memory: DeviceMemory,

    pub clear_value: ClearValue,
    pub format: Format,
    pub resolution: Extent3D,
    pub array_layers: u32,
    pub samples: SampleCountFlags,
    pub is_depth: bool,
}
impl Texture {
    pub unsafe fn new(create_info: &TextureCreateInfo) -> Self { unsafe {
        if create_info.depth < 1 { panic!("texture depth is too low"); }
        let color_image_create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            image_type: if create_info.depth > 1 { vk::ImageType::TYPE_3D } else if create_info.height > 1 { vk::ImageType::TYPE_2D } else { vk::ImageType::TYPE_1D },
            extent: Extent3D { width: create_info.width, height: create_info.height, depth: create_info.depth },
            mip_levels: 1,
            array_layers: create_info.array_layers,
            format: create_info.format,
            tiling: vk::ImageTiling::OPTIMAL,
            initial_layout: vk::ImageLayout::UNDEFINED,
            usage: if create_info.is_depth { ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT } else { ImageUsageFlags::COLOR_ATTACHMENT } | create_info.usage_flags,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            samples: create_info.samples,
            ..Default::default()
        };
        let image = create_info.device.create_image(&color_image_create_info, None).expect("Failed to create image");
        let image_memory_req = create_info.device.get_image_memory_requirements(image);
        let image_alloc_info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
            allocation_size: image_memory_req.size,
            memory_type_index: find_memorytype_index(
                &image_memory_req,
                &create_info.instance.get_physical_device_memory_properties(*create_info.p_device),
                MemoryPropertyFlags::DEVICE_LOCAL,
            ).expect("unable to get mem type index for texture image"),
            ..Default::default()
        };
        // println!("this texture requires {}", image_memory_req.size);
        let image_memory = create_info.device.allocate_memory(&image_alloc_info, None).expect("Failed to allocate image memory");
        create_info.device.bind_image_memory(image, image_memory, 0).expect("Failed to bind image memory");
        let image_view_info = vk::ImageViewCreateInfo {
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
                aspect_mask: if create_info.is_depth { ImageAspectFlags::DEPTH } else { ImageAspectFlags::COLOR },
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: create_info.array_layers,
                ..Default::default()
            },
            ..Default::default()
        };

        Self {
            device: create_info.device.clone(),

            image,
            image_view: create_info.device.create_image_view(&image_view_info, None).expect("failed to create image view"),
            device_memory: image_memory,

            clear_value: Self::clear_value_for_format(create_info.format, create_info.clear_value),
            format: create_info.format,
            resolution: Extent3D::default().width(create_info.width).height(create_info.height).depth(create_info.depth),
            array_layers: create_info.array_layers,
            samples: create_info.samples,
            is_depth: create_info.is_depth,
        }
    } }
    pub unsafe fn destroy(&self) { unsafe {
        self.device.destroy_image(self.image, None);
        self.device.destroy_image_view(self.image_view, None);
        self.device.free_memory(self.device_memory, None);
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
#[derive(Copy, Clone)]
pub struct TextureCreateInfo<'a> {
    pub device: &'a Device,
    pub p_device: &'a PhysicalDevice,
    pub instance: &'a Instance,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub samples: SampleCountFlags,
    pub format: Format,
    pub is_depth: bool,
    pub usage_flags: ImageUsageFlags,
    pub array_layers: u32,
    pub clear_value: [f32; 4],
}
impl TextureCreateInfo<'_> {
    pub fn new(base: &VkBase) -> TextureCreateInfo {
        TextureCreateInfo {
            device: &base.device,
            p_device: &base.pdevice,
            instance: &base.instance,
            width: base.surface_resolution.width,
            height: base.surface_resolution.height,
            depth: 1,
            samples: SampleCountFlags::TYPE_1,
            format: Format::R16G16B16A16_SFLOAT,
            is_depth: false,
            usage_flags: ImageUsageFlags::SAMPLED,
            array_layers: 1,
            clear_value: [0.0; 4],
        }
    }
    pub fn new_without_base<'a>(device: &'a Device, p_device: &'a PhysicalDevice, instance: &'a Instance, surface_resolution: &vk::Extent2D) -> TextureCreateInfo<'a> {
        TextureCreateInfo {
            device,
            p_device,
            instance,
            width: surface_resolution.width,
            height: surface_resolution.height,
            depth: 1,
            samples: SampleCountFlags::TYPE_1,
            format: Format::R16G16B16A16_SFLOAT,
            is_depth: false,
            usage_flags: ImageUsageFlags::SAMPLED,
            array_layers: 1,
            clear_value: [0.0; 4],
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
    pub fn usage_flags(mut self, usage_flags: ImageUsageFlags) -> Self {
        self.usage_flags = usage_flags;
        self
    }
    pub fn add_usage_flag(mut self, usage_flag: ImageUsageFlags) -> Self {
        self.usage_flags = self.usage_flags | usage_flag;
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
}

pub struct DescriptorSet {
    pub device: Device,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub descriptor_set_layout: DescriptorSetLayout,
    pub descriptor_pool: DescriptorPool,
    pub descriptors: Vec<Descriptor>,
}
impl DescriptorSet {
    pub unsafe fn new(create_info: DescriptorSetCreateInfo) -> DescriptorSet { unsafe {
        let base = create_info.base;
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
        let descriptor_pool = base.device.create_descriptor_pool(&descriptor_pool_create_info, None).expect("failed to create descriptor pool");

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
        let descriptor_set_layout = base.device.create_descriptor_set_layout(&descriptor_layout_create_info, None).expect("failed to create descriptor set layout");
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
        let descriptor_sets = base.device.allocate_descriptor_sets(&alloc_info)
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
            base.device.update_descriptor_sets(&descriptor_writes, &[]);
        }
        DescriptorSet {
            device: base.device.clone(),
            descriptor_sets,
            descriptor_set_layout,
            descriptor_pool,
            descriptors: create_info.descriptors,
        }
    } }
    pub unsafe fn destroy(&self) { unsafe {
        self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        self.device.destroy_descriptor_pool(self.descriptor_pool, None);
        for descriptor in &self.descriptors {
            descriptor.destroy();
        }
    } }
}
pub struct DescriptorSetCreateInfo<'a> {
    pub base: &'a VkBase,
    pub descriptors: Vec<Descriptor>,
    pub frames_in_flight: usize,
}
impl DescriptorSetCreateInfo<'_> {
    pub fn new(base: &VkBase) -> DescriptorSetCreateInfo {
        DescriptorSetCreateInfo {
            base,
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
    pub device: Device,

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
        let base = create_info.base;
        match create_info.descriptor_type {
            DescriptorType::UNIFORM_BUFFER => {
                let mut uniform_buffers = Vec::new();
                let mut uniform_buffers_memory = Vec::new();
                let mut uniform_buffers_mapped = Vec::new();
                for i in 0..create_info.frames_in_flight {
                    uniform_buffers.push(Buffer::null());
                    uniform_buffers_memory.push(DeviceMemory::null());
                    base.create_buffer(
                        create_info.size.expect("DescriptorCreateInfo of type UNIFORM_BUFFER does not contain buffer size"),
                        vk::BufferUsageFlags::UNIFORM_BUFFER,
                        create_info.memory_property_flags,
                        &mut uniform_buffers[i],
                        &mut uniform_buffers_memory[i],
                    );
                    uniform_buffers_mapped.push(base.device.map_memory(
                        uniform_buffers_memory[i],
                        0,
                        create_info.size.expect("DescriptorCreateInfo of type UNIFORM_BUFFER does not contain buffer size"),
                        vk::MemoryMapFlags::empty()
                    ).expect("failed to map uniform buffer"));
                }
                Descriptor {
                    device: base.device.clone(),
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
                    device: base.device.clone(),
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
                    device: base.device.clone(),
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
                    device: base.device.clone(),
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
                self.device.destroy_buffer(self.owned_buffers.0[i], None);
                self.device.free_memory(self.owned_buffers.1[i], None);
            }
        }
    } }
}
pub struct DescriptorCreateInfo<'a> {
    pub base: &'a VkBase,
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
impl DescriptorCreateInfo<'_> {
    pub fn new(base: &VkBase) -> DescriptorCreateInfo {
        DescriptorCreateInfo {
            base,
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
    pub device: Device,

    pub vertex_module: ShaderModule,
    pub geometry_module: Option<ShaderModule>,
    pub fragment_module: ShaderModule,
}
impl Shader {
    pub unsafe fn new(base: &VkBase, vert_path: &str, frag_path: &str, geometry_path: Option<&str>) -> Self { unsafe {
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

        let vertex_shader_module = base
            .device
            .create_shader_module(&vertex_shader_info, None)
            .expect("Vertex shader module error");
        let geometry_shader_module: Option<ShaderModule> = if geometry_path.is_some() {
            Some(base
                .device
                .create_shader_module(&geometry_shader_info.unwrap(), None)
                .expect("Geometry shader module error"))
        } else {
            None
        };
        let fragment_shader_module = base
            .device
            .create_shader_module(&frag_shader_info, None)
            .expect("Fragment shader module error");
        Shader {
            device: base.device.clone(),
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
        self.device.destroy_shader_module(self.vertex_module, None);
        if self.geometry_module.is_some() {
            self.device.destroy_shader_module(self.geometry_module.unwrap(), None);
        }
        self.device.destroy_shader_module(self.fragment_module, None);
    } }
}

pub struct ScreenshotManager<'a> {
    base: &'a VkBase,
    texture: &'a Texture,
    screenshot_pending: bool,
    staging_buffer: (Buffer, DeviceMemory),
}
impl<'a> ScreenshotManager<'a> {
    pub unsafe fn new(base: &'a VkBase, texture: &'a Texture) -> ScreenshotManager<'a> {
        ScreenshotManager {
            base,
            texture,
            screenshot_pending: false,
            staging_buffer: (Buffer::null(), DeviceMemory::null()),
        }
    }
    pub unsafe fn screenshot_queue(&mut self, texture: &'a Texture, layout: vk::ImageLayout, command_buffer: vk::CommandBuffer) { unsafe {
        let base = self.base;

        let bytes_per_pixel = match texture.format {
            Format::R8G8B8A8_SRGB | Format::R8G8B8A8_UNORM |
            Format::B8G8R8A8_SRGB | Format::B8G8R8A8_UNORM => 4,
            Format::R16G16B16A16_SFLOAT | Format::R16G16B16A16_UNORM => 8,
            Format::R32G32B32A32_SFLOAT => 16,
            _ => {
                eprintln!("Unsupported format for screenshot: {:?}", texture.format);
                return;
            }
        };

        let buffer_size = (texture.resolution.width * texture.resolution.height * bytes_per_pixel) as DeviceSize;

        let buffer_info = vk::BufferCreateInfo::default()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let staging_buffer = base.device.create_buffer(&buffer_info, None)
            .expect("Failed to create screenshot staging buffer");

        let mem_requirements = base.device.get_buffer_memory_requirements(staging_buffer);
        let memory_type_index = find_memorytype_index(
            &mem_requirements,
            &base.device_memory_properties,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        );

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type_index.unwrap());

        let staging_memory = base.device.allocate_memory(&alloc_info, None)
            .expect("Failed to allocate screenshot staging memory");

        base.device.bind_buffer_memory(staging_buffer, staging_memory, 0)
            .expect("Failed to bind screenshot staging buffer memory");

        let barrier = vk::ImageMemoryBarrier::default()
            .old_layout(layout)
            .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(texture.image)
            .subresource_range(ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ);

        base.device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );

        let buffer_image_copy = vk::BufferImageCopy::default()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(texture.resolution);

        base.device.cmd_copy_image_to_buffer(
            command_buffer,
            texture.image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            staging_buffer,
            &[buffer_image_copy],
        );

        let barrier_back = vk::ImageMemoryBarrier::default()
            .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
            .new_layout(layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(texture.image)
            .subresource_range(ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::TRANSFER_READ)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

        base.device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier_back],
        );

        self.texture = texture;
        self.screenshot_pending = true;
        self.staging_buffer = (staging_buffer, staging_memory);
    } }

    pub unsafe fn save_screenshot<P: AsRef<Path>>(&mut self, path: P) { unsafe {
        if !self.screenshot_pending {
            return;
        }

        let base = self.base;
        let texture = self.texture;

        let bytes_per_pixel = match texture.format {
            Format::R8G8B8A8_SRGB | Format::R8G8B8A8_UNORM |
            Format::B8G8R8A8_SRGB | Format::B8G8R8A8_UNORM => 4,
            Format::R16G16B16A16_SFLOAT | Format::R16G16B16A16_UNORM => 8,
            Format::R32G32B32A32_SFLOAT => 16,
            _ => {
                eprintln!("Unsupported format for screenshot save");
                return;
            }
        };

        let buffer_size = (texture.resolution.width * texture.resolution.height * bytes_per_pixel) as usize;

        let data_ptr = base.device.map_memory(
            self.staging_buffer.1,
            0,
            buffer_size as DeviceSize,
            vk::MemoryMapFlags::empty(),
        ).unwrap();

        let data_slice = std::slice::from_raw_parts(data_ptr as *const u8, buffer_size);

        let rgba_data = match texture.format {
            Format::R8G8B8A8_SRGB | Format::R8G8B8A8_UNORM => {
                data_slice.to_vec()
            }
            Format::B8G8R8A8_SRGB | Format::B8G8R8A8_UNORM => {
                let mut rgba = Vec::with_capacity(buffer_size);
                for chunk in data_slice.chunks_exact(4) {
                    rgba.push(chunk[2]); // R
                    rgba.push(chunk[1]); // G
                    rgba.push(chunk[0]); // B
                    rgba.push(chunk[3]); // A
                }
                rgba
            }
            Format::R16G16B16A16_SFLOAT => {
                let mut rgba = Vec::with_capacity((texture.resolution.width * texture.resolution.height * 4) as usize);
                for chunk in data_slice.chunks_exact(8) {
                    for i in 0..4 {
                        let bits = u16::from_le_bytes([chunk[i*2], chunk[i*2+1]]);
                        let value = half::f16::from_bits(bits).to_f32();
                        let byte = (value.clamp(0.0, 1.0) * 255.0) as u8;
                        rgba.push(byte);
                    }
                }
                rgba
            }
            Format::R16G16B16A16_UNORM => {
                let mut rgba = Vec::with_capacity((texture.resolution.width * texture.resolution.height * 4) as usize);
                for chunk in data_slice.chunks_exact(8) {
                    for i in 0..4 {
                        let value = u16::from_le_bytes([chunk[i*2], chunk[i*2+1]]);
                        rgba.push((value >> 8) as u8); // high byte
                    }
                }
                rgba
            }
            _ => {
                eprintln!("Unsupported format conversion");
                base.device.unmap_memory(self.staging_buffer.1);
                return;
            }
        };

        base.device.unmap_memory(self.staging_buffer.1);
        base.device.destroy_buffer(self.staging_buffer.0, None);
        base.device.free_memory(self.staging_buffer.1, None);
        self.staging_buffer = (Buffer::null(), DeviceMemory::null());

        let file = File::create(path).unwrap();
        let w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, texture.resolution.width, texture.resolution.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header().expect("Failed to write PNG header");
        writer.write_image_data(&rgba_data).expect("Failed to write PNG data");
        writer.finish().expect("Failed to finish PNG");

        self.screenshot_pending = false;
        println!("Screenshot saved!");
    } }
}