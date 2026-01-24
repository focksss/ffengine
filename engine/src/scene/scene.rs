use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::slice;
use std::sync::Arc;
use std::time::SystemTime;
use ash::vk;
use ash::vk::{CommandBuffer, ShaderStageFlags};
use crate::engine::get_command_buffer;
use crate::math::matrix::Matrix;
use crate::math::Vector;
use crate::render::render::{Renderer, MAX_FRAMES_IN_FLIGHT};
use crate::render::scene_renderer::{CameraMatrixUniformData, SceneRenderer, SHADOW_RES};
use crate::render::vulkan_base::{copy_buffer_synchronous, copy_data_to_memory, Context};
use crate::scene::components::animation::AnimationComponent;
use crate::scene::components::camera::CameraComponent;
use crate::scene::components::hitbox::HitboxComponent;
use crate::scene::components::light::LightComponent;
use crate::scene::components::mesh::Mesh;
use crate::scene::components::rigid_body::RigidBodyComponent;
use crate::scene::components::script::ScriptComponent;
use crate::scene::components::skin::SkinComponent;
use crate::scene::components::sun::SunComponent;
use crate::scene::physics::hitboxes::bounding_box::BoundingBox;
use crate::scene::physics::hitboxes::convex_hull::ConvexHull;
use crate::scene::physics::hitboxes::hitbox::{Hitbox, HitboxType};
use crate::scene::physics::hitboxes::sphere::Sphere;
use crate::scene::physics::physics_engine::{AxisType, ContactInformation, ContactPoint, PhysicsEngine};
use crate::scene::components::transform::Transform;
use crate::scene::ui::image::Image;
use crate::scene::ui::interaction::UiInteractableInformation;
use crate::scene::ui::layout::UiNodeLayout;
use crate::scene::ui::quad::Quad;
use crate::scene::ui::text::Text;
use crate::scene::ui::texture::Texture;
use crate::scene::world::world::{LightSendable, SunSendable, World};


/**
ECS
 - EVERYTHING is contained by an entity and is a type of component.
 - Cameras (will) own their own SceneRenderer and render to either a RenderTexture or the screen.
*/

pub struct Scene {
    pub context: Arc<Context>,

    pub runtime: f32,

    pub running: bool,

    pub entities: Vec<Entity>, // will always have a root node with sun

    pub unupdated_entities: Vec<usize>,

    pub transforms: Vec<Transform>,
    pub mesh_components: Vec<Mesh>,
    pub skin_components: Vec<SkinComponent>,
    pub animation_components: Vec<AnimationComponent>,
    pub rigid_body_components: Vec<RigidBodyComponent>,
    pub hitbox_components: Vec<HitboxComponent>,
    pub camera_components: Vec<CameraComponent>,
    pub light_components: Vec<LightComponent>,
    pub sun_components: Vec<SunComponent>,
    pub script_components: Vec<ScriptComponent>,

    pub ui_node_layouts: Vec<UiNodeLayout>,
    pub ui_interactable_information: Vec<UiInteractableInformation>,
    pub ui_quads: Vec<Quad>,
    pub ui_texts: Vec<Text>,
    pub ui_images: Vec<Image>,
    pub ui_textures: Vec<Texture>,

    pub outlined_components: Vec<usize>,
    pub outlined_bodies: Vec<usize>,

    pub ui_root_entities: Vec<usize>,

    pub renderer: Arc<RefCell<Renderer>>,
    pub world: Arc<RefCell<World>>,
    pub physics_engine: Arc<RefCell<PhysicsEngine>>,

    dirty_render_components: Vec<usize>,
    dirty_light_components: Vec<usize>,
    dirty_sun_components: Vec<usize>,
    dirty_camera_components: Vec<usize>,
}
impl Scene {
    pub fn new(context: &Arc<Context>, renderer: Arc<RefCell<Renderer>>, world: Arc<RefCell<World>>, physics_engine: Arc<RefCell<PhysicsEngine>>) -> Self {
        let mut scene = Self {
            context: context.clone(),

            runtime: 0.0,
            running: false,

            entities: Vec::new(),
            unupdated_entities: Vec::new(),
            transforms: Vec::new(),
            mesh_components: Vec::new(),
            skin_components: Vec::new(),
            animation_components: Vec::new(),
            rigid_body_components: Vec::new(),
            hitbox_components: Vec::new(),
            camera_components: Vec::new(),
            light_components: Vec::new(),
            sun_components: Vec::new(),
            script_components: Vec::new(),

            ui_node_layouts: Vec::new(),
            ui_interactable_information: Vec::new(),
            ui_texts: Vec::new(),
            ui_quads: Vec::new(),
            ui_images: Vec::new(),
            ui_textures: Vec::new(),

            ui_root_entities: Vec::new(),

            outlined_components: Vec::new(),
            outlined_bodies: Vec::new(),
            renderer,
            world,
            physics_engine,
            dirty_render_components: Vec::new(),
            dirty_light_components: Vec::new(),
            dirty_sun_components: Vec::new(),
            dirty_camera_components: Vec::new(),
        };
        scene.transforms.push(Transform::default());
        scene.entities.push(Entity {
            name: String::from("Global Root"),
            transform: 0,
            children_indices: Vec::new(),
            parent: 0,
            ..Default::default()
        });

        scene
    }

    pub fn new_entity_from_model(&mut self, parent_index: usize, uri: &str) -> usize {
        let model_entity_index = self.entities.len();
        self.unupdated_entities.push(model_entity_index);

        let (new_nodes, new_skins, new_animations) = {
            let world = &mut self.world.borrow_mut();

            let model_index = unsafe { world.add_model(uri) };

            let new_model = &world.models[model_index];

            let entity_transform_index = self.transforms.len();
            let mut model_transform = Transform::default();
            model_transform.owner = model_entity_index;
            self.transforms.push(model_transform);
            self.entities[parent_index].children_indices.push(model_entity_index);
            self.entities.push(Entity {
                name: String::from(uri),
                transform: entity_transform_index,
                parent: parent_index,
                animation_objects: new_model.animations.iter().map(|&i| i + self.animation_components.len()).collect(),
                ..Default::default()
            });

            (world.scenes[new_model.scene].nodes.clone(), new_model.skins.clone(), new_model.animations.clone())
        };

        for node_index in new_nodes {
            self.implement_world_node(node_index, model_entity_index);
        }
        let world = &self.world.borrow();
        for skin_index in new_skins {
            let skin = &world.skins[skin_index];
            let mapped_joint_indices = skin.joint_indices.iter().map(|&i| world.nodes[i].mapped_entity_index).collect::<Vec<usize>>();
            self.skin_components.push(SkinComponent {
                joints: mapped_joint_indices,
                inverse_bind_matrices: skin.inverse_bind_matrices.clone(),
            });
        }
        for animation_index in new_animations {
            let animation = &world.animations[animation_index];
            self.animation_components.push(AnimationComponent {
                owner_entity: model_entity_index,
                channels: animation.channels.iter().map(|c| (c.0, world.nodes[c.1].mapped_entity_index, c.2.clone())).collect(),
                samplers: animation.samplers.clone(),
                start_time: SystemTime::now(),
                duration: animation.duration,
                running: animation.running,
                repeat: animation.snap_back,
                snap_back: animation.snap_back,
            });
        }

        model_entity_index
    }
    fn implement_world_node(&mut self, node_index: usize, parent_index: usize) {
        let node_entity_index = self.entities.len();
        self.entities[parent_index].children_indices.push(node_entity_index);

        let child_nodes = {

            let world = &mut self.world.borrow_mut();
            world.nodes[node_index].mapped_entity_index = self.entities.len();
            let node = &world.nodes[node_index];

            let node_transform_index = self.transforms.len();
            self.transforms.push(Transform {
                owner: node_entity_index,
                local_translation: node.translation,
                local_rotation: node.rotation,
                local_scale: node.scale,
                ..Default::default()
            });

            let node_anim_transform_index = self.transforms.len();
            self.transforms.push(Transform {
                owner: node_entity_index,
                local_translation: node.translation,
                local_rotation: node.rotation,
                local_scale: node.scale,
                ..Default::default()
            });

            self.entities.push(Entity {
                name: node.name.clone(),
                transform: node_transform_index,
                animated_transform: (node_anim_transform_index, false),
                parent: parent_index,
                ..Default::default()
            });

            if let Some(mesh_index) = node.mesh {
                let entity = &mut self.entities[node_entity_index];
                for (i, primitive) in world.meshes[mesh_index].primitives.iter().enumerate() {
                    let render_component_transform_index = self.transforms.len();
                    self.transforms.push(Transform {
                        owner: node_entity_index,
                        ..Default::default()
                    });

                    let render_component_index = self.mesh_components.len();
                    self.mesh_components.push(Mesh {
                        mesh_primitive_index: (mesh_index, i),
                        skin_index: node.skin,
                        material_index: primitive.material_index as usize,
                        transform: render_component_transform_index,
                    });

                    entity.render_objects.push(render_component_index);
                }
            }
            node.children_indices.clone()
        };
        for child_node_index in child_nodes {
            self.implement_world_node(child_node_index, node_entity_index);
        }
    }

    pub fn add_light(&mut self, mut light: LightComponent, parent_index: usize) -> usize {
        let index = self.light_components.len();

        let owner_index = self.entities.len();
        self.entities[parent_index].children_indices.push(owner_index);
        let mut entity = Entity::default();
        entity.name = String::from(format!("LightEntity {}", index));
        entity.transform = self.transforms.len();
        entity.lights = vec![index];
        self.entities.push(entity);
        light.owner = owner_index;

        let transform_index = self.transforms.len();
        self.transforms.push(Transform::default());
        light.transform = transform_index;

        self.dirty_light_components.push(index);
        self.light_components.push(light);
        index
    }

    pub fn add_camera(&mut self, mut camera: CameraComponent, parent_index: usize) -> usize {
        let index = self.camera_components.len();

        let owner_index = self.entities.len();
        self.entities[parent_index].children_indices.push(owner_index);
        let mut entity = Entity::default();
        entity.name = String::from(format!("CameraEntity {}", index));
        entity.transform = self.transforms.len();
        entity.cameras = vec![index];
        self.entities.push(entity);
        camera.owner = owner_index;

        let transform_index = self.transforms.len();
        self.transforms.push(Transform::default());
        camera.transform = transform_index;

        self.dirty_camera_components.push(index);
        self.camera_components.push(camera);
        index
    }

    pub fn add_rigid_body_from_entity(&mut self, entity_index: usize, hitbox_type: usize, is_static: bool) {
        assert!(hitbox_type < 5);

        let entity = &mut self.entities[entity_index];

        if !entity.render_objects.is_empty() {
            let render_component = &self.mesh_components[entity.render_objects[0]];
            let mesh = &self.world.borrow().meshes[render_component.mesh_primitive_index.0];

            entity.rigid_body = Some(self.rigid_body_components.len());

            let transform = &self.transforms[render_component.transform].world;
            let scale = transform.extract_scale();
            let (min, max) = mesh.get_min_max();
            let half_extent = (max - min) * 0.5 * scale;

            let mut body = RigidBodyComponent::default();
            body.owner = entity_index;
            body.transform = entity.transform;
            let transform = &self.transforms[entity.transform];
            body.x_f = transform.world_translation;
            body.q_f = transform.world_rotation;
            body.hitbox = self.hitbox_components.len();
            body.stored_hitbox_scale = scale;

            self.rigid_body_components.push(body);
            let body = &mut self.rigid_body_components[entity.rigid_body.unwrap()];

            self.hitbox_components.push(HitboxComponent { hitbox: match hitbox_type {
                0 => {
                    let bounds = BoundingBox {
                        center: (min + max) * scale * 0.5,
                        half_extents: half_extent,
                    };
                    Hitbox::OBB(bounds, ConvexHull::from_bounds(&bounds))
                }
                3 => {
                    Hitbox::Sphere(Sphere {
                        center: (min + max) * scale * 0.5,
                        radius: half_extent.max_of(),
                    })
                }
                _ => unreachable!()
            }});

            body.set_static(&self.hitbox_components[body.hitbox].hitbox, &self.transforms, is_static);
            body.set_mass(&self.hitbox_components[body.hitbox].hitbox, &self.transforms, 1.0);
            //body.angular_velocity = Vector::new3(0.0, 1.0, 0.0);
        }
        for child_index in entity.children_indices.clone() {
            self.add_rigid_body_from_entity(child_index, hitbox_type, is_static);
        }
    }

    pub fn update_physics_objects(&mut self, delta_time: f32) {
        let gravity = Vector::new3(0.0, -9.8, 0.0);
        let iter = 5;
        let dt = delta_time / iter as f32;

        for _ in 0..iter {
            // integrate
            for body in &mut self.rigid_body_components {
                body.integrate(dt, &gravity)
            }

            // collision constraints
            let num_bodies = self.rigid_body_components.len();
            let mut collision_constraints = Vec::new();
            for i in 0..num_bodies {
                let body_a = &self.rigid_body_components[i];
                for j in i + 1..num_bodies {
                    let body_b = &self.rigid_body_components[j];

                    if (body_a.is_static && body_b.is_static) || body_a.owned_by_player || body_b.owned_by_player {
                        continue;
                    }
                    if let Some(collision) = body_a.will_collide_with(&self.hitbox_components, body_b, 0.0) {
                        if collision.contact_points.is_empty() { continue }
                        let normal = collision.normal;
                        let depth = collision.time_of_impact;
                        let pt_on_a = collision.contact_points[0].point_on_a;
                        let pt_on_b = collision.contact_points[0].point_on_b;

                        collision_constraints.push(CollisionConstraint {
                            body_a: i,
                            body_b: j,
                            penetration: depth,
                            normal,
                            pt_on_a,
                            pt_on_b,
                        })
                    }
                }
            }
            for constraint in collision_constraints {
                constraint.solve(dt, &mut self.rigid_body_components)
            }

            // update velocity
            for body in &mut self.rigid_body_components {
                body.update_velocity(dt)
            }
        }
        for body in &mut self.rigid_body_components {
            if !body.is_static {
                let owner = &self.entities[body.owner];
                let parent = &self.entities[owner.parent];
                body.update(&mut self.transforms, parent.transform);
                let entity_index = body.owner;
                self.unupdated_entities.push(entity_index);
            }
        }
        /*
        let gravity = Vector::new3(0.0, -9.8, 0.0);
        // apply gravity
        for body in &mut self.rigid_body_components {
            if !body.is_static {
                body.apply_impulse(gravity * delta_time * body.mass, body.get_center_of_mass_world_space(&self.transforms), &self.transforms);
            }
        }
        // collision detection
        for i in 0..self.rigid_body_components.len() {
            let (a, b) = self.rigid_body_components.split_at_mut(i + 1);
            let body_a = &mut a[i];
            for body_b in b {
                if (body_a.is_static && body_b.is_static) || body_a.owned_by_player || body_b.owned_by_player {
                    continue;
                }
                if let Some(collision) = body_a.will_collide_with(&self.hitbox_components, body_b, 0.0, &self.transforms) {
                    if collision.contact_points.is_empty() { continue; }
                    let normal = collision.normal;
                    let depth = collision.time_of_impact;

                    let im_a = body_a.inv_mass;
                    let im_b = body_b.inv_mass;
                    let s_im = im_a + im_b;
                    let restitution = body_a.restitution_coefficient * body_b.restitution_coefficient;
                    let inv_inertia_a = body_a.get_inverse_inertia_tensor_world_space(&self.transforms);
                    let inv_inertia_b = body_b.get_inverse_inertia_tensor_world_space(&self.transforms);

                    let pt_on_a = collision.contact_points[0].point_on_a;
                    let pt_on_b = collision.contact_points[0].point_on_b;
                    let ra = pt_on_a - body_a.get_center_of_mass_world_space(&self.transforms);
                    let rb = pt_on_b - body_b.get_center_of_mass_world_space(&self.transforms);

                    let angular_j_a = (inv_inertia_a * ra.cross(&normal)).cross(&ra);
                    let angular_j_b = (inv_inertia_b * rb.cross(&normal)).cross(&rb);
                    let angular_factor = (angular_j_a + angular_j_b).dot3(&normal);

                    let vel_a = body_a.velocity + body_a.angular_velocity.cross(&ra);
                    let vel_b = body_b.velocity + body_b.angular_velocity.cross(&rb);

                    let v_diff = vel_a - vel_b;

                    let j = normal * (1.0 + restitution) * v_diff.dot3(&normal) / (s_im + angular_factor);
                    body_a.apply_impulse(-j, pt_on_a, &mut self.transforms);
                    body_b.apply_impulse(j, pt_on_b, &mut self.transforms);

                    let friction = body_a.friction_coefficient * body_b.friction_coefficient;
                    let velocity_normal = normal * normal.dot3(&v_diff);
                    let velocity_tangent = v_diff - velocity_normal;

                    let relative_tangent_vel = velocity_tangent.normalize3();
                    let inertia_a = (inv_inertia_a * ra.cross(&relative_tangent_vel)).cross(&ra);
                    let inertia_b = (inv_inertia_b * rb.cross(&relative_tangent_vel)).cross(&rb);
                    let inv_inertia = (inertia_a + inertia_b).dot3(&relative_tangent_vel);

                    let mass_reduc = 1.0 / (s_im + inv_inertia);
                    let friction_impulse = velocity_tangent * mass_reduc * friction;

                    body_a.apply_impulse(-friction_impulse, pt_on_a, &self.transforms);
                    body_b.apply_impulse(friction_impulse, pt_on_b, &self.transforms);

                    let t_a = im_a / s_im;
                    let t_b = im_b / s_im;

                    let ds = normal * depth;

                    if !body_a.is_static {
                        let a_transform = &mut self.transforms[body_a.transform];
                        a_transform.world_translation -= ds * t_a;
                    }
                    if !body_b.is_static {
                        let b_transform = &mut self.transforms[body_b.transform];
                        b_transform.world_translation += ds * t_b;
                    }
                }
            }
        }
        // apply velocity
        for body in &mut self.rigid_body_components {
            if !body.is_static {
                body.update(delta_time, &mut self.transforms, &self.entities);
                let entity_index = body.owner;
                self.unupdated_entities.push(entity_index);
            }
        }
         */
    }

    pub fn update_scene(&mut self, command_buffer: CommandBuffer, frame: usize, delta_time: f32, force_run: bool) {
        self.layout_ui();

        if self.running || force_run {
            self.update_physics_objects(delta_time);

            self.runtime += delta_time;

            // self.world.borrow_mut().sun.vector = Vector::new3(0.55, f32::sin(self.runtime * 0.05), f32::cos(-self.runtime * 0.05));
            self.sun_components[0].direction = Vector::new3(0.55, f32::sin(self.runtime * 0.05), -f32::cos(self.runtime * 0.05)).normalize3();

            for animation in self.animation_components.iter_mut() {
                animation.update(&mut self.entities, &mut self.transforms, &mut self.unupdated_entities)
            }
        }

        let mut dirty_primitive_instance_data: Vec<Instance> = Vec::new();
        for entity_index in self.unupdated_entities.clone().iter() {
            //for entity_index in &vec![1usize] {
            let parent_index = self.entities[*entity_index].parent;
            let parent_transform = if *entity_index == 0 {
                0
            } else {
                self.entities[parent_index].transform
            };
            self.update_entity(
                frame,
                parent_transform,
                *entity_index,
                &mut dirty_primitive_instance_data
            );
        }
        self.unupdated_entities.clear();

        for dirty_camera_index in &self.dirty_camera_components {
            let camera = &mut self.camera_components[*dirty_camera_index];
            let transform = &self.transforms[camera.transform];
            camera.update_matrices(transform);
            camera.update_frustum(transform);
        }
        self.dirty_camera_components.clear();

        let mut joints = Vec::new();
        let mut total = 0f32;
        for skin in self.skin_components.iter() {
            joints.push(Matrix::new_manual([self.skin_components.len() as f32 + total; 16]));
            total += skin.joints.len() as f32;
        }
        for skin in self.skin_components.iter() {
            skin.update(&self, &mut joints);
        }

        let world = &self.world.borrow();
        unsafe {
            std::ptr::copy_nonoverlapping(
                dirty_primitive_instance_data.as_ptr() as *const u8,
                world.instance_staging_buffer.2 as *mut u8,
                size_of::<Instance>() * dirty_primitive_instance_data.len(),
            );

            let mut copy_regions = Vec::new();
            for (i, &render_component_id) in self.dirty_render_components.iter().enumerate() {
                copy_regions.push(vk::BufferCopy {
                    src_offset: (i * size_of::<Instance>()) as u64,
                    dst_offset: (render_component_id * size_of::<Instance>()) as u64,
                    size: size_of::<Instance>() as u64,
                });
            }
            self.dirty_render_components.clear();
            copy_data_to_memory(world.joints_staging_buffer.2, &joints);
            if !copy_regions.is_empty() {
                for frame in 0..MAX_FRAMES_IN_FLIGHT {
                    copy_buffer_synchronous(
                        &self.context.device,
                        command_buffer,
                        &world.instance_staging_buffer.0,
                        &world.instance_buffers[frame].0,
                        Some(copy_regions.clone()),
                        &0u64
                    );

                    copy_buffer_synchronous(&self.context.device, command_buffer, &world.joints_staging_buffer.0, &world.joints_buffers[frame].0, None, &world.joints_buffers_size);
                }
                copy_regions.clear();
            }

            let mut new_light_data = Vec::new();
            for (i, &light_id) in self.dirty_light_components.iter().enumerate() {
                copy_regions.push(vk::BufferCopy {
                    src_offset: (i * size_of::<LightSendable>()) as u64,
                    dst_offset: (light_id * size_of::<LightSendable>()) as u64,
                    size: size_of::<LightSendable>() as u64,
                });
                let light = &self.light_components[light_id];
                let transform = &self.transforms[light.transform];
                new_light_data.push(light.to_sendable(transform));
            }
            self.dirty_light_components.clear();
            copy_data_to_memory(world.lights_staging_buffer.2, &new_light_data);
            if !copy_regions.is_empty() {
                for frame in 0..MAX_FRAMES_IN_FLIGHT {
                    copy_buffer_synchronous(
                        &self.context.device,
                        command_buffer,
                        &world.lights_staging_buffer.0,
                        &world.lights_buffers[frame].0,
                        Some(copy_regions.clone()),
                        &0u64
                    );
                }
            }
        }
    }
    pub fn update_entity(
        &mut self,
        frame: usize,
        parent_transform: usize,
        entity_index: usize,
        dirty_primitive_instance_data: &mut Vec<Instance>
    ) {
        let (transform, children_indices) = {
            let entity = &self.entities[entity_index];

            if entity.ui_layout.is_some() { return }

            let entity_world_transform = if entity.transform == parent_transform {
                let entity_transform = &mut self.transforms[entity.transform];

                entity_transform.update_local_matrix();

                entity_transform.local
            } else {
                if entity.animated_transform.1 {
                    let [parent_transform, entity_transform, animated_transform] = self.transforms
                        .get_disjoint_mut(
                            [parent_transform, entity.transform, entity.animated_transform.0]
                        ).unwrap();

                    animated_transform.update_local_matrix();
                    entity_transform.update_world_matrix(parent_transform, Some(animated_transform));

                    entity_transform.world
                } else {
                    let [parent_transform, entity_transform] = self.transforms
                        .get_disjoint_mut(
                            [parent_transform, entity.transform]
                        ).unwrap();

                    entity_transform.update_local_matrix();
                    entity_transform.update_world_matrix(parent_transform, None);

                    entity_transform.world
                }
            };

            for (i, render_object_index) in entity.render_objects.iter().enumerate() {
                let render_component = &self.mesh_components[*render_object_index];

                self.transforms[render_component.transform].update_local_matrix();

                let render_component_transform = &mut self.transforms[render_component.transform];
                render_component_transform.world = entity_world_transform * render_component_transform.local;

                self.dirty_render_components.push(*render_object_index);
                dirty_primitive_instance_data.push(
                    Instance {
                        matrix: entity_world_transform.data,
                        indices: [
                            render_component.material_index as i32,
                            render_component.skin_index.map_or(-1, |i| i),
                            entity_index as i32,
                            i as i32
                        ]
                    }
                );
            }

            if let Some(body_index) = entity.rigid_body {
                let body = &mut self.rigid_body_components[body_index];
                body.initialize(&mut self.transforms);
            }

            for light_index in &entity.lights {
                self.dirty_light_components.push(*light_index);
            }

            for camera_index in &entity.cameras {
                self.dirty_camera_components.push(*camera_index);
            }

            (entity.transform, entity.children_indices.clone())
        };

        for child in children_indices {
            self.update_entity(frame, transform, child, dirty_primitive_instance_data)
        }
    }

    pub fn draw(&self, scene_renderer: &SceneRenderer, frame: usize, camera: Option<usize>, draw_mode: DrawMode) {
        let command_buffer = get_command_buffer();
        let world = &self.world.borrow();
        let camera = camera.map(|i| &self.camera_components[i]);
        unsafe {
            let (do_deferred, do_forward, do_outline, do_hitboxes) = match draw_mode {
                DrawMode::Deferred => (true, false, false, false),
                DrawMode::Forward => (false, true, false, false),
                DrawMode::All => (true, true, false, false),
                DrawMode::Outlined => (false, false, true, false),
                DrawMode::Hitboxes => (false, false, false, true),
            };

            if do_hitboxes {
                self.context.device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[scene_renderer.editor_primitives_vertices_buffer.0],
                    &[0],
                );
                self.context.device.cmd_bind_index_buffer(
                    command_buffer,
                    scene_renderer.editor_primitives_indices_buffer.0,
                    0,
                    vk::IndexType::UINT32,
                );

                self.context.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    scene_renderer.opaque_forward_renderpass.pipelines[2].vulkan_pipeline,
                );
                let view_projection = camera.unwrap().projection_matrix * camera.unwrap().view_matrix;
                for rigid_body_index in self.outlined_bodies.iter() {
                    let body = &self.rigid_body_components[*rigid_body_index];
                    let hitbox = &self.hitbox_components[body.hitbox].hitbox;
                    let ((index_count, first_index), model_matrix) = match hitbox {
                        Hitbox::OBB(obb, ..) => {
                            (scene_renderer.editor_primitives_index_info[0],
                             Matrix::new_translation_vec3(&(body.x_f + obb.center.rotate_by_quat(&body.q_f))) *
                             Matrix::new_rotate_quaternion_vec4(&body.q_f) *
                             Matrix::new_scale_vec3(&(obb.half_extents))
                            )
                        },
                        Hitbox::Sphere(sphere, ..) => {
                            (scene_renderer.editor_primitives_index_info[1],
                             Matrix::new_translation_vec3(&(body.x_f + sphere.center.rotate_by_quat(&body.q_f))) *
                                 Matrix::new_rotate_quaternion_vec4(&body.q_f) *
                                 Matrix::new_scale_vec3(&Vector::fill(sphere.radius))
                            )
                        },
                        _ => ((0, 0), Matrix::new())
                    };

                    self.context.device.cmd_push_constants(
                        command_buffer,
                        scene_renderer.opaque_forward_renderpass.pipeline_layout,
                        ShaderStageFlags::ALL_GRAPHICS,
                        0,
                        slice::from_raw_parts(
                            &CameraMatrixUniformData {
                                view: (view_projection * model_matrix).data,
                                projection: [0.5, 0.7, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
                            } as *const CameraMatrixUniformData as *const u8,
                            128
                        )
                    );

                    self.context.device.cmd_draw_indexed(command_buffer, index_count, 1, first_index, 0 ,0);
                }
            } else {
                self.context.device.cmd_bind_vertex_buffers(
                    command_buffer,
                    1,
                    &[world.instance_buffers[frame].0],
                    &[0],
                );
                self.context.device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0,
                    &[world.vertex_buffer.0],
                    &[0],
                );
                self.context.device.cmd_bind_index_buffer(
                    command_buffer,
                    world.index_buffer.0,
                    0,
                    vk::IndexType::UINT32,
                );

                if do_outline {
                    self.context.device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        scene_renderer.opaque_forward_renderpass.pipelines[0].vulkan_pipeline,
                    );
                    for index in self.outlined_components.iter() {
                        self.mesh_components[*index].draw(&self, scene_renderer, &command_buffer, world, *index, camera);
                    }
                } else {
                    if do_deferred {
                        for (i, render_component) in self.mesh_components.iter().enumerate() {
                            render_component.draw(&self, scene_renderer, &command_buffer, world, i, camera);
                        }
                    }
                    if do_forward {

                    }
                }
            }
        }
    }
}
pub enum DrawMode {
    Deferred,
    Forward,
    All,
    Outlined,
    Hitboxes,
}

pub struct Entity {
    pub name: String,
    pub transform: usize,
    pub animated_transform: (usize, bool),
    pub children_indices: Vec<usize>,
    pub parent: usize,

    pub suns: Vec<usize>,

    pub render_objects: Vec<usize>,
    pub joint_object: Option<usize>,
    pub animation_objects: Vec<usize>,
    pub rigid_body: Option<usize>,
    pub cameras: Vec<usize>,
    pub lights: Vec<usize>,

    pub ui_layout: Option<usize>,
    pub ui_interactable_information: Option<usize>,
    pub ui_quads: Vec<usize>,
    pub ui_texts: Vec<usize>,
    pub ui_images: Vec<usize>,
    pub ui_textures: Vec<usize>,

    pub scripts: Vec<usize>,
}
impl Default for Entity {
    fn default() -> Self {
        Self {
            name: String::from("entity"),
            transform: 0,
            animated_transform: (0, false),
            children_indices: Vec::new(),
            parent: 0,
            suns: Vec::new(),
            render_objects: Vec::new(),
            joint_object: None,
            animation_objects: Vec::new(),
            rigid_body: None,
            cameras: Vec::new(),
            lights: Vec::new(),
            scripts: Vec::new(),
            ui_layout: None,
            ui_interactable_information: None,
            ui_images: Vec::new(),
            ui_quads: Vec::new(),
            ui_texts: Vec::new(),
            ui_textures: Vec::new(),
        }
    }
}

struct CollisionConstraint {
    body_a: usize,
    body_b: usize,
    penetration: f32,
    normal: Vector,
    pt_on_a: Vector,
    pt_on_b: Vector,
}
impl CollisionConstraint {
    fn solve(&self, dt: f32, bodies: &mut Vec<RigidBodyComponent>) {
        let [body_a, body_b] = bodies.get_disjoint_mut([self.body_a, self.body_b]).unwrap();

        body_a.apply_correction(dt, self.normal * self.penetration, 0.0, self.pt_on_a, self.pt_on_b, Some(body_b));
    }
}


pub struct CanvasComponent {
    
}

#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct Instance {
    pub matrix: [f32; 16],
    pub indices: [i32; 4], // material id, skin id, owner entity id, component child id
}
impl Instance {
    pub fn new(matrix: Matrix, material: u32, skin: i32, owner_entity: u32, component_number: u32) -> Self {
        Self {
            matrix: matrix.data,
            indices: [material as i32, skin, owner_entity as i32, component_number as i32],
        }
    }
}

pub fn world_position_to_local(world_pos: Vector, parent: &Transform) -> Vector {
    ((world_pos - parent.world_translation) / parent.world_scale)
        .rotate_by_quat(&parent.world_rotation.inverse_quat())
}
pub fn linear_displacement_to_local(dx: Vector, parent: &Transform) -> Vector {
    (dx / parent.world_scale).rotate_by_quat(&parent.world_rotation.inverse_quat())
}
pub fn angular_displacement_to_local(dq: Vector, parent: &Transform) -> Vector {
    parent.world_rotation.inverse_quat().combine(&dq.combine(&parent.world_rotation))
}