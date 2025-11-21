use mlua::{UserData, UserDataFields};
use crate::app::EngineRef;
use crate::scripting::engine_context::client_context::controller_access::ControllerRef;
use crate::scripting::engine_context::render_context::render_context::RendererRef;

impl UserData for EngineRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("renderer", |lua, this| {
            let object = RendererRef(this.renderer.clone());
            lua.create_userdata(object)
        });
        fields.add_field_method_get("controller", |lua, this| {
            let object = ControllerRef(this.controller.clone());
            lua.create_userdata(object)
        });
    }
}

