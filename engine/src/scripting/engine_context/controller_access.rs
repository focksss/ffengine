use std::cell::RefCell;
use std::sync::Arc;
use mlua::{UserData, UserDataMethods};
use crate::client::controller::Controller;
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
}