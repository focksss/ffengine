use std::slice;
use std::sync::Arc;
use std::time::Instant;
use ash::vk;
use ash::vk::{DescriptorType, Format, Sampler, ShaderStageFlags};
use crate::engine::world::camera::Camera;
use crate::engine::world::scene::Scene;
use crate::gui::gui::{GUINode, GUIQuad, GUIText, GUI};
use crate::math::Vector;
use crate::render::*;
use crate::render::scene_renderer::SceneRenderer;

pub const MAX_FRAMES_IN_FLIGHT: usize = 3;

pub struct Renderer {
    pub device: ash::Device,
    pub draw_command_buffers: Vec<vk::CommandBuffer>,

    pub present_renderpass: Renderpass,
    pub compositing_renderpass: Renderpass,

    pub scene_renderer: SceneRenderer,
    pub gui: GUI,

    pub last_fps_render: Instant,

    pub present_sampler: Sampler,
}
impl Renderer {
    pub unsafe fn new(base: &VkBase, world: &Scene) -> Renderer { unsafe {
        Renderer::compile_shaders();
        
        let texture_sampler_create_info = DescriptorCreateInfo::new(base)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT);
        
        let present_pass_create_info = PassCreateInfo::new(base)
            .set_is_present_pass(true);
        let present_descriptor_set_create_info = DescriptorSetCreateInfo::new(base)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info));
        let present_renderpass_create_info = { RenderpassCreateInfo::new(base)
            .pass_create_info(present_pass_create_info)
            .descriptor_set_create_info(present_descriptor_set_create_info)
            .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
            .fragment_shader_uri(String::from("quad\\quad.frag.spv")) };

        let compositing_pass_create_info = PassCreateInfo::new(base)
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R16G16B16A16_SFLOAT).add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC));
        let compositing_descriptor_set_create_info = DescriptorSetCreateInfo::new(base)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&texture_sampler_create_info));
        let compositing_renderpass_create_info = { RenderpassCreateInfo::new(base)
            .pass_create_info(compositing_pass_create_info)
            .descriptor_set_create_info(compositing_descriptor_set_create_info)
            .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
            .fragment_shader_uri(String::from("composite.frag.spv")) };

        let mut renderer = Renderer {
            device: base.device.clone(),
            draw_command_buffers: base.draw_command_buffers.clone(),

            present_renderpass: Renderpass::new(present_renderpass_create_info),
            compositing_renderpass: Renderpass::new(compositing_renderpass_create_info),

            scene_renderer: SceneRenderer::new(base, world),
            gui: GUI::new(base),

            last_fps_render: Instant::now(),

            present_sampler: base.device.create_sampler(&vk::SamplerCreateInfo {
                mag_filter: vk::Filter::LINEAR,
                min_filter: vk::Filter::LINEAR,
                address_mode_u: vk::SamplerAddressMode::CLAMP_TO_BORDER,
                address_mode_v: vk::SamplerAddressMode::CLAMP_TO_BORDER,
                address_mode_w: vk::SamplerAddressMode::CLAMP_TO_BORDER,
                border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
                ..Default::default()
            }, None).unwrap()
        };

        renderer.set_present_textures(renderer.compositing_renderpass.pass.borrow().textures.iter().map(|frame_textures| {
            &frame_textures[0]
        }).collect::<Vec<&Texture>>());

        renderer.set_compositing_textures(vec![
            renderer.scene_renderer.lighting_renderpass.pass.borrow().textures.iter().map(|frame_textures| {
                &frame_textures[0]
            }).collect::<Vec<&Texture>>(),
            renderer.gui.pass.borrow().textures.iter().map(|frame_textures| {
                &frame_textures[0]
            }).collect::<Vec<&Texture>>()
        ]);
        renderer.scene_renderer.update_world_textures_all_frames(&base, &world);

        renderer.gui.load_from_file(base, "resources\\gui\\default.gui");

        renderer
    } }

    pub unsafe fn compile_shaders() {
        #[cfg(debug_assertions)] {
            compile_shaders("resources\\shaders\\glsl").expect("Failed to compile shaders");
        }
    }

    pub unsafe fn set_present_textures(&self, texture_set: Vec<&Texture>) { unsafe {
        for current_frame in 0..MAX_FRAMES_IN_FLIGHT {
            let present_info = [vk::DescriptorImageInfo {
                sampler: self.present_sampler,
                image_view: texture_set[current_frame].image_view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            }];
            let present_descriptor_writes: Vec<vk::WriteDescriptorSet> = vec![
                vk::WriteDescriptorSet::default()
                    .dst_set(self.present_renderpass.descriptor_set.descriptor_sets[current_frame])
                    .dst_binding(0)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&present_info)];
            self.device.update_descriptor_sets(&present_descriptor_writes, &[]);
        }
    } }
    pub unsafe fn set_compositing_textures(&self, texture_sets: Vec<Vec<&Texture>>) { unsafe {
        for current_frame in 0..MAX_FRAMES_IN_FLIGHT {
            let image_infos = texture_sets.iter().map(|texture_set| {
                vk::DescriptorImageInfo {
                    sampler: self.present_sampler,
                    image_view: texture_set[current_frame].image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }
            }).collect::<Vec<vk::DescriptorImageInfo>>();

            let descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(self.compositing_renderpass.descriptor_set.descriptor_sets[current_frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            self.device.update_descriptor_sets(&descriptor_writes, &[]);
        }
    } }

    pub unsafe fn render_frame(&mut self, current_frame: usize, present_index: usize, delta_time: f32, world: &Scene, player_camera: &Camera) { unsafe {
        let frame_command_buffer = self.draw_command_buffers[current_frame];

        if self.last_fps_render.elapsed().as_secs_f32() > 0.1 {
            self.gui.update_text_of_node(0, format!("FPS: {}", 1.0 / delta_time).as_str(), frame_command_buffer);
            self.last_fps_render = Instant::now();
        }

        self.gui.draw(current_frame, frame_command_buffer);

        self.scene_renderer.render_world(current_frame, &world, &player_camera);

        self.compositing_renderpass.do_renderpass(current_frame, frame_command_buffer, None::<fn()>, None::<fn()>, None);
        self.present_renderpass.begin_renderpass(current_frame, frame_command_buffer, Some(present_index));
        self.device.cmd_draw(frame_command_buffer, 6, 1, 0, 0);
        self.device.cmd_end_render_pass(frame_command_buffer);
    } }

    pub unsafe fn destroy(&mut self) { unsafe {
        self.scene_renderer.destroy();
        self.gui.destroy();
        self.compositing_renderpass.destroy();
        self.present_renderpass.destroy();
        self.device.destroy_sampler(self.present_sampler, None);
    } }
}