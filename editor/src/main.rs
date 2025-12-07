#![warn(unused_qualifications)]
use std::default::Default;
use std::path::{Path, PathBuf};
use ffengine::engine::Engine;
use ffengine::math::Vector;
use ffengine::scene::physics::player::{MovementMode, Player};
use ffengine::scene::world::camera::Camera;
use ffengine::scene::world::world::{Light};

fn main() { unsafe {
    let mut app = Engine::new();

    app.load_script(Path::new("editor\\resources\\scripts\\player_controller.lua"));

    {
        let base = &mut app.base;
        let renderer = &mut app.renderer.borrow_mut();
        let physics_engine = &mut app.physics_engine.borrow_mut();

        app.world.borrow_mut().add_light(base, Light {
            position: Vector::new3(0.0, 3.0, 0.0),
            direction: Default::default(),
            color: Vector::new3(1.0, 0.0, 1.0),
            light_type: 0,
            quadratic_falloff: 0.1,
            linear_falloff: 0.1,
            constant_falloff: 0.1,
            inner_cutoff: 0.0,
            outer_cutoff: 0.0,
        });


        {
            app.scene.borrow_mut().new_entity_from_model(base, 0, "editor/resources/models/ffocks/untitled.gltf");
            let anim = &mut app.scene.borrow_mut().animation_components[0];
            anim.repeat = true;
            anim.snap_back = true;
            anim.start();
        }

        //app.scene.borrow_mut().new_entity_from_model(base, 0, "editor/resources/models/grassblockGLTF/grassblock.gltf");
        // app.scene.borrow_mut().new_entity_from_model(base, 0, "C:\\Graphics\\assets\\sponzaGLTF\\sponza.gltf");

        //world.add_model(Model::new("C:\\Graphics\\assets\\flower\\world.gltf"));
        //world.models[0].transform_roots(&Vector::new_vec3(0.0, 1.0, 0.0), &Vector::new_vec(0.0), &Vector::new_vec(1.0));
        // world.preload_model(Model::new("C:\\Graphics\\assets\\rivals\\luna\\gltf\\luna.gltf"));
        // world.models[1].animations[0].repeat = true;
        // world.models[1].animations[0].start();

        //world.preload_model(Model::new(&PathBuf::from("resources/models/sphereScene/scene.gltf").to_str().unwrap()));
        //world.preload_model(Model::new(&PathBuf::from("resources/models/discardTest/scene.gltf").to_str().unwrap()));
        //world.preload_model(Model::new(&PathBuf::from("resources/models/shadowTest/shadowTest.gltf").to_str().unwrap()));
        //world.models[1].transform_roots(&Vector::new_vec3(0.0, 0.0, -5.0), &Vector::new_vec(0.0), &Vector::new_vec(1.0));
        //app.scene.borrow_mut().new_entity_from_model(base, 0, "C:\\Graphics\\assets\\sponzaGLTF\\sponza.gltf");
        //world.preload_model(Model::new("C:\\Graphics\\assets\\neeko\\scene.gltf"));
        //world.preload_model(Model::new("C:\\Graphics\\assets\\bistroGLTF\\untitled.gltf"));
        //world.preload_model(Model::new("C:\\Graphics\\assets\\asgard\\asgard.gltf"));
        //world.preload_model(Model::new("C:\\Graphics\\assets\\helmet\\DamagedHelmet.gltf"));
        //world.models[0].transform_roots(&Vector::new_vec3(1.0, 1.0, 2.0), &Vector::new_vec3(0.0, 0.0, 0.0), &Vector::new_vec3(2.0, 1.0, 1.0));
        //world.preload_model(Model::new(&PathBuf::from("resources/models/coordinateSpace/coordinateSpace.gltf").to_str().unwrap()));

        //physics_engine.add_all_nodes_from_model(&world, 0, 4);
        /*
        world.add_model(base, Model::new(&PathBuf::from("editor/resources/models/sphereScene/scene.gltf").to_str().unwrap()));

        world.add_model(base, Model::new("C:\\Graphics\\assets\\grassblockGLTF\\grassblock.gltf"));

        physics_engine.add_all_nodes_from_model(&world, 1, 4);
        physics_engine.add_all_nodes_from_model(&world, 0, 4);
        physics_engine.rigid_bodies[0].set_static(false);
        physics_engine.rigid_bodies[0].set_mass(1.0);
        physics_engine.rigid_bodies[0].position = Vector::new3(0.5, 10.0, 0.5);
        physics_engine.rigid_bodies[0].restitution_coefficient = 1.0;
        */

        // /*
        app.scene.borrow_mut().new_entity_from_model(base, 0, "editor/resources/models/collisionTest/collisionTestNoWalls.gltf");

        app.scene.borrow_mut().new_entity_from_model(base, 0, "editor/resources/models/demoBall/scene.gltf");
        app.scene.borrow_mut().new_entity_from_model(base, 0, "editor/resources/models/demoBall/scene.gltf");
        app.scene.borrow_mut().new_entity_from_model(base, 0, "editor/resources/models/grassblockGLTF/grassblock.gltf");

        physics_engine.add_all_nodes_from_model(&app.world.borrow(), 2, 3);
        physics_engine.add_all_nodes_from_model(&app.world.borrow(), 3, 3);
        physics_engine.add_all_nodes_from_model(&app.world.borrow(), 4, 3);
        physics_engine.add_all_nodes_from_model(&app.world.borrow(), 1, 0);
        physics_engine.rigid_bodies[0].set_static(false);
        physics_engine.rigid_bodies[0].set_mass(1.0);
        physics_engine.rigid_bodies[0].position = Vector::new3(0.5, 10.0, 0.5);
        physics_engine.rigid_bodies[0].restitution_coefficient = 1.0;

        physics_engine.rigid_bodies[1].set_static(false);
        physics_engine.rigid_bodies[1].set_mass(1.0);
        physics_engine.rigid_bodies[1].position = Vector::new3(0.5, 5.0, 0.5);
        physics_engine.rigid_bodies[1].restitution_coefficient = 1.0;

        physics_engine.rigid_bodies[2].set_static(false);
        physics_engine.rigid_bodies[2].set_mass(1.0);
        physics_engine.rigid_bodies[2].position = Vector::new3(0.5, 15.0, 0.5);
        physics_engine.rigid_bodies[2].restitution_coefficient = 1.0;
        // */

        /*
        world.add_model(base, Model::new(&PathBuf::from("editor/resources/models/sphereScene/scene.gltf").to_str().unwrap()));

        world.add_model(base, Model::new(&PathBuf::from("editor/resources/models/demoBall/scene.gltf").to_str().unwrap()));
        world.add_model(base, Model::new(&PathBuf::from("editor/resources/models/demoBall/scene.gltf").to_str().unwrap()));
        world.add_model(base, Model::new("C:\\Graphics\\assets\\grassblockGLTF\\grassblock.gltf"));

        physics_engine.add_all_nodes_from_model(&world, 1, 4);
        physics_engine.add_all_nodes_from_model(&world, 2, 4);
        physics_engine.add_all_nodes_from_model(&world, 3, 4);
        physics_engine.add_all_nodes_from_model(&world, 0, 4);
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
        */


        renderer.scene_renderer.borrow_mut().update_world_textures_all_frames(base, &app.world.borrow());
        renderer.guis[0].borrow_mut().load_from_file(base, "editor\\resources\\gui\\editor.gui");
    }
    let player = Player::new(
        app.physics_engine.clone(),
        app.world.clone(),
        Camera::new_perspective_rotation(
            Vector::new3(0.0, 2.0, 0.0),
            Vector::empty(),
            100.0,
            1.0,
            0.001,
            1000.0,
            true,
            Vector::new3(0.0, 0.0, 1.0),
        ),
        Vector::new3(-0.15, -0.85, -0.15),
        Vector::new3(0.15, 0.15, 0.15),
        MovementMode::EDITOR,
        0.2,
        4.5,
        0.0015
    );
    app.physics_engine.borrow_mut().add_player(player);

    println!("starting");

    app.run()
} }