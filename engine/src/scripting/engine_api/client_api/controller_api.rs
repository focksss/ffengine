use std::cell::RefCell;
use std::sync::Arc;
use mlua::{UserData, UserDataFields, UserDataMethods};
use crate::client::controller::{Controller, Flags};
use crate::physics::player::MovementMode;
use crate::scripting::engine_api::physics_api::player_api::PlayerRef;

#[derive(Clone)]
pub struct ControllerRef(pub Arc<RefCell<Controller>>);
impl UserData for ControllerRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("flags", |lua, this| {
            lua.create_userdata(FlagsRef(this.0.borrow_mut().flags.clone()))
        });
        fields.add_field_method_get("player", |lua, this| {
            lua.create_userdata(PlayerRef(this.0.borrow_mut().player.clone()))
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("toggle_player_physics", |_, this, _: ()| {
            let ctrl = this.0.borrow_mut();
            let last_state = ctrl.player.borrow().movement_mode.clone();
            match last_state {
                MovementMode::PHYSICS => {
                    ctrl.player.borrow_mut().movement_mode = MovementMode::GHOST
                }
                MovementMode::GHOST => {
                    ctrl.player.borrow_mut().movement_mode = MovementMode::PHYSICS
                }
                _ => ()
            }
            Ok(())
        });

        methods.add_method("get_camera_position", |_, this, _: ()| {
            let pos = this.0.borrow().player.borrow().camera.position;
            Ok((pos.x, pos.y, pos.z))
        });
    }
}

struct FlagsRef(pub Arc<RefCell<Flags>>);
impl UserData for FlagsRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("reload_gui_queued", |_, this| Ok(this.0.borrow().reload_gui_queued));
        fields.add_field_method_set("reload_gui_queued", |_, this, val: bool| {
            this.0.borrow_mut().reload_gui_queued = val;
            Ok(())
        });

        fields.add_field_method_get("reload_shaders_queued", |_, this| Ok(this.0.borrow().reload_shaders_queued));
        fields.add_field_method_set("reload_shaders_queued", |_, this, val: bool| {
            this.0.borrow_mut().reload_shaders_queued = val;
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
    }
}