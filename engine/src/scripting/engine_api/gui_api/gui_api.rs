use std::cell::RefCell;
use std::cmp::PartialEq;
use std::sync::Arc;
use mlua::{FromLua, IntoLua, Lua, UserData, UserDataFields, UserDataMethods, Value};
use mlua::prelude::LuaError;
use crate::engine::{get_command_buffer, EngineRef};
use crate::gui::gui::{AnchorPoint, Element, GUIInteractableInformation, Node, Offset, ParentRelation, Size, GUI};
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

pub struct GUITextPointer {
    gui_index: usize,
    index: usize
}
impl UserData for GUITextPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("text_message", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            match &gui.elements[this.index] {
                Element::Text { text_information, .. } => {
                    Ok(text_information.as_ref().unwrap().text.clone())
                }
                _ => Err(mlua::Error::runtime("Element is not a text"))
            }
        });

        fields.add_field_method_get("font_size", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            match &gui.elements[this.index] {
                Element::Text { text_information, .. } => {
                    Ok(text_information.as_ref().map_or(-1.0, |t| t.font_size))
                }
                _ => Err(mlua::Error::runtime("Element is not a text"))
            }
        });
        fields.add_field_method_set("font_size", |lua, this, val: f32| {
            with_gui_mut!(lua, this.gui_index => gui);
            match &mut gui.elements[this.index] {
                Element::Text { text_information, .. } => {
                    if let Some(text_info) = text_information.as_mut() {
                        text_info.font_size = val;
                    }
                    Ok(())
                }
                _ => Err(mlua::Error::runtime("Element is not a text"))
            }
        });

        fields.add_field_method_get("auto_wrap_distance", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            match &gui.elements[this.index] {
                Element::Text { text_information, .. } => {
                    Ok(text_information.as_ref().map_or(-1.0, |t| t.auto_wrap_distance))
                }
                _ => Err(mlua::Error::runtime("Element is not a text"))
            }
        });
        fields.add_field_method_set("auto_wrap_distance", |lua, this, val: f32| {
            with_gui_mut!(lua, this.gui_index => gui);
            match &mut gui.elements[this.index] {
                Element::Text { text_information, .. } => {
                    if let Some(text_info) = text_information.as_mut() {
                        text_info.auto_wrap_distance = val;
                    }
                    Ok(())
                }
                _ => Err(mlua::Error::runtime("Element is not a text"))
            }
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("update_text", |lua, this, text: String| {
            with_gui_mut!(lua, this.gui_index => gui);
            match &mut gui.elements[this.index] {
                Element::Text { text_information, .. } => {
                    if let Some(text_info) = text_information.as_mut() {
                        let command_buffer = get_command_buffer();

                        text_information.as_mut().unwrap().update_text(text.as_str());
                        text_information.as_mut().unwrap().update_buffers_all_frames(command_buffer);
                    }
                    Ok(())
                }
                _ => Err(mlua::Error::runtime("Element is not a text"))
            }
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
            match &gui.elements[this.index] {
                Element::Quad { color, .. } => {
                    Ok(color.clone())
                }
                _ => Err(mlua::Error::runtime("Element is not a quad"))
            }
        });
        fields.add_field_method_set("color", |lua, this, val: Value| {
            with_gui_mut!(lua, this.gui_index => gui);
            match &mut gui.elements[this.index] {
                Element::Quad { color: quad_color, .. } => {
                    *quad_color = Vector::from_lua(val, lua)?;
                    Ok(())
                }
                _ => Err(mlua::Error::runtime("Element is not a quad"))
            }
        });

        fields.add_field_method_get("corner_radius", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            match &gui.elements[this.index] {
                Element::Quad { corner_radius, .. } => {
                    Ok(*corner_radius)
                }
                _ => Err(mlua::Error::runtime("Element is not a quad"))
            }
        });
        fields.add_field_method_set("corner_radius", |lua, this, val: f32| {
            with_gui_mut!(lua, this.gui_index => gui);
            match &mut gui.elements[this.index] {
                Element::Quad { corner_radius, .. } => {
                    *corner_radius = val;
                    Ok(())
                }
                _ => Err(mlua::Error::runtime("Element is not a quad"))
            }
        })
    }
}

pub struct GUIImagePointer {
    gui_index: usize,
    index: usize
}
impl UserData for GUIImagePointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("additive_tint", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            match &gui.elements[this.index] {
                Element::Image { additive_tint, .. } => {
                    Ok(additive_tint.clone())
                }
                _ => Err(mlua::Error::runtime("Element is not an image"))
            }
        });
        fields.add_field_method_set("additive_tint", |lua, this, val: Value| {
            with_gui_mut!(lua, this.gui_index => gui);
            match &mut gui.elements[this.index] {
                Element::Image { additive_tint: quad_additive_tint, .. } => {
                    *quad_additive_tint = Vector::from_lua(val, lua)?;
                    Ok(())
                }
                _ => Err(mlua::Error::runtime("Element is not an image"))
            }
        });

        fields.add_field_method_get("multiplicative_tint", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            match &gui.elements[this.index] {
                Element::Image { multiplicative_tint, .. } => {
                    Ok(multiplicative_tint.clone())
                }
                _ => Err(mlua::Error::runtime("Element is not an image"))
            }
        });
        fields.add_field_method_set("multiplicative_tint", |lua, this, val: Value| {
            with_gui_mut!(lua, this.gui_index => gui);
            match &mut gui.elements[this.index] {
                Element::Image { multiplicative_tint: quad_multiplicative_tint, .. } => {
                    *quad_multiplicative_tint = Vector::from_lua(val, lua)?;
                    Ok(())
                }
                _ => Err(mlua::Error::runtime("Element is not an image"))
            }
        });

        fields.add_field_method_get("index", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            match &gui.elements[this.index] {
                Element::Image { index, .. } => {
                    Ok(*index)
                }
                _ => Err(mlua::Error::runtime("Element is not an image"))
            }
        });
        fields.add_field_method_set("index", |lua, this, val: usize| {
            with_gui_mut!(lua, this.gui_index => gui);
            match &mut gui.elements[this.index] {
                Element::Image { index, .. } => {
                    *index = val;
                    Ok(())
                }
                _ => Err(mlua::Error::runtime("Element is not an image"))
            }
        });

        fields.add_field_method_get("corner_radius", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            match &gui.elements[this.index] {
                Element::Image { corner_radius, .. } => {
                    Ok(*corner_radius)
                }
                _ => Err(mlua::Error::runtime("Element is not an image"))
            }
        });
        fields.add_field_method_set("corner_radius", |lua, this, val: f32| {
            with_gui_mut!(lua, this.gui_index => gui);
            match &mut gui.elements[this.index] {
                Element::Image { corner_radius, .. } => {
                    *corner_radius = val;
                    Ok(())
                }
                _ => Err(mlua::Error::runtime("Element is not an image"))
            }
        });
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
            Ok(gui.nodes[this.index].hidden)
        });
        fields.add_field_method_set("hidden", |lua, this, val: bool| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.nodes[this.index].hidden = val;
            Ok(())
        });

        fields.add_field_method_get("index", |lua, this| Ok(this.index));

        fields.add_field_method_get("children_indices", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            let children = &gui.nodes[this.index].children_indices;
            let table = lua.create_table()?;
            for (i, child_index) in children.iter().enumerate() {
                table.set(i + 1, *child_index)?;
            }
            Ok(table)
        });
        fields.add_field_method_get("element_indices", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            let elements = &gui.nodes[this.index].element_indices;
            let table = lua.create_table()?;
            for (i, element_index) in elements.iter().enumerate() {
                table.set(i + 1, *element_index)?;
            }
            Ok(table)
        });

        fields.add_field_method_get("position", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.nodes[this.index].position)
        });
        fields.add_field_method_get("size", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.nodes[this.index].size)
        });
        fields.add_field_method_get("clip_min", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.nodes[this.index].clip_min)
        });
        fields.add_field_method_get("clip_max", |lua, this| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.nodes[this.index].clip_max)
        });
    }
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("reset", |lua, this, ()| {
            with_gui_mut!(lua, this.gui_index => gui);
            let original_parent = gui.nodes[this.index].parent_index;
            let original_index = gui.nodes[this.index].index;
            Ok(gui.nodes[this.index] = Node::new(original_index, original_parent))
        });

        methods.add_method("get_parent", |lua, this, ()| {
            with_gui!(lua, this.gui_index => gui);
            lua.create_userdata(GUINodePointer {
                gui_index: this.gui_index,
                index: gui.nodes[this.index].parent_index.unwrap_or(0)
            })
        });
        methods.add_method("get_child_index", |lua, this, val: i32| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.nodes[this.index].children_indices[val as usize])
        });
        methods.add_method("get_child", |lua, this, val: i32| {
            with_gui!(lua, this.gui_index => gui);
            lua.create_userdata(GUINodePointer {
                gui_index: this.gui_index,
                index: gui.nodes[this.index].children_indices[val as usize]
            })
        });
        methods.add_method_mut("add_child_index", |lua, this, val: i32| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.nodes[this.index].children_indices.push(val as usize))
        });
        methods.add_method_mut("remove_child_index_at", |lua, this, val: i32| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.nodes[this.index].children_indices.remove(val as usize))
        });
        methods.add_method_mut("clear_children", |lua, this, ()| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.nodes[this.index].children_indices.clear())
        });

        methods.add_method("get_element_index", |lua, this, val: i32| {
            with_gui!(lua, this.gui_index => gui);
            Ok(gui.nodes[this.index].element_indices[val as usize])
        });
        methods.add_method("get_element", |lua, this, val: i32| {
            with_gui!(lua, this.gui_index => gui);
            let element_index = gui.nodes[this.index].element_indices[val as usize];
            match gui.elements[element_index] {
                Element::Image { .. } => {
                    lua.create_userdata(GUIImagePointer {
                        gui_index: this.gui_index,
                        index: element_index
                    })
                },
                Element::Text { .. } => {
                    lua.create_userdata(GUITextPointer {
                        gui_index: this.gui_index,
                        index: element_index
                    })
                },
                Element::Quad { .. } => {
                    lua.create_userdata(GUIQuadPointer {
                        gui_index: this.gui_index,
                        index: element_index
                    })
                }
            }
        });
        methods.add_method("get_image_at", |lua, this, val: i32| {
            with_gui!(lua, this.gui_index => gui);
            let element_index = gui.nodes[this.index].element_indices[val as usize];
            match gui.elements[element_index] {
                Element::Image { .. } => {
                    lua.create_userdata(GUIImagePointer {
                        gui_index: this.gui_index,
                        index: element_index
                    })
                },
                _ => Err(mlua::Error::runtime("tried to get_image_at on a non image element"))
            }
        });
        methods.add_method("get_quad_at", |lua, this, val: i32| {
            with_gui!(lua, this.gui_index => gui);
            let element_index = gui.nodes[this.index].element_indices[val as usize];
            match gui.elements[element_index] {
                Element::Quad { .. } => {
                    lua.create_userdata(GUIQuadPointer {
                        gui_index: this.gui_index,
                        index: element_index
                    })
                },
                _ => Err(mlua::Error::runtime("tried to get_quad_at on a non quad element"))
            }
        });
        methods.add_method("get_text_at", |lua, this, val: i32| {
            with_gui!(lua, this.gui_index => gui);
            let element_index = gui.nodes[this.index].element_indices[val as usize];
            match gui.elements[element_index] {
                Element::Text { .. } => {
                    lua.create_userdata(GUITextPointer {
                        gui_index: this.gui_index,
                        index: element_index
                    })
                },
                _ => Err(mlua::Error::runtime("tried to get_text_at on a non text element"))
            }
        });
        methods.add_method_mut("add_element_index", |lua, this, val: i32| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.nodes[this.index].element_indices.push(val as usize))
        });
        methods.add_method_mut("set_element_index_at_to", |lua, this, vals: (i32, i32)| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.nodes[this.index].element_indices[vals.0 as usize] = vals.1 as usize)
        });
        methods.add_method_mut("remove_element_index_at", |lua, this, val: i32| {
            with_gui_mut!(lua, this.gui_index => gui);
            Ok(gui.nodes[this.index].element_indices.remove(val as usize))
        });

        methods.add_method_mut("add_left_tap_action", |lua, this, val: (String, usize)| {
            with_gui_mut!(lua, this.gui_index => gui);
            let script_index = gui.script_indices[val.1];
            let node = &mut gui.nodes[this.index];
            if let Some(interactable_information) = &mut node.interactable_information {
                interactable_information.left_tap_actions.push((val.0, script_index));
            } else {
                let mut new_interactable_information = GUIInteractableInformation::default();
                new_interactable_information.left_tap_actions.push((val.0, script_index));
                node.interactable_information = Some(new_interactable_information);
            }
            Ok(())
        });

        methods.add_method_mut("set_width", |lua, this, val: (String, f32)| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.nodes[this.index].width = match val.0.as_str() {
                "Factor" => Size::Factor(val.1),
                "Auto" => Size::Auto,
                "Absolute" => Size::Absolute(val.1),
                "FillFactor" => Size::FillFactor(val.1),
                _ => panic!("Invalid size_type: {}", val.1)
            };
            Ok(())
        });
        methods.add_method_mut("set_height", |lua, this, val: (String, f32)| {
            with_gui_mut!(lua, this.gui_index => gui);
            gui.nodes[this.index].height = match val.0.as_str() {
                "Factor" => Size::Factor(val.1),
                "Auto" => Size::Auto,
                "Absolute" => Size::Absolute(val.1),
                "FillFactor" => Size::FillFactor(val.1),
                _ => panic!("Invalid size_type: {}", val.1)
            };
            Ok(())
        });

        methods.add_method_mut("set_anchor_point", |lua, this, val: LuaAnchorPoint| {
            with_gui_mut!(lua, this.gui_index => gui);
            let node = &mut gui.nodes[this.index];

            let new_relation = ParentRelation::Independent {
                relative: true,
                anchor: val.0.clone(),
                offset_x: Offset::Pixels(0.0),
                offset_y: Offset::Pixels(0.0),
            };

            match &mut node.parent_relation {
                Some(ParentRelation::Independent { anchor, .. }) => {
                    *anchor = val.0;
                }
                Some(_) | None => {
                    node.parent_relation = Some(new_relation);
                }
            }

            Ok(())
        });

        methods.add_method_mut("set_x", |lua, this, vals: (String, f32)| {
            with_gui_mut!(lua, this.gui_index => gui);
            let node = &mut gui.nodes[this.index];

            let offset = match vals.0.as_str() {
                "Pixels" => Offset::Pixels(vals.1),
                "Factor" => Offset::Factor(vals.1),
                _ => panic!("Invalid offset_type: {}", vals.1)
            };

            let new_relation = ParentRelation::Independent {
                relative: true,
                anchor: AnchorPoint::default(),
                offset_x: offset.clone(),
                offset_y: Offset::Pixels(0.0),
            };

            match &mut node.parent_relation {
                Some(ParentRelation::Independent { offset_x, .. }) => {
                    *offset_x = offset;
                }
                Some(_) | None => {
                    node.parent_relation = Some(new_relation);
                }
            }

            Ok(())
        });
        methods.add_method_mut("set_y", |lua, this, vals: (String, f32)| {
            with_gui_mut!(lua, this.gui_index => gui);
            let node = &mut gui.nodes[this.index];

            let offset = match vals.0.as_str() {
                "Pixels" => Offset::Pixels(vals.1),
                "Factor" => Offset::Factor(vals.1),
                _ => panic!("Invalid offset_type: {}", vals.1)
            };

            let new_relation = ParentRelation::Independent {
                relative: true,
                anchor: AnchorPoint::default(),
                offset_x: Offset::Pixels(0.0),
                offset_y: offset.clone(),
            };

            match &mut node.parent_relation {
                Some(ParentRelation::Independent { offset_y, .. }) => {
                    *offset_y = offset;
                }
                Some(_) | None => {
                    node.parent_relation = Some(new_relation);
                }
            }

            Ok(())
        });
    }
}

pub struct GUIPointer {
    pub(crate) index: usize
}
impl UserData for GUIPointer {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("ActiveNode", |lua, this| {
            with_gui!(lua, this.index => gui);

            let node = GUINodePointer {
                gui_index: this.index,
                index: gui.active_node
            };
            lua.create_userdata(node)
        });

        fields.add_field_method_get("num_elements", |lua, this| {
            with_gui!(lua, this.index => gui);
            Ok(gui.elements.len())
        });
        fields.add_field_method_get("num_nodes", |lua, this| {
            with_gui!(lua, this.index => gui);
            Ok(gui.nodes.len())
        });

        fields.add_field_method_get("root_node_indices", |lua, this| {
            with_gui!(lua, this.index => gui);
            let roots = &gui.root_node_indices;
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
            Ok(gui.root_node_indices[val as usize])
        });
        methods.add_method("get_root", |lua, this, val: i32| {
            with_gui!(lua, this.index => gui);
            lua.create_userdata(GUINodePointer {
                gui_index: this.index,
                index: gui.root_node_indices[val as usize]
            })
        });
        methods.add_method_mut("add_root_index", |lua, this, val: i32| {
            with_gui_mut!(lua, this.index => gui);
            Ok(gui.root_node_indices.push(val as usize))
        });
        methods.add_method_mut("remove_root_index_at", |lua, this, val: i32| {
            with_gui_mut!(lua, this.index => gui);
            Ok(gui.root_node_indices.remove(val as usize))
        });

        methods.add_method("get_node", |lua, this, index: usize| {
            let node = GUINodePointer { gui_index: this.index, index };
            lua.create_userdata(node)
        });
        methods.add_method("get_quad", |lua, this, index: usize| {
            with_gui!(lua, this.index => gui);
            if let Element::Quad { .. } = gui.elements[index] {
                let quad = GUIQuadPointer { gui_index: this.index, index };
                lua.create_userdata(quad)
            } else {
                Err(mlua::Error::runtime("attempted to get_quad on a non quad element"))
            }
        });
        methods.add_method("get_text", |lua, this, index: usize| {
            with_gui!(lua, this.index => gui);
            if let Element::Text { .. } = gui.elements[index] {
                let text = GUITextPointer { gui_index: this.index, index };
                lua.create_userdata(text)
            } else {
                Err(mlua::Error::runtime("attempted to get_text on a non text element"))
            }
        });
        methods.add_method("get_image", |lua, this, index: usize| {
            with_gui!(lua, this.index => gui);
            if let Element::Image { .. } = gui.elements[index] {
                let image = GUIImagePointer { gui_index: this.index, index };
                lua.create_userdata(image)
            } else {
                Err(mlua::Error::runtime("attempted to get_image on a non image element"))
            }
        });

        methods.add_method_mut("add_node", |lua, this, parent_index: i32| {
            with_gui_mut!(lua, this.index => gui);
            let num_nodes = gui.nodes.len();
            gui.nodes.push(Node::new(num_nodes, if parent_index < 0 { None } else { Some(parent_index as usize) }));
            Ok(gui.nodes.len() - 1)
        });
        methods.add_method_mut("add_quad", |lua, this, ()| {
            with_gui_mut!(lua, this.index => gui);
            gui.elements.push(Element::default_quad());
            Ok(())
        });
        methods.add_method_mut("add_text", |lua, this, initial_text: String| {
            with_gui_mut!(lua, this.index => gui);
            gui.add_text(initial_text);
            Ok(gui.elements.len() - 1)
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
            AnchorPoint::BottomCenter => { self.eq(&AnchorPoint::BottomCenter) }
            AnchorPoint::BottomRight => { self.eq(&AnchorPoint::BottomRight) }
            AnchorPoint::Center => { self.eq(&AnchorPoint::Center) }
            AnchorPoint::CenterLeft => { self.eq(&AnchorPoint::CenterLeft) }
            AnchorPoint::CenterRight => { self.eq(&AnchorPoint::CenterRight) }
            AnchorPoint::TopLeft => { self.eq(&AnchorPoint::TopLeft) }
            AnchorPoint::TopCenter => { self.eq(&AnchorPoint::TopCenter) }
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
    AnchorPoint::TopLeft,
    AnchorPoint::TopCenter,
    AnchorPoint::TopRight,
    AnchorPoint::CenterLeft,
    AnchorPoint::Center,
    AnchorPoint::CenterRight,
    AnchorPoint::BottomLeft,
    AnchorPoint::BottomCenter,
    AnchorPoint::BottomRight,
];