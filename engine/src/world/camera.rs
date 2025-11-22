use std::cell::RefCell;
use std::sync::Arc;
use crate::math::matrix::Matrix;
use crate::math::Vector;
use crate::world::scene::Scene;

const PI: f32 = std::f32::consts::PI;

#[derive(Clone)]
pub struct CameraPointer {
    pub world: Arc<RefCell<Scene>>,
    pub index: usize,
}
#[derive(Clone, Debug)]
pub struct Camera {
    pub view_matrix: Matrix,
    pub projection_matrix: Matrix,
    pub position: Vector,
    pub target: Vector,
    pub rotation: Vector,
    pub fov_y: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
    pub frustum: Frustum,
    pub infinite_reverse: bool,

    pub third_person: bool,
    pub third_person_vector: Vector, // to be rotated by the cameras rotation and added to the position when creating the view matrix, if in third person
}
impl Camera {
    pub fn new_perspective_rotation(
        position: Vector,
        rotation: Vector,
        fov_y: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
        infinite_reverse: bool,
        third_person_vector: Vector
    ) -> Self {
        Self {
            view_matrix: Matrix::new(),
            projection_matrix: Matrix::new(),
            position,
            target: Vector::new_null(),
            rotation,
            fov_y,
            aspect_ratio,
            near,
            far,
            frustum: Frustum::null(),
            infinite_reverse,
            third_person: false,
            third_person_vector
        }
    }

    pub fn update_matrices(&mut self) {
        self.view_matrix = Matrix::new_view(
            if self.third_person {
                let third_person_position = &self.position + self.third_person_vector.rotate_by_euler(&self.rotation);
                third_person_position
            } else { self.position.clone() },
            &self.rotation
        );
        self.projection_matrix = if self.infinite_reverse { Matrix::new_infinite_reverse_projection(
            self.fov_y.to_radians(), 
            self.aspect_ratio,
            self.near,
        ) } else { Matrix::new_projection(
            self.fov_y.to_radians(),
            self.aspect_ratio,
            self.near,
            self.far,
        ) }
    }

    pub fn update_frustum(&mut self) {
        let rotation = &self.rotation.mul_by_vec(&Vector::new_vec3(-1.0, 1.0, 1.0)) + Vector::new_vec3(0.0,-PI,0.0);
        let cam_front = Vector::new_vec3(0.0,0.0,1.0).rotate_by_euler(&rotation);
        let cam_up = Vector::new_vec3(0.0,1.0,0.0).rotate_by_euler(&rotation);
        let cam_right = cam_up.cross(&cam_front).normalize_3d();

        let half_v = self.far * (self.fov_y*0.5).to_radians().tan();
        let half_h = half_v*self.aspect_ratio;

        let front_by_far = cam_front * self.far;

        let position = self.position;

        self.frustum = Frustum {
            planes: [
                Plane {
                    normal: cam_front,
                    point: position + (cam_front * self.near),
                },
                Plane {
                    normal: cam_front * -1.0,
                    point: position + front_by_far,
                },
                Plane {
                    normal: cam_up.cross(&(front_by_far - (cam_right * half_h))),
                    point: position,
                },
                Plane {
                    normal: (front_by_far + (cam_right * half_h)).cross(&cam_up),
                    point: position,
                },
                Plane {
                    normal: cam_right.cross(&(front_by_far + (cam_up * half_v))),
                    point: position,
                },
                Plane {
                    normal: (front_by_far - (cam_up * half_v)).cross(&cam_right),
                    point: position,
                }
            ],
        }
    }

    pub fn get_frustum_corners_with_near_far(&self, near: f32, far: f32) -> [Vector; 8] {
        let inverse_view_projection = (Matrix::new_projection(self.fov_y.to_radians(), self.aspect_ratio, near, far) * self.view_matrix).inverse4();
        let mut corners = [
            Vector::new_vec4(-1.0,1.0,0.0, 1.0),
            Vector::new_vec4(1.0,1.0,0.0, 1.0),
            Vector::new_vec4(1.0,-1.0,0.0, 1.0),
            Vector::new_vec4(-1.0,-1.0,0.0, 1.0),
            Vector::new_vec4(-1.0,1.0,1.0, 1.0),
            Vector::new_vec4(1.0,1.0,1.0, 1.0),
            Vector::new_vec4(1.0,-1.0,1.0, 1.0),
            Vector::new_vec4(-1.0,-1.0,1.0, 1.0),
        ];
        for i in 0..corners.len() {
            corners[i] = inverse_view_projection * corners[i];
            corners[i] = corners[i] / corners[i].w;
        }

        corners
    }
}

#[derive(Clone, Debug, Copy)]
pub struct Plane {
    pub normal: Vector,
    pub point: Vector,
}
impl Plane {
    pub fn null() -> Self {
        Self {
            normal: Vector::new_null(),
            point: Vector::new_null(),
        }
    }

    pub fn test_point_within(&self, point: &Vector) -> bool {
        let evaluated = self.normal * (point - self.point);
        evaluated.x + evaluated.y + evaluated.z > 0.0
    }

    pub fn test_sphere_within(&self, center: &Vector, radius: f32) -> bool {
        center.sub_vec(&self.point).dot(&self.normal) > -radius
    }
}
#[derive(Clone, Debug)]
pub struct Frustum {
    pub planes: [Plane; 6],
}
impl Frustum {
    pub fn null() -> Self {
        Self {
            planes: [Plane::null(); 6],
        }
    }

    pub fn test_point_within(&self, point: &Vector) -> bool {
        let mut i = 0;
        for plane in self.planes.iter() {
            if plane.test_point_within(point) { i += 1 }
        }
        i >= 6
    }

    pub fn test_sphere_within(&self, center: &Vector, radius: f32) -> bool {
        let mut i = 0;
        for plane in self.planes.iter() {
            if plane.test_sphere_within(center, radius) { i += 1 }
        }
        i >= 6
    }
}