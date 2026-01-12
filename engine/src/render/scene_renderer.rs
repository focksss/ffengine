use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::io::Cursor;
use std::mem;
use crate::offset_of;
use ash::vk;
use ash::vk::{DescriptorType, Extent2D, Format, Handle, ImageAspectFlags, ImageLayout, PipelineInputAssemblyStateCreateInfo, ShaderStageFlags};
use rand::{rng, Rng};
use std::path::PathBuf;
use std::slice;
use std::sync::Arc;
use ash::util::read_spv;
use crate::math::matrix::Matrix;
use crate::math::Vector;
use crate::render::render::MAX_FRAMES_IN_FLIGHT;
use crate::render::render_helper::{transition_output, Descriptor, DescriptorCreateInfo, DescriptorSet, DescriptorSetCreateInfo, DeviceTexture, PassCreateInfo, PipelineCreateInfo, Renderpass, RenderpassCreateInfo, Shader, Texture, TextureCreateInfo, Transition, SHADER_PATH};
use crate::render::vulkan_base::{copy_data_to_memory, load_file, Context, VkBase};
use crate::scene::scene::{DrawMode, Instance, Scene};
use crate::scene::world::camera::Camera;
use crate::scene::world::world::{World, SunSendable, Vertex};

const SSAO_KERNAL_SIZE: usize = 16;
const SSAO_RESOLUTION_MULTIPLIER: f32 = 0.5;
pub const SHADOW_RES: u32 = 2048;


//TODO() FIX SSAO UPSAMPLING
pub struct SceneRenderer {
    context: Arc<Context>,

    pub viewport: Arc<RefCell<vk::Viewport>>,

    pub hovered_ids: (usize, usize), // TODO entirely refactor the vulkan abstractions to be ID base (like Scene), be able to sample textures from lua. IN THE MEANTIME: (entity_id, entity-component-id)
    pub queued_id_buffer_sample: bool,

    pub null_texture: DeviceTexture,
    pub null_tex_sampler: vk::Sampler,

    pub geometry_renderpass: Renderpass,
    pub shadow_renderpass: Renderpass,
    pub opaque_forward_renderpass: Renderpass,

    pub outline_renderpass: Renderpass,
    pub cloud_renderpass: Renderpass,

    pub ssao_pre_downsample_renderpass: Renderpass,
    pub ssao_renderpass: Renderpass,
    pub ssao_blur_horizontal_renderpass: Renderpass,
    pub ssao_blur_vertical_renderpass: Renderpass,
    pub ssao_upsample_renderpass: Renderpass,

    pub lighting_renderpass: Renderpass,

    pub sampler: vk::Sampler,
    pub nearest_sampler: vk::Sampler,
    pub repeat_sampler: vk::Sampler,
    pub ssao_kernal: [[f32; 4]; SSAO_KERNAL_SIZE],
    pub ssao_noise_texture: Texture,
    pub editor_primitives_vertices_buffer: (vk::Buffer, vk::DeviceMemory),
    pub editor_primitives_indices_buffer: (vk::Buffer, vk::DeviceMemory),
    pub editor_primitives_index_info: Vec<(u32, u32)>,

    pub cloud_tex_high: Texture,
    pub cloud_tex_low: Texture,
}
impl SceneRenderer {
    pub unsafe fn new(context: &Arc<Context>, world: &World, viewport: vk::Viewport) -> SceneRenderer { unsafe {
        let null_tex_info = context.create_2d_texture_image(&PathBuf::from("").join("engine\\resources\\checker_2x2.png"), true) ;
        context.device.destroy_sampler(null_tex_info.0.1, None);
        let sampler_info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            mag_filter: vk::Filter::NEAREST,
            min_filter: vk::Filter::NEAREST,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            anisotropy_enable: vk::TRUE,
            max_anisotropy: context.pdevice_properties.limits.max_sampler_anisotropy,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            mip_lod_bias: 0.0,
            min_lod: 0.0,
            max_lod: null_tex_info.2 as f32,
            ..Default::default()
        };
        let null_texture = DeviceTexture {
            image: null_tex_info.1.0,
            image_view: null_tex_info.0.0,
            stencil_image_view: None,
            device_memory: null_tex_info.1.1,

            destroyed: false,
        };
        let null_tex_sampler = context.device.create_sampler(&sampler_info, None).expect("failed to create sampler");

        let (
            geometry_renderpass,
            forward_renderpass,
            shadow_renderpass,
            ssao_pre_downsample_renderpass,
            ssao_renderpass,
            ssao_blur_horizontal_renderpass,
            ssao_blur_vertical_renderpass,
            ssao_upsample_renderpass,
            lighting_renderpass,
            outline_renderpass,
            cloud_renderpass,
        ) = SceneRenderer::create_rendering_objects(context, &null_texture, &null_tex_sampler, world, viewport);

        //<editor-fold desc = "ssao sampling setup">
        let mut rng = rng();
        let mut ssao_kernal= [[0.0; 4]; SSAO_KERNAL_SIZE];
        for i in 0..SSAO_KERNAL_SIZE {
            let mut scale = i as f32 / SSAO_KERNAL_SIZE as f32;
            scale = 0.1 + ((scale * scale) * (1.0 - 0.1));
            ssao_kernal[i] = (Vector::from_array(&[rng.random::<f32>() * 2.0 - 1.0, rng.random::<f32>() * 2.0 - 1.0, rng.random::<f32>()])
                .normalize3() * rng.random::<f32>() * scale).to_array4();
        }

        let mut noise_data = Vec::<[f32; 2]>::with_capacity(16);
        for _ in 0..16 {
            noise_data.push([
                rng.random_range(-1.0..1.0),
                rng.random_range(-1.0..1.0),
            ]);
        }
        let ssao_noise_tex_info = TextureCreateInfo::new(context)
            .width(4)
            .height(4)
            .depth(1)
            .format(Format::R16G16_SFLOAT)
            .usage_flags(
                vk::ImageUsageFlags::SAMPLED |
                vk::ImageUsageFlags::TRANSFER_DST
            );
        let ssao_noise_texture = Texture::new(&ssao_noise_tex_info);

        let ((staging_buffer, staging_buffer_memory), _) = context.create_device_and_staging_buffer(
            0,
            &noise_data,
            vk::BufferUsageFlags::TRANSFER_SRC,
            true,
            false,
            true,
        );
        context.transition_image_layout(
            ssao_noise_texture.device_texture.borrow().image,
            vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
                ..Default::default()
            },
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        context.copy_buffer_to_image(
            staging_buffer,
            ssao_noise_texture.device_texture.borrow().image,
            vk::Extent3D { width: 4, height: 4, depth: 1 },
        );
        context.transition_image_layout(
            ssao_noise_texture.device_texture.borrow().image,
            vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
                ..Default::default()
            },
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        context.device.destroy_buffer(staging_buffer, None);
        context.device.free_memory(staging_buffer_memory, None);
        //</editor-fold>

        //<editor-fold desc = "cloud noise setup">
        let cloud_noise_high_tex_info = TextureCreateInfo::new(context)
            .width(128)
            .height(128)
            .depth(128)
            .format(Format::R16G16B16A16_SFLOAT)
            .usage_flags(
                vk::ImageUsageFlags::SAMPLED |
                vk::ImageUsageFlags::STORAGE
            );
        let cloud_tex_high = Texture::new(&cloud_noise_high_tex_info);
        let cloud_noise_low_tex_info = TextureCreateInfo::new(context)
            .width(32)
            .height(32)
            .depth(32)
            .format(Format::R16G16B16A16_SFLOAT)
            .usage_flags(
                vk::ImageUsageFlags::SAMPLED |
                vk::ImageUsageFlags::STORAGE
            );
        let cloud_tex_low = Texture::new(&cloud_noise_low_tex_info);
        generate_cloud_noise(context, &cloud_tex_high, &cloud_tex_low);
        //</editor-fold>

        //<editor-fold desc = "descriptor updates">
        let sampler = context.device.create_sampler(&vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            ..Default::default()
        }, None).unwrap();
        let repeat_sampler = context.device.create_sampler(&vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            ..Default::default()
        }, None).unwrap();
        let nearest_sampler = context.device.create_sampler(&vk::SamplerCreateInfo {
            mag_filter: vk::Filter::NEAREST,
            min_filter: vk::Filter::NEAREST,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            ..Default::default()
        }, None).unwrap();

        for current_frame in 0..MAX_FRAMES_IN_FLIGHT {
            //<editor-fold desc = "lighting">
            let image_infos = [
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.borrow().textures[current_frame][0].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // albedo
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.borrow().textures[current_frame][1].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // metallic roughness
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.borrow().textures[current_frame][2].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // extra properties
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.borrow().textures[current_frame][5].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                }, // depth
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.borrow().textures[current_frame][3].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // view normal
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: shadow_renderpass.pass.borrow().textures[current_frame][0].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                }, // shadow map
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: ssao_upsample_renderpass.pass.borrow().textures[current_frame][0].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // ssao tex (final)
            ];
            let lighting_descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(lighting_renderpass.descriptor_set.borrow().descriptor_sets[current_frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            context.device.update_descriptor_sets(&lighting_descriptor_writes, &[]);
            //</editor-fold>
            //<editor-fold desc = "ssao pre downsample">
            let image_infos = [
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.borrow().textures[current_frame][5].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                }, // geometry depth
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.borrow().textures[current_frame][3].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // view normal
            ];
            let ssao_pre_downsample_descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(ssao_pre_downsample_renderpass.descriptor_set.borrow().descriptor_sets[current_frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            context.device.update_descriptor_sets(&ssao_pre_downsample_descriptor_writes, &[]);
            //</editor-fold>
            //<editor-fold desc = "ssao gen">
            let image_infos = [
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: ssao_pre_downsample_renderpass.pass.borrow().textures[current_frame][0].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // downsampled normal + depth
                vk::DescriptorImageInfo {
                    sampler: nearest_sampler,
                    image_view: ssao_noise_texture.device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // noise tex
            ];
            let ssao_descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(ssao_renderpass.descriptor_set.borrow().descriptor_sets[current_frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            context.device.update_descriptor_sets(&ssao_descriptor_writes, &[]);
            //</editor-fold>
            //<editor-fold desc = "ssao blur">
            let image_infos = [
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: ssao_renderpass.pass.borrow().textures[current_frame][0].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // ssao raw
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: ssao_pre_downsample_renderpass.pass.borrow().textures[current_frame][0].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // downsampled normal + depth
            ];
            let descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(ssao_blur_horizontal_renderpass.descriptor_set.borrow().descriptor_sets[current_frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            context.device.update_descriptor_sets(&descriptor_writes, &[]);

            let image_infos = [
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: ssao_blur_horizontal_renderpass.pass.borrow().textures[current_frame][0].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // ssao blurred horizontal
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: ssao_pre_downsample_renderpass.pass.borrow().textures[current_frame][0].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // downsampled normal + depth
            ];
            let descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(ssao_blur_vertical_renderpass.descriptor_set.borrow().descriptor_sets[current_frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            context.device.update_descriptor_sets(&descriptor_writes, &[]);

            let image_infos = [
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: ssao_blur_vertical_renderpass.pass.borrow().textures[current_frame][0].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // ssao blurred
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: ssao_pre_downsample_renderpass.pass.borrow().textures[current_frame][0].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // downsampled normal + depth
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.borrow().textures[current_frame][3].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // g_normal
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.borrow().textures[current_frame][5].device_texture.borrow().image_view,
                    image_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                }, // geometry depth
            ];
            let descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(ssao_upsample_renderpass.descriptor_set.borrow().descriptor_sets[current_frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            context.device.update_descriptor_sets(&descriptor_writes, &[]);
            //</editor-fold>
            //<editor-fold desc = "outline">
            let image_infos = [
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.borrow().textures[current_frame][5].device_texture.borrow().stencil_image_view.unwrap(),
                    image_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                }, // geometry stencil
            ];
            let descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(outline_renderpass.descriptor_set.borrow().descriptor_sets[current_frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            context.device.update_descriptor_sets(&descriptor_writes, &[]);
            //</editor-fold>
            //<editor-fold desc = "cloud">
            let image_infos = [
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.borrow().textures[current_frame][5].device_texture.borrow().image_view,
                    image_layout: ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                }, // geometry depth
                vk::DescriptorImageInfo {
                    sampler: repeat_sampler,
                    image_view: cloud_tex_high.device_texture.borrow().image_view,
                    image_layout: ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // cloud high
                vk::DescriptorImageInfo {
                    sampler: repeat_sampler,
                    image_view: cloud_tex_low.device_texture.borrow().image_view,
                    image_layout: ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // cloud low
            ];
            let descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(cloud_renderpass.descriptor_set.borrow().descriptor_sets[current_frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            context.device.update_descriptor_sets(&descriptor_writes, &[]);
            //</editor-fold>
        }
        //</editor-fold>

        //<editor-fold desc = "editor primitive buffers">
        let mut indices: Vec<[u32; 2]> = Vec::new();
        let mut vertices: Vec<[f32; 3]> = Vec::new();
        let mut editor_primitives_index_info = Vec::new();

        //cube
        let mut info = (0, indices.len() as u32 * 2);

        for x in [-1.0,1.0] {
            for y in [-1.0,1.0] {
                for z in [-1.0,1.0] {
                    vertices.push([x, y, z]);
                }
            }
        }
        let edges = [
            [0, 1], [1, 3], [3, 2], [2, 0], // left face
            [4, 5], [5, 7], [7, 6], [6, 4], // right face
            [0, 4], [1, 5], [2, 6], [3, 7], // connecting edges
        ];
        for edge in edges {
            indices.push(edge);
        }
        info.0 = indices.len() as u32 * 2 - info.1;
        editor_primitives_index_info.push(info);

        //sphere
        let mut info = (0, indices.len() as u32 * 2);
        let sphere_base_vertex = vertices.len() as u32;

        let res = 50;
        for i in 0..res {
            let theta = 2.0 * PI * i as f32 / res as f32;
            vertices.push([theta.cos(), 0.0, theta.sin()]);
            vertices.push([theta.cos(), theta.sin(), 0.0]);
            vertices.push([0.0, theta.cos(), theta.sin()]);
            if i > 0 {
                indices.push([vertices.len() as u32 - 6, vertices.len() as u32 - 3]);
                indices.push([vertices.len() as u32 - 5, vertices.len() as u32 - 2]);
                indices.push([vertices.len() as u32 - 4, vertices.len() as u32 - 1]);
            }
        }
        let last = sphere_base_vertex + (res as u32 - 1) * 3;
        indices.push([sphere_base_vertex + 0, last + 0]);
        indices.push([sphere_base_vertex + 1, last + 1]);
        indices.push([sphere_base_vertex + 2, last + 2]);

        info.0 = indices.len() as u32 * 2 - info.1;
        editor_primitives_index_info.push(info);

        let editor_primitives_vertices_buffer = context.create_device_and_staging_buffer(
            0,
            &vertices,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            true, false, true
        ).0;
        let editor_primitives_indices_buffer = context.create_device_and_staging_buffer(
            0,
            &indices,
            vk::BufferUsageFlags::INDEX_BUFFER,
            true, false, true
        ).0;
        //</editor-fold>

        SceneRenderer {
            context: context.clone(),

            hovered_ids: (0, 0),
            queued_id_buffer_sample: false,

            viewport: Arc::new(RefCell::new(viewport)),

            null_texture,
            null_tex_sampler,

            geometry_renderpass,
            opaque_forward_renderpass: forward_renderpass,
            shadow_renderpass,
            ssao_pre_downsample_renderpass,
            ssao_renderpass,
            ssao_blur_horizontal_renderpass,
            ssao_blur_vertical_renderpass,
            ssao_upsample_renderpass,
            lighting_renderpass,
            outline_renderpass,
            cloud_renderpass,

            cloud_tex_high,
            cloud_tex_low,

            sampler,
            nearest_sampler,
            repeat_sampler,

            ssao_kernal,
            ssao_noise_texture,

            editor_primitives_indices_buffer,
            editor_primitives_vertices_buffer,
            editor_primitives_index_info,
        }
    } }
    pub unsafe fn create_rendering_objects(
        context: &Arc<Context>,
        null_tex: &DeviceTexture,
        null_tex_sampler: &vk::Sampler,
        world: &World,
        viewport: vk::Viewport
    ) -> (
        Renderpass, // geometry pass
        Renderpass, // forward pass
        Renderpass, // shadow pass
        Renderpass, // ssao_pre_downsample_renderpass
        Renderpass, // ssao_renderpass
        Renderpass, // ssao blur horizontal renderpass
        Renderpass, // ssao blur vertical renderpass
        Renderpass, // ssao upsample renderpass
        Renderpass, // lighting pass
        Renderpass, // outline pass
        Renderpass, // cloud pass
    ) { unsafe {
        let resolution = Extent2D { width: viewport.width as u32, height: viewport.height as u32 };
        let image_infos: Vec<vk::DescriptorImageInfo> = vec![vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view: null_tex.image_view.clone(),
            sampler: null_tex_sampler.clone(),
            ..Default::default()
        }; 1024];

        let texture_sampler_create_info = DescriptorCreateInfo::new(context)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT);
        //<editor-fold desc = "passes">
        let ssao_res_color_tex_create_info = TextureCreateInfo::new(context).format(Format::R8_UNORM)
            .width(resolution.width as u32).height(resolution.height as u32)
            .resolution_denominator((1.0 / SSAO_RESOLUTION_MULTIPLIER) as u32);
        let geometry_pass_create_info = PassCreateInfo::new(context)
            // .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R16_SINT)) // material
            .add_color_attachment_info(TextureCreateInfo::new(context).format(Format::R8G8B8A8_UNORM)
                .width(resolution.width).height(resolution.height)) // albedo
            .add_color_attachment_info(TextureCreateInfo::new(context).format(Format::R8G8B8A8_UNORM)
                .width(resolution.width).height(resolution.height)) // metallic roughness
            .add_color_attachment_info(TextureCreateInfo::new(context).format(Format::R8G8B8A8_UNORM)
                .width(resolution.width).height(resolution.height)) // extra properties
            .add_color_attachment_info(TextureCreateInfo::new(context).format(Format::R16G16B16A16_SFLOAT)
                .width(resolution.width).height(resolution.height)) // view normal
            .add_color_attachment_info(TextureCreateInfo::new(context).format(Format::R16G16_UINT)
                .usage_flags(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC)
                .width(resolution.width).height(resolution.height)) // id buffer
            .depth_attachment_info(TextureCreateInfo::new(context).format(Format::D32_SFLOAT_S8_UINT)
                .width(resolution.width).height(resolution.height)
                .is_depth(true).clear_value([0.0, 0.0, 0.0, 0.0])
                .has_stencil(true)); // depth

        let shadow_pass_create_info = PassCreateInfo::new(context)
            .depth_attachment_info(TextureCreateInfo::new(context).format(Format::D16_UNORM).is_depth(true).clear_value([0.0, 0.0, 0.0, 0.0]).width(SHADOW_RES).height(SHADOW_RES).array_layers(5)); // depth;

        let ssao_depth_downsample_pass_create_info = PassCreateInfo::new(context)
            .add_color_attachment_info(TextureCreateInfo::new(context).format(Format::R16G16B16A16_SFLOAT)
                .width(resolution.width).height(resolution.height)
                .resolution_denominator((1.0 / SSAO_RESOLUTION_MULTIPLIER) as u32));
        let ssao_pass_create_info = PassCreateInfo::new(context)
            .add_color_attachment_info(ssao_res_color_tex_create_info.clone());
        let ssao_blur_horizontal_pass_create_info = PassCreateInfo::new(context)
            .add_color_attachment_info(ssao_res_color_tex_create_info.clone());
        let ssao_blur_vertical_pass_create_info = PassCreateInfo::new(context)
            .add_color_attachment_info(ssao_res_color_tex_create_info);
        let ssao_upsample_pass_create_info = PassCreateInfo::new(context)
            .add_color_attachment_info(TextureCreateInfo::new(context).format(Format::R8_UNORM)
                .width(resolution.width).height(resolution.height));

        let lighting_pass_create_info = PassCreateInfo::new(context)
            .add_color_attachment_info(TextureCreateInfo::new(context).format(Format::R16G16B16A16_SFLOAT).add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC)
                .width(resolution.width).height(resolution.height));
        //</editor-fold>
        //<editor-fold desc = "geometry + shadow descriptor sets"
        let sun_ubo_create_info = DescriptorCreateInfo::new(context)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .shader_stages(ShaderStageFlags::GEOMETRY)
            .size(size_of::<SunSendable>() as u64);
        let material_ssbo_create_info = DescriptorCreateInfo::new(context)
            .descriptor_type(DescriptorType::STORAGE_BUFFER)
            .shader_stages(ShaderStageFlags::FRAGMENT)
            .buffers(world.material_buffers.iter().map(|b| {b.0.clone()}).collect());
        let joints_ssbo_create_info = DescriptorCreateInfo::new(context)
            .descriptor_type(DescriptorType::STORAGE_BUFFER)
            .shader_stages(ShaderStageFlags::VERTEX)
            .buffers(world.joints_buffers.iter().map(|b| {b.0.clone()}).collect());
        let world_texture_samplers_create_info = DescriptorCreateInfo::new(context)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT)
            .dynamic(true)
            .image_infos(image_infos.clone());

        let geometry_descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
            .add_descriptor(Descriptor::new(&material_ssbo_create_info))
            .add_descriptor(Descriptor::new(&joints_ssbo_create_info))
            .add_descriptor(Descriptor::new(&world_texture_samplers_create_info));

        let shadow_descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
            .add_descriptor(Descriptor::new(&material_ssbo_create_info))
            .add_descriptor(Descriptor::new(&joints_ssbo_create_info))
            .add_descriptor(Descriptor::new(&sun_ubo_create_info))
            .add_descriptor(Descriptor::new(&world_texture_samplers_create_info));
        //</editor-fold>]
        // <editor-fold desc = "SSAO descriptor sets">
        let ssao_depth_downsample_descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&texture_sampler_create_info));
        let ssbo_ubo_create_info = DescriptorCreateInfo::new(context)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .size(size_of::<SSAOPassUniformData>() as u64)
            .shader_stages(ShaderStageFlags::FRAGMENT);
        let ssao_descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&ssbo_ubo_create_info));
        let ssao_blur_horizontal_descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info)) // input (ssao)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info)); // g_info (ssao res)
        let ssao_blur_vertical_descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info)) // input (ssao)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info)); // g_info (ssao res)
        let ssao_upsample_descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info)) // input (low res)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info)) // g_info (low res)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info)) // normal (full_res)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info)); // depth (full_res)
        //</editor-fold>
        //<editor-fold desc = "lighting descriptor set">
        let lights_ssbo_create_info = DescriptorCreateInfo::new(context)
            .descriptor_type(DescriptorType::STORAGE_BUFFER)
            .shader_stages(ShaderStageFlags::FRAGMENT)
            .buffers(world.lights_buffers.iter().map(|b| {b.0.clone()}).collect());
        let sun_ubo_create_info = DescriptorCreateInfo::new(context)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .size(size_of::<SunSendable>() as u64)
            .shader_stages(ShaderStageFlags::FRAGMENT);
        let lighting_ubo_create_info = DescriptorCreateInfo::new(context)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .size(size_of::<LightingUniformData>() as u64)
            .shader_stages(ShaderStageFlags::FRAGMENT);
        let lighting_descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&lights_ssbo_create_info))
            .add_descriptor(Descriptor::new(&lighting_ubo_create_info))
            .add_descriptor(Descriptor::new(&sun_ubo_create_info))
            .add_descriptor(Descriptor::new(&DescriptorCreateInfo::new(context)
                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                .shader_stages(ShaderStageFlags::FRAGMENT)
                .binding_flags(vk::DescriptorBindingFlags::UPDATE_AFTER_BIND))
            );
        //</editor-fold>

        let camera_push_constant_range_vertex = vk::PushConstantRange {
            stage_flags: ShaderStageFlags::VERTEX,
            offset: 0,
            size: size_of::<CameraMatrixUniformData>() as _,
        };
        let camera_push_constant_range_fragment = vk::PushConstantRange {
            stage_flags: ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: size_of::<CameraMatrixUniformData>() as _,
        };
        let camera_push_constant_range = vk::PushConstantRange {
            stage_flags: ShaderStageFlags::ALL_GRAPHICS,
            offset: 0,
            size: size_of::<CameraMatrixUniformData>() as _,
        };
        //<editor-fold desc = "full graphics pipeline initiation">
        let vertex_input_binding_descriptions = [
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: size_of::<Vertex>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            },
            vk::VertexInputBindingDescription {
                binding: 1,
                stride: size_of::<Instance>() as u32,
                input_rate: vk::VertexInputRate::INSTANCE,
            }
        ];
        let geometry_vertex_input_attribute_descriptions = [
            // vertex
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, position) as u32,
            }, // position
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, normal) as u32,
            }, // normal
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, uv) as u32,
            }, // uv
            vk::VertexInputAttributeDescription {
                location: 3,
                binding: 0,
                format: Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, tangent) as u32,
            }, // tangent
            vk::VertexInputAttributeDescription {
                location: 4,
                binding: 0,
                format: Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, bitangent) as u32,
            }, // bitangent
            vk::VertexInputAttributeDescription {
                location: 5,
                binding: 0,
                format: Format::R32G32B32A32_UINT,
                offset: offset_of!(Vertex, joint_indices) as u32,
            }, // joint indices
            vk::VertexInputAttributeDescription {
                location: 6,
                binding: 0,
                format: Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, joint_weights) as u32,
            }, // joint weights

            // instance
            vk::VertexInputAttributeDescription {
                location: 7,
                binding: 1,
                format: Format::R32G32B32A32_SFLOAT,
                offset: 0,
            }, // model matrix
            vk::VertexInputAttributeDescription {
                location: 8,
                binding: 1,
                format: Format::R32G32B32A32_SFLOAT,
                offset: 16,
            }, // |
            vk::VertexInputAttributeDescription {
                location: 9,
                binding: 1,
                format: Format::R32G32B32A32_SFLOAT,
                offset: 32,
            }, // |
            vk::VertexInputAttributeDescription {
                location: 10,
                binding: 1,
                format: Format::R32G32B32A32_SFLOAT,
                offset: 48,
            }, // |
            vk::VertexInputAttributeDescription {
                location: 11,
                binding: 1,
                format: Format::R32G32B32A32_SINT,
                offset: offset_of!(Instance, indices) as u32,
            }, // indices (material + skin)
        ];
        let shadow_vertex_input_attribute_descriptions = [
            // vertex
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, position) as u32,
            }, // position
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, uv) as u32,
            }, // uv
            vk::VertexInputAttributeDescription {
                location: 5,
                binding: 0,
                format: Format::R32G32B32A32_UINT,
                offset: offset_of!(Vertex, joint_indices) as u32,
            }, // joint indices
            vk::VertexInputAttributeDescription {
                location: 6,
                binding: 0,
                format: Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, joint_weights) as u32,
            }, // joint weights

            // instance
            vk::VertexInputAttributeDescription {
                location: 7,
                binding: 1,
                format: Format::R32G32B32A32_SFLOAT,
                offset: 0,
            }, // model matrix
            vk::VertexInputAttributeDescription {
                location: 8,
                binding: 1,
                format: Format::R32G32B32A32_SFLOAT,
                offset: 16,
            }, // |
            vk::VertexInputAttributeDescription {
                location: 9,
                binding: 1,
                format: Format::R32G32B32A32_SFLOAT,
                offset: 32,
            }, // |
            vk::VertexInputAttributeDescription {
                location: 10,
                binding: 1,
                format: Format::R32G32B32A32_SFLOAT,
                offset: 48,
            }, // |
            vk::VertexInputAttributeDescription {
                location: 11,
                binding: 1,
                format: Format::R32G32B32_SINT,
                offset: offset_of!(Instance, indices) as u32,
            }, // indices (material + skin)
        ];
        let geometry_vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&geometry_vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);
        let shadow_vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&shadow_vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);
        let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            cull_mode: vk::CullModeFlags::BACK,
            line_width: 1.0,
            polygon_mode: vk::PolygonMode::FILL,
            ..Default::default()
        };
        let shadow_rasterization_info = vk::PipelineRasterizationStateCreateInfo {
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            cull_mode: vk::CullModeFlags::NONE,
            line_width: 1.0,
            polygon_mode: vk::PolygonMode::FILL,
            ..Default::default()
        };
        let noop_stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            ..Default::default()
        };
        let infinite_reverse_depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: 1,
            depth_write_enable: 1,
            depth_compare_op: vk::CompareOp::GREATER,
            front: noop_stencil_state,
            back: noop_stencil_state,
            max_depth_bounds: 1.0,
            ..Default::default()
        };
        let shadow_depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: 1,
            depth_write_enable: 1,
            depth_compare_op: vk::CompareOp::GREATER,
            front: noop_stencil_state,
            back: noop_stencil_state,
            max_depth_bounds: 1.0,
            ..Default::default()
        };
        let null_blend_attachment = vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::FALSE,  // Disable blending
            src_color_blend_factor: vk::BlendFactor::ONE,
            dst_color_blend_factor: vk::BlendFactor::ZERO,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        };
        let null_blend_states = vec![null_blend_attachment; 5];
        let null_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(&null_blend_states);
        //</editor-fold>

        let geometry_renderpass_create_info = { RenderpassCreateInfo::new(context)
            .pass_create_info(geometry_pass_create_info)
            .resolution(resolution)
            .descriptor_set_create_info(geometry_descriptor_set_create_info)
            .add_push_constant_range(camera_push_constant_range_vertex)
            .add_pipeline_create_info(PipelineCreateInfo::new()
                .pipeline_vertex_input_state(geometry_vertex_input_state_info)
                .pipeline_rasterization_state(rasterization_info)
                .pipeline_depth_stencil_state(infinite_reverse_depth_state_info)
                .pipeline_color_blend_state_create_info(null_blend_state)
                .vertex_shader_uri(String::from("geometry\\geometry.vert.spv"))
                .fragment_shader_uri(String::from("geometry\\geometry.frag.spv"))) };
        let geometry_renderpass = Renderpass::new(geometry_renderpass_create_info);

        let shadow_renderpass_create_info = { RenderpassCreateInfo::new(context)
            .pass_create_info(shadow_pass_create_info)
            .descriptor_set_create_info(shadow_descriptor_set_create_info)
            .add_pipeline_create_info(PipelineCreateInfo::new()
                .pipeline_vertex_input_state(shadow_vertex_input_state_info)
                .pipeline_rasterization_state(shadow_rasterization_info)
                .pipeline_depth_stencil_state(shadow_depth_state_info)
                .vertex_shader_uri(String::from("shadow\\shadow.vert.spv"))
                .fragment_shader_uri(String::from("shadow\\shadow.frag.spv"))
                .geometry_shader_uri(String::from("shadow\\cascade.geom.spv")))
            .viewport(vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: SHADOW_RES as f32,
                height: SHADOW_RES as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            })
            .scissor(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width: SHADOW_RES, height: SHADOW_RES } }) };
        let shadow_renderpass = Renderpass::new(shadow_renderpass_create_info);

        let ssao_pre_downsample_renderpass_create_info = {
            RenderpassCreateInfo::new(context)
                .pass_create_info(ssao_depth_downsample_pass_create_info)
                .descriptor_set_create_info(ssao_depth_downsample_descriptor_set_create_info)
                .add_pipeline_create_info(PipelineCreateInfo::new()
                    .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
                    .fragment_shader_uri(String::from("ssao\\pre_downsample.frag.spv")))
                .resolution(Extent2D {
                    width: (resolution.width as f32 * SSAO_RESOLUTION_MULTIPLIER) as u32,
                    height: (resolution.height as f32 * SSAO_RESOLUTION_MULTIPLIER) as u32,
                }) };
        let ssao_pre_downsample_renderpass = Renderpass::new(ssao_pre_downsample_renderpass_create_info);
        let ssao_renderpass_create_info = { RenderpassCreateInfo::new(context)
            .pass_create_info(ssao_pass_create_info)
            .descriptor_set_create_info(ssao_descriptor_set_create_info)
            .add_pipeline_create_info(PipelineCreateInfo::new()
                .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
                .fragment_shader_uri(String::from("ssao\\ssao.frag.spv")))
            .resolution(vk::Extent2D {
                width: (resolution.width as f32 * SSAO_RESOLUTION_MULTIPLIER) as u32,
                height: (resolution.height as f32 * SSAO_RESOLUTION_MULTIPLIER) as u32,
            }) };
        let ssao_renderpass = Renderpass::new(ssao_renderpass_create_info);
        let ssao_blur_horizontal_renderpass_create_info = { RenderpassCreateInfo::new(context)
            .pass_create_info(ssao_blur_horizontal_pass_create_info)
            .descriptor_set_create_info(ssao_blur_horizontal_descriptor_set_create_info)
            .add_pipeline_create_info(PipelineCreateInfo::new()
                .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
                .fragment_shader_uri(String::from("bilateral_blur\\bilateral_blur.frag.spv")))
            .add_push_constant_range(vk::PushConstantRange {
                stage_flags: ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: size_of::<BlurPassData>() as _,
            })
            .resolution(vk::Extent2D {
                width: (resolution.width as f32 * SSAO_RESOLUTION_MULTIPLIER) as u32,
                height: (resolution.height as f32 * SSAO_RESOLUTION_MULTIPLIER) as u32,
            }) };
        let ssao_blur_horizontal_renderpass = Renderpass::new(ssao_blur_horizontal_renderpass_create_info);
        let ssao_blur_vertical_renderpass_create_info = { RenderpassCreateInfo::new(context)
            .pass_create_info(ssao_blur_vertical_pass_create_info)
            .descriptor_set_create_info(ssao_blur_vertical_descriptor_set_create_info)
            .add_pipeline_create_info(PipelineCreateInfo::new()
                .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
                .fragment_shader_uri(String::from("bilateral_blur\\bilateral_blur.frag.spv")))
            .add_push_constant_range(vk::PushConstantRange {
                stage_flags: ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: size_of::<BlurPassData>() as _,
            })
            .resolution(vk::Extent2D {
                width: (resolution.width as f32 * SSAO_RESOLUTION_MULTIPLIER) as u32,
                height: (resolution.height as f32 * SSAO_RESOLUTION_MULTIPLIER) as u32,
            }) };
        let ssao_blur_vertical_renderpass = Renderpass::new(ssao_blur_vertical_renderpass_create_info);
        let ssao_upsample_renderpass_create_info = { RenderpassCreateInfo::new(context)
            .pass_create_info(ssao_upsample_pass_create_info)
            .descriptor_set_create_info(ssao_upsample_descriptor_set_create_info)
            .add_pipeline_create_info(PipelineCreateInfo::new()
                .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
                .fragment_shader_uri(String::from("geometry_aware_upsample.frag.spv")))
            .add_push_constant_range(vk::PushConstantRange {
                stage_flags: ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: size_of::<DepthAwareUpsamplePassData>() as _,
            })
            .resolution(resolution)
        };
        let ssao_upsample_renderpass = Renderpass::new(ssao_upsample_renderpass_create_info);

        let lighting_renderpass_create_info = { RenderpassCreateInfo::new(context)
            .pass_create_info(lighting_pass_create_info)
            .resolution(resolution)
            .descriptor_set_create_info(lighting_descriptor_set_create_info)
            .add_pipeline_create_info(PipelineCreateInfo::new()
                .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
                .fragment_shader_uri(String::from("lighting.frag.spv")))
            .add_push_constant_range(camera_push_constant_range_fragment)
        };
        let lighting_renderpass = Renderpass::new(lighting_renderpass_create_info);

        let forward_renderpass = {
            let light_pass = lighting_renderpass.pass.borrow();
            let geometry_pass = geometry_renderpass.pass.borrow();
            let forward_pass_create_info = PassCreateInfo::new(context)
                .grab_attachment(&light_pass, 0, vk::AttachmentLoadOp::LOAD, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .grab_depth_attachment(&geometry_pass, 5, vk::AttachmentLoadOp::LOAD, vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL);
            let forward_descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
                .add_descriptor(Descriptor::new(&material_ssbo_create_info))
                .add_descriptor(Descriptor::new(&joints_ssbo_create_info))
                .add_descriptor(Descriptor::new(&DescriptorCreateInfo::new(context)
                    .descriptor_type(DescriptorType::UNIFORM_BUFFER)
                    .size(16u64)
                    .shader_stages(ShaderStageFlags::VERTEX)))
                .add_descriptor(Descriptor::new(&world_texture_samplers_create_info));
            let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
                blend_enable: vk::TRUE,
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
            let mask_stencil_state = vk::StencilOpState {
                compare_op: vk::CompareOp::ALWAYS,
                pass_op: vk::StencilOp::REPLACE,
                fail_op: vk::StencilOp::KEEP,
                depth_fail_op: vk::StencilOp::KEEP,
                compare_mask: 0xFF,
                write_mask: 0xFF,
                reference: 1,
                ..Default::default()
            };
            let mask_depth_stencil_state_info = vk::PipelineDepthStencilStateCreateInfo {
                depth_test_enable: 1,
                depth_write_enable: 1,
                depth_compare_op: vk::CompareOp::ALWAYS,
                stencil_test_enable: 1,
                front: mask_stencil_state,
                back: mask_stencil_state,
                ..Default::default()
            };
            let skybox_depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
                depth_test_enable: 1,
                depth_write_enable: 0,
                depth_compare_op: vk::CompareOp::EQUAL,
                front: noop_stencil_state,
                back: noop_stencil_state,
                max_depth_bounds: 1.0,
                ..Default::default()
            };

            let hitbox_vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
                    binding: 0,
                    stride: 12,
                    input_rate: vk::VertexInputRate::VERTEX,
                }];
            let hitbox_vertex_input_attribute_descriptions = [vk::VertexInputAttributeDescription {
                    location: 0,
                    binding: 0,
                    format: Format::R32G32B32_SFLOAT,
                    offset: 0,
                }];
            let hitbox_vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_attribute_descriptions(&hitbox_vertex_input_attribute_descriptions)
                .vertex_binding_descriptions(&hitbox_vertex_input_binding_descriptions);
            let hitbox_rasterization_info = vk::PipelineRasterizationStateCreateInfo {
                front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                cull_mode: vk::CullModeFlags::BACK,
                line_width: 1.0,
                polygon_mode: vk::PolygonMode::FILL,
                ..Default::default()
            };

            let forward_renderpass_create_info = { RenderpassCreateInfo::new(context)
                .pass_create_info(forward_pass_create_info)
                .resolution(resolution)
                .descriptor_set_create_info(forward_descriptor_set_create_info)
                .add_push_constant_range(camera_push_constant_range)
                .add_pipeline_create_info(PipelineCreateInfo::new()
                    .pipeline_vertex_input_state(geometry_vertex_input_state_info)
                    .pipeline_rasterization_state(rasterization_info)
                    .pipeline_depth_stencil_state(mask_depth_stencil_state_info)
                    .pipeline_color_blend_state_create_info(color_blend_state)
                    .vertex_shader_uri(String::from("geometry\\position_only.vert.spv"))
                    .fragment_shader_uri(String::from("empty.frag.spv")))
                .add_pipeline_create_info(PipelineCreateInfo::new()
                    .pipeline_depth_stencil_state(skybox_depth_state_info)
                    .vertex_shader_uri(String::from("skybox\\skybox.vert.spv"))
                    .fragment_shader_uri(String::from("skybox\\skybox.frag.spv")))
                .add_pipeline_create_info(PipelineCreateInfo::new()
                    .pipeline_input_assembly_state(PipelineInputAssemblyStateCreateInfo {
                        topology: vk::PrimitiveTopology::LINE_LIST,
                        primitive_restart_enable: vk::FALSE,
                        ..Default::default()
                    })
                    .pipeline_vertex_input_state(hitbox_vertex_input_state_info)
                    .pipeline_rasterization_state(hitbox_rasterization_info)
                    .vertex_shader_uri(String::from("hitbox_display\\hitbox.vert.spv"))
                    .fragment_shader_uri(String::from("hitbox_display\\hitbox.frag.spv")))
            };

            Renderpass::new(forward_renderpass_create_info)
        };

        let outline_renderpass = {
            let light_pass = lighting_renderpass.pass.borrow();

            let pass_create_info = PassCreateInfo::new(context)
                .grab_attachment(&light_pass, 0, vk::AttachmentLoadOp::LOAD, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
            let descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
                .add_descriptor(Descriptor::new(&texture_sampler_create_info));

            let renderpass_create_info = RenderpassCreateInfo::new(context)
                .pass_create_info(pass_create_info)
                .resolution(resolution)
                .descriptor_set_create_info(descriptor_set_create_info)
                .add_push_constant_range(vk::PushConstantRange {
                    stage_flags: ShaderStageFlags::FRAGMENT,
                    offset: 0,
                    size: size_of::<OutlineConstantSendable>() as _,
                })
                .add_pipeline_create_info(PipelineCreateInfo::new()
                    .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
                    .fragment_shader_uri(String::from("outline\\outline.frag.spv")));
            Renderpass::new(renderpass_create_info)
        };

        let cloud_renderpass = {
            let light_pass = lighting_renderpass.pass.borrow();

            let pass_create_info = PassCreateInfo::new(context)
                .grab_attachment(&light_pass, 0, vk::AttachmentLoadOp::LOAD, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
            let descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
                .add_descriptor(Descriptor::new(&texture_sampler_create_info))
                .add_descriptor(Descriptor::new(&texture_sampler_create_info))
                .add_descriptor(Descriptor::new(&texture_sampler_create_info));

            let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
                blend_enable: vk::TRUE,
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

            let renderpass_create_info = RenderpassCreateInfo::new(context)
                .pass_create_info(pass_create_info)
                .resolution(resolution)
                .descriptor_set_create_info(descriptor_set_create_info)
                .add_push_constant_range(vk::PushConstantRange {
                    stage_flags: ShaderStageFlags::FRAGMENT,
                    offset: 0,
                    size: size_of::<CameraMatrixUniformData>() as _,
                })
                .add_pipeline_create_info(PipelineCreateInfo::new()
                    .pipeline_color_blend_state_create_info(color_blend_state)
                    .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
                    .fragment_shader_uri(String::from("cloud\\cloud.frag.spv")));
            Renderpass::new(renderpass_create_info)
        };

        (
            geometry_renderpass,
            forward_renderpass,
            shadow_renderpass,
            ssao_pre_downsample_renderpass,
            ssao_renderpass,
            ssao_blur_horizontal_renderpass,
            ssao_blur_vertical_renderpass,
            ssao_upsample_renderpass,
            lighting_renderpass,
            outline_renderpass,
            cloud_renderpass
        )
    } }
    pub fn update_world_textures_all_frames(&self, world: &World) {
        for frame in 0..MAX_FRAMES_IN_FLIGHT {
            self.update_world_textures(world, frame);
        }
    }
    pub fn update_world_textures(&self, world: &World, frame: usize) { unsafe {
        let mut image_infos: Vec<vk::DescriptorImageInfo> = Vec::with_capacity(1024);
        for texture in &world.textures {
            let texture_source = &world.images[texture.source];
            if texture_source.generated {
                image_infos.push(vk::DescriptorImageInfo {
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    image_view: texture_source.image_view,
                    sampler: texture.sampler,
                    ..Default::default()
                });
            } else {
                image_infos.push(vk::DescriptorImageInfo {
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    image_view: self.null_texture.image_view,
                    sampler: self.null_tex_sampler,
                    ..Default::default()
                });
            }
        }
        let missing = 1024 - image_infos.len();
        for _ in 0..missing {
            image_infos.push(vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image_view: self.null_texture.image_view,
                sampler: self.null_tex_sampler,
                ..Default::default()
            });
        }
        let image_infos = image_infos.as_slice().as_ptr();

        let descriptor_write = vk::WriteDescriptorSet {
            s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
            dst_set: self.geometry_renderpass.descriptor_set.borrow().descriptor_sets[frame],
            dst_binding: 2,
            dst_array_element: 0,
            descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1024,
            p_image_info: image_infos,
            ..Default::default()
        };
        let forward_descriptor_write = vk::WriteDescriptorSet {
            s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
            dst_set: self.opaque_forward_renderpass.descriptor_set.borrow().descriptor_sets[frame],
            dst_binding: 3,
            dst_array_element: 0,
            descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1024,
            p_image_info: image_infos,
            ..Default::default()
        };
        self.context.device.update_descriptor_sets(&[descriptor_write, forward_descriptor_write], &[]);

        //self.generate_skybox(frame, "editor/resources/citrus_orchard_road_puresky_4k.hdr");
    }}

    pub unsafe fn render_world(
        &self,
        current_frame: usize,
        scene: &Scene, player_camera_index: usize,
    ) { unsafe {
        let device = &self.context.device;

        let frame_command_buffer = self.context.draw_command_buffers[current_frame];

        let ubo = scene.world.borrow().sun.to_sendable();
        
        let player_camera = &scene.world.borrow().cameras[player_camera_index];
        
        copy_data_to_memory(self.lighting_renderpass.descriptor_set.borrow().descriptors[10].owned_buffers.2[current_frame], &[ubo]);
        copy_data_to_memory(self.shadow_renderpass.descriptor_set.borrow().descriptors[2].owned_buffers.2[current_frame], &[ubo]);
        let ubo = SSAOPassUniformData {
            samples: self.ssao_kernal,
            projection: player_camera.projection_matrix.data,
            inverse_projection: player_camera.projection_matrix.inverse4().data,
            radius: 1.5,
            width: (self.ssao_renderpass.viewport.width * SSAO_RESOLUTION_MULTIPLIER) as i32,
            height: (self.ssao_renderpass.viewport.height * SSAO_RESOLUTION_MULTIPLIER) as i32,
            _pad0: 0.0,
        };
        copy_data_to_memory(self.ssao_renderpass.descriptor_set.borrow().descriptors[2].owned_buffers.2[current_frame], &[ubo]);
        let ubo = LightingUniformData {
            shadow_cascade_distances: [player_camera.far * 0.005, player_camera.far * 0.015, player_camera.far * 0.045, player_camera.far * 0.15],
            num_lights: scene.world.borrow().lights.len() as u32,
        };
        copy_data_to_memory(self.lighting_renderpass.descriptor_set.borrow().descriptors[9].owned_buffers.2[current_frame], &[ubo]);
        copy_data_to_memory(
            self.opaque_forward_renderpass.descriptor_set.borrow().descriptors[2].owned_buffers.2[current_frame],
            &[Vector::new2(
                self.viewport.borrow().width, self.viewport.borrow().width,
            )]
        );
        let camera_constants = CameraMatrixUniformData {
            view: player_camera.view_matrix.data,
            projection: player_camera.projection_matrix.data,
        };
        let camera_inverse_constants = CameraMatrixUniformData {
            view: player_camera.view_matrix.inverse4().data,
            projection: player_camera.projection_matrix.inverse4().data,
        };

        let radius = 20;
        let ssao_blur_constants_horizontal = BlurPassData {
            horizontal: 1,
            radius,
            near: player_camera.near,
            sigma_spatial: 20.0,
            sigma_depth: 0.25, // weighted within shader
            sigma_normal: 0.2,
            infinite_reverse_depth: 1
        };
        let ssao_blur_constants_vertical = BlurPassData {
            horizontal: 0,
            radius,
            near: player_camera.near,
            sigma_spatial: 20.0,
            sigma_depth: 0.25, // weighted within shader
            sigma_normal: 0.2,
            infinite_reverse_depth: 1
        };
        let ssao_upsample_constants = DepthAwareUpsamplePassData {
            near: player_camera.near,
            depth_threshold: 0.01,
            normal_threshold: 0.9,
            sharpness: 8.0,
            infinite_reverse_depth: 1
        };

        self.geometry_renderpass.do_renderpass(
            current_frame,
            frame_command_buffer,
            Some(|| {
                device.cmd_push_constants(frame_command_buffer, self.geometry_renderpass.pipeline_layout, ShaderStageFlags::VERTEX, 0, slice::from_raw_parts(
                    &camera_constants as *const CameraMatrixUniformData as *const u8,
                    size_of::<CameraMatrixUniformData>(),
                ));
            }),
            Some(|| {
                scene.draw(self, current_frame, Some(&player_camera), DrawMode::Deferred);
            }),
            None,
            Transition::ALL
        );

        self.shadow_renderpass.do_renderpass(
            current_frame,
            frame_command_buffer,
            None::<fn()>,
            Some(|| {
                scene.draw(self, current_frame, None, DrawMode::All);
            }),
            None,
            Transition::ALL
        );

        self.ssao_pre_downsample_renderpass.do_renderpass(
            current_frame,
            frame_command_buffer,
            None::<fn()>,
            None::<fn()>,
            None,
            Transition::ALL
        );
        self.ssao_renderpass.do_renderpass(current_frame, frame_command_buffer, None::<fn()>, None::<fn()>, None, Transition::ALL);
        self.ssao_blur_horizontal_renderpass.do_renderpass(current_frame, frame_command_buffer, Some(|| {
            device.cmd_push_constants(frame_command_buffer, self.ssao_blur_horizontal_renderpass.pipeline_layout, ShaderStageFlags::FRAGMENT, 0, slice::from_raw_parts(
                &ssao_blur_constants_horizontal as *const BlurPassData as *const u8,
                size_of::<BlurPassData>(),
            ));
        }), None::<fn()>, None, Transition::ALL);
        self.ssao_blur_vertical_renderpass.do_renderpass(current_frame, frame_command_buffer, Some(|| {
            device.cmd_push_constants(frame_command_buffer, self.ssao_blur_vertical_renderpass.pipeline_layout, ShaderStageFlags::FRAGMENT, 0, slice::from_raw_parts(
                &ssao_blur_constants_vertical as *const BlurPassData as *const u8,
                size_of::<BlurPassData>(),
            ));
        }), None::<fn()>, None, Transition::ALL);
        self.ssao_upsample_renderpass.do_renderpass(current_frame, frame_command_buffer, Some(|| {
            device.cmd_push_constants(frame_command_buffer, self.ssao_upsample_renderpass.pipeline_layout, ShaderStageFlags::FRAGMENT, 0, slice::from_raw_parts(
                &ssao_upsample_constants as *const DepthAwareUpsamplePassData as *const u8,
                size_of::<DepthAwareUpsamplePassData>(),
            ));
        }), None::<fn()>, None, Transition::ALL);


        self.lighting_renderpass.do_renderpass(
            current_frame,
            frame_command_buffer,
            Some(|| {
            device.cmd_push_constants(frame_command_buffer, self.lighting_renderpass.pipeline_layout, ShaderStageFlags::FRAGMENT, 0, slice::from_raw_parts(
                &camera_inverse_constants as *const CameraMatrixUniformData as *const u8,
                size_of::<CameraMatrixUniformData>(),
            ))
        }),
            None::<fn()>,
            None,
            Transition::START,
        );

        self.geometry_renderpass.pass.borrow().transition(
            frame_command_buffer,
            current_frame,
            None,
            Some((ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL, ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL, vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ)),
        );
        self.opaque_forward_renderpass.do_renderpass(
            current_frame,
            frame_command_buffer,
            Some(|| {
                device.cmd_push_constants(
                    frame_command_buffer,
                    self.opaque_forward_renderpass.pipeline_layout,
                    ShaderStageFlags::ALL_GRAPHICS,
                    0,
                    slice::from_raw_parts(
                        &camera_constants as *const CameraMatrixUniformData as *const u8,
                        128
                    )
                );
            }),
            Some(|| {
                self.context.device.cmd_bind_pipeline(
                    frame_command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.opaque_forward_renderpass.pipelines[1].vulkan_pipeline,
                );
                device.cmd_draw(frame_command_buffer, 36, 1, 0, 0);

                scene.draw(self, current_frame, Some(&player_camera), DrawMode::Forward);
                scene.draw(self, current_frame, Some(&player_camera), DrawMode::Outlined);
                scene.draw(self, current_frame, Some(&player_camera), DrawMode::Hitboxes);
            }),
            None,
            Transition::NONE
        );
        self.geometry_renderpass.pass.borrow().transition(
            frame_command_buffer,
            current_frame,
            None,
            Some((ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL, ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL, vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)),
        );

        self.cloud_renderpass.do_renderpass(
            current_frame,
            frame_command_buffer,
            Some(|| {
                device.cmd_push_constants(frame_command_buffer, self.cloud_renderpass.pipeline_layout, ShaderStageFlags::FRAGMENT, 0, slice::from_raw_parts(
                    &camera_constants as *const CameraMatrixUniformData as *const u8,
                    size_of::<CameraMatrixUniformData>(),
                ))
            }),
            None::<fn()>,
            None,
            Transition::NONE
        );

        self.outline_renderpass.do_renderpass(
            current_frame,
            frame_command_buffer,
            Some(|| {
                device.cmd_push_constants(
                    frame_command_buffer,
                    self.outline_renderpass.pipeline_layout,
                    ShaderStageFlags::FRAGMENT,
                    0,
                    slice::from_raw_parts(
                        &OutlineConstantSendable {
                            color: [0.93, 0.72, 0.0, 1.0],
                            thickness: 5.0,
                            _pad: [0.0; 3]
                        } as *const OutlineConstantSendable as *const u8,
                        size_of::<OutlineConstantSendable>(),
                    )
                )
            }),
            None::<fn()>,
            None,
            Transition::END
        );
    } }

    pub unsafe fn destroy(&mut self) { unsafe {
        self.geometry_renderpass.destroy();
        self.opaque_forward_renderpass.destroy();
        self.shadow_renderpass.destroy();
        self.ssao_pre_downsample_renderpass.destroy();
        self.ssao_renderpass.destroy();
        self.ssao_blur_horizontal_renderpass.destroy();
        self.ssao_blur_vertical_renderpass.destroy();
        self.ssao_upsample_renderpass.destroy();
        self.lighting_renderpass.destroy();
        self.outline_renderpass.destroy();
        self.cloud_renderpass.destroy();

        self.ssao_noise_texture.destroy();
        self.cloud_tex_low.destroy();
        self.cloud_tex_high.destroy();

        self.context.device.destroy_sampler(self.sampler, None);
        self.context.device.destroy_sampler(self.nearest_sampler, None);
        self.context.device.destroy_sampler(self.null_tex_sampler, None);
        self.context.device.destroy_sampler(self.repeat_sampler, None);

        self.null_texture.destroy(&self.context);

        self.context.device.destroy_buffer(self.editor_primitives_indices_buffer.0, None);
        self.context.device.destroy_buffer(self.editor_primitives_vertices_buffer.0, None);
        self.context.device.free_memory(self.editor_primitives_indices_buffer.1, None);
        self.context.device.free_memory(self.editor_primitives_vertices_buffer.1, None);
    } }
}
unsafe fn generate_cloud_noise(context: &Arc<Context>, high: &Texture, low: &Texture) { unsafe {
    let shape_noise_info = NoiseInfo {
        r: [3.0, 7.0, 11.0, 0.65],
        g: [9.0, 15.0, 23.0, 0.33],
        b: [13.0, 28.0, 42.0, 0.58],
        a: [20.0, 31.0, 45.0, 0.75],
        seeds: [1.0, 2.0, 3.0, 4.0],
        high: 1,
    };
    let detail_noise_info = NoiseInfo {
        r: [8.0, 18.0, 20.0, 0.76],
        g: [13.0, 24.0, 28.0, 0.5],
        b: [20.0, 28.0, 32.0, 0.5],
        a: [24.0, 30.0, 34.0, 0.5],
        seeds: [5.0, 6.0, 7.0, 8.0],
        high: 0,
    };

    let command_buffers = context.begin_single_time_commands(1);
    //<editor-fold desc = "transition in">
    let barrier_info = (
        ImageLayout::UNDEFINED,
        ImageLayout::GENERAL,
        vk::AccessFlags::COLOR_ATTACHMENT_READ,
        ImageAspectFlags::COLOR,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
    );
    transition_output(
        context,
        command_buffers[0],
        high.device_texture.borrow().image,
        1,
        barrier_info
    );
    transition_output(
        context,
        command_buffers[0],
        low.device_texture.borrow().image,
        1,
        barrier_info
    );
    //</editor-fold>
    //<editor-fold desc = "descriptors">
    let descriptor_create_info = DescriptorCreateInfo::new(context)
        .descriptor_type(DescriptorType::STORAGE_IMAGE)
        .shader_stages(ShaderStageFlags::COMPUTE);

    let descriptor_set_create_info = DescriptorSetCreateInfo::new(context)
        .frames_in_flight(1)
        .add_descriptor(Descriptor::new(&descriptor_create_info))
        .add_descriptor(Descriptor::new(&descriptor_create_info));
    let mut descriptor_set = DescriptorSet::new(descriptor_set_create_info);

    let image_infos = [
        vk::DescriptorImageInfo {
            sampler: vk::Sampler::null(),
            image_view: high.device_texture.borrow().image_view,
            image_layout: ImageLayout::GENERAL,
        }, // cloud high
        vk::DescriptorImageInfo {
            sampler: vk::Sampler::null(),
            image_view: low.device_texture.borrow().image_view,
            image_layout: ImageLayout::GENERAL,
        }, // cloud low
    ];
    let descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
        vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set.descriptor_sets[0])
            .dst_binding(i as u32)
            .descriptor_type(DescriptorType::STORAGE_IMAGE)
            .image_info(slice::from_ref(info))
    }).collect();
    context.device.update_descriptor_sets(&descriptor_writes, &[]);
    //</editor-fold>
    //<editor-fold desc = "pipeline">
    let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo {
        s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
        set_layout_count: 1,
        p_set_layouts: &descriptor_set.descriptor_set_layout,
        push_constant_range_count: 1,
        p_push_constant_ranges: &vk::PushConstantRange {
            stage_flags: ShaderStageFlags::COMPUTE,
            offset: 0,
            size: size_of::<NoiseInfo>() as u32,
        },
        ..Default::default()
    };
    let pipeline_layout = context.device.create_pipeline_layout(&pipeline_layout_create_info, None).unwrap();
    let mut spv_file = Cursor::new(load_file(&(SHADER_PATH.to_owned() + "\\cloud\\noise.comp.spv")).unwrap());
    let code = read_spv(&mut spv_file).expect("Failed to read compute shader spv file");
    let shader_info = vk::ShaderModuleCreateInfo::default().code(&code);
    let shader_module = context
        .device
        .create_shader_module(&shader_info, None)
        .expect("Compute shader module error");
    let shader_entry_name = c"main";
    let shader_stage_create_info = vk::PipelineShaderStageCreateInfo {
        module: shader_module,
        p_name: shader_entry_name.as_ptr(),
        stage: ShaderStageFlags::COMPUTE,
        ..Default::default()
    };
    let compute_pipeline_create_info = vk::ComputePipelineCreateInfo {
        s_type: vk::StructureType::COMPUTE_PIPELINE_CREATE_INFO,
        stage: shader_stage_create_info,
        layout: pipeline_layout,
        ..Default::default()
    };
    let compute_pipeline = context.device.create_compute_pipelines(
        vk::PipelineCache::null(),
        &[compute_pipeline_create_info],
        None
    ).unwrap()[0];
    //</editor-fold>
    //<editor-fold desc = "compute">
    context.device.cmd_bind_pipeline(command_buffers[0], vk::PipelineBindPoint::COMPUTE, compute_pipeline);
    context.device.cmd_bind_descriptor_sets(
        command_buffers[0],
        vk::PipelineBindPoint::COMPUTE,
        pipeline_layout,
        0,
        &descriptor_set.descriptor_sets,
        &[],
    );
    let sub_divs = 4;

    context.device.cmd_push_constants(command_buffers[0], pipeline_layout, ShaderStageFlags::COMPUTE, 0, slice::from_raw_parts(
        &shape_noise_info as *const NoiseInfo as *const u8,
        size_of::<NoiseInfo>(),
    ));
    context.device.cmd_dispatch(command_buffers[0], 128 / sub_divs, 128 / sub_divs, 128 / sub_divs);

    context.device.cmd_push_constants(command_buffers[0], pipeline_layout, ShaderStageFlags::COMPUTE, 0, slice::from_raw_parts(
        &detail_noise_info as *const NoiseInfo as *const u8,
        size_of::<NoiseInfo>(),
    ));
    context.device.cmd_dispatch(command_buffers[0], 32 / sub_divs, 32 / sub_divs, 32 / sub_divs);

    //</editor-fold>
    //<editor-fold desc = "transition out">
    let barrier_info = (
        ImageLayout::GENERAL,
        ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        ImageAspectFlags::COLOR,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
    );
    transition_output(
        context,
        command_buffers[0],
        high.device_texture.borrow().image,
        1,
        barrier_info
    );
    transition_output(
        context,
        command_buffers[0],
        low.device_texture.borrow().image,
        1,
        barrier_info
    );
    //</editor-fold>

    context.end_single_time_commands(command_buffers);

    descriptor_set.destroy();
    context.device.destroy_pipeline_layout(pipeline_layout, None);
    context.device.destroy_shader_module(shader_module, None);
    context.device.destroy_pipeline(compute_pipeline, None);
} }

#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub(crate) struct CameraMatrixUniformData {
    pub(crate) view: [f32; 16],
    pub(crate) projection: [f32; 16],
}
#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct LightingUniformData {
    shadow_cascade_distances: [f32; 4],
    num_lights: u32,
}
#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct SSAOPassUniformData {
    samples: [[f32; 4]; SSAO_KERNAL_SIZE],
    projection: [f32; 16],
    inverse_projection: [f32; 16],
    radius: f32,
    width: i32,
    height: i32,
    _pad0: f32,
}
#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct BlurPassData {
    horizontal: i32,
    radius: i32,
    near: f32,
    sigma_spatial: f32, // texel-space sigma
    sigma_depth: f32, // view-space sigma
    sigma_normal: f32, // normal dot sigma
    infinite_reverse_depth: i32,
}
#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct DepthAwareUpsamplePassData {
    near: f32,
    depth_threshold: f32, // sensitivity for depth edges (e.g., 0.01-0.1)
    normal_threshold: f32, // sensitivity for normal edges (e.g., 0.9)
    sharpness: f32, // edge sharpness multiplier (e.g., 8.0)
    infinite_reverse_depth: i32
}
#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct OutlineConstantSendable {
    color: [f32; 4],

    thickness: f32,
    _pad: [f32; 3],
}
#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct NoiseInfo {
    r: [f32; 4],
    g: [f32; 4],
    b: [f32; 4],
    a: [f32; 4],
    seeds: [f32; 4],
    high: i32,
}
/*
        let equirectangular_to_cubemap_pass_create_info = PassCreateInfo::new(context)
        .add_color_attachment_info(TextureCreateInfo::new(context).format(Format::R16G16B16A16_SFLOAT)
            .add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC)
            .width(512).height(512)
            .is_cubemap(true)
        );
    let equirectangular_to_cubemap_renderpass_create_info = RenderpassCreateInfo::new(&context)
        .pass_create_info(equirectangular_to_cubemap_pass_create_info)
        .viewport(vk::Viewport {
            x: 0.0, y: 0.0,
            width: 512.0, height: 512.0,
            min_depth: 0.0, max_depth: 1.0,
        })
        .descriptor_set_create_info(DescriptorSetCreateInfo::new(context)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
        )
        .add_pipeline_create_info(PipelineCreateInfo::new()
            .vertex_shader_uri(String::from("equirectangular_to_cubemap\\equirectangular_to_cubemap.vert.spv"))
            .fragment_shader_uri(String::from("equirectangular_to_cubemap\\equirectangular_to_cubemap.frag.spv"))
        );
    let equirectangular_to_cubemap_renderpass = Renderpass::new(equirectangular_to_cubemap_renderpass_create_info);



pub unsafe fn generate_skybox(&self, current_frame: usize, uri: &str) { unsafe {
    let texture_info = self.context.load_textures_batched(&[PathBuf::from(uri)], false);
    let texture = DeviceTexture {
        image: texture_info[0].1.0,
        image_view: texture_info[0].0.0,
        stencil_image_view: None,
        device_memory: texture_info[0].1.1,
        destroyed: false,
    };

    let device = &self.context.device;

    let command_buffer = self.context.begin_single_time_commands(1);

    let image_info = vk::DescriptorImageInfo {
        sampler: self.sampler,
        image_view: texture.image_view,
        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    };
    let descriptor_write = vk::WriteDescriptorSet {
        s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
        dst_set: self.equirectangular_to_cubemap_renderpass.descriptor_set.borrow().descriptor_sets[current_frame],
        dst_binding: 0,
        dst_array_element: 0,
        descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
        descriptor_count: 1,
        p_image_info: [image_info].as_ptr(),
        ..Default::default()
    };
    device.update_descriptor_sets(&[descriptor_write], &[]);

    self.equirectangular_to_cubemap_renderpass.do_renderpass(
        current_frame,
        command_buffer[0],
        None::<fn()>,
        Some(|| {
            device.cmd_draw(command_buffer[0], 36, 1, 0, 0);
        }),
        None,
        true
    );

    self.context.end_single_time_commands(command_buffer);
} }
 */