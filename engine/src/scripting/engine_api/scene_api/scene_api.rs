use crate::engine::EngineRef;
use std::cell::RefCell;
use std::sync::Arc;
use mlua::{UserData, UserDataFields, UserDataMethods};
use crate::scene::scene::Scene;

macro_rules! with_scene {
    ($lua:expr => $scene:ident) => {
        let __engine = $lua.app_data_ref::<EngineRef>().unwrap();
        let $scene = __engine.scene.borrow();
    };
}
macro_rules! with_scene_mut {
    ($lua:expr => $scene:ident) => {
        let __engine = $lua.app_data_ref::<EngineRef>().unwrap();
        let mut $scene = __engine.scene.borrow_mut();
    };
}

pub struct SceneRef;
impl UserData for SceneRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {

    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_entity", |lua, this, index: usize| {
            Ok(lua.create_userdata(EntityPointer {
                index
            }))
        });
        methods.add_method("get_render_component", |lua, this, index: usize| {
            Ok(lua.create_userdata(RenderComponentPointer { index }))
        })
    }
}

pub struct EntityPointer {
    index: usize,
}
impl UserData for EntityPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("transform_index", |lua, this| {
            with_scene!(lua => scene);
            Ok(scene.entities[this.index].transform)
        });

        fields.add_field_method_get("children_indices", |lua, this| {
            with_scene!(lua => scene);
            let children = &scene.entities[this.index].children_indices;
            let table = lua.create_table()?;
            for (i, child_index) in children.iter().enumerate() {
                table.set(i + 1, *child_index)?;
            }
            Ok(table)
        });

        fields.add_field_method_get("render_component_indices", |lua, this| {
            with_scene!(lua => scene);
            let render_components = &scene.entities[this.index].render_objects;
            let table = lua.create_table()?;
            for (i, element_index) in render_components.iter().enumerate() {
                table.set(i + 1, *element_index)?;
            }
            Ok(table)
        });

        fields.add_field_method_get("name", |lua, this| {
            with_scene!(lua => scene);
            Ok(scene.entities[this.index].name.clone())
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {

    }
}

pub struct TransformPointer {
    index: usize,
}
impl UserData for TransformPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {

    }
}

pub struct RenderComponentPointer {
    index: usize,
}
impl UserData for RenderComponentPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {

    }
}
