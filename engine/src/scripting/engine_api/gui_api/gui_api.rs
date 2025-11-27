use std::cell::RefCell;
use std::sync::Arc;
use mlua::{UserData, UserDataFields, UserDataMethods};
use crate::gui::gui::{GUINode, GUIQuad, GUIText, GUI};
use crate::math::Vector;

pub struct GUITextPointer {
    gui: Arc<RefCell<GUI>>,
    index: usize
}
impl UserData for GUITextPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("text_message", |_, this| Ok(
            this.gui.borrow().gui_texts[this.index].text_information.as_ref().unwrap().text.clone()
        ));
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("update_text", |_, this, text: String| {
            this.gui.borrow_mut().gui_texts[this.index].update_text(text.as_str());
            Ok(())
        });
    }
}

pub struct GUIQuadPointer {
    gui: Arc<RefCell<GUI>>,
    index: usize
}
impl UserData for GUIQuadPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("color", |_, this| Ok(
            this.gui.borrow().gui_quads[this.index].color
        ));
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

pub struct GUINodePointer {
    gui: Arc<RefCell<GUI>>,
    index: usize
}
impl UserData for GUINodePointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("hidden", |_, this| Ok(
            this.gui.borrow().gui_nodes[this.index].hidden
        ));
        fields.add_field_method_set("hidden", |_, this, val: bool| {
            this.gui.borrow_mut().gui_nodes[this.index].hidden = val;
            Ok(())
        });

        fields.add_field_method_get("index", |_, this| Ok(
            this.index
        ));

        fields.add_field_method_get("quad", |lua, this| {
            let quad = GUIQuadPointer {
                gui: this.gui.clone(),
                index: this.gui.borrow().gui_nodes[this.index].quad.unwrap_or(0)
            };
            lua.create_userdata(quad)
        });

        fields.add_field_method_get("text", |lua, this| {
            let text = GUITextPointer {
                gui: this.gui.clone(),
                index: this.gui.borrow().gui_nodes[this.index].text.unwrap_or(0)
            };
            lua.create_userdata(text)
        });

        fields.add_field_method_get("position", |_, this| Ok(
            this.gui.borrow().gui_nodes[this.index].position
        ));
        fields.add_field_method_set("position", |_, this, val: Vector| {
            Ok(this.gui.borrow_mut().gui_nodes[this.index].position = val)
        });

        fields.add_field_method_get("scale", |_, this| Ok(
            this.gui.borrow().gui_nodes[this.index].scale
        ));
        fields.add_field_method_set("scale", |_, this, val: Vector| {
            Ok(this.gui.borrow_mut().gui_nodes[this.index].scale = val)
        });

        fields.add_field_method_get("quad_index", |_, this| Ok(
            this.gui.borrow().gui_nodes[this.index].quad.map_or(-1, |quad| { quad as i32 })
        ));
        fields.add_field_method_set("quad_index", |_, this, val: i32| {
            this.gui.borrow_mut().gui_nodes[this.index].quad = if val >= 0 {
                Some(val as usize)
            } else {
                None
            };
            Ok(())
        });

        fields.add_field_method_get("text_index", |_, this| Ok(
            this.gui.borrow().gui_nodes[this.index].text.map_or(-1, |text| { text as i32 })
        ));
        fields.add_field_method_set("text_index", |_, this, val: i32| {
            this.gui.borrow_mut().gui_nodes[this.index].text = if val >= 0 {
                Some(val as usize)
            } else {
                None
            };
            Ok(())
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_child_index", |_, this, val: i32| {
            Ok(this.gui.borrow().gui_nodes[this.index].children_indices[val as usize])
        });
        methods.add_method_mut("add_child_index", |_, this, val: i32| {
            Ok(this.gui.borrow_mut().gui_nodes[this.index].children_indices.push(val as usize))
        });
        methods.add_method_mut("remove_child_index_at", |_, this, val: i32| {
            Ok(this.gui.borrow_mut().gui_nodes[this.index].children_indices.remove(val as usize))
        });
    }
}

pub struct GUIRef(pub Arc<RefCell<GUI>>);
impl UserData for GUIRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("ActiveNode", |lua, this| {
            let node = GUINodePointer { gui: this.0.clone(), index: this.0.borrow().active_node };
            lua.create_userdata(node)
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_node", |lua, this, index: usize| {
            let node = GUINodePointer { gui: this.0.clone(), index };
            lua.create_userdata(node)
        });

        methods.add_method("get_quad", |lua, this, index: usize| {
            let quad = GUIQuadPointer { gui: this.0.clone(), index };
            lua.create_userdata(quad)
        });

        methods.add_method("get_text", |lua, this, index: usize| {
            let text = GUITextPointer { gui: this.0.clone(), index };
            lua.create_userdata(text)
        });

        methods.add_method("destroy_node", |_, this, index: usize| {
            this.0.borrow_mut().gui_nodes.remove(index);
            Ok(())
        });
        methods.add_method("destroy_quad", |_, this, index: usize| {
            this.0.borrow_mut().gui_quads.remove(index);
            Ok(())
        });
        methods.add_method("destroy_text", |_, this, index: usize| {
            this.0.borrow_mut().gui_texts.remove(index);
            Ok(())
        });

        methods.add_method_mut("add_node", |_, this, ()| {
            let mut borrowed = this.0.borrow_mut();
            let num_nodes = borrowed.gui_nodes.len();
            borrowed.gui_nodes.push(GUINode::new(num_nodes));
            Ok(())
        });
        methods.add_method_mut("add_quad", |_, this, ()| {
            let mut borrowed = this.0.borrow_mut();
            borrowed.gui_quads.push(GUIQuad::default());
            Ok(())
        });
        methods.add_method_mut("add_text", |_, this, initial_text: String| {
            let mut borrowed = this.0.borrow_mut();
            borrowed.add_text(initial_text);
            Ok(())
        });
    }
}