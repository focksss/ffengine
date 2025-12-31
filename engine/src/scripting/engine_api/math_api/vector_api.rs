use mlua::{FromLua, IntoLua, Lua, MetaMethod, UserData, UserDataMethods, Value};
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
        methods.add_function("new_vec4", |_, (x, y, z, w): (f32, f32, f32, f32)| {
            Ok(Vector::new4(x, y, z, w))
        });
        methods.add_function("new", |_, ()| {
            Ok(Vector::new())
        });
        methods.add_function("new3", |_, (x, y, z): (f32, f32, f32)| {
            Ok(Vector::new3(x, y, z))
        });
        methods.add_function("new4", |_, (x, y, z, w): (f32, f32, f32, f32)| {
            Ok(Vector::new4(x, y, z, w))
        });
        methods.add_function("new2", |_, (x, y): (f32, f32)| {
            Ok(Vector::new2(x, y))
        });
        methods.add_function("fill", |_, x: f32| {
            Ok(Vector::fill(x))
        });

        methods.add_method("normalize3", |_, this, ()| {
            Ok(this.normalize3())
        });
        methods.add_method("rotate_by_euler", |_, this, rot: Vector| {
            Ok(this.rotate_by_euler(&rot))
        });

        methods.add_method("quat_to_euler", |_, this, ()| {
            Ok(this.quat_to_euler())
        });
        methods.add_method("euler_to_quat", |_, this, ()| {
            Ok(this.euler_to_quat())
        });

        methods.add_meta_method(MetaMethod::Add, |_, this, other: Vector| {
            Ok(*this + other)
        });
        methods.add_meta_method(MetaMethod::Sub, |_, this, other: Vector| {
            Ok(*this - other)
        });
        methods.add_meta_method(MetaMethod::Mul, |_, this, rhs: Value| {
            match rhs {
                Value::UserData(ud) => {
                    let other = ud.borrow::<Vector>()?;
                    Ok(*this * *other)
                }
                Value::Number(n) => {
                    let f = n as f32;
                    Ok(Vector::new4(this.x * f, this.y * f, this.z * f, this.w * f))
                }
                _ => Err(mlua::Error::runtime("Vector can only be multiplied by Vector or number")),
            }
        });
        methods.add_meta_method(MetaMethod::Div, |_, this, other: Vector| {
            Ok(*this / other)
        });
        methods.add_meta_method(MetaMethod::Unm, |_, this, ()| {
            Ok(Vector::new4(-this.x, -this.y, -this.z, -this.w))
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