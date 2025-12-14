use crate::engine::EngineRef;
use std::cell::RefCell;
use std::sync::Arc;
use mlua::{UserData, UserDataFields, UserDataMethods};
use crate::math::Vector;
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
        });
        methods.add_method("get_transform", |lua, this, index: usize| {
            Ok(lua.create_userdata(TransformPointer { index }))
        });

        methods.add_method("load_model", |lua, this, parent: usize| {
            with_scene_mut!(lua => scene);

            let file = rfd::FileDialog::new()
                .add_filter("GLTF Models", &["gltf", "glb"])
                .pick_file();
            if let Some(file) = file {
                scene.new_entity_from_model(parent, file.to_str().unwrap());
            }

            Ok(())
        });

        methods.add_method("reset_outlines", |lua, this, ()| {
            with_scene_mut!(lua => scene);
            scene.outlined_components.clear();
            Ok(())
        });
        methods.add_method("add_outlined", |lua, this, index: usize| {
            with_scene_mut!(lua => scene);
            scene.outlined_components.push(index);
            Ok(())
        });
    }
}

pub struct EntityPointer {
    index: usize,
}
impl UserData for EntityPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("index", |_, this| {
           Ok(this.index)
        });

        fields.add_field_method_get("parent_index", |lua, this| {
            with_scene!(lua => scene);
            Ok(scene.entities[this.index].parent)
        });
        fields.add_field_method_get("parent", |lua, this| {
            with_scene!(lua => scene);
            Ok(EntityPointer{ index: scene.entities[this.index].parent })
        });

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
        methods.add_method("get_render_component", |lua, this, index: usize| {
            with_scene!(lua => scene);
            Ok(lua.create_userdata(RenderComponentPointer { index: scene.entities[this.index].render_objects[index] }))
        });

        methods.add_method("get_render_component_index", |lua, this, index: usize| {
            with_scene!(lua => scene);
            Ok(scene.entities[this.index].render_objects[index])
        });
    }
}

pub struct TransformPointer {
    index: usize,
}
impl UserData for TransformPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("index", |lua, this|{
            Ok(this.index)
        });

        fields.add_field_method_get("owner_index", |lua, this| {
            with_scene!(lua => scene);
            Ok(scene.transforms[this.index].owner)
        });
        fields.add_field_method_get("owner", |lua, this| {
            with_scene!(lua => scene);
            Ok(EntityPointer{ index: scene.transforms[this.index].owner })
        });

        fields.add_field_method_get("translation", |lua, this|{
            with_scene!(lua => scene);
            Ok(scene.transforms[this.index].translation)
        });
        fields.add_field_method_get("rotation", |lua, this|{
            with_scene!(lua => scene);
            Ok(scene.transforms[this.index].rotation)
        });
        fields.add_field_method_get("scale", |lua, this|{
            with_scene!(lua => scene);
            Ok(scene.transforms[this.index].scale)
        });

        fields.add_field_method_set("translation", |lua, this, vector: Vector|{
            with_scene_mut!(lua => scene);
            // one level of the hierarchy up, for safety and to make this work properly for editing transforms of render components (non-entity components)
            let owner = scene.entities[scene.transforms[this.index].owner].parent;
            scene.unupdated_entities.push(owner);
            Ok(scene.transforms[this.index].translation = vector)
        });
        fields.add_field_method_set("rotation", |lua, this, vector: Vector|{
            with_scene_mut!(lua => scene);
            // one level of the hierarchy up, for safety and to make this work properly for editing transforms of render components (non-entity components)
            let owner = scene.entities[scene.transforms[this.index].owner].parent;
            scene.unupdated_entities.push(owner);
            Ok(scene.transforms[this.index].rotation = vector)
        });
        fields.add_field_method_set("scale", |lua, this, vector: Vector|{
            with_scene_mut!(lua => scene);
            // one level of the hierarchy up, for safety and to make this work properly for editing transforms of render components (non-entity components)
            let owner = scene.entities[scene.transforms[this.index].owner].parent;
            scene.unupdated_entities.push(owner);
            Ok(scene.transforms[this.index].scale = vector)
        });
    }
}

pub struct RenderComponentPointer {
    index: usize,
}
impl UserData for RenderComponentPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("index", |lua, this|{
            Ok(this.index)
        });
    }
}
