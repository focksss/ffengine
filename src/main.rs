#![warn(unused_qualifications)]
mod matrix;
mod vector;
mod vk_helper;
mod scene;
mod model;

use std::default::Default;
use std::error::Error;
use std::io::Cursor;
use std::{fs, io, mem};
use std::collections::HashSet;
use std::ffi::c_void;
use std::mem::{align_of, size_of};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use ash::util::*;
use ash::vk;
use ash::vk::{Buffer, BufferMemoryBarrier, CommandBuffer, DeviceMemory, Extent3D, Image, ImageSubresourceRange, ImageView};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::CursorGrabMode;
use crate::{vk_helper::*, matrix::*, vector::*};
use crate::model::{Gltf};
use crate::scene::Camera;

#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct Vertex {
    pos: [f32; 4],
    normal: [f32; 4],
    uv: [f32; 2],
}
#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct UniformData {
    view: [f32; 16],
    projection: [f32; 16],
}

const MAX_FRAMES_IN_FLIGHT: usize = 2;
const PI: f32 = std::f32::consts::PI;

fn main() -> Result<(), Box<dyn Error>> {
    let vertices = [
        Vertex {
            pos: [-1.0, 1.0, 0.0, 1.0],
            normal: [1.0, 0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
        },
        Vertex {
            pos: [1.0, 1.0, 0.0, 1.0],
            normal: [0.0, 1.0, 0.0, 1.0],
            uv: [1.0, 0.0],
        },
        Vertex {
            pos: [0.0, -1.0, 0.0, 1.0],
            normal: [0.0, 0.0, 1.0, 1.0],
            uv: [0.5, 1.0],
        },
    ];
    unsafe {
        println!("main code running");
        let mut shader_paths = Vec::new();
        shader_paths.push("src\\shaders\\glsl\\hello_triangle");

        compile_shaders(shader_paths).expect("Failed to compile shaders");

        let mut base = VkBase::new(1000, 800, MAX_FRAMES_IN_FLIGHT)?;
        run(&mut base, vertices).expect("Application launch failed!");
    }
    Ok(())
}

unsafe fn run(base: &mut VkBase, vertices: [Vertex; 3]) -> Result<(), Box<dyn Error>> {
    unsafe {
        //<editor-fold desc = "renderpass init">
        let renderpass_attachments = [
            vk::AttachmentDescription {
                format: base.surface_format.format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                ..Default::default()
            },
            vk::AttachmentDescription {
                format: vk::Format::D16_UNORM,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                initial_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
        ];
        let color_attachment_refs = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
        let depth_attachment_ref = vk::AttachmentReference {
            attachment: 1,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };
        let dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
                | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ..Default::default()
        }];

        let subpass = vk::SubpassDescription::default()
            .color_attachments(&color_attachment_refs)
            .depth_stencil_attachment(&depth_attachment_ref)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

        let renderpass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(&renderpass_attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies);

        let renderpass = base
            .device
            .create_render_pass(&renderpass_create_info, None)
            .unwrap();
        //</editor-fold>
        //<editor-fold desc = "framebuffers">
        let framebuffers: Vec<vk::Framebuffer> = base
            .present_image_views
            .iter()
            .map(|&present_image_view| {
                let framebuffer_attachments = [present_image_view, base.depth_image_view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(renderpass)
                    .attachments(&framebuffer_attachments)
                    .width(base.surface_resolution.width)
                    .height(base.surface_resolution.height)
                    .layers(1);

                base.device
                    .create_framebuffer(&frame_buffer_create_info, None)
                    .unwrap()
            })
            .collect();
        //</editor-fold>
        //<editor-fold desc = "image staging buffer">
        let image = image::load_from_memory(include_bytes!("C:\\Graphics\\assets\\luna\\textures\\T_1031001_Head_01_D.png"))
            .unwrap()
            .to_rgba8();
        let (img_width, img_height) = image.dimensions();
        let image_extent = vk::Extent2D { width: img_width, height: img_height };
        let image_data = image.into_raw();
        let image_size = (img_width * img_height * 4) as u64;

        let mut image_staging_buffer = Buffer::null();
        let mut image_staging_buffer_memory = DeviceMemory::null();
        VkBase::create_buffer(
          base,
          image_size,
          vk::BufferUsageFlags::TRANSFER_SRC,
          vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
          &mut image_staging_buffer,
          &mut image_staging_buffer_memory,
        );
        let image_ptr = base
            .device
            .map_memory(
                image_staging_buffer_memory,
                0,
                image_size,
                vk::MemoryMapFlags::empty(),
            )
            .expect("Failed to map image buffer memory");
        copy_data_to_memory(image_ptr, &image_data);
        base.device.unmap_memory(image_staging_buffer_memory);
        //</editor-fold>
        //<editor-fold desc = "texture image">
        let texture_image_create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            image_type: vk::ImageType::TYPE_2D,
            extent: Extent3D { width: image_extent.width, height: image_extent.height, depth: 1 },
            mip_levels: 1,
            array_layers: 1,
            format: vk::Format::R8G8B8A8_SRGB,
            tiling: vk::ImageTiling::OPTIMAL,
            initial_layout: vk::ImageLayout::UNDEFINED,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        let mut texture_image = Image::null();
        let mut texture_image_memory = DeviceMemory::null();
        base.create_image(
            &texture_image_create_info,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &mut texture_image,
            &mut texture_image_memory,
        );
        base.transition_image_layout(texture_image, vk::Format::R8G8B8A8_SRGB, vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        base.copy_buffer_to_image(image_staging_buffer, texture_image, image_extent.into());
        base.transition_image_layout(texture_image, vk::Format::R8G8B8A8_SRGB, vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        base.device.destroy_buffer(image_staging_buffer, None);
        base.device.free_memory(image_staging_buffer_memory, None);

        let texture_image_view = base.create_2_d_image_view(texture_image, vk::Format::R8G8B8A8_SRGB);
        //</editor-fold>
        //<editor-fold desc = "texture sampler">
        let sampler_info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            anisotropy_enable: vk::TRUE,
            max_anisotropy: base.pdevice_properties.limits.max_sampler_anisotropy,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            mip_lod_bias: 0.0,
            min_lod: 0.0,
            max_lod: 0.0,
            ..Default::default()
        };
        let texture_sampler = base.device.create_sampler(&sampler_info, None).expect("failed to create sampler");
        //</editor-fold>
        //<editor-fold desc = "uniform buffers">
        let ubo_layout_bindings = [
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            }
        ];
        let ubo_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            binding_count: ubo_layout_bindings.len() as u32,
            p_bindings: ubo_layout_bindings.as_ptr(),
            ..Default::default()
        };
        let ubo_descriptor_set_layout = base.device.create_descriptor_set_layout(&ubo_layout_create_info, None)?;
        
        let ubo_buffer_size = size_of::<UniformData>() as u64;
        let mut uniform_buffers = Vec::new();
        let mut uniform_buffers_memory = Vec::new();
        let mut uniform_buffers_mapped = Vec::new();
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            uniform_buffers.push(Buffer::null());
            uniform_buffers_memory.push(DeviceMemory::null());
            base.create_buffer(
                ubo_buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                &mut uniform_buffers[i],
                &mut uniform_buffers_memory[i],
            );
            uniform_buffers_mapped.push(base.device.map_memory(
                uniform_buffers_memory[i],
                0,
                ubo_buffer_size,
                vk::MemoryMapFlags::empty()
            ).expect("failed to map uniform buffer"));
        }
        //</editor-fold>
        //<editor-fold desc = "descriptor pool">
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
                ..Default::default()
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
                ..Default::default()
            }
        ];
        let pool_create_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
            max_sets: MAX_FRAMES_IN_FLIGHT as u32,
            ..Default::default()
        };
        let descriptor_pool = base.device.create_descriptor_pool(&pool_create_info, None).expect("failed to create descriptor pool");
        //</editor-fold>
        //<editor-fold desc = "descriptor set">
        let layouts = vec![ubo_descriptor_set_layout; MAX_FRAMES_IN_FLIGHT];

        let alloc_info = vk::DescriptorSetAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
            descriptor_pool,
            descriptor_set_count: MAX_FRAMES_IN_FLIGHT as u32,
            p_set_layouts: layouts.as_ptr(),
            ..Default::default()
        };
        let descriptor_sets = base.device.allocate_descriptor_sets(&alloc_info)
            .expect("Failed to allocate descriptor sets");

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let buffer_info = vk::DescriptorBufferInfo {
                buffer: uniform_buffers[i],
                offset: 0,
                range: size_of::<UniformData>() as vk::DeviceSize,
            };
            let image_info = vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image_view: texture_image_view,
                sampler: texture_sampler,
                ..Default::default()
            };
            let descriptor_writes = [
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: descriptor_sets[i],
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: &buffer_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: descriptor_sets[i],
                    dst_binding: 1,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1,
                    p_image_info: &image_info,
                    ..Default::default()
                }
            ];
            base.device.update_descriptor_sets(&descriptor_writes, &[]);
        }
        //</editor-fold>
        //<editor-fold desc = "index buffer">
        let indices = [0u32, 1, 2, 0, 1, 2, 0, 1, 2];
        let indice_buffer_size = 4u64 * 9;
        let mut indice_staging_buffer = Buffer::null();
        let mut indice_staging_buffer_memory = DeviceMemory::null();
        base.create_buffer(
            indice_buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            &mut indice_staging_buffer,
            &mut indice_staging_buffer_memory,
        );

        let indices_ptr = base
            .device
            .map_memory(
                indice_staging_buffer_memory,
                0,
                indice_buffer_size,
                vk::MemoryMapFlags::empty(),
            )
            .expect("Failed to map vertex buffer memory");
        copy_data_to_memory(indices_ptr, &indices);
        base.device.unmap_memory(indice_staging_buffer_memory);

        let mut indice_buffer = Buffer::null();
        let mut indice_buffer_memory = DeviceMemory::null();
        base.create_buffer(
            indice_buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &mut indice_buffer,
            &mut indice_buffer_memory,
        );
        base.copy_buffer(&indice_staging_buffer, &indice_buffer, &indice_buffer_size);
        base.device.destroy_buffer(indice_staging_buffer, None);
        base.device.free_memory(indice_staging_buffer_memory, None);
        //</editor-fold>
        //<editor-fold desc = "vertex buffer">
        let vertex_buffer_size = 3 * size_of::<Vertex>() as u64;
        let mut vertex_input_buffer = Buffer::null();
        let mut vertex_input_buffer_memory = DeviceMemory::null();
        base.create_buffer(
            vertex_buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            &mut vertex_input_buffer,
            &mut vertex_input_buffer_memory,
        );

        let vert_ptr = base
            .device
            .map_memory(
                vertex_input_buffer_memory,
                0,
                vertex_buffer_size,
                vk::MemoryMapFlags::empty(),
            )
            .expect("Failed to map vertex buffer memory");
        copy_data_to_memory(vert_ptr, &vertices);
        base.device.unmap_memory(vertex_input_buffer_memory);

        let mut vertex_buffer = Buffer::null();
        let mut vertex_buffer_memory = DeviceMemory::null();
        base.create_buffer(
            vertex_buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &mut vertex_buffer,
            &mut vertex_buffer_memory,
        );
        base.copy_buffer(&vertex_input_buffer, &vertex_buffer, &vertex_buffer_size);
        base.device.destroy_buffer(vertex_input_buffer, None);
        base.device.free_memory(vertex_input_buffer_memory, None);
        //</editor-fold>
        //<editor-fold desc = "instance buffers">
        let instance_buffer_size = 3 * size_of::<model::Instance>() as u64;
        let mut instance_buffers = Vec::new();
        let mut instance_buffers_memory = Vec::new();
        let mut instance_ptrs = Vec::new();
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            instance_buffers.push(Buffer::null());
            instance_buffers_memory.push(DeviceMemory::null());
            base.create_buffer(
                instance_buffer_size,
                vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                &mut instance_buffers[i],
                &mut instance_buffers_memory[i],
            );
            instance_ptrs.push(base
                .device
                .map_memory(
                    instance_buffers_memory[i],
                    0,
                    instance_buffer_size,
                    vk::MemoryMapFlags::empty(),
                )
                .expect("Failed to map instance buffer memory"));
        }
        let instance_data = [
            model::Instance {matrix: Matrix::new_translation_vec3(&Vector::new_vec3(0.0, 0.0, 0.0)).data, material: 0, _pad: [0,0,0]},
            model::Instance {matrix: Matrix::new_translation_vec3(&Vector::new_vec3(0.0, 0.0, 1.0)).data, material: 1, _pad: [0,0,0]},
            model::Instance {matrix: Matrix::new_translation_vec3(&Vector::new_vec3(0.0, 0.0, -1.0)).data, material: 2, _pad: [0,0,0]},
        ];
        //</editor-fold>
        //<editor-fold desc = "shaders">
        let mut vertex_spv_file = Cursor::new(load_file("hello_triangle\\Triangle.vert.spv")?);
        let mut frag_spv_file = Cursor::new(load_file("hello_triangle\\Triangle.frag.spv")?);

        let vertex_code =
            read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
        let vertex_shader_info = vk::ShaderModuleCreateInfo::default().code(&vertex_code);

        let frag_code =
            read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
        let frag_shader_info = vk::ShaderModuleCreateInfo::default().code(&frag_code);

        let vertex_shader_module = base
            .device
            .create_shader_module(&vertex_shader_info, None)
            .expect("Vertex shader module error");

        let fragment_shader_module = base
            .device
            .create_shader_module(&frag_shader_info, None)
            .expect("Fragment shader module error");
        //</editor-fold>
        //<editor-fold desc = "full graphics pipeline initiation">
        let layout_create_info = vk::PipelineLayoutCreateInfo {
            s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
            set_layout_count: 1,
            p_set_layouts: &ubo_descriptor_set_layout,
            ..Default::default()
        };

        let pipeline_layout = base
            .device
            .create_pipeline_layout(&layout_create_info, None)
            .unwrap();

        let shader_entry_name = c"main";
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo {
                module: vertex_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                module: fragment_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];
        let vertex_input_binding_descriptions = [
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: size_of::<Vertex>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            },
            vk::VertexInputBindingDescription {
                binding: 1,
                stride: size_of::<model::Instance>() as u32,
                input_rate: vk::VertexInputRate::INSTANCE,
            }
        ];
        let vertex_input_attribute_descriptions = [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, normal) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, uv) as u32,
            },

            vk::VertexInputAttributeDescription {
                location: 3,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                location: 4,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 16,
            },
            vk::VertexInputAttributeDescription {
                location: 5,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 32,
            },
            vk::VertexInputAttributeDescription {
                location: 6,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 48,
            },
            vk::VertexInputAttributeDescription {
                location: 7,
                binding: 1,
                format: vk::Format::R32_UINT,
                offset: 64,
            },
        ];

        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);
        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };
        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: base.surface_resolution.width as f32,
            height: base.surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [base.surface_resolution.into()];
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
            .scissors(&scissors)
            .viewports(&viewports);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            cull_mode: vk::CullModeFlags::NONE,
            line_width: 1.0,
            polygon_mode: vk::PolygonMode::FILL,
            ..Default::default()
        };
        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        let noop_stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            ..Default::default()
        };
        let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: 1,
            depth_write_enable: 1,
            depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
            front: noop_stencil_state,
            back: noop_stencil_state,
            max_depth_bounds: 1.0,
            ..Default::default()
        };
        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
            blend_enable: 0,
            src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ZERO,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        }];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);

        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_state);

        let graphic_pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_state_info)
            .depth_stencil_state(&depth_state_info)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(renderpass);

        let graphics_pipelines = base
            .device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[graphic_pipeline_info], None)
            .expect("Unable to create graphics pipeline");

        let graphic_pipeline = graphics_pipelines[0];
        //</editor-fold>
        let model_test = Gltf::new("C:\\Graphics\\assets\\luna\\MRLunaSnow.gltf");
        model_test.construct_buffers(base);
        println!("{:?}", model_test.scene.nodes[0].borrow().children[0].borrow().mesh.clone().unwrap().borrow().name);

        let mut player_camera = Camera::new_perspective_rotation(
            Vector::new_vec3(0.0, 0.0, 0.0),
            Vector::new_empty(),
            1.0,
            0.001,
            100.0
        );
        let mut current_frame = 0usize;
        let mut pressed_keys = HashSet::new();
        let mut new_pressed_keys = HashSet::new();
        let mut mouse_delta = (0.0, 0.0);
        let mut last_frame_time = Instant::now();
        let mut cursor_locked = false;
        let mut saved_cursor_pos = PhysicalPosition::new(0.0, 0.0);
        base.event_loop.borrow_mut().run_on_demand(|event, elwp| {
            elwp.set_control_flow(ControlFlow::Poll);
            match event {
                //<editor-fold desc = "event handling">
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    elwp.exit();
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized( new_size ),
                    ..
                } => {
                    println!("bruh");
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {

                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput {
                        event: KeyEvent {
                            state,
                            physical_key,
                            ..
                        },
                        ..
                    },
                    ..
                } => {
                    match state {
                        ElementState::Pressed => {
                            if !pressed_keys.contains(&physical_key) { new_pressed_keys.insert(physical_key.clone()); }
                            pressed_keys.insert(physical_key.clone());
                        }
                        ElementState::Released => {
                            pressed_keys.remove(&physical_key);
                            new_pressed_keys.remove(&physical_key);
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    ..
                } => {
                    if base.window.has_focus() && cursor_locked {
                        mouse_delta = (
                            -position.x as f32 + 0.5 * base.window.inner_size().width as f32,
                            position.y as f32 - 0.5 * base.window.inner_size().height as f32,
                        );
                        base.window.set_cursor_position(PhysicalPosition::new(
                            base.window.inner_size().width as f32 * 0.5,
                            base.window.inner_size().height as f32 * 0.5))
                        .expect("failed to reset mouse position");
                        do_mouse(&mut player_camera, mouse_delta, &mut cursor_locked);
                    } else {
                        saved_cursor_pos = position;
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Focused(true),
                    ..
                } => {
                    if !cursor_locked {
                        if let Err(err) = base.window.set_cursor_grab(CursorGrabMode::Confined) {
                            eprintln!("Cursor lock failed: {:?}", err);
                        } else {
                            base.window.set_cursor_visible(false);
                            cursor_locked = true;
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Focused(false),
                    ..
                } => {
                    cursor_locked = false;
                    if let Err(err) = base.window.set_cursor_grab(CursorGrabMode::None) {
                        eprintln!("Cursor unlock failed: {:?}", err);
                    } else {
                        base.window.set_cursor_visible(true);
                    }
                    base.window.set_cursor_position(saved_cursor_pos).expect("Cursor pos reset failed");
                }
                //</editor-fold>
                Event::AboutToWait => {
                    let now = Instant::now();
                    let delta_time = now.duration_since(last_frame_time).as_secs_f32();
                    last_frame_time = now;
                    do_controls(&mut player_camera, &pressed_keys, &mut new_pressed_keys, delta_time, &mut cursor_locked, base, &mut saved_cursor_pos);

                    let (present_index, _) = base
                        .swapchain_loader
                        .acquire_next_image(
                            base.swapchain,
                            u64::MAX,
                            base.present_complete_semaphore,
                            vk::Fence::null(),
                        )
                        .unwrap();
                    let clear_values = [
                        vk::ClearValue {
                            color: vk::ClearColorValue {
                                float32: [0.0, 0.0, 0.0, 0.0],
                            },
                        },
                        vk::ClearValue {
                            depth_stencil: vk::ClearDepthStencilValue {
                                depth: 1.0,
                                stencil: 0,
                            },
                        },
                    ];
                    player_camera.update_matrices(&base);
                    let ubo = UniformData {
                        view: player_camera.view_matrix.data,
                        projection: player_camera.projection_matrix.data,
                    };
                    copy_data_to_memory(uniform_buffers_mapped[current_frame], &[ubo]);

                    copy_data_to_memory(instance_ptrs[current_frame], &instance_data);

                    let render_pass_begin_info = vk::RenderPassBeginInfo::default()
                        .render_pass(renderpass)
                        .framebuffer(framebuffers[present_index as usize])
                        .render_area(base.surface_resolution.into())
                        .clear_values(&clear_values);

                    let current_rendering_complete_semaphore = base.rendering_complete_semaphores[current_frame];
                    let current_draw_command_buffer = base.draw_command_buffers[current_frame];
                    let current_draw_commands_reuse_fence = base.draw_commands_reuse_fences[current_frame];
                    record_submit_commandbuffer(
                        &base.device,
                        current_draw_command_buffer,
                        current_draw_commands_reuse_fence,
                        base.present_queue,
                        &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
                        &[base.present_complete_semaphore],
                        &[current_rendering_complete_semaphore],
                        |device, draw_command_buffer| {
                            device.cmd_begin_render_pass(
                                draw_command_buffer,
                                &render_pass_begin_info,
                                vk::SubpassContents::INLINE,
                            );
                            device.cmd_bind_pipeline(
                                draw_command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                graphic_pipeline,
                            );
                            device.cmd_set_viewport(draw_command_buffer, 0, &viewports);
                            device.cmd_set_scissor(draw_command_buffer, 0, &scissors);
                            device.cmd_bind_descriptor_sets(
                                draw_command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                pipeline_layout,
                                0,
                                &[descriptor_sets[current_frame]],
                                &[],
                            );
                            device.cmd_bind_vertex_buffers(
                                draw_command_buffer,
                                1,
                                &[instance_buffers[current_frame]],
                                &[0],
                            );
                            for node in model_test.scene.nodes.iter() {
                                node.borrow().draw(base, &draw_command_buffer, &Matrix::new())
                            }
                            device.cmd_bind_vertex_buffers(
                                draw_command_buffer,
                                0,
                                &[vertex_buffer],
                                &[0],
                            );
                            device.cmd_bind_index_buffer(
                                draw_command_buffer,
                                indice_buffer,
                                0,
                                vk::IndexType::UINT32,
                            );
                            device.cmd_draw_indexed(
                                draw_command_buffer,
                                indices.len() as u32,
                                3,
                                0,
                                0,
                                0,
                            );

                            device.cmd_end_render_pass(draw_command_buffer);
                        },
                    );
                    let wait_semaphores = [current_rendering_complete_semaphore];
                    let swapchains = [base.swapchain];
                    let image_indices = [present_index];
                    let present_info = vk::PresentInfoKHR::default()
                        .wait_semaphores(&wait_semaphores)
                        .swapchains(&swapchains)
                        .image_indices(&image_indices);

                    base.swapchain_loader
                        .queue_present(base.present_queue, &present_info)
                        .unwrap();
                    current_frame = (current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
                },
                _ => (),
            }
        }).expect("Failed to initiate render loop");

        println!("Render loop exited successfully, cleaning up");
        //<editor-fold desc = "cleanup">
        base.device.device_wait_idle().unwrap();

        for pipeline in graphics_pipelines {
            base.device.destroy_pipeline(pipeline, None);
        }
        base.device.destroy_pipeline_layout(pipeline_layout, None);

        base.device.destroy_shader_module(vertex_shader_module, None);
        base.device.destroy_shader_module(fragment_shader_module, None);

        base.device.free_memory(vertex_buffer_memory, None);
        base.device.destroy_buffer(vertex_buffer, None);

        base.device.free_memory(indice_buffer_memory, None);
        base.device.destroy_buffer(indice_buffer, None);

        for framebuffer in framebuffers {
            base.device.destroy_framebuffer(framebuffer, None);
        }

        base.device.destroy_render_pass(renderpass, None);

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            base.device.destroy_buffer(uniform_buffers[i], None);
            base.device.free_memory(uniform_buffers_memory[i], None);

            base.device.destroy_buffer(instance_buffers[i], None);
            base.device.unmap_memory(instance_buffers_memory[i]);
            base.device.free_memory(instance_buffers_memory[i], None);
        }

        base.device.destroy_descriptor_set_layout(ubo_descriptor_set_layout, None);
        base.device.destroy_descriptor_pool(descriptor_pool, None);

        base.device.destroy_sampler(texture_sampler, None);
        base.device.destroy_image_view(texture_image_view, None);
        base.device.free_memory(texture_image_memory, None);
        base.device.destroy_image(texture_image, None);
        //</editor-fold>
    }
    Ok(())
}

fn do_controls(
    player_camera: &mut Camera,
    pressed_keys: &HashSet<PhysicalKey>,
    new_pressed_keys: &mut HashSet<PhysicalKey>,
    delta_time: f32,
    cursor_locked: &mut bool,
    base: &VkBase,
    saved_cursor_pos: &mut PhysicalPosition<f64>,
) {
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyW)) {
        player_camera.position.x -= player_camera.speed*delta_time * (player_camera.rotation.y + PI/2.0).cos();
        player_camera.position.z += player_camera.speed*delta_time * (player_camera.rotation.y + PI/2.0).sin();
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyA)) {
        player_camera.position.x += player_camera.speed*delta_time * player_camera.rotation.y.cos();
        player_camera.position.z -= player_camera.speed*delta_time * player_camera.rotation.y.sin();
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyS)) {
        player_camera.position.x += player_camera.speed*delta_time * (player_camera.rotation.y + PI/2.0).cos();
        player_camera.position.z -= player_camera.speed*delta_time * (player_camera.rotation.y + PI/2.0).sin();
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyD)) {
        player_camera.position.x -= player_camera.speed*delta_time * player_camera.rotation.y.cos();
        player_camera.position.z += player_camera.speed*delta_time * player_camera.rotation.y.sin();
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::Space)) {
        player_camera.position.y += player_camera.speed*delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::ShiftLeft)) {
        player_camera.position.y -= player_camera.speed*delta_time;
    }

    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::ArrowUp)) {
        player_camera.rotation.x -= delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::ArrowDown)) {
        player_camera.rotation.x += delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::ArrowLeft)) {
        player_camera.rotation.y += delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::ArrowRight)) {
        player_camera.rotation.y -= delta_time;
    }

    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::Equal)) {
        player_camera.speed *= 1.0 + 1.0*delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::Minus)) {
        player_camera.speed /= 1.0 + 1.0*delta_time;
    }

    if new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::Escape)) {
        *cursor_locked = !*cursor_locked;
        if *cursor_locked {
            if let Err(err) = base.window.set_cursor_grab(CursorGrabMode::Confined) {
                eprintln!("Cursor lock failed: {:?}", err);
            } else {
                base.window.set_cursor_visible(false);
            }
            base.window.set_cursor_position(PhysicalPosition::new(
                base.window.inner_size().width as f32 * 0.5,
                base.window.inner_size().height as f32 * 0.5))
                .expect("failed to reset mouse position");
        } else {
            if let Err(err) = base.window.set_cursor_grab(CursorGrabMode::None) {
                eprintln!("Cursor unlock failed: {:?}", err);
            } else {
                base.window.set_cursor_visible(true);
            }
            base.window.set_cursor_position(*saved_cursor_pos).expect("Cursor pos reset failed");
        }
    }

    new_pressed_keys.clear();
}
fn do_mouse(player_camera: &mut Camera, mouse_delta: (f32, f32), cursor_locked: &mut bool) {
    if *cursor_locked {
        player_camera.rotation.y += player_camera.sensitivity * mouse_delta.0;
        player_camera.rotation.x += player_camera.sensitivity * mouse_delta.1;
        player_camera.rotation.x = player_camera.rotation.x.clamp(-89.99_f32, 89.99_f32);
    }
}