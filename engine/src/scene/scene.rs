use std::cell::RefCell;
use std::ops::Add;
use std::slice;
use std::sync::Arc;
use std::time::SystemTime;
use ash::{vk, Device};
use ash::vk::{CommandBuffer, ShaderStageFlags};
use parry3d::na::Quaternion;
use crate::engine::get_command_buffer;
use crate::math::matrix::Matrix;
use crate::math::Vector;
use crate::render::render::{Renderer, MAX_FRAMES_IN_FLIGHT};
use crate::render::scene_renderer::{CameraMatrixUniformData, SceneRenderer};
use crate::render::vulkan_base::{copy_buffer_synchronous, copy_data_to_memory, Context, VkBase};
use crate::scene::physics::hitboxes::bounding_box::BoundingBox;
use crate::scene::physics::hitboxes::capsule::Capsule;
use crate::scene::physics::hitboxes::convex_hull::ConvexHull;
use crate::scene::physics::hitboxes::hitbox::{Hitbox, HitboxType};
use crate::scene::physics::hitboxes::hitbox::HitboxType::OBB;
use crate::scene::physics::hitboxes::mesh::MeshCollider;
use crate::scene::physics::hitboxes::sphere::Sphere;
use crate::scene::physics::physics_engine::{AxisType, ContactInformation, ContactPoint, PhysicsEngine};
use crate::scene::world::camera::{Camera, Frustum};
use crate::scene::world::world::{World};


//TODO
// - EVERYTHING is contained by an entity and is a type of component, including gui elements and rendering pipelines


pub struct Scene {
    context: Arc<Context>,

    pub running: bool,

    pub entities: Vec<Entity>, // will always have a root node with sun

    pub unupdated_entities: Vec<usize>,

    pub transforms: Vec<Transform>,
    pub render_components: Vec<RenderComponent>,
    pub skin_components: Vec<SkinComponent>,
    pub animation_components: Vec<AnimationComponent>,
    pub rigid_body_components: Vec<RigidBodyComponent>,
    pub hitbox_components: Vec<HitboxComponent>,
    pub camera_components: Vec<CameraComponent>,
    pub light_components: Vec<LightComponent>,

    pub outlined_components: Vec<usize>,
    pub outlined_bodies: Vec<usize>,

    pub renderer: Arc<RefCell<Renderer>>,
    pub world: Arc<RefCell<World>>,
    pub physics_engine: Arc<RefCell<PhysicsEngine>>,

    dirty_render_components: Vec<usize>,
}
impl Scene {
    pub fn new(context: &Arc<Context>, renderer: Arc<RefCell<Renderer>>, world: Arc<RefCell<World>>, physics_engine: Arc<RefCell<PhysicsEngine>>) -> Self {
        let mut scene = Self {
            context: context.clone(),

            running: false,

            entities: Vec::new(),
            unupdated_entities: Vec::new(),
            transforms: Vec::new(),
            render_components: Vec::new(),
            skin_components: Vec::new(),
            animation_components: Vec::new(),
            rigid_body_components: Vec::new(),
            hitbox_components: Vec::new(),
            camera_components: Vec::new(),
            light_components: Vec::new(),
            outlined_components: Vec::new(),
            outlined_bodies: Vec::new(),
            renderer,
            world,
            physics_engine,
            dirty_render_components: Vec::new(),
        };
        scene.transforms.push(Transform::default());
        scene.entities.push(Entity {
            name: String::from("Scene"),
            transform: 0,
            children_indices: Vec::new(),
            parent: 0,

            sun: Some(SunComponent {
                direction: Vector::new3(-1.0, -5.0, -1.0),
                color: Vector::new3(0.98, 0.84, 0.64)
            }),
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

                    let render_component_index = self.render_components.len();
                    self.render_components.push(RenderComponent {
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

    pub fn add_rigid_body_from_entity(&mut self, entity_index: usize, hitbox_type: usize, is_static: bool) {
        assert!(hitbox_type < 5);

        let entity = &mut self.entities[entity_index];

        if !entity.render_objects.is_empty() {
            let render_component = &self.render_components[entity.render_objects[0]];
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

    pub unsafe fn update_scene(&mut self, command_buffer: CommandBuffer, frame: usize, delta_time: f32, force_run: bool) {
        if self.running || force_run {
            self.update_physics_objects(delta_time);
        }
        if frame == 0 {
            if self.running || force_run {
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
                let render_component = &self.render_components[*render_object_index];

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

            (entity.transform, entity.children_indices.clone())
        };

        for child in children_indices {
            self.update_entity(frame, transform, child, dirty_primitive_instance_data)
        }
    }

    pub unsafe fn draw(&self, scene_renderer: &SceneRenderer, frame: usize, camera: Option<&Camera>, draw_mode: DrawMode) {
        let command_buffer = get_command_buffer();
        let world = &self.world.borrow();
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
                        self.render_components[*index].draw(&self, scene_renderer, &command_buffer, world, *index, camera);
                    }
                } else {
                    if do_deferred {
                        for (i, render_component) in self.render_components.iter().enumerate() {
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

    pub sun: Option<SunComponent>,
    pub render_objects: Vec<usize>,
    pub joint_object: Option<usize>,
    pub animation_objects: Vec<usize>,
    pub rigid_body: Option<usize>,
    pub camera: Option<usize>,
    pub light: Option<usize>,
}
impl Default for Entity {
    fn default() -> Self {
        Self {
            name: String::from("entity"),
            transform: 0,
            animated_transform: (0, false),
            children_indices: Vec::new(),
            parent: 0,
            sun: None,
            render_objects: Vec::new(),
            joint_object: None,
            animation_objects: Vec::new(),
            rigid_body: None,
            camera: None,
            light: None,
        }
    }
}
pub struct Transform {
    is_identity: bool,
    pub owner: usize,

    pub local_translation: Vector,
    pub local_rotation: Vector,
    pub local_scale: Vector,

    local: Matrix,

    pub world_translation: Vector,
    pub world_rotation: Vector,
    pub world_scale: Vector,

    world: Matrix
}
impl Transform {
    fn update_local_matrix(&mut self) {
        let rotate = Matrix::new_rotate_quaternion_vec4(&self.local_rotation);
        let scale = Matrix::new_scale_vec3(&self.local_scale);
        let translate = Matrix::new_translation_vec3(&self.local_translation);

        self.local = translate * rotate * scale;
    }
    fn update_world_matrix(&mut self, parent_transform: &Transform, animated_transform: Option<&Transform>) {
        if let Some(animated) = animated_transform {
            self.world_scale = parent_transform.world_scale * animated.local_scale;
            self.world_rotation = parent_transform.world_rotation.combine(&animated.local_rotation);
            self.world_translation = parent_transform.world_translation +
                (animated.local_translation * parent_transform.world_scale)
                    .rotate_by_quat(&parent_transform.world_rotation);
            self.world = parent_transform.world * animated.local;
        } else {
            self.world_scale = parent_transform.world_scale * self.local_scale;
            self.world_rotation = parent_transform.world_rotation.combine(&self.local_rotation);
            self.world_translation = parent_transform.world_translation +
                (self.local_translation * parent_transform.world_scale)
                    .rotate_by_quat(&parent_transform.world_rotation);
            self.world = parent_transform.world * self.local;
        }
    }

    fn local_to_world_position(&self, position: Vector) -> Vector {
        self.world_translation + (position * self.world_scale).rotate_by_quat(&self.world_rotation)
    }
    fn world_to_local_position(&self, position: Vector, parent: &Transform) -> Vector {
        ((position - parent.world_translation) / parent.world_scale).rotate_by_quat(&parent.world_rotation.inverse_quat())
    }

    fn local_to_world_rotation(&self, rotation: Vector) -> Vector {
        self.world_rotation * rotation
    }
    fn world_to_local_rotation(&self, rotation: Vector, parent: &Transform) -> Vector {
        parent.world_rotation.inverse_quat().combine(&rotation)
    }
}
impl Default for Transform {
    fn default() -> Self {
        Transform {
            owner: 0,
            is_identity: true,

            local_translation: Vector::new(),
            local_rotation: Vector::new4(0.0, 0.0, 0.0, 1.0),
            local_scale: Vector::fill(1.0),

            local: Matrix::new(),

            world_translation: Vector::new(),
            world_rotation: Vector::new4(0.0, 0.0, 0.0, 1.0),
            world_scale: Vector::fill(1.0),

            world: Matrix::new(),
        }
    }
}



pub struct AnimationComponent {
    owner_entity: usize,
    pub channels: Vec<(usize, usize, String)>, // sampler index, impacted node, target transform component
    pub samplers: Vec<(Vec<f32>, String, Vec<Vector>)>, // input times, interpolation method, output vectors
    pub start_time: SystemTime,
    pub duration: f32,
    pub running: bool,
    pub repeat: bool,
    pub snap_back: bool,
}
impl AnimationComponent {
    pub fn start(&mut self) {
        self.start_time = SystemTime::now();
        self.running = true;
    }

    pub fn stop(&mut self, entities: &mut Vec<Entity>) {
        self.running = false;
        if self.snap_back {
            for channel in self.channels.iter() {
                entities[channel.1].animated_transform.1 = false;
            }
        }
    }

    pub fn update(&mut self, entities: &mut Vec<Entity>, transforms: &mut Vec<Transform>, unnupdated_entities: &mut Vec<usize>) {
        if !self.running {
            return
        }
        unnupdated_entities.push(self.owner_entity);
        let current_time = SystemTime::now();
        let elapsed_time = current_time.duration_since(self.start_time).unwrap().as_secs_f32();
        let mut repeat = false;
        if elapsed_time > self.duration {
            if self.repeat {
                repeat = true
            } else {
                self.stop(entities);
                return
            }
        }
        for channel in self.channels.iter() {
            let sampler = &self.samplers[channel.0];
            let mut current_time_index = 0;
            for i in 0..sampler.0.len() - 1 {
                if elapsed_time >= sampler.0[i] && elapsed_time < sampler.0[i + 1] {
                    current_time_index = i;
                    break
                }
            }
            let current_time_index = current_time_index.min(sampler.0.len() - 1);
            let interpolation_factor = ((elapsed_time - sampler.0[current_time_index]) / (sampler.0[current_time_index + 1] - sampler.0[current_time_index])).min(1.0).max(0.0);
            let vector1 = &sampler.2[current_time_index];
            let vector2 = &sampler.2[current_time_index + 1];
            let new_vector;
            if channel.2.eq("translation") || channel.2.eq("scale") {
                new_vector = Vector::new3(
                    vector1.x + interpolation_factor * (vector2.x - vector1.x),
                    vector1.y + interpolation_factor * (vector2.y - vector1.y),
                    vector1.z + interpolation_factor * (vector2.z - vector1.z),
                )
            } else {
                new_vector = Vector::spherical_lerp(vector1, vector2, interpolation_factor)
            }

            let entity = &mut entities[channel.1];
            entity.animated_transform.1 = true;
            let animated_transform = &mut transforms[entity.animated_transform.0];

            if channel.2.eq("translation") {
                animated_transform.local_translation = new_vector
            } else if channel.2.eq("rotation") {
                animated_transform.local_rotation = new_vector
            } else if channel.2.eq("scale") {
                animated_transform.local_scale = new_vector
            } else {
                panic!("Illogical animation channel target! Should be translation, rotation or scale");
            }
        }
        if repeat {
            self.start()
        }
    }
}
pub struct SkinComponent {
    joints: Vec<usize>, // entity indices
    inverse_bind_matrices: Vec<Matrix>
}
impl SkinComponent {
    pub fn update(&self, scene: &Scene, joints: &mut Vec<Matrix>) {
        for (i, joint_entity_index) in self.joints.iter().enumerate() {
            let world_transform = scene.transforms[scene.entities[*joint_entity_index].transform].world;
            joints.push(world_transform * self.inverse_bind_matrices[i]);
        }
    }
}
pub struct RigidBodyComponent {
    pub owner: usize,
    pub transform: usize,
    pub owned_by_player: bool,

    pub hitbox: usize,
    pub is_static: bool,
    pub restitution_coefficient: f32,
    pub friction_coefficient: f32,
    pub mass: f32,
    pub inv_mass: f32,

    pub x_i: Vector,
    pub x_f: Vector,
    pub q_i: Vector,
    pub q_f: Vector,

    pub velocity: Vector,
    pub angular_velocity: Vector, // axis angle
    pub differential_rotation: Vector, // quaternion
    center_of_mass: Vector,

    inertia_tensor: Matrix, // 3x3
    inv_inertia_tensor: Matrix,

    stored_hitbox_scale: Vector,
}
impl RigidBodyComponent {
    pub fn set_mass(&mut self, hitbox: &Hitbox, transforms: &Vec<Transform>, mass: f32) {
        self.mass = mass;
        self.inv_mass = 1.0 / mass;
        self.update_shape_properties(hitbox, transforms);
    }
    pub fn set_static(&mut self, hitbox: &Hitbox, transforms: &Vec<Transform>, is_static: bool) {
        self.is_static = is_static;
        if is_static {
            self.inv_mass = 0.0;
            self.mass = 999.0
        } else {
            self.inv_mass = 1.0 / self.mass;
        }
        self.update_shape_properties(hitbox, transforms);
    }

    pub fn update_shape_properties(&mut self, hitbox: &Hitbox, transforms: &Vec<Transform>) {
        match &hitbox {
            Hitbox::OBB(obb, _) => {
                let a = obb.half_extents.x * 2.0;
                let b = obb.half_extents.y * 2.0;
                let c = obb.half_extents.z * 2.0;


                self.inertia_tensor = Matrix::new();
                self.inertia_tensor.set(0, 0, (1.0 / 12.0) * (b * b + c * c));
                self.inertia_tensor.set(1, 1, (1.0 / 12.0) * (a * a + c * c));
                self.inertia_tensor.set(2, 2, (1.0 / 12.0) * (a * a + b * b));

                let d = -obb.center;
                let d2 = d.dot3(&d);

                // parallel axis theorem tensor
                let pat = Matrix::new_manual([
                    d2 - d.x*d.x, -d.x*d.y,     -d.x*d.z,     0.0,
                    -d.y*d.x,     d2 - d.y*d.y, -d.y*d.z,     0.0,
                    -d.z*d.x,     -d.z*d.y,     d2 - d.z*d.z, 0.0,
                    0.0,          0.0,          0.0,          0.0
                ]);

                self.inertia_tensor += pat;

                self.center_of_mass = obb.center
            }
            Hitbox::Mesh(_) => {
                //TODO inertia tensor and center of mass
            }
            Hitbox::Capsule(capsule) => {
                //TODO inertia tensor

                self.center_of_mass = (capsule.a + capsule.b) * 0.5
            }
            Hitbox::Sphere(sphere) => {
                let r2 = sphere.radius * sphere.radius;
                let c = 2.0 / 5.0;
                let v = c * r2;
                self.inertia_tensor = Matrix::new();
                self.inertia_tensor.set(0, 0, v);
                self.inertia_tensor.set(1, 1, v);
                self.inertia_tensor.set(2, 2, v);

                self.center_of_mass = sphere.center
            }
            Hitbox::ConvexHull(convex) => {
                self.center_of_mass = convex.center_of_mass(10);
                self.inertia_tensor = convex.inertia_tensor(&self.center_of_mass, 10);
            }
        }
        self.inv_inertia_tensor = self.inertia_tensor.inverse3().mul_float_into3(self.inv_mass);
    }
    pub fn get_inverse_inertia_tensor_world_space(&self, transforms: &Vec<Transform>) -> Matrix {
        let transform = &transforms[self.transform];
        let rot = Matrix::new_rotate_quaternion_vec4(&transform.world_rotation);
        rot * self.inv_inertia_tensor * rot.transpose3()
    }
    pub fn get_center_of_mass_world_space(&self, transforms: &Vec<Transform>) -> Vector {
        let transform = &transforms[self.transform];
        transform.world_translation + self.center_of_mass.rotate_by_quat(&transform.world_rotation)
    }
    pub fn get_inverse_mass_world_space(&self, normal: &Vector, position: Option<&Vector>) -> f32 {
        if self.is_static || self.inv_mass == 0.0 { return 0.0 }

        let inv_rot = Matrix::new_rotate_quaternion_vec4(&self.q_f.inverse_quat());
        let rn = if let Some(position) = position {
            inv_rot * ((position - self.x_f).cross(normal).with('w', 0.0))
        } else {
            inv_rot * normal.with('w', 0.0)
        };

        let inv_inertia = Vector::new3(self.inv_inertia_tensor.data[0], self.inv_inertia_tensor.data[5], self.inv_inertia_tensor.data[10]);
        let mut w = rn.dot3(&(rn * inv_inertia));

        if position.is_some() {
            w += self.inv_mass
        }
        w
    }

    pub fn initialize(&mut self, transforms: &Vec<Transform>) {
        let transform = &transforms[self.transform];
        self.x_f = transform.world_translation;
        self.q_f = transform.world_rotation;
    }
    pub fn integrate(&mut self, dt: f32, g: &Vector) {
        if self.is_static { return }

        self.x_i = self.x_f;
        self.q_i = self.q_f;

        self.velocity += g * dt;
        self.x_f += self.velocity * dt;

        self.differential_rotation = self.angular_velocity.with('w', 0.0).combine(&self.q_f);
        self.q_f += self.differential_rotation * (0.5 * dt);
        self.q_f = self.q_f.normalize4();
    }
    pub fn update_velocity(&mut self, dt: f32) {
        if self.is_static { return }

        self.velocity = (self.x_f - self.x_i) / dt;

        self.differential_rotation = self.q_f.combine(&self.q_i.inverse_quat());
        self.angular_velocity = self.differential_rotation.with('w', 0.0) * 2.0 / dt;
        if self.differential_rotation.w < 0.0 { self.angular_velocity = -self.angular_velocity }
    }
    pub fn update(&mut self, transforms: &mut Vec<Transform>, parent_transform: usize) {
        if self.is_static { return }

        let [transform, parent_transform] = transforms.get_disjoint_mut([self.transform, parent_transform]).unwrap();

        transform.local_translation = transform.world_to_local_position(self.x_f, parent_transform);
        transform.local_rotation = transform.world_to_local_rotation(self.q_f, parent_transform);
    }
    pub fn apply_correction(
        &mut self,
        dt: f32,
        correction: Vector,
        compliance: f32,
        pos_a: Vector,
        pos_b: Vector,
        body_b: Option<&mut RigidBodyComponent>
    ) -> Vector {
        if correction.magnitude3_sq() == 0.0 { return Vector::empty() }

        let c = correction.magnitude3();
        let mut n = correction.normalize3();
        let mut w = self.get_inverse_mass_world_space(&n, Some(&pos_a));
        if let Some(body_b) = &body_b {
            w += body_b.get_inverse_mass_world_space(&n, Some(&pos_b));
        }
        if w == 0.0 { return Vector::empty() }

        // XPBD
        let alpha = compliance / dt / dt;
        let lambda = -c / (w + alpha);
        n = n * -lambda;

        let correct = |body: &mut RigidBodyComponent, correction: &Vector, position: &Vector| {
            if body.is_static || body.inv_mass == 0.0 { return }

            body.x_f += correction * body.inv_mass;

            let inv_inertia = Vector::new3(body.inv_inertia_tensor.data[0], body.inv_inertia_tensor.data[5], body.inv_inertia_tensor.data[10]);
            let d_angular_vel = ((position - body.x_f)
                .cross(correction)
                .with('w', 0.0)
                .rotate_by_quat(&body.q_f.inverse_quat())
                * inv_inertia)
                .with('w', 0.0)
                .rotate_by_quat(&body.q_f);
            body.differential_rotation = d_angular_vel.with('w', 0.0).combine(&body.q_f);
            body.q_f += (0.5 * body.differential_rotation).normalize4();
        };

        correct(self, &-n, &pos_a);
        if let Some(body_b) = body_b {
            correct(body_b, &n, &pos_b);
        }
        lambda * correction / dt / dt
    }

    pub fn will_collide_with(
        &self,
        hitbox_components: &Vec<HitboxComponent>,
        other: &RigidBodyComponent,
        dt: f32,
    ) -> Option<ContactInformation> {
        let self_hitbox = &hitbox_components[self.hitbox].hitbox;
        let other_hitbox = &hitbox_components[other.hitbox].hitbox;
        let other_type = other_hitbox.get_type();

        match other_type {
            HitboxType::SPHERE => self.intersects_sphere(self_hitbox, other_hitbox, other, dt),
            HitboxType::OBB => self.intersects_obb(self_hitbox, other_hitbox, other, dt),
            HitboxType::CONVEX => self.intersects_convex_hull(self_hitbox, other_hitbox, other, dt),
            _ => { panic!("intersection not implemented") }
        }
    }

    fn intersects_sphere(
        &self,
        self_hitbox: &Hitbox,
        other_hitbox: &Hitbox,
        other: &RigidBodyComponent,
        dt: f32,
    ) -> Option<ContactInformation> {
        if let Hitbox::Sphere(sphere) = other_hitbox {
            return match self_hitbox {
                Hitbox::Sphere(a) => {
                    let p_a = a.center.rotate_by_quat(&self.q_f) + self.x_f;
                    let p_b = sphere.center.rotate_by_quat(&other.q_f) + other.x_f;
                    /*
                                        if let Some((point, time_of_impact)) = {
                                            let relative_vel = self.velocity - other.velocity;

                                            let end_pt_a = p_a + relative_vel * dt;
                                            let dir = end_pt_a - p_a;

                                            let mut t0 = 0.0;
                                            let mut t1 = 0.0;
                                            if dir.magnitude3() < 0.001 {
                                                let ab = p_b - p_a;
                                                let radius = a.radius + sphere.radius + 0.001;
                                                if ab.magnitude3() > radius {
                                                    return None
                                                }
                                            } else if let Some((i_t0, i_t1)) = Sphere::ray_sphere(&p_a, &dir, &p_b, a.radius + sphere.radius) {
                                                t0 = i_t0;
                                                t1 = i_t1;
                                            } else {
                                                return None
                                            }

                                            // convert 0-1 to 0-dt
                                            t0 *= dt;
                                            t1 *= dt;

                                            // collision happened in past
                                            if t1 < 0.0 { return None }

                                            let toi = if t0 < 0.0 { 0.0 } else { t0 };

                                            // collision happens past dt
                                            if toi > dt { return None }

                                            let new_pos_a = p_a + self.velocity * toi;
                                            let new_pos_b = p_b + other.velocity * toi;

                                            let ab = (new_pos_b - new_pos_a).normalize3();

                                            Some((ContactPoint {
                                                point_on_a: new_pos_a + ab * a.radius,
                                                point_on_b: new_pos_b - ab * sphere.radius,

                                                penetration: 0.0,
                                            }, toi))
                                        } {
                                            // there will be a collision
                                            /*
                                            self.update(time_of_impact);
                                            other.update(time_of_impact);

                                             */

                                            let normal = (self_transform.world_translation - other_transform.world_translation).normalize3();

                                            /*
                                            self.update(-time_of_impact);
                                            other.update(-time_of_impact);

                                             */

                                            let ab = other_transform.world_translation - self_transform.world_translation;
                                            let r = ab.magnitude3() - (a.radius + sphere.radius);

                                            Some(ContactInformation {
                                                contact_points: vec![point],
                                                time_of_impact,
                                                normal,
                                            })
                                        } else {
                                            None
                                        }

                     */
                    let d = p_b - p_a;
                    let d_m = d.magnitude3();
                    let n = if d_m > 1e-6 { d / d_m } else { Vector::new3(0.0, 1.0, 0.0) };

                    if d_m > a.radius + sphere.radius { return None }

                    let point_on_a = p_a + n * a.radius;
                    let point_on_b = p_b - n * sphere.radius;

                    let penetration = a.radius + sphere.radius - d_m;

                    Some(ContactInformation {
                        contact_points: vec![ContactPoint {
                            point_on_a,
                            point_on_b,
                            penetration,
                        }],
                        normal: n,
                        time_of_impact: penetration,
                    })
                }
                Hitbox::OBB(obb, _) => {
                    let rot = Matrix::new_rotate_quaternion_vec4(&self.q_f);

                    let sphere_center = sphere.center.rotate_by_quat(&other.q_f) + other.x_f;
                    let obb_center = (rot * obb.center.with('w', 1.0)) + self.x_f;

                    let axes = [
                        rot * Vector::new4(1.0, 0.0, 0.0, 1.0),
                        rot * Vector::new4(0.0, 1.0, 0.0, 1.0),
                        rot * Vector::new4(0.0, 0.0, 1.0, 1.0),
                    ];

                    let delta = sphere_center - obb_center;
                    let local_sphere_center = Vector::new3(delta.dot3(&axes[0]), delta.dot3(&axes[1]), delta.dot3(&axes[2]));

                    let closest_local = local_sphere_center.clamp3(&(-1.0 * obb.half_extents), &obb.half_extents);
                    let closest_world = obb_center + (axes[0] * closest_local.x) + (axes[1] * closest_local.y) + (axes[2] * closest_local.z);

                    let diff = sphere_center - closest_world;
                    let dist_sq = diff.dot3(&diff);
                    let radius_sq = sphere.radius * sphere.radius;
                    if dist_sq > radius_sq {
                        return None
                    }
                    let dist = dist_sq.sqrt();

                    let (normal, penetration, point_on_obb) = if dist < 1e-6 {
                        let penetrations = [
                            (obb.half_extents.x - local_sphere_center.x.abs(), 0, local_sphere_center.x.signum()),
                            (obb.half_extents.y - local_sphere_center.y.abs(), 1, local_sphere_center.y.signum()),
                            (obb.half_extents.z - local_sphere_center.z.abs(), 2, local_sphere_center.z.signum()),
                        ];
                        let mut min_pen = penetrations[0];
                        for &pen in &penetrations[1..] {
                            if pen.0 < min_pen.0 {
                                min_pen = pen;
                            }
                        }

                        let axis_idx = min_pen.1;
                        let sign = min_pen.2;
                        let normal = axes[axis_idx] * sign;
                        let penetration = min_pen.0 + sphere.radius;

                        let mut local_point = local_sphere_center;
                        match axis_idx {
                            0 => local_point.x = obb.half_extents.x * sign,
                            1 => local_point.y = obb.half_extents.y * sign,
                            2 => local_point.z = obb.half_extents.z * sign,
                            _ => unreachable!(),
                        }

                        let point_on_obb = obb_center + (
                            axes[0] * local_point.x + axes[1] * local_point.y + axes[2] * local_point.z
                        );

                        (normal, penetration, point_on_obb)
                    } else {
                        let normal = diff * (1.0 / dist);
                        let penetration = sphere.radius - dist;
                        (normal, penetration, closest_world)
                    };

                    let point_on_sphere = sphere_center - (normal * sphere.radius);

                    let tolerance = 1e-4;
                    let mut contact_points = vec![ContactPoint {
                        point_on_a: point_on_obb,
                        point_on_b: point_on_sphere,
                        penetration,
                    }];

                    let on_face_x = (local_sphere_center.x.abs() - obb.half_extents.x).abs() < tolerance;
                    let on_face_y = (local_sphere_center.y.abs() - obb.half_extents.y).abs() < tolerance;
                    let on_face_z = (local_sphere_center.z.abs() - obb.half_extents.z).abs() < tolerance;

                    let faces_count = on_face_x as u8 + on_face_y as u8 + on_face_z as u8;

                    // additional contact points for edge/corner
                    if faces_count >= 2 && dist > 1e-6 {
                        let tangent1 = if normal.x.abs() < 0.9 {
                            Vector::new3(1.0, 0.0, 0.0).cross(&normal).normalize3()
                        } else {
                            Vector::new3(0.0, 1.0, 0.0).cross(&normal).normalize3()
                        };
                        let tangent2 = normal.cross(&tangent1).normalize3();

                        // contact points in a circle around the main contact
                        let num_additional = if faces_count == 3 { 3 } else { 2 }; // corner/edge
                        for i in 1..=num_additional {
                            let angle = (i as f32) * std::f32::consts::PI * 2.0 / (num_additional + 1) as f32;
                            let offset = tangent1 * (angle.cos() * 0.01) + tangent2 * (angle.sin() * 0.01);

                            contact_points.push(ContactPoint {
                                point_on_a: point_on_obb + offset,
                                point_on_b: point_on_sphere + offset,
                                penetration,
                            });
                        }
                    }

                    Some(ContactInformation {
                        contact_points,
                        normal,
                        time_of_impact: penetration,
                    })
                }
                _ => { None }
            }
        }
        None
    }
    pub fn intersects_obb(
        &self,
        self_hitbox: &Hitbox,
        other_hitbox: &Hitbox,
        other: &RigidBodyComponent,
        dt: f32,
    ) -> Option<ContactInformation> {
        if let Hitbox::OBB(obb, _) = other_hitbox {
            return match self_hitbox {
                Hitbox::OBB(this_obb, _) => {
                    let (a, b) = (this_obb, obb);

                    let a_center = a.center.rotate_by_quat(&self.q_f) + self.x_f;
                    let b_center = b.center.rotate_by_quat(&other.q_f) + other.x_f;

                    let a_quat = self.q_f;
                    let b_quat = other.q_f;

                    let a_axes = [
                        Vector::new3(1.0, 0.0, 0.0).rotate_by_quat(&a_quat),
                        Vector::new3(0.0, 1.0, 0.0).rotate_by_quat(&a_quat),
                        Vector::new3(0.0, 0.0, 1.0).rotate_by_quat(&a_quat),
                    ];
                    let b_axes = [
                        Vector::new3(1.0, 0.0, 0.0).rotate_by_quat(&b_quat),
                        Vector::new3(0.0, 1.0, 0.0).rotate_by_quat(&b_quat),
                        Vector::new3(0.0, 0.0, 1.0).rotate_by_quat(&b_quat),
                    ];

                    let t = b_center - a_center;

                    let mut min_penetration = f32::MAX;
                    let mut collision_normal = Vector::empty();
                    let mut best_axis_type = AxisType::FaceA(0);
                    for i in 0..3 {
                        { // a_normals
                            let axis = a_axes[i];
                            let penetration = test_axis(&axis, &t, &a.half_extents, &b.half_extents, &a_axes, &b_axes, true);
                            if penetration < 0.0 {
                                return None
                            }
                            if penetration < min_penetration {
                                min_penetration = penetration;
                                collision_normal = axis;
                                best_axis_type = AxisType::FaceA(i);
                            }
                        }
                        { // b_normals
                            let axis = b_axes[i];
                            let penetration = test_axis(&axis, &t, &a.half_extents, &b.half_extents, &a_axes, &b_axes, true);
                            if penetration < 0.0 {
                                return None
                            }
                            if penetration < min_penetration {
                                min_penetration = penetration;
                                collision_normal = axis;
                                best_axis_type = AxisType::FaceB(i);
                            }
                        }
                        for j in 0..3 { // edge-edge cross-products
                            let axis = a_axes[i].cross(&b_axes[j]);
                            let axis_length = axis.magnitude3();

                            if axis_length < 1e-6 {
                                continue;
                            }

                            let axis_normalized = axis / axis_length;
                            let penetration = test_axis_cross(&axis_normalized, &t, &a.half_extents, &b.half_extents, &a_axes, &b_axes);

                            if penetration < 0.0 {
                                return None;
                            }

                            if penetration < min_penetration {
                                min_penetration = penetration;
                                collision_normal = axis_normalized;
                                best_axis_type = AxisType::Edge(i, j);
                            }
                        }
                    }

                    if collision_normal.dot3(&t) < 0.0 {
                        collision_normal = -collision_normal;
                    }

                    Some(ContactInformation {
                        contact_points: vec![ContactPoint {
                            point_on_a: a_center + t.normalize3() * a.half_extents.magnitude3(),
                            point_on_b: b_center - t.normalize3() * b.half_extents.magnitude3(),
                            penetration: min_penetration
                        }],
                        normal: collision_normal,
                        time_of_impact: min_penetration,
                    })
                }
                Hitbox::Sphere(_) => {
                    if let Some(contact) = other.intersects_sphere(other_hitbox, self_hitbox, self, dt) {
                        Some(contact.flip())
                    } else { None }
                }
                _ => None
            }
        }
        None
    }
    fn intersects_convex_hull(
        &self,
        self_hitbox: &Hitbox,
        other_hitbox: &Hitbox,
        other: &RigidBodyComponent,
        dt: f32,
    ) -> Option<ContactInformation> {
        // TODO
        None
    }
}
impl Default for RigidBodyComponent {
    fn default() -> Self {
        Self {
            owned_by_player: false,
            owner: 0,
            transform: 0,
            hitbox: 0,
            is_static: true,
            restitution_coefficient: 0.5,
            friction_coefficient: 0.5,
            mass: 999.0,
            inv_mass: 0.0,
            x_i: Vector::new(),
            x_f: Vector::new(),
            q_i: Vector::new(),
            q_f: Vector::new(),
            velocity: Default::default(),
            angular_velocity: Default::default(),
            differential_rotation: Default::default(),
            center_of_mass: Vector::new(),
            inertia_tensor: Matrix::new(),
            inv_inertia_tensor: Matrix::new(),
            stored_hitbox_scale: Vector::fill(1.0),
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
pub struct HitboxComponent {
    pub hitbox: Hitbox,
}

pub struct RenderComponent {
    mesh_primitive_index: (usize, usize), // world mesh, mesh-primitive index
    transform: usize, // independent from parent
    skin_index: Option<i32>,
    material_index: usize,
}
impl RenderComponent {
    unsafe fn draw(
        &self,
        scene:
        &Scene,
        scene_renderer: &SceneRenderer,
        command_buffer: &CommandBuffer,
        world: &World,
        index: usize,
        camera: Option<&Camera>
    ) {
        let mut all_points_outside_of_same_plane = false;

        let primitive = &world.meshes[self.mesh_primitive_index.0].primitives[self.mesh_primitive_index.1];

        if camera.is_some() {
            for plane_idx in 0..6 {
                let mut all_outside_this_plane = true;

                for corner in primitive.corners.iter() {
                    let world_pos = scene.transforms[self.transform].world * Vector::new4(corner.x, corner.y, corner.z, 1.0);

                    if camera.unwrap().frustum.planes[plane_idx].test_point_within(&world_pos) {
                        all_outside_this_plane = false;
                        break;
                    }
                }
                if all_outside_this_plane {
                    all_points_outside_of_same_plane = true;
                    break;
                }
            }
        }
        if !all_points_outside_of_same_plane || camera.is_none() {
            unsafe {
                scene.context.device.cmd_draw_indexed(
                    *command_buffer,
                    world.accessors[primitive.indices].count as u32,
                    1,
                    primitive.index_buffer_offset as u32,
                    0,
                    index as u32,
                );
            }
        }
    }
}
pub struct CameraComponent {
    pub camera_index: usize,
}
pub struct LightComponent {
    pub light_index: usize,
}
pub struct SunComponent {
    pub direction: Vector,
    pub color: Vector,
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

fn test_axis(
    axis: &Vector,
    t: &Vector,
    half_a: &Vector,
    half_b: &Vector,
    axes_a: &[Vector; 3],
    axes_b: &[Vector; 3],
    is_a: bool,
) -> f32 {
    let ra = if is_a {
        half_a.x * axes_a[0].dot3(axis).abs() +
            half_a.y * axes_a[1].dot3(axis).abs() +
            half_a.z * axes_a[2].dot3(axis).abs()
    } else {
        half_a.x * axes_a[0].dot3(axis).abs() +
            half_a.y * axes_a[1].dot3(axis).abs() +
            half_a.z * axes_a[2].dot3(axis).abs()
    };
    let rb = half_b.x * axes_b[0].dot3(axis).abs() +
        half_b.y * axes_b[1].dot3(axis).abs() +
        half_b.z * axes_b[2].dot3(axis).abs();
    let distance = t.dot3(axis).abs();
    ra + rb - distance
}
fn test_axis_cross(
    axis: &Vector,
    t: &Vector,
    half_a: &Vector,
    half_b: &Vector,
    axes_a: &[Vector; 3],
    axes_b: &[Vector; 3],
) -> f32 {
    let ra = half_a.x * axes_a[0].dot3(axis).abs() +
        half_a.y * axes_a[1].dot3(axis).abs() +
        half_a.z * axes_a[2].dot3(axis).abs();

    let rb = half_b.x * axes_b[0].dot3(axis).abs() +
        half_b.y * axes_b[1].dot3(axis).abs() +
        half_b.z * axes_b[2].dot3(axis).abs();

    let distance = t.dot3(axis).abs();

    ra + rb - distance
}
fn signed_volume1(a: Vector, b: Vector) -> (f32, f32) {
    let ab = b - a; // segment
    let ap = Vector::fill(0.0) - a; // a to origin
    let p = a + ab * ab.dot3(&ap) / ab.magnitude3_sq(); // origin projected onto the segment

    // get axis with the greatest length
    let mut index = 0;
    let mut max_mu: f32 = 0.0;
    for i in 0..3 {
        let mu = ab.get(i);
        if mu.abs() > max_mu.abs() {
            max_mu = mu;
            index = i;
        }
    }
    // project all to found axis
    let a = a.get(index);
    let b = b.get(index);
    let p = p.get(index);
    // signed distance from a to p and p to b
    let c0 = p - a;
    let c1 = b - p;
    // p on segment
    if (p > a && p < b) || (p < a && p > b) {
        return (c1 / max_mu, c0 / max_mu)
    }
    // p off the segment on a-side
    if (a <= b && p <= a) || (a >= b && p <= a) {
        return (1.0, 0.0)
    }
    // p off the segment on b-side
    (0.0, 1.0)
}
fn signed_volume2(a: Vector, b: Vector, c: Vector) -> Vector {
    let n = (b - a).cross(&(c - a)); // normal
    let p = n * a.dot3(&n) / n.magnitude3_sq(); // origin projected onto the triangle

    // find axis with the largest area
    let mut index = 0;
    let mut max_area: f32 = 0.0;
    for i in 0..3 {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;

        let a = Vector::new2(a.get(j), a.get(k));
        let b = Vector::new2(b.get(j), b.get(k));
        let c = Vector::new2(c.get(j), c.get(k));
        let ab = b - a;
        let ac = c - a;
        let area = ab.x * ac.y - ab.y * ac.x;
        if area.abs() > max_area.abs() {
            max_area = area;
            index = i;
        }
    }

    // project onto axis with the largest area
    let x = (index + 1) % 3;
    let y = (index + 2) % 3;
    let vertices = [
        Vector::new2(a.get(x), a.get(y)),
        Vector::new2(b.get(x), b.get(y)),
        Vector::new2(c.get(x), c.get(y)),
    ];
    let p = Vector::new2(p.get(x), p.get(y));

    // areas of all tris formed by projected origin and edges
    let mut areas = Vector::empty();
    for i in 0..3 {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;

        let a = p;
        let b = vertices[j];
        let c = vertices[k];
        let ab = b - a;
        let ac = c - a;
        areas.set(i, ab.x * ac.y - ab.y * ac.x);
    }

    // if the projected origin is inside the triangle, return barycentric coords of it
    if same_sign(max_area, areas.x) && same_sign(max_area, areas.y) && same_sign(max_area, areas.z) {
        return areas / max_area
    }

    // project onto edges, return barycentric coords of the closest point to origin
    let mut min_dist = f32::MAX;
    let mut lambdas = Vector::new3(1.0, 0.0, 0.0);
    for i in 0..3 {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;

        let edge_points = [a, b, c];
        let lambda_edge = signed_volume1(edge_points[j], edge_points[k]);
        let point = edge_points[j] * lambda_edge.0 + edge_points[k] * lambda_edge.1;
        let dist = point.magnitude3_sq();
        if dist < min_dist {
            min_dist = dist;
            lambdas.set(i, 0.0);
            lambdas.set(j, lambda_edge.0);
            lambdas.set(k, lambda_edge.1);
        }
    }
    lambdas
}
fn signed_volume3(a: Vector, b: Vector, c: Vector, d: Vector) -> Vector {
    let m = Matrix::new_manual([
        a.x, b.x, c.x, d.x,
        a.y, b.y, c.y, d.y,
        a.z, b.z, c.z, d.z,
        1.0, 1.0, 1.0, 1.0,
    ]);
    let c = Vector::new4(
        m.cofactor(3, 0),
        m.cofactor(3, 1),
        m.cofactor(3, 2),
        m.cofactor(3, 3),
    );
    let det = c.x + c.y + c.z + c.w;

    // origin already inside tetrahedron
    if same_sign(det, c.x) && same_sign(det, c.y) && same_sign(det, c.z) && same_sign(det, c.w) {
        return c / det
    }

    // project origin onto faces, find closest, return its barycentric coords
    let mut lambdas = Vector::empty();
    let mut min_dist = f32::MAX;
    for i in 0..4 {
        let j = (i + 1) % 4;
        let k = (i + 2) % 4;

        let face_points = [a, b, c, d];
        let lambda_face = signed_volume2(face_points[i], face_points[j], face_points[k]);
        let point = face_points[i] * lambda_face.x + face_points[j] * lambda_face.y + face_points[k] * lambda_face.z;
        let dist = point.magnitude3_sq();
        if dist < min_dist {
            min_dist = dist;
            lambdas = lambda_face;
        }
    }

    lambdas
}
fn same_sign(a: f32, b: f32) -> bool {
    a*b > 0.0
}