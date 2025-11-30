use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;
use winit::dpi::PhysicalPosition;
use winit::event::{DeviceEvent, ElementState, Event, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{PhysicalKey};
use winit::window::CursorGrabMode;
use crate::scripting::lua_engine::Lua;

pub struct Client {
    pub window: Arc<winit::window::Window>,

    pub sensitivity: f32,

    pub cursor_position: PhysicalPosition<f64>,
    pub flags: Arc<RefCell<Flags>>,

    pub pressed_keys: HashSet<PhysicalKey>,
    pub new_pressed_keys: HashSet<PhysicalKey>,

    pub pressed_mouse_buttons: HashSet<MouseButton>,
    pub new_pressed_mouse_buttons: HashSet<MouseButton>,
    /// Set to the last pressed button, to be used in scripts responding to MouseButtonPressed
    pub button_pressed: MouseButton,
    /// Set to the last released button, to be used in scripts responding to MouseButtonReleased
    pub button_released: MouseButton,

    pub mouse_delta: (f32, f32),
    pub scroll_delta: (f32, f32),
    pub cursor_locked: bool,
    pub saved_cursor_pos: PhysicalPosition<f64>,
    pub paused: bool,
    cursor_inside_window: bool
}
impl Client {
    pub fn new(window: Arc<winit::window::Window>) -> Client {
        window.set_cursor_position(PhysicalPosition::new(
            window.inner_size().width as f32 * 0.5,
            window.inner_size().height as f32 * 0.5))
            .expect("failed to reset mouse position");
        Client {
            window: window.clone(),
            sensitivity: 0.001,
            cursor_position: Default::default(),
            flags: Arc::new(RefCell::new(Flags::default())),
            pressed_keys: Default::default(),
            new_pressed_keys: Default::default(),
            pressed_mouse_buttons: Default::default(),
            new_pressed_mouse_buttons: Default::default(),
            mouse_delta: (0.0, 0.0),
            scroll_delta: (0.0, 0.0),
            cursor_locked: false,
            saved_cursor_pos: Default::default(),
            paused: false,
            button_pressed: MouseButton::Left,
            button_released: MouseButton::Left,
            cursor_inside_window: false,
        }
    }

    pub unsafe fn reset_deltas(
        &mut self,
    ) {
        self.scroll_delta = (0.0, 0.0);
        self.new_pressed_keys.clear();
        self.new_pressed_mouse_buttons.clear();
    }

    pub fn handle_event<T>(controller_ref: Arc<RefCell<Client>>, event: Event<T>) {
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
                            if !controller.pressed_mouse_buttons.contains(&button) { controller.new_pressed_mouse_buttons.insert(button.clone()); }
                            controller.pressed_mouse_buttons.insert(button.clone());
                            controller.button_pressed = button;
                            should_mouse_button_pressed_event = true;
                        }
                        ElementState::Released => {
                            controller.new_pressed_mouse_buttons.remove(&button);
                            controller.pressed_mouse_buttons.remove(&button);
                            controller.button_released = button;
                            should_mouse_button_released_event = true;
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::MouseWheel { delta, .. },
                    ..
                } => {
                    if controller.window.has_focus() {
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
                    if controller.window.has_focus() && controller.cursor_locked {
                        controller.mouse_delta = (
                            -position.x as f32 + 0.5 * controller.window.inner_size().width as f32,
                            position.y as f32 - 0.5 * controller.window.inner_size().height as f32,
                        );
                        controller.window.set_cursor_position(PhysicalPosition::new(
                            controller.window.inner_size().width as f32 * 0.5,
                            controller.window.inner_size().height as f32 * 0.5))
                            .expect("failed to reset mouse position");
                        should_mouse_move_event = true
                    } else {
                        controller.saved_cursor_pos = position;
                    }
                    controller.cursor_position = position;
                    controller.cursor_inside_window = true;
                }

                Event::WindowEvent {
                    event: WindowEvent::CursorLeft { .. },
                    ..
                } => {
                    controller.cursor_inside_window = false;
                }

                Event::WindowEvent {
                    event: WindowEvent::CursorEntered { .. },
                    ..
                } => {
                    controller.cursor_inside_window = true;
                }

                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta },
                    ..
                } => {
                    if controller.window.has_focus() && !controller.cursor_locked && !controller.cursor_inside_window {
                        controller.cursor_position.x += delta.0;
                        controller.cursor_position.y += delta.1;
                        // println!("mouse moved, {:?}", controller.cursor_position);
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Focused(true),
                    ..
                } => {
                    if controller.cursor_locked {
                        if let Err(err) = controller.window.set_cursor_grab(CursorGrabMode::Confined) {
                            eprintln!("Cursor lock failed: {:?}", err);
                        } else {
                            controller.window.set_cursor_visible(false);
                            controller.cursor_locked = true;
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Focused(false),
                    ..
                } => {
                    if let Err(err) = controller.window.set_cursor_grab(CursorGrabMode::None) {
                        eprintln!("Cursor unlock failed: {:?}", err);
                    } else {
                        controller.window.set_cursor_visible(true);
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
    pub pause_rendering: bool,
    pub screenshot_queued: bool,
    pub draw_hitboxes: bool,
    pub do_physics: bool,
    pub reload_rendering_queued: bool,
    pub reload_scripts_queued: bool,
    pub close_requested: bool,
}