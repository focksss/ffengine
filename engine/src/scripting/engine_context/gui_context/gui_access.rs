use std::cell::RefCell;
use std::sync::Arc;
use std::time::Instant;
use ash::vk::CommandBuffer;
use mlua::{UserData, UserDataFields, UserDataMethods};
use crate::gui::gui::{GUINode, GUIQuad, GUIText, GUI};
use crate::math::Vector;
use crate::scripting::lua_engine::Lua;

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
            this.gui.borrow_mut().gui_quads[this.index].color = Vector::new_vec4(color.0, color.1, color.2, color.3);
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
    }
}