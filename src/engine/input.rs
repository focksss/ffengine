use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use ash::vk;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::EventLoopWindowTarget;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::CursorGrabMode;
use crate::engine::input;
use crate::engine::physics::player::Player;
use crate::engine::world::camera::Camera;
use crate::engine::world::scene::Scene;
use crate::gui::gui::{GUIInteractableInformation, GUINode, GUIQuad};
use crate::math::Vector;
use crate::PI;
use crate::render::render::{screenshot_texture, Renderer};
use crate::render::VkBase;

pub struct Controller {
    pub window_ptr: *const winit::window::Window,
    pub player: Arc<RefCell<Player>>,

    pub cursor_position: PhysicalPosition<f64>,
    pub queue_flags: Flags,

    pub pressed_keys: HashSet<PhysicalKey>,
    pub new_pressed_keys: HashSet<PhysicalKey>,

    pub pressed_mouse_buttons: HashSet<MouseButton>,

    pub mouse_delta: (f32, f32),
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
            cursor_position: Default::default(),
            queue_flags: Default::default(),
            player: Arc::new(RefCell::new(Player::new(
                Camera::new_perspective_rotation(
                    Vector::new_vec3(0.0, 1.0, 0.0),
                    Vector::new_empty(),
                    1.0,
                    0.001,
                    100.0,
                    window.inner_size().width as f32 / window.inner_size().height as f32,
                    0.001,
                    1000.0,
                    true,
                    Vector::new_vec3(0.0, 0.0, 1.0),
                ),
                Vector::new_vec3(-0.1, -0.5, -0.1),
                Vector::new_vec3(0.1, 0.1, 0.1))
            )),
            pressed_keys: Default::default(),
            new_pressed_keys: Default::default(),
            pressed_mouse_buttons: Default::default(),
            mouse_delta: (0.0, 0.0),
            cursor_locked: false,
            saved_cursor_pos: Default::default(),
            paused: false,
        }
    }
    fn window(&self) -> &winit::window::Window {
        unsafe { &*self.window_ptr }
    }

    pub unsafe fn do_controls(
        &mut self,
        delta_time: f32,
        base: &VkBase,
        renderer: &mut Renderer,
        world: &Scene,
        frame: usize,
    ) { unsafe {
        if self.queue_flags.screenshot_queued {
            self.queue_flags.screenshot_queued = false;
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
        if self.queue_flags.reload_shaders_queued {
            self.queue_flags.reload_shaders_queued = false;
            Renderer::compile_shaders();
            renderer.reload(base, world);
        }
        if self.queue_flags.reload_gui_queued {
            self.queue_flags.reload_gui_queued = false;
            renderer.gui.load_from_file(base, "resources\\gui\\default.gui");
        }

        let mut move_direction= Vector::new_vec(0.0);
        let camera_rotation = self.player.borrow().camera.rotation;
        if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyW)) {
            move_direction.x += (camera_rotation.y + PI/2.0).cos();
            move_direction.z -= (camera_rotation.y + PI/2.0).sin();
        }
        if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyA)) {
            move_direction.x -= camera_rotation.y.cos();
            move_direction.z += camera_rotation.y.sin();
        }
        if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyS)) {
            move_direction.x -= (camera_rotation.y + PI/2.0).cos();
            move_direction.z += (camera_rotation.y + PI/2.0).sin();
        }
        if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyD)) {
            move_direction.x += camera_rotation.y.cos();
            move_direction.z -= camera_rotation.y.sin();
        }
        if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::Space)) {
            move_direction.y += 1.0;
        }
        if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::ShiftLeft)) {
            move_direction.y -= 1.0;
        }

        if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::Equal)) {
            self.player.borrow_mut().camera.speed *= 1.0 + 1.0*delta_time;
        }
        if self.pressed_keys.contains(&PhysicalKey::Code(KeyCode::Minus)) {
            self.player.borrow_mut().camera.speed /= 1.0 + 1.0*delta_time;
        }

        if self.new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::Escape)) {
            self.cursor_locked = !self.cursor_locked;
            if self.cursor_locked {
                if let Err(err) = self.window().set_cursor_grab(CursorGrabMode::Confined) {
                } else {
                    self.window().set_cursor_visible(false);
                }
                self.window().set_cursor_position(PhysicalPosition::new(
                    self.window().inner_size().width as f32 * 0.5,
                    self.window().inner_size().height as f32 * 0.5))
                    .expect("failed to reset mouse position");
            } else {
                if let Err(err) = self.window().set_cursor_grab(CursorGrabMode::None) {
                } else {
                    self.window().set_cursor_visible(true);
                }
                self.window().set_cursor_position(self.saved_cursor_pos).expect("Cursor pos reset failed");
            }
        }
        if self.new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyP)) {
            self.paused = !self.paused
        }

        if self.new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::F2)) {
            self.queue_flags.screenshot_queued = true;
        }
        if self.new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::F5)) {
            let last_third_person_state = self.player.borrow().camera.third_person;
            self.player.borrow_mut().camera.third_person = !last_third_person_state;
        }

        let speed = { self.player.borrow().camera.speed };

        self.player.borrow_mut().step(move_direction * (delta_time * speed));

        self.new_pressed_keys.clear();
    } }
    
    pub fn update_camera(&mut self) {
        self.player.borrow_mut().camera.update_matrices();
        if !self.paused {
            self.player.borrow_mut().camera.update_frustum()
        }
    }

    pub fn handle_event<T>(&mut self, event: Event<T>, elwp: &EventLoopWindowTarget<T>) {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                elwp.exit();
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
                        if !self.pressed_keys.contains(&physical_key) { self.new_pressed_keys.insert(physical_key.clone()); }
                        self.pressed_keys.insert(physical_key.clone());
                    }
                    ElementState::Released => {
                        self.pressed_keys.remove(&physical_key);
                        self.new_pressed_keys.remove(&physical_key);
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
                        if !self.pressed_mouse_buttons.contains(&button) { self.pressed_mouse_buttons.insert(button.clone()); }
                    }
                    ElementState::Released => {
                        if self.pressed_mouse_buttons.contains(&button) { self.pressed_mouse_buttons.remove(&button); }
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                if self.window().has_focus() && self.cursor_locked {
                    self.mouse_delta = (
                        -position.x as f32 + 0.5 * self.window().inner_size().width as f32,
                        position.y as f32 - 0.5 * self.window().inner_size().height as f32,
                    );
                    self.window().set_cursor_position(PhysicalPosition::new(
                        self.window().inner_size().width as f32 * 0.5,
                        self.window().inner_size().height as f32 * 0.5))
                        .expect("failed to reset mouse position");
                    self.do_mouse();
                } else {
                    self.saved_cursor_pos = position;
                }
                self.cursor_position = position;
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(true),
                ..
            } => {
                if !self.cursor_locked {
                    if let Err(err) = self.window().set_cursor_grab(CursorGrabMode::Confined) {
                        eprintln!("Cursor lock failed: {:?}", err);
                    } else {
                        self.window().set_cursor_visible(false);
                        self.cursor_locked = true;
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(false),
                ..
            } => {
                self.cursor_locked = false;
                if let Err(err) = self.window().set_cursor_grab(CursorGrabMode::None) {
                    eprintln!("Cursor unlock failed: {:?}", err);
                } else {
                    self.window().set_cursor_visible(true);
                }
            }
            _ => {}
        }
    }

    fn do_mouse(&mut self) {
        if self.cursor_locked {
            let rotation_x_delta = self.mouse_delta.1;
            let rotation_y_delta = self.mouse_delta.0;
            let sense = { self.player.borrow().camera.sensitivity };
            { self.player.borrow_mut().camera.rotation.y += rotation_y_delta * sense; }
            { self.player.borrow_mut().camera.rotation.x -= rotation_x_delta * sense; }
            let new_rotation_x = { self.player.borrow().camera.rotation.x };
            self.player.borrow_mut().camera.rotation.x = new_rotation_x.clamp(-PI * 0.5, PI * 0.5);
        }
    }
}

#[derive(Default)]
pub struct Flags {
    pub reload_gui_queued: bool,
    pub reload_shaders_queued: bool,
    pub pause_rendering: bool,
    pub screenshot_queued: bool,
}