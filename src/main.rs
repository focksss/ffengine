#![warn(unused_qualifications)]
mod render;
mod math;
mod engine;

use std::default::Default;
use std::error::Error;
use std::{mem, slice};
use std::collections::HashSet;
use std::mem::size_of;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ash::vk;
use ash::vk::{DescriptorType, Extent2D, Format, ImageAspectFlags, ImageSubresourceRange, Offset2D, QueryPool, ShaderStageFlags};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::CursorGrabMode;
use rand::*;
use math::vector::*;
use engine::scene::{Instance, Light, Model, Scene};
use engine::camera::Camera;
use engine::scene;
use crate::engine::render_engine::RenderEngine;
use crate::render::*;

const MAX_FRAMES_IN_FLIGHT: usize = 3;
const PI: f32 = std::f32::consts::PI;

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        #[cfg(debug_assertions)] {
            let mut shader_paths = Vec::new();
            shader_paths.push("resources\\shaders\\glsl\\geometry");
            shader_paths.push("resources\\shaders\\glsl\\shadow");
            shader_paths.push("resources\\shaders\\glsl\\ssao");
            shader_paths.push("resources\\shaders\\glsl\\bilateral_blur");
            shader_paths.push("resources\\shaders\\glsl\\lighting");
            shader_paths.push("resources\\shaders\\glsl\\quad");
            shader_paths.push("resources\\shaders\\glsl\\text");

            compile_shaders(shader_paths).expect("Failed to compile shaders");
        }

        let mut base = VkBase::new("ffengine".to_string(), 1920, 1080, MAX_FRAMES_IN_FLIGHT)?;
        run(&mut base).expect("Application launch failed!");
    }
    Ok(())
}

unsafe fn run(base: &mut VkBase) -> Result<(), Box<dyn Error>> { unsafe {
    //let font = Font::new(base, "resources\\fonts\\JetBrainsMono-Bold.ttf", Some(64), Some(2.0));
    //let font = Font::new(base, "resources\\fonts\\MonsieurLaDoulaise-Regular.ttf", Some(128), Some(2.0));
    let font = Font::new(base, "resources\\fonts\\Oxygen-Regular.ttf", Some(32), Some(2.0));
    let text_renderer = TextRenderer::new(base);

    let mut world = Scene::new(base);

    //world.preload_model(Model::new(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources\\models\\ffocks\\untitled.gltf").to_str().unwrap()));
    //world.models[0].transform_roots(&Vector::new_vec(0.0), &Vector::new_vec(0.0), &Vector::new_vec(0.01));
    //world.models[0].animations[0].repeat = true;
    //world.models[0].animations[0].start();

    //world.add_model(Model::new("C:\\Graphics\\assets\\flower\\scene.gltf"));
    //world.models[0].transform_roots(&Vector::new_vec3(0.0, 1.0, 0.0), &Vector::new_vec(0.0), &Vector::new_vec(1.0));
    //world.preload_model(Model::new("C:\\Graphics\\assets\\rivals\\luna\\gltf\\luna.gltf"));

    //world.preload_model(Model::new(&PathBuf::from("resources/models/shadowTest/shadowTest.gltf").to_str().unwrap()));
    world.preload_model(Model::new("C:\\Graphics\\assets\\sponzaGLTF\\sponza.gltf"));
    //world.preload_model(Model::new("C:\\Graphics\\assets\\bistroGLTF\\untitled.gltf"));
    //world.add_model(Model::new("C:\\Graphics\\assets\\asgard\\asgard.gltf"));
    //sa
    //world.preload_model(Model::new("C:\\Graphics\\assets\\helmet\\DamagedHelmet.gltf"));
    //world.add_model(Model::new("C:\\Graphics\\assets\\hydrant\\untitled.gltf"));

    //world.models[0].animations[0].repeat = true;
    //world.models[0].animations[0].start();
    world.initialize(MAX_FRAMES_IN_FLIGHT, true);
    let render_engine = RenderEngine::new(base, &world);

    //<editor-fold desc = "present renderpass">

    //</editor-fold>

    let mut player_camera = Camera::new_perspective_rotation(
        Vector::new_vec3(0.0, 0.0, 0.0),
        Vector::new_empty(),
        1.0,
        0.001,
        100.0,
        base.window.inner_size().width as f32 / base.window.inner_size().height as f32,
        0.001,
        1000.0,
        true,
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

    //let mut screenshot_manager = ScreenshotManager::new(base, &render_engine.lighting_pass.textures[0][0]);
    let mut screenshot_pending = false;

    let mut frametime_manager = FrametimeManager::new(base);

    let mut last_fps_render = Instant::now();
    let mut fps_tex = TextInformation::new(&font)
        .text("making this text long is one way to force the buffers to be large enough...")
        .position(Vector::new_vec2(100.0, 100.0))
        .font_size(32.0)
        .newline_distance(1720.0)
        .set_buffers();

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
                event: WindowEvent::Resized( _ ), // _ = new_size
                ..
            } => {
                println!("bruh");
                player_camera.aspect_ratio = base.window.inner_size().width as f32 / base.window.inner_size().height as f32;
                needs_resize = true;
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

                frametime_manager.record_cpu_action_start(String::from("deltatime setup"));
                //world.update_nodes(current_frame);
                if !pause_frustum { world.update_sun(&player_camera) };
                //<editor-fold desc = "frame setup">
                let now = Instant::now();
                let delta_time = now.duration_since(last_frame_time).as_secs_f32();
                last_frame_time = now;
                if needs_resize {

                }

                frametime_manager.record_cpu_action_end();
                frametime_manager.record_cpu_action_start(String::from("controls"));

                do_controls(
                    &mut player_camera,
                    &pressed_keys,
                    &mut new_pressed_keys,
                    delta_time,
                    &mut cursor_locked,
                    base,
                    &mut saved_cursor_pos,
                    &mut pause_frustum,
                    &mut screenshot_pending,
                    &mut world
                );

                frametime_manager.record_cpu_action_end();
                frametime_manager.record_cpu_action_start(String::from("update player camera"));

                player_camera.update_matrices();
                if !pause_frustum {
                    player_camera.update_frustum()
                }

                frametime_manager.record_cpu_action_end();
                frametime_manager.record_cpu_action_start(String::from("update light matrices"));


                //</editor-fold>

                frametime_manager.record_cpu_action_end();
                frametime_manager.record_cpu_action_start(String::from("wait for fence"));

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

                frametime_manager.record_cpu_action_end();
                frametime_manager.record_cpu_action_start(String::from("commandbuffer"));

                record_submit_commandbuffer(
                    &base.device,
                    current_draw_command_buffer,
                    current_fence,
                    base.present_queue,
                    &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
                    &[base.present_complete_semaphores[current_frame]],
                    &[current_rendering_complete_semaphore],
                    |device, frame_command_buffer| {
                        if last_fps_render.elapsed().as_secs_f32() > 0.1 {
                            fps_tex.update_text(format!("FPS: {}", 1.0 / delta_time).as_str());
                            fps_tex.update_buffers_all_frames(frame_command_buffer);
                            last_fps_render = Instant::now();
                        }

                        frametime_manager.record_gpu_action_start(frame_command_buffer, current_frame, String::from(
                            "frame ".to_owned() + current_frame.to_string().as_str())
                        );

                        text_renderer.render_text(current_frame, &fps_tex);

                        render_engine.render_frame(current_frame, present_index, &world, &player_camera, &mut frametime_manager, &text_renderer);

                        frametime_manager.record_gpu_action_end(frame_command_buffer, current_frame);
                    },
                );

                frametime_manager.record_cpu_action_end();
                frametime_manager.record_cpu_action_start(String::from("post commandbuffer"));

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

                frametime_manager.record_cpu_action_end();
                if screenshot_pending {

                    frametime_manager.report();

                    /*
                    base.device.queue_wait_idle(base.present_queue).unwrap();

                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let filename = format!("screenshots\\screenshot_{}.png", timestamp);

                    screenshot_manager.save_screenshot(filename);
                     */
                    screenshot_pending = false;
                }

                frametime_manager.reset();
                current_frame = (current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
            },
            _ => (),
        }
    }).expect("Failed to initiate render loop");


    println!("Render loop exited successfully, cleaning up");

    //<editor-fold desc = "cleanup">
    base.device.device_wait_idle().unwrap();

    font.destroy();
    text_renderer.destroy();
    //</editor-fold>
} Ok(()) }

unsafe fn do_controls(
    player_camera: &mut Camera,
    pressed_keys: &HashSet<PhysicalKey>,
    new_pressed_keys: &mut HashSet<PhysicalKey>,
    delta_time: f32,
    cursor_locked: &mut bool,
    base: &VkBase,
    saved_cursor_pos: &mut PhysicalPosition<f64>,
    paused: &mut bool,
    screenshot_pending: &mut bool,
    world: &mut Scene,
) { unsafe {
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
    if new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyM)) {
        let models = world.models.len();
        if models < 2 {
            world.add_model(Model::new("C:\\Graphics\\assets\\helmet\\DamagedHelmet.gltf"));
            world.models[0.max(models)].transform_roots(&player_camera.position, &player_camera.rotation, &Vector::new_vec(1.0));
        }
    }
    if new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::F2)) {
        *screenshot_pending = true;
    }

    //player_camera.position.println();

    new_pressed_keys.clear();
} }
fn do_mouse(player_camera: &mut Camera, mouse_delta: (f32, f32), cursor_locked: &mut bool) {
    if *cursor_locked {
        player_camera.rotation.y += player_camera.sensitivity * mouse_delta.0;
        player_camera.rotation.x += player_camera.sensitivity * mouse_delta.1;
        player_camera.rotation.x = player_camera.rotation.x.clamp(-PI * 0.5, PI * 0.5);
    }
}

pub struct FrametimeManager<'a> {
    base: &'a VkBase,
    cpu_actions: Vec<(Instant, Duration, String)>, // Start time, duration, name
    gpu_action_timestamp_pairs: Vec<String>,
    current_cpu_action_index: usize,
    recording_cpu: bool,

    query_pool: QueryPool,
}
impl FrametimeManager<'_> {
    pub fn new(base: &VkBase) -> FrametimeManager {
        let query_pool_info = vk::QueryPoolCreateInfo::default()
            .query_type(vk::QueryType::TIMESTAMP)
            .query_count(2 * MAX_FRAMES_IN_FLIGHT as u32);
        let query_pool = unsafe {
            base.device.create_query_pool(&query_pool_info, None).unwrap()
        };
        FrametimeManager {
            base,
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

    pub fn record_gpu_action_start(&mut self, command_buffer: vk::CommandBuffer, action_index: usize, name: String) { unsafe {
        self.base.device.cmd_reset_query_pool(command_buffer, self.query_pool, action_index as u32 * 2, 2);
        self.base.device.cmd_write_timestamp(
            command_buffer,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            self.query_pool,
            action_index as u32 * 2,
        );
        self.gpu_action_timestamp_pairs[action_index] = name;
    } }
    pub fn record_gpu_action_end(&mut self, command_buffer: vk::CommandBuffer, action_index: usize) { unsafe {
        self.base.device.cmd_write_timestamp(
            command_buffer,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            self.query_pool,
            (action_index as u32 * 2) + 1,
        );
    } }

    pub fn report(&mut self) {
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
            self.base.device.get_query_pool_results(
                self.query_pool,
                i as u32 * 2,
                &mut timestamps,
                vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
            ).unwrap();
            let timestamp_period = unsafe { self.base.instance.get_physical_device_properties(self.base.pdevice).limits.timestamp_period };
            let gpu_time_ns = (timestamps[1] - timestamps[0]) as f64 * timestamp_period as f64;
            let gpu_time_ms = gpu_time_ns / 1_000_000.0;
            println!("    - {} with duration of: {:.3} ms", action_name, gpu_time_ms);
        } }
    }
}
impl Drop for FrametimeManager<'_> {
    fn drop(&mut self) { unsafe {
        self.base.device.destroy_query_pool(self.query_pool, None);
    } }
}