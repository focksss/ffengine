use std::fs;
use std::rc::Rc;
use std::io::Read;
use std::path::{Path, PathBuf};
use json::JsonValue;
use winit::dpi::Position;
use crate::vector::Vector;

pub struct Gltf {
    pub json: JsonValue,
    pub extensions_used: Vec<String>,
    pub active_scene: i32,
    pub scenes: Vec<Scene>,
    pub textures: Vec<Rc<Texture>>,
    pub images: Vec<Rc<Image>>,
    pub accessors: Vec<Rc<Accessor>>,
    pub buffer_views: Vec<Rc<BufferView>>,
    pub buffers: Vec<Rc<Buffer>>,
}
impl Gltf {
    pub fn new(path: &str) -> Self {
        let json = json::parse(fs::read_to_string(path).expect("failed to load json file").as_str()).expect("json parse error");
        let active_scene = json["scene"].as_i32().unwrap();
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

        Gltf {
            json,
            extensions_used,
            active_scene,
            scenes: Vec::new(),
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
    pub double_sided: bool,
    // KHR_materials_specular
        pub specular_color_factor: Vector,
    // KHR_materials_ior
        pub ior: f32,
    pub name: String,
    // pbrMetallicRoughness
        pub base_color_factor: Vector,
        pub base_color_texture: Option<Rc<Texture>>,
        pub metallic_factor: f32,
        pub roughness_factor: f32,
}

pub struct Primitive {
    pub attributes: Vec<(String, Rc<BufferView>)>,
    pub indices: Rc<BufferView>,
}

pub struct Mesh {

}

pub struct Node {
    pub children: Vec<Node>,
    pub mesh: Mesh,
}

pub struct Scene {
    pub nodes: Vec<Node>,
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