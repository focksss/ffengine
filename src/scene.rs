use crate::matrix::Matrix;
use crate::vector::Vector;

pub struct Camera {
    view_matrix: Matrix,
    projection_matrix: Matrix,
    speed: f32,
    fov_y: f32,
}
impl Camera {
    pub fn new(position: &Vector, target: &Vector, rotation: &Vector, speed: f32, fov_y: f32) -> Self {
        Self {
            view_matrix: if target.null { 
                Matrix::new_view(position, rotation) 
            } else {
                Matrix::new_look_at(position, target, &Vector::new_vec3(0.0,1.0,0.0))
            },
            projection_matrix: Matrix::new(),
            speed,
            fov_y,
        }
    }
}