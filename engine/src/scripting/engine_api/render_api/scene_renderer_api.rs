use std::cell::RefCell;
use std::sync::Arc;
use ash::vk;
use mlua::{UserData, UserDataFields, UserDataMethods};
use crate::render::scene_renderer::SceneRenderer;

struct ViewportRef(pub Arc<RefCell<vk::Viewport>>);
impl UserData for ViewportRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| {
            Ok(this.0.borrow().x)
        });
        fields.add_field_method_set("x", |_, this, v| {
            Ok(this.0.borrow_mut().x = v)
        });

        fields.add_field_method_get("y", |_, this| {
            Ok(this.0.borrow().y)
        });
        fields.add_field_method_set("y", |_, this, v| {
            Ok(this.0.borrow_mut().y = v)
        });

        fields.add_field_method_get("width", |_, this| {
            Ok(this.0.borrow().width)
        });
        fields.add_field_method_set("width", |_, this, v: f32| {
            Ok(this.0.borrow_mut().width = v.max(1.0))
        });

        fields.add_field_method_get("height", |_, this| {
            Ok(this.0.borrow().height)
        });
        fields.add_field_method_set("height", |_, this, v: f32| {
            Ok(this.0.borrow_mut().height = v.max(1.0))
        });
    }
}

pub struct SceneRendererRef(pub Arc<RefCell<SceneRenderer>>);
impl UserData for SceneRendererRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("viewport", |lua, this| {
            let object = ViewportRef(this.0.borrow().viewport.clone());
            lua.create_userdata(object)
        });
        
        fields.add_field_method_get("hovered_id", |lua, this| {
            Ok(this.0.borrow().hovered_component_id)
        });
    }
}