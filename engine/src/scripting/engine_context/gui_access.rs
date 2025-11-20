use std::cell::RefCell;
use std::sync::Arc;
use std::time::Instant;
use ash::vk::CommandBuffer;
use mlua::{UserData, UserDataFields, UserDataMethods};
use crate::gui::gui::{GUINode, GUIQuad, GUIText, GUI};
use crate::math::Vector;
use crate::scripting::lua_engine::Lua;

pub struct GUITextRef(pub Arc<RefCell<GUIText>>);
impl UserData for GUITextRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("text_message", |_, this| Ok(this.0.borrow().text_information.text.clone()));
    }
}


pub struct GUIQuadRef(pub Arc<RefCell<GUIQuad>>);
impl UserData for GUIQuadRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("color", |_, this| Ok(this.0.borrow().color.clone()));
        fields.add_field_method_set("color", |_, this, val: Vector| {
            this.0.borrow_mut().color = val;
            Ok(())
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("set_color", |_, this, color: (f32, f32, f32, f32)| {
            this.0.borrow_mut().color = Vector::new_vec4(color.0, color.1, color.2, color.3);
            Ok(())
        });
    }
}


pub struct GUINodeRef(pub Arc<RefCell<GUINode>>);
impl UserData for GUINodeRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("hidden", |_, this| Ok(this.0.borrow().hidden.clone()));
        fields.add_field_method_set("hidden", |_, this, val: bool| {
            this.0.borrow_mut().hidden = val;
            Ok(())
        });

        fields.add_field_method_get("index", |_, this| Ok(this.0.borrow().index.clone()));

        fields.add_field_method_get("quad", |_, this| Ok(this.0.borrow().quad.unwrap_or(0).clone()));

        fields.add_field_method_get("text", |_, this| Ok(this.0.borrow().text.unwrap_or(0).clone()));
    }
}

pub struct ScriptGUI<'a> {
    pub(crate) gui: &'a mut GUI,
    pub(crate) command_buffer: CommandBuffer,
}
impl<'a> UserData for ScriptGUI<'a> {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("update_text_of_node", |_, this, (node_index, text): (usize, String)| {
            this.gui.update_text_of_node(node_index, &text, this.command_buffer);
            Ok(())
        });
        methods.add_method("get_node", |lua, this, index: usize| {
            let node = GUINodeRef(this.gui.gui_nodes[index].clone());
            lua.create_userdata(node)
        });

        methods.add_method_mut("set_quad_color", |_, this, (quad_index, r, g, b, a): (usize, f32, f32, f32, f32)| {
            Ok(this.gui.gui_quads[quad_index].borrow_mut()
                .color = Vector::new_vec4(r, g, b, a)
            )
        });

        methods.add_method("get_quad", |lua, this, index: usize| {
            let quad = GUIQuadRef(this.gui.gui_quads[index].clone());
            lua.create_userdata(quad)
        });

        methods.add_method("get_text", |lua, this, index: usize| {
            let text = GUITextRef(this.gui.gui_texts[index].clone());
            lua.create_userdata(text)
        });
    }
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("ActiveNode", |_, this| Ok(this.gui.active_node));
    }
}