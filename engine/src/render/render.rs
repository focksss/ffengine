use ash::vk;
use ash::vk::{DescriptorType, DeviceSize, Format, ImageAspectFlags, ImageSubresourceRange, MemoryPropertyFlags, PipelineInputAssemblyStateCreateInfo, PipelineRasterizationStateCreateInfo, Sampler, ShaderStageFlags};
use std::cell::RefCell;
use std::fs::File;
use std::io::BufWriter;
use std::{iter, slice};
use std::sync::Arc;
use crate::client::client::Client;
use crate::gui::gui::{Element, GUI};
use crate::math::Vector;
use crate::scene::physics::hitboxes::hitbox::Hitbox;
use crate::scene::physics::hitboxes::hitbox::Hitbox::OBB;
use crate::scene::physics::hitboxes::hitbox::Hitbox::Sphere;
use crate::scene::physics::hitboxes::mesh::Bvh;
use crate::scene::physics::physics_engine::PhysicsEngine;
use crate::scene::physics::player::Player;
use crate::render::render_helper::{Descriptor, DescriptorCreateInfo, DescriptorSetCreateInfo, PassCreateInfo, PipelineCreateInfo, Renderpass, RenderpassCreateInfo, Texture, TextureCreateInfo};
use crate::render::scene_renderer::SceneRenderer;
use crate::render::vulkan_base::{compile_shaders, copy_data_to_memory, find_memorytype_index, VkBase};
use crate::scene::scene::Scene;
use crate::scene::world::camera::{Camera, CameraPointer};
use crate::scene::world::world::World;

pub const MAX_FRAMES_IN_FLIGHT: usize = 3;

pub struct Renderer {
    pub device: ash::Device,
    pub draw_command_buffers: Vec<vk::CommandBuffer>,

    pub camera: CameraPointer,

    pub present_renderpass: Renderpass,
    pub compositing_renderpass: Renderpass,
    compositing_renderpass_layers: usize,

    null_tex_info: vk::DescriptorImageInfo,

    pub scene_renderer: Arc<RefCell<SceneRenderer>>,
    pub guis: Vec<Arc<RefCell<GUI>>>,

    pub present_sampler: Sampler,
}
impl Renderer {
    pub unsafe fn new(base: &VkBase, controller: Arc<RefCell<Client>>, primary_camera: CameraPointer, num_guis: usize) -> Renderer { unsafe {
        Renderer::compile_shaders();
        let world_ref = primary_camera.world.clone();
        let world = world_ref.borrow();

        let (
            scene_renderer,
            compositing_renderpass,
            present_renderpass,
        ) = Renderer::create_rendering_objects(base, &world, vk::Viewport {
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
        let mut guis = Vec::new();
        for i in 0..num_guis {
            guis.push(
                Arc::new(RefCell::new(GUI::new(i,
                    &base.context,
                    controller.clone(),
                    scene_renderer.borrow().null_tex_sampler.clone(),
                    scene_renderer.borrow().null_texture.image_view.clone())
                ))
            );
        }
        let mut renderer = Renderer {
            device: base.device.clone(),
            draw_command_buffers: base.draw_command_buffers.clone(),

            camera: primary_camera,

            present_renderpass,
            compositing_renderpass,
            compositing_renderpass_layers: 3,

            null_tex_info,

            guis,
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
        {
            renderer.set_present_textures(renderer.compositing_renderpass.pass.borrow().textures.iter().map(|frame_textures| {
                &frame_textures[0]
            }).collect::<Vec<&Texture>>());

            let scene_renderer_ref = renderer.scene_renderer.borrow();
            let scene_pass = scene_renderer_ref.forward_renderpass.pass.borrow();
            let scene_textures = scene_pass.textures
                .iter()
                .map(|f| f[0].clone())
                .collect::<Vec<Texture>>();

            let gui_textures = renderer.guis.iter()
                .map(|gui| {
                    let gui_ref = gui.borrow();
                    let gui_pass = gui_ref.pass.borrow();
                    gui_pass.textures
                        .iter()
                        .map(|f| f[0].clone())
                        .collect::<Vec<Texture>>()
                })
                .collect::<Vec<Vec<Texture>>>();

            let mut compositing_textures = vec![scene_textures];
            compositing_textures.extend(gui_textures);

            renderer.set_compositing_textures(compositing_textures);
        }
        renderer.set_compositing_layers(4);
        renderer.scene_renderer.borrow_mut().update_world_textures_all_frames(&world);

        renderer
    } }
    unsafe fn create_rendering_objects(
        base: &VkBase, world: &World, scene_viewport: vk::Viewport
    ) -> (
        Arc<RefCell<SceneRenderer>>,
        Renderpass,
        Renderpass,
    ) { unsafe {
        let scene_renderer = SceneRenderer::new(&base.context, world, scene_viewport);

        let context = &base.context;

        let texture_sampler_create_info = DescriptorCreateInfo::new(context)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT);

        let outline_pass_create_info = PassCreateInfo::new(context)
            .add_color_attachment_info(TextureCreateInfo::new(context).format(Format::R8G8B8A8_UNORM).add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC));
        let outline_descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info));
        let outline_renderpass_create_info = RenderpassCreateInfo::new(context)
            .pass_create_info(outline_pass_create_info)
            .descriptor_set_create_info(outline_descriptor_set_create_info)
            .add_pipeline_create_info(PipelineCreateInfo::new()
                .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
                .fragment_shader_uri(String::from("outline\\outline.frag.spv")))
            .viewport(scene_viewport)
            .add_push_constant_range(vk::PushConstantRange {
                stage_flags: ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: size_of::<OutlineConstantSendable>() as u32,
            });
        let outline_renderpass = Renderpass::new(outline_renderpass_create_info);

        let present_renderpass = Renderpass::new_present_renderpass(base);

        let composite_layers_descriptor_create_info = DescriptorCreateInfo::new(context)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT)
            .dynamic(true)
            .image_infos(vec![vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image_view: scene_renderer.null_texture.image_view,
                sampler: scene_renderer.null_tex_sampler,
                ..Default::default()
            }; 5]);
        let compositing_pass_create_info = PassCreateInfo::new(context)
            .add_color_attachment_info(TextureCreateInfo::new(context).format(Format::R16G16B16A16_SFLOAT).add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC));
        let compositing_descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
            .add_descriptor(Descriptor::new(&composite_layers_descriptor_create_info));
        let compositing_renderpass_create_info = { RenderpassCreateInfo::new(context)
            .pass_create_info(compositing_pass_create_info)
            .descriptor_set_create_info(compositing_descriptor_set_create_info)
            .add_pipeline_create_info(PipelineCreateInfo::new()
                .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
                .fragment_shader_uri(String::from("composite.frag.spv")))
            .add_push_constant_range(vk::PushConstantRange {
                stage_flags: ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: size_of::<usize>() as _,
            })
        };

        for frame in 0..MAX_FRAMES_IN_FLIGHT {
            let image_infos = [
                vk::DescriptorImageInfo {
                    sampler: scene_renderer.nearest_sampler,
                    image_view: scene_renderer.geometry_renderpass.pass.borrow().textures[frame][5].device_texture.borrow().stencil_image_view.unwrap(),
                    image_layout: vk::ImageLayout::STENCIL_READ_ONLY_OPTIMAL,
                } // id buffer
            ];
            let outline_descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(outline_renderpass.descriptor_set.borrow().descriptor_sets[frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            base.device.update_descriptor_sets(&outline_descriptor_writes, &[]);
        }

        (
            Arc::new(RefCell::new(scene_renderer)),
            Renderpass::new(compositing_renderpass_create_info),
            present_renderpass,
        )
    } }
    pub unsafe fn reload(&mut self, base: &VkBase, world: &World) { unsafe {
        self.device.device_wait_idle().unwrap();

        {
            let scene_renderer = &mut self.scene_renderer.borrow_mut();
            scene_renderer.destroy();
            self.compositing_renderpass.destroy();
            self.present_renderpass.destroy();
        }
        (self.scene_renderer, self.compositing_renderpass, self.present_renderpass) = Renderer::create_rendering_objects(
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
        {

            let mut gui = self.guis[0].borrow_mut();
            let viewport_node_index = gui.nodes[gui.nodes[gui.root_node_indices[0]].children_indices[2]].element_indices[0];
            gui.elements[viewport_node_index] = Element::Texture {
                texture_set: scene_renderer.lighting_renderpass.pass.borrow().get_texture_set(0),
                index: gui.image_count,
                additive_tint: Vector::empty(),
                multiplicative_tint: Vector::fill(1.0),
                corner_radius: 0.0,
                aspect_ratio: None,
            };
        }

        {
            for gui in self.guis.iter() {
                gui.borrow_mut().reload_rendering(scene_renderer.null_tex_sampler, scene_renderer.null_texture.image_view)
            };
        }

        {
            self.set_present_textures(self.compositing_renderpass.pass.borrow().textures.iter().map(|frame_textures| {
                &frame_textures[0]
            }).collect::<Vec<&Texture>>());

            let gui_textures = self.guis.iter()
                .map(|gui| {
                    let gui_ref = gui.borrow();
                    let gui_pass = gui_ref.pass.borrow();
                    gui_pass.textures
                        .iter()
                        .map(|f| f[0].clone())
                        .collect::<Vec<Texture>>()
                })
                .collect::<Vec<Vec<Texture>>>();

            self.set_compositing_textures(gui_textures);
        }
        scene_renderer.update_world_textures_all_frames(&world);
    } }

    pub unsafe fn compile_shaders() {
        #[cfg(debug_assertions)] {
            compile_shaders("engine\\resources\\shaders\\glsl").expect("Failed to compile shaders");
        }
    }

    pub unsafe fn set_present_textures(&self, texture_set: Vec<&Texture>) { unsafe {
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
    pub fn set_compositing_layers(&mut self, layers: usize) { self.compositing_renderpass_layers = layers }
    pub unsafe fn set_compositing_textures(&self, texture_sets: Vec<Vec<Texture>>) { unsafe {
        for current_frame in 0..MAX_FRAMES_IN_FLIGHT {
            let mut image_infos = texture_sets.iter().map(|texture_set| {
                vk::DescriptorImageInfo {
                    sampler: self.present_sampler,
                    image_view: texture_set[current_frame].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }
            }).collect::<Vec<vk::DescriptorImageInfo>>();
            for _ in 0..(5 - image_infos.len()) {
                image_infos.push(self.null_tex_info.clone())
            }

            let image_infos = image_infos.as_slice().as_ptr();
            for frame in 0..MAX_FRAMES_IN_FLIGHT {
                let descriptor_write = vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: self.compositing_renderpass.descriptor_set.borrow().descriptor_sets[frame],
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 5,
                    p_image_info: image_infos,
                    ..Default::default()
                };
                self.device.update_descriptor_sets(&[descriptor_write], &[]);
            }
        }
    } }

    pub unsafe fn render_frame(
        &mut self,
        current_frame: usize,
        present_index: usize,
        scene: Arc<RefCell<Scene>>,
        render_hitboxes: bool,
        physics_engine: &PhysicsEngine
    ) { unsafe {
        let frame_command_buffer = self.draw_command_buffers[current_frame];

        let mut scene = &mut scene.borrow_mut();
        scene.world.borrow_mut().update_sun(self.camera.index);
        //println!("camera data {:?}", camera);

        self.scene_renderer.borrow().render_world(current_frame, &scene, self.camera.index);

        for gui in self.guis.iter() {
            gui.borrow_mut().draw(current_frame, frame_command_buffer);
        }

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

        self.compositing_renderpass.do_renderpass(
            current_frame,
            frame_command_buffer,
            Some(|| {
                self.device.cmd_push_constants(frame_command_buffer, self.compositing_renderpass.pipeline_layout, ShaderStageFlags::FRAGMENT, 0, slice::from_raw_parts(
                    &self.compositing_renderpass_layers as *const usize as *const u8,
                    size_of::<usize>(),
                ));
            }),
            None::<fn()>,
            None,
            true
        );
        self.present_renderpass.begin_renderpass(current_frame, frame_command_buffer, Some(present_index));
        self.device.cmd_draw(frame_command_buffer, 6, 1, 0, 0);
        self.device.cmd_end_render_pass(frame_command_buffer);
    } }

    pub unsafe fn destroy(&mut self) { unsafe {
        for gui in self.guis.iter() {
            gui.borrow_mut().destroy();
        }
        self.scene_renderer.borrow_mut().destroy();
        self.compositing_renderpass.destroy();
        self.present_renderpass.destroy();
        self.device.destroy_sampler(self.present_sampler, None);
    } }
}

/*
pub unsafe fn screenshot_texture(texture: &Texture, layout: vk::ImageLayout, path: &str) {
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

pub struct HitboxPushConstantSendable {
    view_proj: [f32; 16],
    center: [f32; 4],
    half_extent: [f32; 4],
    quat: [f32; 4],
    color: [f32; 4],
}
pub struct LinePushConstantSendable {
    view_proj: [f32; 16],
    a: [f32; 4],
    b: [f32; 4],
    color: [f32; 4],
}
pub struct OutlineConstantSendable {
    color: [f32; 4],

    thickness: f32,
    _pad: [f32; 3],
}
//TODO REMOVE THIS GRAPHICS CRIME AND REPLACE WITH VERTEX BUFFERS FOR HITBOXES
// LOOKING AT THIS MAKES ME WANT TO DIE AND I WROTE IT
const CORNERS: [Vector; 8] = [
Vector { x: 1.0, y: 1.0, z: 1.0, w: 1.0 },
Vector { x: 1.0, y: 1.0, z: -1.0, w: 1.0 },
Vector { x: 1.0, y: -1.0, z: 1.0, w: 1.0 },
Vector { x: 1.0, y: -1.0, z: -1.0, w: 1.0 },
Vector { x: -1.0, y: 1.0, z: 1.0, w: 1.0 },
Vector { x: -1.0, y: 1.0, z: -1.0, w: 1.0 },
Vector { x: -1.0, y: -1.0, z: 1.0, w: 1.0 },
Vector { x: -1.0, y: -1.0, z: -1.0, w: 1.0 },
];
const EDGES: [(usize, usize); 12] = [
(0, 1), (0, 2), (1, 3), (2, 3),
(4, 5), (4, 6), (5, 7), (6, 7),
(0, 4), (1, 5), (2, 6), (3, 7),
];