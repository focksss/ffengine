use std::cell::RefCell;
use std::{fs, slice};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ash::vk;
use ash::vk::{CommandBuffer, DescriptorType, Format, ImageLayout, ShaderStageFlags};
use json::JsonValue;
use winit::event::MouseButton;
use winit::keyboard::{Key, PhysicalKey, SmolStr};
use crate::client::client::*;
use crate::math::*;
use crate::render::render_helper::{Descriptor, DescriptorCreateInfo, DescriptorSetCreateInfo, Pass, PassCreateInfo, PipelineCreateInfo, Renderpass, RenderpassCreateInfo, Texture, TextureCreateInfo};
use crate::gui::text::font::Font;
use crate::gui::text::text_render::{TextInformation, TextRenderer};
use crate::render::render::MAX_FRAMES_IN_FLIGHT;
use crate::render::vulkan_base::Context;
use crate::scripting::lua_engine::Lua;

pub struct GUI {
    index: usize,
    context: Arc<Context>,

    pub text_field_focused: bool,

    controller: Arc<RefCell<Client>>,
    null_tex_info: vk::DescriptorImageInfo,

    pub pass: Arc<RefCell<Pass>>,
    pub text_renderer: TextRenderer,
    pub quad_renderpass: Renderpass,

    pub nodes: Vec<Node>,
    pub root_node_indices: Vec<usize>,
    pub unparented_node_indices: Vec<usize>,

    pub elements: Vec<Element>,
    pub image_count: usize,

    pub interactable_node_indices: Vec<usize>,

    pub fonts: Vec<Arc<Font>>,

    pub active_node: usize,
    pub hovered_nodes: HashSet<usize>,

    pub script_indices: Vec<usize>,

    new_texts: Vec<usize>
}
impl GUI {
    fn handle_gui_interaction(
        &mut self,
        node_index: usize,
        min: Vector,
        max: Vector,
        can_trigger_left_click_events: &mut bool,
        can_trigger_right_click_events: &mut bool,
    ) {
        let node = &mut self.nodes[node_index];
        let interactable_information = node.interactable_information.as_mut().unwrap();

        for passive_action in interactable_information.passive_actions.iter() {
            Lua::cache_call(
                passive_action.1,
                passive_action.0.as_str(),
                Some(self.active_node),
                Some(self.index)
            )
        }

        let (x, y, left_pressed, left_just_pressed, right_pressed, right_just_pressed) = {
            let client = self.controller.borrow();
            let x = client.cursor_position.x as f32;
            let y = client.cursor_position.y as f32;
            let left_pressed = client.pressed_mouse_buttons.contains(&MouseButton::Left);
            let right_pressed = client.pressed_mouse_buttons.contains(&MouseButton::Right);
            let left_just_pressed = client.new_pressed_mouse_buttons.contains(&MouseButton::Left);
            let right_just_pressed = client.new_pressed_mouse_buttons.contains(&MouseButton::Right);
            (x, y, left_pressed, left_just_pressed, right_pressed, right_just_pressed)
        };

        let hovered =
            x > min.x && x < max.x &&
            y > min.y && y < max.y;

        if !hovered {
            self.hovered_nodes.remove(&node_index);
            for unhover_action in interactable_information.unhover_actions.iter() {
                Lua::cache_call(
                    unhover_action.1,
                    unhover_action.0.as_str(),
                    Some(self.active_node),
                    Some(self.index)
                )
            }
        } else {
            self.hovered_nodes.insert(node_index);
            for hover_action in interactable_information.hover_actions.iter() {
                Lua::cache_call(
                    hover_action.1,
                    hover_action.0.as_str(),
                    Some(self.active_node),
                    Some(self.index)
                )
            }
        }

        loop {
            if left_just_pressed && hovered {
                for left_down_action in interactable_information.left_down_actions.iter() {
                    Lua::cache_call(
                        left_down_action.1,
                        left_down_action.0.as_str(),
                        Some(self.active_node),
                        Some(self.index)
                    );
                }
                if !interactable_information.left_up_actions.is_empty() || !interactable_information.left_hold_actions.is_empty() {
                    interactable_information.was_initially_left_pressed = true;
                    *can_trigger_left_click_events = false;
                }
            }

            // discard any buttons that happen to be hovered over while holding another down
            if !interactable_information.was_initially_left_pressed {
                break
            }

            if !*can_trigger_left_click_events {
                if !left_pressed {
                    interactable_information.was_initially_left_pressed = false;
                }
                break;
            }

            *can_trigger_left_click_events = false;
            if left_pressed {
                for left_hold_action in interactable_information.left_hold_actions.iter() {
                    Lua::cache_call(
                        left_hold_action.1,
                        left_hold_action.0.as_str(),
                        Some(self.active_node),
                        Some(self.index)
                    )
                }
                break;
            } else {
                if hovered {
                    for left_tap_action in interactable_information.left_up_actions.iter() {
                        Lua::cache_call(
                            left_tap_action.1,
                            left_tap_action.0.as_str(),
                            Some(self.active_node),
                            Some(self.index)
                        )
                    }
                }
                interactable_information.was_initially_left_pressed = false;
            }
        }
        loop {
            if right_just_pressed && hovered {
                for right_down_action in interactable_information.right_down_actions.iter() {
                    Lua::cache_call(
                        right_down_action.1,
                        right_down_action.0.as_str(),
                        Some(self.active_node),
                        Some(self.index)
                    );
                }
                if !interactable_information.right_up_actions.is_empty() || !interactable_information.right_hold_actions.is_empty() {
                    interactable_information.was_initially_right_pressed = true;
                    *can_trigger_right_click_events = false;
                }
            }

            // discard any buttons that happen to be hovered over while holding another down
            if !interactable_information.was_initially_right_pressed {
                break;
            }

            if !*can_trigger_right_click_events {
                if !right_pressed {
                    interactable_information.was_initially_right_pressed = false;
                }
                break;
            }

            *can_trigger_right_click_events = false;
            if right_pressed {
                for right_hold_action in interactable_information.right_hold_actions.iter() {
                    Lua::cache_call(
                        right_hold_action.1,
                        right_hold_action.0.as_str(),
                        Some(self.active_node),
                        Some(self.index)
                    )
                }
                break;
            } else {
                if hovered {
                    for right_tap_action in interactable_information.right_up_actions.iter() {
                        Lua::cache_call(
                            right_tap_action.1,
                            right_tap_action.0.as_str(),
                            Some(self.active_node),
                            Some(self.index)
                        )
                    }
                }
                interactable_information.was_initially_right_pressed = false;
            }
        };
    }
    pub fn handle_typing_input(&mut self, logical_key: Key, text: Option<SmolStr>, physical_key: Option<PhysicalKey>) {
        // println!("handle_typing_input: {:?}", text);

    }

    pub fn new(
        index: usize,
        context: &Arc<Context>,
        controller: Arc<RefCell<Client>>,
        null_tex_sampler: vk::Sampler,
        null_tex_img_view: vk::ImageView
    ) -> GUI {
        let null_info = vk::DescriptorImageInfo {
            sampler: null_tex_sampler,
            image_view: null_tex_img_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        let (pass_ref, quad_renderpass, text_renderer) = GUI::create_rendering_objects(context, null_info);

        let gui = GUI {
            index,

            text_field_focused: false,

            context: context.clone(),
            controller,
            null_tex_info: null_info,

            pass: pass_ref.clone(),
            quad_renderpass,

            nodes: Vec::new(),
            root_node_indices: Vec::new(),
            unparented_node_indices: Vec::new(),

            elements: Vec::new(),
            image_count: 0,

            interactable_node_indices: Vec::new(),

            fonts: vec![text_renderer.default_font.clone()],

            script_indices: Vec::new(),

            text_renderer,

            active_node: 0,
            hovered_nodes: HashSet::new(),

            new_texts: Vec::new(),
        };
        gui.update_descriptors();
        gui
    }
    pub fn create_rendering_objects(context: &Arc<Context>, null_info: vk::DescriptorImageInfo) -> (Arc<RefCell<Pass>>, Renderpass, TextRenderer) {
        let pass_create_info = PassCreateInfo::new(context)
            .add_color_attachment_info(TextureCreateInfo::new(context).format(Format::R16G16B16A16_SFLOAT).add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC));
        let pass_ref = Arc::new(RefCell::new(Pass::new(pass_create_info)));

        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
            blend_enable: 1,
            src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        }];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);

        let image_infos: Vec<vk::DescriptorImageInfo> = vec![null_info; 1024];
        let image_texture_samplers_create_info = DescriptorCreateInfo::new(context)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT)
            .dynamic(true)
            .image_infos(image_infos.clone());
        let quad_descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
            .add_descriptor(Descriptor::new(&image_texture_samplers_create_info));
        let quad_renderpass_create_info = { RenderpassCreateInfo::new(context)
            .pass_ref(pass_ref.clone())
            .descriptor_set_create_info(quad_descriptor_set_create_info)
            .add_pipeline_create_info(PipelineCreateInfo::new()
                .pipeline_color_blend_state_create_info(color_blend_state)
                .vertex_shader_uri(String::from("gui\\quad\\quad.vert.spv"))
                .fragment_shader_uri(String::from("gui\\quad\\quad.frag.spv")))
            .add_push_constant_range(vk::PushConstantRange {
                stage_flags: ShaderStageFlags::ALL_GRAPHICS,
                offset: 0,
                size: size_of::<GUIQuadSendable>() as _,
            }) };
        let quad_renderpass = Renderpass::new(quad_renderpass_create_info);

        (pass_ref.clone(), quad_renderpass, TextRenderer::new(context, Some(pass_ref.clone())))
    }
    pub fn set_fonts(&mut self, fonts: Vec<Arc<Font>>) {
        self.fonts = fonts.clone();
        self.text_renderer.update_font_atlases_all_frames(fonts);
    }
    pub fn reload_rendering(&mut self, null_tex_sampler: vk::Sampler, null_tex_img_view: vk::ImageView) {
        let null_info = vk::DescriptorImageInfo {
            sampler: null_tex_sampler,
            image_view: null_tex_img_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        self.null_tex_info = null_info;
        self.text_renderer.destroy();
        self.quad_renderpass.destroy();
        (self.pass, self.quad_renderpass, self.text_renderer) = GUI::create_rendering_objects(&self.context, self.null_tex_info);
        self.update_descriptors();
    }

    /**
    * Uses custom JSON .gui files
    * * Refer to default.gui in resources/gui
    * * Nodes are drawn recursively and without depth testing. To make a node appear in front of another, define it after another.
    */
    fn parse_element(&mut self, element_json: &JsonValue) -> usize {
        let mut element_type = "";
        match &element_json["type"] {
            JsonValue::String(s) => {
                element_type = (*s).as_str();
            }
            JsonValue::Short(s) => {
                element_type = (*s).as_str();
            }
            _ => ()
        }

        /*
            Text {
                text_information: Option<TextInformation>,
                font_index: usize,
            }
         */
        let idx = self.elements.len();
        let element;
        if let JsonValue::Object(ref element_info) = element_json["info"] {
            match element_type {
                "Quad" => {
                    let mut color = Vector::fill(1.0);
                    if let JsonValue::Array(ref color_json) = element_info["color"] {
                        if color_json.len() >= 4 {
                            color = Vector::new4(
                                color_json[0].as_f32().unwrap(),
                                color_json[1].as_f32().unwrap(),
                                color_json[2].as_f32().unwrap(),
                                color_json[3].as_f32().unwrap(),
                            );
                        }
                    }
                    let mut corner_radius = 0.0;
                    if let JsonValue::Number(ref corner_radius_json) = element_info["corner_radius"] {
                        if let Ok(v) = corner_radius_json.to_string().parse::<f32>() {
                            corner_radius = v;
                        }
                    }
                    element = Element::Quad {
                        color,
                        corner_radius,
                    }
                },
                "Image" => {
                    let index = self.image_count;
                    self.image_count += 1;

                    let uri: String = match &element_info["uri"] {
                        JsonValue::String(s) => {
                            (*s).parse().expect("failed to parse URI")
                        }
                        JsonValue::Short(s) => {
                            (*s).parse().expect("failed to parse URI")
                        }
                        _ => panic!("no uri given for image")
                    };

                    let mut alpha_threshold = 5.0;
                    if let JsonValue::Number(ref alpha_threshold_json) = element_info["alpha_threshold"] {
                        if let Ok(v) = alpha_threshold_json.to_string().parse::<f32>() {
                            alpha_threshold = v;
                        }
                    }

                    let mut additive_tint = Vector::fill(0.0);
                    if let JsonValue::Array(ref additive_tint_json) = element_info["additive_tint"] {
                        if additive_tint_json.len() >= 4 {
                            additive_tint = Vector::new4(
                                additive_tint_json[0].as_f32().unwrap(),
                                additive_tint_json[1].as_f32().unwrap(),
                                additive_tint_json[2].as_f32().unwrap(),
                                additive_tint_json[3].as_f32().unwrap(),
                            );
                        }
                    }

                    let mut multiplicative_tint = Vector::fill(1.0);
                    if let JsonValue::Array(ref multiplicative_tint_json) = element_info["multiplicative_tint"] {
                        if multiplicative_tint_json.len() >= 4 {
                            multiplicative_tint = Vector::new4(
                                multiplicative_tint_json[0].as_f32().unwrap(),
                                multiplicative_tint_json[1].as_f32().unwrap(),
                                multiplicative_tint_json[2].as_f32().unwrap(),
                                multiplicative_tint_json[3].as_f32().unwrap(),
                            );
                        }
                    }

                    let mut corner_radius = 0.0;
                    if let JsonValue::Number(ref corner_radius_json) = element_info["corner_radius"] {
                        if let Ok(v) = corner_radius_json.to_string().parse::<f32>() {
                            corner_radius = v;
                        }
                    }

                    let mut aspect_ratio = None;
                    if let JsonValue::Number(ref aspect_ratio_json) = element_info["aspect_ratio"] {
                        if let Ok(v) = aspect_ratio_json.to_string().parse::<f32>() {
                            aspect_ratio = Some(v);
                        }
                    }

                    element = Element::Image {
                        index,
                        uri,
                        alpha_threshold,
                        additive_tint,
                        multiplicative_tint,
                        corner_radius,
                        aspect_ratio,

                        image_view: vk::ImageView::null(),
                        sampler: vk::Sampler::null(),
                        image: vk::Image::null(),
                        memory: vk::DeviceMemory::null(),
                    }
                },
                "Text" => {
                    let mut text_font = 0usize;
                    let mut text_text = "placeholder text";
                    let mut text_font_size = 32.0;
                    let mut text_newline_size = 1720.0;


                    if let JsonValue::Number(ref text_information_font_json) = element_info["font"] {
                        if let Ok(v) = text_information_font_json.to_string().parse::<usize>() {
                            text_font = v;
                        }
                    }
                    match &element_info["text"] {
                        JsonValue::String(s) => {
                            text_text = s.as_str();
                        }
                        JsonValue::Short(s) => {
                            text_text = s.as_str();
                        }
                        _ => {}
                    }
                    if let JsonValue::Number(ref text_information_font_size_json) = element_info["font_size"] {
                        if let Ok(v) = text_information_font_size_json.to_string().parse::<f32>() {
                            text_font_size = v;
                        }
                    }
                    if let JsonValue::Number(ref text_information_newline_distance_json) = element_info["newline_distance"] {
                        if let Ok(v) = text_information_newline_distance_json.to_string().parse::<f32>() {
                            text_newline_size = v;
                        }
                    }

                    let mut color = Vector::empty();
                    if let JsonValue::Array(ref color_json) = element_info["color"] {
                        if color_json.len() >= 4 {
                            color = Vector::new4(
                                color_json[0].as_f32().unwrap(),
                                color_json[1].as_f32().unwrap(),
                                color_json[2].as_f32().unwrap(),
                                color_json[3].as_f32().unwrap(),
                            );
                        }
                    }

                    element = Element::Text {
                        text_information: Some(TextInformation::new(self.fonts[text_font].clone())
                            .text(text_text)
                            .font_size(text_font_size)
                            .newline_distance(text_newline_size)),
                        font_index: text_font,
                        color,
                    };
                    self.new_texts.push(idx)
                },
                _ => {
                    panic!("unknown element type: {}", element_type);
                }
            }
        } else {
            panic!("no info given for element of type: {}", element_type);
        }
        self.elements.push(element);
        idx
    }
    fn parse_node(&mut self, node_json: &JsonValue, unparented_true_indices: &Vec<usize>) -> usize {
        let mut name = String::from("unnamed node");
        match &node_json["name"] {
            JsonValue::String(s) => {
                name = (*s).parse().unwrap();
            }
            JsonValue::Short(s) => {
                name = (*s).parse().unwrap();
            }
            _ => ()
        }

        let mut interactable_information = None;
        if let JsonValue::Object(ref interactable_information_json) = node_json["interactable_information"] {
            let mut interactable_passive_actions = Vec::new();
            let mut interactable_hover_actions = Vec::new();
            let mut interactable_unhover_actions = Vec::new();
            let mut interactable_left_up_actions = Vec::new();
            let mut interactable_left_down_actions = Vec::new();
            let mut interactable_right_up_actions = Vec::new();
            let mut interactable_right_down_actions = Vec::new();
            let mut interactable_left_hold_actions = Vec::new();
            let mut interactable_right_hold_actions = Vec::new();

            match &interactable_information_json["passive_actions"] {
                JsonValue::Array(arr) => {
                    for method in arr {
                        let name = match &method["method"] {
                            JsonValue::String(s) => {
                                s.as_str()
                            }
                            JsonValue::Short(s) => {
                                s.as_str()
                            }
                            _ => ""
                        };
                        interactable_passive_actions.push((
                            String::from(name),
                            self.script_indices[method["script"].as_usize().expect("interactable passive_action index parse error")]
                        ));
                    }
                }
                _ => {}
            }
            match &interactable_information_json["hover_actions"] {
                JsonValue::Array(arr) => {
                    for method in arr {
                        let name = match &method["method"] {
                            JsonValue::String(s) => {
                                s.as_str()
                            }
                            JsonValue::Short(s) => {
                                s.as_str()
                            }
                            _ => ""
                        };
                        interactable_hover_actions.push((
                            String::from(name),
                            self.script_indices[method["script"].as_usize().expect("interactable hover_action index parse error")]
                        ));
                    }
                }
                _ => {}
            }
            match &interactable_information_json["unhover_actions"] {
                JsonValue::Array(arr) => {
                    for method in arr {
                        let name = match &method["method"] {
                            JsonValue::String(s) => {
                                s.as_str()
                            }
                            JsonValue::Short(s) => {
                                s.as_str()
                            }
                            _ => ""
                        };
                        interactable_unhover_actions.push((
                            String::from(name),
                            self.script_indices[method["script"].as_usize().expect("interactable unhover_action index parse error")]
                        ));
                    }
                }
                _ => {}
            }
            match &interactable_information_json["left_up_actions"] {
                JsonValue::Array(arr) => {
                    for method in arr {
                        let name = match &method["method"] {
                            JsonValue::String(s) => {
                                s.as_str()
                            }
                            JsonValue::Short(s) => {
                                s.as_str()
                            }
                            _ => ""
                        };
                        interactable_left_up_actions.push((
                            String::from(name),
                            self.script_indices[method["script"].as_usize().expect("interactable left_up_action index parse error")]
                        ));
                    }
                }
                _ => {}
            }
            match &interactable_information_json["left_down_actions"] {
                JsonValue::Array(arr) => {
                    for method in arr {
                        let name = match &method["method"] {
                            JsonValue::String(s) => {
                                s.as_str()
                            }
                            JsonValue::Short(s) => {
                                s.as_str()
                            }
                            _ => ""
                        };
                        interactable_left_down_actions.push((
                            String::from(name),
                            self.script_indices[method["script"].as_usize().expect("interactable left_down_action index parse error")]
                        ));
                    }
                }
                _ => {}
            }
            match &interactable_information_json["right_up_actions"] {
                JsonValue::Array(arr) => {
                    for method in arr {
                        let name = match &method["method"] {
                            JsonValue::String(s) => {
                                s.as_str()
                            }
                            JsonValue::Short(s) => {
                                s.as_str()
                            }
                            _ => ""
                        };
                        interactable_right_up_actions.push((
                            String::from(name),
                            self.script_indices[method["script"].as_usize().expect("interactable right_up_actions index parse error")]
                        ));
                    }
                }
                _ => {}
            }
            match &interactable_information_json["right_down_actions"] {
                JsonValue::Array(arr) => {
                    for method in arr {
                        let name = match &method["method"] {
                            JsonValue::String(s) => {
                                s.as_str()
                            }
                            JsonValue::Short(s) => {
                                s.as_str()
                            }
                            _ => ""
                        };
                        interactable_right_down_actions.push((
                            String::from(name),
                            self.script_indices[method["script"].as_usize().expect("interactable right_down_actions index parse error")]
                        ));
                    }
                }
                _ => {}
            }
            match &interactable_information_json["left_hold_actions"] {
                JsonValue::Array(arr) => {
                    for method in arr {
                        let name = match &method["method"] {
                            JsonValue::String(s) => {
                                s.as_str()
                            }
                            JsonValue::Short(s) => {
                                s.as_str()
                            }
                            _ => ""
                        };
                        interactable_left_hold_actions.push((
                            String::from(name),
                            self.script_indices[method["script"].as_usize().expect("interactable left_hold_actions index parse error")]
                        ));
                    }
                }
                _ => {}
            }
            match &interactable_information_json["right_hold_actions"] {
                JsonValue::Array(arr) => {
                    for method in arr {
                        let name = match &method["method"] {
                            JsonValue::String(s) => {
                                s.as_str()
                            }
                            JsonValue::Short(s) => {
                                s.as_str()
                            }
                            _ => ""
                        };
                        interactable_right_hold_actions.push((
                            String::from(name),
                            self.script_indices[method["script"].as_usize().expect("interactable right_hold_actions index parse error")]
                        ));
                    }
                }
                _ => {}
            }

            let temp = GUIInteractableInformation {
                passive_actions: interactable_passive_actions,
                hover_actions: interactable_hover_actions,
                unhover_actions: interactable_unhover_actions,
                left_up_actions: interactable_left_up_actions,
                left_down_actions: interactable_left_down_actions,
                left_hold_actions: interactable_left_hold_actions,
                right_up_actions: interactable_right_up_actions,
                right_down_actions: interactable_right_down_actions,
                right_hold_actions: interactable_right_hold_actions,
                ..Default::default()
            };
            interactable_information = Some(temp);
        }

        let mut hidden = false;
        if let JsonValue::Boolean(ref hidden_json) = node_json["hidden"] {
            hidden = *hidden_json;
        }

        let mut clipping = true;
        if let JsonValue::Boolean(ref clipping_json) = node_json["clipping"] {
            clipping = *clipping_json;
        }

        let mut container = Container::default();
        if let JsonValue::Object(ref container_json) = node_json["container"] {
            let mut container_type = "";
            match &container_json["type"] {
                JsonValue::String(s) => {
                    container_type = (*s).as_str();
                }
                JsonValue::Short(s) => {
                    container_type = (*s).as_str();
                }
                _ => ()
            }

            match container_type {
                "Stack" => {
                    if let JsonValue::Object(ref container_info_json) = container_json["info"] {
                        let mut horizontal = false;
                        if let JsonValue::Boolean(ref horizontal_json) = container_info_json["horizontal"] {
                            horizontal = *horizontal_json;
                        }

                        let mut spacing = 0.0;
                        if let JsonValue::Number(ref spacing_json) = container_info_json["spacing"] {
                            if let Ok(v) = spacing_json.to_string().parse::<f32>() {
                                spacing = v;
                            }
                        }

                        let mut padding = Padding::default();
                        if let JsonValue::Object(ref padding_json) = container_info_json["padding"] {
                            if let JsonValue::Number(ref json) = padding_json["left"] {
                                if let Ok(v) = json.to_string().parse::<f32>() {
                                    padding.left = v;
                                }
                            }
                            if let JsonValue::Number(ref json) = padding_json["right"] {
                                if let Ok(v) = json.to_string().parse::<f32>() {
                                    padding.right = v;
                                }
                            }
                            if let JsonValue::Number(ref json) = padding_json["top"] {
                                if let Ok(v) = json.to_string().parse::<f32>() {
                                    padding.top = v;
                                }
                            }
                            if let JsonValue::Number(ref json) = padding_json["bottom"] {
                                if let Ok(v) = json.to_string().parse::<f32>() {
                                    padding.bottom = v;
                                }
                            }
                        }

                        let mut packing = PackingMode::default();
                        let mut packing_str = "";
                        match &container_info_json["packing"] {
                            JsonValue::String(s) => {
                                packing_str = (*s).as_str();
                            }
                            JsonValue::Short(s) => {
                                packing_str = (*s).as_str();
                            }
                            _ => ()
                        }
                        match packing_str {
                            "End" => { packing = PackingMode::End },
                            "Start" => { packing = PackingMode::Start },
                            "Center" => { packing = PackingMode::Center },
                            "SpaceExcludeEdge" => { packing = PackingMode::SpaceExcludeEdge },
                            "SpaceIncludeEdge" => { packing = PackingMode::SpaceIncludeEdge },
                            _ => ()
                        }

                        let mut alignment = Alignment::default();
                        let mut alignment_str = "";
                        match &container_info_json["alignment"] {
                            JsonValue::String(s) => {
                                alignment_str = (*s).as_str();
                            }
                            JsonValue::Short(s) => {
                                alignment_str = (*s).as_str();
                            }
                            _ => ()
                        }
                        match alignment_str {
                            "End" => { alignment = Alignment::End },
                            "Start" => { alignment = Alignment::Start },
                            "Center" => { alignment = Alignment::Center },
                            "Stretch" => { alignment = Alignment::Stretch },
                            _ => ()
                        }

                        let mut stack_direction = StackDirection::default();
                        let mut stack_direction_str = "";
                        match &container_info_json["stack_direction"] {
                            JsonValue::String(s) => {
                                stack_direction_str = (*s).as_str();
                            }
                            JsonValue::Short(s) => {
                                stack_direction_str = (*s).as_str();
                            }
                            _ => ()
                        }
                        match stack_direction_str {
                            "Alternating" => { stack_direction = StackDirection::Alternating },
                            "Normal" => { stack_direction = StackDirection::Normal },
                            "Reverse" => { stack_direction = StackDirection::Reverse },
                            _ => ()
                        }

                        container = Container::Stack {
                            horizontal,
                            spacing,
                            padding,
                            packing,
                            alignment,
                            stack_direction,
                        }
                    }
                },
                "Dock" => {
                    container = Container::Dock;
                },
                _ => { panic!("unknown container type: {}", container_type) }
            }
        }

        let mut parent_relation = None;
        if let JsonValue::Object(ref parent_relation_json) = node_json["parent_relation"] {
            let type_str;
            match &parent_relation_json["type"] {
                JsonValue::String(s) => {
                    type_str = (*s).as_str();
                }
                JsonValue::Short(s) => {
                    type_str = (*s).as_str();
                }
                _ => panic!("no type given for parent relation")
            }

            match type_str {
                "Docking" => {
                    if let JsonValue::Object(ref relation_info_json) = parent_relation_json["info"] {
                        let mut dock_mode = DockMode::default();
                        let mut dock_mode_str = "";
                        match &relation_info_json["mode"] {
                            JsonValue::String(s) => {
                                dock_mode_str = (*s).as_str();
                            }
                            JsonValue::Short(s) => {
                                dock_mode_str = (*s).as_str();
                            }
                            _ => ()
                        }
                        match dock_mode_str {
                            "Top" => { dock_mode = DockMode::Top },
                            "Bottom" => { dock_mode = DockMode::Bottom },
                            "Left" => { dock_mode = DockMode::Left },
                            "Right" => { dock_mode = DockMode::Right },
                            _ => ()
                        }

                        parent_relation = Some(ParentRelation::Docking( dock_mode ));
                    }
                },
                "Independent" => {
                    let mut relative = true;
                    let mut anchor = AnchorPoint::default();
                    let mut offset_x = Offset::Pixels(0.0);
                    let mut offset_y = Offset::Pixels(0.0);
                    if let JsonValue::Object(ref independent_info_json) = parent_relation_json["info"] {
                        if let JsonValue::Boolean(ref relative_json) = independent_info_json["relative"] {
                            relative = *relative_json;
                        }

                        let mut anchor_str = "";
                        match &independent_info_json["anchor"] {
                            JsonValue::String(s) => {
                                anchor_str = (*s).as_str();
                            }
                            JsonValue::Short(s) => {
                                anchor_str = (*s).as_str();
                            }
                            _ => ()
                        }
                        match anchor_str {
                            "TopLeft" => { anchor = AnchorPoint::TopLeft },
                            "TopCenter" => { anchor = AnchorPoint::TopCenter },
                            "TopRight" => { anchor = AnchorPoint::TopRight },
                            "BottomLeft" => { anchor = AnchorPoint::BottomLeft },
                            "BottomCenter" => { anchor = AnchorPoint::BottomCenter },
                            "BottomRight" => { anchor = AnchorPoint::BottomRight },
                            "CenterLeft" => { anchor = AnchorPoint::CenterLeft },
                            "Center" => { anchor = AnchorPoint::Center },
                            "CenterRight" => { anchor = AnchorPoint::CenterRight },
                            _ => ()
                        }

                        if let JsonValue::Object(ref offset_x_json) = independent_info_json["offset_x"] {
                            let mut value = 0.0;
                            if let JsonValue::Number(ref val_json) = offset_x_json["value"] {
                                if let Ok(v) = val_json.to_string().parse::<f32>() {
                                    value = v;
                                }
                            }

                            let mut type_str = "";
                            match &offset_x_json["type"] {
                                JsonValue::String(s) => {
                                    type_str = (*s).as_str();
                                }
                                JsonValue::Short(s) => {
                                    type_str = (*s).as_str();
                                }
                                _ => ()
                            }
                            match type_str {
                                "Pixels" => { offset_x = Offset::Pixels(value); },
                                "Factor" => { offset_x = Offset::Factor(value); },
                                _ => ()
                            }
                        }

                        if let JsonValue::Object(ref offset_y_json) = independent_info_json["offset_y"] {
                            let mut value = 0.0;
                            if let JsonValue::Number(ref val_json) = offset_y_json["value"] {
                                if let Ok(v) = val_json.to_string().parse::<f32>() {
                                    value = v;
                                }
                            }

                            let mut type_str = "";
                            match &offset_y_json["type"] {
                                JsonValue::String(s) => {
                                    type_str = (*s).as_str();
                                }
                                JsonValue::Short(s) => {
                                    type_str = (*s).as_str();
                                }
                                _ => ()
                            }
                            match type_str {
                                "Pixels" => { offset_y = Offset::Pixels(value); },
                                "Factor" => { offset_y = Offset::Factor(value); },
                                _ => ()
                            }
                        }

                    }
                    parent_relation = Some(ParentRelation::Independent {
                        relative,
                        anchor,
                        offset_x,
                        offset_y
                    })
                },
                _ => ()
            }
        }

        let mut width = Size::default();
        if let JsonValue::Object(ref width_json) = node_json["width"] {
            let mut width_type = "";
            match &width_json["type"] {
                JsonValue::String(s) => {
                    width_type = (*s).as_str();
                }
                JsonValue::Short(s) => {
                    width_type = (*s).as_str();
                }
                _ => ()
            }

            if let JsonValue::Object(ref width_info_json) = width_json["info"] {
                match width_type {
                    "Absolute" => {
                        let mut pixels = 0.0;
                        if let JsonValue::Number(ref pixels_json) = width_info_json["pixels"] {
                            if let Ok(v) = pixels_json.to_string().parse::<f32>() {
                                pixels = v;
                            }
                        }

                        width = Size::Absolute(pixels)
                    },
                    "Factor" => {
                        let mut factor = 0.0;
                        if let JsonValue::Number(ref factor_json) = width_info_json["factor"] {
                            if let Ok(v) = factor_json.to_string().parse::<f32>() {
                                factor = v;
                            }
                        }

                        width = Size::Factor(factor)
                    },
                    "FillFactor" => {
                        let mut factor = 0.0;
                        if let JsonValue::Number(ref factor_json) = width_info_json["factor"] {
                            if let Ok(v) = factor_json.to_string().parse::<f32>() {
                                factor = v;
                            }
                        }

                        width = Size::FillFactor(factor)
                    },
                    "Auto" => {
                        width = Size::Auto
                    },
                    _ => { panic!("unknown width size type: {}", width_type) }
                }
            }
        }

        let mut height = Size::default();
        if let JsonValue::Object(ref height_json) = node_json["height"] {
            let mut height_type = "";
            match &height_json["type"] {
                JsonValue::String(s) => {
                    height_type = (*s).as_str();
                }
                JsonValue::Short(s) => {
                    height_type = (*s).as_str();
                }
                _ => ()
            }

            if let JsonValue::Object(ref height_info_json) = height_json["info"] {
                match height_type {
                    "Absolute" => {
                        let mut pixels = 0.0;
                        if let JsonValue::Number(ref pixels_json) = height_info_json["pixels"] {
                            if let Ok(v) = pixels_json.to_string().parse::<f32>() {
                                pixels = v;
                            }
                        }

                        height = Size::Absolute(pixels)
                    },
                    "Factor" => {
                        let mut factor = 0.0;
                        if let JsonValue::Number(ref factor_json) = height_info_json["factor"] {
                            if let Ok(v) = factor_json.to_string().parse::<f32>() {
                                factor = v;
                            }
                        }

                        height = Size::Factor(factor)
                    },
                    "FillFactor" => {
                        let mut factor = 0.0;
                        if let JsonValue::Number(ref factor_json) = height_info_json["factor"] {
                            if let Ok(v) = factor_json.to_string().parse::<f32>() {
                                factor = v;
                            }
                        }

                        height = Size::FillFactor(factor)
                    },
                    "Auto" => {
                        height = Size::Auto
                    },
                    _ => { panic!("unknown height size type: {}", height_type) }
                }
            }
        }

        let mut element_indices = Vec::new();
        if let JsonValue::Array(ref elements_json) = node_json["elements"] {
            for element_json in elements_json {
                match element_json {
                    JsonValue::Number(_) => {
                        element_indices.push(element_json.as_usize().expect("element should be a usize if its a number"));
                    },
                    JsonValue::Object(_) => {
                        element_indices.push(self.parse_element(element_json))
                    },
                    _ => { panic!("element should be a usize or element"); }
                }
            }
        }

        let mut children_indices = Vec::new();
        if let JsonValue::Array(ref elements_json) = node_json["children"] {
            for child_json in elements_json {
                match child_json {
                    JsonValue::Number(_) => {
                        children_indices.push(unparented_true_indices[child_json.as_usize().expect("element should be a usize if its a number")]);
                    },
                    JsonValue::Object(_) => {
                        children_indices.push(self.parse_node(child_json, unparented_true_indices))
                    },
                    _ => { panic!("element should be a usize or element"); }
                }
            }
        }
        let index = self.nodes.len();
        for child_index in children_indices.iter() {
            self.nodes[*child_index].parent_index = Some(index);
        }
        self.nodes.push(Node {
            parent_index: None,
            index,
            name,
            interactable_information,
            hidden,
            clipping,
            container,
            parent_relation,
            width,
            height,
            element_indices,
            children_indices,

            position: Vector::empty(),
            size: Vector::empty(),
            clip_min: Vector::empty(),
            clip_max: Vector::empty(),
        });
        index
    }
    pub fn load_from_file(&mut self, path: &str) {
        for element in self.elements.drain(..) {
            element.destroy(&self.context.device)
        }

        let json = json::parse(fs::read_to_string(path).expect("failed to load json file").as_str()).expect("json parse error");

        let mut script_uris = Vec::new();
        for script in json["scripts"].members() {
            match &script["uri"] {
                JsonValue::String(s) => {
                    script_uris.push(Path::new((*s).as_str()));
                }
                JsonValue::Short(s) => {
                    script_uris.push(Path::new((*s).as_str()));
                }
                _ => ()
            }
        }
        let script_indices = Lua::load_scripts(script_uris).expect("script loading error");
        self.script_indices = script_indices;

        let mut fonts = Vec::new();
        for font in json["fonts"].members() {
            let mut uri = String::from("engine\\resources\\fonts\\Oxygen-Regular.ttf");
            match &font["uri"] {
                JsonValue::String(s) => {
                    uri = (*s).parse().expect("font uri parse error");
                }
                JsonValue::Short(s) => {
                    uri = (*s).parse().expect("font uri parse error");
                }
                _ => ()
            }

            let mut glyph_msdf_size = 32u32;
            if let JsonValue::Number(ref glyph_msdf_size_json) = font["glyph_msdf_size"] {
                if let Ok(v) = glyph_msdf_size_json.to_string().parse::<u32>() {
                    glyph_msdf_size = v;
                }
            }

            let mut glyph_msdf_distance_range = 2.0;
            if let JsonValue::Number(ref glyph_msdf_distance_range_json) = font["glyph_msdf_distance_range"] {
                if let Ok(v) = glyph_msdf_distance_range_json.to_string().parse::<f32>() {
                    glyph_msdf_distance_range = v;
                }
            }

            fonts.push(Arc::new(Font::new(&self.context, uri.as_str(), Some(glyph_msdf_size), Some(glyph_msdf_distance_range))));
        }
        self.set_fonts(fonts);

        for element_json in json["elements"].members() {
            self.parse_element(element_json);
        }

        self.nodes.clear();
        let mut unparented_true_indices = Vec::new();
        for node in json["nodes"].members() {
            unparented_true_indices.push(self.parse_node(node, &unparented_true_indices));
        }

        let uris: Vec<PathBuf> = self.elements.iter()
            .filter_map(|element| { match element {
                Element::Image { uri, ..} => { Some(PathBuf::from(uri)) }
                _ => None
            }}).collect();
        let textures = self.context.load_textures_batched(uris.as_slice(), true);

        for element in self.elements.iter_mut() {
            if let Element::Image {
                index,
                image_view,
                image,
                memory,
                sampler,
                ..
            } = element {
                let tex = textures[*index];
                (*image, *image_view, *memory, *sampler) = (tex.1.0, tex.0.0, tex.1.1, tex.0.1)
            }
        }

        self.update_descriptors();

        let mut guis = Vec::new();
        for gui in json["guis"].members() {
            let mut nodes = Vec::new();
            if let JsonValue::Array(ref nodes_json) = gui["nodes"] {
                for node_json in nodes_json {
                    nodes.push(
                        unparented_true_indices[node_json.as_usize().expect("node child index parse error")]
                    );
                }
            }
            guis.push(nodes);
        }
        self.unparented_node_indices = unparented_true_indices;
        self.root_node_indices = guis[0].clone();

        /*
        println!("\nGUI Hierarchy:");
        for &root_index in &self.root_node_indices {
            self.print_hierarchy(root_index, 0);
        }
        */
    }
    fn print_hierarchy(&self, index: usize, depth: usize) {
        let indent = "  ".repeat(depth);
        let node = &self.nodes[index];
        println!("{}[{}] {}, {:?}, {}", indent, index, node.name, node.parent_relation, node.container == Container::Dock);
        for &child_index in &node.children_indices {
            self.print_hierarchy(child_index, depth + 1);
        }
    }
    fn update_descriptors(&self) {
        let mut image_infos: Vec<vk::DescriptorImageInfo> = Vec::with_capacity(1024);

        let mut borrowed_sampler = None;
        let mut texture_indices = Vec::new();
        for element in self.elements.iter() {
            if let Element::Image { image_view, sampler, ..} = element {
                if borrowed_sampler.is_none() {
                    borrowed_sampler = Some(sampler);
                }
                image_infos.push(vk::DescriptorImageInfo {
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    image_view: *image_view,
                    sampler: *sampler,
                })
            }
        }
        for (i, element) in self.elements.iter().enumerate() {
            if let Element::Texture { .. } = element {
                texture_indices.push(i);
            }
        }

        unsafe {
            for frame in 0..MAX_FRAMES_IN_FLIGHT {
                let mut frame_image_infos = image_infos.clone();
                for texture_index in texture_indices.iter() {
                    let element = self.elements.get(*texture_index).unwrap();
                    if let Element::Texture{ texture_set, .. } = element {
                        frame_image_infos.push(vk::DescriptorImageInfo {
                            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                            image_view: texture_set[frame].device_texture.borrow().image_view.clone(),
                            sampler: *borrowed_sampler.unwrap(),
                        })
                    }
                }
                let missing = 1024 - frame_image_infos.len();
                for _ in 0..missing {
                    frame_image_infos.push(self.null_tex_info);
                }
                let frame_image_infos = frame_image_infos.as_slice().as_ptr();

                let descriptor_write = vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: self.quad_renderpass.descriptor_set.borrow().descriptor_sets[frame],
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1024,
                    p_image_info: frame_image_infos,
                    ..Default::default()
                };
                self.context.device.update_descriptor_sets(&[descriptor_write], &[]);
            }
        }
    }

    pub fn add_text(&mut self, text: String) {
        let new_text = Element::Text {
            text_information: Some(TextInformation::new(self.fonts[0].clone())
                .text(text.as_str())
                .font_size(17.0)
                .newline_distance(100.0)),
            font_index: 0,
            color: Vector::fill(1.0)
        };
        self.new_texts.push(self.elements.len());
        self.elements.push(new_text);
    }
    pub fn initialize_new_texts(&mut self) {
        for new_text in self.new_texts.drain(..) {
            if let Element::Text { text_information, ..} = &mut self.elements[new_text] {
                if let Some(info) = text_information {
                    info.set_buffers();
                }
            }
        }
    }

    /// returns new node index
    pub fn clone_node(&mut self, index: usize, parent_index: usize) -> usize {
        let node_index = self.nodes.len();
        self.nodes.push(self.nodes[index].clone());

        let mut new_children_indices = Vec::new();
        for child in self.nodes[node_index].children_indices.clone() {
            new_children_indices.push(self.clone_node(child, node_index));
        }
        let mut new_element_indices = Vec::new();
        for element_index in self.nodes[node_index].element_indices.clone() {
            let element = &self.elements[element_index];
            match element {
                Element::Text {
                    text_information,
                    font_index,
                    color,
                } => {
                    let new_text = Element::Text {
                        text_information: Some(TextInformation::new(self.fonts[0].clone())
                            .text(text_information.as_ref().unwrap().text.as_str())
                            .font_size(text_information.as_ref().unwrap().font_size)
                            .newline_distance(text_information.as_ref().unwrap().auto_wrap_distance)),
                        font_index: *font_index,
                        color: color.clone(),
                    };
                    new_element_indices.push(self.elements.len());
                    self.new_texts.push(self.elements.len());
                    self.elements.push(new_text)
                },
                Element::Quad {
                    corner_radius,
                    color,
                } => {
                    new_element_indices.push(self.elements.len());
                    self.elements.push(Element::Quad {
                        corner_radius: *corner_radius,
                        color: color.clone(),
                    })
                },
                Element::Image {
                    index,
                    uri,
                    alpha_threshold,
                    additive_tint,
                    multiplicative_tint,
                    corner_radius,
                    aspect_ratio,
                    image_view,
                    image,
                    memory,
                    sampler
                } => {
                    new_element_indices.push(element_index);
                    // Element::Image {
                    //     index: *index,
                    //     uri: uri.clone(),
                    //     alpha_threshold: *alpha_threshold,
                    //     additive_tint: additive_tint.clone(),
                    //     multiplicative_tint: multiplicative_tint.clone(),
                    //     corner_radius: *corner_radius,
                    //     aspect_ratio: *aspect_ratio,
                    //     image_view: image_view.clone(),
                    //     image: image.clone(),
                    //     memory: memory.clone(),
                    //     sampler: sampler.clone(),
                    // }
                },
                Element::Texture {
                    texture_set,
                    index,
                    additive_tint,
                    multiplicative_tint,
                    aspect_ratio,
                    corner_radius,
                } => {
                    new_element_indices.push(self.elements.len());
                    self.elements.push(Element::Texture {
                        texture_set: texture_set.clone(),
                        index: *index,
                        additive_tint: *additive_tint,
                        multiplicative_tint: *multiplicative_tint,
                        corner_radius: *corner_radius,
                        aspect_ratio: *aspect_ratio,
                    })
                }
            }
        }
        self.nodes[node_index].children_indices = new_children_indices;
        self.nodes[node_index].element_indices = new_element_indices;
        self.nodes[node_index].parent_index = Some(parent_index);

        node_index
    }

    fn layout(&mut self) {
        let window_size = (
            self.context.window.inner_size().width as f32,
            self.context.window.inner_size().height as f32
        );

        for root_node_index in self.root_node_indices.iter() {
            Self::layout_node(
                &window_size,
                &mut self.nodes,
                *root_node_index,
                &(0.0, 0.0),
                &window_size,
                &((0.0, 0.0), window_size),
            );
        }
    }
    fn layout_node(
        gui_viewport: &(f32, f32),
        nodes: &mut Vec<Node>,
        node_index: usize,
        parent_origin: &(f32, f32),
        parent_size: &(f32, f32),
        parent_clipping: &((f32, f32), (f32, f32)),
    ) {
        // set size and position if this container is independent, otherwise it was already set by the parent
        {
            let node = &mut nodes[node_index];
            if let Some(relation) = &node.parent_relation {
                match relation {
                    ParentRelation::Independent { relative, anchor, offset_x, offset_y } => {
                        let anchor_factor = match anchor {
                            AnchorPoint::TopLeft => (0.0, 0.0),
                            AnchorPoint::TopCenter => (0.5, 0.0),
                            AnchorPoint::TopRight => (1.0, 0.0),
                            AnchorPoint::CenterLeft => (0.0, 0.5),
                            AnchorPoint::Center => (0.5, 0.5),
                            AnchorPoint::CenterRight => (1.0, 0.5),
                            AnchorPoint::BottomLeft => (0.0, 1.0),
                            AnchorPoint::BottomCenter => (0.5, 1.0),
                            AnchorPoint::BottomRight => (1.0, 1.0),
                        };
                        let final_parent_size = if *relative { parent_size } else { gui_viewport };

                        let node_size = Self::calculate_size(&node.width, &node.height, &final_parent_size);
                        node.size = Vector::new2(node_size.0.0, node_size.1.0);

                        node.position.x = parent_origin.0 + final_parent_size.0*anchor_factor.0 + match offset_x {
                            Offset::Pixels(p) => *p,
                            Offset::Factor(f) => *f * final_parent_size.0
                        };
                        node.position.y = parent_origin.1 + final_parent_size.1*anchor_factor.1 + match offset_y {
                            Offset::Pixels(p) => *p,
                            Offset::Factor(f) => *f * final_parent_size.1
                        }
                    },
                    _ => ()
                }
            }
        }

        // fetch properties first to avoid borrow checker issues
        let container = nodes[node_index].container.clone();
        let children_indices = nodes[node_index].children_indices.clone();
        let node_pos = (nodes[node_index].position.x, nodes[node_index].position.y);
        let node_size = (nodes[node_index].size.x, nodes[node_index].size.y);
        let node_clipping = nodes[node_index].clipping;

        let node_clip_bounds = if node_clipping {
            let clip_min = (
                node_pos.0.max(parent_clipping.0.0),
                node_pos.1.max(parent_clipping.0.1),
            );
            let clip_max = (
                (node_pos.0 + node_size.0).min(parent_clipping.1.0),
                (node_pos.1 + node_size.1).min(parent_clipping.1.1),
            );
            (clip_min, clip_max)
        } else {
            (node_pos, (node_pos.0 + node_size.0, node_pos.1 + node_size.1))
        };

        nodes[node_index].clip_min.x = node_clip_bounds.0.0;
        nodes[node_index].clip_min.y = node_clip_bounds.0.1;
        nodes[node_index].clip_max.x = node_clip_bounds.1.0;
        nodes[node_index].clip_max.y = node_clip_bounds.1.1;

        // layout children based on this container type
        match container {
            Container::Stack { horizontal, spacing, padding, packing, alignment, stack_direction } => {
                Self::layout_stack(
                    nodes,
                    &children_indices,
                    node_pos,
                    node_size,
                    horizontal,
                    spacing,
                    padding,
                    packing,
                    alignment,
                    stack_direction,
                );
            },
            Container::Dock => {
                Self::layout_dock(
                    nodes,
                    &children_indices,
                    node_pos,
                    node_size,
                );
            },
        }
        for child_index in children_indices.iter() {
            Self::layout_node(
                gui_viewport,
                nodes,
                *child_index,
                &node_pos,
                &node_size,
                &node_clip_bounds,
            )
        }
    }
    fn calculate_size(
        width: &Size,
        height: &Size,
        parent_size: &(f32, f32),
    ) -> ((f32, bool), (f32, bool)) {
        let mut width_is_copy = false;
        let mut width = match width {
            Size::Absolute(p) => (*p, false),
            Size::Factor(f) => (parent_size.0 * *f, false),
            Size::FillFactor(f) => (*f, true), // recalculated in container
            Size::Auto => (100.0, false), // TODO: Calculate from content,
            Size::Copy => { width_is_copy = true;
                println!("e"); (parent_size.0, false) },
        };

        let height = match height {
            Size::Absolute(p) => (*p, false),
            Size::Factor(f) => (parent_size.1 * *f, false),
            Size::FillFactor(f) => (*f, true), // recalculated in container
            Size::Auto => (100.0, false), // TODO: Calculate from content,
            Size::Copy => {
                if width_is_copy {
                    panic!("Cannot have both width and height copy eachother")
                } else { width }
            }
        };

        if width_is_copy { width = height }

        (width, height)
    }
    fn layout_dock(
        nodes: &mut Vec<Node>,
        children_indices: &Vec<usize>,
        node_position: (f32, f32),
        node_size: (f32, f32),
    ) {
        let operable_children_indices = children_indices.iter().filter_map(|&child_index| {
            if let Some(relation) = &nodes[child_index].parent_relation {
                match relation {
                    ParentRelation::Independent { .. } => None,
                    ParentRelation::Docking(_) => Some(child_index),
                }
            } else {
                None
            }
        }).collect::<Vec<_>>();
        if operable_children_indices.is_empty() {
            return;
        }

        let mut remaining_space = node_size;
        let mut offset = (0.0, 0.0);

        for &child_index in operable_children_indices.iter() {
            let child = &nodes[child_index];
            let dock_mode;
            if let Some(relation) = &child.parent_relation {
                match relation {
                    ParentRelation::Independent { .. } => continue,
                    ParentRelation::Docking(mode) => dock_mode = mode.clone(),
                }
            } else {
                continue
            }

            // child size based on remaining space
            let child_size = Self::calculate_size(&child.width, &child.height, &remaining_space);
            let child_size = (child_size.0.0, child_size.1.0);

            // position and update remaining space based on dock mode
            let (child_pos, child_final_size) = match dock_mode {
                DockMode::Top => {
                    let pos = (node_position.0 + offset.0, node_position.1 + offset.1);
                    let size = (remaining_space.0, child_size.1);

                    offset.1 += child_size.1;
                    remaining_space.1 -= child_size.1;

                    (pos, size)
                },
                DockMode::Bottom => {
                    remaining_space.1 -= child_size.1;
                    let pos = (
                        node_position.0 + offset.0,
                        node_position.1 + offset.1 + remaining_space.1
                    );
                    let size = (remaining_space.0, child_size.1);

                    (pos, size)
                },
                DockMode::Left => {
                    let pos = (node_position.0 + offset.0, node_position.1 + offset.1);
                    let size = (child_size.0, remaining_space.1);

                    offset.0 += child_size.0;
                    remaining_space.0 -= child_size.0;

                    (pos, size)
                },
                DockMode::Right => {
                    remaining_space.0 -= child_size.0;
                    let pos = (
                        node_position.0 + offset.0 + remaining_space.0,
                        node_position.1 + offset.1
                    );
                    let size = (child_size.0, remaining_space.1);

                    (pos, size)
                },
            };

            nodes[child_index].position.x = child_pos.0;
            nodes[child_index].position.y = child_pos.1;
            nodes[child_index].size.x = child_final_size.0;
            nodes[child_index].size.y = child_final_size.1;
        }
    }
    fn layout_stack(
        nodes: &mut Vec<Node>,
        children_indices: &Vec<usize>,
        node_position: (f32, f32),
        node_size: (f32, f32),
        horizontal: bool,
        spacing: f32,
        padding: Padding,
        packing: PackingMode,
        alignment: Alignment,
        stack_direction: StackDirection,
    ) {
        let mut operable_children_indices = children_indices.iter().filter_map(|&child_index| {
            if let Some(relation) = &nodes[child_index].parent_relation {
                match relation {
                    ParentRelation::Independent { .. } => None,
                    ParentRelation::Docking(_) => None,
                }
            } else {
                Some(child_index)
            }
        }).collect::<Vec<_>>();
        if operable_children_indices.is_empty() {
            return;
        }

        // inner space AFTER padding
        let inner_space = (
            node_size.0 - padding.left - padding.right,
            node_size.1 - padding.top - padding.bottom,
        );

        // calculate sizes + determine fill distribution
        let mut total_fill_weight = 0.0;
        let mut used_space = 0.0;
        let mut child_sizes = Vec::new();

        // apply reversals
        if matches!(packing, PackingMode::End) {
            operable_children_indices.reverse();
        }
        if matches!(stack_direction, StackDirection::Reverse) {
            operable_children_indices.reverse();
        }

        for &child_index in &operable_children_indices {
            let child = &nodes[child_index];

            let sizes = Self::calculate_size(&child.width, &child.height, &inner_space);
            let size = if horizontal { sizes.0 } else { sizes.1 };

            if size.1 {
                total_fill_weight += size.0;
                child_sizes.push(0.0); // replaced later
            } else {
                used_space += size.0;
                child_sizes.push(size.0);
            }
            /*
            let size_mode = if horizontal {
                &child.width
            } else {
                &child.height
            };

            match size_mode {
                Size::Absolute(s) => {
                    used_space += s;
                    child_sizes.push(*s);
                },
                Size::Factor(f) => {
                    let size = if horizontal { inner_space.0 * f } else { inner_space.1 * f };
                    used_space += size;
                    child_sizes.push(size);
                },
                Size::FillFactor(weight) => {
                    total_fill_weight += weight;
                    child_sizes.push(0.0); // replaced later
                },
                Size::Auto => {
                    let size = 100.0; // TODO: Calculate from content
                    used_space += size;
                    child_sizes.push(size);
                },
                Size::Copy => {
                    panic!("Cannot have both width and height copy eachother")
                }
            }
            */
        }

        // implement spacing
        if children_indices.len() > 1 {
            used_space += spacing * (children_indices.len() - 1) as f32;
        }

        // distribute remaining space to FillFactor children
        let primary_axis_space = if horizontal { inner_space.0 } else { inner_space.1 };
        let remaining_space = (primary_axis_space - used_space).max(0.0);

        for (idx, &child_index) in operable_children_indices.iter().enumerate() {
            let child = &nodes[child_index];
            let size_mode = if horizontal { &child.width } else { &child.height };

            if let Size::FillFactor(weight) = size_mode {
                child_sizes[idx] = if total_fill_weight > 0.0 {
                    remaining_space * (weight / total_fill_weight)
                } else {
                    0.0
                };
            }
        }

        // starting position based on packing
        let mut current_pos = match packing {
            PackingMode::Start => 0.0,
            PackingMode::End => primary_axis_space,
            PackingMode::Center => (primary_axis_space - used_space) * 0.5,
            PackingMode::SpaceIncludeEdge => {
                if children_indices.len() > 0 {
                    primary_axis_space / (children_indices.len() + 1) as f32
                } else {
                    0.0
                }
            },
            PackingMode::SpaceExcludeEdge => 0.0,
        };

        let item_spacing = match packing {
            PackingMode::SpaceIncludeEdge => {
                if children_indices.len() > 0 {
                    primary_axis_space / (children_indices.len() + 1) as f32
                } else {
                    0.0
                }
            },
            PackingMode::SpaceExcludeEdge => {
                if children_indices.len() > 1 {
                    (remaining_space + spacing * (children_indices.len() - 1) as f32) / (children_indices.len() - 1) as f32
                } else {
                    0.0
                }
            },
            _ => spacing,
        };

        // position children
        for (idx, &child_index) in operable_children_indices.iter().enumerate() {
            let child = &nodes[child_index];
            let primary_size = child_sizes[idx];

            // flip if end
            let actual_pos = if matches!(packing, PackingMode::End) {
                current_pos - primary_size
            } else {
                current_pos
            };

            // cross-axis size
            let cross_size = if horizontal {
                match child.height {
                    Size::Absolute(h) => h,
                    Size::Factor(f) => inner_space.1 * f,
                    Size::FillFactor(_) => inner_space.1,
                    Size::Auto => 100.0,
                    Size::Copy => primary_size
                }
            } else {
                match child.width {
                    Size::Absolute(w) => w,
                    Size::Factor(f) => inner_space.0 * f,
                    Size::FillFactor(_) => inner_space.0,
                    Size::Auto => 100.0,
                    Size::Copy => primary_size
                }
            };

            // cross-axis position based on alignment
            let cross_pos = match alignment {
                Alignment::Start => 0.0,
                Alignment::Center => (if horizontal { inner_space.1 } else { inner_space.0 } - cross_size) / 2.0,
                Alignment::End => if horizontal { inner_space.1 - cross_size } else { inner_space.0 - cross_size },
                Alignment::Stretch => 0.0,
            };

            let final_cross_size = match alignment {
                Alignment::Stretch => if horizontal { inner_space.1 } else { inner_space.0 },
                _ => cross_size,
            };

            let (child_pos, child_final_size) = if horizontal {
                (
                    (
                        node_position.0 + padding.left + actual_pos,
                        node_position.1 + padding.top + cross_pos
                    ),
                    (primary_size, final_cross_size)
                )
            } else {
                (
                    (
                        node_position.0 + padding.left + cross_pos,
                        node_position.1 + padding.top + actual_pos
                    ),
                    (final_cross_size, primary_size)
                )
            };

            nodes[child_index].position.x = child_pos.0;
            nodes[child_index].position.y = child_pos.1;
            nodes[child_index].size.x = child_final_size.0;
            nodes[child_index].size.y = child_final_size.1;

            // to next child start pos
            if matches!(packing, PackingMode::End) {
                current_pos -= primary_size + item_spacing;
            } else {
                current_pos += primary_size + item_spacing;
            }
        }
    }

    pub fn draw(&mut self, current_frame: usize, command_buffer: CommandBuffer,) { unsafe {
        self.layout();
        let mut interactable_action_parameter_sets = Vec::new();

        self.pass.borrow().transition(
            command_buffer,
            current_frame,
            Some((ImageLayout::UNDEFINED, ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::AccessFlags::COLOR_ATTACHMENT_READ)),
            Some((ImageLayout::UNDEFINED, ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL, vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ)),
        );
        self.pass.borrow().begin(command_buffer, current_frame, &self.text_renderer.renderpass.scissor);

        for node_index in &self.root_node_indices {
            self.draw_node(
                *node_index,
                current_frame,
                command_buffer,
                &mut interactable_action_parameter_sets,
            );
        }

        self.context.device.cmd_end_rendering(command_buffer);
        self.pass.borrow().transition(
            command_buffer,
            current_frame,
            Some((ImageLayout::COLOR_ATTACHMENT_OPTIMAL, ImageLayout::SHADER_READ_ONLY_OPTIMAL, vk::AccessFlags::COLOR_ATTACHMENT_WRITE)),
            Some((ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL, ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL, vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)),
        );

        let mut can_trigger_left_click_event = true;
        let mut can_trigger_right_click_event = true;
        for parameter_set in interactable_action_parameter_sets.iter().rev() {
            self.active_node = parameter_set.0;
            self.handle_gui_interaction(
                parameter_set.0,
                parameter_set.1,
                parameter_set.2,
                &mut can_trigger_left_click_event,
                &mut can_trigger_right_click_event
            )
        }
    } }
    fn draw_node(
        &self,
        node_index: usize,
        current_frame: usize,
        command_buffer: CommandBuffer,
        interactable_parameter_sets: &mut Vec<(usize, Vector, Vector)>
    ) { unsafe {
        let node = &self.nodes[node_index];
        if node.hidden { return };

        if node.clip_max.x < 0.0 || node.clip_max.y < 0.0
            || node.clip_min.x > self.quad_renderpass.viewport.width || node.clip_min.y > self.quad_renderpass.viewport.height {
            return;
        }

        for element_index in node.element_indices.iter() {
            let element = &self.elements[*element_index];
            match element {
                Element::Quad {
                    color,
                    corner_radius
                } => {
                    //println!("{}", node.index);
                    let quad_constants = GUIQuadSendable {
                        additive_color: color.to_array4(),
                        multiplicative_color: [1.0; 4],
                        resolution: [self.quad_renderpass.viewport.width as i32, self.quad_renderpass.viewport.height as i32],
                        clip_min: node.clip_min.to_array2(),
                        clip_max: node.clip_max.to_array2(),
                        position: node.position.to_array2(),
                        scale: node.size.to_array2(),
                        corner_radius: *corner_radius,
                        image: -1,
                    };

                    let device = &self.context.device;
                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.quad_renderpass.pipelines[0].vulkan_pipeline,
                    );
                    device.cmd_set_viewport(command_buffer, 0, &[self.quad_renderpass.viewport]);
                    device.cmd_set_scissor(command_buffer, 0, &[self.quad_renderpass.scissor]);
                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.quad_renderpass.pipeline_layout,
                        0,
                        &[self.quad_renderpass.descriptor_set.borrow().descriptor_sets[current_frame]],
                        &[],
                    );
                    device.cmd_push_constants(command_buffer, self.quad_renderpass.pipeline_layout, ShaderStageFlags::ALL_GRAPHICS, 0, slice::from_raw_parts(
                        &quad_constants as *const GUIQuadSendable as *const u8,
                        size_of::<GUIQuadSendable>(),
                    ));
                    device.cmd_draw(command_buffer, 6, 1, 0, 0);
                },
                Element::Image {
                    index,
                    alpha_threshold,
                    additive_tint,
                    multiplicative_tint,
                    corner_radius,
                    aspect_ratio,
                    ..
                } => {
                    let mut scale = node.size.to_array2();
                    if let Some(ratio) = aspect_ratio {
                        let min = scale[0].min(scale[1]);
                        let min_axis = if scale[0] < scale[1] { 0 } else { 1 };
                        scale[min_axis] = min;
                        scale[1 - min_axis] = ratio * min;
                    }

                    let quad_constants = GUIQuadSendable {
                        additive_color: additive_tint.to_array4(),
                        multiplicative_color: multiplicative_tint.to_array4(),
                        resolution: [self.quad_renderpass.viewport.width as i32, self.quad_renderpass.viewport.height as i32],
                        clip_min: node.clip_min.to_array2(),
                        clip_max: node.clip_max.to_array2(),
                        position: node.position.to_array2(),
                        scale,
                        corner_radius: *corner_radius,
                        image: *index as i32,
                    };

                    let device = &self.context.device;
                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.quad_renderpass.pipelines[0].vulkan_pipeline,
                    );
                    device.cmd_set_viewport(command_buffer, 0, &[self.quad_renderpass.viewport]);
                    device.cmd_set_scissor(command_buffer, 0, &[self.quad_renderpass.scissor]);
                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.quad_renderpass.pipeline_layout,
                        0,
                        &[self.quad_renderpass.descriptor_set.borrow().descriptor_sets[current_frame]],
                        &[],
                    );
                    device.cmd_push_constants(command_buffer, self.quad_renderpass.pipeline_layout, ShaderStageFlags::ALL_GRAPHICS, 0, slice::from_raw_parts(
                        &quad_constants as *const GUIQuadSendable as *const u8,
                        size_of::<GUIQuadSendable>(),
                    ));
                    device.cmd_draw(command_buffer, 6, 1, 0, 0);
                },
                Element::Texture {
                    texture_set,
                    index,
                    additive_tint,
                    multiplicative_tint,
                    corner_radius,
                    aspect_ratio,
                    ..
                } => {
                    let mut scale = node.size.to_array2();
                    if let Some(ratio) = aspect_ratio {
                        let min = scale[0].min(scale[1]);
                        let min_axis = if scale[0] < scale[1] { 0 } else { 1 };
                        scale[min_axis] = min;
                        scale[1 - min_axis] = ratio * min;
                    }

                    let quad_constants = GUIQuadSendable {
                        additive_color: additive_tint.to_array4(),
                        multiplicative_color: multiplicative_tint.to_array4(),
                        resolution: [self.quad_renderpass.viewport.width as i32, self.quad_renderpass.viewport.height as i32],
                        clip_min: node.clip_min.to_array2(),
                        clip_max: node.clip_max.to_array2(),
                        position: node.position.to_array2(),
                        scale,
                        corner_radius: *corner_radius,
                        image: *index as i32,
                    };

                    let device = &self.context.device;
                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.quad_renderpass.pipelines[0].vulkan_pipeline,
                    );
                    device.cmd_set_viewport(command_buffer, 0, &[self.quad_renderpass.viewport]);
                    device.cmd_set_scissor(command_buffer, 0, &[self.quad_renderpass.scissor]);
                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.quad_renderpass.pipeline_layout,
                        0,
                        &[self.quad_renderpass.descriptor_set.borrow().descriptor_sets[current_frame]],
                        &[],
                    );
                    device.cmd_push_constants(command_buffer, self.quad_renderpass.pipeline_layout, ShaderStageFlags::ALL_GRAPHICS, 0, slice::from_raw_parts(
                        &quad_constants as *const GUIQuadSendable as *const u8,
                        size_of::<GUIQuadSendable>(),
                    ));
                    device.cmd_draw(command_buffer, 6, 1, 0, 0);
                },
                Element::Text {
                    text_information,
                    ..
                } => {
                    self.text_renderer.draw_gui_text(
                        current_frame,
                        text_information.as_ref().unwrap(),
                        node.position,
                        node.size,
                        node.clip_min,
                        node.clip_max,
                    );
                }
            }
        }

        if node.interactable_information.is_some() {
            interactable_parameter_sets.push((node_index, node.clip_min, node.clip_max));
        }

        for child in &node.children_indices {
            self.draw_node(*child, current_frame, command_buffer, interactable_parameter_sets);
        }
    } }

    pub fn destroy(&mut self) {
        self.text_renderer.destroy();
        self.quad_renderpass.destroy();
        for font in &self.fonts {
            font.destroy();
        }
        for element in self.elements.iter() {
            element.destroy(&self.context.device)
        }
    }
}

#[derive(Clone)]
pub enum Container {
    Stack {
        horizontal: bool, // else vertical
        spacing: f32,
        padding: Padding,
        packing: PackingMode,
        stack_direction: StackDirection,
        alignment: Alignment,
    },
    Dock,
}
#[derive(Clone, Debug)]
pub enum Offset {
    Pixels(f32),
    Factor(f32),
}
impl Default for Container {
    fn default() -> Self {
        Container::Dock
    }
}
impl PartialEq for Container {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Container::Dock, Container::Dock) => true,
            (Container::Stack { .. }, Container::Stack { .. }) => true,
            _ => false,
        }
    }
}
#[derive(Clone)]
pub enum PackingMode {
    Start, // top if vertical,
    End, // bottom if vertical
    Center,
    SpaceIncludeEdge,
    SpaceExcludeEdge,
}
impl Default for PackingMode {
    fn default() -> Self {
        PackingMode::Start
    }
}
#[derive(Clone)]
pub enum StackDirection {
    Reverse,
    Normal,
    Alternating
}
impl Default for StackDirection {
    fn default() -> Self {
        StackDirection::Normal
    }
}
#[derive(Clone)]
pub struct Padding {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}
impl Default for Padding {
    fn default() -> Padding {
        Padding {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
        }
    }
}
#[derive(Clone)]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
}
impl Default for Alignment {
    fn default() -> Alignment {
        Alignment::Start
    }
}
#[derive(Clone, Debug)]
pub enum ParentRelation {
    Docking(DockMode),
    Independent {
        relative: bool,
        anchor: AnchorPoint,
        offset_x: Offset,
        offset_y: Offset,
    }
}
#[derive(Clone)]
#[derive(Debug)]
pub enum AnchorPoint {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}
impl Default for AnchorPoint {
    fn default() -> AnchorPoint {
        AnchorPoint::TopLeft
    }
}
#[derive(Clone, Debug)]
pub enum DockMode {
    Left,
    Right,
    Top,
    Bottom,
}
impl Default for DockMode {
    fn default() -> DockMode {
        DockMode::Top
    }
}
#[derive(Clone)]
pub enum Size {
    Absolute(f32), // pixels
    Factor(f32), // factor of parent size
    FillFactor(f32), // factor of final remaining space, after allocation of other Size types.
    Auto, // fit content,
    Copy,
}
impl Default for Size {
    fn default() -> Size {
        Size::FillFactor(1.0)
    }
}
pub enum Element {
    Quad {
        color: Vector,
        corner_radius: f32,
    },
    Image {
        index: usize,
        uri: String,
        alpha_threshold: f32,
        additive_tint: Vector,
        multiplicative_tint: Vector,
        corner_radius: f32,
        aspect_ratio: Option<f32>,

        image_view: vk::ImageView,
        sampler: vk::Sampler,
        image: vk::Image,
        memory: vk::DeviceMemory,
    },
    Texture {
        texture_set: Vec<Texture>,
        index: usize,

        additive_tint: Vector,
        multiplicative_tint: Vector,
        corner_radius: f32,
        aspect_ratio: Option<f32>,
    },
    Text {
        text_information: Option<TextInformation>,
        font_index: usize,
        color: Vector,
    }
}
impl Element {
    fn destroy(&self, device: &ash::Device) {
        match self {
            Element::Image {
                image_view,
                sampler,
                image,
                memory,
                ..
            } => {
                unsafe {
                    device.destroy_sampler(*sampler, None);
                    device.destroy_image_view(*image_view, None);
                    device.destroy_image(*image, None);
                    device.free_memory(*memory, None);
                }
            },
            Element::Text {
                text_information,
                ..
            } => {
                if let Some(text_information) = text_information {
                    text_information.destroy();
                }
            }
            _ => ()
        }
    }
    pub fn default_quad() -> Self {
        Self::default()
    }
}
impl Default for Element {
    fn default() -> Element {
        Element::Quad {
            color: Vector::fill(1.0),
            corner_radius: 5.0,
        }
    }
}

#[derive(Clone)]
pub struct Node {
    pub parent_index: Option<usize>,
    pub index: usize,
    pub name: String,
    pub interactable_information: Option<GUIInteractableInformation>,
    pub hidden: bool,
    pub clipping: bool, // clips to parent, recursively affects clipping enabled children

    pub container: Container,
    pub parent_relation: Option<ParentRelation>,
    pub width: Size,
    pub height: Size,

    pub children_indices: Vec<usize>,
    pub element_indices: Vec<usize>,

    // computed during format pass
    pub position: Vector,
    pub size: Vector,
    pub clip_min: Vector,
    pub clip_max: Vector,
}
impl Node {
    pub fn new(index: usize, parent_index: Option<usize>) -> Self {
        Self {
            parent_index,
            index,
            name: String::from(""),
            interactable_information: None,
            hidden: false,
            clipping: true,

            container: Container::Dock,
            parent_relation: None,
            width: Size::Factor(1.0),
            height: Size::Factor(1.0),

            position: Vector::empty(),
            size: Vector::empty(),
            clip_min: Vector::empty(),
            clip_max: Vector::empty(),

            children_indices: Vec::new(),
            element_indices: Vec::new(),
        }
    }
}
#[derive(Clone)]
pub struct GUIInteractableInformation {
    was_initially_left_pressed: bool,
    was_initially_right_pressed: bool,

    passive_actions: Vec<(String, usize)>,
    pub hover_actions: Vec<(String, usize)>,
    unhover_actions: Vec<(String, usize)>,
    pub left_up_actions: Vec<(String, usize)>,
    pub left_down_actions: Vec<(String, usize)>,
    pub left_hold_actions: Vec<(String, usize)>,
    pub(crate) right_up_actions: Vec<(String, usize)>,
    right_down_actions: Vec<(String, usize)>,
    right_hold_actions: Vec<(String, usize)>,
}
impl Default for GUIInteractableInformation {
    fn default() -> Self {
        Self {
            was_initially_left_pressed: false,
            was_initially_right_pressed: false,
            passive_actions: Vec::new(),
            hover_actions: Vec::new(),
            unhover_actions: Vec::new(),
            left_hold_actions: Vec::new(),
            left_up_actions: Vec::new(),
            left_down_actions: Vec::new(),
            right_hold_actions: Vec::new(),
            right_down_actions: Vec::new(),
            right_up_actions: Vec::new(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GUIQuadSendable {
    pub additive_color: [f32; 4],

    pub multiplicative_color: [f32; 4],

    pub resolution: [i32; 2],

    pub clip_min: [f32; 2],
    pub clip_max: [f32; 2],

    pub position: [f32; 2],

    pub scale: [f32; 2],

    pub corner_radius: f32,

    pub image: i32,
}