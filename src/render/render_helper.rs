use std::ffi::c_void;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use ash::{vk, Device, Instance};
use ash::util::read_spv;
use ash::vk::{Buffer, ClearColorValue, ClearDepthStencilValue, ClearValue, DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateFlags, DescriptorPoolSize, DescriptorSetLayout, DescriptorSetLayoutCreateFlags, DescriptorType, DeviceMemory, DeviceSize, Extent3D, Format, ImageAspectFlags, ImageSubresourceRange, ImageUsageFlags, MemoryPropertyFlags, PhysicalDevice, PipelineShaderStageCreateInfo, SampleCountFlags, ShaderModule, ShaderModuleCreateInfo, ShaderStageFlags};
use crate::{MAX_FRAMES_IN_FLIGHT};
use crate::render::*;

const SHADER_PATH: &str = "resources\\shaders\\spv\\";

pub struct Shader {
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

    pub unsafe fn destroy(&self, base: &VkBase) { unsafe {
        base.device.destroy_shader_module(self.vertex_module, None);
        if self.geometry_module.is_some() {
            base.device.destroy_shader_module(self.geometry_module.unwrap(), None);
        }
        base.device.destroy_shader_module(self.fragment_module, None);
    } }
}

pub struct Pass {
    pub renderpass: vk::RenderPass,
    pub textures: Vec<Vec<Texture>>,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub clear_values: Vec<ClearValue>,
}
impl Pass {
    pub unsafe fn new(create_info: PassCreateInfo) -> Self { unsafe {
        let mut textures = Vec::new();
        let base = create_info.base;
        if !create_info.is_present_pass {
            for _ in 0..create_info.frames_in_flight {
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
                        .width(create_info.depth_attachment_create_info.width)
                        .height(create_info.depth_attachment_create_info.height)
                        .layers(create_info.depth_attachment_create_info.array_layers);

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
                ClearValue {
                    depth_stencil: ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
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
                    //println!("Created present framebuffer: 0x{:x}", fb.as_raw());
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
                    layer_count: tex.array_layers,
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

#[derive(Debug)]
pub struct Descriptor {
    pub descriptor_type: DescriptorType,
    pub shader_stages: ShaderStageFlags,
    pub is_dynamic: bool,
    pub offset: Option<DeviceSize>,
    pub range: Option<DeviceSize>,
    pub owned_buffers: (Vec<Buffer>, Vec<DeviceMemory>, Vec<*mut c_void>),
    pub buffer_refs: Vec<Buffer>,
    pub image_infos: Option<*const DescriptorImageInfo>,
    pub descriptor_count: u32,
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
                    descriptor_type: DescriptorType::UNIFORM_BUFFER,
                    shader_stages: create_info.shader_stages,
                    is_dynamic: create_info.dynamic,
                    offset: Some(create_info.offset),
                    range: Some(create_info.range),
                    descriptor_count: 1,
                    owned_buffers: (uniform_buffers, uniform_buffers_memory, uniform_buffers_mapped),
                    buffer_refs: Vec::new(),
                    image_infos: None
                }
            }
            DescriptorType::STORAGE_BUFFER => {
                Descriptor {
                    descriptor_type: DescriptorType::STORAGE_BUFFER,
                    shader_stages: create_info.shader_stages,
                    is_dynamic: create_info.dynamic,
                    offset: Some(create_info.offset),
                    range: Some(create_info.range),
                    descriptor_count: 1,
                    owned_buffers: (Vec::new(), Vec::new(), Vec::new()),
                    buffer_refs: create_info.buffers.clone().unwrap(),
                    image_infos: None
                }
            }
            DescriptorType::COMBINED_IMAGE_SAMPLER => {
                Descriptor {
                    descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
                    shader_stages: create_info.shader_stages,
                    is_dynamic: create_info.dynamic,
                    offset: None,
                    range: None,
                    descriptor_count: create_info.image_infos.as_ref().map_or(1, |v| v.len()) as u32,
                    owned_buffers: (Vec::new(), Vec::new(), Vec::new()),
                    buffer_refs: create_info.buffers.clone().as_ref().map_or(Vec::new(), |v| v.to_vec()),
                    image_infos: create_info.image_infos.as_ref().map_or(None, |i| Some(i.as_ptr())),
                }
            }
            _ => {
                Descriptor {
                    descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
                    shader_stages: ShaderStageFlags::FRAGMENT,
                    is_dynamic: false,
                    offset: None,
                    range: None,
                    owned_buffers: (Vec::new(), Vec::new(), Vec::new()),
                    buffer_refs: Vec::new(),
                    image_infos: None,
                    descriptor_count: 0,
                }
            }
        }
    } }

    pub unsafe fn destroy(&self, base: &VkBase) { unsafe {
         if self.descriptor_type == DescriptorType::UNIFORM_BUFFER || self.descriptor_type == DescriptorType::STORAGE_BUFFER {
             for i in 0..self.owned_buffers.0.len() {
                 base.device.destroy_buffer(self.owned_buffers.0[i], None);
                 base.device.free_memory(self.owned_buffers.1[i], None);
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
}
impl DescriptorCreateInfo<'_> {
    pub fn new(base: &VkBase) -> DescriptorCreateInfo {
        DescriptorCreateInfo {
            base,
            frames_in_flight: 1,
            descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
            size: None,
            shader_stages: ShaderStageFlags::FRAGMENT,
            offset: 0,
            range: vk::WHOLE_SIZE,
            buffers: None,
            image_infos: None,
            memory_property_flags: MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            dynamic: false,
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
}

pub struct DescriptorSet {
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub descriptor_set_layout: DescriptorSetLayout,
    pub descriptor_pool: DescriptorPool,
    pub descriptors: Vec<Descriptor>,
}
impl DescriptorSet {
    pub unsafe fn new(create_info: DescriptorSetCreateInfo) -> DescriptorSet { unsafe {
        let base = create_info.base;
        let mut has_dynamic = false;
        let mut variable_descriptor_count = 0;
        let descriptor_pool_sizes = create_info.descriptors.iter().map(|d| {
            if d.is_dynamic {has_dynamic = true; variable_descriptor_count = d.descriptor_count;}
            DescriptorPoolSize {
                ty: d.descriptor_type,
                descriptor_count: create_info.frames_in_flight as u32 * d.descriptor_count,
            }
        }).collect::<Vec<DescriptorPoolSize>>();
        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            pool_size_count: descriptor_pool_sizes.len() as u32,
            p_pool_sizes: descriptor_pool_sizes.as_ptr(),
            max_sets: MAX_FRAMES_IN_FLIGHT as u32,
            flags: if has_dynamic {DescriptorPoolCreateFlags::UPDATE_AFTER_BIND} else {DescriptorPoolCreateFlags::empty()},
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
                vk::DescriptorBindingFlags::PARTIALLY_BOUND |
                vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT |
                vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            } else {
                vk::DescriptorBindingFlags::empty()
            }
        }).collect::<Vec<vk::DescriptorBindingFlags>>();
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
            flags: if has_dynamic {DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL} else {DescriptorSetLayoutCreateFlags::empty()},
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
            descriptor_set_count: MAX_FRAMES_IN_FLIGHT as u32,
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
            descriptor_sets,
            descriptor_set_layout,
            descriptor_pool,
            descriptors: create_info.descriptors,
        }
    } }
    pub unsafe fn destroy(self, base: &VkBase) { unsafe {
        base.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        base.device.destroy_descriptor_pool(self.descriptor_pool, None);
        for descriptor in self.descriptors {
            descriptor.destroy(base);
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
            frames_in_flight: 1,
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