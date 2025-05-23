use std::cell::RefCell;
use std::ffi::c_void;
use std::fs;
use std::rc::Rc;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::time::SystemTime;
use ash::vk;
use ash::vk::{CommandBuffer, DeviceMemory, ImageView, Sampler};
use json::JsonValue;
use crate::matrix::Matrix;
use crate::camera::Frustum;
use crate::vector::Vector;
use crate::vk_helper::{copy_data_to_memory, VkBase};

pub struct Scene {
    pub models: Vec<Model>,

    pub index_buffer: (vk::Buffer, DeviceMemory),
    pub vertex_buffer: (vk::Buffer, DeviceMemory),

    pub instance_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
    pub instance_buffers: Vec<(vk::Buffer, DeviceMemory)>,
    pub instance_buffer_size: u64,

    pub material_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
    pub material_buffers: Vec<(vk::Buffer, DeviceMemory)>,
    pub material_buffer_size: u64,

    pub joints_staging_buffer: (vk::Buffer, DeviceMemory, *mut c_void),
    pub joints_buffers: Vec<(vk::Buffer, DeviceMemory)>,
    pub joints_buffers_size: u64,

    pub instance_data: Vec<Instance>,
    pub dirty_instances: Vec<usize>,
    pub primitive_count: usize,
}
impl Scene {
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            index_buffer: (vk::Buffer::null(), DeviceMemory::null()),
            vertex_buffer: (vk::Buffer::null(), DeviceMemory::null()),
            instance_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
            instance_buffers: Vec::new(),
            instance_buffer_size: 0,
            material_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
            material_buffers: Vec::new(),
            material_buffer_size: 0,
            joints_staging_buffer: (vk::Buffer::null(), DeviceMemory::null(), null_mut()),
            joints_buffers: Vec::new(),
            joints_buffers_size: 0,
            instance_data: Vec::new(),
            dirty_instances: Vec::new(),
            primitive_count: 0,
        }
    }

    pub fn add_model(&mut self, mut model: Model) {
        let mut num_skins = 0i32;
        for scene_model in &self.models {
            num_skins += scene_model.skins.len() as i32;
        }
        for node in model.nodes.iter_mut() {
            if node.skin.is_some() {
               node.skin = Some(node.skin.unwrap() + num_skins)
            }
        }
        self.models.push(model);
    }

    pub unsafe fn initialize(&mut self, base: &VkBase, frames_in_flight: usize, load_textures: bool) { unsafe {
        let mut all_vertices: Vec<Vertex> = vec![];
        let mut all_indices: Vec<u32> = vec![];
        self.primitive_count = 0;
        let mut texture_count = 0;
        let mut materials_send = Vec::new();
        for model in self.models.iter_mut() {
            for mesh in &model.meshes {
                for primitive in &mut mesh.borrow_mut().primitives {
                    primitive.id = self.primitive_count;
                    self.primitive_count += 1;
                    primitive.construct_data();
                    primitive.vertex_buffer_offset = all_vertices.len();
                    primitive.index_buffer_offset = all_indices.len();
                    all_vertices.extend_from_slice(&primitive.vertex_data);
                    if !primitive.index_data_u8.is_empty() {
                        all_indices.extend(
                            primitive.index_data_u8.iter().map(|&i| i as u32 + primitive.vertex_buffer_offset as u32)
                        );
                    } else if !primitive.index_data_u16.is_empty() {
                        all_indices.extend(
                            primitive.index_data_u16.iter().map(|&i| i as u32 + primitive.vertex_buffer_offset as u32)
                        );
                    } else if !primitive.index_data_u32.is_empty() {
                        all_indices.extend(
                            primitive.index_data_u32.iter().map(|&i| i + primitive.vertex_buffer_offset as u32)
                        );
                    }
                    self.instance_data.push(Instance::new(Matrix::new(), primitive.material_index + materials_send.len() as u32, 0));
                    primitive.construct_min_max()
                }
            }
            for material in model.materials.iter() {
                materials_send.push(material.to_sendable(texture_count));
            }
            texture_count += model.textures.len() as i32;
        }
        self.instance_buffer_size = self.primitive_count as u64 * size_of::<Instance>() as u64;
        self.material_buffer_size = materials_send.len() as u64 * size_of::<MaterialSendable>() as u64;
        self.vertex_buffer = base.create_device_and_staging_buffer(0, &*all_vertices, vk::BufferUsageFlags::VERTEX_BUFFER, true, false, true).0;
        self.index_buffer = base.create_device_and_staging_buffer(0, &*all_indices, vk::BufferUsageFlags::INDEX_BUFFER, true, false, true).0;
        for i in 0..frames_in_flight {
            self.instance_buffers.push((vk::Buffer::null(), DeviceMemory::null()));
            self.material_buffers.push((vk::Buffer::null(), DeviceMemory::null()));
            self.joints_buffers.push((vk::Buffer::null(), DeviceMemory::null()));
            if i == 0 {
                (self.instance_buffers[i], self.instance_staging_buffer) =
                    base.create_device_and_staging_buffer(self.instance_buffer_size, &[0], vk::BufferUsageFlags::VERTEX_BUFFER, false, true, false);
                (self.material_buffers[i], self.material_staging_buffer) =
                    base.create_device_and_staging_buffer(0, &materials_send, vk::BufferUsageFlags::STORAGE_BUFFER, false, false, true);
            } else {
                self.instance_buffers[i] = base.create_device_and_staging_buffer(self.instance_buffer_size, &[0], vk::BufferUsageFlags::VERTEX_BUFFER, true, false, false).0;
                self.material_buffers[i] = base.create_device_and_staging_buffer(0, &materials_send, vk::BufferUsageFlags::STORAGE_BUFFER, true, false, true).0;
            }
        }

        self.update_instances_all_frames(base);
        self.joints_buffers_size = 1;
        let mut joints_send = Vec::new();
        for model in self.models.iter_mut() {
            for skin in model.skins.iter_mut() {
                self.joints_buffers_size += size_of::<Matrix>() as u64; // for skin joint offset matrix
                skin.construct_joint_matrices(&model.nodes);
                for joint in skin.joint_matrices.iter() {
                    self.joints_buffers_size += size_of::<Matrix>() as u64;
                    joints_send.push(joint.clone());
                }
            }
        }
        for i in 0..frames_in_flight {
            if i == 0 {
                (self.joints_buffers[i], self.joints_staging_buffer) =
                    base.create_device_and_staging_buffer(self.joints_buffers_size, &joints_send, vk::BufferUsageFlags::STORAGE_BUFFER, false, true, true);
            } else {
                self.joints_buffers[i] = base.create_device_and_staging_buffer(self.joints_buffers_size, &joints_send, vk::BufferUsageFlags::STORAGE_BUFFER, true, false, true).0;
            }
        }

        if load_textures {
            for model in self.models.iter_mut() {
                model.construct_textures(base)
            }
        }
    } }

    pub unsafe fn update_instances(&mut self, base: &VkBase, frame: usize) { unsafe {
        self.dirty_instances.clear();
        for model in self.models.iter_mut() {
            for node in &model.scene.nodes.clone() {
                model.update_node(&mut self.instance_data, &mut self.dirty_instances, *node, &mut Matrix::new(), false);
            }
        }
        if self.dirty_instances.len() > 0 {
            for idx in &self.dirty_instances {
                let dst = (self.instance_staging_buffer.2 as *mut u8).add(idx * size_of::<Instance>());
                std::ptr::copy_nonoverlapping(
                    &self.instance_data[*idx] as *const _ as *const u8,
                    dst,
                    size_of::<Instance>(),
                );
            }
            let copy_regions: Vec<vk::BufferCopy> = self.dirty_instances.iter().map(|&idx| {
                let offset = (idx * size_of::<Instance>()) as u64;
                vk::BufferCopy {
                    src_offset: offset,
                    dst_offset: offset,
                    size: size_of::<Instance>() as u64,
                }
            }).collect();
            let command_buffers = base.begin_single_time_commands(1);
            base.device.cmd_copy_buffer(
                command_buffers[0],
                self.instance_staging_buffer.0,
                self.instance_buffers[frame].0,
                &copy_regions,
            );
            base.end_single_time_commands(command_buffers);
        }
    } }

    pub unsafe fn update_instances_all_frames(&mut self, base: &VkBase) { unsafe {
        for model in self.models.iter_mut() {
            for node in &model.scene.nodes.clone() {
                model.update_node(&mut self.instance_data, &mut self.dirty_instances, *node, &mut Matrix::new(), true);
            }
        }
        copy_data_to_memory(self.instance_staging_buffer.2, &self.instance_data);
        for instance_buffer in &self.instance_buffers {
            base.copy_buffer(&self.instance_staging_buffer.0, &instance_buffer.0, &self.instance_buffer_size);
        }
    } }

    pub unsafe fn update_nodes(&mut self, base: &VkBase, frame: usize) { unsafe {
        for model in self.models.iter_mut() {
            for animation in model.animations.iter_mut() {
                animation.update(&mut model.nodes)
            }
        }
        self.update_instances(base, frame);
        self.update_joints(base, frame);
    } }

    pub unsafe fn update_joints(&mut self, base: &VkBase, frame: usize) { unsafe {
        let mut joints = Vec::new();
        let mut total_skins = 0f32;
        for model in self.models.iter_mut() {
            for skin in &mut model.skins {
                skin.update_joint_matrices(&model.nodes);
                total_skins += 1.0;
            }
        }
        let mut total = 0f32;
        for model in self.models.iter() {
            for skin in model.skins.iter() {
                joints.push(Matrix::new_manual([total_skins + total; 16]));
                total += skin.joint_matrices.len() as f32;
            }
        }
        for model in self.models.iter_mut() {
            for skin in &model.skins {
                for joint in skin.joint_matrices.iter() {
                    joints.push(joint.clone());
                }
            }
        }
        copy_data_to_memory(self.joints_staging_buffer.2, &joints);
        base.copy_buffer(&self.joints_staging_buffer.0, &self.joints_buffers[frame].0, &self.joints_buffers_size);
    }}

    pub unsafe fn draw(&self, base: &VkBase, draw_command_buffer: &CommandBuffer, frame: usize, frustum: &Frustum) { unsafe {
        base.device.cmd_bind_vertex_buffers(
            *draw_command_buffer,
            1,
            &[self.instance_buffers[frame].0],
            &[0],
        );
        base.device.cmd_bind_vertex_buffers(
            *draw_command_buffer,
            0,
            &[self.vertex_buffer.0],
            &[0],
        );
        base.device.cmd_bind_index_buffer(
            *draw_command_buffer,
            self.index_buffer.0,
            0,
            vk::IndexType::UINT32,
        );
        for model in self.models.iter() {
            model.draw(base, draw_command_buffer, frame, frustum);
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

        for joints_buffer in &self.joints_buffers {
            base.device.destroy_buffer(joints_buffer.0, None);
            base.device.free_memory(joints_buffer.1, None);
        }
        base.device.unmap_memory(self.joints_staging_buffer.1);
        base.device.destroy_buffer(self.joints_staging_buffer.0, None);
        base.device.free_memory(self.joints_staging_buffer.1, None);

        base.device.destroy_buffer(self.index_buffer.0, None);
        base.device.free_memory(self.index_buffer.1, None);
        base.device.destroy_buffer(self.vertex_buffer.0, None);
        base.device.free_memory(self.vertex_buffer.1, None);

        for model in &self.models {
            model.cleanup(base);
        }
    } }
}

pub struct Model {
    pub extensions_used: Vec<String>,
    pub scene: Rc<GltfScene>,
    pub scenes: Vec<Rc<GltfScene>>,
    pub animations: Vec<Animation>,
    pub skins: Vec<Skin>,
    pub nodes: Vec<Node>,
    pub meshes: Vec<Rc<RefCell<Mesh>>>,
    pub materials: Vec<Material>,
    pub textures: Vec<Rc<RefCell<Texture>>>,
    pub images: Vec<Rc<RefCell<Image>>>,
    pub accessors: Vec<Rc<Accessor>>,
    pub buffer_views: Vec<Rc<BufferView>>,
    pub buffers: Vec<Rc<Buffer>>,
}
impl Model {
    pub fn new(path: &str) -> Self {
        let json = json::parse(fs::read_to_string(path).expect("failed to load json file").as_str()).expect("json parse error");

        let mut extensions_used = Vec::new();
        for extension in json["extensionsUsed"].members() {
            extensions_used.push(extension.as_str().unwrap().to_string());
        }

        let mut buffers = Vec::new();
        for buffer in json["buffers"].members() {
            buffers.push(
                Rc::new(Buffer::new(
                    resolve_gltf_uri(path, buffer["uri"].as_str().unwrap()),
                    buffer["byteLength"].as_usize().unwrap()
                )))
        }

        let mut buffer_views = Vec::new();
        for buffer_view in json["bufferViews"].members() {
            buffer_views.push(
                Rc::new(BufferView {
                    buffer: buffers[buffer_view["buffer"].as_usize().unwrap()].clone(),
                    byte_length: buffer_view["byteLength"].as_usize().unwrap(),
                    byte_offset: buffer_view["byteOffset"].as_usize().unwrap_or(0),
                    target: buffer_view["target"].as_usize().unwrap_or(0)
                }))
        }

        let mut accessors = Vec::new();
        for accessor in json["accessors"].members() {
            let mut min: Option<Vector> = None;
            let mut max: Option<Vector> = None;
            if let JsonValue::Array(ref min_data) = accessor["min"] {
                if min_data.len() >= 3 {
                    min = Some(Vector::new_vec3(
                        min_data[0].as_f32().unwrap(),
                        min_data[1].as_f32().unwrap(),
                        min_data[2].as_f32().unwrap()));
                }
            }
            if let JsonValue::Array(ref max_data) = accessor["max"] {
                if max_data.len() >= 3 {
                    max = Some(Vector::new_vec3(
                        max_data[0].as_f32().unwrap(), 
                        max_data[1].as_f32().unwrap(), 
                        max_data[2].as_f32().unwrap()));
                }
            }
            accessors.push(
                Rc::new(Accessor {
                    buffer_view: buffer_views[accessor["bufferView"].as_usize().unwrap()].clone(),
                    component_type: ComponentType::from_u32(accessor["componentType"].as_u32().unwrap()).expect("unsupported component type"),
                    count: accessor["count"].as_usize().unwrap(),
                    r#type: accessor["type"].as_str().unwrap().parse().unwrap(),
                    min,
                    max,
                    data: Vec::new(),
                }))
        }

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

            images.push(
                Rc::new(RefCell::new(Image::new(
                    mime_type,
                    name,
                    resolve_gltf_uri(path, image["uri"].as_str().unwrap())
                ))))
        }

        let mut textures = Vec::new();
        for texture in json["textures"].members() {
            textures.push(
                Rc::new(RefCell::new(Texture {
                    source: images[texture["source"].as_usize().unwrap()].clone(),
                    sampler: Sampler::null()
                })))
        }

        let mut materials = Vec::new();
        materials.push(Material {
            alpha_mode: String::from("BLEND"),
            double_sided: false,
            normal_texture: None,
            specular_color_factor: [1.0; 3],
            ior: 1.0,
            name: String::from("default material"),
            base_color_factor: [1.0; 4],
            base_color_texture: None,
            metallic_factor: 0.1,
            metallic_texture: None,
            roughness_factor: 0.5,
            roughness_texture: None,
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

            let mut double_sided = false;
            if let JsonValue::Boolean(ref double_sided_json) = material["doubleSided"] {
                double_sided = *double_sided_json;
            }

            let mut normal_texture = None;
            if let JsonValue::Object(ref normal_texture_json) = material["normalTexture"] {
                normal_texture = Some(normal_texture_json["index"].as_i32().expect(""));
            }

            let mut base_color_factor = [0.5, 0.5, 0.5, 1.0];
            let mut base_color_texture = None;
            let mut metallic_factor = 0.1;
            let mut roughness_factor = 0.5;
            let mut metallic_texture = None;
            let mut roughness_texture = None;
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
                if let JsonValue::Object(ref json_value) = pbr_metallic_roughness["baseColorTexture"] {
                    base_color_texture = Some(json_value["index"].as_i32().expect("FAULTY GLTF: \n    Missing index for baseColorTexture at pbrMetallicRoughness"));
                }

                if let JsonValue::Number(ref json_value) = pbr_metallic_roughness["metallicFactor"] {
                    if let Ok(f) = json_value.to_string().parse::<f32>() {
                        metallic_factor = f;
                    }
                }
                if let JsonValue::Object(ref json_value) = pbr_metallic_roughness["metallicTexture"] {
                    metallic_texture = Some(json_value["index"].as_i32().expect("FAULTY GLTF: \n    Missing index for metallicTexture at pbrMetallicRoughness"));
                }

                if let JsonValue::Number(ref json_value) = pbr_metallic_roughness["roughnessFactor"] {
                    if let Ok(f) = json_value.to_string().parse::<f32>() {
                        roughness_factor = f;
                    }
                }
                if let JsonValue::Object(ref json_value) = pbr_metallic_roughness["roughnessTexture"] {
                    roughness_texture = Some(json_value["index"].as_i32().expect("FAULTY GLTF: \n    Missing index for roughnessTexture at pbrMetallicRoughness"));
                }

                if let JsonValue::Object(ref json_value) = pbr_metallic_roughness["metallicRoughnessTexture"] {
                    roughness_texture = Some(json_value["index"].as_i32().expect("FAULTY GLTF: \n    Missing index for metallicRoughnessTexture at pbrMetallicRoughness"));
                    metallic_texture = Some(json_value["index"].as_i32().expect("FAULTY GLTF: \n    Missing index for metallicRoughnessTexture at pbrMetallicRoughness"));
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

            materials.push(
                Material {
                    name,
                    alpha_mode,
                    double_sided,
                    normal_texture,
                    // KHR_materials_specular
                        specular_color_factor,
                    // KHR_materials_ior
                        ior,
                    // pbrMetallicRoughness
                        base_color_factor,
                        base_color_texture,
                        metallic_factor,
                        metallic_texture,
                        roughness_factor,
                        roughness_texture,
                })
        }

        let mut primitive_count = 0usize;
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
                    let mut attributes: Vec<(String, Rc<Accessor>)> = Vec::new();
                    if let JsonValue::Object(ref attributes_json) = primitive_json["attributes"] {
                        for (name, accessor) in attributes_json.iter() {
                            attributes.push((name.to_string(), accessors[accessor.as_usize().unwrap()].clone()));
                        }
                    }

                    let indices = accessors[primitive_json["indices"].as_usize().unwrap()].clone();

                    let material_index_maybe: Option<u32> = primitive_json["material"].as_u32();
                    let mut material_index = 0u32;
                    match material_index_maybe {
                        Some(material_index_value) => material_index = material_index_value + 1,
                        None => (),
                    }
                    primitives.push(Primitive {
                        attributes,
                        indices_count: indices.count,
                        indices,
                        index_buffer_offset: 0,
                        vertex_buffer_offset: 0,
                        index_data_u8: Vec::new(),
                        index_data_u16: Vec::new(),
                        index_data_u32: Vec::new(),
                        vertex_data: Vec::new(),
                        material_index,
                        min: Vector::new_null(),
                        max: Vector::new_null(),
                        corners: [Vector::new_null(); 8],
                        id: primitive_count,
                    });
                    primitive_count += 1;
                }
            }

            meshes.push(
                Rc::new(RefCell::new(Mesh {
                    name,
                    primitives,
                }))
            );
        }

        let mut nodes = Vec::new();
        for node in json["nodes"].members() {
            let name_maybe: Option<&str> = node["name"].as_str();
            let mut name = String::from("unnamed node");
            match name_maybe {
                Some(name_str) => name = String::from(name_str),
                None => (),
            }

            let mesh_maybe: Option<usize> = node["mesh"].as_usize();
            let mut mesh = None;
            match mesh_maybe {
                Some(mesh_index) => mesh = Some(meshes[mesh_index].clone()),
                None => (),
            }

            let skin_maybe: Option<usize> = node["skin"].as_usize();
            let mut skin = None;
            match skin_maybe {
                Some(skin_index) => skin = Some(skin_index as i32),
                None => (),
            }

            let mut rotation = Vector::new_empty();
            if let JsonValue::Array(ref rotation_json) = node["rotation"] {
                if rotation_json.len() >= 4 {
                    rotation = Vector::new_vec4(
                        rotation_json[0].as_f32().unwrap(),
                        rotation_json[1].as_f32().unwrap(),
                        rotation_json[2].as_f32().unwrap(),
                        rotation_json[3].as_f32().unwrap()
                    ).normalize_4d();
                };
            }

            let mut scale = Vector::new_vec(1.0);
            if let JsonValue::Array(ref scale_json) = node["scale"] {
                if scale_json.len() >= 3 {
                    scale = Vector::new_vec3(
                        scale_json[0].as_f32().unwrap(),
                        scale_json[1].as_f32().unwrap(),
                        scale_json[2].as_f32().unwrap()
                    );
                }
            }

            let mut translation = Vector::new_empty();
            if let JsonValue::Array(ref translation_json) = node["translation"] {
                if translation_json.len() >= 3 {
                    translation = Vector::new_vec3(
                        translation_json[0].as_f32().unwrap(),
                        translation_json[1].as_f32().unwrap(),
                        translation_json[2].as_f32().unwrap()
                    );
                }
            }

            let mut children_indices = Vec::new();
            if let JsonValue::Array(ref children_json) = node["children"] {
                for child_json in children_json {
                    children_indices.push(child_json.as_usize().unwrap());
                }
            }

            nodes.push(
                Node {
                    name,
                    mesh,
                    skin,
                    rotation,
                    scale,
                    translation,
                    needs_update: true,
                    user_rotation: Vector::new_empty(),
                    user_scale: Vector::new_vec(1.0),
                    user_translation: Vector::new_empty(),
                    original_rotation: rotation,
                    original_scale: scale,
                    original_translation: translation,
                    local_transform: Matrix::new_empty(),
                    world_transform: Matrix::new_empty(),
                    children_indices,
                }
            )
        }

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
                    joint_indices.push(joint.as_usize().unwrap());
                }
            }

            let inverse_bind_matrices_accessor = accessors[skin["inverseBindMatrices"].as_usize().unwrap()].clone();

            let mut skeleton: Option<usize> = None;
            match skin["skeleton"].as_usize() {
                Some(skeleton_idx) => skeleton = Some(skeleton_idx),
                None => (),
            }

            skins.push(Skin {
                name,
                inverse_bind_matrices_accessor,
                inverse_bind_matrices: Vec::new(),
                joint_indices,
                joint_matrices: Vec::new(),
                skeleton,
            })
        }

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
                        channel["sampler"].as_usize().unwrap(),
                        channel["target"]["node"].as_usize().unwrap(),
                        String::from(channel["target"]["path"].as_str().unwrap())
                    ))
                }
            }

            let mut samplers = Vec::new();
            if let JsonValue::Array(ref samplers_json) = animation["samplers"] {
                for sampler_json in samplers_json {
                    samplers.push((
                        accessors[sampler_json["input"].as_usize().unwrap()].clone(),
                        String::from(sampler_json["interpolation"].as_str().unwrap()),
                        accessors[sampler_json["output"].as_usize().unwrap()].clone()
                    ))
                }
            }

            animations.push(Animation::new(
                name,
                channels,
                samplers,
            ))
        }

        let mut scenes = Vec::new();
        for scene in json["scenes"].members() {
            let name_maybe: Option<&str> = scene["name"].as_str();
            let mut name = String::from("unnamed scene");
            match name_maybe {
                Some(name_str) => name = String::from(name_str),
                None => (),
            }

            let mut scene_nodes = Vec::new();
            if let JsonValue::Array(ref nodes_json) = scene["nodes"] {
                for node_json in nodes_json {
                    scene_nodes.push(node_json.as_usize().unwrap());
                }
            }

            scenes.push(
                Rc::new(GltfScene {
                    name,
                    nodes: scene_nodes,
                })
            )
        }

        let scene = scenes[json["scene"].as_usize().unwrap_or(0)].clone();



        Self {
            extensions_used,
            scene,
            scenes,
            animations,
            skins,
            nodes,
            meshes,
            materials,
            textures,
            images,
            accessors,
            buffer_views,
            buffers,
        }
    }

    pub fn transform_roots(&mut self, translation: &Vector, rotation: &Vector, scale: &Vector) {
        for node_index in self.scene.nodes.iter() {
            let node = &mut self.nodes[*node_index];
            node.user_translation.add_vec_to_self(translation);
            node.user_rotation.combine_to_self(&rotation.normalize_4d());
            node.user_scale.mul_by_vec_to_self(scale);
            node.needs_update = true;
        }
    }

    pub unsafe fn construct_textures(&mut self, base: &VkBase) { unsafe {
        for i in 0..self.images.len() {
            let mut image = self.images[i].borrow_mut();
            print!("\rloading image {}/{}, {:?}",i,self.images.len(), image.name);
            image.construct_image_view(base);
        }
        for texture in &mut self.textures {
            texture.borrow_mut().construct_sampler(base);
        }
        println!();
    }}

    pub fn update_node(&mut self, instances: &mut Vec<Instance>, dirty_instances: &mut Vec<usize>, node_index: usize, parent_transform: &Matrix, parent_needs_update: bool) {
        let (transform, children_indices, needs_update) = {
            let node = &mut self.nodes[node_index];
            let mut node_needs_update = false;
            if node.needs_update {
                node.needs_update = false;
                node_needs_update = true;
                node.update_local_transform();
                node.update_world_transform(parent_transform);
                node.update_instances(instances, dirty_instances, true);
            } else if parent_needs_update {
                node.update_world_transform(parent_transform);
                node.update_instances(instances, dirty_instances, true);
            }
            let node = &self.nodes[node_index];
            (&node.world_transform.clone(), &node.children_indices.clone(), node_needs_update)
        };
        for child in children_indices {
            self.update_node(instances, dirty_instances, *child, transform, parent_needs_update || needs_update);
        }
    }

    pub unsafe fn draw(&self, base: &VkBase, draw_command_buffer: &CommandBuffer, frame: usize, frustum: &Frustum) { unsafe {
        for node_index in self.scene.nodes.iter() {
            let node = &self.nodes[*node_index];
            node.draw(base, &self, &draw_command_buffer, frustum)
        }
    } }

    pub unsafe fn cleanup(&self, base: &VkBase) { unsafe {
        for texture in &self.textures {
            let texture = texture.borrow();
            base.device.destroy_sampler(texture.sampler, None);
        }
        for image in &self.images {
            let image = image.borrow();
            base.device.destroy_image_view(image.image_view, None);
            base.device.destroy_image(image.image.0, None);
            base.device.free_memory(image.image.1, None);
        }
    } }
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
    pub buffer: Rc<Buffer>,
    pub byte_length: usize,
    pub byte_offset: usize,
    pub target: usize,
}

pub struct Accessor {
    pub buffer_view: Rc<BufferView>,
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

    unsafe fn construct_image_view(&mut self, base: &VkBase) { unsafe {
        let (image_view, image, mips) = base.create_2d_texture_image(&self.uri, true);
        self.image = image;
        self.image_view = image_view.0;
        self.mip_levels = mips;
        self.generated = true;
        base.device.destroy_sampler(image_view.1, None);
    } }
}

pub struct Texture {
    pub source: Rc<RefCell<Image>>,

    pub sampler: Sampler,
}
impl Texture {
    pub unsafe fn construct_sampler(&mut self, base: &VkBase) { unsafe {
        let sampler_info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            anisotropy_enable: vk::TRUE,
            max_anisotropy: base.pdevice_properties.limits.max_sampler_anisotropy,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            mip_lod_bias: 0.0,
            min_lod: 0.0,
            max_lod: self.source.borrow().mip_levels as f32,
            ..Default::default()
        };
        self.sampler = base.device.create_sampler(&sampler_info, None).expect("failed to create sampler");
    } }
}
pub struct Material {
    pub alpha_mode: String,
    pub double_sided: bool,
    pub normal_texture: Option<i32>,
    // KHR_materials_specular
        pub specular_color_factor: [f32; 3],
    // KHR_materials_ior
        pub ior: f32,
    pub name: String,
    // pbrMetallicRoughness
        pub base_color_factor: [f32; 4],
        pub base_color_texture: Option<i32>,
        pub metallic_factor: f32,
        pub metallic_texture: Option<i32>,
        pub roughness_factor: f32,
        pub roughness_texture: Option<i32>,
}
impl Material {
    fn to_sendable(&self, texture_offset: i32) -> MaterialSendable {
        MaterialSendable {
            normal_texture: self.normal_texture.map(|val| val + texture_offset).unwrap_or(-1),
            _pad0: [0; 3],
            specular_color_factor: self.specular_color_factor,
            _pad1: 0,
            ior: self.ior,
            _pad2: [0; 3],
            base_color_factor: self.base_color_factor,
            base_color_texture: self.base_color_texture.map(|val| val + texture_offset).unwrap_or(-1),
            roughness_factor: self.roughness_factor,
            roughness_texture: self.roughness_texture.map(|val| val + texture_offset).unwrap_or(-1),
            metallic_factor: self.metallic_factor,
            metallic_texture: self.metallic_texture.map(|val| val + texture_offset).unwrap_or(-1),
            _pad3: [0u32; 3],
        }
    }
}
#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct MaterialSendable {
    pub normal_texture: i32,
    pub _pad0: [u32; 3],

    pub specular_color_factor: [f32; 3],
    pub _pad1: u32,

    pub ior: f32,
    pub _pad2: [u32; 3],

    pub base_color_factor: [f32; 4],

    pub base_color_texture: i32,
    pub metallic_factor: f32,
    pub metallic_texture: i32,
    pub roughness_factor: f32,

    pub roughness_texture: i32,
    pub _pad3: [u32; 3],
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
    pub attributes: Vec<(String, Rc<Accessor>)>,
    pub indices: Rc<Accessor>,

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
    fn construct_data(&mut self) {
        let mut position_accessor: Option<&Rc<Accessor>> = None;
        let mut normal_accessor: Option<&Rc<Accessor>> = None;
        let mut texcoord_accessor: Option<&Rc<Accessor>> = None;
        let mut joint_accessor: Option<&Rc<Accessor>> = None;
        let mut weight_accessor: Option<&Rc<Accessor>> = None;
        for attribute in self.attributes.iter() {
            if attribute.0.eq("POSITION") {
                position_accessor = Some(&attribute.1);
            } else if attribute.0.eq("NORMAL") {
                normal_accessor = Some(&attribute.1);
            } else if attribute.0.eq("TEXCOORD_0") {
                texcoord_accessor = Some(&attribute.1);
            } else if attribute.0.eq("JOINTS_0") {
                joint_accessor = Some(&attribute.1);
            } else if attribute.0.eq("WEIGHTS_0") {
                weight_accessor = Some(&attribute.1);
            }
        }
        if position_accessor.is_none() {
            println!("Primitive has no POSITION attribute!");
        } else {
            let indices_accessor = self.indices.clone();
            let mut byte_offset = indices_accessor.buffer_view.byte_offset;
            let mut byte_length = indices_accessor.buffer_view.byte_length;
            let bytes = &indices_accessor.buffer_view.buffer.data[byte_offset..(byte_offset + byte_length)];
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

            byte_offset = position_accessor.unwrap().buffer_view.byte_offset;
            byte_length = position_accessor.unwrap().buffer_view.byte_length;
            let bytes = &position_accessor.unwrap().buffer_view.buffer.data[byte_offset..(byte_offset + byte_length)];
            let positions: &[[f32; 3]] = bytemuck::cast_slice(bytes);

            let mut normals: &[[f32; 3]] = &[];
            if !normal_accessor.is_none() {
                byte_offset = normal_accessor.unwrap().buffer_view.byte_offset;
                byte_length = normal_accessor.unwrap().buffer_view.byte_length;
                let bytes = &normal_accessor.unwrap().buffer_view.buffer.data[byte_offset..(byte_offset + byte_length)];
                normals = bytemuck::cast_slice(bytes);
            }

            let mut tex_coords: &[[f32; 2]] = &[];
            if !texcoord_accessor.is_none() {
                byte_offset = texcoord_accessor.unwrap().buffer_view.byte_offset;
                byte_length = texcoord_accessor.unwrap().buffer_view.byte_length;
                let bytes = &texcoord_accessor.unwrap().buffer_view.buffer.data[byte_offset..(byte_offset + byte_length)];
                tex_coords = bytemuck::cast_slice(bytes);
            }

            let mut joints= Vec::new();
            if !joint_accessor.is_none() {
                let component_type = joint_accessor.unwrap().component_type;
                byte_offset = joint_accessor.unwrap().buffer_view.byte_offset;
                byte_length = joint_accessor.unwrap().buffer_view.byte_length;
                let bytes = &joint_accessor.unwrap().buffer_view.buffer.data[byte_offset..(byte_offset + byte_length)];
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
            if !weight_accessor.is_none() {
                byte_offset = weight_accessor.unwrap().buffer_view.byte_offset;
                byte_length = weight_accessor.unwrap().buffer_view.byte_length;
                let bytes = &weight_accessor.unwrap().buffer_view.buffer.data[byte_offset..(byte_offset + byte_length)];
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
        for i in (0..index_data.len()).step_by(3) {
            let i0 = index_data[i].as_usize();
            let i1 = index_data[i + 1].as_usize();
            let i2 = index_data[i + 2].as_usize();
            let v1 = &mut vertices[i0].borrow_mut();
            let v2 = &mut vertices[i1].borrow_mut();
            let v3 = &mut vertices[i2].borrow_mut();

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
            let f = 1.0 / (delta_uv1.0 * delta_uv2.1 - delta_uv2.0 * delta_uv1.1);
            let mut tangent = Vector::new_vec3(
                f * (delta_uv2.1 * e1.0 - delta_uv1.1 * e2.0),
                f * (delta_uv2.1 * e1.1 + delta_uv1.1 * e2.1),
                f * (delta_uv2.1 * e2.2 - delta_uv1.1 * e2.2),
            );
            let mut bitangent = Vector::new_vec3(
                f * (-delta_uv2.0 * e1.0 - delta_uv1.0 * e2.0),
                f * (-delta_uv2.0 * e1.1 + delta_uv1.0 * e2.1),
                f * (-delta_uv2.0 * e2.2 - delta_uv1.0 * e2.2),
            );
            tangent.normalize_self_3d();
            bitangent.normalize_self_3d();

            v1.tangent = tangent.to_array3();
            v2.tangent = tangent.to_array3();
            v3.tangent = tangent.to_array3();

            v1.bitangent = bitangent.to_array3();
            v2.bitangent = bitangent.to_array3();
            v3.bitangent = bitangent.to_array3();
        }
    }

    fn construct_min_max(&mut self) {
        let mut min = Vector::new_vec(f32::MAX);
        let mut max = Vector::new_vec(f32::MIN);
        for vertex in self.vertex_data.iter() {
            min = Vector::min(&Vector::new_from_array(&vertex.position), &min);
            max = Vector::max(&Vector::new_from_array(&vertex.position), &max);
        }
        self.min = min;
        self.max = max;
        self.corners = [
            self.min,
            Vector::new_vec3(max.x, min.y, min.z),
            Vector::new_vec3(max.x, min.y, max.z),
            Vector::new_vec3(min.x, min.y, max.z),
            Vector::new_vec3(min.x, max.y, min.z),
            Vector::new_vec3(max.x, max.y, min.z),
            self.max,
            Vector::new_vec3(min.x, max.y, max.z),
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
#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct Instance {
    pub matrix: [f32; 16],
    pub indices: [i32; 2],
}
impl Instance {
    pub fn new(matrix: Matrix, material: u32, skin: i32) -> Self {
        Self {
            matrix: matrix.data,
            indices: [material as i32, skin],
        }
    }
}

pub struct Mesh {
    pub name: String,
    pub primitives: Vec<Primitive>
}

pub struct Node {
    pub mesh: Option<Rc<RefCell<Mesh>>>,
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
impl Node {
    pub unsafe fn draw(&self, base: &VkBase, owner: &Model, draw_command_buffer: &CommandBuffer, frustum: &Frustum) { unsafe {
        if self.mesh.is_some() {
            for primitive in self.mesh.as_ref().unwrap().borrow().primitives.iter() {
                let mut all_points_outside_of_same_plane = false;

                for plane_idx in 0..6 {
                    let mut all_outside_this_plane = true;

                    for corner in primitive.corners.iter() {
                        let world_pos = self.world_transform.mul_vector4(&Vector::new_vec4(corner.x, corner.y, corner.z, 1.0));

                        if frustum.planes[plane_idx].test_point_within(&world_pos) {
                            all_outside_this_plane = false;
                            break;
                        }
                    }
                    if all_outside_this_plane {
                        all_points_outside_of_same_plane = true;
                        break;
                    }
                }
                if !all_points_outside_of_same_plane {
                    base.device.cmd_draw_indexed(
                        *draw_command_buffer,
                        primitive.indices.count as u32,
                        1,
                        primitive.index_buffer_offset as u32,
                        0,
                        primitive.id as u32,
                    );
                }
            }
        }

        for child in &self.children_indices {
            owner.nodes[*child].draw(base, &owner, draw_command_buffer, frustum);
        }
    } }

    pub fn update_local_transform(&mut self) {
        let rotate = Matrix::new_rotate_quaternion_vec4(&self.rotation.combine(&self.user_rotation.euler_to_quat()));
        let scale = Matrix::new_scale_vec3(&self.scale.mul_by_vec(&self.user_scale));
        let translate = Matrix::new_translation_vec3(&self.translation.add_vec(&self.user_translation));

        self.local_transform = Matrix::new();
        self.local_transform.set_and_mul_mat4(&translate);
        self.local_transform.set_and_mul_mat4(&rotate);
        self.local_transform.set_and_mul_mat4(&scale);
    }

    pub fn update_world_transform(&mut self, parent_transform: &Matrix) {
        self.world_transform = parent_transform.mul_mat4(&self.local_transform);
    }

    pub fn update_instances(&self, instances: &mut Vec<Instance>, dirty_instances: &mut Vec<usize>, add_dirty: bool) {
        if let Some(mesh) = &self.mesh {
            for primitive in mesh.borrow().primitives.iter() {
                instances[primitive.id].matrix = self.world_transform.data;
                instances[primitive.id].indices[1] = self.skin.unwrap_or(-1);
                dirty_instances.push(primitive.id);
            }
        }
    }
}

pub struct Skin {
    name: String,
    inverse_bind_matrices_accessor: Rc<Accessor>,
    inverse_bind_matrices: Vec<Matrix>,
    joint_indices: Vec<usize>,
    joint_matrices: Vec<Matrix>,
    skeleton: Option<usize>,
}
impl Skin {
    pub fn construct_joint_matrices(&mut self, nodes: &Vec<Node>) {
        let inverse_bind_matrices_accessor = &self.inverse_bind_matrices_accessor;
        let byte_offset = inverse_bind_matrices_accessor.buffer_view.byte_offset;
        let byte_length = inverse_bind_matrices_accessor.buffer_view.byte_length;
        let bytes = &inverse_bind_matrices_accessor.buffer_view.buffer.data[byte_offset..(byte_offset + byte_length)];
        let inverse_bind_matrices: &[[f32; 16]] = bytemuck::cast_slice(bytes);
        self.inverse_bind_matrices.clear();
        for matrix_data in inverse_bind_matrices.iter() {
            self.inverse_bind_matrices.push(Matrix::new_manual(matrix_data.clone()));
        }

        self.joint_matrices.clear();
        let mut joint = 0;
        for node_index in self.joint_indices.iter() {
            self.joint_matrices.push(
                nodes[*node_index].world_transform.clone().
                    mul_mat4(&self.inverse_bind_matrices[joint])
            );
            joint += 1
        }
    }

    pub fn update_joint_matrices(&mut self, nodes: &Vec<Node>) {
        self.joint_matrices.clear();
        let mut joint = 0;
        for node_index in self.joint_indices.iter() {
            self.joint_matrices.push(
                nodes[*node_index].world_transform.clone().
                    mul_mat4(&self.inverse_bind_matrices[joint])
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
    fn new(name: String, channels: Vec<(usize, usize, String)>, samplers: Vec<(Rc<Accessor>, String, Rc<Accessor>)>) -> Self {
        let mut compiled_samplers = Vec::new();
        for sampler in samplers.iter() {
            let mut byte_offset = sampler.0.buffer_view.byte_offset;
            let mut byte_length = sampler.0.buffer_view.byte_length;
            let mut bytes = &sampler.0.buffer_view.buffer.data[byte_offset..(byte_offset + byte_length)];
            let times: &[f32] = bytemuck::cast_slice(bytes);

            byte_offset = sampler.2.buffer_view.byte_offset;
            byte_length = sampler.2.buffer_view.byte_length;
            bytes = &sampler.2.buffer_view.buffer.data[byte_offset..(byte_offset + byte_length)];
            let mut vectors = Vec::new();
            if sampler.2.r#type.eq("VEC3") {
                let vec3s: &[[f32; 3]] = bytemuck::cast_slice(bytes);
                for vec3 in vec3s.iter() {
                    vectors.push(Vector::new_vec3(vec3[0], vec3[1], vec3[2]));
                }
                compiled_samplers.push((times.to_vec(), sampler.1.clone(), vectors));
            } else if sampler.2.r#type.eq("VEC4") {
                let vec4s: &[[f32; 4]] = bytemuck::cast_slice(bytes);
                for vec4 in vec4s.iter() {
                    vectors.push(Vector::new_vec4(vec4[0], vec4[1], vec4[2], vec4[3]));
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

    pub fn start(&mut self) {
        self.start_time = SystemTime::now();
        self.running = true;
    }

    pub fn stop(&mut self, nodes: &mut Vec<Node>) {
        self.running = false;
        if self.snap_back {
            for channel in self.channels.iter() {
                if channel.2 == "translation" {
                    let value = nodes[channel.1].original_translation.clone();
                    nodes[channel.1].translation = value;
                } else if channel.2 == "rotation" {
                    let value = nodes[channel.1].original_rotation.clone();
                    nodes[channel.1].rotation = value;
                } else if channel.2 == "scale" {
                    let value = nodes[channel.1].original_scale.clone();
                    nodes[channel.1].scale = value;
                }
            }
        }
    }

    pub fn update(&mut self, nodes: &mut Vec<Node>) {
        if !self.running {
             return
        }
        let current_time = SystemTime::now();
        let elapsed_time = current_time.duration_since(self.start_time).unwrap().as_secs_f32();
        let mut repeat = false;
        if elapsed_time > self.duration {
            if self.repeat {
                repeat = true
            } else {
                self.stop(nodes);
                return
            }
        }
        for channel in self.channels.iter() {
            let sampler = &self.samplers[channel.0];
            let mut current_time_index = 0;
            for i in 0..sampler.0.len() - 1 {
                if elapsed_time >= sampler.0[i] && elapsed_time < sampler.0[i + 1] {
                    current_time_index = i;
                    break
                }
            }
            let current_time_index = current_time_index.min(sampler.0.len() - 1);
            let interpolation_factor = ((elapsed_time - sampler.0[current_time_index]) / (sampler.0[current_time_index + 1] - sampler.0[current_time_index])).min(1.0).max(0.0);
            let vector1 = &sampler.2[current_time_index];
            let vector2 = &sampler.2[current_time_index + 1];
            let new_vector;
            if channel.2.eq("translation") || channel.2.eq("scale") {
                new_vector = Vector::new_vec3(
                    vector1.x + interpolation_factor * (vector2.x - vector1.x),
                    vector1.y + interpolation_factor * (vector2.y - vector1.y),
                    vector1.z + interpolation_factor * (vector2.z - vector1.z),
                )
            } else {
                new_vector = Vector::spherical_lerp(vector1, vector2, interpolation_factor)
            }

            if channel.2.eq("translation") {
                nodes[channel.1].translation = new_vector
            } else if channel.2.eq("rotation") {
                nodes[channel.1].rotation = new_vector
            } else if channel.2.eq("scale") {
                nodes[channel.1].scale = new_vector
            } else {
                panic!("Illogical animation channel target! Should be translation, rotation or scale");
            }
            nodes[channel.1].needs_update = true;
        }
        if repeat {
            self.start()
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
