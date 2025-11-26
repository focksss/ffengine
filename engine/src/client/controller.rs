use std::cell::RefCell;
use std::collections::HashSet;
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Instant;
use ash::vk;
use mlua::{UserData, UserDataMethods};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::EventLoopWindowTarget;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::CursorGrabMode;
use crate::math::Vector;
use crate::physics::physics_engine::PhysicsEngine;
use crate::physics::player::{MovementMode, Player, PlayerPointer};
use crate::render::render::{screenshot_texture, Renderer};
use crate::render::vulkan_base::VkBase;
use crate::scripting::lua_engine::Lua;
use crate::world::camera::Camera;
use crate::world::scene::Scene;

pub struct Controller {
    pub window_ptr: *const winit::window::Window,

    pub sensitivity: f32,

    pub cursor_position: PhysicalPosition<f64>,
    pub flags: Arc<RefCell<Flags>>,

    pub pressed_keys: HashSet<PhysicalKey>,
    pub new_pressed_keys: HashSet<PhysicalKey>,

    pub pressed_mouse_buttons: HashSet<MouseButton>,

    /// Set to the last pressed button, to be used in scripts responding to MouseButtonPressed
    pub button_pressed: MouseButton,
    /// Set to the last released button, to be used in scripts responding to MouseButtonReleased
    pub button_released: MouseButton,

    pub mouse_delta: (f32, f32),
    pub scroll_delta: (f32, f32),
    pub cursor_locked: bool,
    pub saved_cursor_pos: PhysicalPosition<f64>,
    pub paused: bool,
}
impl Controller {
    pub fn new(window: &winit::window::Window) -> Controller {
        window.set_cursor_position(PhysicalPosition::new(
            window.inner_size().width as f32 * 0.5,
            window.inner_size().height as f32 * 0.5))
            .expect("failed to reset mouse position");
        Controller {
            window_ptr: window as *const _,
            sensitivity: 0.001,
            cursor_position: Default::default(),
            flags: Arc::new(RefCell::new(Flags::default())),
            pressed_keys: Default::default(),
            new_pressed_keys: Default::default(),
            pressed_mouse_buttons: Default::default(),
            mouse_delta: (0.0, 0.0),
            scroll_delta: (0.0, 0.0),
            cursor_locked: false,
            saved_cursor_pos: Default::default(),
            paused: false,
            button_pressed: MouseButton::Left,
            button_released: MouseButton::Left,
        }
    }
    pub(crate) fn window(&self) -> &winit::window::Window {
        unsafe { &*self.window_ptr }
    }

    pub unsafe fn reset_deltas(
        &mut self,
    ) {
        /*
        let mut move_direction = Vector::new_vec(0.0);
        {
            let physics_engine = &mut self.player_pointer.physics_engine.borrow_mut();
            let player = &mut physics_engine.players[self.player_pointer.index];

            let flags = &mut self.flags.borrow_mut();
            if flags.screenshot_queued {
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
            }
            if flags.reload_shaders_queued {
                flags.reload_shaders_queued = false;
                Renderer::compile_shaders();
                renderer.reload(base, world);
            }
            if flags.reload_gui_queued {
                flags.reload_gui_queued = false;
                renderer.gui.borrow_mut().load_from_file(base, "editor\\resources\\gui\\default\\default.gui");
            }

            let movement_mode = player.movement_mode.clone();

            //println!("{:?}", self.scroll_delta);

            if self.scroll_delta.1 != 0.0 {
                match movement_mode {
                    MovementMode::GHOST => {
                        if self.scroll_delta.1 > 0.0 {
                            player.fly_speed *= 1.0 + 10.0 * delta_time;
                        } else {
                            player.fly_speed /= 1.0 + 10.0 * delta_time;
                        }
                    }
                    MovementMode::PHYSICS => {
                        if self.scroll_delta.1 > 0.0 {
                            player.fly_speed *= 1.0 + 10.0 * delta_time;
                        } else {
                            player.fly_speed /= 1.0 + 10.0 * delta_time;
                        }
                    }
                    MovementMode::EDITOR => {}
                }
            }

            {
                let camera = &mut world.cameras[player.camera_pointer.index];
                let camera_rotation = camera.rotation;
                if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyW)) {
                    move_direction.x += (camera_rotation.y + PI / 2.0).cos();
                    move_direction.z -= (camera_rotation.y + PI / 2.0).sin();
                }
                if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyA)) {
                    move_direction.x -= camera_rotation.y.cos();
                    move_direction.z += camera_rotation.y.sin();
                }
                if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyS)) {
                    move_direction.x -= (camera_rotation.y + PI / 2.0).cos();
                    move_direction.z += (camera_rotation.y + PI / 2.0).sin();
                }
                if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyD)) {
                    move_direction.x += camera_rotation.y.cos();
                    move_direction.z -= camera_rotation.y.sin();
                }
                if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::Space)) {
                    match player.movement_mode {
                        MovementMode::GHOST => {
                            move_direction.y += 1.0;
                        }
                        MovementMode::PHYSICS => {
                            if player.grounded { move_direction.y += 1.0; }
                        }
                        MovementMode::EDITOR => {}
                    }
                }
                if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::ShiftLeft)) {
                    match player.movement_mode {
                        MovementMode::GHOST => {
                            move_direction.y -= 1.0;
                        }
                        MovementMode::PHYSICS => {}
                        MovementMode::EDITOR => {}
                    }
                }

                if self.new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::Escape)) {
                    self.cursor_locked = !self.cursor_locked;
                    if self.cursor_locked {
                        if let Err(err) = self.window().set_cursor_grab(CursorGrabMode::Confined) {} else {
                            self.window().set_cursor_visible(false);
                        }
                        self.window().set_cursor_position(PhysicalPosition::new(
                            self.window().inner_size().width as f32 * 0.5,
                            self.window().inner_size().height as f32 * 0.5))
                            .expect("failed to reset mouse position");
                    } else {
                        if let Err(err) = self.window().set_cursor_grab(CursorGrabMode::None) {} else {
                            self.window().set_cursor_visible(true);
                        }
                        self.window().set_cursor_position(self.saved_cursor_pos).expect("Cursor pos reset failed");
                    }
                }
                if self.new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyP)) {
                    self.paused = !self.paused
                }

                if self.new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::F2)) {
                    flags.screenshot_queued = true;
                }
                if self.new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::F5)) {
                    let last_third_person_state = camera.third_person;
                    camera.third_person = !last_third_person_state;
                }
            }
        }
        {
            let (index, speed, fly_speed, movement_mode, jump_power, move_power) = {
                let physics_engine = &self.player_pointer.physics_engine.borrow();
                let player = &physics_engine.players[self.player_pointer.index];
                let speed = player.move_power;
                (player.rigid_body_pointer.index, speed, player.fly_speed, player.movement_mode, player.jump_power, player.move_power)
            };
            let rb = &mut self.player_pointer.physics_engine.borrow_mut().rigid_bodies[index];
            match movement_mode {
                MovementMode::GHOST => {
                    rb.position += move_direction * fly_speed * delta_time
                }
                MovementMode::PHYSICS => {
                    rb.velocity = rb.velocity + move_direction *
                        Vector::new_vec3(move_power, jump_power, move_power) * speed;
                }
                MovementMode::EDITOR => {}
            }
        }
*/

        self.scroll_delta = (0.0, 0.0);
        self.new_pressed_keys.clear();
    }

    pub fn handle_event<T>(controller_ref: Arc<RefCell<Controller>>, event: Event<T>) {
        let mut should_scroll_event = false;
        let mut should_mouse_move_event = false;
        let mut should_mouse_button_pressed_event = false;
        let mut should_mouse_button_released_event = false;
        {
            let controller = &mut controller_ref.borrow_mut();
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    controller.flags.borrow_mut().close_requested = true;
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
                            if !controller.pressed_keys.contains(&physical_key) { controller.new_pressed_keys.insert(physical_key.clone()); }
                            controller.pressed_keys.insert(physical_key.clone());
                        }
                        ElementState::Released => {
                            controller.pressed_keys.remove(&physical_key);
                            controller.new_pressed_keys.remove(&physical_key);
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::MouseInput {
                        state,
                        button,
                        ..
                    },
                    ..
                } => {
                    match state {
                        ElementState::Pressed => {
                            controller.button_pressed = button;
                            should_mouse_button_pressed_event = true;
                            if !controller.pressed_mouse_buttons.contains(&button) { controller.pressed_mouse_buttons.insert(button.clone()); }
                        }
                        ElementState::Released => {
                            controller.button_released = button;
                            should_mouse_button_released_event = true;
                            if controller.pressed_mouse_buttons.contains(&button) { controller.pressed_mouse_buttons.remove(&button); }
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::MouseWheel { delta, .. },
                    ..
                } => {
                    if controller.window().has_focus() {
                        if let MouseScrollDelta::LineDelta(x, y) = delta {
                            controller.scroll_delta = (x, y);
                        }
                        should_scroll_event = true;
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    ..
                } => {
                    if controller.window().has_focus() && controller.cursor_locked {
                        controller.mouse_delta = (
                            -position.x as f32 + 0.5 * controller.window().inner_size().width as f32,
                            position.y as f32 - 0.5 * controller.window().inner_size().height as f32,
                        );
                        controller.window().set_cursor_position(PhysicalPosition::new(
                            controller.window().inner_size().width as f32 * 0.5,
                            controller.window().inner_size().height as f32 * 0.5))
                            .expect("failed to reset mouse position");
                        should_mouse_move_event = true
                    } else {
                        controller.saved_cursor_pos = position;
                    }
                    controller.cursor_position = position;
                }
                Event::WindowEvent {
                    event: WindowEvent::Focused(true),
                    ..
                } => {
                    if !controller.cursor_locked {
                        if let Err(err) = controller.window().set_cursor_grab(CursorGrabMode::Confined) {
                            eprintln!("Cursor lock failed: {:?}", err);
                        } else {
                            controller.window().set_cursor_visible(false);
                            controller.cursor_locked = true;
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Focused(false),
                    ..
                } => {
                    controller.cursor_locked = false;
                    if let Err(err) = controller.window().set_cursor_grab(CursorGrabMode::None) {
                        eprintln!("Cursor unlock failed: {:?}", err);
                    } else {
                        controller.window().set_cursor_visible(true);
                    }
                }
                _ => {}
            }
        }
        if should_scroll_event {
            Lua::run_scroll_methods().expect("failed to run scroll methods");
        }
        if should_mouse_move_event {
            Lua::run_mouse_moved_methods().expect("failed to run mouse moved methods");
        }
        if should_mouse_button_pressed_event {
            Lua::run_mouse_button_pressed_methods().expect("failed to run mouse button pressed methods");
        }
        if should_mouse_button_released_event {
            Lua::run_mouse_button_released_methods().expect("failed to run mouse button pressed methods");
        }
    }
}

#[derive(Default)]
pub struct Flags {
    pub reload_gui_queued: bool,
    pub reload_shaders_queued: bool,
    pub pause_rendering: bool,
    pub screenshot_queued: bool,
    pub draw_hitboxes: bool,
    pub do_physics: bool,
    pub reload_all_scripts_queued: bool,
    pub close_requested: bool,
}