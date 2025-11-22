use mlua::{FromLua, UserData, UserDataFields, Value};
use crate::math::Vector;
use crate::world::camera::CameraPointer;

impl UserData for CameraPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("position", |_, this| {
            Ok(this.world.borrow().cameras[this.index].position)
        });
        fields.add_field_method_set("position", |lua, this, val: Value| {
            this.world.borrow_mut().cameras[this.index].position = Vector::from_lua(val, lua)?;
            Ok(())
        });

        fields.add_field_method_get("rotation", |_, this| {
            Ok(this.world.borrow().cameras[this.index].rotation)
        });
        fields.add_field_method_set("rotation", |lua, this, val: Value| {
            this.world.borrow_mut().cameras[this.index].rotation = Vector::from_lua(val, lua)?;
            Ok(())
        });

        fields.add_field_method_get("fov_y", |_, this| {
            Ok(this.world.borrow().cameras[this.index].fov_y)
        });
        fields.add_field_method_set("fov_y", |_, this, val: f32| {
            this.world.borrow_mut().cameras[this.index].fov_y = val;
            Ok(())
        });

        fields.add_field_method_get("aspect_ratio", |_, this| {
            Ok(this.world.borrow().cameras[this.index].aspect_ratio)
        });
        fields.add_field_method_set("aspect_ratio", |_, this, val: f32| {
            this.world.borrow_mut().cameras[this.index].aspect_ratio = val;
            Ok(())
        });

        fields.add_field_method_get("near", |_, this| {
            Ok(this.world.borrow().cameras[this.index].near)
        });
        fields.add_field_method_set("near", |_, this, val: f32| {
            this.world.borrow_mut().cameras[this.index].near = val;
            Ok(())
        });

        fields.add_field_method_get("far", |_, this| {
            Ok(this.world.borrow().cameras[this.index].far)
        });
        fields.add_field_method_set("far", |_, this, val: f32| {
            this.world.borrow_mut().cameras[this.index].far = val;
            Ok(())
        });
    }
}