use ash::vk::CommandBuffer;
use crate::math::Vector;
use crate::render::scene_renderer::SceneRenderer;
use crate::scene::components::camera::CameraComponent;
use crate::scene::scene::Scene;
use crate::scene::world::world::World;

pub struct Mesh {
    pub(crate) mesh_primitive_index: (usize, usize), // world mesh, mesh-primitive index
    pub(crate) transform: usize, // independent from parent
    pub(crate) skin_index: Option<i32>,
    pub(crate) material_index: usize,
}
impl Mesh {
    pub(crate) fn draw(
        &self,
        scene:
        &Scene,
        scene_renderer: &SceneRenderer,
        command_buffer: &CommandBuffer,
        world: &World,
        index: usize,
        camera: Option<&CameraComponent>
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