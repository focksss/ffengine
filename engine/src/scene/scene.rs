use crate::math::Vector;

//TODO handle ALL updates + rendering from here (call to World + PhysicsEngine)
pub struct Scene {
    pub entities: Vec<Entity>,

    pub transforms: Vec<Transform>,
    pub render_components: Vec<RenderComponent>,
    pub rigid_body_components: Vec<RigidBodyComponent>,
    pub camera_components: Vec<CameraComponent>,
    pub light_components: Vec<LightComponent>,
}

pub struct Entity {
    pub name: String,
    pub transform: usize,
    pub children: Vec<usize>,
    pub parent: usize,

    pub render_object: Option<usize>,
    pub rigid_body: Option<usize>,
    pub camera: Option<usize>,
    pub light: Option<usize>,
}
pub struct Transform {
    translation: Vector,
    rotation: Vector,
    scale: Vector,
}
pub struct RigidBodyComponent {
    rigid_body_index: usize, // physics object
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