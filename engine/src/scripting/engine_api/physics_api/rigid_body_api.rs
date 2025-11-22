use mlua::{FromLua, Lua, UserData, UserDataFields, Value};
use crate::math::Vector;
use crate::physics::rigid_body::RigidBodyPointer;

impl UserData for RigidBodyPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("position", |_, this| {
            Ok(this.physics_engine.borrow().rigid_bodies[this.index].position)
        });
        fields.add_field_method_set("position", |lua, this, val: Value| {
            this.physics_engine.borrow_mut().rigid_bodies[this.index].position = Vector::from_lua(val, lua)?;
            Ok(())
        });
    }
}