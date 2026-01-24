use std::cell::RefCell;
use std::sync::Arc;
use crate::render::render::Renderer;
use crate::render::vulkan_base::Context;
use crate::scene::physics::physics_engine::PhysicsEngine;
use crate::scene::scene::Scene;
use crate::scene::world::world::World;

pub fn load_scene(
    path: &str,
    context: &Arc<Context>,
    renderer: Arc<RefCell<Renderer>>,
    world: Arc<RefCell<World>>,
    physics_engine:
    Arc<RefCell<PhysicsEngine>>,
) -> Scene {
    let mut scene = Scene::new(context, renderer.clone(), world, physics_engine);

    scene
}