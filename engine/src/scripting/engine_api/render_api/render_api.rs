use std::cell::RefCell;
use std::sync::Arc;
use mlua::{UserData, UserDataFields, UserDataMethods};
use crate::render::render::Renderer;
use crate::scripting::engine_api::gui_api::gui_api::GUIPointer;
use crate::scripting::engine_api::render_api::scene_renderer_api::SceneRendererRef;

pub struct RendererRef(pub Arc<RefCell<Renderer>>);
impl UserData for RendererRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("scene_renderer", |lua, this| {
            let object = SceneRendererRef(this.0.borrow().scene_renderer.clone());
            lua.create_userdata(object)
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("gui", |lua, _, index: usize| {
            let object = GUIPointer { index };
            lua.create_userdata(object)
        })
    }
}