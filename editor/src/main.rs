#![warn(unused_qualifications)]
use std::default::Default;
use std::path::PathBuf;
use ffengine::app::Engine;
use ffengine::math::Vector;
use ffengine::world::scene::{Light, Model};

fn main() { unsafe {
    let mut app = Engine::new();

    let world = &mut app.world;
    let base = &mut app.base;
    let renderer = &mut app.renderer;
    let physics_engine = &mut app.physics_engine;

    world.add_light(base, Light {
        position: Vector::new_vec3(0.0, 3.0, 0.0),
        direction: Default::default(),
        color: Vector::new_vec3(1.0, 0.0, 1.0),
        light_type: 0,
        quadratic_falloff: 0.1,
        linear_falloff: 0.1,
        constant_falloff: 0.1,
        inner_cutoff: 0.0,
        outer_cutoff: 0.0,
    });

    // world.add_model(base, Model::new(&PathBuf::from("editor/resources/models/ffocks/untitled.gltf").to_str().unwrap()));
    // world.models[0].transform_roots(&Vector::new_vec3(0.0, 0.0, 5.0), &Vector::new_vec(0.0), &Vector::new_vec(0.05));
    // world.models[0].animations[0].repeat = true;
    // world.models[0].animations[0].start();

    //world.add_model(Model::new("C:\\Graphics\\assets\\flower\\world.gltf"));
    //world.models[0].transform_roots(&Vector::new_vec3(0.0, 1.0, 0.0), &Vector::new_vec(0.0), &Vector::new_vec(1.0));
    // world.preload_model(Model::new("C:\\Graphics\\assets\\rivals\\luna\\gltf\\luna.gltf"));
    // world.models[1].animations[0].repeat = true;
    // world.models[1].animations[0].start();

    //world.preload_model(Model::new(&PathBuf::from("resources/models/sphereScene/scene.gltf").to_str().unwrap()));
    //world.preload_model(Model::new(&PathBuf::from("resources/models/discardTest/scene.gltf").to_str().unwrap()));
    //world.preload_model(Model::new(&PathBuf::from("resources/models/shadowTest/shadowTest.gltf").to_str().unwrap()));
    //world.models[1].transform_roots(&Vector::new_vec3(0.0, 0.0, -5.0), &Vector::new_vec(0.0), &Vector::new_vec(1.0));
    //world.preload_model(Model::new("C:\\Graphics\\assets\\sponzaGLTF\\sponza.gltf"));
    //world.preload_model(Model::new("C:\\Graphics\\assets\\neeko\\scene.gltf"));
    //world.preload_model(Model::new("C:\\Graphics\\assets\\bistroGLTF\\untitled.gltf"));
    //world.preload_model(Model::new("C:\\Graphics\\assets\\asgard\\asgard.gltf"));
    //world.preload_model(Model::new("C:\\Graphics\\assets\\helmet\\DamagedHelmet.gltf"));
    //world.models[0].transform_roots(&Vector::new_vec3(1.0, 1.0, 2.0), &Vector::new_vec3(0.0, 0.0, 0.0), &Vector::new_vec3(2.0, 1.0, 1.0));
    //world.preload_model(Model::new(&PathBuf::from("resources/models/coordinateSpace/coordinateSpace.gltf").to_str().unwrap()));

    world.add_model(base, Model::new(&PathBuf::from("editor/resources/models/collisionTest/collisionTestNoWalls.gltf").to_str().unwrap()));

    world.add_model(base, Model::new(&PathBuf::from("editor/resources/models/demoBall/scene.gltf").to_str().unwrap()));
    world.add_model(base, Model::new(&PathBuf::from("editor/resources/models/demoBall/scene.gltf").to_str().unwrap()));
    world.add_model(base, Model::new("C:\\Graphics\\assets\\grassblockGLTF\\grassblock.gltf"));

    physics_engine.add_all_nodes_from_model(&world, 1, 3);
    physics_engine.add_all_nodes_from_model(&world, 2, 3);
    physics_engine.add_all_nodes_from_model(&world, 3, 0);
    physics_engine.add_all_nodes_from_model(&world, 0, 0);
    physics_engine.rigid_bodies[0].set_static(false);
    physics_engine.rigid_bodies[0].set_mass(1.0);
    physics_engine.rigid_bodies[0].position = Vector::new_vec3(0.5, 10.0, 0.5);
    physics_engine.rigid_bodies[0].restitution_coefficient = 1.0;
    
    physics_engine.rigid_bodies[1].set_static(false);
    physics_engine.rigid_bodies[1].set_mass(1.0);
    physics_engine.rigid_bodies[1].position = Vector::new_vec3(0.5, 5.0, 0.5);
    physics_engine.rigid_bodies[1].restitution_coefficient = 1.0;
    
    physics_engine.rigid_bodies[2].set_static(false);
    physics_engine.rigid_bodies[2].set_mass(1.0);
    physics_engine.rigid_bodies[2].position = Vector::new_vec3(0.5, 15.0, 0.5);
    physics_engine.rigid_bodies[2].restitution_coefficient = 1.0;

    physics_engine.add_player(app.controller.borrow().player.clone());
    
    renderer.scene_renderer.update_world_textures_all_frames(base, world);
    renderer.gui.load_from_file(base, "editor\\resources\\gui\\default\\default.gui");
    

    app.run()
} }