use crate::math::matrix::Matrix;
use crate::math::Vector;
use crate::render::scene_renderer::SHADOW_RES;
use crate::scene::components::camera::CameraComponent;
use crate::scene::world::world::SunSendable;

pub struct SunComponent {
    pub direction: Vector,
    pub color: Vector,
}
impl SunComponent {
    pub fn new_sun(vector: Vector, color: Vector) -> SunComponent {
        SunComponent {
            direction: Vector::new4(vector.x, vector.y, vector.z, 1.0).normalize3(),
            color,
        }
    }
    pub fn get_sendable(&self, primary_camera: &CameraComponent) -> SunSendable {
        let cascade_levels = [primary_camera.far * 0.005, primary_camera.far * 0.015, primary_camera.far * 0.045, primary_camera.far * 0.15];

        let mut views = Vec::new();
        let mut projections = Vec::new();
        for i in 0..cascade_levels.len() + 1 {
            let matrices: [Matrix; 2];
            if i == 0 {
                matrices = self.get_cascade_matrix(primary_camera, primary_camera.near, cascade_levels[i]);
            }
            else if i < cascade_levels.len() {
                matrices = self.get_cascade_matrix(primary_camera, cascade_levels[i - 1], cascade_levels[i]);
            }
            else {
                matrices = self.get_cascade_matrix(primary_camera, cascade_levels[i - 1], primary_camera.far.min(500.0));
            }
            views.push(matrices[0]);
            projections.push(matrices[1]);
        }

        let mut matrices = Vec::new();
        for i in 0..5 {
            matrices.push((projections[i] * views[i]).data);
        }
        SunSendable {
            matrices: <[[f32; 16]; 5]>::try_from(matrices.as_slice()).unwrap(),
            vector: self.direction.to_array3(),
            _pad0: 0u32,
            color: self.color.to_array3(),
            _pad1: 0u32,
        }
    }

    fn get_cascade_matrix(&self, camera: &CameraComponent, near: f32, far: f32) -> [Matrix; 2] {
        let corners = camera.get_frustum_corners_with_near_far(near, far);

        let mut sum = Vector::empty();
        for corner in corners.iter() {
            sum = sum + corner;
        }
        let mut frustum_center = sum / 8.0;
        frustum_center.w = 1.0;

        let mut max_radius_squared = 0.0f32;
        for corner in &corners {
            let v = *corner - frustum_center;
            let radius_squared = v.dot4(&v);
            if radius_squared > max_radius_squared { max_radius_squared = radius_squared; }
        }
        let radius = max_radius_squared.sqrt();

        let texels_per_unit = SHADOW_RES as f32 / (radius * 2.0);
        let scalar_matrix = Matrix::new_scalar(texels_per_unit);
        let temp_view = Matrix::new_look_at(
            &(1.0 * self.direction),
            &Vector::empty(),
            &Vector::new3(0.0, 1.0, 0.0)
        ) * scalar_matrix;
        frustum_center = temp_view * frustum_center;
        frustum_center.x = frustum_center.x.floor();
        frustum_center.y = frustum_center.y.floor();
        frustum_center.w = 1.0;
        frustum_center = temp_view.inverse4() * frustum_center;

        let view = Matrix::new_look_at(
            &(frustum_center - (-self.direction * 2.0 * radius)),
            &frustum_center,
            &Vector::new3(0.0, 1.0, 0.0)
        );

        let mut min_x = f32::MAX; let mut min_y = f32::MAX; let mut min_z = f32::MAX;
        let mut max_x = f32::MIN; let mut max_y = f32::MIN; let mut max_z = f32::MIN;
        for corner in &corners {
            let corner_light_space = view * corner;
            min_x = min_x.min(corner_light_space.x);
            min_y = min_y.min(corner_light_space.y);
            min_z = min_z.min(corner_light_space.z);
            max_x = max_x.max(corner_light_space.x);
            max_y = max_y.max(corner_light_space.y);
            max_z = max_z.max(corner_light_space.z);
        }

        let projection = Matrix::new_ortho(min_x, max_x, min_y, max_y, -radius * 6.0, radius * 6.0);

        [view, projection]
    }
}