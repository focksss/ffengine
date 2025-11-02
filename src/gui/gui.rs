use std::cell::RefCell;
use std::{fs, slice};
use std::sync::Arc;
use ash::vk;
use ash::vk::{DescriptorType, Format, ShaderStageFlags};
use json::JsonValue;
use winit::event::MouseButton;
use crate::engine::input::Controller;
use crate::math::Vector;
use crate::render::{Descriptor, DescriptorCreateInfo, DescriptorSetCreateInfo, Font, Pass, PassCreateInfo, Renderpass, RenderpassCreateInfo, TextInformation, TextRenderer, TextureCreateInfo, VkBase};

pub struct GUI {
    device: ash::Device,
    window_ptr: *const winit::window::Window,
    controller: Arc<RefCell<Controller>>,

    pub pass: Arc<RefCell<Pass>>,
    pub text_renderer: TextRenderer,
    pub quad_renderpass: Renderpass,

    pub gui_nodes: Vec<GUINode>,
    pub gui_root_node_indices: Vec<usize>,

    pub gui_quads: Vec<GUIQuad>,
    pub gui_texts: Vec<GUIText>,

    pub interactable_node_indices: Vec<usize>,

    pub fonts: Vec<Arc<Font>>,
}
impl GUI {
    pub fn handle_gui_interaction(&mut self, node: GUINode, node_index: usize, min: Vector, max: Vector) {
        let (x, y, left_pressed) = {
            let controller = self.controller.borrow();
            let x = controller.cursor_position.x as f32;
            let y = self.window().inner_size().height as f32 - controller.cursor_position.y as f32;
            let left_pressed = controller.pressed_mouse_buttons.contains(&MouseButton::Left);
            (x, y, left_pressed)
        };

        let hovered = if
                x > min.x && x < max.x &&
                y > min.y && y < max.y
            { true } else { false };
        let node_unhover_action = &node.interactable_information.as_ref().unwrap().unhover_action;
        if !hovered {
            if let Some(potential_unhover_action) = &node_unhover_action {
                let unhover_action = potential_unhover_action.as_str();
                match unhover_action {
                    "color_quad_normal" => {
                        self.gui_quads[node.quad.expect("GUINode with 'color_quad_normal' does not have quad'")].color = Vector::new_vec4(0.5, 0.5, 0.5, 1.0);
                    }
                    _ => ()
                }
            }
            return;
        };

        let node_hover_action = &node.interactable_information.as_ref().unwrap().hover_action;
        if let Some(potential_hover_action) = &node_hover_action {
            let hover_action = potential_hover_action.as_str();
            match hover_action {
                "color_quad_bright" => {
                    self.gui_quads[node.quad.expect("GUINode with 'color_quad_bright' does not have quad'")].color = Vector::new_vec4(0.7, 0.7, 0.7, 1.0);
                }
                _ => ()
            }
        }

        let interactable_information = self.gui_nodes[node_index].interactable_information.as_mut().unwrap();
        if !interactable_information.was_pressed_last_frame && left_pressed {
            interactable_information.was_pressed_last_frame = true;
            let node_left_tap_action = &node.interactable_information.as_ref().unwrap().left_tap_action;
            if let Some(potential_left_tap_action) = &node_left_tap_action {
                let left_tap_action = potential_left_tap_action.as_str();
                match left_tap_action {
                    "reload_shaders" => {
                        self.controller.borrow_mut().queue_flags.reload_shaders_queued = true;
                        println!("Reloading shaders");
                    }
                    "reload_gui" => {
                        self.controller.borrow_mut().queue_flags.reload_gui_queued = true;
                    }
                    "pause_rendering" => {
                        self.controller.borrow_mut().queue_flags.pause_rendering = true;
                    }
                    "screenshot" => {
                        self.controller.borrow_mut().queue_flags.screenshot_queued = true;
                    }
                    _ => ()
                }
            }
        }
    }

    pub unsafe fn new(base: &VkBase, controller: Arc<RefCell<Controller>>) -> GUI { unsafe {
        let (pass_ref, quad_renderpass, text_renderer) = GUI::create_rendering_objects(base);

        let default_font = Arc::new(Font::new(&base, "resources\\fonts\\Oxygen-Regular.ttf", Some(32), Some(2.0)));
        GUI {
            device: base.device.clone(),
            window_ptr: &base.window as *const _,
            controller,

            pass: pass_ref.clone(),
            text_renderer,
            quad_renderpass,

            gui_nodes: Vec::new(),
            gui_root_node_indices: Vec::new(),

            gui_quads: Vec::new(),
            gui_texts: Vec::new(),

            interactable_node_indices: Vec::new(),

            fonts: vec![default_font.clone()],
        }
    } }
    pub unsafe fn create_rendering_objects(base: &VkBase) -> (Arc<RefCell<Pass>>, Renderpass, TextRenderer) { unsafe {
        let pass_create_info = PassCreateInfo::new(base)
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R16G16B16A16_SFLOAT).add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC));
        let pass_ref = Arc::new(RefCell::new(Pass::new(pass_create_info)));

        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
            blend_enable: 0,
            src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ZERO,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        }];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);

        let texture_sampler_create_info = DescriptorCreateInfo::new(base)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT);
        let quad_descriptor_set_create_info = DescriptorSetCreateInfo::new(base)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info));
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
    fn window(&self) -> &winit::window::Window {
        unsafe { &*self.window_ptr }
    }
    pub unsafe fn set_fonts(&mut self, fonts: &Vec<Arc<Font>>) {
        self.fonts = fonts.clone();
        self.text_renderer.update_font_atlases_all_frames(fonts.clone());
    }
    pub unsafe fn reload_rendering(&mut self, base: &VkBase) { unsafe {
        self.text_renderer.destroy();
        self.quad_renderpass.destroy();
        (self.pass, self.quad_renderpass, self.text_renderer) = GUI::create_rendering_objects(base);
    } }

    /**
    * Uses custom JSON .gui files
    * * Refer to default.gui in resources/gui
    * * Nodes are drawn recursively and without depth testing. To make a node appear in front of another, define it after another.
    */
    pub fn load_from_file(&mut self, base: &VkBase, path: &str) {
        let json = json::parse(fs::read_to_string(path).expect("failed to load json file").as_str()).expect("json parse error");

        let mut fonts = Vec::new();
        for font in json["fonts"].members() {
            let mut uri = String::from("resources\\fonts\\Oxygen-Regular.ttf");
            if let JsonValue::String(ref uri_json) = font["uri"] {
                uri = (*uri_json).parse().expect("font uri parse error");
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
        unsafe { self.set_fonts(&fonts) };

        let mut guis = Vec::new();
        for gui in json["guis"].members() {
            let mut nodes = Vec::new();
            if let JsonValue::Array(ref nodes_json) = gui["nodes"] {
                for node_json in nodes_json {
                    nodes.push(node_json.as_usize().expect("node child index parse error"));
                }
            }
            guis.push(nodes);
        }
        self.gui_root_node_indices = guis[0].clone();

        let mut nodes = Vec::new();
        for node in json["nodes"].members() {
            let mut name = String::from("unnamed node");
            if let JsonValue::String(ref name_json) = node["name"] {
                name = (*name_json).parse().expect("node name parse error");
            }

            let mut interactable_information = None;
            if let JsonValue::Object(ref interactable_information_json) = node["interactable_information"] {
                let mut interactable_hover_action = None;
                let mut interactable_unhover_action = None;
                let mut interactable_left_tap_action = None;
                let mut interactable_right_tap_action = None;
                let mut interactable_left_hold_action = None;
                let mut interactable_right_hold_action = None;
                let mut interactable_hitbox_diversion = None;

                match &interactable_information_json["hover_action"] {
                    JsonValue::String(s) => {
                        interactable_hover_action = Some(s.to_string());
                    }
                    JsonValue::Short(s) => {
                        interactable_hover_action = Some(s.to_string());
                    }
                    _ => {}
                }
                match &interactable_information_json["unhover_action"] {
                    JsonValue::String(s) => {
                        interactable_unhover_action = Some(s.to_string());
                    }
                    JsonValue::Short(s) => {
                        interactable_unhover_action = Some(s.to_string());
                    }
                    _ => {}
                }
                match &interactable_information_json["left_tap_action"] {
                    JsonValue::String(s) => {
                        interactable_left_tap_action = Some(s.to_string());
                    }
                    JsonValue::Short(s) => {
                        interactable_left_tap_action = Some(s.to_string());
                    }
                    _ => {}
                }
                match &interactable_information_json["right_tap_action"] {
                    JsonValue::String(s) => {
                        interactable_right_tap_action = Some(s.to_string());
                    }
                    JsonValue::Short(s) => {
                        interactable_right_tap_action = Some(s.to_string());
                    }
                    _ => {}
                }
                match &interactable_information_json["left_hold_action"] {
                    JsonValue::String(s) => {
                        interactable_left_hold_action = Some(s.to_string());
                    }
                    JsonValue::Short(s) => {
                        interactable_left_hold_action = Some(s.to_string());
                    }
                    _ => {}
                }
                match &interactable_information_json["right_hold_action"] {
                    JsonValue::String(s) => {
                        interactable_right_hold_action = Some(s.to_string());;
                    }
                    JsonValue::Short(s) => {
                        interactable_right_hold_action = Some(s.to_string());
                    }
                    _ => {}
                }
                if let JsonValue::Number(ref interactable_information_json) = interactable_information_json["hitbox_diversion"] {
                    if let Ok(v) = interactable_information_json.to_string().parse::<usize>() {
                        interactable_hitbox_diversion = Some(v);
                    }
                }

                let temp = GUIInteractableInformation {
                    was_pressed_last_frame: false,

                    hover_action: interactable_hover_action,
                    unhover_action: interactable_unhover_action,
                    left_tap_action: interactable_left_tap_action,
                    left_hold_action: interactable_left_hold_action,
                    right_tap_action: interactable_right_tap_action,
                    right_hold_action: interactable_right_hold_action,
                    hitbox_diversion: interactable_hitbox_diversion,
                };
                interactable_information = Some(temp);
            }

            let mut hidden = false;
            if let JsonValue::Boolean(ref hidden_json) = node["hidden"] {
                hidden = *hidden_json;
            }

            let mut children_indices = Vec::new();
            if let JsonValue::Array(ref children_json) = node["children"] {
                for child_json in children_json {
                    children_indices.push(child_json.as_usize().expect("node child index parse error"));
                }
            }

            let mut position = Vector::new_empty();
            if let JsonValue::Array(ref position_json) = node["position"] {
                if position_json.len() >= 2 {
                    position = Vector::new_vec2(
                        position_json[0].as_f32().unwrap(),
                        position_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut scale = Vector::new_empty();
            if let JsonValue::Array(ref scale_json) = node["scale"] {
                if scale_json.len() >= 2 {
                    scale = Vector::new_vec2(
                        scale_json[0].as_f32().unwrap(),
                        scale_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut absolute_position = false;
            if let JsonValue::Boolean(ref absolute_position_json) = node["absolute_position"] {
                absolute_position = *absolute_position_json;
            }

            let mut absolute_scale = false;
            if let JsonValue::Boolean(ref absolute_scale_json) = node["absolute_scale"] {
                absolute_scale = *absolute_scale_json;
            }

            let mut text = None;
            if let JsonValue::Number(ref text_json) = node["text"] {
                if let Ok(v) = text_json.to_string().parse::<usize>() {
                    text = Some(v);
                }
            }

            let mut quad = None;
            if let JsonValue::Number(ref quad_json) = node["quad"] {
                if let Ok(v) = quad_json.to_string().parse::<usize>() {
                    quad = Some(v);
                }
            }

            nodes.push(GUINode {
                index: nodes.len(),
                name,
                interactable_information,
                hidden,
                position,
                scale,
                children_indices,
                absolute_position,
                absolute_scale,
                text,
                quad,
            })
        }
        self.gui_nodes = nodes;

        let mut gui_texts = Vec::new();
        for text in json["texts"].members() {
            let mut text_font = 0usize;
            let mut text_text = "placeholder text";
            let mut text_font_size = 32.0;
            let mut text_newline_size = 1720.0;
            if let JsonValue::Object(ref text_information_json) = text["text_information"] {
                if let JsonValue::Number(ref text_information_font_json) = text_information_json["font"] {
                    if let Ok(v) = text_information_font_json.to_string().parse::<usize>() {
                        text_font = v;
                    }
                }
                match &text_information_json["text"] {
                    JsonValue::String(s) => {
                        text_text = s.as_str();
                    }
                    JsonValue::Short(s) => {
                        text_text = s.as_str();
                    }
                    _ => {}
                }
                if let JsonValue::Number(ref text_information_font_size_json) = text_information_json["font_size"] {
                    if let Ok(v) = text_information_font_size_json.to_string().parse::<f32>() {
                        text_font_size = v;
                    }
                }
                if let JsonValue::Number(ref text_information_newline_distance_json) = text_information_json["newline_distance"] {
                    if let Ok(v) = text_information_newline_distance_json.to_string().parse::<f32>() {
                        text_newline_size = v;
                    }
                }
            }

            let mut position = Vector::new_empty();
            if let JsonValue::Array(ref position_json) = text["position"] {
                if position_json.len() >= 2 {
                    position = Vector::new_vec2(
                        position_json[0].as_f32().unwrap(),
                        position_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut scale = Vector::new_empty();
            if let JsonValue::Array(ref scale_json) = text["scale"] {
                if scale_json.len() >= 2 {
                    scale = Vector::new_vec2(
                        scale_json[0].as_f32().unwrap(),
                        scale_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut clip_min = Vector::new_empty();
            if let JsonValue::Array(ref clip_min_json) = text["clip_min"] {
                if clip_min_json.len() >= 2 {
                    clip_min = Vector::new_vec2(
                        clip_min_json[0].as_f32().unwrap(),
                        clip_min_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut clip_max = Vector::new_empty();
            if let JsonValue::Array(ref clip_max_json) = text["clip_max"] {
                if clip_max_json.len() >= 2 {
                    clip_max = Vector::new_vec2(
                        clip_max_json[0].as_f32().unwrap(),
                        clip_max_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut absolute_position = false;
            if let JsonValue::Boolean(ref absolute_position_json) = text["absolute_position"] {
                absolute_position = *absolute_position_json;
            }

            let mut absolute_scale = false;
            if let JsonValue::Boolean(ref absolute_scale_json) = text["absolute_scale"] {
                absolute_scale = *absolute_scale_json;
            }

            let mut color = Vector::new_empty();
            if let JsonValue::Array(ref color_json) = text["color"] {
                if color_json.len() >= 4 {
                    color = Vector::new_vec4(
                        color_json[0].as_f32().unwrap(),
                        color_json[1].as_f32().unwrap(),
                        color_json[2].as_f32().unwrap(),
                        color_json[3].as_f32().unwrap(),
                    );
                }
            }

            gui_texts.push(GUIText {
                text_information: TextInformation::new(self.fonts[text_font].clone())
                    .text(text_text)
                    .font_size(text_font_size)
                    .newline_distance(text_newline_size)
                    .set_buffers(base),
                position,
                scale,
                clip_min,
                clip_max,
                absolute_position,
                absolute_scale,
                color,
            })
        }
        self.gui_texts = gui_texts;

        let mut gui_quads = Vec::new();
        for quad in json["quads"].members() {
            let mut position = Vector::new_empty();
            if let JsonValue::Array(ref position_json) = quad["position"] {
                if position_json.len() >= 2 {
                    position = Vector::new_vec2(
                        position_json[0].as_f32().unwrap(),
                        position_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut scale = Vector::new_empty();
            if let JsonValue::Array(ref scale_json) = quad["scale"] {
                if scale_json.len() >= 2 {
                    scale = Vector::new_vec2(
                        scale_json[0].as_f32().unwrap(),
                        scale_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut clip_min = Vector::new_empty();
            if let JsonValue::Array(ref clip_min_json) = quad["clip_min"] {
                if clip_min_json.len() >= 2 {
                    clip_min = Vector::new_vec2(
                        clip_min_json[0].as_f32().unwrap(),
                        clip_min_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut clip_max = Vector::new_empty();
            if let JsonValue::Array(ref clip_max_json) = quad["clip_max"] {
                if clip_max_json.len() >= 2 {
                    clip_max = Vector::new_vec2(
                        clip_max_json[0].as_f32().unwrap(),
                        clip_max_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut absolute_position = false;
            if let JsonValue::Boolean(ref absolute_position_json) = quad["absolute_position"] {
                absolute_position = *absolute_position_json;
            }

            let mut absolute_scale = false;
            if let JsonValue::Boolean(ref absolute_scale_json) = quad["absolute_scale"] {
                absolute_scale = *absolute_scale_json;
            }

            let mut color = Vector::new_empty();
            if let JsonValue::Array(ref color_json) = quad["color"] {
                if color_json.len() >= 4 {
                    color = Vector::new_vec4(
                        color_json[0].as_f32().unwrap(),
                        color_json[1].as_f32().unwrap(),
                        color_json[2].as_f32().unwrap(),
                        color_json[3].as_f32().unwrap(),
                    );
                }
            }

            gui_quads.push(GUIQuad {
                position,
                scale,
                clip_min,
                clip_max,
                absolute_position,
                absolute_scale,
                color,
            });
        }
        self.gui_quads = gui_quads;
    }

    pub unsafe fn draw(&mut self, current_frame: usize, command_buffer: vk::CommandBuffer,) { unsafe {
        self.device.cmd_begin_render_pass(
            command_buffer,
            &self.pass.borrow().get_pass_begin_info(current_frame, None, self.text_renderer.renderpass.scissor),
            vk::SubpassContents::INLINE,
        );

        for node_index in &self.gui_root_node_indices.clone() {
            self.draw_node(
                *node_index,
                current_frame,
                command_buffer,
                Vector::new_vec(0.0),
                Vector::new_vec2(self.window().inner_size().width as f32, self.window().inner_size().height as f32)
            );
        }

        self.device.cmd_end_render_pass(command_buffer);
        self.pass.borrow().transition_to_readable(command_buffer, current_frame);
    } }
    unsafe fn draw_node(
        &mut self,
        node_index: usize,
        current_frame: usize,
        command_buffer: vk::CommandBuffer,
        parent_position: Vector,
        parent_scale: Vector,
    ) { unsafe {
        let node = self.gui_nodes[node_index].clone();
        if node.hidden { return };

        let position = parent_position + if node.absolute_position { node.position } else { node.position * parent_scale };
        let scale = if node.absolute_scale { node.scale } else { parent_scale * node.scale };

        if let Some(quad_index) = &node.quad {
            self.draw_quad(*quad_index, current_frame, command_buffer, position, scale);
        }
        if let Some(text_index) = &node.text {
            self.draw_text(*text_index, current_frame, command_buffer, position, scale);
        }

        for child in &node.children_indices.clone() {
            self.draw_node(*child, current_frame, command_buffer, position, scale);
        }

        if node.interactable_information.is_some() {
            self.handle_gui_interaction(node, node_index, position, position + scale)
        }
    } }
    unsafe fn draw_quad(
        &self,
        quad_index: usize,
        current_frame: usize,
        command_buffer: vk::CommandBuffer,
        position: Vector,
        scale: Vector,
    ) { unsafe {
        let quad = &self.gui_quads[quad_index];
        let clip_min = position + scale * quad.clip_min; // + if quad.absolute_clip_min { quad.clip_min } else { scale * quad.clip_min }
        let clip_max = position + scale * quad.clip_max; // + if quad.absolute_clip_max { quad.clip_max } else { scale * quad.clip_max }
        let position = position + if quad.absolute_position { quad.position } else { quad.position * scale };
        let scale = if quad.absolute_scale { quad.scale } else { scale * quad.scale };

        let quad_constants = GUIQuadSendable {
            color: quad.color.to_array4(),
            resolution: [self.quad_renderpass.viewport.width as i32, self.quad_renderpass.viewport.height as i32],
            clip_min: clip_min.to_array2(),
            clip_max: clip_max.to_array2(),
            position: position.to_array2(),
            scale: scale.to_array2(),
            _pad: [0.0; 2],
        };

        let device = self.device.clone();
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
    } }
    unsafe fn draw_text(
        &self,
        text_index: usize,
        current_frame: usize,
        command_buffer: vk::CommandBuffer,
        position: Vector,
        scale: Vector,
    ) { unsafe {
        let text = &self.gui_texts[text_index];

        let clip_min = position + scale * text.clip_min; // + if quad.absolute_clip_min { quad.clip_min } else { scale * quad.clip_min }
        let clip_max = position + scale * text.clip_max; // + if quad.absolute_clip_max { quad.clip_max } else { scale * quad.clip_max }
        let position = position + if text.absolute_position { text.position } else { text.position * scale };
        let scale = if text.absolute_scale { text.scale } else { scale * text.scale };

        self.text_renderer.draw_gui_text(current_frame, &text.text_information, position, scale, clip_min, clip_max);
    } }

    pub fn update_text_of_node(&mut self, node_index: usize, text: &str, command_buffer: vk::CommandBuffer) {
        let node_text_index = self.gui_nodes[node_index].text.expect("text index parse error");
        let node_text_information = &mut self.gui_texts[node_text_index].text_information;
        node_text_information.update_text(text);
        node_text_information.update_buffers_all_frames(command_buffer);
    }

    pub unsafe fn destroy(&mut self) { unsafe {
        self.text_renderer.destroy();
        self.quad_renderpass.destroy();
        for node in &self.gui_nodes {
            if let Some(text_index) = &node.text {
                self.gui_texts[*text_index].text_information.destroy();
            }
        }
    } }
}

/**
* Position and scale are relative and normalized.
*/
#[derive(Clone)]
pub struct GUINode {
    pub index: usize,
    pub name: String,
    pub interactable_information: Option<GUIInteractableInformation>,
    pub hidden: bool,
    pub position: Vector,
    pub scale: Vector,
    pub children_indices: Vec<usize>,
    pub absolute_position: bool,
    pub absolute_scale: bool,

    pub text: Option<usize>,
    pub quad: Option<usize>
}
#[derive(Clone)]
pub struct GUIInteractableInformation {
    pub was_pressed_last_frame: bool,

    pub hover_action: Option<String>,
    pub unhover_action: Option<String>,
    pub left_tap_action: Option<String>,
    pub left_hold_action: Option<String>,
    pub right_tap_action: Option<String>,
    pub right_hold_action: Option<String>,
    pub hitbox_diversion: Option<usize>,
}
/**
* Position and scale are relative and normalized.
*/
pub struct GUIQuad {
    pub position: Vector,
    pub scale: Vector,
    pub clip_min: Vector,
    pub clip_max: Vector,
    pub absolute_position: bool,
    pub absolute_scale: bool,

    pub color: Vector,
}
pub struct GUIText {
    pub text_information: TextInformation,

    pub position: Vector,
    pub scale: Vector,
    pub clip_min: Vector,
    pub clip_max: Vector,
    pub absolute_position: bool,
    pub absolute_scale: bool,
    pub color: Vector,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct GUIQuadSendable {
    pub color: [f32; 4],

    pub resolution: [i32; 2],

    pub clip_min: [f32; 2],
    pub clip_max: [f32; 2],

    pub position: [f32; 2],

    pub scale: [f32; 2],

    pub _pad: [f32; 2],
}