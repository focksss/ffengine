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
use crate::scene::physics::physics_engine::PhysicsEngine;
use crate::scene::world::camera::Frustum;
use crate::scene::world::world::{World};

//TODO handle ALL updates + rendering from here (call to World + PhysicsEngine)
//TODO change World structure to have a single flat list of Primitives. Store Meshes here, Meshes have child primitives, and are RenderComponents.
//TODO refactor scene structure to allow multiple components of each type
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

    pub fn new_entity_from_model(&mut self, parent_index: usize, uri: &str) {
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

    pub unsafe fn update_scene(&mut self, frame: usize) {
        if frame == 0 {
            if self.running {
                for animation in self.animation_components.iter_mut() {
                    animation.update(&mut self.entities, &mut self.transforms, &mut self.unupdated_entities)
                }
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
                    scene_renderer.forward_renderpass.pipelines[0].vulkan_pipeline,
                );
                for index in self.outlined_components.iter() {
                    self.render_components[*index].draw(&self, scene_renderer, &command_buffer, world, *index, frustum);
                }
                self.context.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    scene_renderer.forward_renderpass.pipelines[1].vulkan_pipeline,
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
    rigid_body_index: usize, // physics object
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