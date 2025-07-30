#![warn(unused_qualifications)]
mod matrix;
mod vector;
mod vk_helper;
mod camera;
mod scene;

use std::default::Default;
use std::error::Error;
use std::mem;
use std::collections::HashSet;
use std::ffi::c_void;
use std::mem::size_of;
use std::path::PathBuf;
use std::time::Instant;

use ash::vk;
use ash::vk::{Buffer, DeviceMemory, Handle};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::CursorGrabMode;
use crate::{vk_helper::*, vector::*};
use crate::scene::{Scene, Model, Instance};
use crate::camera::Camera;

#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct UniformData {
    view: [f32; 16],
    projection: [f32; 16],
}

const MAX_FRAMES_IN_FLIGHT: usize = 2;
const PI: f32 = std::f32::consts::PI;

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        let mut shader_paths = Vec::new();
        shader_paths.push("src\\shaders\\glsl\\geometry");

        compile_shaders(shader_paths).expect("Failed to compile shaders");

        let mut base = VkBase::new(1000, 800, MAX_FRAMES_IN_FLIGHT)?;
        run(&mut base).expect("Application launch failed!");
    }
    Ok(())
}

unsafe fn run(base: &mut VkBase) -> Result<(), Box<dyn Error>> {
    unsafe {
        let mut world = Scene::new();
        world.add_model(Model::new(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("local_assets\\ffocks\\untitled.gltf").to_str().unwrap()));
        world.models[0].transform_roots(&Vector::new_vec(0.0), &Vector::new_vec(0.0), &Vector::new_vec(0.01));
        //world.add_model(Model::new("C:\\Graphics\\assets\\sponzaGLTF\\sponza.gltf"));
        //world.add_model(Model::new("C:\\Graphics\\assets\\flower\\scene.gltf"));
        //world.add_model(Model::new("C:\\Graphics\\assets\\rivals\\luna\\gltf\\luna.gltf"));
        //world.add_model(Model::new("C:\\Graphics\\assets\\bistro2\\untitled.gltf"));
        world.initialize(base, MAX_FRAMES_IN_FLIGHT, true);
        // world.models[2].animations[0].repeat = true;
        // world.models[2].animations[0].start();
        world.models[0].animations[0].repeat = true;
        world.models[0].animations[0].start();

        let null_tex = base.create_2d_texture_image(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("local_assets\\null8x.png"), true);

        //<editor-fold desc = "uniform buffers">
        let layout_bindings = [
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 2,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 3,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1024u32,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];
        let binding_flags = [
            vk::DescriptorBindingFlags::empty(),

            vk::DescriptorBindingFlags::empty(),

            vk::DescriptorBindingFlags::empty(),

            vk::DescriptorBindingFlags::PARTIALLY_BOUND |
                vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT |
                vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
        ];
        let binding_flags_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_BINDING_FLAGS_CREATE_INFO,
            binding_count: binding_flags.len() as u32,
            p_binding_flags: binding_flags.as_ptr(),
            ..Default::default()
        };
        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            p_next: &binding_flags_info as *const _ as *const c_void,
            binding_count: layout_bindings.len() as u32,
            p_bindings: layout_bindings.as_ptr(),
            flags: vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
            ..Default::default()
        };
        let ubo_descriptor_set_layout = base.device.create_descriptor_set_layout(&layout_info, None)?;

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
            }, // uniform buffer
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
                ..Default::default()
            }, // material SSBO
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
                ..Default::default()
            }, // joint SSBO
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: (MAX_FRAMES_IN_FLIGHT * 1024) as u32,
                ..Default::default()
            }, // array of textures
        ];
        let pool_create_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
            max_sets: MAX_FRAMES_IN_FLIGHT as u32,
            flags: vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
            ..Default::default()
        };
        let descriptor_pool = base.device.create_descriptor_pool(&pool_create_info, None).expect("failed to create descriptor pool");
        //</editor-fold>
        //<editor-fold desc = "descriptor set">
        let layouts = vec![ubo_descriptor_set_layout; MAX_FRAMES_IN_FLIGHT];
        let variable_counts = vec![1024u32; MAX_FRAMES_IN_FLIGHT];
        let variable_count_info = vk::DescriptorSetVariableDescriptorCountAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_VARIABLE_DESCRIPTOR_COUNT_ALLOCATE_INFO,
            descriptor_set_count: MAX_FRAMES_IN_FLIGHT as u32,
            p_descriptor_counts: variable_counts.as_ptr(),
            ..Default::default()
        };
        let alloc_info = vk::DescriptorSetAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next: &variable_count_info as *const _ as *const c_void,
            descriptor_pool,
            descriptor_set_count: MAX_FRAMES_IN_FLIGHT as u32,
            p_set_layouts: layouts.as_ptr(),
            ..Default::default()
        };
        let descriptor_sets = base.device.allocate_descriptor_sets(&alloc_info)
            .expect("Failed to allocate descriptor sets");

        let mut image_infos: Vec<vk::DescriptorImageInfo> = Vec::with_capacity(1024);
        for model in &world.models {
            for texture in &model.textures {
                if texture.borrow().source.borrow().generated {
                    image_infos.push(vk::DescriptorImageInfo {
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        image_view: texture.borrow().source.borrow().image_view,
                        sampler: texture.borrow().sampler,
                        ..Default::default()
                    });
                } else {
                    image_infos.push(vk::DescriptorImageInfo {
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        image_view: null_tex.0.0,
                        sampler: null_tex.0.1,
                        ..Default::default()
                    });
                }
            }
        }
        let missing = 1024 - image_infos.len();
        for _ in 0..missing {
            image_infos.push(vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image_view: null_tex.0.0,
                sampler: null_tex.0.1,
                ..Default::default()
            });
        }
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let uniform_buffer_info = vk::DescriptorBufferInfo {
                buffer: uniform_buffers[i],
                offset: 0,
                range: size_of::<UniformData>() as vk::DeviceSize,
            };
            let material_ssbo_info = vk::DescriptorBufferInfo {
                buffer: world.material_buffers[i].0,
                offset: 0,
                range: vk::WHOLE_SIZE,
            };
            let joints_ssbo_info = vk::DescriptorBufferInfo {
                buffer: world.joints_buffers[i].0,
                offset: 0,
                range: vk::WHOLE_SIZE,
            };
            let descriptor_writes = [
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: descriptor_sets[i],
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: &uniform_buffer_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: descriptor_sets[i],
                    dst_binding: 1,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: &material_ssbo_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: descriptor_sets[i],
                    dst_binding: 2,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: &joints_ssbo_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: descriptor_sets[i],
                    dst_binding: 3,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1024,
                    p_image_info: image_infos.as_ptr(),
                    ..Default::default()
                },
            ];
            base.device.update_descriptor_sets(&descriptor_writes, &[]);
        }
        //</editor-fold>

        //<editor-fold desc = "geometry pass">
        let g_depth_view = VkBase::create_depth_image(
            &base.instance,
            &base.pdevice,
            &base.surface_resolution,
            &base.device,
            vk::SampleCountFlags::TYPE_1,
        );
        let mut g_material_views = Vec::new();
        let mut g_position_views = Vec::new();
        let mut g_normal_views = Vec::new();
        let mut g_view_position_views = Vec::new();
        let mut g_view_normal_views = Vec::new();
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            g_material_views.push(VkBase::create_color_image(
                &base.instance,
                &base.pdevice,
                &base.surface_resolution,
                &base.device,
                vk::SampleCountFlags::TYPE_1,
                vk::Format::R8G8B8A8_SINT,
            ));
            g_position_views.push(VkBase::create_color_image(
                &base.instance,
                &base.pdevice,
                &base.surface_resolution,
                &base.device,
                vk::SampleCountFlags::TYPE_1,
                vk::Format::R16G16B16A16_SFLOAT,
            ));
            g_normal_views.push(VkBase::create_color_image(
                &base.instance,
                &base.pdevice,
                &base.surface_resolution,
                &base.device,
                vk::SampleCountFlags::TYPE_1,
                vk::Format::R16G16B16A16_SFLOAT,
            ));
            g_view_position_views.push(VkBase::create_color_image(
                &base.instance,
                &base.pdevice,
                &base.surface_resolution,
                &base.device,
                vk::SampleCountFlags::TYPE_1,
                vk::Format::R16G16B16A16_SFLOAT,
            ));
            g_view_normal_views.push(VkBase::create_color_image(
                &base.instance,
                &base.pdevice,
                &base.surface_resolution,
                &base.device,
                vk::SampleCountFlags::TYPE_1,
                vk::Format::R16G16B16A16_SFLOAT,
            ));
        }
        let renderpass_attachments = [
            //depth
            vk::AttachmentDescription {
                format: vk::Format::D16_UNORM,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                initial_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
            //material
            vk::AttachmentDescription {
                format: vk::Format::R8G8B8A8_SINT,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
            //position
            vk::AttachmentDescription {
                format: vk::Format::R16G16B16A16_SFLOAT,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
            //normal
            vk::AttachmentDescription {
                format: vk::Format::R16G16B16A16_SFLOAT,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
            //view position
            vk::AttachmentDescription {
                format: vk::Format::R16G16B16A16_SFLOAT,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
            //view normal
            vk::AttachmentDescription {
                format: vk::Format::R16G16B16A16_SFLOAT,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
        ];
        let color_attachment_refs = [
            vk::AttachmentReference {
                attachment: 1,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,

            },
            vk::AttachmentReference {
                attachment: 2,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            },
            vk::AttachmentReference {
                attachment: 3,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,

            },
            vk::AttachmentReference {
                attachment: 4,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            },
            vk::AttachmentReference {
                attachment: 5,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            },
        ];
        let depth_attachment_ref = vk::AttachmentReference {
            attachment: 0,
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
        let geometry_pass = base
            .device
            .create_render_pass(&renderpass_create_info, None)
            .unwrap();
        //</editor-fold>
        //<editor-fold desc = "present pass">
        let renderpass_attachments = [
            vk::AttachmentDescription {
                format: base.surface_format.format,
                samples: base.msaa_samples,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                final_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
            vk::AttachmentDescription {
                format: vk::Format::D16_UNORM,
                samples: base.msaa_samples,
                load_op: vk::AttachmentLoadOp::CLEAR,
                initial_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
            vk::AttachmentDescription {
                format: base.surface_format.format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::DONT_CARE,
                store_op: vk::AttachmentStoreOp::STORE,
                stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
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
        let resolve_attachment_ref = [vk::AttachmentReference {
            attachment: 2,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];
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
            .resolve_attachments(&resolve_attachment_ref)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

        let renderpass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(&renderpass_attachments)
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(&dependencies);

        let present_pass = base
            .device
            .create_render_pass(&renderpass_create_info, None)
            .unwrap();
        //</editor-fold>
        //<editor-fold desc = "framebuffers">
        let present_framebuffers: Vec<vk::Framebuffer> = base
            .present_image_views
            .iter()
            .map(|&present_image_view| {
                let framebuffer_attachments = [base.color_image_view, base.depth_image_view, present_image_view];
                let framebuffer_create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(present_pass)
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
        let geometry_framebuffers: Vec<vk::Framebuffer> = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|i| {
                let framebuffer_attachments = [
                    g_depth_view.1,
                    g_material_views[i].1,
                    g_position_views[i].1,
                    g_normal_views[i].1,
                    g_view_position_views[i].1,
                    g_view_normal_views[i].1,
                ];
                let framebuffer_create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(geometry_pass)
                    .attachments(&framebuffer_attachments)
                    .width(base.surface_resolution.width)
                    .height(base.surface_resolution.height)
                    .layers(1);
                let fb = base
                    .device
                    .create_framebuffer(&framebuffer_create_info, None)
                    .expect("Failed to create framebuffer");
                println!("Created geometry framebuffer[{}]: 0x{:x}", i, fb.as_raw());
                fb
            })
            .collect();
        //</editor-fold>
        //<editor-fold desc = "shaders">
        let shader = Shader::new(base, "geometry\\geometry.vert.spv", "geometry\\geometry.frag.spv");
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

        let vertex_input_binding_descriptions = [
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: size_of::<scene::Vertex>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            },
            vk::VertexInputBindingDescription {
                binding: 1,
                stride: size_of::<Instance>() as u32,
                input_rate: vk::VertexInputRate::INSTANCE,
            }
        ];
        let vertex_input_attribute_descriptions = [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(scene::Vertex, position) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(scene::Vertex, normal) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(scene::Vertex, uv) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 3,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(scene::Vertex, tangent) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 4,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(scene::Vertex, bitangent) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 5,
                binding: 0,
                format: vk::Format::R32G32B32A32_UINT,
                offset: offset_of!(scene::Vertex, joint_indices) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 6,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(scene::Vertex, joint_weights) as u32,
            },

            vk::VertexInputAttributeDescription {
                location: 7,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                location: 8,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 16,
            },
            vk::VertexInputAttributeDescription {
                location: 9,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 32,
            },
            vk::VertexInputAttributeDescription {
                location: 10,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 48,
            },
            vk::VertexInputAttributeDescription {
                location: 11,
                binding: 1,
                format: vk::Format::R32G32_SINT,
                offset: offset_of!(Instance, indices) as u32,
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
            rasterization_samples: base.msaa_samples,
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

        let shader_create_info = shader.generate_shader_stage_create_infos();
        let graphic_pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_create_info)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_state_info)
            .depth_stencil_state(&depth_state_info)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(present_pass);

        let graphics_pipelines = base
            .device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[graphic_pipeline_info], None)
            .expect("Unable to create graphics pipeline");

        let graphic_pipeline = graphics_pipelines[0];
        //</editor-fold>

        let mut player_camera = Camera::new_perspective_rotation(
            Vector::new_vec3(0.0, 0.0, 0.0),
            Vector::new_empty(),
            1.0,
            0.001,
            100.0,
            base.window.inner_size().width as f32 / base.window.inner_size().height as f32,
            0.15,
            1000.0
        );
        let mut current_frame = 0usize;
        let mut pressed_keys = HashSet::new();
        let mut new_pressed_keys = HashSet::new();
        let mut mouse_delta = (0.0, 0.0);
        let mut last_frame_time = Instant::now();
        let mut cursor_locked = false;
        let mut saved_cursor_pos = PhysicalPosition::new(0.0, 0.0);
        let mut needs_resize = false;

        let mut pause_frustum = false;
        base.window.set_cursor_position(PhysicalPosition::new(
            base.window.inner_size().width as f32 * 0.5,
            base.window.inner_size().height as f32 * 0.5))
            .expect("failed to reset mouse position");

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
                    player_camera.aspect_ratio = base.window.inner_size().width as f32 / base.window.inner_size().height as f32;
                    needs_resize = true;
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
                    if needs_resize {

                    }
                    do_controls(&mut player_camera, &pressed_keys, &mut new_pressed_keys, delta_time, &mut cursor_locked, base, &mut saved_cursor_pos, &mut pause_frustum);
                    player_camera.update_matrices();

                    world.update_nodes(base, current_frame);

                    if !pause_frustum {
                        player_camera.update_frustum()
                    }

                    let current_fence = base.draw_commands_reuse_fences[current_frame];
                    base.device.wait_for_fences(&[current_fence], true, u64::MAX).expect("wait failed");
                    base.device.reset_fences(&[current_fence]).expect("reset failed");

                    let (present_index, _) = base
                        .swapchain_loader
                        .acquire_next_image(
                            base.swapchain,
                            u64::MAX,
                            base.present_complete_semaphores[current_frame],
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

                    let ubo = UniformData {
                        view: player_camera.view_matrix.data,
                        projection: player_camera.projection_matrix.data,
                    };
                    copy_data_to_memory(uniform_buffers_mapped[current_frame], &[ubo]);

                    let render_pass_begin_info = vk::RenderPassBeginInfo::default()
                        .render_pass(present_pass)
                        .framebuffer(present_framebuffers[present_index as usize])
                        .render_area(base.surface_resolution.into())
                        .clear_values(&clear_values);

                    let current_rendering_complete_semaphore = base.rendering_complete_semaphores[current_frame];
                    let current_draw_command_buffer = base.draw_command_buffers[current_frame];
                    record_submit_commandbuffer(
                        &base.device,
                        current_draw_command_buffer,
                        current_fence,
                        base.present_queue,
                        &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
                        &[base.present_complete_semaphores[current_frame]],
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

                            // draw scene
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
                            world.draw(base, &draw_command_buffer, current_frame, &player_camera.frustum);

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

        shader.destroy(base);
        world.destroy(base);

        base.device.destroy_render_pass(geometry_pass, None);
        base.device.destroy_render_pass(present_pass, None);

        base.device.destroy_image(g_depth_view.0, None);
        base.device.destroy_image_view(g_depth_view.1, None);
        base.device.free_memory(g_depth_view.2, None);
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            base.device.destroy_buffer(uniform_buffers[i], None);
            base.device.free_memory(uniform_buffers_memory[i], None);

            base.device.destroy_framebuffer(geometry_framebuffers[i], None);

            base.device.destroy_image(g_material_views[i].0, None);
            base.device.destroy_image_view(g_material_views[i].1, None);
            base.device.free_memory(g_material_views[i].2, None);
            base.device.destroy_image(g_position_views[i].0, None);
            base.device.destroy_image_view(g_position_views[i].1, None);
            base.device.free_memory(g_position_views[i].2, None);
            base.device.destroy_image(g_normal_views[i].0, None);
            base.device.destroy_image_view(g_normal_views[i].1, None);
            base.device.free_memory(g_normal_views[i].2, None);
            base.device.destroy_image(g_view_position_views[i].0, None);
            base.device.destroy_image_view(g_view_position_views[i].1, None);
            base.device.free_memory(g_view_position_views[i].2, None);
            base.device.destroy_image(g_view_normal_views[i].0, None);
            base.device.destroy_image_view(g_view_normal_views[i].1, None);
            base.device.free_memory(g_view_normal_views[i].2, None);
        }
        for i in 0..3 {
            base.device.destroy_framebuffer(present_framebuffers[i], None);
        }

        base.device.destroy_descriptor_set_layout(ubo_descriptor_set_layout, None);
        base.device.destroy_descriptor_pool(descriptor_pool, None);

        base.device.destroy_image_view(null_tex.0.0, None);
        base.device.destroy_sampler(null_tex.0.1, None);
        base.device.destroy_image(null_tex.1.0, None);
        base.device.free_memory(null_tex.1.1, None);
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
    paused: &mut bool,
) {
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyW)) {
        player_camera.position.x += player_camera.speed*delta_time * (player_camera.rotation.y + PI/2.0).cos();
        player_camera.position.z += player_camera.speed*delta_time * (player_camera.rotation.y + PI/2.0).sin();
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyA)) {
        player_camera.position.x -= player_camera.speed*delta_time * player_camera.rotation.y.cos();
        player_camera.position.z -= player_camera.speed*delta_time * player_camera.rotation.y.sin();
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyS)) {
        player_camera.position.x -= player_camera.speed*delta_time * (player_camera.rotation.y + PI/2.0).cos();
        player_camera.position.z -= player_camera.speed*delta_time * (player_camera.rotation.y + PI/2.0).sin();
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyD)) {
        player_camera.position.x += player_camera.speed*delta_time * player_camera.rotation.y.cos();
        player_camera.position.z += player_camera.speed*delta_time * player_camera.rotation.y.sin();
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::Space)) {
        player_camera.position.y += player_camera.speed*delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::ShiftLeft)) {
        player_camera.position.y -= player_camera.speed*delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::ArrowUp)) {
        player_camera.rotation.x += delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::ArrowDown)) {
        player_camera.rotation.x -= delta_time;
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
    if new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyP)) {
        *paused = !*paused
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