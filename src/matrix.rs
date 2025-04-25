#![allow(dead_code)]
use crate::vector::Vector;

#[derive(Debug)]
pub struct Matrix {
    pub data: [f32; 16],
}
impl Matrix {
    //<editor-fold desc = "constructors">
    // defaults to identity
    pub fn new() -> Self {
        Self { data: [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ]}
    }

    pub fn new_empty() -> Self {
        Self { data: [0.0; 16] }
    }

    // consumes given array
    pub fn new_manual(data: [f32; 16]) -> Self {
        Self { data }
    }
    //</editor-fold>

    //<editor-fold desc = "matrix matrix operations">
    pub fn mul_mat4(&self, other: &Matrix) -> Matrix {
        let mut result = Matrix::new_empty();
        for row in 0..4 {
            for col in 0..4 {
                let mut sum = 0.0;
                for i in 0..4 {
                    sum += self.data[row * 4 + i] * other.data[i * 4 + col];
                }
                result.data[row * 4 + col] = sum;
            }
        }
        result
    }
    //</editor-fold>

    //<editor-fold desc = "matrix vector operations">
    pub fn mul_vector4(&self, other: &Vector) -> Vector {
        let mut result = [0.0; 4];
        for row in 0..4 {
            result[row] =
                self.data[row * 4] * other.x +
                self.data[row * 4 + 1] * other.y +
                self.data[row * 4 + 2] * other.z +
                self.data[row * 4 + 3] * other.w;
        }
        Vector::new_from_array(&result)
    }
    //</editor-fold>

    //<editor-fold desc = "special constructors">
    pub fn new_translation_vec3(translation: &Vector) -> Self {
        let mut result = Matrix::new();
        result.data[12] = translation.x;
        result.data[13] = translation.y;
        result.data[14] = translation.z;
        result
    }
    pub fn new_translation_3f(x: f32, y: f32, z: f32) -> Self {
        let mut result = Matrix::new();
        result.data[12] = x;
        result.data[13] = y;
        result.data[14] = z;
        result
    }

    pub fn new_scale_vec3(scale: &Vector) -> Self {
        let mut result = Matrix::new();
        result.data[0] = scale.x;
        result.data[5] = scale.y;
        result.data[10] = scale.z;
        result
    }
    pub fn new_scale_3f(x: f32, y: f32, z: f32) -> Self {
        let mut result = Matrix::new();
        result.data[0] = x;
        result.data[5] = y;
        result.data[10] = z;
        result
    }

    pub fn new_rotate_x(theta: f32) -> Self {
        let mut result = Matrix::new();
        let c = theta.cos();
        let s = theta.sin();
        result.data[5] = c;
        result.data[6] = -s;
        result.data[9] = s;
        result.data[10] = c;
        result
    }
    pub fn new_rotate_y(theta: f32) -> Self {
        let mut result = Matrix::new();
        let c = theta.cos();
        let s = theta.sin();
        result.data[0] = c;
        result.data[2] = s;
        result.data[8] = -s;
        result.data[10] = c;
        result
    }
    pub fn new_rotate_z(theta: f32) -> Self {
        let mut result = Matrix::new();
        let c = theta.cos();
        let s = theta.sin();
        result.data[0] = c;
        result.data[1] = -s;
        result.data[4] = s;
        result.data[5] = c;
        result
    }
    pub fn new_rotate_euler_vec3(r: &Vector) -> Self {
        let (x, y, z) = (r.x, r.y, r.z);
        let rx = Matrix::new_rotate_x(x);
        let ry = Matrix::new_rotate_y(y);
        let rz = Matrix::new_rotate_z(z);
        rz.mul_mat4(&ry).mul_mat4(&rx)
    }
    pub fn new_rotate_euler_3f(x: f32, y: f32, z: f32) -> Self {
        let rx = Matrix::new_rotate_x(x);
        let ry = Matrix::new_rotate_y(y);
        let rz = Matrix::new_rotate_z(z);
        rz.mul_mat4(&ry).mul_mat4(&rx)
    }

    pub fn new_rotate_quaternion_vec4(quaternion: &Vector) -> Self {
        let mut result = Matrix::new();
        let (x, y, z, w) = (quaternion.x, quaternion.y, quaternion.z, quaternion.w);
        result.data[0] = 1.0 - 2.0*(y*y + z*z);
        result.data[1] = 2.0*(x*y - z*w);
        result.data[2] = 2.0*(x*z + y*w);
        result.data[4] = 2.0*(x*y + z*w);
        result.data[5] = 1.0 - 2.0*(x*x + z*z);
        result.data[6] = 2.0*(y*z - x*w);
        result.data[8] = 2.0*(x*z - y*w);
        result.data[9] = 2.0*(y*z + x*w);
        result.data[10] = 1.0 - 2.0*(x*x + y*y);
        result
    }
    pub fn new_rotate_quaternion_4f(x: f32, y: f32, z: f32, w: f32) -> Self {
        let mut result = Matrix::new();
        result.data[0] = 1.0 - 2.0*(y*y + z*z);
        result.data[1] = 2.0*(x*y - z*w);
        result.data[2] = 2.0*(x*z + y*w);
        result.data[4] = 2.0*(x*y + z*w);
        result.data[5] = 1.0 - 2.0*(x*x + z*z);
        result.data[6] = 2.0*(y*z - x*w);
        result.data[8] = 2.0*(x*z - y*w);
        result.data[9] = 2.0*(y*z + x*w);
        result.data[10] = 1.0 - 2.0*(x*x + y*y);
        result
    }

    pub fn new_projection(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let mut result = Matrix::new();

        let f = 1.0 / (fov_y / 2.0).tan();

        result.data[0] = f / aspect;
        result.data[5] = f;
        result.data[10] = (far + near) / (near - far);
        result.data[11] = -1.0;
        result.data[14] = (2.0 * far * near) / (near - far);
        result.data[15] = 0.0;

        result
    }
    pub fn new_ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let mut result = Matrix::new();

        let rl = right - left;
        let tb = top - bottom;
        let f_n = far - near;

        result.data[0] = 2.0 / rl;
        result.data[5] = 2.0 / tb;
        result.data[10] = -2.0 / f_n;
        result.data[12] = -(right + left) / rl;
        result.data[13] = -(top + bottom) / tb;
        result.data[14] = -(far + near) / f_n;
        result.data[15] = 1.0;

        result
    }

    pub fn new_view(pos: &Vector, rot: &Vector) -> Self {
        let t = Matrix::new_translation_vec3(pos);
        let r = Matrix::new_rotate_euler_vec3(rot);

        let result = t.mul_mat4(&r);

        result
    }
    pub fn new_look_at(position: &Vector, target: &Vector, up: &Vector) -> Self {
        let mut result = Matrix::new();

        let f = target.sub_vec(position).normalize_3d();
        let s = f.cross(up).normalize_3d();
        let u = s.cross(&f).normalize_3d();

        result.data[0] = s.x;
        result.data[4] = s.y;
        result.data[8] = s.z;

        result.data[1] = u.x;
        result.data[5] = u.y;
        result.data[9] = u.z;

        result.data[2] = -f.x;
        result.data[6] = -f.y;
        result.data[10] = -f.z;

        result.data[3] = 0.0;
        result.data[7] = 0.0;
        result.data[11] = 0.0;

        result.data[12] = -s.dot(position);
        result.data[13] = -u.dot(position);
        result.data[14] = f.dot(position);
        result.data[15] = 1.0;

        result
    }
    //</editor-fold>
    
    pub fn println(&self) {
        for i in 0..4 {
            println!("{:?}", &self.data[i * 4..(i + 1) * 4]);
        }
    }
}