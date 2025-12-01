use std::cell::RefCell;
use std::cmp::PartialEq;
use std::sync::Arc;
use mlua::{FromLua, IntoLua, Lua, UserData, UserDataFields, UserDataMethods, Value};
use mlua::prelude::LuaError;
use crate::engine::EngineRef;
use crate::gui::gui::{GUIInteractableInformation, GUI};
use crate::math::Vector;
use crate::scripting::lua_engine::RegisterToLua;

macro_rules! with_gui {
    ($lua:expr, $gui_index:expr => $gui:ident) => {
        let __engine = $lua.app_data_ref::<EngineRef>().unwrap();
        let __renderer = __engine.renderer.borrow();
        let $gui = __renderer.guis[$gui_index].borrow();
    };
}
macro_rules! with_gui_mut {
    ($lua:expr, $gui_index:expr => $gui:ident) => {
        let __engine = $lua.app_data_ref::<EngineRef>().unwrap();
        let __renderer = __engine.renderer.borrow();
        let mut $gui = __renderer.guis[$gui_index].borrow_mut();
    };
}
/*
pub struct GUITextPointer {
    gui_index: usize,
    index: usize
}
impl UserData for GUITextPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("text_message", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_texts[this.index].text_information.as_ref().unwrap().text.clone())
        });

        fields.add_field_method_get("font_size", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_texts[this.index].text_information.as_ref().map_or(-1.0, |t| t.font_size))
        });
        fields.add_field_method_set("font_size", |lua, this, val: f32| {
            with_gui_mut!(lua, this.gui_index => gui);
            if let Some(text_info) = gui.gui_texts[this.index].text_information.as_mut() {
                text_info.font_size = val;
            }
            Ok(())
        });

        fields.add_field_method_get("auto_wrap_distance", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_texts[this.index].text_information.as_ref().map_or(200.0, |t| t.auto_wrap_distance))
        });
        fields.add_field_method_set("auto_wrap_distance", |lua, this, val: f32| {
            with_gui_mut!(lua, this.gui_index => gui);
            if let Some(text_info) = gui.gui_texts[this.index].text_information.as_mut() {
                text_info.auto_wrap_distance = val;
            }
            Ok(())
        });

        fields.add_field_method_get("position", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_texts[this.index].position)
        });
        fields.add_field_method_set("position", |lua, this, val: Vector| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.gui_texts[this.index].position = val)
        });

        fields.add_field_method_get("scale", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_texts[this.index].scale)
        });
        fields.add_field_method_set("scale", |lua, this, val: Vector| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.gui_texts[this.index].scale = val)
        });

        fields.add_field_method_get("absolute_position_x", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_texts[this.index].absolute_position.0)
        });
        fields.add_field_method_set("absolute_position_x", |lua, this, val: bool| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.gui_texts[this.index].absolute_position.0 = val;
            Ok(())
        });
        fields.add_field_method_get("absolute_position_y", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_texts[this.index].absolute_position.1)
        });
        fields.add_field_method_set("absolute_position_y", |lua, this, val: bool| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.gui_texts[this.index].absolute_position.1 = val;
            Ok(())
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("update_text", |lua, this, text: String| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.gui_texts[this.index].update_text(text.as_str());
            Ok(())
        });
    }
}

pub struct GUIQuadPointer {
    gui_index: usize,
    index: usize
}
impl UserData for GUIQuadPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("color", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_quads[this.index].color)
        });
        fields.add_field_method_set("color", |lua, this, val: Value| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.gui_quads[this.index].color = Vector::from_lua(val, lua)?;
            Ok(())
        });

        fields.add_field_method_get("position", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_quads[this.index].position)
        });
        fields.add_field_method_set("position", |lua, this, val: Value| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.gui_quads[this.index].position = Vector::from_lua(val, lua)?;
            Ok(())
        });

        fields.add_field_method_get("scale", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_quads[this.index].scale)
        });
        fields.add_field_method_set("scale", |lua, this, val: Value| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.gui_quads[this.index].scale = Vector::from_lua(val, lua)?;
            Ok(())
        });

        fields.add_field_method_get("absolute_position_x", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_quads[this.index].absolute_position.0)
        });
        fields.add_field_method_set("absolute_position_x", |lua, this, val: bool| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.gui_quads[this.index].absolute_position.0 = val;
            Ok(())
        });
        fields.add_field_method_get("absolute_position_y", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_quads[this.index].absolute_position.1)
        });
        fields.add_field_method_set("absolute_position_y", |lua, this, val: bool| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.gui_quads[this.index].absolute_position.1 = val;
            Ok(())
        });

        fields.add_field_method_get("corner_radius", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_quads[this.index].corner_radius)
        });
        fields.add_field_method_set("corner_radius", |lua, this, val: f32| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.gui_quads[this.index].corner_radius = val)
        })
    }
}

pub struct GUINodePointer {
    gui_index: usize,
    index: usize
}
impl UserData for GUINodePointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("hidden", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].hidden)
        });
        fields.add_field_method_set("hidden", |lua, this, val: bool| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.gui_nodes[this.index].hidden = val;
            Ok(())
        });

        fields.add_field_method_get("absolute_position_x", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].absolute_position.0)
        });
        fields.add_field_method_set("absolute_position_x", |lua, this, val: bool| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.gui_nodes[this.index].absolute_position.0 = val;
            Ok(())
        });
        fields.add_field_method_get("absolute_position_y", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].absolute_position.1)
        });
        fields.add_field_method_set("absolute_position_y", |lua, this, val: bool| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.gui_nodes[this.index].absolute_position.1 = val;
            Ok(())
        });

        fields.add_field_method_get("absolute_scale_x", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].absolute_scale.0)
        });
        fields.add_field_method_set("absolute_scale_x", |lua, this, val: bool| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.gui_nodes[this.index].absolute_scale.0 = val;
            Ok(())
        });
        fields.add_field_method_get("absolute_scale_y", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].absolute_scale.1)
        });
        fields.add_field_method_set("absolute_scale_y", |lua, this, val: bool| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.gui_nodes[this.index].absolute_scale.1 = val;
            Ok(())
        });

        fields.add_field_method_get("index", |lua, this| Ok(this.index));

        fields.add_field_method_get("anchor_point", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(LuaAnchorPoint(gui.gui_nodes[this.index].anchor_point.clone()))
        });

        fields.add_field_method_get("position", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].position)
        });
        fields.add_field_method_set("position", |lua, this, val: Vector| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].position = val)
        });

        fields.add_field_method_get("scale", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].scale)
        });
        fields.add_field_method_set("scale", |lua, this, val: Vector| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].scale = val)
        });

        fields.add_field_method_get("quad_indices", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            let quads = &gui.gui_nodes[this.index].quad_indices;
            let table = lua.create_table()?;
            for (i, quad_index) in quads.iter().enumerate() {
                table.set(i + 1, *quad_index)?;
            }
            Ok(table)
        });
        fields.add_field_method_get("text_indices", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            let texts = &gui.gui_nodes[this.index].text_indices;
            let table = lua.create_table()?;
            for (i, text_index) in texts.iter().enumerate() {
                table.set(i + 1, *text_index)?;
            }
            Ok(table)
        });
        fields.add_field_method_get("children_indices", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            let children = &gui.gui_nodes[this.index].children_indices;
            let table = lua.create_table()?;
            for (i, child_index) in children.iter().enumerate() {
                table.set(i + 1, *child_index)?;
            }
            Ok(table)
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_child_index", |lua, this, val: i32| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].children_indices[val as usize])
        });
        methods.add_method("get_child", |lua, this, val: i32| {
            with_gui!(lua, this.gui_index => gui);
            lua.create_userdata(GUINodePointer {
                gui_index: this.gui_index,
                index: gui.gui_nodes[this.index].children_indices[val as usize]
            })
        });
        methods.add_method_mut("add_child_index", |lua, this, val: i32| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].children_indices.push(val as usize))
        });
        methods.add_method_mut("remove_child_index_at", |lua, this, val: i32| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].children_indices.remove(val as usize))
        });

        methods.add_method("get_quad_index", |lua, this, val: i32| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].quad_indices[val as usize])
        });
        methods.add_method("get_quad", |lua, this, val: i32| {
            with_gui!(lua, this.gui_index => gui);
            lua.create_userdata(GUIQuadPointer {
                gui_index: this.gui_index,
                index: gui.gui_nodes[this.index].quad_indices[val as usize]
            })
        });
        methods.add_method_mut("add_quad_index", |lua, this, val: i32| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].quad_indices.push(val as usize))
        });
        methods.add_method_mut("remove_quad_index_at", |lua, this, val: i32| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].quad_indices.remove(val as usize))
        });

        methods.add_method("get_text_index", |lua, this, val: i32| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].text_indices[val as usize])
        });
        methods.add_method("get_text", |lua, this, val: i32| {
            with_gui!(lua, this.gui_index => gui);
            lua.create_userdata(GUITextPointer {
                gui_index: this.gui_index,
                index: gui.gui_nodes[this.index].text_indices[val as usize]
            })
        });
        methods.add_method_mut("add_text_index", |lua, this, val: i32| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].text_indices.push(val as usize))
        });
        methods.add_method_mut("remove_text_index_at", |lua, this, val: i32| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].text_indices.remove(val as usize))
        });

        methods.add_method_mut("set_anchor_point", |lua, this, val: LuaAnchorPoint| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.gui_nodes[this.index].anchor_point = val.0)
        });
        methods.add_method_mut("add_left_tap_action", |lua, this, val: (String, usize)| {
            with_gui_mut!(lua, this.gui_index => gui);
            let script_index = gui.script_indices[val.1];
            let node = &mut gui.gui_nodes[this.index];
            if let Some(interactable_information) = &mut node.interactable_information {
                interactable_information.left_tap_actions.push((val.0, script_index));
            } else {
                let mut new_interactable_information = GUIInteractableInformation::default();
                new_interactable_information.left_tap_actions.push((val.0, script_index));
                node.interactable_information = Some(new_interactable_information);
            }
            Ok(())
        })
    }
}
*/
pub struct GUIPointer {
    pub(crate) index: usize
}
impl UserData for GUIPointer {
    /*
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("ActiveNode", |lua, this| {
            with_gui!(lua, this.index => gui);

            let node = GUINodePointer {
                gui_index: this.index,
                index: gui.active_node
            };
            lua.create_userdata(node)
        });

        fields.add_field_method_get("num_nodes", |lua, this| {
            with_gui!(lua, this.index => gui);
            Ok(gui.gui_nodes.len())
        });
        fields.add_field_method_get("num_quads", |lua, this| {
            with_gui!(lua, this.index => gui);
            Ok(gui.gui_quads.len())
        });
        fields.add_field_method_get("num_texts", |lua, this| {
            with_gui!(lua, this.index => gui);
            Ok(gui.gui_texts.len())
        });

        fields.add_field_method_get("root_node_indices", |lua, this| {
            with_gui!(lua, this.index => gui);
            let roots = &gui.gui_root_node_indices;
            let table = lua.create_table()?;
            for (i, root_index) in roots.iter().enumerate() {
                table.set(i + 1, *root_index)?;
            }
            Ok(table)
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("is_node_hovered", |lua, this, node_index: usize| {
            with_gui!(lua, this.index => gui);
            Ok(gui.hovered_nodes.contains(&node_index))
        });

        methods.add_method("get_root_index", |lua, this, val: i32| {
            with_gui!(lua, this.index => gui);
            Ok(gui.gui_root_node_indices[val as usize])
        });
        methods.add_method("get_root", |lua, this, val: i32| {
            with_gui!(lua, this.index => gui);
            lua.create_userdata(GUINodePointer {
                gui_index: this.index,
                index: gui.gui_root_node_indices[val as usize]
            })
        });
        methods.add_method_mut("add_root_index", |lua, this, val: i32| {
            with_gui_mut!(lua, this.index => gui);
            Ok(gui.gui_root_node_indices.push(val as usize))
        });
        methods.add_method_mut("remove_root_index_at", |lua, this, val: i32| {
            with_gui_mut!(lua, this.index => gui);
            Ok(gui.gui_root_node_indices.remove(val as usize))
        });

        methods.add_method("get_node", |lua, this, index: usize| {
            let node = GUINodePointer { gui_index: this.index, index };
            lua.create_userdata(node)
        });
        methods.add_method("get_quad", |lua, this, index: usize| {
            let quad = GUIQuadPointer { gui_index: this.index, index };
            lua.create_userdata(quad)
        });
        methods.add_method("get_text", |lua, this, index: usize| {
            let text = GUITextPointer { gui_index: this.index, index };
            lua.create_userdata(text)
        });

        methods.add_method("destroy_node", |lua, this, index: usize| {
            with_gui_mut!(lua, this.index => gui);
            gui.gui_nodes.remove(index);
            Ok(())
        });
        methods.add_method("destroy_quad", |lua, this, index: usize| {
            with_gui_mut!(lua, this.index => gui);
            gui.gui_quads.remove(index);
            Ok(())
        });

        methods.add_method_mut("add_node", |lua, this, ()| {
            with_gui_mut!(lua, this.index => gui);
            let num_nodes = gui.gui_nodes.len();
            gui.gui_nodes.push(GUINode::new(num_nodes));
            Ok(())
        });
        methods.add_method_mut("add_quad", |lua, this, ()| {
            with_gui_mut!(lua, this.index => gui);
            gui.gui_quads.push(GUIQuad::default());
            Ok(())
        });
        methods.add_method_mut("add_text", |lua, this, initial_text: String| {
            with_gui_mut!(lua, this.index => gui);
            gui.add_text(initial_text);
            Ok(())
        });
    }
     */
}
/*
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
*/