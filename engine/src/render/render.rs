use ash::vk;
use ash::vk::{DescriptorType, Format, ImageLayout, Sampler, ShaderStageFlags};
use std::cell::RefCell;
use std::slice;
use std::sync::Arc;
use crate::client::client::Client;
use crate::gui::gui::{Element, GUI};
use crate::math::Vector;
use crate::render::render_helper::{Descriptor, DescriptorCreateInfo, DescriptorSetCreateInfo, PassCreateInfo, PipelineCreateInfo, Renderpass, RenderpassCreateInfo, Texture, TextureCreateInfo, Transition};
use crate::render::scene_renderer::SceneRenderer;
use crate::render::vulkan_base::{compile_shaders, VkBase};
use crate::scene::scene::Scene;
use crate::scene::world::world::World;

pub const MAX_FRAMES_IN_FLIGHT: usize = 3;

pub struct Renderer {
    pub device: ash::Device,
    pub draw_command_buffers: Vec<vk::CommandBuffer>,

    pub present_renderpass: Renderpass,

    null_tex_info: vk::DescriptorImageInfo,

    pub scene_renderer: Arc<RefCell<SceneRenderer>>,

    pub present_sampler: Sampler,
}
impl Renderer {
    pub fn new(base: &VkBase, world: Arc<RefCell<World>>) -> Renderer { unsafe {
        Renderer::compile_shaders();

        let (
            scene_renderer,
            present_renderpass,
        ) = Renderer::create_rendering_objects(base, &world.borrow(), vk::Viewport {
             width: base.surface_resolution.width as f32,
             height: base.surface_resolution.height as f32,
             x: 0.0,
             y: 0.0,
             min_depth: 0.0,
             max_depth: 1.0
         });

        let null_tex_info = vk::DescriptorImageInfo {
            sampler: scene_renderer.borrow().sampler.clone(),
            image_view: scene_renderer.borrow().null_texture.image_view.clone(),
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        let mut renderer = Renderer {
            device: base.device.clone(),
            draw_command_buffers: base.draw_command_buffers.clone(),

            present_renderpass,

            null_tex_info,

            scene_renderer,

            present_sampler: base.device.create_sampler(&vk::SamplerCreateInfo {
                mag_filter: vk::Filter::LINEAR,
                min_filter: vk::Filter::LINEAR,
                address_mode_u: vk::SamplerAddressMode::CLAMP_TO_BORDER,
                address_mode_v: vk::SamplerAddressMode::CLAMP_TO_BORDER,
                address_mode_w: vk::SamplerAddressMode::CLAMP_TO_BORDER,
                border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
                ..Default::default()
            }, None).unwrap()
        };

        renderer
    } }
    fn create_rendering_objects(
        base: &VkBase, world: &World, scene_viewport: vk::Viewport
    ) -> (
        Arc<RefCell<SceneRenderer>>,
        Renderpass,
    ) {
        let scene_renderer = SceneRenderer::new(&base.context, 0, world, scene_viewport);

        let present_renderpass = Renderpass::new_present_renderpass(base);

        (
            Arc::new(RefCell::new(scene_renderer)),
            present_renderpass,
        )
    }
    pub fn reload(&mut self, base: &VkBase, world: &World, gui: &mut GUI) { unsafe {
        self.device.device_wait_idle().unwrap();

        {
            let scene_renderer = &mut self.scene_renderer.borrow_mut();
            scene_renderer.destroy();
            self.present_renderpass.destroy();
        }
        (self.scene_renderer, self.present_renderpass) = Renderer::create_rendering_objects(
            base,
            world,
            self.scene_renderer.borrow().viewport.borrow().clone()
        );
        let scene_renderer = &mut self.scene_renderer.borrow_mut();

        self.null_tex_info = vk::DescriptorImageInfo {
            sampler: scene_renderer.sampler.clone(),
            image_view: scene_renderer.null_texture.image_view.clone(),
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };

        // TODO replace this
        let viewport_node_index = gui.nodes[gui.nodes[gui.gui_root_sets[0][0]].children_indices[2]].element_indices[0];
        gui.elements[viewport_node_index] = Element::Texture {
            texture_set: scene_renderer.sky_renderpass.pass.borrow().get_texture_set(0),
            index: gui.image_count,
            additive_tint: Vector::empty(),
            multiplicative_tint: Vector::fill(1.0),
            corner_radius: 0.0,
            aspect_ratio: None,
        };

        gui.reload_rendering(scene_renderer.null_tex_sampler, scene_renderer.null_texture.image_view);

        self.set_present_textures(gui.pass.borrow().textures[0].iter().map(|t| t).collect());

        scene_renderer.update_world_textures_all_frames(&world);
    } }

    pub fn compile_shaders() {
        #[cfg(debug_assertions)] {
            compile_shaders("engine\\resources\\shaders\\glsl").expect("Failed to compile shaders");
        }
    }

    pub fn set_present_textures(&self, texture_set: Vec<&Texture>) { unsafe {
        for current_frame in 0..MAX_FRAMES_IN_FLIGHT {
            let present_info = [vk::DescriptorImageInfo {
                sampler: self.present_sampler,
                image_view: texture_set[current_frame].device_texture.borrow().image_view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            }];
            let present_descriptor_writes: Vec<vk::WriteDescriptorSet> = vec![
                vk::WriteDescriptorSet::default()
                    .dst_set(self.present_renderpass.descriptor_set.borrow().descriptor_sets[current_frame])
                    .dst_binding(0)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&present_info)];
            self.device.update_descriptor_sets(&present_descriptor_writes, &[]);
        }
    } }

    pub fn render_frame(
        &mut self,
        current_frame: usize,
        scene: Arc<RefCell<Scene>>,
        gui: Arc<RefCell<GUI>>,
    ) { unsafe {
        let frame_command_buffer = self.draw_command_buffers[current_frame];

        let scene = &scene.borrow();
        //println!("camera data {:?}", camera);

        self.scene_renderer.borrow().render_world(current_frame, &scene);

        gui.borrow_mut().draw(current_frame, frame_command_buffer);

        /*
        self.hitbox_renderpass.begin_renderpass(current_frame, frame_command_buffer, Some(present_index));
        if render_hitboxes {
            for rigid_body in physics_engine.rigid_bodies.iter() {
                match &rigid_body.hitbox {
                    OBB(a, _) => {
                        for (i, j) in EDGES.iter() {
                            let corner_a = CORNERS[*i] * a.half_extents;
                            let corner_b = CORNERS[*j] * a.half_extents;

                            let constants = LinePushConstantSendable {
                                view_proj: (&camera.projection_matrix * &camera.view_matrix).data,
                                a: ((a.center + corner_a).rotate_by_quat(&rigid_body.orientation) + rigid_body.position).to_array4(),
                                b: ((a.center + corner_b).rotate_by_quat(&rigid_body.orientation) + rigid_body.position).to_array4(),
                                color: Vector::new4(0.0, 0.0, 1.0, 1.0).to_array4()
                            };

                            self.device.cmd_push_constants(frame_command_buffer, self.hitbox_renderpass.pipeline_layout, ShaderStageFlags::ALL_GRAPHICS, 0, slice::from_raw_parts(
                                &constants as *const LinePushConstantSendable as *const u8,
                                size_of::<LinePushConstantSendable>(),
                            ));
                            self.device.cmd_draw(frame_command_buffer, 2, 1, 0, 0);
                        }
                    },
                    Hitbox::ConvexHull(a) => {
                        for tri in &a.triangle_vert_indices {
                            let edges = [
                                (tri.0, tri.1),
                                (tri.1, tri.2),
                                (tri.2, tri.0),
                            ];

                            for (i, j) in edges.iter() {
                                let corner_a = a.points[*i];
                                let corner_b = a.points[*j];

                                let constants = LinePushConstantSendable {
                                    view_proj: (&camera.projection_matrix * &camera.view_matrix).data,
                                    a: (corner_a.rotate_by_quat(&rigid_body.orientation) + rigid_body.position).to_array4(),
                                    b: (corner_b.rotate_by_quat(&rigid_body.orientation) + rigid_body.position).to_array4(),
                                    color: Vector::new4(0.0, 1.0, 0.0, 1.0).to_array4()
                                };

                                self.device.cmd_push_constants(frame_command_buffer, self.hitbox_renderpass.pipeline_layout, ShaderStageFlags::ALL_GRAPHICS, 0, slice::from_raw_parts(
                                    &constants as *const LinePushConstantSendable as *const u8,
                                    size_of::<LinePushConstantSendable>(),
                                ));
                                self.device.cmd_draw(frame_command_buffer, 2, 1, 0, 0);
                            }
                        }
                    }
                    _ => { continue }
                };
            }
        }
        self.device.cmd_end_render_pass(frame_command_buffer);
        { self.hitbox_renderpass.pass.borrow().transition_to_readable(frame_command_buffer, current_frame); }

         */

        /*
        self.outline_renderpass.do_renderpass(
            current_frame,
            frame_command_buffer,
            Some(|| {
                self.device.cmd_push_constants(
                    frame_command_buffer,
                    self.outline_renderpass.pipeline_layout,
                    ShaderStageFlags::FRAGMENT, 0,
                    slice::from_raw_parts(
                        &OutlineConstantSendable {
                            color: [0.93, 0.72, 0.0, 1.0],
                            thickness: 5.0,
                            _pad: [0.0, 0.0, 0.0],
                        } as *const OutlineConstantSendable as *const u8,
                        size_of::<OutlineConstantSendable>(),
                    )
                )
            }),
            None::<fn()>,
            None,
            true,
        );
         */

        self.present_renderpass.pass.borrow().transition(
            frame_command_buffer,
            current_frame,
            Some((ImageLayout::UNDEFINED, ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::AccessFlags::empty())),
            None,
        );
        self.present_renderpass.begin_renderpass(current_frame, frame_command_buffer);
        self.device.cmd_draw(frame_command_buffer, 6, 1, 0, 0);
        self.device.cmd_end_rendering(frame_command_buffer);
        self.present_renderpass.pass.borrow().transition(
            frame_command_buffer,
            current_frame,
            Some((ImageLayout::COLOR_ATTACHMENT_OPTIMAL, ImageLayout::PRESENT_SRC_KHR, vk::AccessFlags::COLOR_ATTACHMENT_WRITE)),
            None,
        );
    } }

    pub fn destroy(&mut self) { unsafe {
        self.scene_renderer.borrow_mut().destroy();
        self.present_renderpass.destroy();
        self.device.destroy_sampler(self.present_sampler, None);
    } }
}

/*
pub fn screenshot_texture(texture: &Texture, layout: vk::ImageLayout, path: &str) {
    unsafe { texture.context.device.device_wait_idle().unwrap(); }

    let bytes_per_pixel = match texture.format {
        Format::R8G8B8A8_SRGB | Format::R8G8B8A8_UNORM |
        Format::B8G8R8A8_SRGB | Format::B8G8R8A8_UNORM => 4,
        Format::R16G16B16A16_SFLOAT | Format::R16G16B16A16_UNORM => 8,
        Format::R32G32B32A32_SFLOAT => 16,
        _ => panic!("Unsupported format for screenshot"),
    };

    let buffer_size = (texture.resolution.width * texture.resolution.height * bytes_per_pixel) as DeviceSize;

    let buffer_info = vk::BufferCreateInfo::default()
        .size(buffer_size)
        .usage(vk::BufferUsageFlags::TRANSFER_DST)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let staging_buffer = base.device.create_buffer(&buffer_info, None).expect("Failed to create buffer");
    let mem_req = base.device.get_buffer_memory_requirements(staging_buffer);
    let mem_type_index = find_memorytype_index(
        &mem_req,
        &base.device_memory_properties,
        MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
    ).unwrap();

    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_req.size)
        .memory_type_index(mem_type_index);

    let staging_mem = base.device.allocate_memory(&alloc_info, None).expect("Failed to allocate memory");
    base.device.bind_buffer_memory(staging_buffer, staging_mem, 0).unwrap();

    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(base.pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);

    let cmd_buffer = base.device.allocate_command_buffers(&alloc_info).unwrap()[0];

    let begin_info = vk::CommandBufferBeginInfo::default()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    base.device.begin_command_buffer(cmd_buffer, &begin_info).unwrap();

    let barrier = vk::ImageMemoryBarrier::default()
        .old_layout(layout)
        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(texture.device_texture.borrow().image)
        .subresource_range(ImageSubresourceRange {
            aspect_mask: if texture.is_depth { ImageAspectFlags::DEPTH } else { ImageAspectFlags::COLOR },
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: texture.array_layers,
        })
        .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .dst_access_mask(vk::AccessFlags::TRANSFER_READ);

    unsafe { base.device.cmd_pipeline_barrier(
        cmd_buffer,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &[barrier],
    ); }

    let copy = vk::BufferImageCopy::default()
        .buffer_offset(0)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_subresource(vk::ImageSubresourceLayers {
            aspect_mask: if texture.is_depth { ImageAspectFlags::DEPTH } else { ImageAspectFlags::COLOR },
            mip_level: 0,
            base_array_layer: 0,
            layer_count: texture.array_layers,
        })
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(texture.resolution);

    unsafe {base.device.cmd_copy_image_to_buffer(cmd_buffer, texture.device_texture.borrow().image, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, staging_buffer, &[copy]); }

    let barrier_back = vk::ImageMemoryBarrier::default()
        .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .new_layout(layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(texture.device_texture.borrow().image)
        .subresource_range(ImageSubresourceRange {
            aspect_mask: if texture.is_depth { ImageAspectFlags::DEPTH } else { ImageAspectFlags::COLOR },
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: texture.array_layers,
        })
        .src_access_mask(vk::AccessFlags::TRANSFER_READ)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

    unsafe {
        base.device.cmd_pipeline_barrier(
            cmd_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier_back],
        );

        base.device.end_command_buffer(cmd_buffer.clone()).unwrap();

        let command_buffers = &[cmd_buffer];
        let submit_info = vk::SubmitInfo::default()
            .command_buffers(command_buffers);

        base.device.queue_submit(base.graphics_queue, &[submit_info], vk::Fence::null()).unwrap();
        base.device.queue_wait_idle(base.graphics_queue).unwrap();

        base.device.free_command_buffers(base.pool, &[cmd_buffer]);

        let data_ptr = base.device.map_memory(staging_mem, 0, buffer_size, vk::MemoryMapFlags::empty()).unwrap();
        let data_slice = slice::from_raw_parts(data_ptr as *const u8, buffer_size as usize);

        let mut rgba_data = Vec::with_capacity((texture.resolution.width * texture.resolution.height * 4) as usize);

        match texture.format {
            Format::R8G8B8A8_UNORM | Format::R8G8B8A8_SRGB => {
                rgba_data.extend_from_slice(data_slice);
            }
            Format::B8G8R8A8_UNORM | Format::B8G8R8A8_SRGB => {
                for chunk in data_slice.chunks_exact(4) {
                    rgba_data.push(chunk[2]); // R
                    rgba_data.push(chunk[1]); // G
                    rgba_data.push(chunk[0]); // B
                    rgba_data.push(chunk[3]); // A
                }
            }
            Format::R16G16B16A16_UNORM => {
                for chunk in data_slice.chunks_exact(8) {
                    for i in 0..4 {
                        let value = u16::from_le_bytes([chunk[i*2], chunk[i*2+1]]);
                        rgba_data.push((value >> 8) as u8);
                    }
                }
            }
            Format::R16G16B16A16_SFLOAT => {
                for chunk in data_slice.chunks_exact(8) {
                    for i in 0..4 {
                        let bits = u16::from_le_bytes([chunk[i*2], chunk[i*2+1]]);
                        let f = half::f16::from_bits(bits).to_f32();
                        rgba_data.push((f.clamp(0.0, 1.0) * 255.0) as u8);
                    }
                }
            }
            _ => panic!("Unsupported format for screenshot"),
        }
        if texture.format == Format::B8G8R8A8_UNORM || texture.format == Format::B8G8R8A8_SRGB {
            for chunk in rgba_data.chunks_exact_mut(4) {
                chunk.swap(0, 2);
            }
        }

        base.device.unmap_memory(staging_mem);
        base.device.destroy_buffer(staging_buffer, None);
        base.device.free_memory(staging_mem, None);

        let file = File::create(path).unwrap();
        let w = BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, texture.resolution.width, texture.resolution.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&rgba_data).unwrap();
        writer.finish().unwrap();
    }

    println!("Screenshot saved to {}", path);
}
 */