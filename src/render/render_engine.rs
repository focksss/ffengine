use crate::{mem, FrametimeManager};
use std::path::PathBuf;
use std::slice;
use ash::vk;
use ash::vk::{DescriptorType, Format, ShaderStageFlags};
use rand::{rng, Rng};
use crate::{offset_of, render, MAX_FRAMES_IN_FLIGHT};
use crate::engine::camera::Camera;
use crate::engine::scene;
use crate::math::*;
use crate::render::*;
use crate::scene::*;

const SSAO_KERNAL_SIZE: usize = 16;
const SSAO_RESOLUTION_MULTIPLIER: f32 = 0.5;
pub const SHADOW_RES: u32 = 4096;

pub struct RenderEngine<'a> {
    base: &'a VkBase,

    pub null_texture: Texture,
    pub null_tex_sampler: vk::Sampler,

    pub geometry_renderpass: Renderpass,
    pub shadow_renderpass: Renderpass,

    pub ssao_pre_downsample_renderpass: Renderpass,
    pub ssao_renderpass: Renderpass,
    pub ssao_blur_renderpass_upsample: Renderpass,

    pub lighting_renderpass: Renderpass,

    pub present_renderpass: Renderpass,

    pub sampler: vk::Sampler,
    pub nearest_sampler: vk::Sampler,
    pub ssao_kernal: [[f32; 4]; SSAO_KERNAL_SIZE],
    pub ssao_noise_texture: Texture,
}
impl<'a> RenderEngine<'a> {
    pub unsafe fn new(base: &'a VkBase, world: &Scene) -> RenderEngine<'a> { unsafe {
        let null_tex_info = unsafe { base.create_2d_texture_image(&PathBuf::from("").join("resources\\null8x.png"), true) };

        let null_texture = Texture {
            image: null_tex_info.1.0,
            image_view: null_tex_info.0.0,
            device_memory: null_tex_info.1.1,
            clear_value: vk::ClearValue::default(),
            format: Format::R8G8B8A8_UNORM,
            resolution: vk::Extent3D::default(),
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            is_depth: false,
        };
        let null_tex_sampler = null_tex_info.0.1;

        let image_infos: Vec<vk::DescriptorImageInfo> = vec![vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view: null_texture.image_view,
            sampler: null_tex_sampler,
            ..Default::default()
        }; 1024];

        let texture_sampler_create_info = DescriptorCreateInfo::new(base)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT);
        //<editor-fold desc = "passes">
        let ssao_res_color_tex_create_info = TextureCreateInfo::new(base).format(Format::R8_UNORM).resolution_denominator((1.0 / SSAO_RESOLUTION_MULTIPLIER) as u32);
        let geometry_pass_create_info = PassCreateInfo::new(base)
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R16_SINT)) // material
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R8G8B8A8_UNORM)) // albedo
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R8G8B8A8_UNORM)) // metallic roughness
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R8G8B8A8_UNORM)) // extra properties
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R16G16B16A16_SFLOAT)) // view normal
            .depth_attachment_info(TextureCreateInfo::new(base).format(Format::D32_SFLOAT).is_depth(true).clear_value([0.0, 0.0, 0.0, 0.0])); // depth

        let shadow_pass_create_info = PassCreateInfo::new(base)
            .depth_attachment_info(TextureCreateInfo::new(base).format(Format::D16_UNORM).is_depth(true).clear_value([0.0, 0.0, 0.0, 0.0]).width(SHADOW_RES).height(SHADOW_RES).array_layers(5)); // depth;

        let ssao_depth_downsample_pass_create_info = PassCreateInfo::new(base)
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R16G16B16A16_SFLOAT).resolution_denominator((1.0 / SSAO_RESOLUTION_MULTIPLIER) as u32));
        let ssao_pass_create_info = PassCreateInfo::new(base)
            .add_color_attachment_info(ssao_res_color_tex_create_info);
        let ssao_blur_pass_create_info_upsample = PassCreateInfo::new(base)
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R8_UNORM));

        let lighting_pass_create_info = PassCreateInfo::new(base)
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R16G16B16A16_SFLOAT).add_usage_flag(vk::ImageUsageFlags::TRANSFER_SRC));

        let present_pass_create_info = PassCreateInfo::new(base)
            .set_is_present_pass(true);
        //</editor-fold>
        //<editor-fold desc = "geometry + shadow descriptor sets"
        let sun_ubo_create_info = DescriptorCreateInfo::new(base)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .shader_stages(ShaderStageFlags::GEOMETRY)
            .size(size_of::<SunSendable>() as u64);
        let material_ssbo_create_info = DescriptorCreateInfo::new(base)
            .descriptor_type(DescriptorType::STORAGE_BUFFER)
            .shader_stages(ShaderStageFlags::FRAGMENT)
            .buffers(world.material_buffers.iter().map(|b| {b.0.clone()}).collect());
        let joints_ssbo_create_info = DescriptorCreateInfo::new(base)
            .descriptor_type(DescriptorType::STORAGE_BUFFER)
            .shader_stages(ShaderStageFlags::VERTEX)
            .buffers(world.joints_buffers.iter().map(|b| {b.0.clone()}).collect());
        let world_texture_samplers_create_info = DescriptorCreateInfo::new(base)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .shader_stages(ShaderStageFlags::FRAGMENT)
            .dynamic(true)
            .image_infos(image_infos.clone());

        let geometry_descriptor_set_create_info = DescriptorSetCreateInfo::new(base)
            .add_descriptor(Descriptor::new(&material_ssbo_create_info))
            .add_descriptor(Descriptor::new(&joints_ssbo_create_info))
            .add_descriptor(Descriptor::new(&world_texture_samplers_create_info));

        let shadow_descriptor_set_create_info = DescriptorSetCreateInfo::new(base)
            .add_descriptor(Descriptor::new(&material_ssbo_create_info))
            .add_descriptor(Descriptor::new(&joints_ssbo_create_info))
            .add_descriptor(Descriptor::new(&sun_ubo_create_info))
            .add_descriptor(Descriptor::new(&world_texture_samplers_create_info));
        //</editor-fold>]
        // <editor-fold desc = "SSAO descriptor sets">
        let ssao_depth_downsample_descriptor_set_create_info = DescriptorSetCreateInfo::new(base)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&texture_sampler_create_info));
        let ssbo_ubo_create_info = DescriptorCreateInfo::new(base)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .size(size_of::<SSAOPassUniformData>() as u64)
            .shader_stages(ShaderStageFlags::FRAGMENT);
        let ssao_descriptor_set_create_info = DescriptorSetCreateInfo::new(base)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&texture_sampler_create_info))
            .add_descriptor(Descriptor::new(&ssbo_ubo_create_info));
        let ssao_blur_descriptor_set_create_info_upsample_horiz = DescriptorSetCreateInfo::new(base)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info)) // input (ssao)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info)) // g_info (ssao res)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info)) // normal (full res)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info)); // depth (full res)
        //</editor-fold>
        //<editor-fold desc = "lighting descriptor set">
        let lights_ssbo_create_info = DescriptorCreateInfo::new(base)
            .descriptor_type(DescriptorType::STORAGE_BUFFER)
            .shader_stages(ShaderStageFlags::FRAGMENT)
            .buffers(world.lights_buffers.iter().map(|b| {b.0.clone()}).collect());
        let sun_ubo_create_info = DescriptorCreateInfo::new(base)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .size(size_of::<SunSendable>() as u64)
            .shader_stages(ShaderStageFlags::FRAGMENT);
        let lighting_ubo_create_info = DescriptorCreateInfo::new(base)
            .descriptor_type(DescriptorType::UNIFORM_BUFFER)
            .size(size_of::<LightingUniformData>() as u64)
            .shader_stages(ShaderStageFlags::FRAGMENT);
        let lighting_descriptor_set_create_info = DescriptorSetCreateInfo::new(base)
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
            .add_descriptor(Descriptor::new(&DescriptorCreateInfo::new(base)
                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                .shader_stages(ShaderStageFlags::FRAGMENT)
                .binding_flags(vk::DescriptorBindingFlags::UPDATE_AFTER_BIND))
            );
        //</editor-fold>
        // <editor-fold desc = "present descriptor set">
        let present_descriptor_set_create_info = DescriptorSetCreateInfo::new(base)
            .add_descriptor(Descriptor::new(&texture_sampler_create_info));
        //</editor-fold>

        //<editor-fold desc = "ssao sampling setup">
        let mut rng = rng();
        let mut ssao_kernal= [[0.0; 4]; SSAO_KERNAL_SIZE];
        for i in 0..SSAO_KERNAL_SIZE {
            let mut scale = i as f32 / SSAO_KERNAL_SIZE as f32;
            scale = 0.1 + ((scale * scale) * (1.0 - 0.1));
            ssao_kernal[i] = (Vector::new_from_array(&[rng.random::<f32>() * 2.0 - 1.0, rng.random::<f32>() * 2.0 - 1.0, rng.random::<f32>()])
                .normalize_3d() * rng.random::<f32>() * scale).to_array4();
        }

        let mut noise_data = Vec::<[f32; 2]>::with_capacity(16);
        for _ in 0..16 {
            noise_data.push([
                rng.random_range(-1.0..1.0),
                rng.random_range(-1.0..1.0),
            ]);
        }
        let ssao_noise_tex_info = TextureCreateInfo::new(base)
            .width(4)
            .height(4)
            .depth(1)
            .format(Format::R16G16_SFLOAT)
            .usage_flags(
                vk::ImageUsageFlags::SAMPLED |
                    vk::ImageUsageFlags::TRANSFER_DST
            );
        let ssao_noise_texture = Texture::new(&ssao_noise_tex_info);

        let ((staging_buffer, staging_buffer_memory), _) = base.create_device_and_staging_buffer(
            0,
            &noise_data,
            vk::BufferUsageFlags::TRANSFER_SRC,
            true,
            false,
            true,
        );
        base.transition_image_layout(
            ssao_noise_texture.image,
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
        base.copy_buffer_to_image(
            staging_buffer,
            ssao_noise_texture.image,
            vk::Extent3D { width: 4, height: 4, depth: 1 },
        );
        base.transition_image_layout(
            ssao_noise_texture.image,
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
        base.device.destroy_buffer(staging_buffer, None);
        base.device.free_memory(staging_buffer_memory, None);
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
            }, // join weights

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
                format: Format::R32G32_SINT,
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
                format: Format::R32G32_SINT,
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
        let null_blend_states = [null_blend_attachment; 5];
        let null_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(&null_blend_states);
        let null_blend_states2 = [null_blend_attachment; 2];
        let null_blend_state2 = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(&null_blend_states2);
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);
        //</editor-fold>


        let geometry_renderpass_create_info = { RenderpassCreateInfo::new(base)
            .pass_create_info(geometry_pass_create_info)
            .descriptor_set_create_info(geometry_descriptor_set_create_info)
            .vertex_shader_uri(String::from("geometry\\geometry.vert.spv"))
            .fragment_shader_uri(String::from("geometry\\geometry.frag.spv"))
            .push_constant_range(camera_push_constant_range_vertex)
            .pipeline_vertex_input_state(geometry_vertex_input_state_info)
            .pipeline_rasterization_state(rasterization_info)
            .pipeline_depth_stencil_state(infinite_reverse_depth_state_info)
            .pipeline_color_blend_state_create_info(null_blend_state) };
        let geometry_renderpass = Renderpass::new(geometry_renderpass_create_info);

        let shadow_renderpass_create_info = { RenderpassCreateInfo::new(base)
            .pass_create_info(shadow_pass_create_info)
            .descriptor_set_create_info(shadow_descriptor_set_create_info)
            .vertex_shader_uri(String::from("shadow\\shadow.vert.spv"))
            .fragment_shader_uri(String::from("shadow\\shadow.frag.spv"))
            .geometry_shader_uri(String::from("shadow\\cascade.geom.spv"))
            .pipeline_vertex_input_state(shadow_vertex_input_state_info)
            .pipeline_rasterization_state(shadow_rasterization_info)
            .pipeline_depth_stencil_state(shadow_depth_state_info)
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

        let ssao_pre_downsample_renderpass_create_info = { RenderpassCreateInfo::new(base)
            .pass_create_info(ssao_depth_downsample_pass_create_info)
            .descriptor_set_create_info(ssao_depth_downsample_descriptor_set_create_info)
            .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
            .fragment_shader_uri(String::from("ssao\\pre_downsample.frag.spv"))
            .viewport(vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: base.surface_resolution.width as f32 * SSAO_RESOLUTION_MULTIPLIER,
                height: base.surface_resolution.height as f32 * SSAO_RESOLUTION_MULTIPLIER,
                min_depth: 0.0,
                max_depth: 1.0,
            })
            .scissor(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width: (base.surface_resolution.width as f32 * SSAO_RESOLUTION_MULTIPLIER) as u32,
                height: (base.surface_resolution.height as f32 * SSAO_RESOLUTION_MULTIPLIER) as u32
            } }) };
        let ssao_pre_downsample_renderpass = Renderpass::new(ssao_pre_downsample_renderpass_create_info);
        let ssao_renderpass_create_info = { RenderpassCreateInfo::new(base)
            .pass_create_info(ssao_pass_create_info)
            .descriptor_set_create_info(ssao_descriptor_set_create_info)
            .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
            .fragment_shader_uri(String::from("ssao\\ssao.frag.spv"))
            .viewport(vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: base.surface_resolution.width as f32 * SSAO_RESOLUTION_MULTIPLIER,
                height: base.surface_resolution.height as f32 * SSAO_RESOLUTION_MULTIPLIER,
                min_depth: 0.0,
                max_depth: 1.0,
            })
            .scissor(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width: (base.surface_resolution.width as f32 * SSAO_RESOLUTION_MULTIPLIER) as u32,
                height: (base.surface_resolution.height as f32 * SSAO_RESOLUTION_MULTIPLIER) as u32
            } }) };
        let ssao_renderpass = Renderpass::new(ssao_renderpass_create_info);
        let ssao_blur_renderpass_create_info_upsample = { RenderpassCreateInfo::new(base)
            .pass_create_info(ssao_blur_pass_create_info_upsample)
            .descriptor_set_create_info(ssao_blur_descriptor_set_create_info_upsample_horiz)
            .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
            .fragment_shader_uri(String::from("bilateral_blur\\bilateral_blur_upsample.frag.spv"))
            .push_constant_range(vk::PushConstantRange {
                stage_flags: ShaderStageFlags::FRAGMENT,
                offset: 0,
                size: size_of::<BlurPassData>() as _,
            }) };
        let ssao_blur_renderpass_upsample = Renderpass::new(ssao_blur_renderpass_create_info_upsample);

        let lighting_renderpass_create_info = { RenderpassCreateInfo::new(base)
            .pass_create_info(lighting_pass_create_info)
            .descriptor_set_create_info(lighting_descriptor_set_create_info)
            .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
            .fragment_shader_uri(String::from("lighting\\lighting.frag.spv"))
            .push_constant_range(camera_push_constant_range_fragment) };
        let lighting_renderpass = Renderpass::new(lighting_renderpass_create_info);

        let present_renderpass_create_info = { RenderpassCreateInfo::new(base)
            .pass_create_info(present_pass_create_info)
            .descriptor_set_create_info(present_descriptor_set_create_info)
            .vertex_shader_uri(String::from("quad\\quad.vert.spv"))
            .fragment_shader_uri(String::from("quad\\quad.frag.spv")) };
        let present_renderpass = Renderpass::new(present_renderpass_create_info);

        //<editor-fold desc = "descriptor updates">
        let sampler = base.device.create_sampler(&vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_BORDER,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            ..Default::default()
        }, None).unwrap();
        let nearest_sampler = base.device.create_sampler(&vk::SamplerCreateInfo {
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
                    image_view: geometry_renderpass.pass.textures[current_frame][0].image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // material
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.textures[current_frame][1].image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // albedo
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.textures[current_frame][2].image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // metallic roughness
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.textures[current_frame][3].image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // extra properties
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.textures[current_frame][5].image_view,
                    image_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                }, // depth
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.textures[current_frame][4].image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // view normal
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: shadow_renderpass.pass.textures[current_frame][0].image_view,
                    image_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                }, // shadow map
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: ssao_blur_renderpass_upsample.pass.textures[current_frame][0].image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // ssao tex (final)
            ];
            let lighting_descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(lighting_renderpass.descriptor_set.descriptor_sets[current_frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            base.device.update_descriptor_sets(&lighting_descriptor_writes, &[]);
            //</editor-fold>
            //<editor-fold desc = "present">
            let present_info = [vk::DescriptorImageInfo {
                sampler,
                image_view: lighting_renderpass.pass.textures[current_frame][0].image_view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            }];
            let present_descriptor_writes: Vec<vk::WriteDescriptorSet> = vec![
                vk::WriteDescriptorSet::default()
                    .dst_set(present_renderpass.descriptor_set.descriptor_sets[current_frame])
                    .dst_binding(0)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&present_info)];
            base.device.update_descriptor_sets(&present_descriptor_writes, &[]);
            //</editor-fold>
            //<editor-fold desc = "ssao pre downsample">
            let image_infos = [
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.textures[current_frame][5].image_view,
                    image_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                }, // geometry depth
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.textures[current_frame][4].image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // view normal
            ];
            let ssao_pre_downsample_descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(ssao_pre_downsample_renderpass.descriptor_set.descriptor_sets[current_frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            base.device.update_descriptor_sets(&ssao_pre_downsample_descriptor_writes, &[]);
            //</editor-fold>
            //<editor-fold desc = "ssao gen">
            let image_infos = [
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: ssao_pre_downsample_renderpass.pass.textures[current_frame][0].image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // downsampled normal + depth
                vk::DescriptorImageInfo {
                    sampler: nearest_sampler,
                    image_view: ssao_noise_texture.image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // noise tex
            ];
            let ssao_descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(ssao_renderpass.descriptor_set.descriptor_sets[current_frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            base.device.update_descriptor_sets(&ssao_descriptor_writes, &[]);
            //</editor-fold>
            //<editor-fold desc = "ssao blur">
            let image_infos = [
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: ssao_renderpass.pass.textures[current_frame][0].image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // ssao raw
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: ssao_pre_downsample_renderpass.pass.textures[current_frame][0].image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // downsampled normal + depth
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.textures[current_frame][4].image_view,
                    image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                }, // view normal
                vk::DescriptorImageInfo {
                    sampler,
                    image_view: geometry_renderpass.pass.textures[current_frame][5].image_view,
                    image_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                }, // geometry depth
            ];
            let descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(ssao_blur_renderpass_upsample.descriptor_set.descriptor_sets[current_frame])
                    .dst_binding(i as u32)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(slice::from_ref(info))
            }).collect();
            base.device.update_descriptor_sets(&descriptor_writes, &[]);
            //</editor-fold>
        }
        //</editor-fold>

        RenderEngine {
            base,

            null_texture,
            null_tex_sampler,

            present_renderpass,

            geometry_renderpass,
            shadow_renderpass,
            ssao_pre_downsample_renderpass,
            ssao_renderpass,
            ssao_blur_renderpass_upsample,
            lighting_renderpass,

            sampler,
            nearest_sampler,
            ssao_kernal,
            ssao_noise_texture,
        }
    } }
    pub fn update_world_textures_all_frames(&self, world: &Scene) {
        for frame in 0..MAX_FRAMES_IN_FLIGHT {
            self.update_world_textures(world, frame);
        }
    }
    pub fn update_world_textures(&self, world: &Scene, frame: usize) { unsafe {
        let mut image_infos: Vec<vk::DescriptorImageInfo> = Vec::with_capacity(1024);
        for model in &world.models {
            for texture in &model.textures {
                if texture.borrow().source.borrow().generated {
                    image_infos.push(vk::DescriptorImageInfo {
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        image_view: texture.borrow().source.borrow().image_view,
                        sampler: texture.borrow().sampler,
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
            dst_set: self.geometry_renderpass.descriptor_set.descriptor_sets[frame],
            dst_binding: 2,
            dst_array_element: 0,
            descriptor_type: DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1024,
            p_image_info: image_infos,
            ..Default::default()
        };
        self.base.device.update_descriptor_sets(&[descriptor_write], &[]);
    }}

    pub unsafe fn render_frame(
        &self,
        current_frame: usize,
        world: &Scene, player_camera: &Camera,
        frametime_manager: &mut FrametimeManager,
        text_renderer: &TextRenderer
    ) { unsafe {
        let base = self.base;
        let device = &base.device;

        let frame_command_buffer = base.draw_command_buffers[current_frame];

        let image_info = vk::DescriptorImageInfo {
            sampler: self.sampler,
            image_view: text_renderer.renderpass.pass.textures[current_frame][0].image_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };
        let descriptor_write = vk::WriteDescriptorSet::default()
            .dst_set(self.lighting_renderpass.descriptor_set.descriptor_sets[current_frame])
            .dst_binding(11u32)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(slice::from_ref(&image_info));

        base.device.update_descriptor_sets(&[descriptor_write], &[]);

        let ubo = world.sun.to_sendable();
        copy_data_to_memory(self.lighting_renderpass.descriptor_set.descriptors[10].owned_buffers.2[current_frame], &[ubo]);
        copy_data_to_memory(self.shadow_renderpass.descriptor_set.descriptors[2].owned_buffers.2[current_frame], &[ubo]);
        let ubo = SSAOPassUniformData {
            samples: self.ssao_kernal,
            projection: player_camera.projection_matrix.data,
            inverse_projection: player_camera.projection_matrix.inverse().data,
            radius: 1.5,
            width: (base.surface_resolution.width as f32 * SSAO_RESOLUTION_MULTIPLIER) as i32,
            height: (base.surface_resolution.height as f32 * SSAO_RESOLUTION_MULTIPLIER) as i32,
            _pad0: 0.0,
        };
        copy_data_to_memory(self.ssao_renderpass.descriptor_set.descriptors[2].owned_buffers.2[current_frame], &[ubo]);
        let ubo = LightingUniformData {
            shadow_cascade_distances: [player_camera.far * 0.005, player_camera.far * 0.015, player_camera.far * 0.045, player_camera.far * 0.15]
        };
        copy_data_to_memory(self.lighting_renderpass.descriptor_set.descriptors[9].owned_buffers.2[current_frame], &[ubo]);
        let camera_constants = CameraMatrixUniformData {
            view: player_camera.view_matrix.data,
            projection: player_camera.projection_matrix.data,
        };
        let camera_inverse_constants = CameraMatrixUniformData {
            view: player_camera.view_matrix.inverse().data,
            projection: player_camera.projection_matrix.inverse().data,
        };

        let ssao_blur_constants = BlurPassData {
            radius: 5,
            near: player_camera.near,
            sigma_spatial: 2.5,
            sigma_depth: 0.25, // weighted within shader
            sigma_normal: 0.2,
            infinite_reverse_depth: 1
        };

        self.geometry_renderpass.do_renderpass(
            base,
            current_frame,
            frame_command_buffer,
            Some(|| {
                device.cmd_push_constants(frame_command_buffer, self.geometry_renderpass.pipeline_layout, ShaderStageFlags::VERTEX, 0, slice::from_raw_parts(
                    &camera_constants as *const CameraMatrixUniformData as *const u8,
                    size_of::<CameraMatrixUniformData>(),
                ));
            }),
            Some(|| {
                world.draw(&frame_command_buffer, current_frame, Some(&player_camera.frustum));
            })
        );

        self.shadow_renderpass.do_renderpass(
            base,
            current_frame,
            frame_command_buffer,
            None::<fn()>,
            Some(|| {
                world.draw(&frame_command_buffer, current_frame, None);
            })
        );



        self.ssao_pre_downsample_renderpass.do_renderpass(base, current_frame, frame_command_buffer, None::<fn()>, None::<fn()>);

        self.ssao_renderpass.do_renderpass(base, current_frame, frame_command_buffer, None::<fn()>, None::<fn()>);

        self.ssao_blur_renderpass_upsample.do_renderpass(base, current_frame, frame_command_buffer, Some(|| {
            device.cmd_push_constants(frame_command_buffer, self.ssao_blur_renderpass_upsample.pipeline_layout, ShaderStageFlags::FRAGMENT, 0, slice::from_raw_parts(
                &ssao_blur_constants as *const BlurPassData as *const u8,
                size_of::<BlurPassData>(),
            ));
        }), None::<fn()>);


        //<editor-fold desc = "lighting pass">
        device.cmd_begin_render_pass(
            frame_command_buffer,
            &self.lighting_renderpass.get_pass_begin_info(current_frame),
            vk::SubpassContents::INLINE,
        );
        device.cmd_bind_pipeline(
            frame_command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.lighting_renderpass.pipeline,
        );
        device.cmd_push_constants(frame_command_buffer, self.lighting_renderpass.pipeline_layout, ShaderStageFlags::FRAGMENT, 0, slice::from_raw_parts(
            &camera_inverse_constants as *const CameraMatrixUniformData as *const u8,
            size_of::<CameraMatrixUniformData>(),
        ));

        // draw quad
        device.cmd_set_viewport(frame_command_buffer, 0, &[self.present_renderpass.viewport]);
        device.cmd_set_scissor(frame_command_buffer, 0, &[self.present_renderpass.scissor]);
        device.cmd_bind_descriptor_sets(
            frame_command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.lighting_renderpass.pipeline_layout,
            0,
            &[self.lighting_renderpass.descriptor_set.descriptor_sets[current_frame]],
            &[],
        );
        device.cmd_draw(frame_command_buffer, 6, 1, 0, 0);

        device.cmd_end_render_pass(frame_command_buffer);
        //</editor-fold>
        self.lighting_renderpass.pass.transition_to_readable(base, frame_command_buffer, current_frame);

        // <editor-fold desc = "present pass">
        device.cmd_begin_render_pass(
            frame_command_buffer,
            &self.present_renderpass.get_pass_begin_info(current_frame),
            vk::SubpassContents::INLINE,
        );
        device.cmd_bind_pipeline(
            frame_command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.present_renderpass.pipeline,
        );

        // draw quad
        device.cmd_set_viewport(frame_command_buffer, 0, &[self.present_renderpass.viewport]);
        device.cmd_set_scissor(frame_command_buffer, 0, &[self.present_renderpass.scissor]);
        device.cmd_bind_descriptor_sets(
            frame_command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.present_renderpass.pipeline_layout,
            0,
            &[self.present_renderpass.descriptor_set.descriptor_sets[current_frame]],
            &[],
        );
        device.cmd_draw(frame_command_buffer, 6, 1, 0, 0);

        device.cmd_end_render_pass(frame_command_buffer);
        //</editor-fold>
    } }
}
impl Drop for RenderEngine<'_> {
    fn drop(&mut self) { unsafe {
        let base = self.base;

        self.geometry_renderpass.destroy(base);
        self.shadow_renderpass.destroy(base);
        self.ssao_pre_downsample_renderpass.destroy(base);
        self.ssao_renderpass.destroy(base);
        self.ssao_blur_renderpass_upsample.destroy(base);
        self.lighting_renderpass.destroy(base);
        self.present_renderpass.destroy(base);

        self.ssao_noise_texture.destroy(base);

        base.device.destroy_sampler(self.sampler, None);
        base.device.destroy_sampler(self.nearest_sampler, None);

        self.null_texture.destroy(base);
        base.device.destroy_sampler(self.null_tex_sampler, None);
    } }
}

#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct CameraMatrixUniformData {
    view: [f32; 16],
    projection: [f32; 16],
}
#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct LightingUniformData {
    shadow_cascade_distances: [f32; 4],
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
    radius: i32,
    near: f32,
    sigma_spatial: f32, // texel-space sigma
    sigma_depth: f32, // view-space sigma
    sigma_normal: f32, // normal dot sigma
    infinite_reverse_depth: i32,
}