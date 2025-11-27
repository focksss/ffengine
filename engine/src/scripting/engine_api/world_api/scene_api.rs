use std::cell::RefCell;
use std::sync::Arc;
use mlua::{UserData, UserDataFields, UserDataMethods};
use crate::math::Vector;
use crate::world::scene::World;

/*
pub struct SceneRef(pub Arc<RefCell<Scene>>);
impl UserData for SceneRef {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_object", |lua, this, index: usize| {
            let node = SceneNodePointer { scene: this.0.clone(), index };
            lua.create_userdata(node)
        });
    }
}

struct SceneObjectPointer {
    scene: Arc<RefCell<Scene>>,
    index: usize
}

struct SceneNodePointer {
    scene: Arc<RefCell<Scene>>,
    index: usize
}
impl UserData for SceneNodePointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
            fields.add_field_method_get("color", |lua, this| {
            let scene = this.scene.borrow();
            let children = &scene.models[this.index].children_indices;

            let table = lua.create_table()?;
            for (i, &child_index) in children.iter().enumerate() {
                let child_ptr = GUINodePointer {
                    gui: this.gui.clone(),
                    index: child_index,
                };
                table.set(i + 1, lua.create_userdata(child_ptr)?)?;
            }
            Ok(table)
        });
        fields.add_field_method_set("color", |_, this, val: Vector| {
            this.gui.borrow_mut().gui_quads[this.index].color = val;
            Ok(())
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("set_color", |_, this, color: (f32, f32, f32, f32)| {
            this.gui.borrow_mut().gui_quads[this.index].color = Vector::new4(color.0, color.1, color.2, color.3);
            Ok(())
        });
    }
}
 */