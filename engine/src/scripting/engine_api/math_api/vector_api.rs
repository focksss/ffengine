use mlua::{FromLua, IntoLua, Lua, UserData, UserDataMethods, Value};
use crate::math::Vector;
use crate::scripting::lua_engine::RegisterToLua;

impl RegisterToLua for Vector {
    fn register_to_lua(lua: &Lua) -> mlua::Result<()> {
        let globals = lua.globals();
        globals.set("Vector", lua.create_proxy::<Vector>()?)?;
        Ok(())
    }
}

impl UserData for Vector {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.x));
        fields.add_field_method_set("x", |_, this, val: f32| {
            this.x = val;
            Ok(())
        });

        fields.add_field_method_get("y", |_, this| Ok(this.y));
        fields.add_field_method_set("y", |_, this, val: f32| {
            this.y = val;
            Ok(())
        });

        fields.add_field_method_get("z", |_, this| Ok(this.z));
        fields.add_field_method_set("z", |_, this, val: f32| {
            this.z = val;
            Ok(())
        });

        fields.add_field_method_get("w", |_, this| Ok(this.w));
        fields.add_field_method_set("w", |_, this, val: f32| {
            this.w = val;
            Ok(())
        });
    }

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("new", |_, (x, y, z, w): (f32, f32, f32, f32)| {
            Ok(Vector::new_vec4(x, y, z, w))
        });
    }
}
impl<'lua> FromLua<'lua> for Vector {
    fn from_lua(value: Value<'lua>, _lua: &'lua Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => {
                Ok(*ud.borrow::<Vector>()?)
            }
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Vector",
                message: Some("expected Vector userdata".into()),
            })
        }
    }
}