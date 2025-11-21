use std::cell::RefCell;
use std::sync::Arc;
use mlua::{FromLua, Lua, UserData, UserDataFields, Value};
use crate::math::Vector;
use crate::physics::player::{MovementMode, Player};

impl MovementMode {
    pub fn register(lua: &Lua) -> mlua::Result<()> {
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

pub struct PlayerRef(pub Arc<RefCell<Player>>);
impl UserData for PlayerRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("movement_mode", |_, this| {
            Ok(this.0.borrow().movement_mode)
        });
        fields.add_field_method_set("movement_mode", |lua, this, val: Value| {
            this.0.borrow_mut().movement_mode = MovementMode::from_lua(val, lua)?;
            Ok(())
        });

        fields.add_field_method_get("position", |lua, this| {
            Ok(this.0.borrow().rigid_body.position)
        });
    }
}