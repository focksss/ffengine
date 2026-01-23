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
use crate::render::text::font::Font;
use crate::render::text::text_render::{TextInformation, TextRenderer};
use crate::render::render::MAX_FRAMES_IN_FLIGHT;
use crate::render::vulkan_base::Context;
use crate::scripting::lua_engine::{Field, Lua};

pub struct UiRenderer {
    context: Arc<Context>,

    pub gui_root_sets: Vec<Vec<usize>>,

    pub text_field_focused: bool,

    controller: Arc<RefCell<Client>>,
    null_tex_info: vk::DescriptorImageInfo,

    pub pass: Arc<RefCell<Pass>>,
    pub text_renderer: TextRenderer,
    pub quad_renderpass: Renderpass,

    pub image_count: usize,

    pub fonts: Vec<Arc<Font>>,

    pub hovered_nodes: HashSet<usize>,

    new_texts: Vec<usize>
}
impl UiRenderer {
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
                passive_action.2,
                passive_action.0.as_str(),
                Some(self.active_node),
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
                    unhover_action.2,
                    unhover_action.0.as_str(),
                    Some(self.active_node),
                )
            }
        } else {
            self.hovered_nodes.insert(node_index);
            for hover_action in interactable_information.hover_actions.iter() {
                Lua::cache_call(
                    hover_action.1,
                    hover_action.2,
                    hover_action.0.as_str(),
                    Some(self.active_node),
                )
            }
        }

        loop {
            if left_just_pressed && hovered {
                for left_down_action in interactable_information.left_down_actions.iter() {
                    Lua::cache_call(
                        left_down_action.1,
                        left_down_action.2,
                        left_down_action.0.as_str(),
                        Some(self.active_node),
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
                        left_hold_action.2,
                        left_hold_action.0.as_str(),
                        Some(self.active_node),
                    )
                }
                break;
            } else {
                if hovered {
                    for left_tap_action in interactable_information.left_up_actions.iter() {
                        Lua::cache_call(
                            left_tap_action.1,
                            left_tap_action.2,
                            left_tap_action.0.as_str(),
                            Some(self.active_node),
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
                        right_down_action.2,
                        right_down_action.0.as_str(),
                        Some(self.active_node),
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
                        right_hold_action.2,
                        right_hold_action.0.as_str(),
                        Some(self.active_node),
                    )
                }
                break;
            } else {
                if hovered {
                    for right_tap_action in interactable_information.right_up_actions.iter() {
                        Lua::cache_call(
                            right_tap_action.1,
                            right_tap_action.2,
                            right_tap_action.0.as_str(),
                            Some(self.active_node),
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
            gui_root_sets: Vec::new(),

            text_field_focused: false,

            context: context.clone(),
            controller,
            null_tex_info: null_info,

            pass: pass_ref.clone(),
            quad_renderpass,

            nodes: Vec::new(),
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