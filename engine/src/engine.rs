#![warn(unused_qualifications)]
use std::cell::RefCell;
use std::default::Default;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};
use ash::vk;
use ash::vk::QueryPool;
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use crate::math::Vector;
use crate::client::client::Client;
use crate::gui::gui::GUI;
use crate::scene::physics::physics_engine::PhysicsEngine;
use crate::render::render::{Renderer, MAX_FRAMES_IN_FLIGHT};
use crate::render::vulkan_base::{record_submit_commandbuffer, VkBase};
use crate::scene::scene::Scene;
use crate::scripting::lua_engine::Lua;
use crate::scene::world::camera::{Camera, CameraPointer};
use crate::scene::world::world::{Light, World};

const PI: f32 = std::f32::consts::PI;

static COMMAND_BUFFER: OnceLock<RwLock<vk::CommandBuffer>> = OnceLock::new();
pub fn get_command_buffer() -> vk::CommandBuffer {
    *COMMAND_BUFFER
        .get()
        .expect("not initialized")
        .read()
        .unwrap()
}

pub struct Engine {
    pub scene: Arc<RefCell<Scene>>,

    pub base: VkBase,
    pub world: Arc<RefCell<World>>,
    pub renderer: Arc<RefCell<Renderer>>,
    pub physics_engine: Arc<RefCell<PhysicsEngine>>,
    pub client: Arc<RefCell<Client>>,
}
#[derive(Clone)]
pub struct EngineRef {
    pub scene: Arc<RefCell<Scene>>,

    pub world: Arc<RefCell<World>>,
    pub renderer: Arc<RefCell<Renderer>>,
    pub physics_engine: Arc<RefCell<PhysicsEngine>>,
    pub client: Arc<RefCell<Client>>,
}
impl Engine {
    pub unsafe fn new() -> Engine {
        let base = VkBase::new("ffengine".to_string(), 1920, 1080, MAX_FRAMES_IN_FLIGHT).unwrap();
        let mut world = World::new(&base);
        unsafe { world.initialize(&base) }
        let world = Arc::new(RefCell::new(world));
        let physics_engine = Arc::new(RefCell::new(PhysicsEngine::new(Vector::new3(0.0, -9.8, 0.0), 0.9, 0.5)));

        let client = Arc::new(RefCell::new(Client::new(base.window.clone())));

        let rw_lock = RwLock::new(base.draw_command_buffers[0]);
        COMMAND_BUFFER.set(rw_lock).expect("Failed to initialize frame command buffer global");

        let renderer = unsafe { Arc::new(RefCell::new(Renderer::new(&base, client.clone(), CameraPointer {
            world: world.clone(),
            index: 0
        }, 1))) };

        let engine = Engine {
            scene: Arc::new(RefCell::new(Scene::new(renderer.clone(), world.clone(), physics_engine.clone()))),

            physics_engine,
            renderer,
            world,
            client,
            base,
        };
        Lua::initialize(engine.as_ref()).expect("failed to initialize lua");
        engine
    }

    pub fn as_ref(&self) -> EngineRef {
        EngineRef {
            scene: self.scene.clone(),
            world: self.world.clone(),
            renderer: self.renderer.clone(),
            physics_engine: self.physics_engine.clone(),
            client: self.client.clone(),
        }
    }

    pub fn load_script(&self, path: &Path) {
        Lua::load_scripts(vec![path]).expect("failed to load scripts");
    }

    pub unsafe fn run(&mut self) {
        let engine_ref = self.as_ref();
        let base = &mut self.base;

        let mut current_frame = 0usize;
        let mut last_frame_time = Instant::now();
        let mut needs_resize = false;

        let mut last_resize = Instant::now();
        let event_loop_ptr = base.event_loop.as_ptr();
        let mut first_frame = true;
        unsafe {
            (*event_loop_ptr).run_on_demand(|event, elwp| {
                elwp.set_control_flow(ControlFlow::Poll);
                match event {
                    Event::WindowEvent {
                        event: WindowEvent::Resized(_),
                        ..
                    } => {
                        if first_frame { return }
                        base.needs_swapchain_recreate = true;
                        last_resize = Instant::now();

                        needs_resize = true;
                    },
                    Event::AboutToWait => {
                        {
                            let controller_ref = &mut self.client.borrow_mut();
                            let flag_ref = &mut controller_ref.flags.borrow_mut();
                            if flag_ref.close_requested {
                                elwp.exit();
                                return;
                            }

                            first_frame = false;
                            if base.needs_swapchain_recreate {
                                base.device.device_wait_idle().unwrap();
                                base.set_surface_and_present_images();
                                self.renderer.borrow_mut().reload(&base, &self.world.borrow());

                                base.needs_swapchain_recreate = false;
                                // frametime_manager.reset();
                                return;
                            }

                            if flag_ref.reload_rendering_queued {
                                base.device.device_wait_idle().unwrap();
                                Renderer::compile_shaders();
                                base.device.device_wait_idle().unwrap();

                                let renderer = &mut self.renderer.borrow_mut();

                                renderer.reload(base, &self.world.borrow());
                                
                                // renderer.gui.borrow_mut().load_from_file(base, "editor\\resources\\gui\\editor.gui");
                                
                                flag_ref.reload_rendering_queued = false;
                            }

                            if flag_ref.reload_scripts_queued {
                                Lua::reload_scripts();
                                flag_ref.reload_scripts_queued = false;
                            }

                            let lock = COMMAND_BUFFER.get().expect("not initialized");
                            *lock.write().unwrap() = base.draw_command_buffers[current_frame];

                            /*
                                flags.screenshot_queued = false;
                                let timestamp = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();
                                screenshot_texture(
                                    &base,
                                    &renderer.compositing_renderpass.pass.borrow().textures[frame][0],
                                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                                    format!("screenshots\\screenshot_{}.png", timestamp).as_str()
                                );
                            */
                        }

                        let now = Instant::now();
                        let delta_time = now.duration_since(last_frame_time).as_secs_f32();
                        Lua::with_lua(|lua| lua.globals().set("dt", delta_time)).expect("Failed to set lua deltatime global");
                        last_frame_time = now;

                        Lua::run_update_methods().expect("Failed to run Update methods");
                        for gui in self.renderer.borrow_mut().guis.iter() {
                            gui.borrow_mut().initialize_new_texts(base);
                        }
                        {
                            if self.client.borrow().flags.borrow().do_physics { self.physics_engine.borrow_mut().tick(delta_time, &mut self.world.borrow_mut()); }
                        }

                        let current_fence = base.draw_commands_reuse_fences[current_frame];
                        {
                            base.device.wait_for_fences(&[current_fence], true, u64::MAX).expect("wait failed");
                            base.device.reset_fences(&[current_fence]).expect("reset failed");
                        }
                        let (present_index, _) = {
                            base
                                .swapchain_loader
                                .acquire_next_image(
                                    base.swapchain,
                                    u64::MAX,
                                    base.present_complete_semaphores[current_frame],
                                    vk::Fence::null(),
                                )
                                .unwrap()
                        };

                        let current_rendering_complete_semaphore = base.rendering_complete_semaphores[current_frame];
                        let current_draw_command_buffer = base.draw_command_buffers[current_frame];
                        let current_fence = base.draw_commands_reuse_fences[current_frame];

                        record_submit_commandbuffer(
                            &base.device,
                            current_draw_command_buffer,
                            current_fence,
                            base.present_queue,
                            &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
                            &[base.present_complete_semaphores[current_frame]],
                            &[current_rendering_complete_semaphore],
                            |_device, frame_command_buffer| {
                                {
                                    self.scene.borrow_mut().update_scene(base, current_frame);
                                    let world_ref = &mut self.world.borrow_mut();
                                    world_ref.update_lights(frame_command_buffer, current_frame);
                                    world_ref.update_cameras()
                                }

                                let flags = self.client.borrow().flags.clone();
                                {
                                    self.renderer.borrow_mut().render_frame(current_frame, present_index as usize, self.scene.clone(), flags.borrow().draw_hitboxes, &self.physics_engine.borrow());
                                }

                                Lua::run_cache(&engine_ref);
                            },
                        );
                        {
                            let mut controller_mut = self.client.borrow_mut();
                            controller_mut.reset_deltas()
                        };
                        // let start = std::time::Instant::now();
                        Lua::force_gc();
                        // let elapsed = start.elapsed();
                        // println!("GC took: {:?}", elapsed);

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

                        // frametime_manager.reset();
                        current_frame = (current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
                    },
                    _ => { Client::handle_event(self.client.clone(), event) },
                }
            }).expect("Failed to initiate render loop");

            base.device.device_wait_idle().unwrap();

            self.renderer.borrow_mut().destroy();
            self.world.borrow_mut().destroy(&base);
            // frametime_manager.destroy(&base);
        }
    }
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