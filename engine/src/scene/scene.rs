use std::cell::RefCell;
use std::ops::Add;
use std::sync::Arc;
use ash::{vk, Device};
use ash::vk::CommandBuffer;
use crate::engine::get_command_buffer;
use crate::math::matrix::Matrix;
use crate::math::Vector;
use crate::render::render::{Renderer, MAX_FRAMES_IN_FLIGHT};
use crate::render::vulkan_base::{copy_buffer_synchronous, VkBase};
use crate::scene::physics::physics_engine::PhysicsEngine;
use crate::scene::world::camera::Frustum;
use crate::scene::world::world::{Instance, World, MAX_INSTANCES};

//TODO handle ALL updates + rendering from here (call to World + PhysicsEngine)
//TODO change World structure to have a single flat list of Primitives. Store Meshes here, Meshes have child primitives, and are RenderComponents. 
pub struct Scene {
    pub entities: Vec<Entity>, // will always have a root node with sun

    pub unupdated_entities: Vec<usize>,

    pub transforms: Vec<Transform>,
    pub render_components: Vec<RenderComponent>,
    pub rigid_body_components: Vec<RigidBodyComponent>,
    pub camera_components: Vec<CameraComponent>,
    pub light_components: Vec<LightComponent>,

    pub renderer: Arc<RefCell<Renderer>>,
    pub world: Arc<RefCell<World>>,
    pub physics_engine: Arc<RefCell<PhysicsEngine>>,

    dirty_primitives: Vec<usize>,
}
impl Scene {
    pub fn new(renderer: Arc<RefCell<Renderer>>, world: Arc<RefCell<World>>, physics_engine: Arc<RefCell<PhysicsEngine>>) -> Self {
        let mut scene = Self {
            entities: Vec::new(),
            unupdated_entities: Vec::new(),
            transforms: Vec::new(),
            render_components: Vec::new(),
            rigid_body_components: Vec::new(),
            camera_components: Vec::new(),
            light_components: Vec::new(),
            renderer,
            world,
            physics_engine,
            dirty_primitives: Vec::new(),
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

    pub fn new_entity_from_model(&mut self, base: &VkBase, parent_index: usize, uri: &str) {
        let model_entity_index = self.entities.len();
        self.unupdated_entities.push(model_entity_index);
        let new_nodes = {
            let world = &mut self.world.borrow_mut();

            unsafe { world.add_model(base, uri) }

            let new_model = &world.models[world.models.len() - 1];

            let entity_transform_index = self.transforms.len();
            self.transforms.push(Transform::default());
            self.entities[parent_index].children_indices.push(model_entity_index);
            self.entities.push(Entity {
                name: String::from(uri),
                transform: entity_transform_index,
                parent: parent_index,
                ..Default::default()
            });

            world.scenes[new_model.scene].nodes.clone()
        };

        for node_index in new_nodes {
            self.implement_world_node(node_index, model_entity_index);
        }
    }
    fn implement_world_node(&mut self, node_index: usize, parent_index: usize) {
        let node_entity_index = self.entities.len();
        self.entities[parent_index].children_indices.push(node_entity_index);

        let child_nodes = {

            let world = &mut self.world.borrow_mut();
            let node = &world.nodes[node_index];

            let node_transform_index = self.transforms.len();
            self.transforms.push(Transform {
                translation: node.translation,
                rotation: node.rotation,
                scale: node.scale,
                ..Default::default()
            });
            self.entities.push(Entity {
                name: node.name.clone(),
                transform: node_transform_index,
                parent: parent_index,
                ..Default::default()
            });

            if let Some(mesh_index) = node.mesh {
                for (i, primitive) in world.meshes[mesh_index].primitives.iter().enumerate() {
                    let primitive_entity_index = self.entities.len();
                    self.entities[node_entity_index].children_indices.push(primitive_entity_index);

                    let primitive_entity_transform_index = self.transforms.len();
                    self.transforms.push(Transform::default());

                    let render_component_index = self.render_components.len();
                    self.render_components.push(RenderComponent {
                        mesh_primitive_index: (mesh_index, i),
                        skin_index: node.skin,
                        material_index: primitive.material_index as usize,
                        transform: primitive_entity_transform_index,
                    });

                    self.entities.push(Entity {
                        name: node.name.clone().add(format!(".primitive{}", primitive.id).as_str()),
                        transform: primitive_entity_transform_index,
                        parent: node_entity_index,
                        render_object: Some(render_component_index),
                        ..Default::default()
                    });
                }
            }
            node.children_indices.clone()
        };
        for child_node_index in child_nodes {
            self.implement_world_node(child_node_index, node_entity_index);
        }
    }

    pub unsafe fn update_scene(&mut self, base: &VkBase, frame: usize) {
        {
            // let mut world = self.world.borrow_mut();
            // for i in 0..world.animations.len() {
            //     let nodes = &mut world.nodes;
            //     world.animations[i].update(nodes);
            // }
        }
        if frame == 0 {
            let mut dirty_primitive_instance_data: Vec<Instance> = Vec::new();
            for entity_index in self.unupdated_entities.clone().iter() {
                self.update_entity(
                    base,
                    frame,
                    &self.transforms[0].matrix.clone(),
                    *entity_index,
                    &mut dirty_primitive_instance_data
                )
            }
            self.unupdated_entities.clear();

            let world = &self.world.borrow();
            unsafe {
                std::ptr::copy_nonoverlapping(
                    dirty_primitive_instance_data.as_ptr() as *const u8,
                    world.instance_staging_buffer.2 as *mut u8,
                    size_of::<Instance>() * dirty_primitive_instance_data.len(),
                );

                let mut copy_regions = Vec::new();
                for (i, &primitive_id) in self.dirty_primitives.iter().enumerate() {
                    copy_regions.push(vk::BufferCopy {
                        src_offset: (i * size_of::<Instance>()) as u64,
                        dst_offset: (primitive_id * size_of::<Instance>()) as u64,
                        size: size_of::<Instance>() as u64,
                    });
                }
                self.dirty_primitives.clear();

                let command_buffer = get_command_buffer();
                if !copy_regions.is_empty() {
                    for frame in 0..MAX_FRAMES_IN_FLIGHT {
                        copy_buffer_synchronous(
                            &base.device,
                            command_buffer,
                            &world.instance_staging_buffer.0,
                            &world.instance_buffers[frame].0,
                            Some(copy_regions.clone()),
                            &0u64
                        )
                    }
                }
            }
        }
    }
    pub fn update_entity(
        &mut self,
        base: &VkBase,
        frame: usize,
        parent_world_transform: &Matrix,
        entity: usize,
        dirty_primitive_instance_data: &mut Vec<Instance>
    ) {
        let entity = &self.entities[entity];
        let entity_transform_component = &mut self.transforms[entity.transform];
        entity_transform_component.update_matrix();
        let entity_local_transform = &entity_transform_component.matrix;

        let entity_world_transform = parent_world_transform * entity_local_transform;
        entity_transform_component.world = entity_world_transform;

        {
            let world = &mut self.world.borrow_mut();
            if let Some(render_object) = entity.render_object {
                let render_component = &self.render_components[render_object];
                let primitive = &world.meshes[render_component.mesh_primitive_index.0].primitives[render_component.mesh_primitive_index.1];
                self.dirty_primitives.push(primitive.id);
                dirty_primitive_instance_data.push(
                    Instance {
                        matrix: entity_world_transform.data,
                        indices: [
                            render_component.material_index as i32,
                            render_component.skin_index.map_or(-1, |i| i)
                        ]
                    }
                );
            }
        }

        for child in entity.children_indices.clone().iter() {
            self.update_entity(base, frame, &entity_world_transform, *child, dirty_primitive_instance_data)
        }
    }

    pub unsafe fn draw(&self, device: &Device, frame: usize, frustum: Option<&Frustum>) {
        let command_buffer = get_command_buffer();
        let world = &self.world.borrow();
        unsafe {
            device.cmd_bind_vertex_buffers(
                command_buffer,
                1,
                &[world.instance_buffers[frame].0],
                &[0],
            );
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[world.vertex_buffer.0],
                &[0],
            );
            device.cmd_bind_index_buffer(
                command_buffer,
                world.index_buffer.0,
                0,
                vk::IndexType::UINT32,
            );


            for render_component in self.render_components.iter() {
                render_component.draw(&self, device, &command_buffer, world, frustum);
            }
        }
    }
}

pub struct Entity {
    pub name: String,
    pub transform: usize,
    pub children_indices: Vec<usize>,
    pub parent: usize,

    pub sun: Option<SunComponent>,
    pub render_object: Option<usize>,
    pub rigid_body: Option<usize>,
    pub camera: Option<usize>,
    pub light: Option<usize>,
}
impl Default for Entity {
    fn default() -> Self {
        Self {
            name: String::from("entity"),
            transform: 0,
            children_indices: Vec::new(),
            parent: 0,
            sun: None,
            render_object: None,
            rigid_body: None,
            camera: None,
            light: None,
        }
    }
}
pub struct Transform {
    translation: Vector,
    rotation: Vector,
    scale: Vector,

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
            translation: Vector::new(),
            rotation: Vector::new(),
            scale: Vector::fill(1.0),

            matrix: Matrix::new(),

            world: Matrix::new(),
        }
    }
}
pub struct RigidBodyComponent {
    rigid_body_index: usize, // physics object
}
pub struct RenderComponent {
    mesh_primitive_index: (usize, usize), // world mesh, mesh-primitive index
    transform: usize, // shared with owner Entity, also here to allow for avoiding graph traversal during rendering
    skin_index: Option<i32>,
    material_index: usize,
}
impl RenderComponent {
    unsafe fn draw(&self, scene: &Scene, device: &Device, command_buffer: &CommandBuffer, world: &World, frustum: Option<&Frustum>) {
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
                device.cmd_draw_indexed(
                    *command_buffer,
                    world.accessors[primitive.indices].count as u32,
                    1,
                    primitive.index_buffer_offset as u32,
                    0,
                    primitive.id as u32,
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