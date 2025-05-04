use std::cell::RefCell;
use std::fs;
use std::rc::Rc;
use std::io::Read;
use std::path::{Path, PathBuf};
use ash::vk;
use ash::vk::{CommandBuffer, DeviceMemory};
use json::JsonValue;
use winit::dpi::Position;
use crate::matrix::Matrix;
use crate::vector::Vector;
use crate::vk_helper::{copy_data_to_memory, VkBase};

pub struct Gltf {
    pub json: JsonValue,
    pub extensions_used: Vec<String>,
    pub scene: Rc<Scene>,
    pub scenes: Vec<Rc<Scene>>,
    pub nodes: Vec<Rc<RefCell<Node>>>,
    pub meshes: Vec<Rc<RefCell<Mesh>>>,
    pub materials: Vec<Rc<Material>>,
    pub textures: Vec<Rc<Texture>>,
    pub images: Vec<Rc<Image>>,
    pub accessors: Vec<Rc<Accessor>>,
    pub buffer_views: Vec<Rc<BufferView>>,
    pub buffers: Vec<Rc<Buffer>>,
}
impl Gltf {
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
                    byte_offset: buffer_view["byteOffset"].as_usize().unwrap(),
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
            images.push(
                Rc::new(Image {
                    mime_type: image["mimeType"].as_str().unwrap().to_string(),
                    name: image["name"].as_str().unwrap().to_string(),
                    uri: resolve_gltf_uri(path, image["uri"].as_str().unwrap())
                }))
        }

        let mut textures = Vec::new();
        for texture in json["textures"].members() {
            textures.push(
                Rc::new(Texture {
                   source: images[texture["source"].as_usize().unwrap()].clone(),
                }))
        }

        let mut materials = Vec::new();
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
                normal_texture = Some(textures[normal_texture_json["index"].as_usize().expect("")].clone());
            }

            let mut base_color_factor = Vector::new_vec4(0.5, 0.5, 0.5, 1.0);
            let mut base_color_texture = None;
            let mut metallic_factor = 0.1;
            let mut roughness_factor = 0.5;
            let mut metallic_texture = None;
            let mut roughness_texture = None;
            if let JsonValue::Object(ref pbr_metallic_roughness) = material["pbrMetallicRoughness"] {
                if let JsonValue::Array(ref json_value) = pbr_metallic_roughness["baseColorFactor"] {
                    if json_value.len() >= 4 {
                        base_color_factor = Vector::new_vec4(
                            json_value[0].as_f32().unwrap(),
                            json_value[1].as_f32().unwrap(),
                            json_value[2].as_f32().unwrap(),
                            json_value[3].as_f32().unwrap(),
                        );
                    }
                }
                if let JsonValue::Object(ref json_value) = pbr_metallic_roughness["baseColorTexture"] {
                    base_color_texture = Some(textures[json_value["index"].as_usize().expect("FAULTY GLTF: \n    Missing index for baseColorTexture at pbrMetallicRoughness")].clone());
                }

                if let JsonValue::Number(ref json_value) = pbr_metallic_roughness["metallicFactor"] {
                    if let Ok(f) = json_value.to_string().parse::<f32>() {
                        metallic_factor = f;
                    }
                }
                if let JsonValue::Object(ref json_value) = pbr_metallic_roughness["metallicTexture"] {
                    metallic_texture = Some(textures[json_value["index"].as_usize().expect("FAULTY GLTF: \n    Missing index for metallicTexture at pbrMetallicRoughness")].clone());
                }

                if let JsonValue::Number(ref json_value) = pbr_metallic_roughness["roughnessFactor"] {
                    if let Ok(f) = json_value.to_string().parse::<f32>() {
                        roughness_factor = f;
                    }
                }
                if let JsonValue::Object(ref json_value) = pbr_metallic_roughness["roughnessTexture"] {
                    roughness_texture = Some(textures[json_value["index"].as_usize().expect("FAULTY GLTF: \n    Missing index for roughnessTexture at pbrMetallicRoughness")].clone());
                }
            }

            let mut specular_color_factor = Vector::new_vec(1.0);
            if let JsonValue::Object(ref khr_materials_specular) = material["KHR_materials_specular"] {
                if let JsonValue::Array(ref json_val) = khr_materials_specular["baseColorFactor"] {
                    if json_val.len() >= 3 {
                        specular_color_factor = Vector::new_vec3(
                            json_val[0].as_f32().unwrap(),
                            json_val[1].as_f32().unwrap(),
                            json_val[2].as_f32().unwrap(),
                        );
                    }
                }
            }

            let mut ior = 1.0;
            let mut specular_color_factor = Vector::new_vec(1.0);
            if let JsonValue::Object(ref khr_materials_ior) = material["KHR_materials_ior"] {
                if let JsonValue::Number(ref json_value) = khr_materials_ior["ior"] {
                    if let Ok(f) = json_value.to_string().parse::<f32>() {
                        ior = f;
                    }
                }
            }

            materials.push(
                Rc::new(Material {
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
                }))
        }

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

                    let mut indices = accessors[primitive_json["indices"].as_usize().unwrap()].clone();

                    let mut material_index = 0;
                    if let JsonValue::Object(ref material_json) = primitive_json["material"] {
                        material_index = material_json["material"].as_usize().unwrap()
                    }
                    let material = Some(materials[material_index].clone());

                    primitives.push(Primitive {
                        attributes,
                        indices,
                        material,
                        index_buffer: vk::Buffer::null(),
                        index_buffer_memory: DeviceMemory::null(),
                        vertex_buffer: vk::Buffer::null(),
                        vertex_buffer_memory: DeviceMemory::null(),
                        material_index,
                    })
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

            let mut mesh_maybe: Option<usize> = node["mesh"].as_usize();
            let mut mesh = None;
            match mesh_maybe {
                Some(mesh_index) => mesh = Some(meshes[mesh_index].clone()),
                None => mesh = None,
            }

            let mut rotation = Vector::new_empty();
            if let JsonValue::Array(ref rotation_json) = node["rotation"] {
                if rotation_json.len() >= 4 {
                    rotation = Vector::new_vec4(
                        rotation_json[0].as_f32().unwrap(),
                        rotation_json[1].as_f32().unwrap(),
                        rotation_json[2].as_f32().unwrap(),
                        rotation_json[3].as_f32().unwrap()
                    );
                }
            }

            let mut scale = Vector::new_vec(1.0);
            if let JsonValue::Array(ref scale_json) = node["rotation"] {
                if scale_json.len() >= 3 {
                    scale = Vector::new_vec3(
                        scale_json[0].as_f32().unwrap(),
                        scale_json[1].as_f32().unwrap(),
                        scale_json[2].as_f32().unwrap()
                    );
                }
            }

            let mut translation = Vector::new_empty();
            if let JsonValue::Array(ref translation_json) = node["rotation"] {
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
                Rc::new(RefCell::new(Node {
                    name,
                    mesh,
                    rotation,
                    scale,
                    translation,
                    children: Vec::new(),
                    children_indices,
                }))
            )
        }
        for node_reference in &nodes {
            let children_indices: Vec<usize>; {
                let node = node_reference.borrow();
                children_indices = node.children_indices.clone();
            }
            let mut node = node_reference.borrow_mut();
            for &child_index in &children_indices {
                node.children.push(Rc::clone(&nodes[child_index]));
            }
        }

        let mut scenes = Vec::new();
        for scene in json["scenes"].members() {
            let name_maybe: Option<&str> = scene["name"].as_str();
            let mut name = String::from("unnamed node");
            match name_maybe {
                Some(name_str) => name = String::from(name_str),
                None => (),
            }

            let mut scene_nodes = Vec::new();
            if let JsonValue::Array(ref nodes_json) = scene["nodes"] {
                for node_json in nodes_json {
                    scene_nodes.push(nodes[node_json.as_usize().unwrap()].clone());
                }
            }

            scenes.push(
                Rc::new(Scene {
                    name,
                    nodes: scene_nodes,
                })
            )
        }

        let scene = scenes[json["scene"].as_usize().unwrap()].clone();

        Gltf {
            json,
            extensions_used,
            scene,
            scenes,
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

    pub unsafe fn construct_buffers(&self, base: &VkBase) { unsafe {
        for mesh in &self.meshes {
            for primitive in &mut mesh.borrow_mut().primitives {
                primitive.construct_buffers(&base)
            }
        }
    } }

    pub unsafe fn cleanup(&self, base: &VkBase) { unsafe {
        for mesh in &self.meshes {
            for primitive in &mut mesh.borrow_mut().primitives {
                base.device.free_memory(primitive.vertex_buffer_memory, None);
                base.device.destroy_buffer(primitive.vertex_buffer, None);
                base.device.free_memory(primitive.index_buffer_memory, None);
                base.device.destroy_buffer(primitive.index_buffer, None);
            }
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
}

pub struct Texture {
    pub source: Rc<Image>,
}

pub struct Material {
    pub alpha_mode: String,
    pub double_sided: bool,
    pub normal_texture: Option<Rc<Texture>>,
    // KHR_materials_specular
        pub specular_color_factor: Vector,
    // KHR_materials_ior
        pub ior: f32,
    pub name: String,
    // pbrMetallicRoughness
        pub base_color_factor: Vector,
        pub base_color_texture: Option<Rc<Texture>>,
        pub metallic_factor: f32,
        pub metallic_texture: Option<Rc<Texture>>,
        pub roughness_factor: f32,
        pub roughness_texture: Option<Rc<Texture>>,
}

#[derive(Debug, Clone, Copy)]
enum ComponentType {
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
    fn byte_size(&self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::U32 | Self::F32 => 4,
        }
    }

    fn index_type(&self) -> vk::IndexType {
        match self {
            Self::I8 | Self::U8 => vk::IndexType::UINT8_KHR,
            Self::I16 | Self::U16 => vk::IndexType::UINT16,
            Self::U32 | Self::F32 => vk::IndexType::UINT32,
        }
    }
}
pub struct Primitive {
    pub attributes: Vec<(String, Rc<Accessor>)>,
    pub indices: Rc<Accessor>,
    pub material: Option<Rc<Material>>,

    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: DeviceMemory,
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: DeviceMemory,
    pub material_index: usize,
}
impl Primitive {
    unsafe fn construct_buffers(&mut self, base: &VkBase) { unsafe {
        let mut position_accessor: Option<&Rc<Accessor>> = None;
        let mut normal_accessor: Option<&Rc<Accessor>> = None;
        let mut texcoord_accessor: Option<&Rc<Accessor>> = None;
        for attribute in self.attributes.iter() {
            if attribute.0.eq("POSITION") {
                position_accessor = Some(&attribute.1);
            } else if attribute.0.eq("NORMAL") {
                normal_accessor = Some(&attribute.1);
            } else if attribute.0.eq("TEXCOORD_0") {
                texcoord_accessor = Some(&attribute.1);
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
                    (self.index_buffer, self.index_buffer_memory) = base.create_device_buffer(indices);
                },
                ComponentType::U16 => {
                    let indices: &[u16] = bytemuck::cast_slice(bytes);
                    (self.index_buffer, self.index_buffer_memory) = base.create_device_buffer(indices);
                },
                ComponentType::U32 => {
                    let indices: &[u32] = bytemuck::cast_slice(bytes);
                    (self.index_buffer, self.index_buffer_memory) = base.create_device_buffer(indices);
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

            let mut vertices = Vec::new();
            for i in 0..positions.len() {
                vertices.push(Vertex {
                    position: positions[i],
                    normal: *normals.get(i).unwrap_or(&[0.0, 0.0, 0.0]),
                    uv: *tex_coords.get(i).unwrap_or(&[0.0, 0.0]),
                });
            }
            (self.vertex_buffer, self.vertex_buffer_memory) = base.create_device_buffer(&vertices);
        }
    } }
}
#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}
#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct Instance {
    pub matrix: [f32; 16],
    pub material: u32,
    pub _pad: [u32; 3],
}

pub struct Mesh {
    pub name: String,
    pub primitives: Vec<Primitive>
}

pub struct Node {
    pub mesh: Option<Rc<RefCell<Mesh>>>,
    pub name: String,
    pub rotation: Vector,
    pub scale: Vector,
    pub translation: Vector,
    pub children: Vec<Rc<RefCell<Node>>>,
    pub children_indices: Vec<usize>,
}
impl Node {
    pub unsafe fn draw(&self, base: &VkBase, draw_command_buffer: &CommandBuffer, transform: &Matrix) { unsafe {
        if self.mesh.is_some() {
            for primitive in self.mesh.as_ref().unwrap().borrow().primitives.iter() {
                //println!("{:?}",primitive.material_index);

                base.device.cmd_bind_vertex_buffers(
                    *draw_command_buffer,
                    0,
                    &[primitive.vertex_buffer],
                    &[0],
                );
                base.device.cmd_bind_index_buffer(
                    *draw_command_buffer,
                    primitive.index_buffer,
                    0,
                    primitive.indices.component_type.index_type(),
                );
                base.device.cmd_draw_indexed(
                    *draw_command_buffer,
                    primitive.indices.count as u32,
                    1,
                    0,
                    0,
                    0,
                );
            }
        }
        for child in self.children.iter() {
            child.borrow().draw(base, draw_command_buffer, transform);
        }
    } }
}

pub struct Scene {
    pub name: String,
    pub nodes: Vec<Rc<RefCell<Node>>>,
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