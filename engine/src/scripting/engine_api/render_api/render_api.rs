use std::cell::RefCell;
use std::sync::Arc;
use mlua::{UserData, UserDataFields};
use crate::render::render::Renderer;
use crate::scripting::engine_api::gui_api::gui_api::{GUIRef};
use crate::scripting::engine_api::render_api::scene_renderer_api::SceneRendererRef;

pub struct RendererRef(pub Arc<RefCell<Renderer>>);
impl UserData for RendererRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("gui", |lua, this| {
            let object = GUIRef(this.0.borrow().gui.clone());
            lua.create_userdata(object)
        });
        fields.add_field_method_get("scene_renderer", |lua, this| {
            let object = SceneRendererRef(this.0.borrow().scene_renderer.clone());
            lua.create_userdata(object)
        });
    }
}