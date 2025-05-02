use crate::matrix::Matrix;
use crate::vector::Vector;
use crate::vk_initializer::VkBase;

pub struct Camera {
    pub view_matrix: Matrix,
    pub projection_matrix: Matrix,
    pub position: Vector,
    pub target: Vector,
    pub rotation: Vector,
    pub speed: f32,
    pub sensitivity: f32,
    pub fov_y: f32,
}
impl Camera {
    pub fn new(position: Vector, target: Vector, rotation: Vector, speed: f32, sensitivity: f32, fov_y: f32) -> Self {
        Self {
            view_matrix: Matrix::new(),
            projection_matrix: Matrix::new(),
            position,
            target,
            rotation,
            speed,
            sensitivity,
            fov_y,
        }
    }
    
    pub fn update_matrices(&mut self, base: &VkBase) {
        self.view_matrix = Matrix::new_view(&self.position, &self.rotation);
        self.projection_matrix = Matrix::new_projection(
            self.fov_y.to_radians(), 
            base.window.inner_size().width as f32 / base.window.inner_size().height as f32,
            0.01,
            1000.0
        )
    }
}