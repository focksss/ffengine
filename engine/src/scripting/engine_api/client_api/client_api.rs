use std::cell::RefCell;
use std::sync::Arc;
use mlua::{FromLua, IntoLua, UserData, UserDataFields, UserDataMethods, Value};
use winit::dpi::PhysicalPosition;
use winit::event::MouseButton;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::keyboard::NativeKey::MacOS;
use winit::window::{CursorGrabMode, ResizeDirection};
use crate::client::client::{Client, Flags};
use crate::math::Vector;
use crate::physics::player::PlayerPointer;
use crate::scripting::lua_engine::RegisterToLua;

#[derive(Clone)]
pub struct ClientRef(pub Arc<RefCell<Client>>);
impl UserData for ClientRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("flags", |lua, this| {
            lua.create_userdata(FlagsRef(this.0.borrow_mut().flags.clone()))
        });

        fields.add_field_method_get("cursor_position", |lua, this| {
            lua.create_userdata(Vector::new2(this.0.borrow().cursor_position.x as f32, this.0.borrow().cursor_position.y as f32))
        });

        fields.add_field_method_get("scroll_delta", |lua, this| {
            let borrowed = this.0.borrow();
            lua.create_userdata(Vector::new2(borrowed.scroll_delta.0, borrowed.scroll_delta.1))
        });

        fields.add_field_method_get("mouse_delta", |lua, this| {
            let borrowed = this.0.borrow();
            lua.create_userdata(Vector::new2(borrowed.mouse_delta.0, borrowed.mouse_delta.1))
        });

        fields.add_field_method_get("cursor_locked", |_, this| {
            Ok(this.0.borrow().cursor_locked)
        });
        fields.add_field_method_set("cursor_locked", |_, this, val: bool| {
            let borrowed = &mut this.0.borrow_mut();
            borrowed.cursor_locked = val;
            if borrowed.cursor_locked {
                if let Err(err) = borrowed.window.set_cursor_grab(CursorGrabMode::Confined) {} else {
                    borrowed.window.set_cursor_visible(false);
                }
                borrowed.window.set_cursor_position(PhysicalPosition::new(
                    borrowed.window.inner_size().width as f32 * 0.5,
                    borrowed.window.inner_size().height as f32 * 0.5))
                    .expect("failed to reset mouse position");
            } else {
                if let Err(err) = borrowed.window.set_cursor_grab(CursorGrabMode::None) {} else {
                    borrowed.window.set_cursor_visible(true);
                }
                borrowed.window.set_cursor_position(borrowed.saved_cursor_pos).expect("Cursor pos reset failed");
            }
            Ok(())
        });

        fields.add_field_method_get("window_size", |_, this| {
            let borrowed = this.0.borrow();
            let window = borrowed.window.clone();
            Ok(Vector::new2(window.inner_size().width as f32, window.inner_size().height as f32))
        });

        fields.add_field_method_get("ButtonPressed", |_, this| {
            Ok(LuaMouseButton(this.0.borrow().button_pressed))
        });
        fields.add_field_method_get("ButtonReleased", |_, this| {
            Ok(LuaMouseButton(this.0.borrow().button_released))
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("new_key_pressed", |_, this, key: LuaKeyCode| {
            let physical_key = PhysicalKey::from(key.0);
            Ok(this.0.borrow().new_pressed_keys.contains(&physical_key))
        });
        methods.add_method("key_pressed", |_, this, key: LuaKeyCode| {
            let physical_key = PhysicalKey::from(key.0);
            Ok(this.0.borrow().pressed_keys.contains(&physical_key))
        });

        methods.add_method("drag_window", |_, this, ()| {
            let window = &this.0.borrow().window;
            if window.is_maximized() {
                window.set_maximized(false);
            }
            window.drag_window().ok();
            Ok(())
        });

        methods.add_method("drag_resize_window", |_, this, direction: LuaResizeDirection| {
            let window = &this.0.borrow().window;
            if window.is_maximized() {
                window.set_maximized(false);
            }
            Ok(this.0.borrow().window.drag_resize_window(ResizeDirection::from(direction.0)).expect("Failed to drag window"))
        });

        methods.add_method("mouse_button_pressed", |_, this, key: LuaMouseButton| {
            let mouse_button = MouseButton::from(key.0);
            Ok(this.0.borrow().pressed_mouse_buttons.contains(&mouse_button))
        });
    }
}

struct FlagsRef(pub Arc<RefCell<Flags>>);
impl UserData for FlagsRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("reload_rendering_queued", |_, this| Ok(this.0.borrow().reload_rendering_queued));
        fields.add_field_method_set("reload_rendering_queued", |_, this, val: bool| {
            this.0.borrow_mut().reload_rendering_queued = val;
            Ok(())
        });

        fields.add_field_method_get("reload_scripts_queued", |_, this| Ok(this.0.borrow().reload_scripts_queued));
        fields.add_field_method_set("reload_scripts_queued", |_, this, val: bool| {
            this.0.borrow_mut().reload_scripts_queued = val;
            Ok(())
        });

        fields.add_field_method_get("pause_rendering", |_, this| Ok(this.0.borrow().pause_rendering));
        fields.add_field_method_set("pause_rendering", |_, this, val: bool| {
            this.0.borrow_mut().pause_rendering = val;
            Ok(())
        });

        fields.add_field_method_get("screenshot_queued", |_, this| Ok(this.0.borrow().screenshot_queued));
        fields.add_field_method_set("screenshot_queued", |_, this, val: bool| {
            this.0.borrow_mut().pause_rendering = val;
            Ok(())
        });

        fields.add_field_method_get("draw_hitboxes", |_, this| Ok(this.0.borrow().draw_hitboxes));
        fields.add_field_method_set("draw_hitboxes", |_, this, val: bool| {
            this.0.borrow_mut().draw_hitboxes = val;
            Ok(())
        });

        fields.add_field_method_get("do_physics", |_, this| Ok(this.0.borrow().do_physics));
        fields.add_field_method_set("do_physics", |_, this, val: bool| {
            this.0.borrow_mut().do_physics = val;
            Ok(())
        });

        fields.add_field_method_get("close_requested", |_, this| Ok(this.0.borrow().close_requested));
        fields.add_field_method_set("close_requested", |_, this, val: bool| {
            this.0.borrow_mut().close_requested = val;
            Ok(())
        });
    }
}

pub struct LuaMouseButton(pub MouseButton);
impl RegisterToLua for LuaMouseButton {
    fn register_to_lua(lua: &mlua::Lua) -> mlua::Result<()> {
        let globals = lua.globals();
        let table = lua.create_table()?;
        for (idx, key) in ALL_MOUSE_BUTTONS.iter().enumerate() {
            table.set(format!("{:?}", key), idx as u32)?;
        }
        globals.set("MouseButton", table)?;
        Ok(())
    }
}
impl<'lua> IntoLua<'lua> for LuaMouseButton {
    fn into_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<Value<'lua>> {
        let index = ALL_MOUSE_BUTTONS
            .iter()
            .position(|k| *k == self.0)
            .ok_or_else(|| mlua::Error::ToLuaConversionError {
                from: "LuaMouseButton",
                to: "Value",
                message: Some("MouseButton not found in ALL_MOUSE_BUTTONS".into()),
            })?;
        (index as u32).into_lua(lua)
    }
}
impl<'lua> FromLua<'lua> for LuaMouseButton {
    fn from_lua(value: Value<'lua>, _: &'lua mlua::Lua) -> mlua::Result<Self> {
        if let Value::Integer(i) = value {
            if i >= 0 && (i as usize) < ALL_MOUSE_BUTTONS.len() {
                return Ok(LuaMouseButton(ALL_MOUSE_BUTTONS[i as usize]));
            }
        }
        Err(mlua::Error::FromLuaConversionError {
            from: value.type_name(),
            to: "LuaMouseButton",
            message: Some("invalid MouseButton value".into()),
        })
    }
}
pub struct LuaKeyCode(pub KeyCode);
impl RegisterToLua for LuaKeyCode {
    fn register_to_lua(lua: &mlua::Lua) -> mlua::Result<()> {
        let globals = lua.globals();
        let table = lua.create_table()?;
        for (idx, key) in ALL_KEYS.iter().enumerate() {
            table.set(format!("{:?}", key), idx as u32)?;
        }
        globals.set("KeyCode", table)?;
        Ok(())
    }
}
impl<'lua> IntoLua<'lua> for LuaKeyCode {
    fn into_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<Value<'lua>> {
        let index = ALL_KEYS
            .iter()
            .position(|k| *k == self.0)
            .ok_or_else(|| mlua::Error::ToLuaConversionError {
                from: "LuaKeyCode",
                to: "Value",
                message: Some("KeyCode not found in ALL_KEYS".into()),
            })?;
        (index as u32).into_lua(lua)
    }
}
impl<'lua> FromLua<'lua> for LuaKeyCode {
    fn from_lua(value: Value<'lua>, _: &'lua mlua::Lua) -> mlua::Result<Self> {
        if let Value::Integer(i) = value {
            if i >= 0 && (i as usize) < ALL_KEYS.len() {
                return Ok(LuaKeyCode(ALL_KEYS[i as usize]));
            }
        }
        Err(mlua::Error::FromLuaConversionError {
            from: value.type_name(),
            to: "LuaKeyCode",
            message: Some("invalid KeyCode value".into()),
        })
    }
}
pub struct LuaResizeDirection(pub ResizeDirection);
impl RegisterToLua for LuaResizeDirection {
    fn register_to_lua(lua: &mlua::Lua) -> mlua::Result<()> {
        let globals = lua.globals();
        let table = lua.create_table()?;
        for (idx, key) in ALL_RESIZE_DIRECTIONS.iter().enumerate() {
            table.set(format!("{:?}", key), idx as u32)?;
        }
        globals.set("ResizeDirection", table)?;
        Ok(())
    }
}
impl<'lua> IntoLua<'lua> for LuaResizeDirection {
    fn into_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<Value<'lua>> {
        let index = ALL_RESIZE_DIRECTIONS
            .iter()
            .position(|k| *k == self.0)
            .ok_or_else(|| mlua::Error::ToLuaConversionError {
                from: "LuaResizeDirection",
                to: "Value",
                message: Some("ResizeDirection not found in ALL_RESIZE_DIRECTIONS".into()),
            })?;
        (index as u32).into_lua(lua)
    }
}
impl<'lua> FromLua<'lua> for LuaResizeDirection {
    fn from_lua(value: Value<'lua>, _: &'lua mlua::Lua) -> mlua::Result<Self> {
        if let Value::Integer(i) = value {
            if i >= 0 && (i as usize) < ALL_RESIZE_DIRECTIONS.len() {
                return Ok(LuaResizeDirection(ALL_RESIZE_DIRECTIONS[i as usize]));
            }
        }
        Err(mlua::Error::FromLuaConversionError {
            from: value.type_name(),
            to: "LuaResizeDirection",
            message: Some("invalid ResizeDirection value".into()),
        })
    }
}
const ALL_RESIZE_DIRECTIONS: &[ResizeDirection] = &[
    ResizeDirection::East,
    ResizeDirection::North,
    ResizeDirection::NorthEast,
    ResizeDirection::NorthWest,
    ResizeDirection::South,
    ResizeDirection::SouthEast,
    ResizeDirection::SouthWest,
    ResizeDirection::West,
];
const ALL_MOUSE_BUTTONS: &[MouseButton] = &[
    MouseButton::Left,
    MouseButton::Right,
    MouseButton::Middle,
    MouseButton::Back,
    MouseButton::Forward,
];
const ALL_KEYS: &[KeyCode] = &[
    KeyCode::Backquote,
    KeyCode::Backslash,
    KeyCode::BracketLeft,
    KeyCode::BracketRight,
    KeyCode::Comma,
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    KeyCode::Equal,
    KeyCode::IntlBackslash,
    KeyCode::IntlRo,
    KeyCode::IntlYen,
    KeyCode::KeyA,
    KeyCode::KeyB,
    KeyCode::KeyC,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyI,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyN,
    KeyCode::KeyO,
    KeyCode::KeyP,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyS,
    KeyCode::KeyT,
    KeyCode::KeyU,
    KeyCode::KeyV,
    KeyCode::KeyW,
    KeyCode::KeyX,
    KeyCode::KeyY,
    KeyCode::KeyZ,
    KeyCode::Minus,
    KeyCode::Period,
    KeyCode::Quote,
    KeyCode::Semicolon,
    KeyCode::Slash,

    KeyCode::AltLeft,
    KeyCode::AltRight,
    KeyCode::Backspace,
    KeyCode::CapsLock,
    KeyCode::ContextMenu,
    KeyCode::ControlLeft,
    KeyCode::ControlRight,
    KeyCode::Enter,
    KeyCode::SuperLeft,
    KeyCode::SuperRight,
    KeyCode::ShiftLeft,
    KeyCode::ShiftRight,
    KeyCode::Space,
    KeyCode::Tab,

    KeyCode::Convert,
    KeyCode::KanaMode,
    KeyCode::Lang1,
    KeyCode::Lang2,
    KeyCode::Lang3,
    KeyCode::Lang4,
    KeyCode::Lang5,
    KeyCode::NonConvert,

    KeyCode::Delete,
    KeyCode::End,
    KeyCode::Help,
    KeyCode::Home,
    KeyCode::Insert,
    KeyCode::PageDown,
    KeyCode::PageUp,

    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::ArrowUp,

    KeyCode::NumLock,
    KeyCode::Numpad0,
    KeyCode::Numpad1,
    KeyCode::Numpad2,
    KeyCode::Numpad3,
    KeyCode::Numpad4,
    KeyCode::Numpad5,
    KeyCode::Numpad6,
    KeyCode::Numpad7,
    KeyCode::Numpad8,
    KeyCode::Numpad9,
    KeyCode::NumpadAdd,
    KeyCode::NumpadBackspace,
    KeyCode::NumpadClear,
    KeyCode::NumpadClearEntry,
    KeyCode::NumpadComma,
    KeyCode::NumpadDecimal,
    KeyCode::NumpadDivide,
    KeyCode::NumpadEnter,
    KeyCode::NumpadEqual,
    KeyCode::NumpadHash,
    KeyCode::NumpadMemoryAdd,
    KeyCode::NumpadMemoryClear,
    KeyCode::NumpadMemoryRecall,
    KeyCode::NumpadMemoryStore,
    KeyCode::NumpadMemorySubtract,
    KeyCode::NumpadMultiply,
    KeyCode::NumpadParenLeft,
    KeyCode::NumpadParenRight,
    KeyCode::NumpadStar,
    KeyCode::NumpadSubtract,

    KeyCode::Escape,
    KeyCode::Fn,
    KeyCode::FnLock,
    KeyCode::PrintScreen,
    KeyCode::ScrollLock,
    KeyCode::Pause,

    KeyCode::BrowserBack,
    KeyCode::BrowserFavorites,
    KeyCode::BrowserForward,
    KeyCode::BrowserHome,
    KeyCode::BrowserRefresh,
    KeyCode::BrowserSearch,
    KeyCode::BrowserStop,

    KeyCode::Eject,
    KeyCode::LaunchApp1,
    KeyCode::LaunchApp2,
    KeyCode::LaunchMail,

    KeyCode::MediaPlayPause,
    KeyCode::MediaSelect,
    KeyCode::MediaStop,
    KeyCode::MediaTrackNext,
    KeyCode::MediaTrackPrevious,

    KeyCode::Power,
    KeyCode::Sleep,

    KeyCode::AudioVolumeDown,
    KeyCode::AudioVolumeMute,
    KeyCode::AudioVolumeUp,
    KeyCode::WakeUp,

    KeyCode::Meta,
    KeyCode::Hyper,
    KeyCode::Turbo,
    KeyCode::Abort,
    KeyCode::Resume,
    KeyCode::Suspend,

    KeyCode::Again,
    KeyCode::Copy,
    KeyCode::Cut,
    KeyCode::Find,
    KeyCode::Open,
    KeyCode::Paste,
    KeyCode::Props,
    KeyCode::Select,
    KeyCode::Undo,

    KeyCode::Hiragana,
    KeyCode::Katakana,

    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
    KeyCode::F7,
    KeyCode::F8,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::F11,
    KeyCode::F12,
    KeyCode::F13,
    KeyCode::F14,
    KeyCode::F15,
    KeyCode::F16,
    KeyCode::F17,
    KeyCode::F18,
    KeyCode::F19,
    KeyCode::F20,
    KeyCode::F21,
    KeyCode::F22,
    KeyCode::F23,
    KeyCode::F24,
    KeyCode::F25,
    KeyCode::F26,
    KeyCode::F27,
    KeyCode::F28,
    KeyCode::F29,
    KeyCode::F30,
    KeyCode::F31,
    KeyCode::F32,
    KeyCode::F33,
    KeyCode::F34,
    KeyCode::F35,
];