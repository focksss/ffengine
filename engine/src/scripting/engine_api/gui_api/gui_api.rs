use std::cell::RefCell;
use std::cmp::PartialEq;
use std::sync::Arc;
use mlua::{FromLua, IntoLua, UserData, UserDataFields, UserDataMethods, Value};
use winit::window::CursorIcon;
use crate::gui::gui::{AnchorPoint, GUINode, GUIQuad, GUIText, GUI};
use crate::math::Vector;
use crate::scripting::lua_engine::RegisterToLua;

pub struct GUITextPointer {
    gui: Arc<RefCell<GUI>>,
    index: usize
}
impl UserData for GUITextPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("text_message", |_, this| Ok(
            this.gui.borrow().gui_texts[this.index].text_information.as_ref().unwrap().text.clone()
        ));

        fields.add_field_method_get("font_size", |_, this| {
            Ok(this.gui.borrow().gui_texts[this.index].text_information.as_ref().map_or(-1.0, |t| t.font_size))
        });
        fields.add_field_method_set("font_size", |_, this, val: f32| {
            if let Some(text_info) = this.gui.borrow_mut().gui_texts[this.index].text_information.as_mut() {
                text_info.font_size = val;
            }
            Ok(())
        });
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
        fields.add_field_method_set("color", |lua, this, val: Value| {
            this.gui.borrow_mut().gui_quads[this.index].color = Vector::from_lua(val, lua)?;
            Ok(())
        });

        fields.add_field_method_get("position", |_, this| Ok(
            this.gui.borrow().gui_quads[this.index].position
        ));
        fields.add_field_method_set("position", |lua, this, val: Value| {
            this.gui.borrow_mut().gui_quads[this.index].position = Vector::from_lua(val, lua)?;
            Ok(())
        });

        fields.add_field_method_get("scale", |_, this| Ok(
            this.gui.borrow().gui_quads[this.index].scale
        ));
        fields.add_field_method_set("scale", |lua, this, val: Value| {
            this.gui.borrow_mut().gui_quads[this.index].scale = Vector::from_lua(val, lua)?;
            Ok(())
        });

        fields.add_field_method_get("corner_radius", |_, this| {
            Ok(this.gui.borrow().gui_quads[this.index].corner_radius)
        });
        fields.add_field_method_set("corner_radius", |_, this, val: f32| {
            Ok(this.gui.borrow_mut().gui_quads[this.index].corner_radius = val)
        })
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

        fields.add_field_method_get("absolute_position_x", |_, this| {
            Ok(this.gui.borrow().gui_nodes[this.index].absolute_position.0)
        });
        fields.add_field_method_set("absolute_position_x", |_, this, val: bool| {
            Ok(this.gui.borrow_mut().gui_nodes[this.index].absolute_position.0 = val)
        });
        fields.add_field_method_get("absolute_position_y", |_, this| {
            Ok(this.gui.borrow().gui_nodes[this.index].absolute_position.1)
        });
        fields.add_field_method_set("absolute_position_y", |_, this, val: bool| {
            Ok(this.gui.borrow_mut().gui_nodes[this.index].absolute_position.1 = val)
        });

        fields.add_field_method_get("absolute_scale_x", |_, this| {
            Ok(this.gui.borrow().gui_nodes[this.index].absolute_scale.0)
        });
        fields.add_field_method_set("absolute_scale_x", |_, this, val: bool| {
            Ok(this.gui.borrow_mut().gui_nodes[this.index].absolute_scale.0 = val)
        });
        fields.add_field_method_get("absolute_scale_y", |_, this| {
            Ok(this.gui.borrow().gui_nodes[this.index].absolute_scale.1)
        });
        fields.add_field_method_set("absolute_scale_y", |_, this, val: bool| {
            Ok(this.gui.borrow_mut().gui_nodes[this.index].absolute_scale.1 = val)
        });

        fields.add_field_method_get("index", |_, this| Ok(this.index));

        fields.add_field_method_get("anchor_point", |_, this| {
            Ok(LuaAnchorPoint(this.gui.borrow().gui_nodes[this.index].anchor_point.clone()))
        });

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

        fields.add_field_method_get("children_indices", |lua, this| {
            let scene = this.gui.borrow();
            let children = &scene.gui_nodes[this.index].children_indices;
            let table = lua.create_table()?;
            for (i, child_index) in children.iter().enumerate() {
                table.set(i + 1, *child_index)?;
            }
            Ok(table)
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
        methods.add_method_mut("set_anchor_point", |_, this, val: LuaAnchorPoint| {
            Ok(this.gui.borrow_mut().gui_nodes[this.index].anchor_point = val.0)
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
        fields.add_field_method_get("num_nodes", |lua, this| {
            Ok(this.0.borrow().gui_nodes.len())
        });
        fields.add_field_method_get("num_quads", |lua, this| {
            Ok(this.0.borrow().gui_quads.len())
        });
        fields.add_field_method_get("num_texts", |lua, this| {
            Ok(this.0.borrow().gui_texts.len())
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

pub struct LuaAnchorPoint(pub AnchorPoint);
impl RegisterToLua for LuaAnchorPoint {
    fn register_to_lua(lua: &mlua::Lua) -> mlua::Result<()> {
        let globals = lua.globals();
        let table = lua.create_table()?;
        for (idx, key) in ALL_ANCHOR_POINTS.iter().enumerate() {
            table.set(format!("{:?}", key), idx as u32)?;
        }
        globals.set("AnchorPoint", table)?;
        Ok(())
    }
}
impl PartialEq for AnchorPoint {
    fn eq(&self, other: &Self) -> bool {
        match other {
            AnchorPoint::BottomLeft => { self.eq(&AnchorPoint::BottomLeft) }
            AnchorPoint::BottomMiddle => { self.eq(&AnchorPoint::BottomMiddle) }
            AnchorPoint::BottomRight => { self.eq(&AnchorPoint::BottomRight) }
            AnchorPoint::Center => { self.eq(&AnchorPoint::Center) }
            AnchorPoint::Left => { self.eq(&AnchorPoint::Left) }
            AnchorPoint::Right => { self.eq(&AnchorPoint::Right) }
            AnchorPoint::TopLeft => { self.eq(&AnchorPoint::TopLeft) }
            AnchorPoint::TopMiddle => { self.eq(&AnchorPoint::TopMiddle) }
            AnchorPoint::TopRight => { self.eq(&AnchorPoint::TopRight) }
        }
    }
}
impl<'lua> IntoLua<'lua> for LuaAnchorPoint {
    fn into_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<Value<'lua>> {
        let index = ALL_ANCHOR_POINTS
            .iter()
            .position(|k| *k == self.0)
            .ok_or_else(|| mlua::Error::ToLuaConversionError {
                from: "LuaAnchorPoint",
                to: "Value",
                message: Some("AnchorPoint not found in ALL_ANCHOR_POINTS".into()),
            })?;
        (index as u32).into_lua(lua)
    }
}
impl<'lua> FromLua<'lua> for LuaAnchorPoint {
    fn from_lua(value: Value<'lua>, _: &'lua mlua::Lua) -> mlua::Result<Self> {
        if let Value::Integer(i) = value {
            if i >= 0 && (i as usize) < ALL_ANCHOR_POINTS.len() {
                return Ok(LuaAnchorPoint(ALL_ANCHOR_POINTS[i as usize].clone()));
            }
        }
        Err(mlua::Error::FromLuaConversionError {
            from: value.type_name(),
            to: "LuaAnchorPoint",
            message: Some("invalid AnchorPoint value".into()),
        })
    }
}
const ALL_ANCHOR_POINTS: &[AnchorPoint] = &[
    AnchorPoint::BottomLeft,
    AnchorPoint::BottomMiddle,
    AnchorPoint::BottomRight,
    AnchorPoint::Right,
    AnchorPoint::TopRight,
    AnchorPoint::TopMiddle,
    AnchorPoint::TopLeft,
    AnchorPoint::Left,
    AnchorPoint::Center,
];