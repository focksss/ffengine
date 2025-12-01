use std::cell::RefCell;
use std::{fs, slice};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ash::vk;
use ash::vk::{CommandBuffer, DescriptorType, Format, Handle, ShaderStageFlags};
use json::JsonValue;
use winit::event::MouseButton;
use crate::engine::get_command_buffer;
use crate::client::client::*;
use crate::math::*;
use crate::render::render_helper::{Descriptor, DescriptorCreateInfo, DescriptorSetCreateInfo, Pass, PassCreateInfo, Renderpass, RenderpassCreateInfo, Texture, TextureCreateInfo};
use crate::gui::text::font::Font;
use crate::gui::text::text_render::{TextInformation, TextRenderer};
use crate::render::render::MAX_FRAMES_IN_FLIGHT;
use crate::render::vulkan_base::{VkBase};
use crate::scripting::lua_engine::Lua;

enum GUIInteractionResult {
    None,
    LeftTap,
    LeftHold,
}

pub struct GUI {
    index: usize,

    device: ash::Device,
    window: Arc<winit::window::Window>,
    controller: Arc<RefCell<Client>>,
    null_tex_info: vk::DescriptorImageInfo,

    pub pass: Arc<RefCell<Pass>>,
    pub text_renderer: TextRenderer,
    pub quad_renderpass: Renderpass,

    pub nodes: Vec<Node>,
    pub root_node_indices: Vec<usize>,

    pub elements: Vec<Element>,
    image_count: usize,

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
        can_trigger_click_events: &mut bool,
    ) -> GUIInteractionResult {
        let mut result = GUIInteractionResult::None;

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

        let (x, y, left_pressed, left_just_pressed) = {
            let client = self.controller.borrow();
            let x = client.cursor_position.x as f32;
            let y = self.window.inner_size().height as f32 - client.cursor_position.y as f32;
            let left_pressed = client.pressed_mouse_buttons.contains(&MouseButton::Left);
            let left_just_pressed = client.new_pressed_mouse_buttons.contains(&MouseButton::Left);
            (x, y, left_pressed, left_just_pressed)
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

        if left_just_pressed && hovered {
            if !interactable_information.left_tap_actions.is_empty() || !interactable_information.left_hold_actions.is_empty() {
                interactable_information.was_initially_pressed = true;
                *can_trigger_click_events = false;
            }
        }

        // discard any buttons that happen to be hovered over while holding another down
        if !interactable_information.was_initially_pressed {
            return result;
        }

        if !*can_trigger_click_events {
            if !left_pressed {
                interactable_information.was_initially_pressed = false;
            }
            return result;
        }

        *can_trigger_click_events = false;
        if left_pressed {
            for left_hold_action in interactable_information.left_hold_actions.iter() {
                Lua::cache_call(
                    left_hold_action.1,
                    left_hold_action.0.as_str(),
                    Some(self.active_node),
                    Some(self.index)
                )
            }
            return GUIInteractionResult::LeftHold;
        } else {
            if hovered {
                result = GUIInteractionResult::LeftTap;
                for left_tap_action in interactable_information.left_tap_actions.iter() {
                    Lua::cache_call(
                        left_tap_action.1,
                        left_tap_action.0.as_str(),
                        Some(self.active_node),
                        Some(self.index)
                    )
                }
            }
            interactable_information.was_initially_pressed = false;
        }

        result
    }

    pub unsafe fn new(
        index: usize,
        base: &VkBase,
        controller: Arc<RefCell<Client>>,
        null_tex_sampler: vk::Sampler,
        null_tex_img_view: vk::ImageView
    ) -> GUI { unsafe {
        let null_info = vk::DescriptorImageInfo {
            sampler: null_tex_sampler,
            image_view: null_tex_img_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        let (pass_ref, quad_renderpass, text_renderer) = GUI::create_rendering_objects(&base, null_info);

        let mut gui = GUI {
            index,

            device: base.device.clone(),
            window: base.window.clone(),
            controller,
            null_tex_info: null_info,

            pass: pass_ref.clone(),
            quad_renderpass,

            nodes: Vec::new(),
            root_node_indices: Vec::new(),

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
        gui.update_descriptors(&base);
        gui
    } }
    pub unsafe fn create_rendering_objects(base: &VkBase, null_info: vk::DescriptorImageInfo) -> (Arc<RefCell<Pass>>, Renderpass, TextRenderer) { unsafe {
        let pass_create_info = PassCreateInfo::new(base)
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R16G16B16A16_SFLOAT).add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC));
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
        let image_texture_samplers_create_info = DescriptorCreateInfo::new(base)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT)
            .dynamic(true)
            .image_infos(image_infos.clone());
        let quad_descriptor_set_create_info = DescriptorSetCreateInfo::new(base)
            .add_descriptor(Descriptor::new(&image_texture_samplers_create_info));
        let quad_renderpass_create_info = { RenderpassCreateInfo::new(base)
            .pass_ref(pass_ref.clone())
            .descriptor_set_create_info(quad_descriptor_set_create_info)
            .vertex_shader_uri(String::from("gui\\quad\\quad.vert.spv"))
            .fragment_shader_uri(String::from("gui\\quad\\quad.frag.spv"))
            .pipeline_color_blend_state_create_info(color_blend_state)
            .push_constant_range(vk::PushConstantRange {
                stage_flags: ShaderStageFlags::ALL_GRAPHICS,
                offset: 0,
                size: size_of::<GUIQuadSendable>() as _,
            }) };
        let quad_renderpass = Renderpass::new(quad_renderpass_create_info);

        (pass_ref.clone(), quad_renderpass, TextRenderer::new(base, Some(pass_ref.clone())))
    } }
    pub unsafe fn set_fonts(&mut self, fonts: Vec<Arc<Font>>) {
        self.fonts = fonts.clone();
        self.text_renderer.update_font_atlases_all_frames(fonts);
    }
    pub unsafe fn reload_rendering(&mut self, base: &VkBase, null_tex_sampler: vk::Sampler, null_tex_img_view: vk::ImageView) { unsafe {
        let null_info = vk::DescriptorImageInfo {
            sampler: null_tex_sampler,
            image_view: null_tex_img_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        self.null_tex_info = null_info;
        self.text_renderer.destroy();
        self.quad_renderpass.destroy();
        (self.pass, self.quad_renderpass, self.text_renderer) = GUI::create_rendering_objects(base, self.null_tex_info);
        self.update_descriptors(base);
    } }

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
        let mut element = Element::default();
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

                    let mut corner_radius = 5.0;
                    if let JsonValue::Number(ref corner_radius_json) = element_info["corner_radius"] {
                        if let Ok(v) = corner_radius_json.to_string().parse::<f32>() {
                            corner_radius = v;
                        }
                    }

                    element = Element::Image {
                        index,
                        uri,
                        alpha_threshold,
                        additive_tint,
                        multiplicative_tint,
                        corner_radius,

                        image_view: vk::ImageView::null(),
                        sampler: vk::Sampler::null(),
                        image: vk::Image::null(),
                        memory: vk::DeviceMemory::null(),
                    }
                },
                _ => {
                    panic!("unknown element type: {}", element_type);
                }
            }
        } else {
            panic!("no info given for element of type: {}", element_type);
        }
        let idx = self.elements.len();
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
            let mut interactable_left_tap_actions = Vec::new();
            let mut interactable_right_tap_actions = Vec::new();
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
            match &interactable_information_json["left_tap_actions"] {
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
                        interactable_left_tap_actions.push((
                            String::from(name),
                            self.script_indices[method["script"].as_usize().expect("interactable left_tap_action index parse error")]
                        ));
                    }
                }
                _ => {}
            }
            match &interactable_information_json["right_tap_actions"] {
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
                        interactable_right_tap_actions.push((
                            String::from(name),
                            self.script_indices[method["script"].as_usize().expect("interactable right_tap_actions index parse error")]
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
                was_initially_pressed: false,

                passive_actions: interactable_passive_actions,
                hover_actions: interactable_hover_actions,
                unhover_actions: interactable_unhover_actions,
                left_tap_actions: interactable_left_tap_actions,
                left_hold_actions: interactable_left_hold_actions,
                right_tap_actions: interactable_right_tap_actions,
                right_hold_actions: interactable_right_hold_actions,
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

            if let JsonValue::Object(ref container_info_json) = container_json["info"] {
                match container_type {
                    "Stack" => {
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
                    },
                    "None" => {
                        container = Container::None;
                    },
                    "Dock" => {
                        container = Container::Dock;
                    },
                    _ => { panic!("unknown container type: {}", container_type) }
                }
            }
        }

        let mut dock_mode_str = "";
        match &node_json["dock_mode"] {
            JsonValue::String(s) => {
                dock_mode_str = (*s).as_str();
            }
            JsonValue::Short(s) => {
                dock_mode_str = (*s).as_str();
            }
            _ => ()
        }
        let dock_mode = match dock_mode_str {
            "Left" => { Some(DockMode::Left) },
            "Right" => { Some(DockMode::Right) },
            "Top" => { Some(DockMode::Top) },
            "Bottom" => { Some(DockMode::Bottom) },
            _ => { None }
        };

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
            dock_mode,
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
    pub unsafe fn load_from_file(&mut self, base: &VkBase, path: &str) {
        unsafe {
            for element in self.elements.drain(..) {
                element.destroy(&self.device)
            }
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

            fonts.push(Arc::new(Font::new(base, uri.as_str(), Some(glyph_msdf_size), Some(glyph_msdf_distance_range))));
        }
        unsafe { self.set_fonts(fonts) };

        for element_json in json["elements"].members() {
            self.parse_element(element_json);
        }

        self.nodes.clear();
        let mut unparented_true_indices = Vec::new();
        for node in json["nodes"].members() {
            unparented_true_indices.push(self.parse_node(node, &unparented_true_indices));
        }

        unsafe {
            let uris: Vec<PathBuf> = self.elements.iter()
                .filter_map(|element| { match element {
                    Element::Image { uri, ..} => { Some(PathBuf::from(uri)) }
                    _ => None
                }}).collect();
            let textures = base.load_textures_batched(uris.as_slice(), true);

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

            self.update_descriptors(base);
        };

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
        self.root_node_indices = guis[0].clone();

        println!("\nGUI Hierarchy:");
        for &root_index in &self.root_node_indices {
            self.print_hierarchy(root_index, 0);
        }
    }
    fn print_hierarchy(&self, index: usize, depth: usize) {
        let indent = "  ".repeat(depth);
        let node = &self.nodes[index];
        println!("{}[{}] {}", indent, index, node.name);
        for &child_index in &node.children_indices {
            self.print_hierarchy(child_index, depth + 1);
        }
    }
    unsafe fn update_descriptors(&self, base: &VkBase) {
        let mut image_infos: Vec<vk::DescriptorImageInfo> = Vec::with_capacity(1024);

        for element in self.elements.iter() {
            if let Element::Image { image_view, sampler, ..} = element {
                image_infos.push(vk::DescriptorImageInfo {
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    image_view: *image_view,
                    sampler: *sampler,
                })
            }
        }
        let missing = 1024 - image_infos.len();
        for _ in 0..missing {
            image_infos.push(self.null_tex_info);
        }
        let image_infos = image_infos.as_slice().as_ptr();

        unsafe {
            for frame in 0..MAX_FRAMES_IN_FLIGHT {
                let descriptor_write = vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: self.quad_renderpass.descriptor_set.descriptor_sets[frame],
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1024,
                    p_image_info: image_infos,
                    ..Default::default()
                };
                base.device.update_descriptor_sets(&[descriptor_write], &[]);
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
        };
        self.new_texts.push(self.elements.len());
        self.elements.push(new_text);
    }
    pub unsafe fn initialize_new_texts(&mut self, base: &VkBase) {
        for new_text in self.new_texts.drain(..) {
            if let Element::Text { text_information, ..} = &mut self.elements[new_text] {
                if let Some(info) = text_information {
                    info.set_buffers(base);
                }
            }
        }
    }

    fn layout(&mut self) {
        let window_size = (
            self.window.inner_size().width as f32,
            self.window.inner_size().height as f32
        );

        for root_node_index in self.root_node_indices.iter() {
            Self::layout_node(
                &mut self.nodes,
                *root_node_index,
                (0.0, 0.0),
                window_size,
                ((0.0, 0.0), window_size),
            );
        }
    }
    fn layout_node(
        nodes: &mut Vec<Node>,
        node_index: usize,
        parent_position: (f32, f32),
        available_space: (f32, f32),
        parent_clipping: ((f32, f32), (f32, f32)),
    ) {
        let node_size = Self::calculate_size(nodes, node_index, available_space);

        nodes[node_index].size.x = node_size.0;
        nodes[node_index].size.y = node_size.1;

        if nodes[node_index].container == Container::None {
            nodes[node_index].position.x = parent_position.0;
            nodes[node_index].position.y = parent_position.1;
        }

        // fetch children and container type before borrowing mutably
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
            parent_clipping
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
                    node_clip_bounds,
                );
            },
            Container::Dock => {
                Self::layout_dock(
                    nodes,
                    &children_indices,
                    node_pos,
                    node_size,
                    node_clip_bounds,
                );
            },
            Container::None => {
                // Children position themselves - just recurse
                for &child_index in &children_indices {
                    Self::layout_node(
                        nodes,
                        child_index,
                        node_pos,
                        node_size,
                        node_clip_bounds,
                    );
                }
            }
        }
    }
    fn calculate_size(
        nodes: &Vec<Node>,
        node_index: usize,
        available_space: (f32, f32),
    ) -> (f32, f32) {
        let node = &nodes[node_index];

        let width = match node.width {
            Size::Absolute(w) => w,
            Size::Factor(f) => available_space.0 * f,
            Size::FillFactor(_) => available_space.0, // recalculated in container
            Size::Auto => 100.0, // TODO: Calculate from content
        };

        let height = match node.height {
            Size::Absolute(h) => h,
            Size::Factor(f) => available_space.1 * f,
            Size::FillFactor(_) => available_space.1, // recalculated in container
            Size::Auto => 100.0, // TODO: Calculate from content
        };

        (width, height)
    }
    fn layout_dock(
        nodes: &mut Vec<Node>,
        children_indices: &Vec<usize>,
        node_position: (f32, f32),
        node_size: (f32, f32),
        parent_clipping: ((f32, f32), (f32, f32)),
    ) {
        let mut remaining_space = node_size;
        let mut offset = (0.0, 0.0);

        for &child_index in children_indices {
            let child = &nodes[child_index];
            let dock_mode = child.dock_mode.clone();

            // child size based on remaining space
            let child_size = Self::calculate_size(nodes, child_index, remaining_space);

            // position and update remaining space based on dock mode
            let (child_pos, child_final_size) = match dock_mode {
                Some(DockMode::Top) => {
                    let pos = (node_position.0 + offset.0, node_position.1 + offset.1);
                    let size = (remaining_space.0, child_size.1);

                    offset.1 += child_size.1;
                    remaining_space.1 -= child_size.1;

                    (pos, size)
                },
                Some(DockMode::Bottom) => {
                    remaining_space.1 -= child_size.1;
                    let pos = (
                        node_position.0 + offset.0,
                        node_position.1 + offset.1 + remaining_space.1
                    );
                    let size = (remaining_space.0, child_size.1);

                    (pos, size)
                },
                Some(DockMode::Left) => {
                    let pos = (node_position.0 + offset.0, node_position.1 + offset.1);
                    let size = (child_size.0, remaining_space.1);

                    offset.0 += child_size.0;
                    remaining_space.0 -= child_size.0;

                    (pos, size)
                },
                Some(DockMode::Right) => {
                    remaining_space.0 -= child_size.0;
                    let pos = (
                        node_position.0 + offset.0 + remaining_space.0,
                        node_position.1 + offset.1
                    );
                    let size = (child_size.0, remaining_space.1);

                    (pos, size)
                },
                None => {
                    // Fill remaining space
                    let pos = (node_position.0 + offset.0, node_position.1 + offset.1);
                    let size = remaining_space;

                    (pos, size)
                }
            };

            nodes[child_index].position.x = child_pos.0;
            nodes[child_index].position.y = child_pos.1;
            nodes[child_index].size.x = child_final_size.0;
            nodes[child_index].size.y = child_final_size.1;

            // layout this child
            Self::layout_node(
                nodes,
                child_index,
                child_pos,
                child_final_size,
                parent_clipping,
            );
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
        parent_clipping: ((f32, f32), (f32, f32)),
    ) {
        if children_indices.is_empty() {
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

        let mut children_iterable: Vec<usize> = children_indices.clone();

        // apply reversals
        if matches!(packing, PackingMode::End) {
            children_iterable.reverse();
        }
        if matches!(stack_direction, StackDirection::Reverse) {
            children_iterable.reverse();
        }

        for &child_index in &children_iterable {
            let child = &nodes[child_index];
            let size_mode = if horizontal { &child.width } else { &child.height };

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
            }
        }

        // implement spacing
        if children_indices.len() > 1 {
            used_space += spacing * (children_indices.len() - 1) as f32;
        }

        // distribute remaining space to FillFactor children
        let primary_axis_space = if horizontal { inner_space.0 } else { inner_space.1 };
        let remaining_space = (primary_axis_space - used_space).max(0.0);

        for (idx, &child_index) in children_iterable.iter().enumerate() {
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
        for (idx, &child_index) in children_iterable.iter().enumerate() {
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
                }
            } else {
                match child.width {
                    Size::Absolute(w) => w,
                    Size::Factor(f) => inner_space.0 * f,
                    Size::FillFactor(_) => inner_space.0,
                    Size::Auto => 100.0,
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

            Self::layout_node(
                nodes,
                child_index,
                child_pos,
                child_final_size,
                parent_clipping,
            );

            // to next child start pos
            if matches!(packing, PackingMode::End) {
                current_pos -= primary_size + item_spacing;
            } else {
                current_pos += primary_size + item_spacing;
            }
        }
    }

    pub unsafe fn draw(&mut self, current_frame: usize, command_buffer: CommandBuffer,) { unsafe {
        self.layout();
        let mut interactable_action_parameter_sets = Vec::new();

        self.device.cmd_begin_render_pass(
            command_buffer,
            &self.pass.borrow().get_pass_begin_info(current_frame, None, self.text_renderer.renderpass.scissor),
            vk::SubpassContents::INLINE,
        );

        for node_index in &self.root_node_indices {
            self.draw_node(
                *node_index,
                current_frame,
                command_buffer,
                &mut interactable_action_parameter_sets,
            );
        }

        self.device.cmd_end_render_pass(command_buffer);
        self.pass.borrow().transition_to_readable(command_buffer, current_frame);

        let mut can_trigger_click_event = true;
        for parameter_set in interactable_action_parameter_sets.iter().rev() {
            self.active_node = parameter_set.0;
            match self.handle_gui_interaction(parameter_set.0, parameter_set.1, parameter_set.2, &mut can_trigger_click_event) {
                GUIInteractionResult::None => (),
                _ => { }
            }
        }
    } }
    unsafe fn draw_node(
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

                    let device = &self.device;
                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.quad_renderpass.pipeline,
                    );
                    device.cmd_set_viewport(command_buffer, 0, &[self.quad_renderpass.viewport]);
                    device.cmd_set_scissor(command_buffer, 0, &[self.quad_renderpass.scissor]);
                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.quad_renderpass.pipeline_layout,
                        0,
                        &[self.quad_renderpass.descriptor_set.descriptor_sets[current_frame]],
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
                    ..
                } => {
                    let quad_constants = GUIQuadSendable {
                        additive_color: additive_tint.to_array4(),
                        multiplicative_color: multiplicative_tint.to_array4(),
                        resolution: [self.quad_renderpass.viewport.width as i32, self.quad_renderpass.viewport.height as i32],
                        clip_min: node.clip_min.to_array2(),
                        clip_max: node.clip_max.to_array2(),
                        position: node.position.to_array2(),
                        scale: node.size.to_array2(),
                        corner_radius: *corner_radius,
                        image: *index as i32,
                    };

                    let device = &self.device;
                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.quad_renderpass.pipeline,
                    );
                    device.cmd_set_viewport(command_buffer, 0, &[self.quad_renderpass.viewport]);
                    device.cmd_set_scissor(command_buffer, 0, &[self.quad_renderpass.scissor]);
                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.quad_renderpass.pipeline_layout,
                        0,
                        &[self.quad_renderpass.descriptor_set.descriptor_sets[current_frame]],
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

    pub unsafe fn destroy(&mut self) { unsafe {
        self.text_renderer.destroy();
        self.quad_renderpass.destroy();
        for font in &self.fonts {
            font.destroy();
        }
        for element in self.elements.iter() {
            element.destroy(&self.device)
        }
    } }
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
    None,
}
impl Default for Container {
    fn default() -> Self {
        Container::None
    }
}
impl PartialEq for Container {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Container::None, Container::None) => true,
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
#[derive(Clone)]
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
    FillFactor(f32), // grows to fill, weighted by value
    Auto, // fit content
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

        image_view: vk::ImageView,
        sampler: vk::Sampler,
        image: vk::Image,
        memory: vk::DeviceMemory,
    },
    Text {
        text_information: Option<TextInformation>,
        font_index: usize,
    }
}
impl Element {
    unsafe fn destroy(&self, device: &ash::Device) {
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
    pub dock_mode: Option<DockMode>,
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
            dock_mode: None,
            width: Size::FillFactor(1.0),
            height: Size::FillFactor(1.0),

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
    was_initially_pressed: bool,

    passive_actions: Vec<(String, usize)>,
    hover_actions: Vec<(String, usize)>,
    unhover_actions: Vec<(String, usize)>,
    pub left_tap_actions: Vec<(String, usize)>,
    left_hold_actions: Vec<(String, usize)>,
    right_tap_actions: Vec<(String, usize)>,
    right_hold_actions: Vec<(String, usize)>,
}
impl Default for GUIInteractableInformation {
    fn default() -> Self {
        Self {
            was_initially_pressed: false,
            passive_actions: Vec::new(),
            hover_actions: Vec::new(),
            unhover_actions: Vec::new(),
            left_hold_actions: Vec::new(),
            left_tap_actions: Vec::new(),
            right_hold_actions: Vec::new(),
            right_tap_actions: Vec::new(),
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