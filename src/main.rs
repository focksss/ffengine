#![warn(unused_qualifications)]
mod matrix;
mod vector;
mod vk_initializer;

use std::default::Default;
use std::cell::RefCell;
use std::error::Error;
use std::io::Cursor;
use std::{fs, io, mem};
use std::ffi::c_void;
use std::mem::{align_of, size_of, size_of_val};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::null;
// TODO: Remove when bumping MSRV to 1.80

use ash::util::*;
use ash::vk;
use ash::vk::{Buffer, DeviceMemory};
use crate::{vk_initializer::*, matrix::*, vector::*};

#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct Vertex {
    pos: [f32; 4],
    color: [f32; 4],
}
#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct UniformData {
    view: [f32; 16],
}

const MAX_FRAMES_IN_FLIGHT: usize = 2;

fn main() -> Result<(), Box<dyn Error>> {
    let vertices = [
        Vertex {
            pos: [-1.0, 1.0, 0.0, 1.0],
            color: [1.0, 0.0, 0.0, 1.0],
        },
        Vertex {
            pos: [1.0, 1.0, 0.0, 1.0],
            color: [0.0, 1.0, 0.0, 1.0],
        },
        Vertex {
            pos: [0.0, -1.0, 0.0, 1.0],
            color: [0.0, 0.0, 1.0, 1.0],
        },
    ];
    let uniform_data = UniformData {
        view: Matrix::new().data
    };
    unsafe {
        println!("main code running");
        let mut shader_paths = Vec::new();
        shader_paths.push("src\\shaders\\glsl\\hello_triangle");

        compile_shaders(shader_paths).expect("Failed to compile shaders");

        let mut base = VkBase::new(800, 600, MAX_FRAMES_IN_FLIGHT)?;
        init_rendering(&mut base, vertices).expect("Application launch failed!");
    }
    Ok(())
}

unsafe fn init_rendering(base: &mut VkBase, vertices: [Vertex; 3]) -> Result<(), Box<dyn Error>> {
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
        //<editor-fold desc = "uniform buffers">
        let ubo_layout_binding = vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            ..Default::default()
        };
        let ubo_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            binding_count: 1,
            p_bindings: &ubo_layout_binding,
            ..Default::default()
        };
        let ubo_descriptor_set_layout = unsafe { base.device.create_descriptor_set_layout(&ubo_layout_create_info, None)? };
        
        let ubo_buffer_size = size_of::<UniformData>() as u64;
        let mut uniform_buffers = Vec::new();
        let mut uniform_buffers_memory = Vec::new();
        let mut uniform_buffers_mapped = Vec::new();
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            uniform_buffers.push(Buffer::null());
            uniform_buffers_memory.push(DeviceMemory::null());
            create_buffer(
                base,
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
        let pool_size = vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
            ..Default::default()
        };
        let pool_create_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            pool_size_count: 1,
            p_pool_sizes: &pool_size,
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
            let descriptor_write = vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                dst_set: descriptor_sets[i],
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                p_buffer_info: &buffer_info,
                ..Default::default()
            };
            base.device.update_descriptor_sets(&[descriptor_write], &[]);
        }
        //</editor-fold>
        //<editor-fold desc = "vertex buffer">
        let vertex_buffer_size = 3 * size_of::<Vertex>() as u64;
        let mut vertex_input_buffer = Buffer::null();
        let mut vertex_input_buffer_memory = DeviceMemory::null();
        create_buffer(
            &base,
            vertex_buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
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
        //</editor-fold>
        //<editor-fold desc = "shaders">
        let mut vertex_spv_file = Cursor::new(load_file("hello_triangle\\triangle.vert.spv")?);
        let mut frag_spv_file = Cursor::new(load_file("hello_triangle\\triangle.frag.spv")?);

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
        let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
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
                offset: offset_of!(Vertex, color) as u32,
            },
        ];

        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);
        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
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
        // SETUP RENDER LOOP, AUTO RUNS
        let current_frame = RefCell::new(0usize);
        let _ = base.render_loop(|| {
            let mut frame = current_frame.borrow_mut();
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

            let ubo = UniformData {view: Matrix::new().data};
            copy_data_to_memory(uniform_buffers_mapped[*frame], &[ubo]);

            let render_pass_begin_info = vk::RenderPassBeginInfo::default()
                .render_pass(renderpass)
                .framebuffer(framebuffers[present_index as usize])
                .render_area(base.surface_resolution.into())
                .clear_values(&clear_values);

            let current_rendering_complete_semaphore = base.rendering_complete_semaphores[*frame];
            let current_draw_command_buffer = base.draw_command_buffers[*frame];
            let current_draw_commands_reuse_fence = base.draw_commands_reuse_fences[*frame];
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
                    device.cmd_bind_vertex_buffers(
                        draw_command_buffer,
                        0,
                        &[vertex_input_buffer],
                        &[0],
                    );

                    device.cmd_bind_descriptor_sets(
                        draw_command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline_layout,
                        0,
                        &[descriptor_sets[*frame]],
                        &[],
                    );

                    device.cmd_draw(draw_command_buffer, 3, 1, 0, 0);

                    device.cmd_end_render_pass(draw_command_buffer);
                },
            );
            let wait_semaphores = [current_rendering_complete_semaphore];
            let swapchains = [base.swapchain];
            let image_indices = [present_index];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&wait_semaphores) // &base.rendering_complete_semaphore)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            base.swapchain_loader
                .queue_present(base.present_queue, &present_info)
                .unwrap();

            *frame = (*frame + 1) % MAX_FRAMES_IN_FLIGHT;
        });
        println!("Render loop exited successfully, cleaning up");
        //<editor-fold desc = "cleanup">
        base.device.device_wait_idle().unwrap();
        for pipeline in graphics_pipelines {
            base.device.destroy_pipeline(pipeline, None);
        }
        base.device.destroy_pipeline_layout(pipeline_layout, None);
        base.device.destroy_shader_module(vertex_shader_module, None);
        base.device.destroy_shader_module(fragment_shader_module, None);
        base.device.free_memory(vertex_input_buffer_memory, None);
        base.device.destroy_buffer(vertex_input_buffer, None);
        for framebuffer in framebuffers {
            base.device.destroy_framebuffer(framebuffer, None);
        }
        base.device.destroy_render_pass(renderpass, None);
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            base.device.destroy_buffer(uniform_buffers[i], None);
            base.device.free_memory(uniform_buffers_memory[i], None);
        }
        base.device.destroy_descriptor_set_layout(ubo_descriptor_set_layout, None);
        base.device.destroy_descriptor_pool(descriptor_pool, None);
        //</editor-fold>
    }
    Ok(())
}
unsafe fn create_buffer(
    base: &VkBase,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
    buffer: &mut Buffer,
    buffer_memory: &mut DeviceMemory)
{ unsafe {
    let buffer_info = vk::BufferCreateInfo {
        s_type: vk::StructureType::BUFFER_CREATE_INFO,
        size: size,
        usage: usage,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
    };
    *buffer = base.device.create_buffer(&buffer_info, None).expect("failed to create buffer");

    let memory_requirements = base.device.get_buffer_memory_requirements(*buffer);
    let memory_indices = find_memorytype_index(
        &memory_requirements,
        &base.device_memory_properties,
        properties,
    ).expect("failed to find suitable memory type for buffer");
    let allocation_info = vk::MemoryAllocateInfo {
        allocation_size: memory_requirements.size,
        memory_type_index: memory_indices,
        ..Default::default()
    };

    *buffer_memory = base.device.allocate_memory(&allocation_info, None).expect("failed to allocate buffer memory");

    base.device
        .bind_buffer_memory(*buffer, *buffer_memory, 0)
        .expect("failed to bind buffer memory");
}
}

unsafe fn copy_data_to_memory<T: Copy>(ptr: *mut c_void, data: &[T]) { unsafe {
    let mut aligned = Align::new(
        ptr,
        align_of::<T>() as u64,
        (data.len() * size_of::<T>()) as u64,
    );
    aligned.copy_from_slice(&data);
}}
fn compile_shaders(shader_directories: Vec<&str>) -> io::Result<()> {
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
                        let spv_modified = spv_file.metadata()?.modified()?;

                        if glsl_modified > spv_modified || !spv_file.exists() {
                            println!("RECOMPILING:\n{}", spv_file.display());
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

fn load_file(path: &str) -> io::Result<Vec<u8>> {
    let path_final = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src\\shaders\\spv").join(path);
    fs::read(path_final)
}