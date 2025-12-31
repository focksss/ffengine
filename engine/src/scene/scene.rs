use std::cell::RefCell;
use std::ops::Add;
use std::sync::Arc;
use std::time::SystemTime;
use ash::{vk, Device};
use ash::vk::CommandBuffer;
use crate::engine::get_command_buffer;
use crate::math::matrix::Matrix;
use crate::math::Vector;
use crate::render::render::{Renderer, MAX_FRAMES_IN_FLIGHT};
use crate::render::scene_renderer::SceneRenderer;
use crate::render::vulkan_base::{copy_buffer_synchronous, copy_data_to_memory, Context, VkBase};
use crate::scene::physics::hitboxes::bounding_box::BoundingBox;
use crate::scene::physics::hitboxes::capsule::Capsule;
use crate::scene::physics::hitboxes::convex_hull::ConvexHull;
use crate::scene::physics::hitboxes::hitbox::{Hitbox, HitboxType};
use crate::scene::physics::hitboxes::mesh::MeshCollider;
use crate::scene::physics::hitboxes::sphere::Sphere;
use crate::scene::physics::physics_engine::{AxisType, ContactInformation, ContactPoint, PhysicsEngine};
use crate::scene::world::camera::Frustum;
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
                translation: node.translation,
                rotation: node.rotation,
                scale: node.scale,
                ..Default::default()
            });

            let node_anim_transform_index = self.transforms.len();
            self.transforms.push(Transform {
                owner: node_entity_index,
                translation: node.translation,
                rotation: node.rotation,
                scale: node.scale,
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
            body.hitbox = self.hitbox_components.len();
            body.position = transform * Vector::new4(0.0, 0.0, 0.0, 1.0);
            body.orientation = transform.extract_quaternion();
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

            body.set_static(&self.hitbox_components[body.hitbox].hitbox, is_static);
            body.set_mass(&self.hitbox_components[body.hitbox].hitbox, 1.0);
        }
        for child_index in entity.children_indices.clone() {
            self.add_rigid_body_from_entity(child_index, hitbox_type, is_static);
        }
    }

    pub fn update_physics_objects(&mut self, delta_time: f32) {
        let gravity = Vector::new3(0.0, -9.8, 0.0);
        // apply gravity
        for body in &mut self.rigid_body_components {
            if body.is_static {
                continue;
            } else {
                body.apply_impulse(gravity * delta_time * body.mass, body.get_center_of_mass_world_space());
            }
        }
        // collision detection
        let num_bodies = self.rigid_body_components.len();
        let mut contacts = Vec::new();
        for i in 0..num_bodies {
            for j in i + 1..num_bodies {
                let (a, b) = self.rigid_body_components.split_at_mut(j);
                let body_a = &mut a[i];
                let body_b = &mut b[0];

                if body_a.is_static && body_b.is_static {
                    continue;
                }

                if let Some(contact) = body_a.will_collide_with(&self.hitbox_components, body_b, delta_time) {
                    if !contact.contact_points.is_empty() {
                        contacts.push((contact, (i, j)));
                    }
                }
            }
        }
        contacts.sort_by(|a, b| a.0.time_of_impact.partial_cmp(&b.0.time_of_impact).unwrap());
        let mut accum_time = 0.0;
        for contact in &contacts {
            let (i, j) = contact.1;

            let collision = &contact.0;
            let dt = collision.time_of_impact - accum_time;

            for body in &mut self.rigid_body_components {
                if !body.is_static {
                    body.update(dt);
                }
            }

            let (first_idx, second_idx) = if i < j { (i, j) } else { (j, i) };
            let (left, right) = self.rigid_body_components.split_at_mut(second_idx);
            let body_a = &mut left[first_idx];
            let body_b = &mut right[0];


            let normal = collision.normal;
            let deepest = collision.contact_points.iter().max_by(
                |a_point, b_point|
                    a_point.penetration.partial_cmp(&b_point.penetration).unwrap()
            ).unwrap();
            let depth = deepest.penetration;
            let im_a = body_a.inv_mass; let im_b = body_b.inv_mass;
            let s_im = im_a + im_b;
            let restitution = body_a.restitution_coefficient * body_b.restitution_coefficient;
            let inv_inertia_a = body_a.get_inverse_inertia_tensor_world_space();
            let inv_inertia_b = body_b.get_inverse_inertia_tensor_world_space();

            let pt_on_a = collision.contact_points[0].point_on_a;
            let pt_on_b = collision.contact_points[0].point_on_b;
            let ra = pt_on_a - body_a.get_center_of_mass_world_space();
            let rb = pt_on_b - body_b.get_center_of_mass_world_space();

            let angular_j_a = (inv_inertia_a * ra.cross(&normal)).cross(&ra);
            let angular_j_b = (inv_inertia_b * rb.cross(&normal)).cross(&rb);
            let angular_factor = (angular_j_a + angular_j_b).dot3(&normal);

            let vel_a = body_a.velocity + body_a.angular_velocity.cross(&ra);
            let vel_b = body_b.velocity + body_b.angular_velocity.cross(&rb);

            let v_diff = vel_a - vel_b;

            let j = normal * (1.0 + restitution) * v_diff.dot3(&normal) / (s_im + angular_factor);
            body_a.apply_impulse(-j, pt_on_a);
            body_b.apply_impulse(j, pt_on_b);

            let friction = body_a.friction_coefficient * body_b.friction_coefficient;
            let velocity_normal = normal * normal.dot3(&v_diff);
            let velocity_tangent = v_diff - velocity_normal;

            let relative_tangent_vel = velocity_tangent.normalize3();
            let inertia_a = (inv_inertia_a * ra.cross(&relative_tangent_vel)).cross(&ra);
            let inertia_b = (inv_inertia_b * rb.cross(&relative_tangent_vel)).cross(&rb);
            let inv_inertia = (inertia_a + inertia_b).dot3(&relative_tangent_vel);

            let mass_reduc = 1.0 / (s_im + inv_inertia);
            let friction_impulse = velocity_tangent * mass_reduc * friction;

            body_a.apply_impulse(-friction_impulse, pt_on_a);
            body_b.apply_impulse(friction_impulse, pt_on_b);

            if collision.time_of_impact == 0.0 {
                let t_a = im_a / s_im;
                let t_b = im_b / s_im;

                let ds = pt_on_b - pt_on_a;
                body_a.position += ds * t_a;
                body_b.position -= ds * t_b;
            }
            accum_time += dt;
        }
        // apply velocity
        for body in &mut self.rigid_body_components {
            if !body.is_static {
                body.update(delta_time - accum_time);
                let entity_index = body.owner;
                let transform = &mut self.transforms[self.entities[entity_index].transform];
                transform.translation = body.position;
                transform.rotation = body.orientation;
                self.unupdated_entities.push(entity_index);
            }
        }
    }
    pub unsafe fn update_scene(&mut self, frame: usize, delta_time: f32) {
        if frame == 0 {
            if self.running {
                for animation in self.animation_components.iter_mut() {
                    animation.update(&mut self.entities, &mut self.transforms, &mut self.unupdated_entities)
                }

                self.update_physics_objects(delta_time);
            }

            let mut dirty_primitive_instance_data: Vec<Instance> = Vec::new();
            for entity_index in self.unupdated_entities.clone().iter() {
                //for entity_index in &vec![1usize] {
                let parent_index = self.entities[*entity_index].parent;
                let parent_transform = if *entity_index == 0 {
                    Matrix::new()
                } else {
                    self.transforms[self.entities[parent_index].transform].world.clone()
                };
                self.update_entity(
                    frame,
                    &parent_transform,
                    *entity_index,
                    &mut dirty_primitive_instance_data
                )
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

                let command_buffer = get_command_buffer();
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
        parent_world_transform: &Matrix,
        entity_index: usize,
        dirty_primitive_instance_data: &mut Vec<Instance>
    ) {
        let entity = &self.entities[entity_index];
        let entity_local_transform = {
            let entity_transform_component = &mut self.transforms[entity.transform];
            entity_transform_component.update_matrix();
            &entity_transform_component.matrix
        };

        let entity_world_transform = if entity.animated_transform.1 {
            let animated_transform = &mut self.transforms[entity.animated_transform.0];
            animated_transform.update_matrix();
            parent_world_transform * animated_transform.matrix
        } else {
            parent_world_transform * entity_local_transform
        };
        self.transforms[entity.transform].world = entity_world_transform;

        for (i, render_object_index) in entity.render_objects.iter().enumerate() {
            let render_component = &self.render_components[*render_object_index];

            self.transforms[render_component.transform].update_matrix();

            let render_component_transform = &mut self.transforms[render_component.transform];
            render_component_transform.world = entity_world_transform * render_component_transform.matrix;

            self.dirty_render_components.push(*render_object_index);
            dirty_primitive_instance_data.push(
                Instance {
                    matrix: render_component_transform.world.data,
                    indices: [
                        render_component.material_index as i32,
                        render_component.skin_index.map_or(-1, |i| i),
                        entity_index as i32,
                        i as i32
                    ]
                }
            );
        }

        for child in entity.children_indices.clone().iter() {
            self.update_entity(frame, &entity_world_transform, *child, dirty_primitive_instance_data)
        }
    }

    pub unsafe fn draw(&self, scene_renderer: &SceneRenderer, frame: usize, frustum: Option<&Frustum>, draw_mode: DrawMode) {
        let command_buffer = get_command_buffer();
        let world = &self.world.borrow();
        unsafe {
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

            let (do_deferred, do_forward, do_outline) = match draw_mode {
                DrawMode::Deferred => (true, false, false),
                DrawMode::Forward => (false, true, false),
                DrawMode::All => (true, true, false),
                DrawMode::Outlined => (false, false, true),
            };

            if do_outline {
                self.context.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    scene_renderer.opaque_forward_renderpass.pipelines[0].vulkan_pipeline,
                );
                for index in self.outlined_components.iter() {
                    self.render_components[*index].draw(&self, scene_renderer, &command_buffer, world, *index, frustum);
                }
            } else {
                if do_deferred {
                    for (i, render_component) in self.render_components.iter().enumerate() {
                        render_component.draw(&self, scene_renderer, &command_buffer, world, i, frustum);
                    }
                }
                if do_forward {

                }
            }
        }
    }
}
pub enum DrawMode {
    Deferred,
    Forward,
    All,
    Outlined
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

    pub translation: Vector,
    pub rotation: Vector,
    pub scale: Vector,

    matrix: Matrix,

    world: Matrix
}
impl Transform {
    fn update_matrix(&mut self) {
        let rotate = Matrix::new_rotate_quaternion_vec4(&self.rotation);
        let scale = Matrix::new_scale_vec3(&self.scale);
        let translate = Matrix::new_translation_vec3(&self.translation);

        self.matrix = translate * rotate * scale;
    }
}
impl Default for Transform {
    fn default() -> Self {
        Transform {
            owner: 0,
            is_identity: true,

            translation: Vector::new(),
            rotation: Vector::new(),
            scale: Vector::fill(1.0),

            matrix: Matrix::new(),

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
                animated_transform.translation = new_vector
            } else if channel.2.eq("rotation") {
                animated_transform.rotation = new_vector
            } else if channel.2.eq("scale") {
                animated_transform.scale = new_vector
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
    pub owned_by_player: bool,

    pub hitbox: usize,
    pub is_static: bool,
    pub restitution_coefficient: f32,
    pub friction_coefficient: f32,
    pub mass: f32,
    pub inv_mass: f32,

    pub force: Vector,
    pub torque: Vector,

    pub position: Vector,
    pub velocity: Vector,
    pub orientation: Vector, // quaternion
    pub angular_velocity: Vector,
    center_of_mass: Vector,

    inertia_tensor: Matrix, // 3x3
    inv_inertia_tensor: Matrix,

    stored_hitbox_scale: Vector,
}
impl RigidBodyComponent {
    pub fn set_mass(&mut self, hitbox: &Hitbox, mass: f32) {
        self.mass = mass;
        self.inv_mass = 1.0 / mass;
        self.update_shape_properties(hitbox);
    }
    pub fn set_static(&mut self, hitbox: &Hitbox, is_static: bool) {
        self.is_static = is_static;
        if is_static {
            self.inv_mass = 0.0;
            self.mass = 999.0
        } else {
            self.inv_mass = 1.0 / self.mass;
        }
        self.update_shape_properties(hitbox);
    }

    pub fn update_shape_properties(&mut self, hitbox: &Hitbox) {
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
    pub fn get_inverse_inertia_tensor_world_space(&self) -> Matrix {
        let rot = Matrix::new_rotate_quaternion_vec4(&self.orientation);
        rot * self.inv_inertia_tensor * rot.transpose3()
    }
    pub fn get_center_of_mass_world_space(&self) -> Vector {
        self.position + Matrix::new_rotate_quaternion_vec4(&self.orientation) * self.center_of_mass
    }

    pub fn update(&mut self, delta_time: f32) {
        self.position += self.velocity * delta_time;

        let c = self.get_center_of_mass_world_space();
        let c_to_pos = self.position - c;

        let rot = Matrix::new_rotate_quaternion_vec4(&self.orientation);
        let inertia_world = rot * self.inertia_tensor * rot.transpose3();
        let inv_inertia_world = rot * self.inv_inertia_tensor * rot.transpose3();
        let torque = self.angular_velocity.cross(&(inertia_world * self.angular_velocity));
        let alpha = inv_inertia_world * torque;
        self.angular_velocity += alpha * delta_time;

        let d_theta = self.angular_velocity * delta_time;
        let dq = Vector::axis_angle_quat(&d_theta, d_theta.magnitude3());
        self.orientation = dq.combine(&self.orientation).normalize4();

        self.position = c + Matrix::new_rotate_quaternion_vec4(&dq) * c_to_pos;
    }

    pub fn apply_impulse(&mut self, impulse: Vector, point: Vector) {
        if self.inv_mass == 0.0 { return }

        self.velocity += impulse * self.inv_mass;

        let c = self.get_center_of_mass_world_space();
        let r = point - c;
        let dl = r.cross(&impulse);
        if self.owned_by_player { return }
        self.apply_angular_impulse(dl);
    }
    pub fn apply_angular_impulse(&mut self, impulse: Vector) {
        if self.inv_mass == 0.0 { return }
        self.angular_velocity += self.get_inverse_inertia_tensor_world_space() * impulse;

        const MAX_ANGULAR_SPEED: f32 = 30.0;
        if self.angular_velocity.magnitude3() > MAX_ANGULAR_SPEED {
            self.angular_velocity = self.angular_velocity.normalize3() * MAX_ANGULAR_SPEED;
        }
    }

    pub fn will_collide_with(&mut self, hitbox_components: &Vec<HitboxComponent>, other: &mut RigidBodyComponent, dt: f32) -> Option<ContactInformation> {
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
        &mut self,
        self_hitbox: &Hitbox,
        other_hitbox: &Hitbox,
        other: &mut RigidBodyComponent,
        dt: f32
    ) -> Option<ContactInformation> {
        if let Hitbox::Sphere(sphere) = other_hitbox {
            return match self_hitbox {
                Hitbox::OBB(obb, _) => {
                    let obb_position = self.position;
                    let sphere_position = other.position;
                    let obb_orientation = self.orientation;
                    let sphere_orientation = other.orientation;
                    let rot = Matrix::new_rotate_quaternion_vec4(&obb_orientation);

                    let sphere_center = sphere.center.rotate_by_quat(&sphere_orientation) + sphere_position;
                    let obb_center = (rot * obb.center.with('w', 1.0)) + obb_position;

                    let axes = [
                        rot * Vector::new4(1.0, 0.0, 0.0, 1.0),
                        rot * Vector::new4(0.0, 1.0, 0.0, 1.0),
                        rot * Vector::new4(0.0, 0.0, 1.0, 1.0),
                    ];

                    let delta = sphere_center - obb_center;
                    let local_sphere_center = Vector::new3(delta.dot3(&axes[0]), delta.dot3(&axes[1]), delta.dot3(&axes[2]));

                    let closest_local = local_sphere_center.clamp3(&(-obb.half_extents), &obb.half_extents);
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
                        time_of_impact: 0.0
                    })
                }
                Hitbox::Sphere(a) => {
                    let p_a = a.center.rotate_by_quat(&self.orientation) + self.position;
                    let p_b = sphere.center.rotate_by_quat(&other.orientation) + other.position;

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
                        self.update(time_of_impact);
                        other.update(time_of_impact);

                        let normal = (self.position - other.position).normalize3();

                        self.update(-time_of_impact);
                        other.update(-time_of_impact);

                        let ab = other.position - self.position;
                        let r = ab.magnitude3() - (a.radius + sphere.radius);

                        Some(ContactInformation {
                            contact_points: vec![point],
                            time_of_impact,
                            normal,
                        })
                    } else {
                        None
                    }
                }
                _ => { None }
            }
        }
        None
    }
    pub fn intersects_obb(
        &mut self,
        self_hitbox: &Hitbox,
        other_hitbox: &Hitbox,
        other: &mut RigidBodyComponent,
        dt: f32
    ) -> Option<ContactInformation> {
        if let Hitbox::OBB(obb, _) = other_hitbox {
            return match self_hitbox {
                Hitbox::OBB(this_obb, _) => {
                    let (a, b) = (this_obb, obb);

                    let a_center = a.center.rotate_by_quat(&self.orientation) + self.position;
                    let b_center = b.center.rotate_by_quat(&other.orientation) + other.position;

                    let a_quat = self.orientation;
                    let b_quat = other.orientation;

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

                    if collision_normal.dot4(&t) < 0.0 {
                        collision_normal = -collision_normal;
                    }

                    Some(ContactInformation {
                        contact_points: vec![ContactPoint {
                            point_on_a: a_center + collision_normal * (1.0 - min_penetration),
                            point_on_b: b_center - collision_normal * (1.0 - min_penetration),
                            penetration: min_penetration
                        }],
                        normal: collision_normal,
                        time_of_impact: 0.0,
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
        &mut self,
        self_hitbox: &Hitbox,
        other_hitbox: &Hitbox,
        other: &mut RigidBodyComponent,
        dt: f32
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
            hitbox: 0,
            is_static: true,
            restitution_coefficient: 0.5,
            friction_coefficient: 0.5,
            mass: 999.0,
            inv_mass: 0.0,
            force: Default::default(),
            torque: Default::default(),
            position: Default::default(),
            velocity: Default::default(),
            orientation: Vector::new(),
            angular_velocity: Default::default(),
            center_of_mass: Vector::new(),
            inertia_tensor: Matrix::new(),
            inv_inertia_tensor: Matrix::new(),
            stored_hitbox_scale: Vector::fill(1.0),
        }
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
        frustum: Option<&Frustum>
    ) {
        let mut all_points_outside_of_same_plane = false;

        let primitive = &world.meshes[self.mesh_primitive_index.0].primitives[self.mesh_primitive_index.1];

        if frustum.is_some() {
            for plane_idx in 0..6 {
                let mut all_outside_this_plane = true;

                for corner in primitive.corners.iter() {
                    let world_pos = scene.transforms[self.transform].world * Vector::new4(corner.x, corner.y, corner.z, 1.0);

                    if frustum.unwrap().planes[plane_idx].test_point_within(&world_pos) {
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
        if !all_points_outside_of_same_plane || frustum.is_none() {
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