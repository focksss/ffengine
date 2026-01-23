use std::cell::RefCell;
use std::sync::Arc;
use crate::client::client::Client;
use crate::gui::gui::GUI;
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
    client: Arc<RefCell<Client>>
) -> (Scene, GUI) {
    let mut scene = Scene::new(context, renderer.clone(), world, physics_engine);
    let renderer = renderer.borrow();
    let scene_renderer = renderer.scene_renderer.borrow();
    let mut gui = GUI::new(context, client, scene_renderer.null_tex_sampler, scene_renderer.null_texture.image_view);

    (scene, gui)
}