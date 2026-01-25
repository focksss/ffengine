use std::cell::RefCell;
use std::{fs, slice};
use std::collections::HashSet;
use std::sync::Arc;
use ash::vk;
use ash::vk::{CommandBuffer, DescriptorType, Format, ImageLayout, ShaderStageFlags};
use mlua::IntoLua;
use winit::event::MouseButton;
use winit::keyboard::{Key, PhysicalKey, SmolStr};
use crate::client::client::*;
use crate::math::*;
use crate::render::render_helper::{Descriptor, DescriptorCreateInfo, DescriptorSetCreateInfo, Pass, PassCreateInfo, PipelineCreateInfo, Renderpass, RenderpassCreateInfo, Texture, TextureCreateInfo};
use crate::render::text::font::Font;
use crate::render::text::text_render::{TextInformation, TextRenderer};
use crate::render::render::MAX_FRAMES_IN_FLIGHT;
use crate::render::vulkan_base::Context;
use crate::scene::scene::Scene;
use crate::scene::ui::text::Text;
use crate::scripting::lua_engine::{Field, Lua};

pub struct UiRenderer {
    context: Arc<Context>,

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
        scene: &mut Scene,
        node_index: usize,
        min: Vector,
        max: Vector,
        can_trigger_left_click_events: &mut bool,
        can_trigger_right_click_events: &mut bool,
    ) {
        let node = &mut scene.entities[node_index];
        let interactable_information = &mut scene.ui_interactable_information[*node.ui_interactable_information.as_ref().unwrap()];

        for passive_action in interactable_information.passive_actions.iter() {
            let script = &scene.script_components[passive_action.script];

            Lua::call_method(
                script.script,
                script.instance,
                passive_action.method,
                &passive_action.args
            ).expect("failed to call Lua method");
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
                let script = &scene.script_components[unhover_action.script];

                Lua::call_method(
                    script.script,
                    script.instance,
                    unhover_action.method,
                    &unhover_action.args
                ).expect("failed to call Lua method");
            }
        } else {
            self.hovered_nodes.insert(node_index);
            for hover_action in interactable_information.hover_actions.iter() {
                let script = &scene.script_components[hover_action.script];

                Lua::call_method(
                    script.script,
                    script.instance,
                    hover_action.method,
                    &hover_action.args
                ).expect("failed to call Lua method");
            }
        }

        loop {
            if left_just_pressed && hovered {
                for left_down_action in interactable_information.left_down_actions.iter() {
                    let script = &scene.script_components[left_down_action.script];

                    Lua::call_method(
                        script.script,
                        script.instance,
                        left_down_action.method,
                        &left_down_action.args
                    ).expect("failed to call Lua method");
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
                    let script = &scene.script_components[left_hold_action.script];

                    Lua::call_method(
                        script.script,
                        script.instance,
                        left_hold_action.method,
                        &left_hold_action.args
                    ).expect("failed to call Lua method");
                }
                break;
            } else {
                if hovered {
                    for left_up_action in interactable_information.left_up_actions.iter() {
                        let script = &scene.script_components[left_up_action.script];

                        Lua::call_method(
                            script.script,
                            script.instance,
                            left_up_action.method,
                            &left_up_action.args
                        ).expect("failed to call Lua method");
                    }
                }
                interactable_information.was_initially_left_pressed = false;
            }
        }
        loop {
            if right_just_pressed && hovered {
                for right_down_action in interactable_information.right_down_actions.iter() {
                    let script = &scene.script_components[right_down_action.script];

                    Lua::call_method(
                        script.script,
                        script.instance,
                        right_down_action.method,
                        &right_down_action.args
                    ).expect("failed to call Lua method");
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
                    let script = &scene.script_components[right_hold_action.script];

                    Lua::call_method(
                        script.script,
                        script.instance,
                        right_hold_action.method,
                        &right_hold_action.args
                    ).expect("failed to call Lua method");
                }
                break;
            } else {
                if hovered {
                    for right_up_action in interactable_information.right_up_actions.iter() {
                        let script = &scene.script_components[right_up_action.script];

                        Lua::call_method(
                            script.script,
                            script.instance,
                            right_up_action.method,
                            &right_up_action.args
                        ).expect("failed to call Lua method");
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
        scene: &Scene,
        controller: Arc<RefCell<Client>>,
        null_tex_sampler: vk::Sampler,
        null_tex_img_view: vk::ImageView
    ) -> Self {
        let null_info = vk::DescriptorImageInfo {
            sampler: null_tex_sampler,
            image_view: null_tex_img_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        let (pass_ref, quad_renderpass, text_renderer) = Self::create_rendering_objects(context, null_info);

        let gui = Self {
            text_field_focused: false,

            context: context.clone(),
            controller,
            null_tex_info: null_info,

            pass: pass_ref.clone(),
            quad_renderpass,

            image_count: 0,

            fonts: vec![text_renderer.default_font.clone()],

            text_renderer,

            hovered_nodes: HashSet::new(),

            new_texts: Vec::new(),
        };
        gui.update_descriptors(scene);
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
    pub fn reload_rendering(&mut self, scene: &Scene, null_tex_sampler: vk::Sampler, null_tex_img_view: vk::ImageView) {
        let null_info = vk::DescriptorImageInfo {
            sampler: null_tex_sampler,
            image_view: null_tex_img_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        self.null_tex_info = null_info;
        self.text_renderer.destroy();
        self.quad_renderpass.destroy();
        (self.pass, self.quad_renderpass, self.text_renderer) = Self::create_rendering_objects(&self.context, self.null_tex_info);
        self.update_descriptors(scene);
    }

    /**
    * Uses custom JSON .gui files
    * * Refer to default.gui in resources/gui
    * * Nodes are drawn recursively and without depth testing. To make a node appear in front of another, define it after another.
    */
    fn update_descriptors(&self, scene: &Scene) {
        let mut image_infos: Vec<vk::DescriptorImageInfo> = Vec::with_capacity(1024);

        let mut borrowed_sampler = None;
        for element in scene.ui_images.iter() {
                if borrowed_sampler.is_none() {
                    borrowed_sampler = Some(element.sampler);
                }
                image_infos.push(vk::DescriptorImageInfo {
                    image_layout: ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    image_view: element.image_view,
                    sampler: element.sampler,
                })
        }

        unsafe {
            for frame in 0..MAX_FRAMES_IN_FLIGHT {
                let mut frame_image_infos = image_infos.clone();
                for texture in scene.ui_textures.iter() {
                    frame_image_infos.push(vk::DescriptorImageInfo {
                        image_layout: ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        image_view: texture.texture_set[frame].device_texture.borrow().image_view.clone(),
                        sampler: borrowed_sampler.unwrap(),
                    })
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

    pub fn add_text(&mut self, scene: &mut Scene, text: String) {
        let new_text = Text {
            text_information: Some(TextInformation::new(self.fonts[0].clone())
                .text(text.as_str())
                .font_size(17.0)
                .newline_distance(100.0)),
            font_index: 0,
            color: Vector::fill(1.0)
        };
        self.new_texts.push(scene.ui_texts.len());
        scene.ui_texts.push(new_text);
    }
    pub fn initialize_new_texts(&mut self, scene: &mut Scene) {
        for new_text in self.new_texts.drain(..) {
            if let Some(info) = &mut scene.ui_texts[new_text].text_information {
                info.set_buffers();
            }
        }
    }

    pub fn draw(&mut self, scene: &mut Scene, current_frame: usize, command_buffer: CommandBuffer,) { unsafe {
        let mut interactable_action_parameter_sets = Vec::new();

        self.pass.borrow().transition(
            command_buffer,
            current_frame,
            Some((ImageLayout::UNDEFINED, ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::AccessFlags::COLOR_ATTACHMENT_READ)),
            Some((ImageLayout::UNDEFINED, ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL, vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ)),
        );
        self.pass.borrow().begin(command_buffer, current_frame, &self.text_renderer.renderpass.scissor);

        let ui_root_entities = scene.ui_root_entities.clone();
        for node_index in ui_root_entities {
            self.draw_node(
                scene,
                node_index,
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
            self.handle_gui_interaction(
                scene,
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
        scene: &mut Scene,
        node_index: usize,
        current_frame: usize,
        command_buffer: CommandBuffer,
        interactable_parameter_sets: &mut Vec<(usize, Vector, Vector)>
    ) { unsafe {
        let entity = &scene.entities[node_index];
        let layout = &scene.ui_node_layouts[entity.ui_layout.unwrap()];
        if layout.hidden { return };

        if layout.clip_max.x < 0.0 || layout.clip_max.y < 0.0
            || layout.clip_min.x > self.quad_renderpass.viewport.width || layout.clip_min.y > self.quad_renderpass.viewport.height {
            return;
        }

        for quad_index in &entity.ui_quads {
            let quad = &scene.ui_quads[*quad_index];
            let quad_constants = GUIQuadSendable {
                additive_color: quad.color.to_array4(),
                multiplicative_color: [1.0; 4],
                resolution: [self.quad_renderpass.viewport.width as i32, self.quad_renderpass.viewport.height as i32],
                clip_min: layout.clip_min.to_array2(),
                clip_max: layout.clip_max.to_array2(),
                position: layout.position.to_array2(),
                scale: layout.size.to_array2(),
                corner_radius: quad.corner_radius,
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
        }
        for image_index in &entity.ui_images {
            let image = &scene.ui_images[*image_index];
            let mut scale = layout.size.to_array2();
            if let Some(ratio) = image.aspect_ratio {
                let min = scale[0].min(scale[1]);
                let min_axis = if scale[0] < scale[1] { 0 } else { 1 };
                scale[min_axis] = min;
                scale[1 - min_axis] = ratio * min;
            }

            let quad_constants = GUIQuadSendable {
                additive_color: image.additive_tint.to_array4(),
                multiplicative_color: image.multiplicative_tint.to_array4(),
                resolution: [self.quad_renderpass.viewport.width as i32, self.quad_renderpass.viewport.height as i32],
                clip_min: layout.clip_min.to_array2(),
                clip_max: layout.clip_max.to_array2(),
                position: layout.position.to_array2(),
                scale,
                corner_radius: image.corner_radius,
                image: image.index as i32,
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
        }
        for texture_index in &entity.ui_textures {
            let texture = &scene.ui_textures[*texture_index];
            let mut scale = layout.size.to_array2();
            if let Some(ratio) = texture.aspect_ratio {
                let min = scale[0].min(scale[1]);
                let min_axis = if scale[0] < scale[1] { 0 } else { 1 };
                scale[min_axis] = min;
                scale[1 - min_axis] = ratio * min;
            }

            let quad_constants = GUIQuadSendable {
                additive_color: texture.additive_tint.to_array4(),
                multiplicative_color: texture.multiplicative_tint.to_array4(),
                resolution: [self.quad_renderpass.viewport.width as i32, self.quad_renderpass.viewport.height as i32],
                clip_min: layout.clip_min.to_array2(),
                clip_max: layout.clip_max.to_array2(),
                position: layout.position.to_array2(),
                scale,
                corner_radius: texture.corner_radius,
                image: texture.index as i32,
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
        }
        for text_index in &entity.ui_texts {
            let text = &scene.ui_texts[*text_index];
            self.text_renderer.draw_gui_text(
                current_frame,
                text.text_information.as_ref().unwrap(),
                layout.position,
                layout.size,
                layout.clip_min,
                layout.clip_max,
            );
        }

        if entity.ui_interactable_information.is_some() {
            interactable_parameter_sets.push((node_index, layout.clip_min, layout.clip_max));
        }

        for child in &entity.children_indices.clone() {
            self.draw_node(scene, *child, current_frame, command_buffer, interactable_parameter_sets);
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