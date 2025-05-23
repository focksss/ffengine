#![warn(
    clippy::use_self,
    deprecated_in_future,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unused_qualifications,
    dead_code
)]

use std::{borrow::Cow, cell::RefCell, default::Default, error::Error, ffi, fs, io, ops::Drop, os::raw::c_char};
use std::ffi::c_void;
use std::fs::Metadata;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::null_mut;
use ash::{
    ext::debug_utils,
    khr::{surface, swapchain},
    vk, Device, Entry, Instance,
};
use ash::util::{read_spv, Align};
use ash::vk::{Buffer, CommandBuffer, DeviceMemory, Extent3D, Image, ImageAspectFlags, ImageSubresourceLayers, ImageSubresourceRange, ImageUsageFlags, ImageView, MemoryPropertyFlags, Offset3D, PipelineShaderStageCreateInfo, Sampler, ShaderModule, SurfaceFormatKHR, SwapchainKHR};
use winit::{
    event_loop::EventLoop,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::WindowBuilder,
};

// Simple offset_of macro akin to C++ offsetof
#[macro_export]
macro_rules! offset_of {
    ($base:path, $field:ident) => {{
        #[allow(unused_unsafe)]
        unsafe {
            let b: $base = mem::zeroed();
            std::ptr::addr_of!(b.$field) as isize - std::ptr::addr_of!(b) as isize
        }
    }};
}
/// Helper function for submitting command buffers. Immediately waits for the fence before the command buffer
/// is executed. That way we can delay the waiting for the fences by 1 frame which is good for performance.
/// Make sure to create the fence in a signaled state on the first use.
#[allow(clippy::too_many_arguments)]
pub fn record_submit_commandbuffer<F: FnOnce(&Device, CommandBuffer)>(
    device: &Device,
    command_buffer: CommandBuffer,
    command_buffer_reuse_fence: vk::Fence,
    submit_queue: vk::Queue,
    wait_mask: &[vk::PipelineStageFlags],
    wait_semaphores: &[vk::Semaphore],
    signal_semaphores: &[vk::Semaphore],
    f: F,
) {
    unsafe {
        device
            .reset_command_buffer(
                command_buffer,
                vk::CommandBufferResetFlags::RELEASE_RESOURCES,
            )
            .expect("Reset command buffer failed.");

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        device
            .begin_command_buffer(command_buffer, &command_buffer_begin_info)
            .expect("Begin commandbuffer");
        f(device, command_buffer);
        device
            .end_command_buffer(command_buffer)
            .expect("End commandbuffer");

        let command_buffers = vec![command_buffer];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(wait_mask)
            .command_buffers(&command_buffers)
            .signal_semaphores(signal_semaphores);

        //println!("2, {:?}",device.get_fence_status(command_buffer_reuse_fence));

        if device.get_fence_status(command_buffer_reuse_fence).unwrap() {
            println!("FENCE SIGNALED PRIOR TO QUEUE SUBMIT")
        }
        device
            .queue_submit(submit_queue, &[submit_info], command_buffer_reuse_fence)
            .expect("queue submit failed.");
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 { unsafe {
    if !message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::INFO) {
        let callback_data = *p_callback_data;
        let message_id_number = callback_data.message_id_number;

        let message_id_name = if callback_data.p_message_id_name.is_null() {
            Cow::from("")
        } else {
            ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
        };

        let message = if callback_data.p_message.is_null() {
            Cow::from("")
        } else {
            ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy()
        };

        println!(
            "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
        );
    }
    vk::FALSE
} }

pub fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1u32 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags & flags == flags
        })
        .map(|(index, _memory_type)| index as _)
}

pub struct VkBase {
    pub entry: Entry,
    pub instance: Instance,
    pub device: Device,
    pub surface_loader: surface::Instance,
    pub swapchain_loader: swapchain::Device,
    pub debug_utils_loader: debug_utils::Instance,
    pub window: winit::window::Window,
    pub event_loop: RefCell<EventLoop<()>>,
    pub debug_call_back: vk::DebugUtilsMessengerEXT,

    pub pdevice: vk::PhysicalDevice,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub queue_family_index: u32,
    pub present_queue: vk::Queue,
    pub graphics_queue: vk::Queue,
    pub pdevice_properties: vk::PhysicalDeviceProperties,
    pub msaa_samples: vk::SampleCountFlags,

    pub surface: vk::SurfaceKHR,
    pub surface_format: SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,

    pub swapchain: SwapchainKHR,
    pub present_images: Vec<Image>,
    pub present_image_views: Vec<ImageView>,

    pub pool: vk::CommandPool,
    pub draw_command_buffers: Vec<CommandBuffer>,
    pub setup_command_buffer: CommandBuffer,

    pub depth_image: Image,
    pub depth_image_view: ImageView,
    pub depth_image_memory: DeviceMemory,

    pub color_image: Image,
    pub color_image_view: ImageView,
    pub color_image_memory: DeviceMemory,

    pub present_complete_semaphores: Vec<vk::Semaphore>,
    pub rendering_complete_semaphores: Vec<vk::Semaphore>,

    pub draw_commands_reuse_fences: Vec<vk::Fence>,
    pub setup_commands_reuse_fence: vk::Fence,
}
impl VkBase {
    pub fn new(window_width: u32, window_height: u32, max_frames_in_flight: usize) -> Result<Self, Box<dyn Error>> {
        unsafe {
            let event_loop = EventLoop::new()?;
            let window = WindowBuilder::new()
                .with_title("AHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH")
                .with_inner_size(winit::dpi::LogicalSize::new(
                    f64::from(window_width),
                    f64::from(window_height),
                ))
                .build(&event_loop)
                .unwrap();
            let entry = Entry::linked();
            let app_name = c"ffengine";

            let layer_names = [c"VK_LAYER_KHRONOS_validation"];
            let layers_names_raw: Vec<*const c_char> = layer_names
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect();

            let mut extension_names =
                ash_window::enumerate_required_extensions(window.display_handle()?.as_raw())
                    .unwrap()
                    .to_vec();
            extension_names.push(debug_utils::NAME.as_ptr());

            #[cfg(any(target_os = "macos", target_os = "ios"))]
            {
                extension_names.push(ash::khr::portability_enumeration::NAME.as_ptr());
                // Enabling this extension is a requirement when using `VK_KHR_portability_subset`
                extension_names.push(ash::khr::get_physical_device_properties2::NAME.as_ptr());
            }

            let appinfo = vk::ApplicationInfo::default()
                .application_name(app_name)
                .application_version(0)
                .engine_name(app_name)
                .engine_version(0)
                .api_version(vk::make_api_version(0, 1, 3, 0));

            let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
                vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
            } else {
                vk::InstanceCreateFlags::default()
            };

            let create_info = vk::InstanceCreateInfo::default()
                .application_info(&appinfo)
                .enabled_layer_names(&layers_names_raw)
                .enabled_extension_names(&extension_names)
                .flags(create_flags);

            let instance: Instance = entry
                .create_instance(&create_info, None)
                .expect("Instance creation error");

            //<editor-fold desc = "debug setup">
            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vulkan_debug_callback));

            let debug_utils_loader = debug_utils::Instance::new(&entry, &instance);
            let debug_call_back = debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap();
            //</editor-fold>

            //<editor-fold desc = "surface and physical device creation and locating">
            let surface = ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )
                .unwrap();
            let pdevices = instance
                .enumerate_physical_devices()
                .expect("Physical device error");
            let surface_loader = surface::Instance::new(&entry, &instance);
            let (pdevice, queue_family_index) = pdevices
                .iter()
                .find_map(|pdevice| {
                    instance
                        .get_physical_device_queue_family_properties(*pdevice)
                        .iter()
                        .enumerate()
                        .find_map(|(index, info)| {
                            let supports_graphic_and_surface =
                                info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                    && surface_loader
                                    .get_physical_device_surface_support(
                                        *pdevice,
                                        index as u32,
                                        surface,
                                    )
                                    .unwrap();
                            if supports_graphic_and_surface {
                                Some((*pdevice, index))
                            } else {
                                None
                            }
                        })
                })
                .expect("Couldn't find suitable device.");
            let pdevice_properties = instance.get_physical_device_properties(pdevice);
            let counts = pdevice_properties.limits.framebuffer_color_sample_counts & pdevice_properties.limits.framebuffer_depth_sample_counts;
            let mut msaa_samples = vk::SampleCountFlags::TYPE_1;
            if counts.contains(vk::SampleCountFlags::TYPE_2) {
                msaa_samples = vk::SampleCountFlags::TYPE_2;
                println!("MSAA updated to {:?}", msaa_samples)
            }
            if counts.contains(vk::SampleCountFlags::TYPE_4) {
                msaa_samples = vk::SampleCountFlags::TYPE_4;
                println!("MSAA updated to {:?}", msaa_samples)
            }
            if counts.contains(vk::SampleCountFlags::TYPE_8) {
                msaa_samples = vk::SampleCountFlags::TYPE_8;
                println!("MSAA updated to {:?}", msaa_samples)
            }
            if counts.contains(vk::SampleCountFlags::TYPE_16) {
                msaa_samples = vk::SampleCountFlags::TYPE_16;
                println!("MSAA updated to {:?}", msaa_samples)
            }
            if counts.contains(vk::SampleCountFlags::TYPE_32) {
                msaa_samples = vk::SampleCountFlags::TYPE_32;
                println!("MSAA updated to {:?}", msaa_samples)
            }
            if counts.contains(vk::SampleCountFlags::TYPE_64) {
                msaa_samples = vk::SampleCountFlags::TYPE_64;
                println!("MSAA updated to {:?}", msaa_samples)
            }
            //</editor-fold>

            let queue_family_index = queue_family_index as u32;
            let device_extension_names_raw = [
                swapchain::NAME.as_ptr(),
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                ash::khr::portability_subset::NAME.as_ptr(),
            ];
            let features = vk::PhysicalDeviceFeatures {
                shader_clip_distance: 1,
                sampler_anisotropy: vk::TRUE,
                shader_sampled_image_array_dynamic_indexing: vk::TRUE,
                shader_storage_image_array_dynamic_indexing: vk::TRUE,
                ..Default::default()
            };
            let priorities = [1.0];

            let queue_info = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index)
                .queue_priorities(&priorities);

            //<editor-fold desc = "device creation">
            let mut descriptor_indexing_features = vk::PhysicalDeviceDescriptorIndexingFeatures::default();
            descriptor_indexing_features.runtime_descriptor_array = vk::TRUE;
            descriptor_indexing_features.descriptor_binding_partially_bound = vk::TRUE;
            descriptor_indexing_features.descriptor_binding_variable_descriptor_count = vk::TRUE;
            descriptor_indexing_features.shader_sampled_image_array_non_uniform_indexing = vk::TRUE;
            descriptor_indexing_features.descriptor_binding_sampled_image_update_after_bind = vk::TRUE;
            let mut supported_features2 = vk::PhysicalDeviceFeatures2 {
                p_next: &mut descriptor_indexing_features as *mut _ as *mut c_void,
                ..Default::default()
            };
            instance.get_physical_device_features2(pdevice, &mut supported_features2);

            let device_create_info = vk::DeviceCreateInfo::default()
                .push_next(&mut descriptor_indexing_features)
                .queue_create_infos(std::slice::from_ref(&queue_info))
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features);

            let device: Device = instance
                .create_device(pdevice, &device_create_info, None)
                .unwrap();
            //</editor-fold>

            let present_queue = device.get_device_queue(queue_family_index, 0);
            let graphics_queue = device.get_device_queue(queue_family_index, 0);
            let swapchain_loader = swapchain::Device::new(&instance, &device);
            let device_memory_properties = instance.get_physical_device_memory_properties(pdevice);
            //<editor-fold desc = "command pool/buffers">
            let pool_create_info = vk::CommandPoolCreateInfo::default()
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(queue_family_index);

            let pool = device.create_command_pool(&pool_create_info, None).unwrap();

            let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
                .command_buffer_count(1 + max_frames_in_flight as u32)
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY);

            let command_buffers = device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .unwrap();
            let setup_command_buffer = command_buffers[0];
            let mut draw_command_buffers = Vec::new();
            for i in 0..max_frames_in_flight {
                draw_command_buffers.push(
                    command_buffers[i]);
            }
            //</editor-fold>
            //<editor-fold desc = "fencing">
            let fence_create_info =
                vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

            let mut draw_commands_reuse_fences = Vec::new();
            for _ in 0..max_frames_in_flight {
                draw_commands_reuse_fences.push(
                    device
                        .create_fence(&fence_create_info, None)
                        .expect("Create fence failed.")
                )
            }
            let setup_commands_reuse_fence = device
                .create_fence(&fence_create_info, None)
                .expect("Create fence failed.");
            device.reset_fences(&[setup_commands_reuse_fence]).unwrap();
            //</editor-fold>

            //<editor-fold desc = "swapchain"
            let (surface_format, surface_resolution, swapchain) =
                VkBase::create_swapchain(
                    &surface_loader,
                    &pdevice,
                    &surface,
                    &window,
                    &instance,
                    &device
                );
            //</editor-fold>
            //<editor-fold desc = "present images">
            let present_images_create_info = VkBase::create_present_images(
                &swapchain,
                &surface_format,
                &device,
                &instance,
            );
            let present_images = present_images_create_info.0;
            let present_image_views = present_images_create_info.1;
            //</editor-fold>
            //<editor-fold desc = "depth">
            let depth_image_create_info = VkBase::create_depth_image(
                &instance,
                &pdevice,
                &surface_resolution,
                &device,
                msaa_samples
            );
            let depth_image = depth_image_create_info.0;
            let depth_image_view = depth_image_create_info.1;
            let depth_image_memory = depth_image_create_info.2;
            //</editor-fold>
            //<editor-fold desc = "color">
            let color_image_create_info = VkBase::create_color_image(
                &instance,
                &pdevice,
                &surface_resolution,
                &device,
                msaa_samples,
                surface_format.format,
            );
            let color_image = color_image_create_info.0;
            let color_image_view = color_image_create_info.1;
            let color_image_memory = color_image_create_info.2;
            //</editor-fold>

            record_submit_commandbuffer(
                &device,
                setup_command_buffer,
                setup_commands_reuse_fence,
                present_queue,
                &[],
                &[],
                &[],
                |device, setup_command_buffer| {
                    let layout_transition_barriers = vk::ImageMemoryBarrier::default()
                        .image(depth_image)
                        .dst_access_mask(
                            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        )
                        .new_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .subresource_range(
                            ImageSubresourceRange::default()
                                .aspect_mask(ImageAspectFlags::DEPTH)
                                .layer_count(1)
                                .level_count(1),
                        );

                    device.cmd_pipeline_barrier(
                        setup_command_buffer,
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                        vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[layout_transition_barriers],
                    );
                },
            );
            //<editor-fold desc = "semaphores">
            let semaphore_create_info = vk::SemaphoreCreateInfo::default();

            let present_complete_semaphores = (0..max_frames_in_flight as u32)
                .map(|_| device.create_semaphore(&semaphore_create_info, None).unwrap())
                .collect::<Vec<_>>();

            let mut rendering_complete_semaphores = Vec::new();
            for _ in 0..max_frames_in_flight {
                rendering_complete_semaphores.push(
                    device
                        .create_semaphore(&semaphore_create_info, None)
                        .unwrap()
                )
            }
            //</editor-fold>
            Ok(Self {
                event_loop: RefCell::new(event_loop),
                entry,
                instance,
                device,
                queue_family_index,
                pdevice,
                device_memory_properties,
                window,
                surface_loader,
                surface_format,
                present_queue,
                graphics_queue,
                pdevice_properties,
                msaa_samples,
                surface_resolution,
                swapchain_loader,
                swapchain,
                present_images,
                present_image_views,
                pool,
                draw_command_buffers,
                setup_command_buffer,
                depth_image,
                depth_image_view,
                color_image,
                color_image_view,
                present_complete_semaphores,
                rendering_complete_semaphores,
                draw_commands_reuse_fences,
                setup_commands_reuse_fence,
                surface,
                debug_call_back,
                debug_utils_loader,
                depth_image_memory,
                color_image_memory,
            })
        }
    }
    pub unsafe fn resize_swapchain(&mut self)  {
        //<editor-fold desc = "swapchain"
        let (surface_format, surface_resolution, swapchain) =
            VkBase::create_swapchain(
                &self.surface_loader,
                &self.pdevice,
                &self.surface,
                &self.window,
                &self.instance,
                &self.device
            );
        //</editor-fold>
        //<editor-fold desc = "present images">
        let present_images_create_info = VkBase::create_present_images(
            &swapchain,
            &surface_format,
            &self.device,
            &self.instance,
        );
        self.present_images = present_images_create_info.0;
        self.present_image_views = present_images_create_info.1;
        //</editor-fold>
        //<editor-fold desc = "depth">
        let depth_image_create_info = VkBase::create_depth_image(
            &self.instance,
            &self.pdevice,
            &surface_resolution,
            &self.device,
            self.msaa_samples
        );
        self.depth_image = depth_image_create_info.0;
        self.depth_image_view = depth_image_create_info.1;
        self.depth_image_memory = depth_image_create_info.2;
        //</editor-fold>
        //<editor-fold desc = "color">
        let color_image_create_info = VkBase::create_color_image(
            &self.instance,
            &self.pdevice,
            &surface_resolution,
            &self.device,
            self.msaa_samples,
            surface_format.format,
        );
        self.color_image = color_image_create_info.0;
        self.color_image_view = color_image_create_info.1;
        self.color_image_memory = color_image_create_info.2;
        //</editor-fold> {
    }
    pub fn create_swapchain(
        surface_loader: &surface::Instance,
        pdevice: &vk::PhysicalDevice,
        surface: &vk::SurfaceKHR,
        window: &winit::window::Window,
        instance: &Instance,
        device: &Device,
    ) -> (SurfaceFormatKHR, vk::Extent2D, SwapchainKHR) { unsafe {
        let surface_format = surface_loader
            .get_physical_device_surface_formats(*pdevice, *surface)
            .unwrap()[0];

        let surface_capabilities = surface_loader
            .get_physical_device_surface_capabilities(*pdevice, *surface)
            .unwrap();
        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && desired_image_count > surface_capabilities.max_image_count
        {
            desired_image_count = surface_capabilities.max_image_count;
        }
        let surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: window.inner_size().width,
                height: window.inner_size().height,
            },
            _ => surface_capabilities.current_extent,
        };
        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };
        let present_modes = surface_loader
            .get_physical_device_surface_present_modes(*pdevice, *surface)
            .unwrap();
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);
        let swapchain_loader = swapchain::Device::new(instance, device);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(*surface)
            .min_image_count(desired_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_resolution)
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

        let swapchain = swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .unwrap();

        (surface_format, surface_resolution, swapchain)
    }}
    pub fn create_present_images(
        swapchain: &SwapchainKHR,
        surface_format: &SurfaceFormatKHR,
        device: &Device,
        instance: &Instance,
    ) -> (Vec<Image>, Vec<ImageView>) { unsafe {
        let swapchain_loader = swapchain::Device::new(&instance, &device);
        let present_images = swapchain_loader.get_swapchain_images(*swapchain).unwrap();
        let present_image_views: Vec<ImageView> = present_images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo::default()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(ImageSubresourceRange {
                        aspect_mask: ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);
                device.create_image_view(&create_view_info, None).unwrap()
            })
            .collect();
        (present_images, present_image_views)
    }}
    pub fn create_depth_image(
        instance: &Instance,
        pdevice: &vk::PhysicalDevice,
        surface_resolution: &vk::Extent2D,
        device: &Device,
        samples: vk::SampleCountFlags,
    ) -> (Image, ImageView, DeviceMemory) { unsafe {
        let device_memory_properties = instance.get_physical_device_memory_properties(*pdevice);
        let depth_image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::D16_UNORM)
            .extent((*surface_resolution).into())
            .mip_levels(1)
            .array_layers(1)
            .samples(samples)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let depth_image = device.create_image(&depth_image_create_info, None).unwrap();
        let depth_image_memory_req = device.get_image_memory_requirements(depth_image);
        let depth_image_memory_index = find_memorytype_index(
            &depth_image_memory_req,
            &device_memory_properties,
            MemoryPropertyFlags::DEVICE_LOCAL,
        )
            .expect("Unable to find suitable memory index for depth image.");

        let depth_image_allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(depth_image_memory_req.size)
            .memory_type_index(depth_image_memory_index);

        let depth_image_memory = device
            .allocate_memory(&depth_image_allocate_info, None)
            .unwrap();

        device
            .bind_image_memory(depth_image, depth_image_memory, 0)
            .expect("Unable to bind depth image memory");

        let depth_image_view_info = vk::ImageViewCreateInfo::default()
            .subresource_range(
                ImageSubresourceRange::default()
                    .aspect_mask(ImageAspectFlags::DEPTH)
                    .level_count(1)
                    .layer_count(1),
            )
            .image(depth_image)
            .format(depth_image_create_info.format)
            .view_type(vk::ImageViewType::TYPE_2D);

        let depth_image_view = device
            .create_image_view(&depth_image_view_info, None)
            .unwrap();
        (depth_image, depth_image_view, depth_image_memory)
    }}
    pub fn create_color_image(
        instance: &Instance,
        pdevice: &vk::PhysicalDevice,
        surface_resolution: &vk::Extent2D,
        device: &Device,
        samples: vk::SampleCountFlags,
        format: vk::Format,
    ) -> (Image, ImageView, DeviceMemory) { unsafe {
        let color_image_create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            image_type: vk::ImageType::TYPE_2D,
            extent: Extent3D { width: surface_resolution.width, height: surface_resolution.height, depth: 1 },
            mip_levels: 1,
            array_layers: 1,
            format,
            tiling: vk::ImageTiling::OPTIMAL,
            initial_layout: vk::ImageLayout::UNDEFINED,
            usage: ImageUsageFlags::TRANSIENT_ATTACHMENT | ImageUsageFlags::COLOR_ATTACHMENT,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            samples,
            ..Default::default()
        };
        let color_image = device.create_image(&color_image_create_info, None).expect("Failed to create image");
        let color_image_memory_req = device.get_image_memory_requirements(color_image);
        let color_image_alloc_info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
            allocation_size: color_image_memory_req.size,
            memory_type_index: find_memorytype_index(
                &color_image_memory_req,
                &instance.get_physical_device_memory_properties(*pdevice),
                MemoryPropertyFlags::DEVICE_LOCAL,
            ).expect("unable to get mem type index for texture image"),
            ..Default::default()
        };
        let color_image_memory = device.allocate_memory(&color_image_alloc_info, None).expect("Failed to allocate image memory");
        device.bind_image_memory(color_image, color_image_memory, 0).expect("Failed to bind image memory");
        let color_image_view_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            image: color_image,
            view_type: vk::ImageViewType::TYPE_2D,
            format,
            subresource_range: ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        (color_image, device.create_image_view(&color_image_view_info, None).expect("failed to create image view"), color_image_memory)
    }}



    pub unsafe fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: MemoryPropertyFlags,
        buffer: &mut Buffer,
        buffer_memory: &mut DeviceMemory)
    { unsafe {
        let buffer_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        *buffer = self.device.create_buffer(&buffer_info, None).expect("failed to create buffer");

        let memory_requirements = self.device.get_buffer_memory_requirements(*buffer);
        let memory_indices = find_memorytype_index(
            &memory_requirements,
            &self.device_memory_properties,
            properties,
        ).expect("failed to find suitable memory type for buffer");
        let allocation_info = vk::MemoryAllocateInfo {
            allocation_size: memory_requirements.size,
            memory_type_index: memory_indices,
            ..Default::default()
        };

        *buffer_memory = self.device.allocate_memory(&allocation_info, None).expect("failed to allocate buffer memory");

        self.device
            .bind_buffer_memory(*buffer, *buffer_memory, 0)
            .expect("failed to bind buffer memory");
    }
    }
    pub unsafe fn copy_buffer(&self, src_buffer: &Buffer, dst_buffer: &Buffer, size: &vk::DeviceSize) { unsafe {
        let command_buffers = self.begin_single_time_commands(1);
        let copy_region = [vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size: *size,
            ..Default::default()
        }];
        self.device.cmd_copy_buffer(command_buffers[0], *src_buffer, *dst_buffer, &copy_region);
        self.end_single_time_commands(command_buffers);
    } }
    pub unsafe fn create_device_and_staging_buffer<T: Copy>(&self, buffer_size_in: u64, data: &[T], usage: vk::BufferUsageFlags, destroy_staging: bool, keep_ptr: bool, do_initial_copy: bool) -> ((Buffer, DeviceMemory), (Buffer, DeviceMemory, *mut c_void)) { unsafe {
        let buffer_size;
        if buffer_size_in > 0 {
            buffer_size = buffer_size_in;
        } else {
            buffer_size = (size_of::<T>() * data.len()) as u64;
        }
        let mut staging_buffer = Buffer::null();
        let mut staging_buffer_memory = DeviceMemory::null();
        self.create_buffer(
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            &mut staging_buffer,
            &mut staging_buffer_memory,
        );
        let mut ptr = null_mut();
        if do_initial_copy || keep_ptr {
            ptr = self
                .device
                .map_memory(
                    staging_buffer_memory,
                    0,
                    buffer_size,
                    vk::MemoryMapFlags::empty(),
                )
                .expect("Failed to map index buffer memory");
            if do_initial_copy {
                copy_data_to_memory(ptr, &data);
            }
        }
        let mut buffer = Buffer::null();
        let mut buffer_memory = DeviceMemory::null();
        self.create_buffer(
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | usage,
            MemoryPropertyFlags::DEVICE_LOCAL,
            &mut buffer,
            &mut buffer_memory,
        );
        if do_initial_copy {
            self.copy_buffer(&staging_buffer, &buffer, &buffer_size);
        }
        if destroy_staging {
            self.device.destroy_buffer(staging_buffer, None);
            self.device.free_memory(staging_buffer_memory, None);
            ((buffer, buffer_memory), (Buffer::null(), DeviceMemory::null(), null_mut()))
        } else {
            if !keep_ptr {
                self.device.unmap_memory(staging_buffer_memory);
                ptr = null_mut();
            }
            ((buffer, buffer_memory), (staging_buffer, staging_buffer_memory, ptr))
        }
    } }
    pub unsafe fn create_2d_texture_image(&self, uri: &PathBuf, generate_mipmaps: bool) -> ((ImageView, Sampler), (Image, DeviceMemory), u32) { unsafe {
        let bytes = fs::read(uri).expect(uri.to_string_lossy().as_ref());
        let image = image::load_from_memory(&bytes).expect("Failed to load image").to_rgba8();
        let (img_width, img_height) = image.dimensions();
        let image_extent = vk::Extent2D { width: img_width, height: img_height };
        let image_data = image.into_raw();
        let image_size = (img_width * img_height * 4) as u64;
        let mut image_mip_levels = 1 + image_extent.height.max(image_extent.width).ilog2();
        let usage: ImageUsageFlags;
        if generate_mipmaps {
            usage = ImageUsageFlags::TRANSFER_SRC | ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::SAMPLED;
        } else {
            image_mip_levels = 1;
            usage = ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::SAMPLED;
        }

        let mut image_staging_buffer = Buffer::null();
        let mut image_staging_buffer_memory = DeviceMemory::null();
        VkBase::create_buffer(
            self,
            image_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            &mut image_staging_buffer,
            &mut image_staging_buffer_memory,
        );
        let image_ptr = self
            .device
            .map_memory(
                image_staging_buffer_memory,
                0,
                image_size,
                vk::MemoryMapFlags::empty(),
            )
            .expect("Failed to map image buffer memory");
        copy_data_to_memory(image_ptr, &image_data);
        self.device.unmap_memory(image_staging_buffer_memory);

        let texture_image_create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            image_type: vk::ImageType::TYPE_2D,
            extent: Extent3D { width: image_extent.width, height: image_extent.height, depth: 1 },
            mip_levels: image_mip_levels,
            array_layers: 1,
            format: vk::Format::R8G8B8A8_SRGB,
            tiling: vk::ImageTiling::OPTIMAL,
            initial_layout: vk::ImageLayout::UNDEFINED,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        let mut texture_image = Image::null();
        let mut texture_image_memory = DeviceMemory::null();
        self.create_image(
            &texture_image_create_info,
            MemoryPropertyFlags::DEVICE_LOCAL,
            &mut texture_image,
            &mut texture_image_memory,
        );
        self.transition_image_layout(texture_image, ImageSubresourceRange {
            aspect_mask: ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: image_mip_levels,
            base_array_layer: 0,
            layer_count: 1,
            ..Default::default()
        }, vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        self.copy_buffer_to_image(image_staging_buffer, texture_image, image_extent.into());

        self.generate_mipmaps(texture_image, image_mip_levels, image_extent.into());
        /*
        self.transition_image_layout(texture_image, ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: image_mip_levels,
            base_array_layer: 0,
            layer_count: 1,
            ..Default::default()
        }, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
         */

        self.device.destroy_buffer(image_staging_buffer, None);
        self.device.free_memory(image_staging_buffer_memory, None);
        
        let view_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            image: texture_image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: vk::Format::R8G8B8A8_SRGB,
            subresource_range: ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: image_mip_levels,
                base_array_layer: 0,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        let sampler_info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            anisotropy_enable: vk::TRUE,
            max_anisotropy: self.pdevice_properties.limits.max_sampler_anisotropy,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            mip_lod_bias: 0.0,
            min_lod: 0.0,
            max_lod: 0.0,
            ..Default::default()
        };

        let image = (texture_image, texture_image_memory);
        let texture = (self.device.create_image_view(&view_info, None).expect("failed to create image view"), self.device.create_sampler(&sampler_info, None).expect("failed to create sampler"));
        (texture, image, image_mip_levels)
    } }
    pub unsafe fn generate_mipmaps(&self, image: Image, mips: u32, extent: Extent3D) { unsafe {
        let command_buffers = self.begin_single_time_commands(1);

        let mut barrier = vk::ImageMemoryBarrier {
            s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
            image,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            subresource_range: ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_array_layer: 0,
                layer_count: 1,
                level_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut width = extent.width as i32;
        let mut height = extent.height as i32;
        for i in 1..mips {
            barrier.subresource_range.base_mip_level = i - 1;
            barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;

            self.device.cmd_pipeline_barrier(
                command_buffers[0],
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            let blit = vk::ImageBlit {
                src_offsets: [Offset3D {x: 0, y: 0, z: 0}, Offset3D {x: width, y: height, z: 1}],
                src_subresource: ImageSubresourceLayers {
                    aspect_mask: ImageAspectFlags::COLOR,
                    mip_level: i - 1,
                    base_array_layer: 0,
                    layer_count: 1,
                    ..Default::default()
                },
                dst_offsets: [Offset3D {x: 0, y: 0, z: 0}, Offset3D {x: if width > 1 { width / 2 } else { 1 }, y: if height > 1 { height / 2 } else { 1 }, z: 1}],
                dst_subresource: ImageSubresourceLayers {
                    aspect_mask: ImageAspectFlags::COLOR,
                    mip_level: i,
                    base_array_layer: 0,
                    layer_count: 1,
                    ..Default::default()
                },
                ..Default::default()
            };

            self.device.cmd_blit_image(
                command_buffers[0],
                image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[blit],
                vk::Filter::LINEAR,
            );

            barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

            self.device.cmd_pipeline_barrier(
                command_buffers[0],
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            if width > 1 { width /= 2 }
            if height > 1 { height /= 2 }
        }
        barrier.subresource_range.base_mip_level = mips - 1;
        barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

        self.device.cmd_pipeline_barrier(
            command_buffers[0],
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );

        self.end_single_time_commands(command_buffers);
    } }
    pub unsafe fn create_image(
        &self,
        create_info: &vk::ImageCreateInfo<'_>,
        properties: MemoryPropertyFlags,
        image: &mut Image,
        image_memory: &mut DeviceMemory)
    { unsafe {
        *image = self.device.create_image(create_info, None).expect("Failed to create image");
        let texture_image_memory_req = self.device.get_image_memory_requirements(*image);
        let texture_image_alloc_info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
            allocation_size: texture_image_memory_req.size,
            memory_type_index: find_memorytype_index(
                &texture_image_memory_req,
                &self.instance.get_physical_device_memory_properties(self.pdevice),
                properties
            ).expect("unable to get mem type index for texture image"),
            ..Default::default()
        };
        *image_memory = self.device.allocate_memory(&texture_image_alloc_info, None).expect("Failed to allocate image memory");
        self.device.bind_image_memory(*image, *image_memory, 0).expect("Failed to bind image memory");
    } }
    pub unsafe fn begin_single_time_commands(&self, command_buffer_count: u32) -> Vec<CommandBuffer> { unsafe {
        let alloc_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            level: vk::CommandBufferLevel::PRIMARY,
            command_pool: self.pool,
            command_buffer_count,
            ..Default::default()
        };
        let command_buffers = self.device.allocate_command_buffers(&alloc_info).unwrap();
        let begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };
        for i in 0usize..command_buffer_count as usize {
            self.device.begin_command_buffer(command_buffers[i], &begin_info).unwrap();
        }
        command_buffers
    } }
    pub unsafe fn end_single_time_commands(&self, command_buffers: Vec<CommandBuffer>) { unsafe {
        for command_buffer in command_buffers.iter() {
            self.device.end_command_buffer(*command_buffer).unwrap();
        }
        let submit_info = [vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            command_buffer_count: 1,
            p_command_buffers: &command_buffers[0],
            ..Default::default()
        }];
        self.device.queue_submit(self.graphics_queue, &submit_info, vk::Fence::null()).unwrap();
        self.device.queue_wait_idle(self.graphics_queue).unwrap();
        self.device.free_command_buffers(self.pool, &command_buffers);
    } }
    pub unsafe fn transition_image_layout(&self, image: Image, subresource_range: ImageSubresourceRange, old_layout: vk::ImageLayout, new_layout: vk::ImageLayout) { unsafe {
        let command_buffers = self.begin_single_time_commands(1);
        let mut barrier = vk::ImageMemoryBarrier {
            s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
            old_layout,
            new_layout,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image,
            subresource_range,
            ..Default::default()
        };
        let mut source_stage = vk::PipelineStageFlags::empty();
        let mut destination_stage = vk::PipelineStageFlags::empty();

        if old_layout == vk::ImageLayout::UNDEFINED && new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL {
            barrier.src_access_mask = vk::AccessFlags::empty();
            barrier.dst_access_mask = vk::AccessFlags::TRANSFER_WRITE;

            source_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
            destination_stage = vk::PipelineStageFlags::TRANSFER;
        } 
        else if old_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL {
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;

            source_stage = vk::PipelineStageFlags::TRANSFER;
            destination_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
        } else {
            eprintln!("unsupported layout transition");
        }
        
        self.device.cmd_pipeline_barrier(
            command_buffers[0],
            source_stage,
            destination_stage,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );

        self.end_single_time_commands(command_buffers);
    } }
    pub unsafe fn copy_buffer_to_image(&self, buffer: Buffer, image: Image, extent: Extent3D) { unsafe {
        let command_buffers = self.begin_single_time_commands(1);
        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: ImageSubresourceLayers {
                aspect_mask: ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
                ..Default::default()
            },
            image_offset: Offset3D { x: 0, y: 0, z: 0 },
            image_extent: extent,
            ..Default::default()
        };
        self.device.cmd_copy_buffer_to_image(command_buffers[0], buffer, image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &[region]);
        self.end_single_time_commands(command_buffers);
    } }
}
impl Drop for VkBase {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            for &present_complete_semaphore in &self.present_complete_semaphores {
                self.device
                    .destroy_semaphore(present_complete_semaphore, None);
            }
            for &rendering_complete_semaphore in &self.rendering_complete_semaphores {
                self.device
                    .destroy_semaphore(rendering_complete_semaphore, None);
            }
            for &draw_command_reuse_fence in &self.draw_commands_reuse_fences {
                self.device
                    .destroy_fence(draw_command_reuse_fence, None);
            }
            self.device
                .destroy_fence(self.setup_commands_reuse_fence, None);
            self.device.free_memory(self.depth_image_memory, None);
            self.device.destroy_image_view(self.depth_image_view, None);
            self.device.destroy_image(self.depth_image, None);

            self.device.free_memory(self.color_image_memory, None);
            self.device.destroy_image_view(self.color_image_view, None);
            self.device.destroy_image(self.color_image, None);

            for &image_view in self.present_image_views.iter() {
                self.device.destroy_image_view(image_view, None);
            }
            self.device.destroy_command_pool(self.pool, None);
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
            self.instance.destroy_instance(None);
        }
    }
}

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

pub unsafe fn copy_data_to_memory<T: Copy>(ptr: *mut c_void, data: &[T]) { unsafe {
    let mut aligned = Align::new(
        ptr,
        align_of::<T>() as u64,
        (data.len() * size_of::<T>()) as u64,
    );
    aligned.copy_from_slice(&data);
} }
pub fn compile_shaders(shader_directories: Vec<&str>) -> io::Result<()> {
    for shader_directory in shader_directories {
        let shader_directory_path = Path::new(&shader_directory);

        let spv_folder_str = shader_directory.replace("shaders\\glsl", "shaders\\spv");
        let spv_folder = Path::new(&spv_folder_str);

        if !spv_folder.exists() {
            println!("Creating folder: {:?}", spv_folder);
            fs::create_dir_all(&spv_folder)?;
        }
        for shader in fs::read_dir(shader_directory_path)? {
            let shader = shader?;
            let path = shader.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "vert" || ext == "frag" || ext == "geom" {
                        let file_name = path.file_name().unwrap().to_string_lossy();
                        let spv_file = spv_folder.join(format!("{}.spv", file_name));

                        let glsl_modified = path.metadata()?.modified()?;
                        let spv: Result<Metadata, _> = spv_file.metadata();
                        if spv.is_err() || glsl_modified > spv?.modified()? {
                            println!("RECOMPILING:\n    {}", spv_file.display());
                            let compile_cmd = Command::new("glslc")
                                .arg(&path)
                                .arg("-o")
                                .arg(&spv_file)
                                .status()?;
                            if !compile_cmd.success() {
                                println!("Shader compilation failed");
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
pub fn load_file(path: &str) -> io::Result<Vec<u8>> {
    let path_final = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src\\shaders\\spv").join(path);
    fs::read(path_final)
}