use crate::matrix::Matrix;
use crate::vector::Vector;

const PI: f32 = std::f32::consts::PI;
pub struct Camera {
    pub view_matrix: Matrix,
    pub projection_matrix: Matrix,
    pub position: Vector,
    pub target: Vector,
    pub rotation: Vector,
    pub speed: f32,
    pub sensitivity: f32,
    pub fov_y: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
    pub frustum: Frustum,
}
impl Camera {
    pub fn new_perspective_rotation(position: Vector, rotation: Vector, speed: f32, sensitivity: f32, fov_y: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        Self {
            view_matrix: Matrix::new(),
            projection_matrix: Matrix::new(),
            position,
            target: Vector::new_null(),
            rotation,
            speed,
            sensitivity,
            fov_y,
            aspect_ratio,
            near,
            far,
            frustum: Frustum::null()
        }
    }

    pub fn update_matrices(&mut self) {
        self.view_matrix = Matrix::new_view(&self.position, &self.rotation);
        self.projection_matrix = Matrix::new_projection(
            self.fov_y.to_radians(), 
            self.aspect_ratio,
            self.near,
            self.far,
        )
    }

    pub fn update_frustum(&mut self) {
        let cam_front = Vector::new_vec3(0.0,0.0,1.0).
            rotate(&(&self.rotation.
                mul_by_vec(&Vector::new_vec3(1.0, 1.0, 1.0))).
                add_vec(&Vector::new_vec3(0.0,-PI,0.0)));
        let cam_up = Vector::new_vec3(0.0,1.0,0.0).
            rotate(&(&self.rotation.
                mul_by_vec(&Vector::new_vec3(1.0, 1.0, 1.0))).
                add_vec(&Vector::new_vec3(0.0,-PI,0.0)));
        let cam_right = cam_up.cross(&cam_front).normalize_3d();

        let half_v = self.far * (self.fov_y*0.5).to_radians().tan();
        let half_h = half_v*self.aspect_ratio;

        let front_by_far = cam_front.mul_float(self.far);

        let position = self.position.mul_by_vec(&Vector::new_vec3(1.0,1.0,-1.0));

        self.frustum = Frustum { planes: [
            Plane {
                normal: cam_front,
                point: position.add_vec(&cam_front.mul_float(self.near)),
            },
            Plane {
                normal: cam_front.mul_float(-1.0),
                point: position.add_vec(&front_by_far),
            },
            Plane {
                normal: cam_up.cross( &front_by_far.sub_vec(&cam_right.mul_float(half_h)) ),
                point: position,
            },
            Plane {
                normal: ( &front_by_far.add_vec(&cam_right.mul_float(half_h)) ).cross(&cam_up),
                point: position,
            },
            Plane {
                normal: cam_right.cross( &front_by_far.add_vec(&cam_up.mul_float(half_v)) ),
                point: position,
            },
            Plane {
                normal: ( &front_by_far.sub_vec(&cam_up.mul_float(half_v)) ).cross(&cam_right),
                point: position,
            }
        ]}
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
        let evaluated = self.normal.mul_by_vec(&point.sub_vec(&self.point));
        evaluated.x + evaluated.y + evaluated.z > 0.0
    }

    pub fn test_sphere_within(&self, center: &Vector, radius: f32) -> bool {
        center.sub_vec(&self.point).dot(&self.normal) > -radius
    }
}
pub struct Frustum {
    pub planes: [Plane; 6],
}
impl Frustum {
    pub fn null() -> Self {
        Self {
            planes: [Plane::null(); 6]
        }
    }

    pub fn test_point_within(&self, point: &Vector) -> bool {
        let mut i = 0;
        for plane in self.planes.iter() {
            if plane.test_point_within(point) { i += 1 }
        }
        return i >= 6
    }

    pub fn test_sphere_within(&self, center: &Vector, radius: f32) -> bool {
        let mut i = 0;
        for plane in self.planes.iter() {
            if plane.test_sphere_within(center, radius) { i += 1 }
        }
        i >= 6
    }
}