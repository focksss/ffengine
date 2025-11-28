use crate::math::Vector;

pub struct Scene {

}

pub struct Entity {
    pub name: String,
    pub transform: Transform,

    pub render_object: Option<RenderComponent>,
    pub rigid_body: Option<RigidBodyComponent>, // physics object
    pub camera: Option<CameraComponent>,
    pub light: Option<LightComponent>,
}
pub struct Transform {
    translation: Vector,
    rotation: Vector,
    scale: Vector,
}
pub struct RigidBodyComponent {
    rigid_body_index: usize,
}
pub struct RenderComponent {
    pub node_index: usize, // world node
}
pub struct CameraComponent {
    pub camera_index: usize,
}
pub struct LightComponent {
    pub light_index: usize,
}