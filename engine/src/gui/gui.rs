use std::cell::RefCell;
use std::{fs, slice};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use ash::vk;
use ash::vk::{CommandBuffer, DescriptorType, Format, Handle, ShaderStageFlags};
use json::JsonValue;
use mlua::{UserData, UserDataFields, UserDataMethods};
use winit::event::MouseButton;
use crate::app::get_command_buffer;
use crate::client::client::*;
use crate::math::*;
use crate::scene::physics::player::MovementMode;
use crate::render::render_helper::{Descriptor, DescriptorCreateInfo, DescriptorSetCreateInfo, Pass, PassCreateInfo, Renderpass, RenderpassCreateInfo, Texture, TextureCreateInfo};
use crate::gui::text::font::Font;
use crate::gui::text::text_render::{TextInformation, TextRenderer};
use crate::render::render::MAX_FRAMES_IN_FLIGHT;
use crate::render::vulkan_base::VkBase;
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

    pub gui_nodes: Vec<GUINode>,
    pub gui_root_node_indices: Vec<usize>,

    pub gui_quads: Vec<GUIQuad>,
    pub gui_texts: Vec<GUIText>,
    pub gui_images: Vec<GUIImage>,

    pub interactable_node_indices: Vec<usize>,

    pub fonts: Vec<Arc<Font>>,

    pub active_node: usize,

    new_texts: Vec<usize>
}
impl GUI {
    fn handle_gui_interaction(
        &mut self,
        node_index: usize,
        min: Vector,
        max: Vector,
        has_triggered_hover: &mut bool,
    ) -> GUIInteractionResult {
        let mut result = GUIInteractionResult::None;

        let node = &mut self.gui_nodes[node_index];
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

        let hovered = if
        x > min.x && x < max.x &&
            y > min.y && y < max.y
        { true } else { false };

        if !hovered {
            for unhover_action in interactable_information.unhover_actions.iter() {
                Lua::cache_call(
                    unhover_action.1,
                    unhover_action.0.as_str(),
                    Some(self.active_node),
                    Some(self.index)
                )
            }
        } else {
            if !*has_triggered_hover {
                *has_triggered_hover = true;
                for hover_action in interactable_information.hover_actions.iter() {
                    Lua::cache_call(
                        hover_action.1,
                        hover_action.0.as_str(),
                        Some(self.active_node),
                        Some(self.index)
                    )
                }
            }
        }

        if left_just_pressed && hovered {
            interactable_information.was_initially_pressed = true;
        }

        // discard any buttons that happen to be hovered over while holding another down
        if !interactable_information.was_initially_pressed {
            return result;
        }

        if left_pressed {
            for left_hold_action in interactable_information.left_hold_actions.clone().iter() {
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
                for left_tap_action in interactable_information.left_tap_actions.clone().iter() {
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

    pub unsafe fn new(index: usize, base: &VkBase, controller: Arc<RefCell<Client>>, null_tex_sampler: vk::Sampler, null_tex_img_view: vk::ImageView) -> GUI { unsafe {
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

            gui_nodes: Vec::new(),
            gui_root_node_indices: Vec::new(),

            gui_quads: Vec::new(),
            gui_texts: Vec::new(),
            gui_images: Vec::new(),

            interactable_node_indices: Vec::new(),

            fonts: vec![text_renderer.default_font.clone()],

            text_renderer,

            active_node: 0,

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
    pub unsafe fn load_from_file(&mut self, base: &VkBase, path: &str) {
        for text in self.gui_texts.iter() {
            text.text_information.as_ref().unwrap().destroy();
        }
        unsafe {
            for image in self.gui_images.iter() {
                image.destroy(&self.device);
            }
        }
        self.gui_images.clear();

        let json = json::parse(fs::read_to_string(path).expect("failed to load json file").as_str()).expect("json parse error");

        let mut script_uris = Vec::new();
        for script in json["scripts"].members() {
            if let JsonValue::String(ref uri_json) = script["uri"] {
                let path = Path::new(uri_json);
                script_uris.push(path);
            }
        }
        let script_indices = Lua::load_scripts(script_uris).expect("script loading error");

        let mut fonts = Vec::new();
        for font in json["fonts"].members() {
            let mut uri = String::from("engine\\resources\\fonts\\Oxygen-Regular.ttf");
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
        unsafe { self.set_fonts(fonts) };

        let mut image_uris = Vec::new();
        let mut image_alpha_thresholds = Vec::new();
        for image in json["images"].members() {
            let mut uri = None;
            if let JsonValue::String(ref uri_json) = image["uri"] {
                uri = Some((*uri_json).as_str());
            }
            image_uris.push(uri);

            let mut alpha_threshold = 0.1;
            if let JsonValue::Number(ref alpha_threshold_json) = image["alpha_threshold"] {
                if let Ok(v) = alpha_threshold_json.to_string().parse::<f32>() {
                    alpha_threshold = v;
                }
            }
            image_alpha_thresholds.push(alpha_threshold);
        }
        unsafe {
            let uris: Vec<PathBuf> = image_uris.iter().map(|uri| { PathBuf::from(uri.expect("gui image did not have uri")) }).collect();
            let textures = base.load_textures_batched(uris.as_slice(), true);
            let gui_images: Vec<GUIImage> = textures.iter().enumerate().map(|(i, texture)| {
                GUIImage {
                    image: texture.1.0,
                    image_view: texture.0.0,
                    memory: texture.1.1,
                    sampler: texture.0.1,

                    alpha_threshold: image_alpha_thresholds[i]
                }
            }).collect();

            self.gui_images = gui_images;

            self.update_descriptors(base);

        };

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

            let mut position = Vector::empty();
            if let JsonValue::Array(ref position_json) = text["position"] {
                if position_json.len() >= 2 {
                    position = Vector::new2(
                        position_json[0].as_f32().unwrap(),
                        position_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut scale = Vector::empty();
            if let JsonValue::Array(ref scale_json) = text["scale"] {
                if scale_json.len() >= 2 {
                    scale = Vector::new2(
                        scale_json[0].as_f32().unwrap(),
                        scale_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut clip_min = Vector::empty();
            if let JsonValue::Array(ref clip_min_json) = text["clip_min"] {
                if clip_min_json.len() >= 2 {
                    clip_min = Vector::new2(
                        clip_min_json[0].as_f32().unwrap(),
                        clip_min_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut clip_max = Vector::empty();
            if let JsonValue::Array(ref clip_max_json) = text["clip_max"] {
                if clip_max_json.len() >= 2 {
                    clip_max = Vector::new2(
                        clip_max_json[0].as_f32().unwrap(),
                        clip_max_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut absolute_scale = (false, false);
            if let JsonValue::Array(ref absolute_scale_json) = text["absolute_scale"] {
                if absolute_scale_json.len() >= 2 {
                    absolute_scale = (
                        absolute_scale_json[0].as_bool().unwrap(),
                        absolute_scale_json[1].as_bool().unwrap(),
                    )
                }
            }

            let mut absolute_position = (false, false);
            if let JsonValue::Array(ref absolute_position_json) = text["absolute_position"] {
                if absolute_position_json.len() >= 2 {
                    absolute_position = (
                        absolute_position_json[0].as_bool().unwrap(),
                        absolute_position_json[1].as_bool().unwrap(),
                    )
                }
            }

            let mut anchor_point = AnchorPoint::default();
            match &text["anchor_point"] {
                JsonValue::String(s) => {
                    anchor_point = AnchorPoint::from_string(s.as_str());
                }
                JsonValue::Short(s) => {
                    anchor_point = AnchorPoint::from_string(s.as_str());
                }
                _ => ()
            }

            let mut color = Vector::empty();
            if let JsonValue::Array(ref color_json) = text["color"] {
                if color_json.len() >= 4 {
                    color = Vector::new4(
                        color_json[0].as_f32().unwrap(),
                        color_json[1].as_f32().unwrap(),
                        color_json[2].as_f32().unwrap(),
                        color_json[3].as_f32().unwrap(),
                    );
                }
            }

            gui_texts.push(GUIText {
                text_information: Some(TextInformation::new(self.fonts[text_font].clone())
                    .text(text_text)
                    .font_size(text_font_size)
                    .newline_distance(text_newline_size)
                    .build_set_buffers(base)),
                position,
                scale,
                clip_min,
                clip_max,
                absolute_position,
                absolute_scale,
                anchor_point,
                color,
            })
        }
        self.gui_texts = gui_texts;

        let mut gui_quads = Vec::new();
        for quad in json["quads"].members() {
            let mut position = Vector::empty();
            if let JsonValue::Array(ref position_json) = quad["position"] {
                if position_json.len() >= 2 {
                    position = Vector::new2(
                        position_json[0].as_f32().unwrap(),
                        position_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut scale = Vector::empty();
            if let JsonValue::Array(ref scale_json) = quad["scale"] {
                if scale_json.len() >= 2 {
                    scale = Vector::new2(
                        scale_json[0].as_f32().unwrap(),
                        scale_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut clip_min = Vector::empty();
            if let JsonValue::Array(ref clip_min_json) = quad["clip_min"] {
                if clip_min_json.len() >= 2 {
                    clip_min = Vector::new2(
                        clip_min_json[0].as_f32().unwrap(),
                        clip_min_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut clip_max = Vector::empty();
            if let JsonValue::Array(ref clip_max_json) = quad["clip_max"] {
                if clip_max_json.len() >= 2 {
                    clip_max = Vector::new2(
                        clip_max_json[0].as_f32().unwrap(),
                        clip_max_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut absolute_scale = (false, false);
            if let JsonValue::Array(ref absolute_scale_json) = quad["absolute_scale"] {
                if absolute_scale_json.len() >= 2 {
                    absolute_scale = (
                        absolute_scale_json[0].as_bool().unwrap(),
                        absolute_scale_json[1].as_bool().unwrap(),
                    )
                }
            }

            let mut absolute_position = (false, false);
            if let JsonValue::Array(ref absolute_position_json) = quad["absolute_position"] {
                if absolute_position_json.len() >= 2 {
                    absolute_position = (
                        absolute_position_json[0].as_bool().unwrap(),
                        absolute_position_json[1].as_bool().unwrap(),
                    )
                }
            }

            let mut anchor_point = AnchorPoint::default();
            match &quad["anchor_point"] {
                JsonValue::String(s) => {
                    anchor_point = AnchorPoint::from_string(s.as_str());
                }
                JsonValue::Short(s) => {
                    anchor_point = AnchorPoint::from_string(s.as_str());
                }
                _ => ()
            }

            let mut color = Vector::empty();
            if let JsonValue::Array(ref color_json) = quad["color"] {
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
            if let JsonValue::Number(ref corner_radius_json) = quad["corner_radius"] {
                if let Ok(v) = corner_radius_json.to_string().parse::<f32>() {
                    corner_radius = v;
                }
            }

            let mut image = None;
            if let JsonValue::Number(ref image_json) = quad["image"] {
                if let Ok(v) = image_json.to_string().parse::<i32>() {
                    image = Some(v);
                }
            }

            gui_quads.push(GUIQuad {
                position,
                scale,
                clip_min,
                clip_max,
                absolute_position,
                absolute_scale,
                anchor_point,
                color,
                corner_radius,
                image
            })
        }
        self.gui_quads = gui_quads;

        let mut nodes = Vec::new();
        for node in json["nodes"].members() {
            let mut name = String::from("unnamed node");
            if let JsonValue::String(ref name_json) = node["name"] {
                name = (*name_json).parse().expect("node name parse error");
            }

            let mut interactable_information = None;
            if let JsonValue::Object(ref interactable_information_json) = node["interactable_information"] {
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
                                script_indices[method["script"].as_usize().expect("interactable passive_action index parse error")]
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
                                script_indices[method["script"].as_usize().expect("interactable hover_action index parse error")]
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
                                script_indices[method["script"].as_usize().expect("interactable unhover_action index parse error")]
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
                                script_indices[method["script"].as_usize().expect("interactable left_tap_action index parse error")]
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
                                script_indices[method["script"].as_usize().expect("interactable right_tap_actions index parse error")]
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
                                script_indices[method["script"].as_usize().expect("interactable left_hold_actions index parse error")]
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
                                script_indices[method["script"].as_usize().expect("interactable right_hold_actions index parse error")]
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
            if let JsonValue::Boolean(ref hidden_json) = node["hidden"] {
                hidden = *hidden_json;
            }

            let mut children_indices = Vec::new();
            if let JsonValue::Array(ref children_json) = node["children"] {
                for child_json in children_json {
                    children_indices.push(child_json.as_usize().expect("node child index parse error"));
                }
            }

            let mut position = Vector::empty();
            if let JsonValue::Array(ref position_json) = node["position"] {
                if position_json.len() >= 2 {
                    position = Vector::new2(
                        position_json[0].as_f32().unwrap(),
                        position_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut scale = Vector::empty();
            if let JsonValue::Array(ref scale_json) = node["scale"] {
                if scale_json.len() >= 2 {
                    scale = Vector::new2(
                        scale_json[0].as_f32().unwrap(),
                        scale_json[1].as_f32().unwrap(),
                    );
                }
            }

            let mut absolute_scale = (false, false);
            if let JsonValue::Array(ref absolute_scale_json) = node["absolute_scale"] {
                if absolute_scale_json.len() >= 2 {
                    absolute_scale = (
                        absolute_scale_json[0].as_bool().unwrap(),
                        absolute_scale_json[1].as_bool().unwrap(),
                    )
                }
            }

            let mut absolute_position = (false, false);
            if let JsonValue::Array(ref absolute_position_json) = node["absolute_position"] {
                if absolute_position_json.len() >= 2 {
                    absolute_position = (
                        absolute_position_json[0].as_bool().unwrap(),
                        absolute_position_json[1].as_bool().unwrap(),
                    )
                }
            }

            let mut anchor_point = AnchorPoint::default();
            match &node["anchor_point"] {
                JsonValue::String(s) => {
                    anchor_point = AnchorPoint::from_string(s.as_str());
                }
                JsonValue::Short(s) => {
                    anchor_point = AnchorPoint::from_string(s.as_str());
                }
                _ => ()
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
                anchor_point,
                text,
                quad,
            })
        }
        self.gui_nodes = nodes;
    }
    unsafe fn update_descriptors(&mut self, base: &VkBase) {
        let mut image_infos: Vec<vk::DescriptorImageInfo> = Vec::with_capacity(1024);

        for texture in &self.gui_images {
            image_infos.push(vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image_view: texture.image_view,
                sampler: texture.sampler,
            })
        }
        let missing = 1024 - image_infos.len();
        for _ in 0..missing {
            image_infos.push(self.null_tex_info.clone());
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
        let mut new_text = GUIText::default();
        new_text.text_information.as_mut().unwrap().text = text;
        self.new_texts.push(self.gui_texts.len());
        self.gui_texts.push(GUIText::default());
    }
    pub unsafe fn initialize_new_texts(&mut self, base: &VkBase) {
        for new_text in self.new_texts.drain(..) {
            if let Some(info) = self.gui_texts[new_text].text_information.as_mut() {
                info.set_buffers(base);
            }
        }
    }

    pub unsafe fn draw(&mut self, current_frame: usize, command_buffer: CommandBuffer,) { unsafe {
        let mut interactable_action_parameter_sets = Vec::new();

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
                Vector::fill(0.0),
                Vector::new2(self.window.inner_size().width as f32, self.window.inner_size().height as f32),
                &mut interactable_action_parameter_sets,
            );
        }

        self.device.cmd_end_render_pass(command_buffer);
        self.pass.borrow().transition_to_readable(command_buffer, current_frame);

        let mut has_hover_actioned = false;
        for parameter_set in interactable_action_parameter_sets.iter().rev() {
            self.active_node = parameter_set.0;
            match self.handle_gui_interaction(parameter_set.0, parameter_set.1, parameter_set.2, &mut has_hover_actioned) {
                GUIInteractionResult::None => (),
                _ => {
                    return
                }
            }
        }
    } }
    unsafe fn draw_node(
        &mut self,
        node_index: usize,
        current_frame: usize,
        command_buffer: CommandBuffer,
        parent_position: Vector,
        parent_scale: Vector,
        interactable_parameter_sets: &mut Vec<(usize, Vector, Vector)>
    ) { unsafe {
        let node = &mut self.gui_nodes[node_index].clone();
        if node.hidden { return };

        /*
        let position = parent_position + if node.absolute_position { node.position } else { node.position * parent_scale };
        let scale = if node.absolute_scale { node.scale } else { parent_scale * node.scale };
         */
        let offset_factor = node.anchor_point.offset_factor();
        let scale = Vector::new2(
            if node.absolute_scale.0 { node.scale.x } else { parent_scale.x * node.scale.x },
            if node.absolute_scale.1 { node.scale.y } else { parent_scale.y * node.scale.y }
        );
        let position = offset_factor * parent_scale - scale * offset_factor
            + parent_position
            + Vector::new2(
                if node.absolute_position.0 { node.position.x } else { parent_scale.x * node.position.x },
                if node.absolute_position.1 { node.position.y } else { parent_scale.y * node.position.y }
            );

        if let Some(quad) = &node.quad {
            self.draw_quad(*quad, current_frame, command_buffer, position, scale);
        }
        if let Some(text) = &node.text {
            self.draw_text(*text, current_frame, position, scale);
        }

        if node.interactable_information.is_some() {
            interactable_parameter_sets.push((node_index, position, position + scale));
        }

        for child in &node.children_indices.clone() {
            self.draw_node(*child, current_frame, command_buffer, position, scale, interactable_parameter_sets);
        }
    } }
    unsafe fn draw_quad(
        &self,
        quad: usize,
        current_frame: usize,
        command_buffer: CommandBuffer,
        parent_position: Vector,
        parent_scale: Vector,
    ) { unsafe {
        let quad = &self.gui_quads[quad];
        let clip_min = parent_position + parent_scale * quad.clip_min; // + if quad.absolute_clip_min { quad.clip_min } else { scale * quad.clip_min }
        let clip_max = parent_position + parent_scale * quad.clip_max; // + if quad.absolute_clip_max { quad.clip_max } else { scale * quad.clip_max }

        let offset_factor = quad.anchor_point.offset_factor();
        let scale = Vector::new2(
            if quad.absolute_scale.0 { quad.scale.x } else { parent_scale.x * quad.scale.x },
            if quad.absolute_scale.1 { quad.scale.y } else { parent_scale.y * quad.scale.y }
        );
        let position = offset_factor * parent_scale - scale * offset_factor
            + parent_position
            + Vector::new2(
            if quad.absolute_position.0 { quad.position.x } else { parent_scale.x * quad.position.x },
            if quad.absolute_position.1 { quad.position.y } else { parent_scale.y * quad.position.y }
        );

        let quad_constants = GUIQuadSendable {
            color: quad.color.to_array4(),
            resolution: [self.quad_renderpass.viewport.width as i32, self.quad_renderpass.viewport.height as i32],
            clip_min: clip_min.to_array2(),
            clip_max: clip_max.to_array2(),
            position: position.to_array2(),
            scale: scale.to_array2(),
            corner_radius: quad.corner_radius,
            image: quad.image.unwrap_or(-1)
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
        text: usize,
        current_frame: usize,
        parent_position: Vector,
        parent_scale: Vector,
    ) { unsafe {
        let text = &self.gui_texts[text];

        let clip_min = parent_position + parent_scale * text.clip_min; // + if quad.absolute_clip_min { quad.clip_min } else { scale * quad.clip_min }
        let clip_max = parent_position + parent_scale * text.clip_max; // + if quad.absolute_clip_max { quad.clip_max } else { scale * quad.clip_max }

        let offset_factor = text.anchor_point.offset_factor();
        let scale = Vector::new2(
            if text.absolute_scale.0 { text.scale.x } else { parent_scale.x * text.scale.x },
            if text.absolute_scale.1 { text.scale.y } else { parent_scale.y * text.scale.y }
        );
        let position = offset_factor * parent_scale - scale * offset_factor
            + parent_position
            + Vector::new2(
            if text.absolute_position.0 { text.position.x } else { parent_scale.x * text.position.x },
            if text.absolute_position.1 { text.position.y } else { parent_scale.y * text.position.y }
        );

        self.text_renderer.draw_gui_text(current_frame, &text.text_information.as_ref().unwrap(), position, scale, clip_min, clip_max);
    } }

    pub unsafe fn destroy(&mut self) { unsafe {
        self.text_renderer.destroy();
        self.quad_renderpass.destroy();
        for text in &mut self.gui_texts {
            text.text_information.as_mut().unwrap().destroy();
        }
        for font in &self.fonts {
            font.destroy();
        }
        for image in &self.gui_images {
            image.destroy(&self.device)
        }
    } }
}

#[derive(Clone)]
pub enum AnchorPoint {
    TopLeft,
    TopMiddle,
    TopRight,
    BottomLeft,
    BottomMiddle,
    BottomRight,
    Right,
    Left,
    Center
}
impl AnchorPoint {
    pub fn from_string(string: &str) -> AnchorPoint {
        match string {
            "top_left" => AnchorPoint::TopLeft,
            "top_middle" => AnchorPoint::TopMiddle,
            "top_right" => AnchorPoint::TopRight,
            "bottom_left" => AnchorPoint::BottomLeft,
            "bottom_middle" => AnchorPoint::BottomMiddle,
            "bottom_right" => AnchorPoint::BottomRight,
            "right" => AnchorPoint::Right,
            "left" => AnchorPoint::Left,
            "center" => AnchorPoint::Center,
            _ => AnchorPoint::default()
        }
    }
    pub fn offset_factor(&self) -> Vector {
        let (rx, ry) = match self {
            AnchorPoint::BottomLeft   => (0.0, 0.0),
            AnchorPoint::BottomMiddle => (0.5, 0.0),
            AnchorPoint::BottomRight => (1.0, 0.0),
            AnchorPoint::TopLeft => (0.0, 1.0),
            AnchorPoint::TopMiddle => (0.5, 1.0),
            AnchorPoint::TopRight => (1.0, 1.0),
            AnchorPoint::Left => (0.0, 0.5),
            AnchorPoint::Right => (1.0, 0.5),
            AnchorPoint::Center => (0.5, 0.5),
        };
        Vector::new2(rx, ry)
    }
}
impl Default for AnchorPoint {
    fn default() -> Self { AnchorPoint::BottomLeft }
}

struct GUIImage {
    image_view: vk::ImageView,
    sampler: vk::Sampler,
    image: vk::Image,
    memory: vk::DeviceMemory,

    alpha_threshold: f32,
}
impl GUIImage {
    unsafe fn destroy(&self, device: &ash::Device) { unsafe {
        device.destroy_sampler(self.sampler, None);
        device.destroy_image_view(self.image_view, None);
        device.destroy_image(self.image, None);
        device.free_memory(self.memory, None);
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
    pub absolute_position: (bool, bool),
    pub absolute_scale: (bool, bool),
    pub anchor_point: AnchorPoint,

    pub text: Option<usize>,
    pub quad: Option<usize>
}
impl GUINode {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            name: String::from(""),
            interactable_information: None,
            hidden: false,
            position: Vector::empty(),
            scale: Vector::empty(),
            children_indices: Vec::new(),
            absolute_position: (false, false),
            absolute_scale: (false, false),
            anchor_point: AnchorPoint::default(),
            text: None,
            quad: None,
        }
    }
}
#[derive(Clone)]
pub struct GUIInteractableInformation {
    was_initially_pressed: bool,

    passive_actions: Vec<(String, usize)>,
    hover_actions: Vec<(String, usize)>,
    unhover_actions: Vec<(String, usize)>,
    left_tap_actions: Vec<(String, usize)>,
    left_hold_actions: Vec<(String, usize)>,
    right_tap_actions: Vec<(String, usize)>,
    right_hold_actions: Vec<(String, usize)>,
}

/**
* Position and scale are relative and normalized.
*/
#[derive(Clone)]
pub struct GUIQuad {
    pub position: Vector,
    pub scale: Vector,
    pub clip_min: Vector,
    pub clip_max: Vector,
    pub absolute_position: (bool, bool),
    pub absolute_scale: (bool, bool),
    pub anchor_point: AnchorPoint,
    pub color: Vector,
    pub corner_radius: f32,
    image: Option<i32>
}
impl Default for GUIQuad {
    fn default() -> Self {
        GUIQuad {
            position: Default::default(),
            scale: Default::default(),
            clip_min: Default::default(),
            clip_max: Default::default(),
            absolute_position: (false, false),
            absolute_scale: (false, false),
            anchor_point: AnchorPoint::default(),
            color: Default::default(),
            corner_radius: 0.0,
            image: None
        }
    }
}

pub struct GUIText {
    pub text_information: Option<TextInformation>,

    pub position: Vector,
    pub scale: Vector,
    pub clip_min: Vector,
    pub clip_max: Vector,
    pub absolute_position: (bool, bool),
    pub absolute_scale: (bool, bool),
    pub anchor_point: AnchorPoint,
    pub color: Vector,
}
impl GUIText {
    pub fn update_text(&mut self, text: &str) {
        let command_buffer = get_command_buffer();

        self.text_information.as_mut().unwrap().update_text(text);
        self.text_information.as_mut().unwrap().update_buffers_all_frames(command_buffer);
    }
}
impl Default for GUIText {
    fn default() -> Self {
        GUIText {
            text_information: None,
            position: Default::default(),
            scale: Default::default(),
            clip_min: Default::default(),
            clip_max: Default::default(),
            absolute_position: (false, false),
            absolute_scale: (false, false),
            anchor_point: AnchorPoint::default(),
            color: Default::default(),
        }
    }
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

    pub corner_radius: f32,

    pub image: i32,
}