use mlua::{UserData, UserDataFields, UserDataMethods};
use crate::engine::EngineRef;
use crate::scripting::engine_api::client_api::client_api::ClientRef;
use crate::scripting::engine_api::scene_api::physics_api::physics_engine_api::PhysicsEngineRef;
use crate::scripting::engine_api::render_api::render_api::RendererRef;
use crate::scripting::engine_api::scene_api::scene_api::SceneRef;
//use crate::scripting::engine_api::world_api::scene_api::SceneRef;
use crate::scripting::lua_engine::Lua;

impl UserData for EngineRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("renderer", |lua, this| {
            let object = RendererRef(this.renderer.clone());
            lua.create_userdata(object)
        });
        fields.add_field_method_get("client", |lua, this| {
            let object = ClientRef(this.client.clone());
            lua.create_userdata(object)
        });
        fields.add_field_method_get("physics_engine", |lua, this| {
            let object = PhysicsEngineRef(this.physics_engine.clone());
            lua.create_userdata(object)
        });
        fields.add_field_method_get("scene", |lua, this| {
             let object = SceneRef(this.scene.clone());
             lua.create_userdata(object)
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("queue_script_reload", |lua, this, ()| {
            Lua::reload_scripts();
            Ok(())
        })
    }
}

