use std::cell::RefCell;
use std::sync::Arc;
use mlua::{FromLua, Lua, UserData, UserDataFields, Value};
use crate::math::Vector;
use crate::scene::physics::player::{MovementMode, PlayerPointer};
use crate::scene::physics::rigid_body::RigidBodyPointer;
use crate::scripting::lua_engine::RegisterToLua;

impl RegisterToLua for MovementMode {
    fn register_to_lua(lua: &Lua) -> mlua::Result<()> {
        let globals = lua.globals();

        let movement_mode = lua.create_table()?;
        movement_mode.set("GHOST", 0)?;
        movement_mode.set("PHYSICS", 1)?;
        movement_mode.set("EDITOR", 2)?;

        globals.set("MovementMode", movement_mode)?;
        Ok(())
    }
}
impl mlua::IntoLua<'_> for MovementMode {
    fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
        Ok((self as u32).into_lua(lua)?)
    }
}
impl<'lua> FromLua<'lua> for MovementMode {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> mlua::Result<Self> {
        match value {
            Value::Integer(0) => Ok(MovementMode::GHOST),
            Value::Integer(1) => Ok(MovementMode::PHYSICS),
            Value::Integer(2) => Ok(MovementMode::EDITOR),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "MovementMode",
                message: Some("invalid MovementMode value".into()),
            })
        }
    }
}

impl UserData for PlayerPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("movement_mode", |_, this| {
            Ok(this.physics_engine.borrow().players[this.index].movement_mode)
        });
        fields.add_field_method_set("movement_mode", |lua, this, val: Value| {
            this.physics_engine.borrow_mut().players[this.index].movement_mode = MovementMode::from_lua(val, lua)?;
            Ok(())
        });
        fields.add_field_method_get("grounded", |_, this| {
            Ok(this.physics_engine.borrow().players[this.index].grounded)
        });

        fields.add_field_method_get("rigid_body", |lua, this| {
            lua.create_userdata(this.physics_engine.borrow().players[this.index].rigid_body_pointer.clone())
        });

        fields.add_field_method_get("camera", |lua, this| {
            lua.create_userdata(this.physics_engine.borrow().players[this.index].camera_pointer.clone())
        });
    }
}