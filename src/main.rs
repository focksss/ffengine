#![warn(unused_qualifications)]
mod matrix;
mod vector;
mod vk_helper;
mod camera;
mod scene;
mod render;

use std::default::Default;
use std::error::Error;
use std::mem;
use std::collections::HashSet;
use std::ffi::c_void;
use std::mem::size_of;
use std::path::PathBuf;
use std::time::Instant;

use ash::vk;
use ash::vk::{Buffer, DeviceMemory, Format, ShaderStageFlags};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::CursorGrabMode;
use crate::{vk_helper::*, vector::*};
use crate::scene::{Scene, Model, Instance, Light};
use crate::camera::Camera;
use crate::matrix::Matrix;
use crate::render::{Pass, PassCreateInfo, Shader, TextureCreateInfo};

#[derive(Clone, Debug, Copy)]
#[repr(C)]
struct UniformData {
    view: [f32; 16],
    projection: [f32; 16],
}

const MAX_FRAMES_IN_FLIGHT: usize = 2;
const PI: f32 = std::f32::consts::PI;

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        let mut shader_paths = Vec::new();
        shader_paths.push("src\\shaders\\glsl\\geometry");
        shader_paths.push("src\\shaders\\glsl\\shadow");
        shader_paths.push("src\\shaders\\glsl\\lighting");
        shader_paths.push("src\\shaders\\glsl\\quad");

        compile_shaders(shader_paths).expect("Failed to compile shaders");

        let mut base = VkBase::new("ffengine".to_string(), 1920, 1080, MAX_FRAMES_IN_FLIGHT)?;
        run(&mut base).expect("Application launch failed!");
    }
    Ok(())
}

unsafe fn run(base: &mut VkBase) -> Result<(), Box<dyn Error>> {
    unsafe {
        let mut world = Scene::new();

        // world.add_model(Model::new(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("local_assets\\ffocks\\untitled.gltf").to_str().unwrap()));
        // world.models[0].transform_roots(&Vector::new_vec(0.0), &Vector::new_vec(0.0), &Vector::new_vec(0.01));
        // world.models[0].animations[0].repeat = true;
        // world.models[0].animations[0].start();

        //world.add_model(Model::new("C:\\Graphics\\assets\\flower\\scene.gltf"));
        //world.add_model(Model::new("C:\\Graphics\\assets\\rivals\\luna\\gltf\\luna.gltf"));

        //world.add_model(Model::new("C:\\Graphics\\assets\\bistro2\\untitled.gltf"));
        world.add_model(Model::new("C:\\Graphics\\assets\\sponzaGLTF\\sponza.gltf"));
        //world.add_model(Model::new("C:\\Graphics\\assets\\catTest\\catTest.gltf"));
        //world.add_model(Model::new("C:\\Graphics\\assets\\helmet\\DamagedHelmet.gltf"));
        //world.add_model(Model::new("C:\\Graphics\\assets\\hydrant\\untitled.gltf"));

        world.add_light(Light::new(Vector::new_vec3(-1.0, -5.0, -1.0)));

        world.initialize(base, MAX_FRAMES_IN_FLIGHT, true);

        // world.models[0].animations[0].repeat = true;
        // world.models[0].animations[0].start();



        let null_tex = base.create_2d_texture_image(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("local_assets\\null8x.png"), true);

        //<editor-fold desc = "geometry/shadow uniform buffers">
        let geometry_ubo = render::Descriptor::new_ubo(base, MAX_FRAMES_IN_FLIGHT, size_of::<UniformData>() as u64, 0u32, ShaderStageFlags::VERTEX);
        let shadow_ubo = render::Descriptor::new_ubo(base, MAX_FRAMES_IN_FLIGHT, size_of::<UniformData>() as u64, 0u32, ShaderStageFlags::VERTEX);
        //</editor-fold>
        //<editor-fold desc = "geometry/shadow descriptor pools">
        let geometry_descriptor_pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
                ..Default::default()
            }, // uniform buffer
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
                ..Default::default()
            }, // material SSBO
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
                ..Default::default()
            }, // joint SSBO
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: (MAX_FRAMES_IN_FLIGHT * 1024) as u32,
                ..Default::default()
            }, // array of textures
        ];
        let geometry_descriptor_pool_create_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            pool_size_count: geometry_descriptor_pool_sizes.len() as u32,
            p_pool_sizes: geometry_descriptor_pool_sizes.as_ptr(),
            max_sets: MAX_FRAMES_IN_FLIGHT as u32,
            flags: vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
            ..Default::default()
        };
        let geometry_descriptor_pool = base.device.create_descriptor_pool(&geometry_descriptor_pool_create_info, None).expect("failed to create descriptor pool");
        let shadow_descriptor_pool = base.device.create_descriptor_pool(&geometry_descriptor_pool_create_info, None).expect("failed to create descriptor pool");
        //</editor-fold>
        //<editor-fold desc = "geometry/shadow descriptor sets">
        let bindings = [
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 2,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 3,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1024u32,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];
        let binding_flags = [
            vk::DescriptorBindingFlags::empty(),
            vk::DescriptorBindingFlags::empty(),
            vk::DescriptorBindingFlags::empty(),
            vk::DescriptorBindingFlags::PARTIALLY_BOUND |
                vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT |
                vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
        ];
        let geometry_set_binding_flags_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_BINDING_FLAGS_CREATE_INFO,
            binding_count: binding_flags.len() as u32,
            p_binding_flags: binding_flags.as_ptr(),
            ..Default::default()
        };
        let descriptor_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            p_next: &geometry_set_binding_flags_info as *const _ as *const c_void,
            binding_count: bindings.len() as u32,
            p_bindings: bindings.as_ptr(),
            flags: vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
            ..Default::default()
        };
        let geometry_descriptor_set_layout = base.device.create_descriptor_set_layout(&descriptor_layout_create_info, None)?;
        let geometry_descriptor_set_layouts = vec![geometry_descriptor_set_layout; MAX_FRAMES_IN_FLIGHT];

        let variable_counts = vec![1024u32; MAX_FRAMES_IN_FLIGHT];
        let variable_count_info = vk::DescriptorSetVariableDescriptorCountAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_VARIABLE_DESCRIPTOR_COUNT_ALLOCATE_INFO,
            descriptor_set_count: MAX_FRAMES_IN_FLIGHT as u32,
            p_descriptor_counts: variable_counts.as_ptr(),
            ..Default::default()
        };
        let geometry_alloc_info = vk::DescriptorSetAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next: &variable_count_info as *const _ as *const c_void,
            descriptor_pool: geometry_descriptor_pool,
            descriptor_set_count: MAX_FRAMES_IN_FLIGHT as u32,
            p_set_layouts: geometry_descriptor_set_layouts.as_ptr(),
            ..Default::default()
        };
        let shadow_alloc_info = vk::DescriptorSetAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next: &variable_count_info as *const _ as *const c_void,
            descriptor_pool: shadow_descriptor_pool,
            descriptor_set_count: MAX_FRAMES_IN_FLIGHT as u32,
            p_set_layouts: geometry_descriptor_set_layouts.as_ptr(),
            ..Default::default()
        };
        let geometry_descriptor_sets = base.device.allocate_descriptor_sets(&geometry_alloc_info)
            .expect("Failed to allocate geometry descriptor sets");
        let shadow_descriptor_sets = base.device.allocate_descriptor_sets(&shadow_alloc_info)
            .expect("Failed to allocate shadow descriptor sets");

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
                        image_view: null_tex.0.0,
                        sampler: null_tex.0.1,
                        ..Default::default()
                    });
                }
            }
        }
        let missing = 1024 - image_infos.len();
        for _ in 0..missing {
            image_infos.push(vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image_view: null_tex.0.0,
                sampler: null_tex.0.1,
                ..Default::default()
            });
        }

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let geometry_uniform_buffer_info = vk::DescriptorBufferInfo {
                buffer: geometry_ubo.buffers.0[i],
                offset: 0,
                range: size_of::<UniformData>() as vk::DeviceSize,
            };
            let shadow_uniform_buffer_info = vk::DescriptorBufferInfo {
                buffer: shadow_ubo.buffers.0[i],
                offset: 0,
                range: size_of::<UniformData>() as vk::DeviceSize,
            };
            let material_ssbo_info = vk::DescriptorBufferInfo {
                buffer: world.material_buffers[i].0,
                offset: 0,
                range: vk::WHOLE_SIZE,
            };
            let joints_ssbo_info = vk::DescriptorBufferInfo {
                buffer: world.joints_buffers[i].0,
                offset: 0,
                range: vk::WHOLE_SIZE,
            };
            let descriptor_writes = [
                // geometry
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: geometry_descriptor_sets[i],
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: &geometry_uniform_buffer_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: geometry_descriptor_sets[i],
                    dst_binding: 1,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: &material_ssbo_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: geometry_descriptor_sets[i],
                    dst_binding: 2,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: &joints_ssbo_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: geometry_descriptor_sets[i],
                    dst_binding: 3,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1024,
                    p_image_info: image_infos.as_ptr(),
                    ..Default::default()
                },

                // shadow
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: shadow_descriptor_sets[i],
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: &shadow_uniform_buffer_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: shadow_descriptor_sets[i],
                    dst_binding: 1,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: &material_ssbo_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: shadow_descriptor_sets[i],
                    dst_binding: 2,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: &joints_ssbo_info,
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: shadow_descriptor_sets[i],
                    dst_binding: 3,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count: 1024,
                    p_image_info: image_infos.as_ptr(),
                    ..Default::default()
                },
            ];
            base.device.update_descriptor_sets(&descriptor_writes, &[]);
        }
        //</editor-fold>

        //<editor-fold desc = "lighting uniform buffers">
        let lighting_ubo = render::Descriptor::new_ubo(base, MAX_FRAMES_IN_FLIGHT, size_of::<UniformData>() as u64, 7u32, ShaderStageFlags::VERTEX);
        //</editor-fold>
        //<editor-fold desc = "lighting descriptor pools">
        let lighting_descriptor_pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: (MAX_FRAMES_IN_FLIGHT * 7) as u32, // position, normal, albedo, etc.
                ..Default::default()
            }, // images
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
                ..Default::default()
            }, // uniform buffer
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
                ..Default::default()
            }, // light space matrix ssbo
        ];
        let lighting_descriptor_pool_create_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            pool_size_count: lighting_descriptor_pool_sizes.len() as u32,
            p_pool_sizes: lighting_descriptor_pool_sizes.as_ptr(),
            max_sets: MAX_FRAMES_IN_FLIGHT as u32,
            ..Default::default()
        };
        let lighting_descriptor_pool = base.device
            .create_descriptor_pool(&lighting_descriptor_pool_create_info, None)
            .expect("Failed to create lighting descriptor pool");
        //</editor-fold>
        //<editor-fold desc = "lighting descriptor sets">
        let bindings = [
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
                _marker: Default::default(),
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
                _marker: Default::default(),
            },
            vk::DescriptorSetLayoutBinding {
                binding: 2,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
                _marker: Default::default(),
            },
            vk::DescriptorSetLayoutBinding {
                binding: 3,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
                _marker: Default::default(),
            },
            vk::DescriptorSetLayoutBinding {
                binding: 4,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
                _marker: Default::default(),
            },
            vk::DescriptorSetLayoutBinding {
                binding: 5,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
                _marker: Default::default(),
            },
            vk::DescriptorSetLayoutBinding {
                binding: 6,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
                _marker: Default::default(),
            },
            vk::DescriptorSetLayoutBinding {
                binding: 7,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 8,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];
        let descriptor_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            binding_count: bindings.len() as u32,
            p_bindings: bindings.as_ptr(),
            ..Default::default()
        };
        let lighting_descriptor_set_layout = base.device
            .create_descriptor_set_layout(&descriptor_layout_create_info, None)
            .expect("Failed to create lighting descriptor set layout");

        let lighting_descriptor_set_layouts = vec![lighting_descriptor_set_layout; MAX_FRAMES_IN_FLIGHT];
        let alloc_info = vk::DescriptorSetAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
            descriptor_pool: lighting_descriptor_pool,
            descriptor_set_count: MAX_FRAMES_IN_FLIGHT as u32,
            p_set_layouts: lighting_descriptor_set_layouts.as_ptr(),
            ..Default::default()
        };
        let lighting_descriptor_sets = base.device
            .allocate_descriptor_sets(&alloc_info)
            .expect("Failed to allocate lighting descriptor sets");
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let light_matrices_ssbo_info = vk::DescriptorBufferInfo {
                buffer: world.lights_buffers[i].0,
                offset: 0,
                range: vk::WHOLE_SIZE,
            };
            let descriptor_writes = [
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: lighting_descriptor_sets[i],
                    dst_binding: 7,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: &vk::DescriptorBufferInfo {
                        buffer: lighting_ubo.buffers.0[i],
                        offset: 0,
                        range: vk::WHOLE_SIZE
                    },
                    ..Default::default()
                },
                vk::WriteDescriptorSet {
                    s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                    dst_set: lighting_descriptor_sets[i],
                    dst_binding: 8,
                    dst_array_element: 0,
                    descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                    p_buffer_info: &light_matrices_ssbo_info,
                    ..Default::default()
                },
            ];
            base.device.update_descriptor_sets(&descriptor_writes, &[]);
        }
        //</editor-fold>

        // <editor-fold desc = "present descriptor pools">
        let present_descriptor_pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
                ..Default::default()
            },
        ];
        let present_descriptor_pool_create_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            pool_size_count: present_descriptor_pool_sizes.len() as u32,
            p_pool_sizes: present_descriptor_pool_sizes.as_ptr(),
            max_sets: MAX_FRAMES_IN_FLIGHT as u32,
            ..Default::default()
        };
        let bindings = [
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
                _marker: Default::default(),
            },
        ];
        let descriptor_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            binding_count: bindings.len() as u32,
            p_bindings: bindings.as_ptr(),
            ..Default::default()
        };
        let present_descriptor_set_layout = base.device
            .create_descriptor_set_layout(&descriptor_layout_create_info, None)
            .expect("Failed to create lighting descriptor set layout");
        let present_descriptor_pool = base.device
            .create_descriptor_pool(&present_descriptor_pool_create_info, None)
            .expect("Failed to create lighting descriptor pool");
        //</editor-fold>
        //<editor-fold desc = "present descriptor sets">
        let present_descriptor_set_layouts = vec![present_descriptor_set_layout; MAX_FRAMES_IN_FLIGHT];
        let alloc_info = vk::DescriptorSetAllocateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
            descriptor_pool: present_descriptor_pool,
            descriptor_set_count: MAX_FRAMES_IN_FLIGHT as u32,
            p_set_layouts: present_descriptor_set_layouts.as_ptr(),
            ..Default::default()
        };
        let present_descriptor_sets = base.device
            .allocate_descriptor_sets(&alloc_info)
            .expect("Failed to allocate lighting descriptor sets");
        //</editor-fold>

        //<editor-fold desc = "passes">
        let color_tex_create_info = TextureCreateInfo::new(base).format(Format::R16G16B16A16_SFLOAT);
        let geometry_pass_create_info = PassCreateInfo::new(base)
            .frames_in_flight(MAX_FRAMES_IN_FLIGHT)
            .add_color_attachment_info(TextureCreateInfo::new(base).format(Format::R8G8B8A8_SINT)) // material
            .add_color_attachment_info(color_tex_create_info) // albedo
            .add_color_attachment_info(color_tex_create_info) // metallic roughness
            .add_color_attachment_info(color_tex_create_info) // extra properties
            .add_color_attachment_info(color_tex_create_info) // view normal
            .depth_attachment_info(TextureCreateInfo::new(base).format(Format::D32_SFLOAT).is_depth(true).clear_value([0.0, 0.0, 0.0, 0.0])); // depth
        let geometry_pass = Pass::new(base, geometry_pass_create_info);

        let shadow_pass_create_info = PassCreateInfo::new(base)
            .frames_in_flight(MAX_FRAMES_IN_FLIGHT)
            .depth_attachment_info(TextureCreateInfo::new(base).format(Format::D16_UNORM).is_depth(true).clear_value([0.0, 0.0, 0.0, 0.0])); // depth
        let shadow_pass = Pass::new(base, shadow_pass_create_info);

        let lighting_pass_create_info = PassCreateInfo::new(base)
            .frames_in_flight(MAX_FRAMES_IN_FLIGHT)
            .add_color_attachment_info(color_tex_create_info)
            .depth_attachment_info(TextureCreateInfo::new(base).format(Format::D16_UNORM).is_depth(true).clear_value([1.0, 0.0, 0.0, 0.0])); // depth
        let lighting_pass = Pass::new(base, lighting_pass_create_info);

        let present_pass_create_info = PassCreateInfo::new(base)
            .set_is_present_pass(true);
        let present_pass = Pass::new(base, present_pass_create_info);
        //</editor-fold>

        //<editor-fold desc = "shaders">
        let geom_shader = Shader::new(base, "geometry\\geometry.vert.spv", "geometry\\geometry.frag.spv");
        let shadow_shader = Shader::new(base, "shadow\\shadow.vert.spv", "shadow\\shadow.frag.spv");
        let lighting_shader = Shader::new(base, "quad\\quad.vert.spv", "lighting\\lighting.frag.spv");
        let present_shader = Shader::new(base, "quad\\quad.vert.spv", "quad\\quad.frag.spv");
        //</editor-fold>

        //<editor-fold desc = "full graphics pipeline initiation">

        // reused for shadow pass
        let geometry_pipeline_layout = base
            .device
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo {
                    s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                    set_layout_count: 1,
                    p_set_layouts: &geometry_descriptor_set_layout,
                    ..Default::default()
                }, None
            ).unwrap();
        let lighting_pipeline_layout = base
            .device
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo {
                    s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                    set_layout_count: 1,
                    p_set_layouts: lighting_descriptor_set_layouts.as_ptr(),
                    ..Default::default()
                }, None
            ).unwrap();
        let present_pipeline_layout = base
            .device
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo {
                    s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                    set_layout_count: 1,
                    p_set_layouts: present_descriptor_set_layouts.as_ptr(),
                    ..Default::default()
                }, None
            ).unwrap();

        let vertex_input_binding_descriptions = [
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: size_of::<scene::Vertex>() as u32,
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
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(scene::Vertex, position) as u32,
            }, // position
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(scene::Vertex, normal) as u32,
            }, // normal
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(scene::Vertex, uv) as u32,
            }, // uv
            vk::VertexInputAttributeDescription {
                location: 3,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(scene::Vertex, tangent) as u32,
            }, // tangent
            vk::VertexInputAttributeDescription {
                location: 4,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(scene::Vertex, bitangent) as u32,
            }, // bitangent
            vk::VertexInputAttributeDescription {
                location: 5,
                binding: 0,
                format: vk::Format::R32G32B32A32_UINT,
                offset: offset_of!(scene::Vertex, joint_indices) as u32,
            }, // joint indices
            vk::VertexInputAttributeDescription {
                location: 6,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(scene::Vertex, joint_weights) as u32,
            }, // join weights

            // instance
            vk::VertexInputAttributeDescription {
                location: 7,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 0,
            }, // model matrix
            vk::VertexInputAttributeDescription {
                location: 8,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 16,
            }, // |
            vk::VertexInputAttributeDescription {
                location: 9,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 32,
            }, // |
            vk::VertexInputAttributeDescription {
                location: 10,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 48,
            }, // |
            vk::VertexInputAttributeDescription {
                location: 11,
                binding: 1,
                format: vk::Format::R32G32_SINT,
                offset: offset_of!(Instance, indices) as u32,
            }, // indices (material + skin)
        ];
        let shadow_vertex_input_attribute_descriptions = [
            // vertex
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(scene::Vertex, position) as u32,
            }, // position
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(scene::Vertex, uv) as u32,
            }, // uv
            vk::VertexInputAttributeDescription {
                location: 5,
                binding: 0,
                format: vk::Format::R32G32B32A32_UINT,
                offset: offset_of!(scene::Vertex, joint_indices) as u32,
            }, // joint indices
            vk::VertexInputAttributeDescription {
                location: 6,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(scene::Vertex, joint_weights) as u32,
            }, // joint weights

            // instance
            vk::VertexInputAttributeDescription {
                location: 7,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 0,
            }, // model matrix
            vk::VertexInputAttributeDescription {
                location: 8,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 16,
            }, // |
            vk::VertexInputAttributeDescription {
                location: 9,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 32,
            }, // |
            vk::VertexInputAttributeDescription {
                location: 10,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 48,
            }, // |
            vk::VertexInputAttributeDescription {
                location: 11,
                binding: 1,
                format: vk::Format::R32G32_SINT,
                offset: offset_of!(Instance, indices) as u32,
            }, // indices (material + skin)
        ];

        let geometry_vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&geometry_vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);
        let shadow_vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&shadow_vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);
        let null_vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default();

        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: base.surface_resolution.width as f32,
            height: base.surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        let scissors = [base.surface_resolution.into()];

        let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
            .scissors(&scissors)
            .viewports(&viewports);

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
        let fullscreen_quad_rasterization_info = vk::PipelineRasterizationStateCreateInfo {
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            cull_mode: vk::CullModeFlags::NONE,
            line_width: 1.0,
            polygon_mode: vk::PolygonMode::FILL,
            ..Default::default()
        };

        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: base.msaa_samples,
            ..Default::default()
        };
        let null_multisample_state_info = vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
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
        let default_depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: 1,
            depth_write_enable: 1,
            depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
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
        let null_blend_states_singular = [null_blend_attachment];
        let null_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(&null_blend_states);
        let null_blend_state_singular = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(&null_blend_states_singular);

        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);

        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_state);

        let geom_shader_create_info = geom_shader.generate_shader_stage_create_infos();
        let shadow_shader_create_info = shadow_shader.generate_shader_stage_create_infos();
        let lighting_shader_create_info = lighting_shader.generate_shader_stage_create_infos();
        let present_shader_create_info = present_shader.generate_shader_stage_create_infos();

        let base_pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .dynamic_state(&dynamic_state_info);
        let geometry_pipeline_info = base_pipeline_info
            .stages(&geom_shader_create_info)
            .vertex_input_state(&geometry_vertex_input_state_info)
            .multisample_state(&null_multisample_state_info)
            .render_pass(geometry_pass.renderpass)
            .color_blend_state(&null_blend_state)
            .layout(geometry_pipeline_layout)
            .depth_stencil_state(&infinite_reverse_depth_state_info);
        let shadow_pipeline_info = base_pipeline_info
            .stages(&shadow_shader_create_info)
            .vertex_input_state(&shadow_vertex_input_state_info)
            .multisample_state(&null_multisample_state_info)
            .render_pass(shadow_pass.renderpass)
            .color_blend_state(&null_blend_state)
            .rasterization_state(&shadow_rasterization_info)
            .depth_stencil_state(&shadow_depth_state_info)
            .layout(geometry_pipeline_layout);
        let lighting_pipeline_info = base_pipeline_info
            .stages(&lighting_shader_create_info)
            .vertex_input_state(&null_vertex_input_state_info)
            .multisample_state(&null_multisample_state_info)
            .render_pass(lighting_pass.renderpass)
            .color_blend_state(&null_blend_state_singular)
            .layout(lighting_pipeline_layout)
            .rasterization_state(&fullscreen_quad_rasterization_info)
            .depth_stencil_state(&default_depth_state_info);
        let present_pipeline_info = base_pipeline_info
            .stages(&present_shader_create_info)
            .vertex_input_state(&null_vertex_input_state_info)
            .multisample_state(&multisample_state_info)
            .render_pass(present_pass.renderpass)
            .color_blend_state(&color_blend_state)
            .layout(present_pipeline_layout)
            .rasterization_state(&fullscreen_quad_rasterization_info)
            .depth_stencil_state(&default_depth_state_info);

        let graphics_pipelines = base
            .device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[geometry_pipeline_info, shadow_pipeline_info, lighting_pipeline_info, present_pipeline_info], None)
            .expect("Unable to create graphics pipeline");

        let geometry_pipeline = graphics_pipelines[0];
        let shadow_pipeline = graphics_pipelines[1];
        let lighting_pipeline = graphics_pipelines[2];
        let present_pipeline = graphics_pipelines[3];
        //</editor-fold>

        let sampler = base.device.create_sampler(&vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            ..Default::default()
        }, None)?;

        let mut player_camera = Camera::new_perspective_rotation(
            Vector::new_vec3(0.0, 0.0, 0.0),
            Vector::new_empty(),
            1.0,
            0.001,
            100.0,
            base.window.inner_size().width as f32 / base.window.inner_size().height as f32,
            0.01,
            1000.0,
            true,
        );

        let mut current_frame = 0usize;
        let mut pressed_keys = HashSet::new();
        let mut new_pressed_keys = HashSet::new();
        let mut mouse_delta = (0.0, 0.0);
        let mut last_frame_time = Instant::now();
        let mut cursor_locked = false;
        let mut saved_cursor_pos = PhysicalPosition::new(0.0, 0.0);
        let mut needs_resize = false;

        let mut pause_frustum = false;
        base.window.set_cursor_position(PhysicalPosition::new(
            base.window.inner_size().width as f32 * 0.5,
            base.window.inner_size().height as f32 * 0.5))
            .expect("failed to reset mouse position");

        base.event_loop.borrow_mut().run_on_demand(|event, elwp| {
            elwp.set_control_flow(ControlFlow::Poll);
            match event {
                //<editor-fold desc = "event handling">
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    elwp.exit();
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized( _ ), // _ = new_size
                    ..
                } => {
                    println!("bruh");
                    player_camera.aspect_ratio = base.window.inner_size().width as f32 / base.window.inner_size().height as f32;
                    needs_resize = true;
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput {
                        event: KeyEvent {
                            state,
                            physical_key,
                            ..
                        },
                        ..
                    },
                    ..
                } => {
                    match state {
                        ElementState::Pressed => {
                            if !pressed_keys.contains(&physical_key) { new_pressed_keys.insert(physical_key.clone()); }
                            pressed_keys.insert(physical_key.clone());
                        }
                        ElementState::Released => {
                            pressed_keys.remove(&physical_key);
                            new_pressed_keys.remove(&physical_key);
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    ..
                } => {
                    if base.window.has_focus() && cursor_locked {
                        mouse_delta = (
                            -position.x as f32 + 0.5 * base.window.inner_size().width as f32,
                            position.y as f32 - 0.5 * base.window.inner_size().height as f32,
                        );
                        base.window.set_cursor_position(PhysicalPosition::new(
                            base.window.inner_size().width as f32 * 0.5,
                            base.window.inner_size().height as f32 * 0.5))
                            .expect("failed to reset mouse position");
                        do_mouse(&mut player_camera, mouse_delta, &mut cursor_locked);
                    } else {
                        saved_cursor_pos = position;
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Focused(true),
                    ..
                } => {
                    if !cursor_locked {
                        if let Err(err) = base.window.set_cursor_grab(CursorGrabMode::Confined) {
                            eprintln!("Cursor lock failed: {:?}", err);
                        } else {
                            base.window.set_cursor_visible(false);
                            cursor_locked = true;
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Focused(false),
                    ..
                } => {
                    cursor_locked = false;
                    if let Err(err) = base.window.set_cursor_grab(CursorGrabMode::None) {
                        eprintln!("Cursor unlock failed: {:?}", err);
                    } else {
                        base.window.set_cursor_visible(true);
                    }
                    base.window.set_cursor_position(saved_cursor_pos).expect("Cursor pos reset failed");
                }
                //</editor-fold>
                Event::AboutToWait => {
                    let now = Instant::now();
                    let delta_time = now.duration_since(last_frame_time).as_secs_f32();
                    last_frame_time = now;
                    if needs_resize {

                    }
                    do_controls(&mut player_camera, &pressed_keys, &mut new_pressed_keys, delta_time, &mut cursor_locked, base, &mut saved_cursor_pos, &mut pause_frustum);
                    player_camera.update_matrices();

                    world.update_nodes(base, current_frame);

                    if !pause_frustum {
                        player_camera.update_frustum()
                    }

                    let current_fence = base.draw_commands_reuse_fences[current_frame];
                    base.device.wait_for_fences(&[current_fence], true, u64::MAX).expect("wait failed");
                    base.device.reset_fences(&[current_fence]).expect("reset failed");

                    let (present_index, _) = base
                        .swapchain_loader
                        .acquire_next_image(
                            base.swapchain,
                            u64::MAX,
                            base.present_complete_semaphores[current_frame],
                            vk::Fence::null(),
                        )
                        .unwrap();

                    let image_infos = [
                        vk::DescriptorImageInfo {
                            sampler,
                            image_view: geometry_pass.textures[current_frame][0].image_view,
                            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        }, // material
                        vk::DescriptorImageInfo {
                            sampler,
                            image_view: geometry_pass.textures[current_frame][1].image_view,
                            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        }, // albedo
                        vk::DescriptorImageInfo {
                            sampler,
                            image_view: geometry_pass.textures[current_frame][2].image_view,
                            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        }, // metallic roughness
                        vk::DescriptorImageInfo {
                            sampler,
                            image_view: geometry_pass.textures[current_frame][3].image_view,
                            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        }, // extra properties
                        vk::DescriptorImageInfo {
                            sampler,
                            image_view: geometry_pass.textures[current_frame][5].image_view,
                            image_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                        }, // depth
                        vk::DescriptorImageInfo {
                            sampler,
                            image_view: geometry_pass.textures[current_frame][4].image_view,
                            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        }, // view normal
                        vk::DescriptorImageInfo {
                            sampler,
                            image_view: shadow_pass.textures[current_frame][0].image_view,
                            image_layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
                        }, // shadow map
                    ];
                    let lighting_descriptor_writes: Vec<vk::WriteDescriptorSet> = image_infos.iter().enumerate().map(|(i, info)| {
                        vk::WriteDescriptorSet::default()
                            .dst_set(lighting_descriptor_sets[current_frame])
                            .dst_binding((i) as u32)
                            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                            .image_info(std::slice::from_ref(info))
                    }).collect();
                    base.device.update_descriptor_sets(&lighting_descriptor_writes, &[]);

                    let info = [vk::DescriptorImageInfo {
                        sampler,
                        image_view: lighting_pass.textures[current_frame][0].image_view,
                        image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    }];
                    let present_descriptor_writes: Vec<vk::WriteDescriptorSet> = vec![
                        vk::WriteDescriptorSet::default()
                            .dst_set(present_descriptor_sets[current_frame])
                            .dst_binding(0)
                            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                            .image_info(&info)];
                    base.device.update_descriptor_sets(&present_descriptor_writes, &[]);

                    let ubo = UniformData {
                        view: player_camera.view_matrix.data,
                        projection: player_camera.projection_matrix.data,
                    };
                    copy_data_to_memory(geometry_ubo.buffers.2[current_frame], &[ubo]);
                    copy_data_to_memory(lighting_ubo.buffers.2[current_frame], &[ubo]);
                    let ubo = UniformData {
                        view: world.lights[0].view.data,
                        projection: world.lights[0].projection.data,
                    };
                    copy_data_to_memory(shadow_ubo.buffers.2[current_frame], &[ubo]);

                    let geometry_pass_pass_begin_info = vk::RenderPassBeginInfo::default()
                        .render_pass(geometry_pass.renderpass)
                        .framebuffer(geometry_pass.framebuffers[current_frame])
                        .render_area(base.surface_resolution.into())
                        .clear_values(&geometry_pass.clear_values);
                    let shadow_pass_pass_begin_info = vk::RenderPassBeginInfo::default()
                        .render_pass(shadow_pass.renderpass)
                        .framebuffer(shadow_pass.framebuffers[current_frame])
                        .render_area(base.surface_resolution.into())
                        .clear_values(&shadow_pass.clear_values);
                    let lighting_pass_pass_begin_info = vk::RenderPassBeginInfo::default()
                        .render_pass(lighting_pass.renderpass)
                        .framebuffer(lighting_pass.framebuffers[current_frame])
                        .render_area(base.surface_resolution.into())
                        .clear_values(&lighting_pass.clear_values);
                    let present_pass_pass_begin_info = vk::RenderPassBeginInfo::default()
                        .render_pass(present_pass.renderpass)
                        .framebuffer(present_pass.framebuffers[present_index as usize])
                        .render_area(base.surface_resolution.into())
                        .clear_values(&present_pass.clear_values);
                    let current_rendering_complete_semaphore = base.rendering_complete_semaphores[current_frame];
                    let current_draw_command_buffer = base.draw_command_buffers[current_frame];
                    record_submit_commandbuffer(
                        &base.device,
                        current_draw_command_buffer,
                        current_fence,
                        base.present_queue,
                        &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
                        &[base.present_complete_semaphores[current_frame]],
                        &[current_rendering_complete_semaphore],
                        |device, frame_command_buffer| {
                            //<editor-fold desc = "geometry pass">
                            device.cmd_begin_render_pass(
                                frame_command_buffer,
                                &geometry_pass_pass_begin_info,
                                vk::SubpassContents::INLINE,
                            );
                            device.cmd_bind_pipeline(
                                frame_command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                geometry_pipeline,
                            );

                            // draw scene
                            device.cmd_set_viewport(frame_command_buffer, 0, &viewports);
                            device.cmd_set_scissor(frame_command_buffer, 0, &scissors);
                            device.cmd_bind_descriptor_sets(
                                frame_command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                geometry_pipeline_layout,
                                0,
                                &[geometry_descriptor_sets[current_frame]],
                                &[],
                            );
                            world.draw(base, &frame_command_buffer, current_frame, Some(&player_camera.frustum));

                            device.cmd_end_render_pass(frame_command_buffer);
                            //</editor-fold>
                            geometry_pass.transition_to_readable(base, frame_command_buffer, current_frame);
                            //<editor-fold desc = "shadow pass">
                            device.cmd_begin_render_pass(
                                frame_command_buffer,
                                &shadow_pass_pass_begin_info,
                                vk::SubpassContents::INLINE,
                            );
                            device.cmd_bind_pipeline(
                                frame_command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                shadow_pipeline,
                            );

                            // draw scene
                            device.cmd_set_viewport(frame_command_buffer, 0, &viewports);
                            device.cmd_set_scissor(frame_command_buffer, 0, &scissors);
                            device.cmd_bind_descriptor_sets(
                                frame_command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                geometry_pipeline_layout,
                                0,
                                &[shadow_descriptor_sets[current_frame]],
                                &[],
                            );
                            world.draw(base, &frame_command_buffer, current_frame, None);

                            device.cmd_end_render_pass(frame_command_buffer);
                            //</editor-fold>
                            shadow_pass.transition_to_readable(base, frame_command_buffer, current_frame);
                            //<editor-fold desc = "lighting pass">
                            device.cmd_begin_render_pass(
                                frame_command_buffer,
                                &lighting_pass_pass_begin_info,
                                vk::SubpassContents::INLINE,
                            );
                            device.cmd_bind_pipeline(
                                frame_command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                lighting_pipeline,
                            );

                            // draw quad
                            device.cmd_set_viewport(frame_command_buffer, 0, &viewports);
                            device.cmd_set_scissor(frame_command_buffer, 0, &scissors);
                            device.cmd_bind_descriptor_sets(
                                frame_command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                lighting_pipeline_layout,
                                0,
                                &[lighting_descriptor_sets[current_frame]],
                                &[],
                            );
                            device.cmd_draw(current_draw_command_buffer, 6, 1, 0, 0);

                            device.cmd_end_render_pass(frame_command_buffer);
                            //</editor-fold>
                            lighting_pass.transition_to_readable(base, frame_command_buffer, current_frame);
                            // <editor-fold desc = "present pass">
                            device.cmd_begin_render_pass(
                                frame_command_buffer,
                                &present_pass_pass_begin_info,
                                vk::SubpassContents::INLINE,
                            );
                            device.cmd_bind_pipeline(
                                frame_command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                present_pipeline,
                            );

                            // draw quad
                            device.cmd_set_viewport(frame_command_buffer, 0, &viewports);
                            device.cmd_set_scissor(frame_command_buffer, 0, &scissors);
                            device.cmd_bind_descriptor_sets(
                                frame_command_buffer,
                                vk::PipelineBindPoint::GRAPHICS,
                                present_pipeline_layout,
                                0,
                                &[present_descriptor_sets[current_frame]],
                                &[],
                            );
                            device.cmd_draw(current_draw_command_buffer, 6, 1, 0, 0);

                            device.cmd_end_render_pass(frame_command_buffer);
                            //</editor-fold>
                        },
                    );
                    let wait_semaphores = [current_rendering_complete_semaphore];
                    let swapchains = [base.swapchain];
                    let image_indices = [present_index];
                    let present_info = vk::PresentInfoKHR::default()
                        .wait_semaphores(&wait_semaphores)
                        .swapchains(&swapchains)
                        .image_indices(&image_indices);

                    base.swapchain_loader
                        .queue_present(base.present_queue, &present_info)
                        .unwrap();
                    current_frame = (current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
                },
                _ => (),
            }
        }).expect("Failed to initiate render loop");


        println!("Render loop exited successfully, cleaning up");
        //<editor-fold desc = "cleanup">
        base.device.device_wait_idle().unwrap();

        for pipeline in graphics_pipelines {
            base.device.destroy_pipeline(pipeline, None);
        }
        base.device.destroy_pipeline_layout(geometry_pipeline_layout, None);
        base.device.destroy_pipeline_layout(lighting_pipeline_layout, None);
        base.device.destroy_pipeline_layout(present_pipeline_layout, None);

        geom_shader.destroy(base);
        shadow_shader.destroy(base);
        lighting_shader.destroy(base);
        present_shader.destroy(base);
        world.destroy(base);

        geometry_pass.destroy(base);
        shadow_pass.destroy(base);
        lighting_pass.destroy(base);
        present_pass.destroy(base);

        lighting_ubo.destroy(base);
        geometry_ubo.destroy(base);
        shadow_ubo.destroy(base);

        base.device.destroy_descriptor_set_layout(geometry_descriptor_set_layout, None);
        base.device.destroy_descriptor_pool(geometry_descriptor_pool, None);
        base.device.destroy_descriptor_pool(shadow_descriptor_pool, None);
        base.device.destroy_descriptor_set_layout(lighting_descriptor_set_layout, None);
        base.device.destroy_descriptor_pool(lighting_descriptor_pool, None);
        base.device.destroy_descriptor_set_layout(present_descriptor_set_layout, None);
        base.device.destroy_descriptor_pool(present_descriptor_pool, None);

        base.device.destroy_sampler(sampler, None);

        base.device.destroy_image_view(null_tex.0.0, None);
        base.device.destroy_sampler(null_tex.0.1, None);
        base.device.destroy_image(null_tex.1.0, None);
        base.device.free_memory(null_tex.1.1, None);
        //</editor-fold>
    }
    Ok(())
}

fn do_controls(
    player_camera: &mut Camera,
    pressed_keys: &HashSet<PhysicalKey>,
    new_pressed_keys: &mut HashSet<PhysicalKey>,
    delta_time: f32,
    cursor_locked: &mut bool,
    base: &VkBase,
    saved_cursor_pos: &mut PhysicalPosition<f64>,
    paused: &mut bool,
) {
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyW)) {
        player_camera.position.x += player_camera.speed*delta_time * (player_camera.rotation.y + PI/2.0).cos();
        player_camera.position.z += player_camera.speed*delta_time * (player_camera.rotation.y + PI/2.0).sin();
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyA)) {
        player_camera.position.x -= player_camera.speed*delta_time * player_camera.rotation.y.cos();
        player_camera.position.z -= player_camera.speed*delta_time * player_camera.rotation.y.sin();
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyS)) {
        player_camera.position.x -= player_camera.speed*delta_time * (player_camera.rotation.y + PI/2.0).cos();
        player_camera.position.z -= player_camera.speed*delta_time * (player_camera.rotation.y + PI/2.0).sin();
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyD)) {
        player_camera.position.x += player_camera.speed*delta_time * player_camera.rotation.y.cos();
        player_camera.position.z += player_camera.speed*delta_time * player_camera.rotation.y.sin();
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::Space)) {
        player_camera.position.y += player_camera.speed*delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::ShiftLeft)) {
        player_camera.position.y -= player_camera.speed*delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::ArrowUp)) {
        player_camera.rotation.x += delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::ArrowDown)) {
        player_camera.rotation.x -= delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::ArrowLeft)) {
        player_camera.rotation.y += delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::ArrowRight)) {
        player_camera.rotation.y -= delta_time;
    }

    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::Equal)) {
        player_camera.speed *= 1.0 + 1.0*delta_time;
    }
    if pressed_keys.contains(&PhysicalKey::Code(KeyCode::Minus)) {
        player_camera.speed /= 1.0 + 1.0*delta_time;
    }

    if new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::Escape)) {
        *cursor_locked = !*cursor_locked;
        if *cursor_locked {
            if let Err(err) = base.window.set_cursor_grab(CursorGrabMode::Confined) {
                eprintln!("Cursor lock failed: {:?}", err);
            } else {
                base.window.set_cursor_visible(false);
            }
            base.window.set_cursor_position(PhysicalPosition::new(
                base.window.inner_size().width as f32 * 0.5,
                base.window.inner_size().height as f32 * 0.5))
                .expect("failed to reset mouse position");
        } else {
            if let Err(err) = base.window.set_cursor_grab(CursorGrabMode::None) {
                eprintln!("Cursor unlock failed: {:?}", err);
            } else {
                base.window.set_cursor_visible(true);
            }
            base.window.set_cursor_position(*saved_cursor_pos).expect("Cursor pos reset failed");
        }
    }
    if new_pressed_keys.contains(&PhysicalKey::Code(KeyCode::KeyP)) {
        *paused = !*paused
    }

    //player_camera.position.println();

    new_pressed_keys.clear();
}
fn do_mouse(player_camera: &mut Camera, mouse_delta: (f32, f32), cursor_locked: &mut bool) {
    if *cursor_locked {
        player_camera.rotation.y += player_camera.sensitivity * mouse_delta.0;
        player_camera.rotation.x += player_camera.sensitivity * mouse_delta.1;
        player_camera.rotation.x = player_camera.rotation.x.clamp(-PI * 0.5, PI * 0.5);
    }
}