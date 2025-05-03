use std::cell::RefCell;
use std::fs;
use std::rc::Rc;
use std::io::Read;
use std::path::{Path, PathBuf};
use json::JsonValue;
use winit::dpi::Position;
use crate::matrix::Matrix;
use crate::vector::Vector;

pub struct Gltf {
    pub json: JsonValue,
    pub extensions_used: Vec<String>,
    pub scene: Rc<Scene>,
    pub scenes: Vec<Rc<Scene>>,
    pub nodes: Vec<Rc<RefCell<Node>>>,
    pub meshes: Vec<Rc<Mesh>>,
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
                    target: buffer_view["target"].as_usize().unwrap()
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
                    component_type: accessor["componentType"].as_u32().unwrap(),
                    count: accessor["count"].as_usize().unwrap(),
                    r#type: accessor["type"].as_str().unwrap().parse().unwrap(),
                    min,
                    max,
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

                    let mut material = None;
                    if let JsonValue::Object(ref material_json) = primitive_json["material"] {
                        material = Some(materials[material_json["material"].as_usize().unwrap()].clone());
                    }

                    primitives.push(Primitive {
                        attributes,
                        indices,
                        material,
                    })
                }
            }

            meshes.push(
                Rc::new(Mesh {
                    name,
                    primitives,
                })
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
    pub component_type: u32,
    pub count: usize,
    pub r#type: String,
    pub min: Option<Vector>,
    pub max: Option<Vector>,
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

pub struct Primitive {
    pub attributes: Vec<(String, Rc<Accessor>)>,
    pub indices: Rc<Accessor>,
    pub material: Option<Rc<Material>>,
}

pub struct Mesh {
    pub name: String,
    pub primitives: Vec<Primitive>
}

pub struct Node {
    pub mesh: Option<Rc<Mesh>>,
    pub name: String,
    pub rotation: Vector,
    pub scale: Vector,
    pub translation: Vector,
    pub children: Vec<Rc<RefCell<Node>>>,
    pub children_indices: Vec<usize>,
}

pub struct Scene {
    pub name: String,
    pub nodes: Vec<Rc<RefCell<Node>>>,
}

pub struct Triangle {
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