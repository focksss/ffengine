use std::cell::RefCell;
use std::sync::Arc;
use mlua::{UserData, UserDataFields, UserDataMethods};
use crate::scene::scene::Scene;

pub struct SceneRef(pub Arc<RefCell<Scene>>);
impl UserData for SceneRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {

    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_entity", |lua, this, index: usize| {
            Ok(lua.create_userdata(EntityPointer {
                scene: this.0.clone(),
                index
            }))
        });
    }
}

pub struct EntityPointer {
    scene: Arc<RefCell<Scene>>,
    index: usize,
}
impl UserData for EntityPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("children_indices", |lua, this| {
            let scene = this.scene.borrow();
            let children = &scene.entities[this.index].children_indices;
            let table = lua.create_table()?;
            for (i, child_index) in children.iter().enumerate() {
                table.set(i + 1, *child_index)?;
            }
            Ok(table)
        });
        fields.add_field_method_get("name", |_lua, this| { 
            Ok(this.scene.borrow().entities[this.index].name.clone())
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {

    }
}