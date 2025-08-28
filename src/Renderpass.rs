use ash::{vk, Device, Instance};
use ash::vk::{Extent3D, ImageAspectFlags, ImageSubresourceRange, ImageUsageFlags, MemoryPropertyFlags, PhysicalDevice, SampleCountFlags};
use crate::vk_helper::{find_memorytype_index, VkBase};

pub struct Renderpass {

}

impl Renderpass {

}

pub struct Texture {
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub device_memory: vk::DeviceMemory,
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
            usage: if create_info.depth { ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT } else { ImageUsageFlags::COLOR_ATTACHMENT } | ImageUsageFlags::SAMPLED,
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
                aspect_mask: if create_info.depth { ImageAspectFlags::DEPTH } else { ImageAspectFlags::COLOR },
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
        }
    } }
    pub unsafe fn destroy(&self, base: &VkBase) {
        unsafe {
            base.device.destroy_image(self.image, None);
            base.device.destroy_image_view(self.image_view, None);
            base.device.free_memory(self.device_memory, None);
        }
    }
}

pub struct TextureCreateInfo<'a> {
    pub device: &'a Device,
    pub p_device: &'a PhysicalDevice,
    pub instance: &'a Instance,
    pub width: u32,
    pub height: u32,
    pub samples: SampleCountFlags,
    pub format: vk::Format,
    pub depth: bool,
    pub usage_flags: ImageUsageFlags,
}

impl TextureCreateInfo<'_> {
    pub fn new(base: &VkBase) -> TextureCreateInfo {
        TextureCreateInfo {
            device: &base.device,
            p_device: &base.pdevice,
            instance: &base.instance,
            width: base.surface_resolution.width,
            height: base.surface_resolution.height,
            samples: SampleCountFlags::TYPE_1,
            format: vk::Format::R16G16B16A16_SFLOAT,
            depth: false,
            usage_flags: ImageUsageFlags::SAMPLED,
        }
    }
    pub fn new_without_base<'a>(device: &'a Device, p_device: &'a PhysicalDevice, instance: &'a Instance, surface_resolution: &vk::Extent2D) -> TextureCreateInfo<'a> {
        TextureCreateInfo {
            device,
            p_device,
            instance,
            width: surface_resolution.width,
            height: surface_resolution.height,
            samples: vk::SampleCountFlags::TYPE_1,
            format: vk::Format::R16G16B16A16_SFLOAT,
            depth: false,
            usage_flags: ImageUsageFlags::SAMPLED,
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
    pub fn samples(mut self, samples: SampleCountFlags) -> Self {
        self.samples = samples;
        self
    }
    pub fn format(mut self, format: vk::Format) -> Self {
        self.format = format;
        self
    }
    pub fn depth(mut self, depth: bool) -> Self {
        self.depth = depth;
        self
    }
    pub fn usage_flags(mut self, usage_flags: ImageUsageFlags) -> Self {
        self.usage_flags = usage_flags;
        self
    }
}