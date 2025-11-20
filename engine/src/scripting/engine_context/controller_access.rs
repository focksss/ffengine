use std::cell::RefCell;
use std::sync::Arc;
use mlua::{UserData, UserDataFields, UserDataMethods};
use crate::client::controller::{Controller, Flags};
use crate::physics::player::MovementMode;

#[derive(Clone)]
pub struct ScriptController(pub Arc<RefCell<Controller>>);

impl UserData for ScriptController {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("set_reload_shaders", |_, this, val: bool| {
            this.0.borrow_mut().flags.reload_shaders_queued = val;
            Ok(())
        });

        methods.add_method("set_reload_gui", |_, this, val: bool| {
            this.0.borrow_mut().flags.reload_gui_queued = val;
            Ok(())
        });

        methods.add_method("set_pause_rendering", |_, this, val: bool| {
            this.0.borrow_mut().flags.pause_rendering = val;
            Ok(())
        });

        methods.add_method("set_screenshot", |_, this, val: bool| {
            this.0.borrow_mut().flags.screenshot_queued = val;
            Ok(())
        });

        methods.add_method("toggle_draw_hitboxes", |_, this, _: ()| {
            let mut ctrl = this.0.borrow_mut();
            ctrl.flags.draw_hitboxes = !ctrl.flags.draw_hitboxes;
            Ok(())
        });

        methods.add_method("toggle_physics", |_, this, _: ()| {
            let mut ctrl = this.0.borrow_mut();
            ctrl.flags.do_physics = !ctrl.flags.do_physics;
            Ok(())
        });

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
    fn add_fields<'lua, M: UserDataMethods<'lua, Self>>(fields: &mut M) {
        fields.add_field_method_get("flags", |_, this| Ok(this.flags))
    }
}

impl UserData for Flags {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("reload_gui_queued", |_, this| Ok(this.reload_gui_queued));
        fields.add_field_method_set("reload_gui_queued", |_, this, val: bool| {
            this.reload_gui_queued = val;
            Ok(())
        });

        fields.add_field_method_get("reload_shaders_queued", |_, this| Ok(this.reload_shaders_queued));
        fields.add_field_method_set("reload_shaders_queued", |_, this, val: bool| {
            this.reload_shaders_queued = val;
            Ok(())
        });

        fields.add_field_method_get("pause_rendering", |_, this| Ok(this.pause_rendering));
        fields.add_field_method_set("pause_rendering", |_, this, val: bool| {
            this.pause_rendering = val;
            Ok(())
        });

        fields.add_field_method_get("screenshot_queued", |_, this| Ok(this.screenshot_queued));
        fields.add_field_method_set("screenshot_queued", |_, this, val: bool| {
            this.pause_rendering = val;
            Ok(())
        });

        fields.add_field_method_get("draw_hitboxes", |_, this| Ok(this.draw_hitboxes));
        fields.add_field_method_set("draw_hitboxes", |_, this, val: bool| {
            this.draw_hitboxes = val;
            Ok(())
        });

        fields.add_field_method_get("do_physics", |_, this| Ok(this.do_physics));
        fields.add_field_method_set("do_physics", |_, this, val: bool| {
            this.do_physics = val;
            Ok(())
        });
    }
}