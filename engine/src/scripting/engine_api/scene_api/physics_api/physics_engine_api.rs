use std::cell::RefCell;
use std::sync::Arc;
use mlua::{FromLua, UserData, UserDataFields, UserDataMethods, Value};
use crate::math::Vector;
use crate::scene::physics::physics_engine::PhysicsEngine;
use crate::scene::physics::player::PlayerPointer;

#[derive(Clone)]
pub struct PhysicsEngineRef(pub Arc<RefCell<PhysicsEngine>>);
impl UserData for PhysicsEngineRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("gravity", |_, this| {
            Ok(this.0.borrow_mut().gravity.clone())
        });
        fields.add_field_method_set("gravity", |lua, this, val: Value| {
            this.0.borrow_mut().gravity = Vector::from_lua(val, lua)?;
            Ok(())
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_player", |lua, this, index: usize| {
            let node = PlayerPointer { physics_engine: this.0.clone(), index };
            lua.create_userdata(node)
        });
    }
}