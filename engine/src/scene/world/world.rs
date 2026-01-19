use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::sync::Arc;
use std::time::SystemTime;
use ash::vk;
use ash::vk::{CommandBuffer, DeviceMemory, ImageView, Sampler};
use json::JsonValue;
use crate::math::matrix::Matrix;
use crate::math::Vector;
use crate::render::render::MAX_FRAMES_IN_FLIGHT;
use crate::render::vulkan_base::{copy_buffer_synchronous, copy_data_to_memory, Context, VkBase};
use crate::scene::scene::{Instance, Scene};

// SHOULD DETECT MATH VS COLOR DATA TEXTURES, LOAD COLOR AS SRGB, MATH AS UNORM
const MAX_VERTICES: u64 = 3 * 10u64.pow(6); // 7 for bistro
const MAX_INDICES: u64 = 4 * 10u64.pow(5); // 6 for bistro
pub const MAX_INSTANCES: u64 = 10u64 * 10u64.pow(4); // 5 for bistro
const MAX_MATERIALS: u64 = 10u64 * 10u64.pow(4);
const MAX_JOINTS: u64 = 10u64 * 10u64.pow(4);
const MAX_LIGHTS: u64 = 10u64 * 10u64.pow(3);

pub struct World {
    context: Arc<Context>,

    pub loaded_files: HashMap<String, usize>,

    pub models: Vec<ModelContainer>,
    buffers_need_update: bool,
    new_vertices: Vec<Vertex>,
    new_indices: Vec<u32>,
    new_joints: Vec<Matrix>,
    new_materials: Vec<MaterialSendable>,

    pub nodes: Vec<Node>,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub textures: Vec<SceneTexture>,
    pub images: Vec<Image>,
    pub skins: Vec<Skin>,
    pub animations: Vec<Animation>,
    pub accessors: Vec<Accessor>,
    pub buffer_views: Vec<BufferView>,
    pub buffers: Vec<Buffer>,
    pub scenes: Vec<GltfScene>,

    pub texture_count: i32,

    pub index_buffer: (vk::Buffer, DeviceMemory),
    pub index_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
    pub vertex_buffer: (vk::Buffer, DeviceMemory),
    pub vertex_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
    pub indices_count: usize,
    buffer_indices_count: usize,
    indices_buffer_size: u64,
    pub vertices_count: usize,
    buffer_vertices_count: usize,
    vertex_buffer_size: u64,

    pub instance_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
    pub instance_buffers: Vec<(vk::Buffer, DeviceMemory)>,
    pub instance_buffer_size: u64,

    pub material_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
    pub material_buffers: Vec<(vk::Buffer, DeviceMemory)>,
    pub materials_count: usize,
    buffer_materials_count: usize,
    pub material_buffer_size: u64,

    pub joints_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
    pub joints_buffers: Vec<(vk::Buffer, DeviceMemory)>,
    pub joints_count: usize,
    buffer_joints_count: usize,
    pub joints_buffers_size: u64,

    pub lights_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
    pub lights_buffers: Vec<(vk::Buffer, DeviceMemory)>,
    pub lights_buffers_size: u64,
    pub lights_count: usize,

    pub primitive_count: usize,
}
impl World {
    pub fn new(context: &Arc<Context>) -> Self {
        Self {
            context: context.clone(),

            buffers_need_update: false,

            loaded_files: HashMap::new(),

            models: Vec::new(),
            new_indices: Vec::new(),
            new_vertices: Vec::new(),
            new_joints: Vec::new(),
            new_materials: Vec::new(),

            nodes: Vec::new(),
            meshes: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            images: Vec::new(),
            skins: Vec::new(),
            animations: Vec::new(),
            accessors: Vec::new(),
            buffer_views: Vec::new(),
            buffers: Vec::new(),
            scenes: Vec::new(),

            texture_count: 0,
            index_buffer: (vk::Buffer::null(), DeviceMemory::null()),
            index_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
            vertex_buffer: (vk::Buffer::null(), DeviceMemory::null()),
            vertex_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
            indices_count: 0,
            vertices_count: 0,
            buffer_indices_count: 0,
            buffer_vertices_count: 0,
            indices_buffer_size: 0,
            vertex_buffer_size: 0,
            instance_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
            instance_buffers: Vec::new(),
            instance_buffer_size: 0,
            material_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
            material_buffers: Vec::new(),
            materials_count: 0,
            buffer_materials_count: 0,
            material_buffer_size: 0,
            joints_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
            joints_buffers: Vec::new(),
            joints_count: 0,
            buffer_joints_count: 0,
            joints_buffers_size: 0,
            lights_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
            lights_buffers: Vec::new(),
            lights_buffers_size: 0,
            lights_count: 0,
            primitive_count: 0,
        }
    }
    pub unsafe fn initialize(&mut self) { unsafe {
        self.instance_buffer_size = MAX_INSTANCES * size_of::<Instance>() as u64;
        self.material_buffer_size = MAX_MATERIALS * size_of::<MaterialSendable>() as u64;
        self.lights_buffers_size = MAX_LIGHTS * size_of::<LightSendable>() as u64;
        self.indices_buffer_size = 3 * MAX_INDICES * size_of::<u32>() as u64;
        self.vertex_buffer_size = MAX_VERTICES * size_of::<Vertex>() as u64;
        (self.vertex_buffer, self.vertex_staging_buffer) = self.context.create_device_and_staging_buffer(self.vertex_buffer_size, &[0], vk::BufferUsageFlags::VERTEX_BUFFER, false, true, false);
        (self.index_buffer, self.index_staging_buffer) = self.context.create_device_and_staging_buffer(self.indices_buffer_size, &[0], vk::BufferUsageFlags::INDEX_BUFFER, false, true, false);
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            self.instance_buffers.push((vk::Buffer::null(), DeviceMemory::null()));
            self.material_buffers.push((vk::Buffer::null(), DeviceMemory::null()));
            self.lights_buffers.push((vk::Buffer::null(), DeviceMemory::null()));
            self.joints_buffers.push((vk::Buffer::null(), DeviceMemory::null()));
            if i == 0 {
                (self.instance_buffers[i], self.instance_staging_buffer) =
                    self.context.create_device_and_staging_buffer(self.instance_buffer_size, &[0], vk::BufferUsageFlags::VERTEX_BUFFER, false, true, false);
                (self.material_buffers[i], self.material_staging_buffer) =
                    self.context.create_device_and_staging_buffer(self.material_buffer_size, &[0], vk::BufferUsageFlags::STORAGE_BUFFER, false, true, false);
                (self.lights_buffers[i], self.lights_staging_buffer) =
                    self.context.create_device_and_staging_buffer(self.lights_buffers_size, &[0], vk::BufferUsageFlags::STORAGE_BUFFER, false, true, false);
            } else {
                self.instance_buffers[i] = self.context.create_device_and_staging_buffer(self.instance_buffer_size, &[0], vk::BufferUsageFlags::VERTEX_BUFFER, true, false, false).0;
                self.material_buffers[i] = self.context.create_device_and_staging_buffer(self.material_buffer_size, &[0], vk::BufferUsageFlags::STORAGE_BUFFER, true, false, false).0;
                self.lights_buffers[i] = self.context.create_device_and_staging_buffer(self.lights_buffers_size, &[0], vk::BufferUsageFlags::STORAGE_BUFFER, true, false, false).0;
            }
        }
        self.joints_buffers_size = MAX_JOINTS * size_of::<Matrix>() as u64;
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            if i == 0 {
                (self.joints_buffers[i], self.joints_staging_buffer) =
                    self.context.create_device_and_staging_buffer(self.joints_buffers_size, &[0], vk::BufferUsageFlags::STORAGE_BUFFER, false, true, false);
            } else {
                self.joints_buffers[i] = self.context.create_device_and_staging_buffer(self.joints_buffers_size, &[0], vk::BufferUsageFlags::STORAGE_BUFFER, true, true, false).0;
            }
        }
    } }

    pub unsafe fn add_model(&mut self, uri: &str) -> usize {
        if !self.loaded_files.contains_key(&String::from(uri)) {
            self.loaded_files.insert(String::from(uri), self.models.len());
            let model = ModelContainer::new(uri, self);
            let mut new_vertices: Vec<Vertex> = vec![];
            let mut new_indices: Vec<u32> = vec![];
            let mut new_materials_send: Vec<MaterialSendable> = vec![];
            for mesh in &model.meshes {
                for primitive in &mut self.meshes[*mesh].primitives {
                    primitive.construct_data(&self.accessors, &self.buffer_views, &self.buffers);
                    primitive.vertex_buffer_offset = self.vertices_count + new_vertices.len();
                    primitive.index_buffer_offset = self.indices_count + new_indices.len();
                    new_vertices.extend_from_slice(&primitive.vertex_data);
                    if !primitive.index_data_u8.is_empty() {
                        new_indices.extend(
                            primitive.index_data_u8.iter().map(|&i| i as u32 + primitive.vertex_buffer_offset as u32)
                        );
                    } else if !primitive.index_data_u16.is_empty() {
                        new_indices.extend(
                            primitive.index_data_u16.iter().map(|&i| i as u32 + primitive.vertex_buffer_offset as u32)
                        );
                    } else if !primitive.index_data_u32.is_empty() {
                        new_indices.extend(
                            primitive.index_data_u32.iter().map(|&i| i + primitive.vertex_buffer_offset as u32)
                        );
                    }
                    primitive.construct_min_max()
                }
            }
            for material in model.materials.iter() {
                new_materials_send.push(self.materials[*material].to_sendable(self.texture_count));
            }

            let mut new_joints_send = Vec::new();
            for skin_index in model.skins.iter() {
                let skin = &mut self.skins[*skin_index];
                skin.construct_joint_matrices(&mut self.nodes, &self.accessors, &self.buffer_views, &self.buffers);
                for joint in skin.joint_matrices.iter() {
                    new_joints_send.push(joint.clone());
                }
            }

            self.vertices_count += new_vertices.len();
            self.indices_count += new_indices.len();
            self.materials_count += new_materials_send.len();
            self.texture_count += model.textures.len() as i32;
            self.joints_count += new_joints_send.len();

            self.new_indices.extend(new_indices);
            self.new_vertices.extend(new_vertices);
            self.new_joints.extend(new_joints_send);
            self.new_materials.extend(new_materials_send);

            self.buffers_need_update = true;

            self.models.push(model);
        }
        *self.loaded_files.get(&String::from(uri)).unwrap()
    }
    pub unsafe fn add_texture(&mut self, uri: &str, generate_mips: bool) -> usize {
        let sampler = if !self.loaded_files.contains_key(uri) {
            let path = PathBuf::from(uri);
            let (image_view, image, mips) = unsafe { self.context.create_2d_texture_image(&path, generate_mips) };
            let image = Image {
                mime_type: String::new(),
                name: String::from(uri),
                uri: path,
                generated: true,
                image,
                image_view: image_view.0,
                mip_levels: mips,
            };
            self.loaded_files.insert(String::from(uri), self.images.len());
            self.images.push(image);
            image_view.1
        } else {
            self.textures[0].sampler
        };

        let index = self.textures.len();
        self.textures.push(SceneTexture {
            source: *self.loaded_files.get(&String::from(uri)).unwrap(),
            sampler,
            sampler_info: SceneSampler {
                mag_filter: vk::Filter::LINEAR,
                min_filter: vk::Filter::LINEAR,
                address_mode_u: vk::SamplerAddressMode::REPEAT,
                address_mode_v: vk::SamplerAddressMode::REPEAT,
                address_mode_w: vk::SamplerAddressMode::REPEAT,
            },
            has_sampler: true
        });

        self.texture_count += 1;

        index
    }
    pub unsafe fn update_buffers(&mut self, base: &VkBase, command_buffer: CommandBuffer) { unsafe {
        if self.buffers_need_update {
            self.buffers_need_update = false;

            self.construct_textures(base);

            /*
            // let new_vertex_buffer_size =
            //     (size_of::<Vertex>() * (self.buffer_vertices_count + self.new_vertices.len())) as u64;
            // if new_vertex_buffer_size > self.vertex_buffer_size {
            //
            //     (self.vertex_buffer, self.vertex_staging_buffer) =
            //         self.context.create_device_and_staging_buffer(
            //             new_vertex_buffer_size,
            //             &[0],
            //             vk::BufferUsageFlags::VERTEX_BUFFER,
            //             false,
            //             true,
            //             false
            //         );
            // }
            */

            self.context.update_buffer_through_staging(
                &command_buffer,
                &self.vertex_buffer,
                &self.vertex_staging_buffer,
                &self.new_vertices,
                size_of::<Vertex>() as u64 * self.buffer_vertices_count as u64,
                true
            );
            self.context.update_buffer_through_staging(
                &command_buffer,
                &self.index_buffer,
                &self.index_staging_buffer,
                &self.new_indices,
                size_of::<u32>() as u64 * self.buffer_indices_count as u64,
                true
            );
            for frame in 0..self.material_buffers.len() {
                self.context.update_buffer_through_staging(
                    &command_buffer,
                    &self.material_buffers[frame],
                    &self.material_staging_buffer,
                    &self.new_materials,
                    size_of::<MaterialSendable>() as u64 * self.buffer_materials_count as u64,
                    frame == 0
                );
                if self.new_joints.len() > 0 {
                    self.context.update_buffer_through_staging(
                        &command_buffer,
                        &self.joints_buffers[frame],
                        &self.joints_staging_buffer,
                        &self.new_joints,
                        size_of::<Matrix>() as u64 * self.buffer_joints_count as u64,
                        frame == 0
                    );
                }
            }
            self.buffer_vertices_count += self.new_vertices.len();
            self.buffer_indices_count += self.new_indices.len();
            self.buffer_materials_count += self.new_materials.len();
            self.buffer_joints_count += self.new_joints.len();

            self.new_indices.clear();
            self.new_vertices.clear();
            self.new_joints.clear();
            self.new_materials.clear();
        }
    } }

    pub unsafe fn construct_textures(&mut self, base: &VkBase) { unsafe {
        let ungenerated_indices = self.images
            .iter()
            .enumerate()
            .filter(|(_, img)| !img.generated)
            .map(|(i, _)| i)
            .collect::<Vec<usize>>();
        let uris: Vec<PathBuf> = ungenerated_indices
            .iter()
            .map(|i| self.images[*i].uri.clone())
            .collect();
        let image_sources = self.context.load_textures_batched(uris.as_slice(), true);
        for (i, ungenerated_image_index) in ungenerated_indices.iter().enumerate() {
            let img = &mut self.images[*ungenerated_image_index];
            let (image_view, image, mips) = image_sources[i];
            img.image = image;
            img.image_view = image_view.0;
            img.mip_levels = mips;
            img.generated = true;
            base.device.destroy_sampler(image_view.1, None);
        }
        for texture in &mut self.textures {
            if texture.has_sampler { continue }
            texture.construct_sampler(self.images[texture.source].mip_levels as f32, base);
        }
    } }

    pub unsafe fn destroy(&mut self, base: &VkBase) { unsafe {
        for instance_buffer in &self.instance_buffers {
            base.device.destroy_buffer(instance_buffer.0, None);
            base.device.free_memory(instance_buffer.1, None);
        }
        base.device.unmap_memory(self.instance_staging_buffer.1);
        base.device.destroy_buffer(self.instance_staging_buffer.0, None);
        base.device.free_memory(self.instance_staging_buffer.1, None);

        for material_buffer in &self.material_buffers {
            base.device.destroy_buffer(material_buffer.0, None);
            base.device.free_memory(material_buffer.1, None);
        }
        base.device.destroy_buffer(self.material_staging_buffer.0, None);
        base.device.free_memory(self.material_staging_buffer.1, None);

        for light_buffer in &self.lights_buffers {
            base.device.destroy_buffer(light_buffer.0, None);
            base.device.free_memory(light_buffer.1, None);
        }
        base.device.destroy_buffer(self.lights_staging_buffer.0, None);
        base.device.free_memory(self.lights_staging_buffer.1, None);

        for joints_buffer in &self.joints_buffers {
            base.device.destroy_buffer(joints_buffer.0, None);
            base.device.free_memory(joints_buffer.1, None);
        }
        base.device.unmap_memory(self.joints_staging_buffer.1);
        base.device.destroy_buffer(self.joints_staging_buffer.0, None);
        base.device.free_memory(self.joints_staging_buffer.1, None);

        base.device.destroy_buffer(self.index_buffer.0, None);
        base.device.free_memory(self.index_buffer.1, None);
        base.device.destroy_buffer(self.index_staging_buffer.0, None);
        base.device.free_memory(self.index_staging_buffer.1, None);
        base.device.destroy_buffer(self.vertex_buffer.0, None);
        base.device.free_memory(self.vertex_buffer.1, None);
        base.device.destroy_buffer(self.vertex_staging_buffer.0, None);
        base.device.free_memory(self.vertex_staging_buffer.1, None);

        for texture in &self.textures {
            base.device.destroy_sampler(texture.sampler, None);
        }
        for image in &self.images {
            base.device.destroy_image_view(image.image_view, None);
            base.device.destroy_image(image.image.0, None);
            base.device.free_memory(image.image.1, None);
        }
    } }
}

#[derive(Copy)]
#[derive(Clone)]
pub struct LightSendable {
    pub position: [f32; 3],
    pub _pad0: u32,
    pub direction: [f32; 3],
    pub light_type: u32,
    pub attenuation_values: [f32; 3],
    pub _pad1: u32,
    pub color: [f32; 3],
    pub _pad2: u32,
}
#[derive(Copy)]
#[derive(Clone)]
pub struct SunSendable {
    pub matrices: [[f32; 16]; 5],
    pub vector: [f32; 3],
    pub _pad0: u32,
    pub color: [f32; 3],
    pub _pad1: u32,
}

#[derive(Clone)]
pub struct ModelContainer {
    pub extensions_used: Vec<String>,
    pub scene: usize,
    pub scenes: Vec<usize>,
    pub animations: Vec<usize>,
    pub skins: Vec<usize>,
    pub nodes: Vec<usize>,
    pub meshes: Vec<usize>,
    pub materials: Vec<usize>,
    pub textures: Vec<usize>,
    pub images: Vec<usize>,
    pub accessors: Vec<usize>,
    pub buffer_views: Vec<usize>,
    pub buffers: Vec<usize>,
}
impl ModelContainer {
    pub fn new(path: &str, world: &mut World) -> Self {
        let json = json::parse(fs::read_to_string(path).expect("failed to load json file").as_str()).expect("json parse error");

        let initial_buffer_count = world.buffers.len();
        let initial_buffer_view_count = world.buffer_views.len();
        let initial_accessors_count = world.accessors.len();
        let initial_images_count = world.images.len();
        let initial_skins_count = world.skins.len();
        let initial_textures_count = world.textures.len();
        let initial_materials_count = world.materials.len();
        let initial_primitive_count = world.primitive_count;
        let initial_animation_count = world.animations.len();
        let initial_node_count = world.nodes.len();
        let initial_mesh_count = world.meshes.len();
        let initial_scene_count = world.scenes.len();

        let mut extensions_used = Vec::new();
        for extension in json["extensionsUsed"].members() {
            extensions_used.push(extension.as_str().unwrap().to_string());
        }

        let mut buffers = Vec::new();
        for buffer in json["buffers"].members() {
            buffers.push(
                Buffer::new(
                    resolve_gltf_uri(path, buffer["uri"].as_str().unwrap()),
                    buffer["byteLength"].as_usize().unwrap()
                ))
        }
        world.buffers.extend(buffers);

        let mut buffer_views = Vec::new();
        for buffer_view in json["bufferViews"].members() {
            buffer_views.push(
                BufferView {
                    buffer: buffer_view["buffer"].as_usize().unwrap() + initial_buffer_count,
                    byte_length: buffer_view["byteLength"].as_usize().unwrap(),
                    byte_offset: buffer_view["byteOffset"].as_usize().unwrap_or(0),
                    target: buffer_view["target"].as_usize().unwrap_or(0)
                })
        }
        world.buffer_views.extend(buffer_views);

        let mut accessors = Vec::new();
        for accessor in json["accessors"].members() {
            let mut min: Option<Vector> = None;
            let mut max: Option<Vector> = None;
            if let JsonValue::Array(ref min_data) = accessor["min"] {
                if min_data.len() >= 3 {
                    min = Some(Vector::new3(
                        min_data[0].as_f32().unwrap(),
                        min_data[1].as_f32().unwrap(),
                        min_data[2].as_f32().unwrap()));
                }
            }
            if let JsonValue::Array(ref max_data) = accessor["max"] {
                if max_data.len() >= 3 {
                    max = Some(Vector::new3(
                        max_data[0].as_f32().unwrap(), 
                        max_data[1].as_f32().unwrap(), 
                        max_data[2].as_f32().unwrap()));
                }
            }
            accessors.push(
                Accessor {
                    buffer_view: accessor["bufferView"].as_usize().unwrap() + initial_buffer_view_count,
                    component_type: ComponentType::from_u32(accessor["componentType"].as_u32().unwrap()).expect("unsupported component type"),
                    count: accessor["count"].as_usize().unwrap(),
                    r#type: accessor["type"].as_str().unwrap().parse().unwrap(),
                    min,
                    max,
                    data: Vec::new(),
                })
        }
        world.accessors.extend(accessors);

        let mut images = Vec::new();
        for image in json["images"].members() {
            let name_maybe: Option<&str> = image["name"].as_str();
            let mut name = String::from("unnamed image");
            match name_maybe {
                Some(name_str) => name = String::from(name_str),
                None => (),
            }

            let mime_type_maybe: Option<&str> = image["mimeType"].as_str();
            let mut mime_type = String::from("no mime type");
            match mime_type_maybe {
                Some(mime_type_str) => mime_type = String::from(mime_type_str),
                None => (),
            }

            let uri = image["uri"].as_str().unwrap();

            world.loaded_files.insert(String::from(uri), initial_images_count + images.len());

            images.push(
                Image::new(
                    mime_type,
                    name,
                    resolve_gltf_uri(path, uri)
                ))
        }
        world.images.extend(images);

        let mut samplers = Vec::new();
        for sampler in json["samplers"].members() {
            samplers.push(SceneSampler {
                min_filter: SceneSampler::get_filter_type(sampler["minFilter"].as_i32().unwrap_or(0)),
                mag_filter: SceneSampler::get_filter_type(sampler["magFilter"].as_i32().unwrap_or(0)),
                address_mode_u: SceneSampler::get_address_mode(sampler["wrapS"].as_i32().unwrap_or(0)),
                address_mode_v: SceneSampler::get_address_mode(sampler["wrapT"].as_i32().unwrap_or(0)),
                address_mode_w: SceneSampler::get_address_mode(sampler["wrapT"].as_i32().unwrap_or(0)), // TODO() Deduce if modern gltf has a w-wrapping field
            })
        }

        let mut textures = Vec::new();
        for texture in json["textures"].members() {
            textures.push(
                SceneTexture {
                    source: texture["source"].as_usize().unwrap() + initial_images_count,
                    sampler: Sampler::null(),
                    sampler_info: samplers[texture["sampler"].as_usize().unwrap().clone()],
                    has_sampler: false
                })
        }
        world.textures.extend(textures);

        let mut materials = Vec::new();
        materials.push(Material {
            alpha_mode: String::from("BLEND"),
            alpha_cutoff: 0.5,
            double_sided: false,
            normal_texture: None,
            normal_texture_offset: None,
            normal_texture_scale: None,
            specular_color_factor: [1.0; 3],
            ior: 1.0,
            name: String::from("default material"),
            base_color_factor: [1.0; 4],
            base_color_texture: None,
            base_color_texture_offset: None,
            base_color_texture_scale: None,
            metallic_factor: 0.1,
            metallic_texture: None,
            metallic_texture_offset: None,
            metallic_texture_scale: None,
            roughness_factor: 0.5,
            roughness_texture: None,
            roughness_texture_offset: None,
            roughness_texture_scale: None,
            emissive_factor: [0.0; 3],
            emissive_texture: None,
            emissive_texture_offset: None,
            emissive_texture_scale: None,
            emissive_strength: 1.0,
        });
        for material in json["materials"].members() {
            let name_maybe: Option<&str> = material["name"].as_str();
            let mut name = String::from("unnamed node");
            match name_maybe {
                Some(name_str) => name = String::from(name_str),
                None => (),
            }

            let mut alpha_mode = String::from("BLEND");
            if let JsonValue::String(ref alpha_mode_json) = material["alphaMode"] {
                alpha_mode = (*alpha_mode_json).parse().unwrap();
            }

            let mut alpha_cutoff = 0.5;
            if let JsonValue::Number(ref alpha_cutoff_json) = material["alphaCutoff"] {
                if let Ok(f) = alpha_cutoff_json.to_string().parse::<f32>() {
                    alpha_cutoff = f;
                }
            }

            let mut double_sided = false;
            if let JsonValue::Boolean(ref double_sided_json) = material["doubleSided"] {
                double_sided = *double_sided_json;
            }

            let mut normal_texture = None;
            let mut normal_texture_offset = None;
            let mut normal_texture_scale = None;
            if let JsonValue::Object(ref normal_texture_json) = material["normalTexture"] {
                normal_texture = Some(normal_texture_json["index"].as_i32().expect(""));
                if let JsonValue::Object(ref extensions_json) = normal_texture_json["extensions"] {
                    if let JsonValue::Object(ref texture_transform_json) = extensions_json["KHR_texture_transform"] {
                        if let JsonValue::Array(ref json_value) = texture_transform_json["offset"] {
                            if json_value.len() >= 3 {
                                normal_texture_offset = Some([
                                    json_value[0].as_f32().unwrap(),
                                    json_value[1].as_f32().unwrap(),
                                ]);
                            }
                        }
                        if let JsonValue::Array(ref json_value) = texture_transform_json["scale"] {
                            if json_value.len() >= 3 {
                                normal_texture_scale = Some([
                                    json_value[0].as_f32().unwrap(),
                                    json_value[1].as_f32().unwrap(),
                                ]);
                            }
                        }
                    }
                }
            }

            let mut emissive_factor = [0.0; 3];
            let mut emissive_texture = None;
            let mut emissive_texture_offset = None;
            let mut emissive_texture_scale = None;
            if let JsonValue::Array(ref json_value) = material["emissiveFactor"] {
                if json_value.len() >= 3 {
                    emissive_factor = [
                        json_value[0].as_f32().unwrap(),
                        json_value[1].as_f32().unwrap(),
                        json_value[2].as_f32().unwrap(),
                    ];
                }
            }
            if let JsonValue::Object(ref emissive_texture_json) = material["emissiveTexture"] {
                emissive_texture = Some(emissive_texture_json["index"].as_i32().expect("FAULTY GLTF: \n    Missing index for emissiveTexture"));
                if let JsonValue::Object(ref extensions_json) = emissive_texture_json["extensions"] {
                    if let JsonValue::Object(ref texture_transform_json) = extensions_json["KHR_texture_transform"] {
                        if let JsonValue::Array(ref json_value) = texture_transform_json["offset"] {
                            if json_value.len() >= 3 {
                                emissive_texture_offset = Some([
                                    json_value[0].as_f32().unwrap(),
                                    json_value[1].as_f32().unwrap(),
                                ]);
                            }
                        }
                        if let JsonValue::Array(ref json_value) = texture_transform_json["scale"] {
                            if json_value.len() >= 3 {
                                emissive_texture_scale = Some([
                                    json_value[0].as_f32().unwrap(),
                                    json_value[1].as_f32().unwrap(),
                                ]);
                            }
                        }
                    }
                }
            }

            let mut base_color_factor = [0.5, 0.5, 0.5, 1.0];
            let mut base_color_texture = None;
            let mut base_color_texture_offset = None;
            let mut base_color_texture_scale = None;
            let mut metallic_factor = 0.1;
            let mut roughness_factor = 0.5;
            let mut metallic_texture = None;
            let mut metallic_texture_offset = None;
            let mut metallic_texture_scale = None;
            let mut roughness_texture = None;
            let mut roughness_texture_offset = None;
            let mut roughness_texture_scale = None;
            if let JsonValue::Object(ref pbr_metallic_roughness) = material["pbrMetallicRoughness"] {
                if let JsonValue::Array(ref json_value) = pbr_metallic_roughness["baseColorFactor"] {
                    if json_value.len() >= 4 {
                        base_color_factor = [
                            json_value[0].as_f32().unwrap(),
                            json_value[1].as_f32().unwrap(),
                            json_value[2].as_f32().unwrap(),
                            json_value[3].as_f32().unwrap(),
                        ];
                    }
                }
                if let JsonValue::Object(ref base_color_texture_json) = pbr_metallic_roughness["baseColorTexture"] {
                    base_color_texture = Some(base_color_texture_json["index"].as_i32().expect("FAULTY GLTF: \n    Missing index for baseColorTexture at pbrMetallicRoughness"));
                    if let JsonValue::Object(ref extensions_json) = base_color_texture_json["extensions"] {
                        if let JsonValue::Object(ref texture_transform_json) = extensions_json["KHR_texture_transform"] {
                            if let JsonValue::Array(ref json_value) = texture_transform_json["offset"] {
                                if json_value.len() >= 2 {
                                    base_color_texture_offset = Some([
                                        json_value[0].as_f32().unwrap(),
                                        json_value[1].as_f32().unwrap(),
                                    ]);
                                }
                            }
                            if let JsonValue::Array(ref json_value) = texture_transform_json["scale"] {
                                if json_value.len() >= 2 {
                                    base_color_texture_scale = Some([
                                        json_value[0].as_f32().unwrap(),
                                        json_value[1].as_f32().unwrap(),
                                    ]);
                                }
                            }
                        }
                    }
                }

                if let JsonValue::Number(ref json_value) = pbr_metallic_roughness["metallicFactor"] {
                    if let Ok(f) = json_value.to_string().parse::<f32>() {
                        metallic_factor = f;
                    }
                }
                if let JsonValue::Object(ref metallic_texture_json) = pbr_metallic_roughness["metallicTexture"] {
                    metallic_texture = Some(metallic_texture_json["index"].as_i32().expect("FAULTY GLTF: \n    Missing index for metallicTexture at pbrMetallicRoughness"));
                    if let JsonValue::Object(ref extensions_json) = metallic_texture_json["extensions"] {
                        if let JsonValue::Object(ref texture_transform_json) = extensions_json["KHR_texture_transform"] {
                            if let JsonValue::Array(ref json_value) = texture_transform_json["offset"] {
                                if json_value.len() >= 2 {
                                    metallic_texture_offset = Some([
                                        json_value[0].as_f32().unwrap(),
                                        json_value[1].as_f32().unwrap(),
                                    ]);
                                }
                            }
                            if let JsonValue::Array(ref json_value) = texture_transform_json["scale"] {
                                if json_value.len() >= 2 {
                                    metallic_texture_scale = Some([
                                        json_value[0].as_f32().unwrap(),
                                        json_value[1].as_f32().unwrap(),
                                    ]);
                                }
                            }
                        }
                    }
                }

                if let JsonValue::Number(ref json_value) = pbr_metallic_roughness["roughnessFactor"] {
                    if let Ok(f) = json_value.to_string().parse::<f32>() {
                        roughness_factor = f;
                    }
                }
                if let JsonValue::Object(ref roughness_texture_json) = pbr_metallic_roughness["roughnessTexture"] {
                    roughness_texture = Some(roughness_texture_json["index"].as_i32().expect("FAULTY GLTF: \n    Missing index for roughnessTexture at pbrMetallicRoughness"));
                    if let JsonValue::Object(ref extensions_json) = roughness_texture_json["extensions"] {
                        if let JsonValue::Object(ref texture_transform_json) = extensions_json["KHR_texture_transform"] {
                            if let JsonValue::Array(ref json_value) = texture_transform_json["offset"] {
                                if json_value.len() >= 2 {
                                    roughness_texture_offset = Some([
                                        json_value[0].as_f32().unwrap(),
                                        json_value[1].as_f32().unwrap(),
                                    ]);
                                }
                            }
                            if let JsonValue::Array(ref json_value) = texture_transform_json["scale"] {
                                if json_value.len() >= 2 {
                                    roughness_texture_scale = Some([
                                        json_value[0].as_f32().unwrap(),
                                        json_value[1].as_f32().unwrap(),
                                    ]);
                                }
                            }
                        }
                    }
                }

                if let JsonValue::Object(ref metallic_roughness_texture_json) = pbr_metallic_roughness["metallicRoughnessTexture"] {
                    roughness_texture = Some(metallic_roughness_texture_json["index"].as_i32().expect("FAULTY GLTF: \n    Missing index for metallicRoughnessTexture at pbrMetallicRoughness"));
                    metallic_texture = Some(metallic_roughness_texture_json["index"].as_i32().expect("FAULTY GLTF: \n    Missing index for metallicRoughnessTexture at pbrMetallicRoughness"));
                    if let JsonValue::Object(ref extensions_json) = metallic_roughness_texture_json["extensions"] {
                        if let JsonValue::Object(ref texture_transform_json) = extensions_json["KHR_texture_transform"] {
                            if let JsonValue::Array(ref json_value) = texture_transform_json["offset"] {
                                if json_value.len() >= 2 {
                                    metallic_texture_offset = Some([
                                        json_value[0].as_f32().unwrap(),
                                        json_value[1].as_f32().unwrap(),
                                    ]);
                                    roughness_texture_offset = Some([
                                        json_value[0].as_f32().unwrap(),
                                        json_value[1].as_f32().unwrap(),
                                    ]);
                                }
                            }
                            if let JsonValue::Array(ref json_value) = texture_transform_json["scale"] {
                                if json_value.len() >= 2 {
                                    metallic_texture_scale = Some([
                                        json_value[0].as_f32().unwrap(),
                                        json_value[1].as_f32().unwrap(),
                                    ]);
                                    roughness_texture_scale = Some([
                                        json_value[0].as_f32().unwrap(),
                                        json_value[1].as_f32().unwrap(),
                                    ]);
                                }
                            }
                        }
                    }
                }
            }

            let mut specular_color_factor = [0.0; 3];
            if let JsonValue::Object(ref khr_materials_specular) = material["KHR_materials_specular"] {
                if let JsonValue::Array(ref json_val) = khr_materials_specular["baseColorFactor"] {
                    if json_val.len() >= 3 {
                        specular_color_factor = [
                            json_val[0].as_f32().unwrap(),
                            json_val[1].as_f32().unwrap(),
                            json_val[2].as_f32().unwrap(),
                        ];
                    }
                }
            }

            let mut ior = 1.0;
            if let JsonValue::Object(ref khr_materials_ior) = material["KHR_materials_ior"] {
                if let JsonValue::Number(ref json_value) = khr_materials_ior["ior"] {
                    if let Ok(f) = json_value.to_string().parse::<f32>() {
                        ior = f;
                    }
                }
            }

            let mut emissive_strength = 1.0;
            if let JsonValue::Object(ref extensions) = material["extensions"] {
                if let JsonValue::Object(ref json_value) = extensions["KHR_materials_emissive_strength"] {
                    emissive_strength = json_value["emissiveStrength"].as_f32().expect("FAULTY GLTF: \n    Missing emissiveStrength for KHR_materials_emissive_strength");
                }
            }

            materials.push(
                Material {
                    name,
                    alpha_mode,
                    alpha_cutoff,
                    double_sided,
                    normal_texture,
                    // KHR_texture_transform
                        normal_texture_offset,
                        normal_texture_scale,
                    // KHR_materials_specular
                        specular_color_factor,
                    // KHR_materials_ior
                        ior,
                    // pbrMetallicRoughness
                        base_color_factor,
                        base_color_texture,
                        // KHR_texture_transform
                            base_color_texture_offset,
                            base_color_texture_scale,
                        metallic_factor,
                        metallic_texture,
                        // KHR_texture_transform
                            metallic_texture_offset,
                            metallic_texture_scale,
                        roughness_factor,
                        roughness_texture,
                        // KHR_texture_transform
                            roughness_texture_offset,
                            roughness_texture_scale,
                    emissive_factor,
                    emissive_texture,
                        // KHR_texture_transform
                        emissive_texture_offset,
                        emissive_texture_scale,
                    // extensions
                        emissive_strength,
                })
        }
        world.materials.extend(materials);

        let mut meshes = Vec::new();
        for mesh in json["meshes"].members() {
            let name_maybe: Option<&str> = mesh["name"].as_str();
            let mut name = String::from("unnamed node");
            match name_maybe {
                Some(name_str) => name = String::from(name_str),
                None => (),
            }

            let mut primitives = Vec::new();
            if let JsonValue::Array(ref primitives_json) = mesh["primitives"] {
                for primitive_json in primitives_json {
                    let mut attributes: Vec<(String, usize)> = Vec::new();
                    if let JsonValue::Object(ref attributes_json) = primitive_json["attributes"] {
                        for (name, accessor) in attributes_json.iter() {
                            attributes.push((name.to_string(), accessor.as_usize().unwrap() + initial_accessors_count));
                        }
                    }

                    let indices = primitive_json["indices"].as_usize().unwrap() + initial_accessors_count;

                    let material_index = if let Some(material_index) = primitive_json["material"].as_u32() {
                        material_index + initial_materials_count as u32 + 1u32
                    } else {
                        0u32
                    };
                    primitives.push(Primitive {
                        attributes,
                        indices_count: world.accessors[indices].count,
                        indices,
                        index_buffer_offset: 0,
                        vertex_buffer_offset: 0,
                        index_data_u8: Vec::new(),
                        index_data_u16: Vec::new(),
                        index_data_u32: Vec::new(),
                        vertex_data: Vec::new(),
                        material_index,
                        min: Vector::new(),
                        max: Vector::new(),
                        corners: [Vector::new(); 8],
                        id: world.primitive_count,
                    });
                    world.primitive_count += 1;
                }
            }

            meshes.push(
                Mesh {
                    name,
                    primitives,
                }
            );
        }
        world.meshes.extend(meshes);

        let mut nodes = Vec::new();
        for node in json["nodes"].members() {
            let name_maybe: Option<&str> = node["name"].as_str();
            let mut name = String::from("unnamed node");
            match name_maybe {
                Some(name_str) => name = String::from(name_str),
                None => (),
            }

            let mut mesh = if let Some(index) = node["mesh"].as_usize() {
                Some(index + initial_mesh_count)
            } else {
                None
            };

            let skin = if let Some(index) = node["skin"].as_usize() {
                Some((index + initial_skins_count) as i32)
            } else {
                None
            };

            let mut rotation = Vector::new();
            if let JsonValue::Array(ref rotation_json) = node["rotation"] {
                if rotation_json.len() >= 4 {
                    rotation = Vector::new4(
                        rotation_json[0].as_f32().unwrap(),
                        rotation_json[1].as_f32().unwrap(),
                        rotation_json[2].as_f32().unwrap(),
                        rotation_json[3].as_f32().unwrap()
                    ).normalize4();
                };
            }

            let mut scale = Vector::fill(1.0);
            if let JsonValue::Array(ref scale_json) = node["scale"] {
                if scale_json.len() >= 3 {
                    scale = Vector::new3(
                        scale_json[0].as_f32().unwrap(),
                        scale_json[1].as_f32().unwrap(),
                        scale_json[2].as_f32().unwrap()
                    );
                }
            }

            let mut translation = Vector::empty();
            if let JsonValue::Array(ref translation_json) = node["translation"] {
                if translation_json.len() >= 3 {
                    translation = Vector::new3(
                        translation_json[0].as_f32().unwrap(),
                        translation_json[1].as_f32().unwrap(),
                        translation_json[2].as_f32().unwrap()
                    );
                }
            }

            let mut children_indices = Vec::new();
            if let JsonValue::Array(ref children_json) = node["children"] {
                for child_json in children_json {
                    children_indices.push(child_json.as_usize().unwrap() + initial_node_count);
                }
            }

            nodes.push(
                Node {
                    mapped_entity_index: 0,
                    name,
                    mesh,
                    skin,
                    rotation,
                    scale,
                    translation,
                    needs_update: true,
                    user_rotation: Vector::new(),
                    user_scale: Vector::fill(1.0),
                    user_translation: Vector::empty(),
                    original_rotation: rotation,
                    original_scale: scale,
                    original_translation: translation,
                    local_transform: Matrix::new_empty(),
                    world_transform: Matrix::new_empty(),
                    children_indices,
                }
            )
        }
        world.nodes.extend(nodes);

        let mut skins = Vec::new();
        for skin in json["skins"].members() {
            let name_maybe: Option<&str> = skin["name"].as_str();
            let mut name = String::from("unnamed skin");
            match name_maybe {
                Some(name_str) => name = String::from(name_str),
                None => (),
            }

            let mut joint_indices = Vec::new();
            if let JsonValue::Array(ref joint_json) = skin["joints"] {
                for joint in joint_json.iter() {
                    joint_indices.push(joint.as_usize().unwrap() + initial_node_count);
                }
            }

            let inverse_bind_matrices_accessor = skin["inverseBindMatrices"].as_usize().unwrap() + initial_accessors_count;

            let mut skeleton: Option<usize> = if let Some(skeleton_index) = skin["skeleton"].as_usize() {
                Some(skeleton_index + initial_node_count)
            } else {
                None
            };

            skins.push(Skin {
                name,
                inverse_bind_matrices_accessor,
                inverse_bind_matrices: Vec::new(),
                joint_indices,
                joint_matrices: Vec::new(),
                skeleton,
            })
        }
        world.skins.extend(skins);

        let mut animations = Vec::new();
        for animation in json["animations"].members() {
            let name_maybe: Option<&str> = animation["name"].as_str();
            let mut name = String::from("unnamed animation");
            match name_maybe {
                Some(name_str) => name = String::from(name_str),
                None => (),
            }

            let mut channels = Vec::new();
            if let JsonValue::Array(ref channels_json) = animation["channels"] {
                for channel in channels_json {
                    channels.push((
                        channel["sampler"].as_usize().unwrap() + initial_accessors_count,
                        channel["target"]["node"].as_usize().unwrap() + initial_node_count,
                        String::from(channel["target"]["path"].as_str().unwrap())
                    ))
                }
            }

            let mut samplers = Vec::new();
            if let JsonValue::Array(ref samplers_json) = animation["samplers"] {
                for sampler_json in samplers_json {
                    samplers.push((
                        sampler_json["input"].as_usize().unwrap() + initial_accessors_count,
                        String::from(sampler_json["interpolation"].as_str().unwrap()),
                        sampler_json["output"].as_usize().unwrap() + initial_accessors_count
                    ))
                }
            }

            animations.push(Animation::new(
                name,
                world,
                channels,
                samplers,
            ))
        }
        world.animations.extend(animations);

        let mut scenes = Vec::new();
        for scene in json["scenes"].members() {
            let name_maybe: Option<&str> = scene["name"].as_str();
            let mut name = String::from("unnamed world");
            match name_maybe {
                Some(name_str) => name = String::from(name_str),
                None => (),
            }

            let mut scene_nodes = Vec::new();
            if let JsonValue::Array(ref nodes_json) = scene["nodes"] {
                for node_json in nodes_json {
                    scene_nodes.push(node_json.as_usize().unwrap() + initial_node_count);
                }
            }

            scenes.push(
                GltfScene {
                    name,
                    nodes: scene_nodes,
                }
            )
        }
        world.scenes.extend(scenes);

        let scene = json["world"].as_usize().unwrap_or(0) + initial_scene_count;

        Self {
            extensions_used,
            scene,
            scenes: (initial_scene_count..world.scenes.len()).collect(),
            animations: (initial_animation_count..world.animations.len()).collect(),
            skins: (initial_skins_count..world.skins.len()).collect(),
            nodes: (initial_node_count..world.nodes.len()).collect(),
            meshes: (initial_mesh_count..world.meshes.len()).collect(),
            materials: (initial_materials_count..world.materials.len()).collect(),
            textures: (initial_textures_count..world.textures.len()).collect(),
            images: (initial_images_count..world.images.len()).collect(),
            accessors: (initial_accessors_count..world.accessors.len()).collect(),
            buffer_views: (initial_buffer_view_count..world.buffer_views.len()).collect(),
            buffers: (initial_buffer_count..world.buffers.len()).collect(),
        }
    }

    ///* Takes euler for rotation, converts to quaternion
    pub fn transform_roots(&mut self, world: &mut World, translation: &Vector, rotation: &Vector, scale: &Vector) {
        let scene = &world.scenes[self.scene];
        for node_index in scene.nodes.iter() {
            let node = &mut world.nodes[*node_index];
            node.user_translation.add_vec_to_self(translation);
            node.user_rotation.combine_to_self(&rotation.euler_to_quat().normalize4());
            node.user_scale.mul_by_vec_to_self(scale);
            node.needs_update = true;
        }
    }
}

pub struct Buffer {
    pub uri: PathBuf,
    pub byte_length: usize,
    pub data: Vec<u8>,
}
impl Buffer {
    fn new(uri: PathBuf, byte_length: usize) -> Self {
        Buffer {
            data: fs::read(&uri).expect("failed to load buffer").to_vec(),
            uri,
            byte_length,
        }
    }
}

pub struct BufferView {
    pub buffer: usize,
    pub byte_length: usize,
    pub byte_offset: usize,
    pub target: usize,
}

pub struct Accessor {
    pub buffer_view: usize,
    pub component_type: ComponentType,
    pub count: usize,
    pub r#type: String,
    pub min: Option<Vector>,
    pub max: Option<Vector>,
    pub data: Vec<Vec<f32>>,
}

pub struct Image {
    pub mime_type: String,
    pub name: String,
    pub uri: PathBuf,

    pub generated: bool,
    pub image: (vk::Image, DeviceMemory),
    pub image_view: ImageView,
    pub mip_levels: u32,
}
impl Image {
    fn new(mime_type: String, name: String, uri: PathBuf) -> Self {
        Self {
            mime_type,
            name,
            uri,
            generated: false,
            image: (vk::Image::null(), DeviceMemory::null()),
            image_view: ImageView::null(),
            mip_levels: 0,
        }
    }

    unsafe fn construct_image_view(&mut self, context: &Arc<Context>) { unsafe {
        let (image_view, image, mips) = context.create_2d_texture_image(&self.uri, true);
        self.image = image;
        self.image_view = image_view.0;
        self.mip_levels = mips;
        self.generated = true;
        context.device.destroy_sampler(image_view.1, None);
    } }
}

#[derive(Copy, Clone)]
pub struct SceneSampler {
    pub mag_filter: vk::Filter,
    pub min_filter: vk::Filter,
    pub address_mode_u: vk::SamplerAddressMode,
    pub address_mode_v: vk::SamplerAddressMode,
    pub address_mode_w: vk::SamplerAddressMode,
}
impl SceneSampler {
    pub fn get_filter_type(id: i32) -> vk::Filter {
        match id {
            9728 => vk::Filter::NEAREST,
            9729 => vk::Filter::LINEAR,
            // TODO() Handle 9984 (NEAREST_MIPMAP_NEAREST), 9985 (LINEAR_MIPMAP_NEAREST), 9986 (NEAREST_MIPMAP_LINEAR), 9987 (LINEAR_MIPMAP_LINEAR)
            _ => vk::Filter::LINEAR,
        }
    }
    ///* Wrapping mode = address mode
    pub fn get_address_mode(id: i32) -> vk::SamplerAddressMode {
        match id {
            33071 => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            33648 => vk::SamplerAddressMode::MIRRORED_REPEAT,
            10497 => vk::SamplerAddressMode::REPEAT,
            _ => vk::SamplerAddressMode::REPEAT,
        }
    }
}

pub struct SceneTexture {
    pub source: usize,
    pub sampler: Sampler,
    pub sampler_info: SceneSampler,
    pub has_sampler: bool,
}
impl SceneTexture {
    pub unsafe fn construct_sampler(&mut self, max_lod: f32, base: &VkBase) { unsafe {
        let sampler_info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            mag_filter: self.sampler_info.mag_filter,
            min_filter: self.sampler_info.min_filter,
            address_mode_u: self.sampler_info.address_mode_u,
            address_mode_v: self.sampler_info.address_mode_v,
            address_mode_w: self.sampler_info.address_mode_w,
            anisotropy_enable: vk::TRUE,
            max_anisotropy: base.pdevice_properties.limits.max_sampler_anisotropy,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            mip_lod_bias: 0.0,
            min_lod: 0.0,
            max_lod,
            ..Default::default()
        };
        self.sampler = base.device.create_sampler(&sampler_info, None).expect("failed to create sampler");
        self.has_sampler = true;
    } }
}
pub struct Material {
    pub alpha_mode: String,
    pub alpha_cutoff: f32,
    pub double_sided: bool,
    pub normal_texture: Option<i32>,
    // KHR_texture_transform
        pub normal_texture_offset: Option<[f32; 2]>,
        pub normal_texture_scale: Option<[f32; 2]>,
    // KHR_materials_specular
        pub specular_color_factor: [f32; 3],
    // KHR_materials_ior
        pub ior: f32,
    pub name: String,
    // pbrMetallicRoughness
        pub base_color_factor: [f32; 4],
        pub base_color_texture: Option<i32>,
        // KHR_texture_transform
            pub base_color_texture_offset: Option<[f32; 2]>,
            pub base_color_texture_scale: Option<[f32; 2]>,
        pub metallic_factor: f32,
        pub metallic_texture: Option<i32>,
        // KHR_texture_transform
            pub metallic_texture_offset: Option<[f32; 2]>,
            pub metallic_texture_scale: Option<[f32; 2]>,
        pub roughness_factor: f32,
        pub roughness_texture: Option<i32>,
        // KHR_texture_transform
            pub roughness_texture_offset: Option<[f32; 2]>,
            pub roughness_texture_scale: Option<[f32; 2]>,
    pub emissive_factor: [f32; 3],
    pub emissive_texture: Option<i32>,
    // KHR_texture_transform
        pub emissive_texture_offset: Option<[f32; 2]>,
        pub emissive_texture_scale: Option<[f32; 2]>,
    // KHR_materials_emissive_strength
        pub emissive_strength: f32,
}
impl Material {
    fn to_sendable(&self, texture_offset: i32) -> MaterialSendable {
        MaterialSendable {
            normal_texture: self.normal_texture.map(|val| val + texture_offset).unwrap_or(-1),
            alpha_cutoff: self.alpha_cutoff,
            emissive_strength: self.emissive_strength,
            emissive_texture: self.emissive_texture.map(|val| val + texture_offset).unwrap_or(-1),

            normal_texture_offset: self.normal_texture_offset.unwrap_or([0.0; 2]),
            normal_texture_scale: self.normal_texture_scale.unwrap_or([1.0; 2]),

            emissive_texture_offset: self.emissive_texture_offset.unwrap_or([0.0; 2]),
            emissive_texture_scale: self.emissive_texture_scale.unwrap_or([1.0; 2]),

            specular_color_factor: self.specular_color_factor,
            _pad1: 0,

            emissive_factor: self.emissive_factor,
            _pad2: 0,

            ior: self.ior,
            _pad3: [0; 3],

            base_color_factor: self.base_color_factor,

            base_color_texture: self.base_color_texture.map(|val| val + texture_offset).unwrap_or(-1),
            metallic_factor: self.metallic_factor,
            metallic_texture: self.metallic_texture.map(|val| val + texture_offset).unwrap_or(-1),
            roughness_factor: self.roughness_factor,

            roughness_texture: self.roughness_texture.map(|val| val + texture_offset).unwrap_or(-1),
            _pad4: [0; 3],

            base_color_texture_offset: self.base_color_texture_offset.unwrap_or([0.0; 2]),
            base_color_texture_scale: self.base_color_texture_scale.unwrap_or([1.0; 2]),

            metallic_texture_offset: self.metallic_texture_offset.unwrap_or([0.0; 2]),
            metallic_texture_scale: self.metallic_texture_scale.unwrap_or([1.0; 2]),

            roughness_texture_offset: self.roughness_texture_offset.unwrap_or([0.0; 2]),
            roughness_texture_scale: self.roughness_texture_scale.unwrap_or([1.0; 2]),
        }
    }
}
#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct MaterialSendable { // newlines at each set of 16 bytes
    pub normal_texture: i32,
    pub alpha_cutoff: f32,
    pub emissive_strength: f32,
    pub emissive_texture: i32,

    pub normal_texture_offset: [f32; 2],
    pub normal_texture_scale: [f32; 2],

    pub emissive_texture_offset: [f32; 2],
    pub emissive_texture_scale: [f32; 2],

    pub specular_color_factor: [f32; 3],
    pub _pad1: u32,

    pub emissive_factor: [f32; 3],
    pub _pad2: u32,

    pub ior: f32,
    pub _pad3: [u32; 3],

    pub base_color_factor: [f32; 4],

    pub base_color_texture: i32,
    pub metallic_factor: f32,
    pub metallic_texture: i32,
    pub roughness_factor: f32,

    pub roughness_texture: i32,
    pub _pad4: [u32; 3],

    pub base_color_texture_offset: [f32; 2],
    pub base_color_texture_scale: [f32; 2],

    pub metallic_texture_offset: [f32; 2],
    pub metallic_texture_scale: [f32; 2],

    pub roughness_texture_offset: [f32; 2],
    pub roughness_texture_scale: [f32; 2],
}

#[derive(Debug, Clone, Copy)]
pub enum ComponentType {
    I8,
    U8,
    I16,
    U16,
    U32,
    F32,
}
impl ComponentType {
    fn from_u32(value: u32) -> Option<Self> {
        match value {
            5120 => Some(Self::I8),
            5121 => Some(Self::U8),
            5122 => Some(Self::I16),
            5123 => Some(Self::U16),
            5125 => Some(Self::U32),
            5126 => Some(Self::F32),
            _ => None,
        }
    }
}
pub struct Primitive {
    pub attributes: Vec<(String, usize)>, // ... + accessor
    pub indices: usize, // accesor

    pub material_index: u32,
    pub id: usize,

    pub min: Vector,
    pub max: Vector,
    pub corners: [Vector; 8],

    pub indices_count: usize,
    pub index_buffer_offset: usize,
    pub vertex_buffer_offset: usize,

    pub index_data_u8: Vec<u8>,
    pub index_data_u16: Vec<u16>,
    pub index_data_u32: Vec<u32>,
    pub vertex_data: Vec<Vertex>,
}
impl Primitive {
    fn construct_data(
        &mut self,
        world_accessors: &Vec<Accessor>,
        world_buffer_views: &Vec<BufferView>,
        world_buffers: &Vec<Buffer>,
    ) {
        let mut position_accessor: Option<&Accessor> = None;
        let mut normal_accessor: Option<&Accessor> = None;
        let mut texcoord_accessor: Option<&Accessor> = None;
        let mut joint_accessor: Option<&Accessor> = None;
        let mut weight_accessor: Option<&Accessor> = None;
        for attribute in self.attributes.iter() {
            if attribute.0.eq("POSITION") {
                position_accessor = Some(&world_accessors[attribute.1]);
            } else if attribute.0.eq("NORMAL") {
                normal_accessor = Some(&world_accessors[attribute.1]);
            } else if attribute.0.eq("TEXCOORD_0") {
                texcoord_accessor = Some(&world_accessors[attribute.1]);
            } else if attribute.0.eq("JOINTS_0") {
                joint_accessor = Some(&world_accessors[attribute.1]);
            } else if attribute.0.eq("WEIGHTS_0") {
                weight_accessor = Some(&world_accessors[attribute.1]);
            }
        }
        if position_accessor.is_none() {
            println!("Primitive has no POSITION attribute!");
        } else {
            let indices_accessor = &world_accessors[self.indices];
            let indices_accessor_buffer_view = &world_buffer_views[indices_accessor.buffer_view];
            let indices_accessor_buffer_view_buffer = &world_buffers[indices_accessor_buffer_view.buffer];
            let mut byte_offset = indices_accessor_buffer_view.byte_offset;
            let mut byte_length = indices_accessor_buffer_view.byte_length;
            let bytes = &indices_accessor_buffer_view_buffer.data[byte_offset..(byte_offset + byte_length)];
            let component_type = indices_accessor.component_type;
            match component_type {
                ComponentType::U8 => {
                    let indices: &[u8] = bytemuck::cast_slice(bytes);
                    self.index_data_u8 = indices.to_vec();
                },
                ComponentType::U16 => {
                    let indices: &[u16] = bytemuck::cast_slice(bytes);
                    self.index_data_u16 = indices.to_vec();
                },
                ComponentType::U32 => {
                    let indices: &[u32] = bytemuck::cast_slice(bytes);
                    self.index_data_u32 = indices.to_vec();
                },
                _ => panic!("Unsupported index type"),
            }

            let mut positions: &[[f32; 3]] = if let Some(accessor) = position_accessor {
                let accessor_buffer_view = &world_buffer_views[accessor.buffer_view];
                let accessor_buffer_view_buffer = &world_buffers[accessor_buffer_view.buffer];
                byte_offset = accessor_buffer_view.byte_offset;
                byte_length = accessor_buffer_view.byte_length;
                let bytes = &accessor_buffer_view_buffer.data[byte_offset..(byte_offset + byte_length)];
                bytemuck::cast_slice(bytes)
            } else {
                &[]
            };

            let mut normals: &[[f32; 3]] = if let Some(accessor) = normal_accessor {
                let accessor_buffer_view = &world_buffer_views[accessor.buffer_view];
                let accessor_buffer_view_buffer = &world_buffers[accessor_buffer_view.buffer];
                byte_offset = accessor_buffer_view.byte_offset;
                byte_length = accessor_buffer_view.byte_length;
                let bytes = &accessor_buffer_view_buffer.data[byte_offset..(byte_offset + byte_length)];
                bytemuck::cast_slice(bytes)
            } else {
                &[]
            };
            
            let mut tex_coords: &[[f32; 2]] = if let Some(accessor) = texcoord_accessor {
                let accessor_buffer_view = &world_buffer_views[accessor.buffer_view];
                let accessor_buffer_view_buffer = &world_buffers[accessor_buffer_view.buffer];
                byte_offset = accessor_buffer_view.byte_offset;
                byte_length = accessor_buffer_view.byte_length;
                let bytes = &accessor_buffer_view_buffer.data[byte_offset..(byte_offset + byte_length)];
                bytemuck::cast_slice(bytes)
            } else {
                &[]
            };

            let mut joints= Vec::new();
            if let Some(accessor) = joint_accessor {
                let component_type = accessor.component_type;
                let accessor_buffer_view = &world_buffer_views[accessor.buffer_view];
                byte_offset = accessor_buffer_view.byte_offset;
                byte_length = accessor_buffer_view.byte_length;
                let accessor_buffer_view_buffer = &world_buffers[accessor_buffer_view.buffer];
                let bytes = &accessor_buffer_view_buffer.data[byte_offset..(byte_offset + byte_length)];
                match component_type {
                    ComponentType::U8 => {
                        let raw: &[[u8; 4]] = bytemuck::cast_slice(bytes);
                        let converted: Vec<[u32; 4]> = raw.iter().map(|x| [x[0] as u32, x[1] as u32, x[2] as u32, x[3] as u32]).collect();
                        joints = converted;
                    },
                    ComponentType::U16 => {
                        let raw: &[[u16; 4]] = bytemuck::cast_slice(bytes);
                        let converted: Vec<[u32; 4]> = raw.iter().map(|x| [x[0] as u32, x[1] as u32, x[2] as u32, x[3] as u32]).collect();
                        joints = converted;
                    },
                    ComponentType::U32 => {
                        let raw: &[[u32; 4]] = bytemuck::cast_slice(bytes);
                        joints = raw.to_vec();
                    },
                    _ => panic!("Unsupported joint index component type"),
                }
            }

            let mut weights: &[[f32; 4]] = &[];
            if let Some(accessor) = weight_accessor {
                let accessor_buffer_view = &world_buffer_views[accessor.buffer_view];
                byte_offset = accessor_buffer_view.byte_offset;
                byte_length = accessor_buffer_view.byte_length;
                let accessor_buffer_view_buffer = &world_buffers[accessor_buffer_view.buffer];
                let bytes = &accessor_buffer_view_buffer.data[byte_offset..(byte_offset + byte_length)];
                weights = bytemuck::cast_slice(bytes);
            }

            let mut vertices = Vec::new();
            for i in 0..positions.len() {
                vertices.push(RefCell::new(Vertex {
                    position: positions[i],
                    normal: *normals.get(i).unwrap_or(&[0.0; 3]),
                    uv: *tex_coords.get(i).unwrap_or(&[0.0; 2]),
                    tangent: [0.0; 3],
                    bitangent: [0.0; 3],
                    joint_indices: *joints.get(i).unwrap_or(&[0; 4]),
                    joint_weights: *weights.get(i).unwrap_or(&[0.0; 4]),
                }));
            }
            match component_type {
                ComponentType::U8 => {
                    Primitive::construct_tangents(&mut vertices, &self.index_data_u8);
                },
                ComponentType::U16 => {
                    Primitive::construct_tangents(&mut vertices, &self.index_data_u16);
                },
                ComponentType::U32 => {
                    Primitive::construct_tangents(&mut vertices, &self.index_data_u32);
                },
                _ => panic!("Unsupported index type"),
            }
            self.vertex_data = vertices
                .into_iter()
                .map(|v| v.into_inner())
                .collect();
        }
    }

    fn construct_tangents<T: AsUsize>(vertices: &mut Vec<RefCell<Vertex>>, index_data: &Vec<T>) {
        for vertex in vertices.iter() {
            let mut v = vertex.borrow_mut();
            v.tangent = [0.0, 0.0, 0.0];
            v.bitangent = [0.0, 0.0, 0.0];
        }

        for i in (0..index_data.len()).step_by(3) {
            let i0 = index_data[i].as_usize();
            let i1 = index_data[i + 1].as_usize();
            let i2 = index_data[i + 2].as_usize();

            let (e1, e2, delta_uv1, delta_uv2) = {
                let v1 = vertices[i0].borrow();
                let v2 = vertices[i1].borrow();
                let v3 = vertices[i2].borrow();

                let e1 = (
                    v2.position[0] - v1.position[0],
                    v2.position[1] - v1.position[1],
                    v2.position[2] - v1.position[2]
                );
                let e2 = (
                    v3.position[0] - v1.position[0],
                    v3.position[1] - v1.position[1],
                    v3.position[2] - v1.position[2]
                );
                let delta_uv1 = (
                    v2.uv[0] - v1.uv[0],
                    v2.uv[1] - v1.uv[1],
                );
                let delta_uv2 = (
                    v3.uv[0] - v1.uv[0],
                    v3.uv[1] - v1.uv[1],
                );
                (e1, e2, delta_uv1, delta_uv2)
            };

            let denom = delta_uv1.0 * delta_uv2.1 - delta_uv2.0 * delta_uv1.1;

            // skip degen
            if denom.abs() < 1e-6 {
                continue;
            }

            let f = 1.0 / denom;

            let tangent = (
                f * (delta_uv2.1 * e1.0 - delta_uv1.1 * e2.0),
                f * (delta_uv2.1 * e1.1 - delta_uv1.1 * e2.1),
                f * (delta_uv2.1 * e1.2 - delta_uv1.1 * e2.2),
            );
            let bitangent = (
                f * (-delta_uv2.0 * e1.0 + delta_uv1.0 * e2.0),
                f * (-delta_uv2.0 * e1.1 + delta_uv1.0 * e2.1),
                f * (-delta_uv2.0 * e1.2 + delta_uv1.0 * e2.2),
            );

            // accumulate
            for idx in [i0, i1, i2] {
                let mut v = vertices[idx].borrow_mut();
                v.tangent[0] += tangent.0;
                v.tangent[1] += tangent.1;
                v.tangent[2] += tangent.2;
                v.bitangent[0] += bitangent.0;
                v.bitangent[1] += bitangent.1;
                v.bitangent[2] += bitangent.2;
            }
        }

        for vertex in vertices.iter() {
            let mut v = vertex.borrow_mut();

            let tangent_vec = Vector::new3(v.tangent[0], v.tangent[1], v.tangent[2]);
            let bitangent_vec = Vector::new3(v.bitangent[0], v.bitangent[1], v.bitangent[2]);

            let tangent_len_sq = v.tangent[0] * v.tangent[0] +
                v.tangent[1] * v.tangent[1] +
                v.tangent[2] * v.tangent[2];
            let bitangent_len_sq = v.bitangent[0] * v.bitangent[0] +
                v.bitangent[1] * v.bitangent[1] +
                v.bitangent[2] * v.bitangent[2];

            if tangent_len_sq > 1e-6 {
                v.tangent = tangent_vec.normalize3().to_array3();
            } else {
                v.tangent = [1.0, 0.0, 0.0];
            }

            if bitangent_len_sq > 1e-6 {
                v.bitangent = bitangent_vec.normalize3().to_array3();
            } else {
                v.bitangent = [0.0, 1.0, 0.0];
            }
        }
    }

    fn construct_min_max(&mut self) {
        let mut min = Vector::fill(f32::MAX);
        let mut max = Vector::fill(f32::MIN);
        for vertex in self.vertex_data.iter() {
            min = Vector::min(&Vector::from_array(&vertex.position), &min);
            max = Vector::max(&Vector::from_array(&vertex.position), &max);
        }
        self.min = min;
        self.max = max;
        self.corners = [
            self.min,
            Vector::new3(max.x, min.y, min.z),
            Vector::new3(max.x, min.y, max.z),
            Vector::new3(min.x, min.y, max.z),
            Vector::new3(min.x, max.y, min.z),
            Vector::new3(max.x, max.y, min.z),
            self.max,
            Vector::new3(min.x, max.y, max.z),
        ]
    }
}
trait AsUsize {
    fn as_usize(&self) -> usize;
}
impl AsUsize for u8 {
    fn as_usize(&self) -> usize {
        *self as usize
    }
}
impl AsUsize for u16 {
    fn as_usize(&self) -> usize {
        *self as usize
    }
}
impl AsUsize for u32 {
    fn as_usize(&self) -> usize {
        *self as usize
    }
}

#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
    pub joint_indices: [u32; 4],
    pub joint_weights: [f32; 4],
}

pub struct Mesh {
    pub name: String,
    pub primitives: Vec<Primitive>
}
impl Mesh {
    pub fn get_min_max(&self) -> (Vector, Vector) {
        let mut min = Vector::fill(f32::MAX);
        let mut max = Vector::fill(f32::MIN);
        for primitive in self.primitives.iter() {
            if primitive.max.x > max.x { max.x = primitive.max.x; }
            if primitive.max.y > max.y { max.y = primitive.max.y; }
            if primitive.max.z > max.z { max.z = primitive.max.z; }
            if primitive.min.x < min.x { min.x = primitive.min.x; }
            if primitive.min.y < min.y { min.y = primitive.min.y; }
            if primitive.min.z < min.z { min.z = primitive.min.z; }
        }
        (min, max)
    }
}

pub struct Node {
    pub mapped_entity_index: usize,

    pub mesh: Option<usize>,
    pub skin: Option<i32>,
    pub name: String,
    pub rotation: Vector,
    pub scale: Vector,
    pub translation: Vector,

    pub needs_update: bool,

    pub user_rotation: Vector,
    pub user_scale: Vector,
    pub user_translation: Vector,

    pub original_rotation: Vector,
    pub original_scale: Vector,
    pub original_translation: Vector,

    pub local_transform: Matrix,
    pub world_transform: Matrix,
    pub children_indices: Vec<usize>,
}

pub struct Skin {
    name: String,
    inverse_bind_matrices_accessor: usize,
    pub inverse_bind_matrices: Vec<Matrix>,
    pub joint_indices: Vec<usize>,
    joint_matrices: Vec<Matrix>,
    skeleton: Option<usize>,
}
impl Skin {
    pub fn construct_joint_matrices(
        &mut self,
        world_nodes: &mut Vec<Node>,
        world_accessors: &Vec<Accessor>,
        world_buffer_views: &Vec<BufferView>,
        world_buffers: &Vec<Buffer>,
    ) {
        let inverse_bind_matrices_accessor = &self.inverse_bind_matrices_accessor;
        let buffer_view = &world_buffer_views[world_accessors[*inverse_bind_matrices_accessor].buffer_view];
        let byte_offset = buffer_view.byte_offset;
        let byte_length = buffer_view.byte_length;

        let buffer = &world_buffers[buffer_view.buffer];
        let bytes = &buffer.data[byte_offset..(byte_offset + byte_length)];
        let inverse_bind_matrices: &[[f32; 16]] = bytemuck::cast_slice(bytes);
        self.inverse_bind_matrices.clear();
        for matrix_data in inverse_bind_matrices.iter() {
            self.inverse_bind_matrices.push(Matrix::new_manual(matrix_data.clone()));
        }

        self.joint_matrices.clear();
        let mut joint = 0;
        for node_index in self.joint_indices.iter() {
            self.joint_matrices.push(
                world_nodes[*node_index].world_transform * self.inverse_bind_matrices[joint]
            );
            joint += 1
        }
    }

    pub fn update_joint_matrices(&mut self, nodes: &Vec<Node>) {
        self.joint_matrices.clear();
        let mut joint = 0;
        for node_index in self.joint_indices.iter() {
            self.joint_matrices.push(
                nodes[*node_index].world_transform * self.inverse_bind_matrices[joint]
            );
            joint += 1
        }
    }
}

pub struct Animation {
    pub name: String,
    pub channels: Vec<(usize, usize, String)>, // sampler index, impacted node, target transform component
    pub samplers: Vec<(Vec<f32>, String, Vec<Vector>)>, // input times, interpolation method, output vectors
    pub start_time: SystemTime,
    pub duration: f32,
    pub running: bool,
    pub repeat: bool,
    pub snap_back: bool,
}
impl Animation {
    fn new(name: String, world: &World, channels: Vec<(usize, usize, String)>, samplers: Vec<(usize, String, usize)>) -> Self { // samplers are accessors
        let mut compiled_samplers = Vec::new();
        for sampler in samplers.iter() {
            let sampler_accessors = (
                &world.accessors[sampler.0],
                &world.accessors[sampler.2]
            );
            let sampler_buffer_views = (
                &world.buffer_views[sampler_accessors.0.buffer_view],
                &world.buffer_views[sampler_accessors.1.buffer_view]
            );
            let sampler_buffers = (
                &world.buffers[sampler_buffer_views.0.buffer],
                &world.buffers[sampler_buffer_views.1.buffer]
            );
            let mut byte_offset = sampler_buffer_views.0.byte_offset;
            let mut byte_length = sampler_buffer_views.0.byte_length;
            let mut bytes = &sampler_buffers.0.data[byte_offset..(byte_offset + byte_length)];
            let times: &[f32] = bytemuck::cast_slice(bytes);

            byte_offset = sampler_buffer_views.1.byte_offset;
            byte_length = sampler_buffer_views.1.byte_length;
            bytes = &sampler_buffers.1.data[byte_offset..(byte_offset + byte_length)];
            let mut vectors = Vec::new();
            if sampler_accessors.1.r#type.eq("VEC3") {
                let vec3s: &[[f32; 3]] = bytemuck::cast_slice(bytes);
                for vec3 in vec3s.iter() {
                    vectors.push(Vector::new3(vec3[0], vec3[1], vec3[2]));
                }
                compiled_samplers.push((times.to_vec(), sampler.1.clone(), vectors));
            } else if sampler_accessors.1.r#type.eq("VEC4") {
                let vec4s: &[[f32; 4]] = bytemuck::cast_slice(bytes);
                for vec4 in vec4s.iter() {
                    vectors.push(Vector::new4(vec4[0], vec4[1], vec4[2], vec4[3]));
                }
                compiled_samplers.push((times.to_vec(), sampler.1.clone(), vectors));
            } else {
                panic!("Illogical animation sampler output type! Should be VEC3 or VEC4");
            }
        }
        let duration = compiled_samplers.iter()
            .map(|s| *s.0.last().unwrap_or(&0.0))
            .fold(0.0_f32, f32::max);
        Self {
            name,
            channels: channels.iter().map(|channel| (channel.0, channel.1, channel.2.clone())).collect(),
            samplers: compiled_samplers,
            start_time: SystemTime::now(),
            duration,
            running: false,
            repeat: false,
            snap_back: false,
        }
    }
}

pub struct GltfScene {
    pub name: String,
    pub nodes: Vec<usize>,
}

fn resolve_gltf_uri(gltf_path: &str, uri: &str) -> PathBuf {
    let gltf_dir = Path::new(gltf_path).parent().unwrap_or_else(|| Path::new(""));

    let uri_path = Path::new(uri);

    if uri_path.is_absolute() {
        uri_path.to_path_buf()
    } else {
        gltf_dir.join(uri_path).canonicalize().unwrap_or_else(|_| gltf_dir.join(uri_path))
    }
}
