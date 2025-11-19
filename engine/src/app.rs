#![warn(unused_qualifications)]
use std::cell::RefCell;
use std::default::Default;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use ash::vk;
use ash::vk::QueryPool;
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use crate::math::Vector;
use crate::client::controller::Controller;
use crate::physics::physics_engine::PhysicsEngine;
use crate::render::render::{Renderer, MAX_FRAMES_IN_FLIGHT};
use crate::render::vulkan_base::{record_submit_commandbuffer, VkBase};
use crate::scripting::lua_engine::Lua;
use crate::world::scene::{Light, Model, Scene};

const PI: f32 = std::f32::consts::PI;

pub struct Engine {
    pub base: VkBase,
    pub world: Scene,
    pub renderer: Renderer,
    pub physics_engine: PhysicsEngine,
    pub controller: Arc<RefCell<Controller>>,
}
impl Engine {
    pub unsafe fn new() -> Engine {
        Lua::initialize().expect("Failed to initialize Lua scripting engine");
        let base = VkBase::new("ffengine".to_string(), 1920, 1080, MAX_FRAMES_IN_FLIGHT).unwrap();
        let controller = Arc::new(RefCell::new(Controller::new(&base.window, Vector::new_vec3(0.0, 20.0, 0.0))));
        let mut world = Scene::new(&base);

        unsafe { world.initialize(&base) }

        Engine {
            physics_engine: PhysicsEngine::new(Vector::new_vec3(0.0, -9.8, 0.0), 0.9, 0.5),
            renderer: unsafe { Renderer::new(&base, &world, controller.clone()) },
            world,
            controller,
            base,
        }
    }

    pub unsafe fn run(&mut self) { unsafe {
        let base = &mut self.base;
        let controller = &self.controller;
        let mut world = &mut self.world;
        let mut renderer = &mut self.renderer;
        let physics_engine = &mut self.physics_engine;

        // renderer.scene_renderer.update_world_textures_all_frames(base, &world);

        // renderer.reload(&base, &world);

        let mut current_frame = 0usize;
        let mut last_frame_time = Instant::now();
        let mut needs_resize = false;

        let mut frametime_manager = FrametimeManager::new(&base);

        let mut last_resize = Instant::now();
        let event_loop_ptr = base.event_loop.as_ptr();
        let mut first_frame = true;
        (*event_loop_ptr).run_on_demand(|event, elwp| {
            elwp.set_control_flow(ControlFlow::Poll);
            match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized( _ ),
                    ..
                } => { if first_frame { return }
                    base.needs_swapchain_recreate = true;
                    last_resize = Instant::now();

                    controller.borrow_mut().player.borrow_mut().camera.aspect_ratio = base.window.inner_size().width as f32 / base.window.inner_size().height as f32;
                    needs_resize = true;
                },
                Event::AboutToWait => {
                    first_frame = false;
                    if base.needs_swapchain_recreate {
                        base.device.device_wait_idle().unwrap();
                        base.set_surface_and_present_images();
                        renderer.reload(&base, &world);

                        base.needs_swapchain_recreate = false;
                        frametime_manager.reset();
                        return;
                    }

                    let now = Instant::now();
                    let delta_time = now.duration_since(last_frame_time).as_secs_f32();
                    last_frame_time = now;

                    { // kill refs once done
                        { let mut controller_mut = controller.borrow_mut();
                            controller_mut.do_controls(delta_time, &base, &mut renderer, &world, current_frame) };

                        if controller.borrow().flags.do_physics { physics_engine.tick(delta_time, &mut world); }


                        { let mut controller_mut = controller.borrow_mut();
                            controller_mut.update_camera(); }
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

                    let current_rendering_complete_semaphore = base.rendering_complete_semaphores[current_frame];
                    let current_draw_command_buffer = base.draw_command_buffers[current_frame];
                    let current_fence = base.draw_commands_reuse_fences[current_frame];

                    { if !controller.borrow().paused { world.update_sun(&controller.borrow().player.borrow().camera) }; };

                    record_submit_commandbuffer(
                        &base.device,
                        current_draw_command_buffer,
                        current_fence,
                        base.present_queue,
                        &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
                        &[base.present_complete_semaphores[current_frame]],
                        &[current_rendering_complete_semaphore],
                        |device, frame_command_buffer| {
                            world.update_nodes(frame_command_buffer, current_frame);
                            world.update_lights(frame_command_buffer, current_frame);

                            let player =  { controller.borrow().player.clone() };

                            let flags = controller.borrow().flags.clone();
                            renderer.render_frame(current_frame, present_index as usize, &world, player, flags.draw_hitboxes, &physics_engine);
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

                    frametime_manager.reset();

                    current_frame = (current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
                },
                _ => { controller.borrow_mut().handle_event(event, elwp) },
            }
        }).expect("Failed to initiate render loop");

        base.device.device_wait_idle().unwrap();

        renderer.destroy();
        world.destroy(&base);
        frametime_manager.destroy(&base);
    } }
}

pub struct FrametimeManager {
    cpu_actions: Vec<(Instant, Duration, String)>, // Start time, duration, name
    gpu_action_timestamp_pairs: Vec<String>,
    current_cpu_action_index: usize,
    recording_cpu: bool,

    query_pool: QueryPool,
}
impl FrametimeManager {
    pub fn new(base: &VkBase) -> FrametimeManager {
        let query_pool_info = vk::QueryPoolCreateInfo::default()
            .query_type(vk::QueryType::TIMESTAMP)
            .query_count(2 * MAX_FRAMES_IN_FLIGHT as u32);
        let query_pool = unsafe {
            base.device.create_query_pool(&query_pool_info, None).unwrap()
        };
        FrametimeManager {
            cpu_actions: Vec::new(),
            gpu_action_timestamp_pairs: vec![String::new(); 32],
            current_cpu_action_index: 0,
            recording_cpu: false,
            query_pool,
        }
    }
    pub fn reset(&mut self) {
        self.cpu_actions.clear();
        self.current_cpu_action_index = 0;
    }

    pub fn record_cpu_action_start(&mut self, name: String) {
        if self.recording_cpu {
            eprintln!("FrametimeManager action recording cpu started when already recording, from {}", name);
            return
        }
        self.cpu_actions.push((Instant::now(), Duration::from_nanos(0), name));
        self.recording_cpu = true;
    }
    pub fn record_cpu_action_end(&mut self) {
        self.recording_cpu = false;
        let cpu_action_len = self.cpu_actions.len();
        let current_action = &mut self.cpu_actions[cpu_action_len - 1] ;
        current_action.1 = current_action.0.elapsed();
    }

    pub fn record_gpu_action_start(&mut self, base: &VkBase, command_buffer: vk::CommandBuffer, action_index: usize, name: String) { unsafe {
        base.device.cmd_reset_query_pool(command_buffer, self.query_pool, action_index as u32 * 2, 2);
        base.device.cmd_write_timestamp(
            command_buffer,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            self.query_pool,
            action_index as u32 * 2,
        );
        self.gpu_action_timestamp_pairs[action_index] = name;
    } }
    pub fn record_gpu_action_end(&mut self, base: &VkBase, command_buffer: vk::CommandBuffer, action_index: usize) { unsafe {
        base.device.cmd_write_timestamp(
            command_buffer,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            self.query_pool,
            (action_index as u32 * 2) + 1,
        );
    } }

    pub fn report(&mut self, base: &VkBase) {
        let current_action = &self.cpu_actions[self.current_cpu_action_index];
        if self.recording_cpu {
            eprintln!("FrametimeManager report called when recording cpu {}", current_action.2);
        }
        println!("FrametimeManager report:");
        println!(" - CPU actions:");
        for action in &self.cpu_actions {
            println!("    - {}, with duration of {}", action.2, action.1.as_micros() as f32 / 1000.0);
        }
        println!(" - GPU actions:");
        let mut timestamps = [0u64; 2];
        for i in 0..self.gpu_action_timestamp_pairs.len() { unsafe {
            let action_name = &self.gpu_action_timestamp_pairs[i];
            if action_name.is_empty() { continue }
            base.device.get_query_pool_results(
                self.query_pool,
                i as u32 * 2,
                &mut timestamps,
                vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
            ).unwrap();
            let timestamp_period = unsafe { base.instance.get_physical_device_properties(base.pdevice).limits.timestamp_period };
            let gpu_time_ns = (timestamps[1] - timestamps[0]) as f64 * timestamp_period as f64;
            let gpu_time_ms = gpu_time_ns / 1_000_000.0;
            println!("    - {} with duration of: {:.3} ms", action_name, gpu_time_ms);
        } }
    }
    pub unsafe fn destroy(&mut self, base: &VkBase) { unsafe {
        base.device.destroy_query_pool(self.query_pool, None);
    } }
}