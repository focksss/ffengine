use std::io::Cursor;
use ash::{vk, Device, Instance};
use ash::util::read_spv;
use ash::vk::{ClearColorValue, ClearDepthStencilValue, ClearValue, Extent3D, Format, Handle, ImageAspectFlags, ImageSubresourceRange, ImageUsageFlags, MemoryPropertyFlags, PhysicalDevice, PipelineShaderStageCreateInfo, SampleCountFlags, ShaderModule};
use crate::vk_helper::{find_memorytype_index, load_file, VkBase};



pub struct Shader {
    pub modules: (ShaderModule, ShaderModule),
}
impl Shader {
    pub unsafe fn new(base: &VkBase, vert_path: &str, frag_path: &str) -> Self { unsafe {
        let mut vertex_spv_file = Cursor::new(load_file(vert_path).unwrap());
        let mut frag_spv_file = Cursor::new(load_file(frag_path).unwrap());

        let vertex_code = read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
        let vertex_shader_info = vk::ShaderModuleCreateInfo::default().code(&vertex_code);

        let frag_code = read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
        let frag_shader_info = vk::ShaderModuleCreateInfo::default().code(&frag_code);

        let vertex_shader_module = base
            .device
            .create_shader_module(&vertex_shader_info, None)
            .expect("Vertex shader module error");

        let fragment_shader_module = base
            .device
            .create_shader_module(&frag_shader_info, None)
            .expect("Fragment shader module error");
        Shader { modules: (vertex_shader_module, fragment_shader_module)}
    } }

    pub fn generate_shader_stage_create_infos(&self) -> [PipelineShaderStageCreateInfo<'_>; 2] {
        let shader_entry_name = c"main";
        [
            PipelineShaderStageCreateInfo {
                module: self.modules.0,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                module: self.modules.1,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ]
    }

    pub unsafe fn destroy(&self, base: &VkBase) { unsafe {
        base.device.destroy_shader_module(self.modules.0, None);
        base.device.destroy_shader_module(self.modules.1, None);
    } }
}

pub struct Pass {
    pub renderpass: vk::RenderPass,
    pub textures: Vec<Vec<Texture>>,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub clear_values: Vec<ClearValue>,
}
impl Pass {
    pub unsafe fn new(base: &VkBase, create_info: PassCreateInfo) -> Self { unsafe {
        let mut textures = Vec::new();
        if !create_info.is_present_pass {
            for frame in 0..create_info.frames_in_flight {
                let mut frame_textures = Vec::new();
                for texture in 0..create_info.color_attachment_create_infos.len() {
                    frame_textures.push(Texture::new(&create_info.color_attachment_create_infos[texture]));
                }
                frame_textures.push(Texture::new(&create_info.depth_attachment_create_info));
                textures.push(frame_textures);
            }
        }

        let mut attachments_vec = Vec::new();
        let mut color_attachment_refs_vec = Vec::new();
        let mut depth_attachment_index = 0;
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
                    depth_attachment_index = i as u32;
                }
            }
        } else {
            attachments_vec.push(vk::AttachmentDescription {
                format: base.color_texture.format,
                samples: base.msaa_samples,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                ..Default::default()
            });
            attachments_vec.push(vk::AttachmentDescription {
                format: base.depth_texture.format,
                samples: base.msaa_samples,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                ..Default::default()
            });
            attachments_vec.push(vk::AttachmentDescription {
                format: base.surface_format.format,
                samples: SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::DONT_CARE,
                store_op: vk::AttachmentStoreOp::STORE,
                stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                ..Default::default()
            });
            color_attachment_refs_vec.push(vk::AttachmentReference {
                attachment: 0,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            });
            depth_attachment_index = 1;
        }

        let attachments = attachments_vec.as_slice();
        let color_attachment_refs = color_attachment_refs_vec.as_slice();
        let depth_attachment_ref = vk::AttachmentReference {
            attachment: depth_attachment_index,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };

        let dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ..Default::default()
        }];

        let mut subpass = vk::SubpassDescription::default()
            .color_attachments(&color_attachment_refs)
            .depth_stencil_attachment(&depth_attachment_ref)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);
        if create_info.is_present_pass {
            subpass = subpass.resolve_attachments(&[vk::AttachmentReference {
                attachment: 2,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            }]);
        }

        let renderpass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies);

        let renderpass = base
            .device
            .create_render_pass(&renderpass_create_info, None)
            .unwrap();

        let mut clear_values = Vec::new();
        let framebuffers: Vec<vk::Framebuffer> = if !create_info.is_present_pass {
            clear_values = textures[0].iter().map(|texture| {texture.clear_value}).collect();
            let framebuffers = (0..create_info.frames_in_flight)
                .map(|i| {
                    let framebuffer_attachments_vec: Vec<vk::ImageView> = textures[i].iter().map(|texture| { texture.image_view }).collect();
                    let framebuffer_attachments = framebuffer_attachments_vec.as_slice();

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
                    println!("Created framebuffer[{}]: 0x{:x}", i, fb.as_raw());
                    fb
                })
                .collect();
            framebuffers
        } else {
            clear_values = vec![
                ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 0.0],
                    },
                },
                ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
                ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 0.0],
                    },
                },
            ];
            let present_framebuffers: Vec<vk::Framebuffer> = base
                .present_image_views
                .iter()
                .map(|&present_image_view| {
                    let framebuffer_attachments = [base.color_texture.image_view, base.depth_texture.image_view, present_image_view];
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
                    println!("Created present framebuffer: 0x{:x}", fb.as_raw());
                    fb
                })
                .collect();

            present_framebuffers
        };

        Self {
            renderpass,
            textures,
            framebuffers,
            clear_values
        }
    } }

    pub unsafe fn transition_to_readable(&self, base: &VkBase, command_buffer: vk::CommandBuffer, frame: usize) { unsafe {
        for tex in &self.textures[frame] {
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
                    layer_count: 1,
                },
                ..Default::default()
            };

            base.device.cmd_pipeline_barrier(
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

    pub unsafe fn destroy(&self, base: &VkBase) { unsafe {
        for framebuffer in &self.framebuffers {
            base.device.destroy_framebuffer(*framebuffer, None);
        }
        for frame_textures in &self.textures { for texture in frame_textures {
            texture.destroy(base);
        }}
        base.device.destroy_render_pass(self.renderpass, None);
    }}
}
pub struct PassCreateInfo<'a> {
    pub base: &'a VkBase,
    pub frames_in_flight: usize,
    pub color_attachment_create_infos: Vec<TextureCreateInfo<'a>>,
    pub depth_attachment_create_info: TextureCreateInfo<'a>,
    pub is_present_pass: bool,
}
impl<'a> PassCreateInfo<'a> {
    pub fn new(base: &'a VkBase) -> PassCreateInfo<'a> {
        PassCreateInfo {
            base,
            frames_in_flight: 1,
            color_attachment_create_infos: Vec::new(),
            depth_attachment_create_info: TextureCreateInfo::new(base),
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
        self.depth_attachment_create_info = depth_attachment_create_info;
        self
    }
    pub fn set_is_present_pass(mut self, is_present_pass: bool) -> Self {
        self.is_present_pass = is_present_pass;
        self
    }
}

pub struct Texture {
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub device_memory: vk::DeviceMemory,

    pub clear_value: ClearValue,
    pub format: Format,
    pub resolution: Extent3D,
    pub samples: SampleCountFlags,
    pub is_depth: bool,
}
impl Texture {
    pub unsafe fn new(create_info: &TextureCreateInfo) -> Self { unsafe {
        let color_image_create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            image_type: vk::ImageType::TYPE_2D,
            extent: Extent3D { width: create_info.width, height: create_info.height, depth: 1 },
            mip_levels: 1,
            array_layers: 1,
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
        let image_memory = create_info.device.allocate_memory(&image_alloc_info, None).expect("Failed to allocate image memory");
        create_info.device.bind_image_memory(image, image_memory, 0).expect("Failed to bind image memory");
        let image_view_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: create_info.format,
            subresource_range: ImageSubresourceRange {
                aspect_mask: if create_info.is_depth { ImageAspectFlags::DEPTH } else { ImageAspectFlags::COLOR },
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        Self {
            image,
            image_view: create_info.device.create_image_view(&image_view_info, None).expect("failed to create image view"),
            device_memory: image_memory,

            clear_value: Self::clear_value_for_format(create_info.format, create_info.clear_value),
            format: create_info.format,
            resolution: Extent3D::default().width(create_info.width).height(create_info.height).depth(create_info.depth),
            samples: create_info.samples,
            is_depth: create_info.is_depth,
        }
    } }
    pub unsafe fn destroy(&self, base: &VkBase) { unsafe {
        base.device.destroy_image(self.image, None);
        base.device.destroy_image_view(self.image_view, None);
        base.device.free_memory(self.device_memory, None);
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
    pub fn clear_value(mut self, clear_value: [f32; 4]) -> Self {
        self.clear_value = clear_value;
        self
    }
}