use std::cell::RefCell;
use std::sync::Arc;
use mlua::{UserData, UserDataFields};
use crate::render::render::Renderer;
use crate::scripting::engine_context::gui_context::gui_access::{GUIRef};

pub struct RendererRef(pub Arc<RefCell<Renderer>>);
impl UserData for RendererRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("GUI", |lua, this| {
            let object = GUIRef(this.0.borrow().gui.clone());
            lua.create_userdata(object)
        });
    }
}