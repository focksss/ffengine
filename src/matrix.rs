#![allow(dead_code)]
use crate::vector::Vector;
const PI: f32 = std::f32::consts::PI;
#[derive(Debug, Copy, Clone)]
pub struct Matrix {
    pub data: [f32; 16],
}
impl Matrix {
    //<editor-fold desc = "constructors">
    // defaults to identity
    pub fn new() -> Self {
        Self {
            data: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ]
        }
    }

    pub fn new_empty() -> Self {
        Self { data: [0.0; 16] }
    }

    // consumes given array
    pub fn new_manual(data: [f32; 16]) -> Self {
        Self { data }
    }
    pub fn new_manual_from_column_major(data: [f32; 16]) -> Self {
        Self {
            data: [
                data[0],  data[4],  data[8],  data[12],
                data[1],  data[5],  data[9],  data[13],
                data[2],  data[6],  data[10], data[14],
                data[3],  data[7],  data[11], data[15],
            ],
        }
    }
    //</editor-fold>

    //<editor-fold desc ="matrix operations">
    pub fn inverse(&self) -> Matrix {
        let det = self.determinant();
        if det.abs() < f32::EPSILON {
            panic!("Matrix is not invertible (determinant is zero)");
        }

        let adjugate = self.adjugate();
        let mut result = Matrix::new_empty();
        for i in 0..16 {
            result.data[i] = adjugate.data[i] / det;
        }
        result
    }

    pub fn determinant(&self) -> f32 {
        let m = &self.data;

        let subfactor00 = m[10] * m[15] - m[11] * m[14];
        let subfactor01 = m[9] * m[15] - m[11] * m[13];
        let subfactor02 = m[9] * m[14] - m[10] * m[13];
        let subfactor03 = m[8] * m[15] - m[11] * m[12];
        let subfactor04 = m[8] * m[14] - m[10] * m[12];
        let subfactor05 = m[8] * m[13] - m[9] * m[12];

        m[0] * (m[5] * subfactor00 - m[6] * subfactor01 + m[7] * subfactor02)
            - m[1] * (m[4] * subfactor00 - m[6] * subfactor03 + m[7] * subfactor04)
            + m[2] * (m[4] * subfactor01 - m[5] * subfactor03 + m[7] * subfactor05)
            - m[3] * (m[4] * subfactor02 - m[5] * subfactor04 + m[6] * subfactor05)
    }

    fn adjugate(&self) -> Matrix {
        let m = &self.data;
        let mut cofactors = [0.0f32; 16];

        for row in 0..4 {
            for col in 0..4 {
                let minor = self.minor(row, col);
                let cofactor = if (row + col) % 2 == 0 { minor } else { -minor };
                cofactors[col * 4 + row] = cofactor; // transpose
            }
        }

        Matrix::new_manual(cofactors)
    }

    fn minor(&self, row: usize, col: usize) -> f32 {
        let mut sub = [0.0f32; 9];
        let mut idx = 0;
        for r in 0..4 {
            for c in 0..4 {
                if r != row && c != col {
                    sub[idx] = self.data[r * 4 + c];
                    idx += 1;
                }
            }
        }

        sub[0] * (sub[4] * sub[8] - sub[5] * sub[7])
            - sub[1] * (sub[3] * sub[8] - sub[5] * sub[6])
            + sub[2] * (sub[3] * sub[7] - sub[4] * sub[6])
    }
    //</editor-fold>

    //<editor-fold desc = "matrix matrix operations">
    pub fn mul_mat4(&self, other: &Matrix) -> Matrix {
        let mut result = Matrix::new_empty();
        for row in 0..4 {
            for col in 0..4 {
                let mut sum = 0.0;
                for i in 0..4 {
                    result.data[col * 4 + row] += self.data[i * 4 + row] * other.data[col * 4 + i];
                }
            }
        }
        result
    }

    pub fn set_and_mul_mat4(&mut self, other: &Matrix) {
        self.data = self.mul_mat4(other).data;
    }
    //</editor-fold>

    //<editor-fold desc = "matrix vector operations">
    pub fn mul_vector4(&self, other: &Vector) -> Vector {
        let x = self.data[0] * other.x + self.data[4] * other.y + self.data[8] * other.z + self.data[12] * other.w;
        let y = self.data[1] * other.x + self.data[5] * other.y + self.data[9] * other.z + self.data[13] * other.w;
        let z = self.data[2] * other.x + self.data[6] * other.y + self.data[10] * other.z + self.data[14] * other.w;
        let w = self.data[3] * other.x + self.data[7] * other.y + self.data[11] * other.z + self.data[15] * other.w;
        Vector::new_vec4(x, y, z, w)
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
        rx.mul_mat4(&ry).mul_mat4(&rz)
    }
    pub fn new_rotate_euler_3f(x: f32, y: f32, z: f32) -> Self {
        let rx = Matrix::new_rotate_x(x);
        let ry = Matrix::new_rotate_y(y);
        let rz = Matrix::new_rotate_z(z);
        rx.mul_mat4(&ry).mul_mat4(&rz)
    }

    pub fn new_rotate_quaternion_vec4(quaternion: &Vector) -> Self {
        let mut result = Matrix::new();
        let (x, y, z, w) = (quaternion.x, quaternion.y, quaternion.z, quaternion.w);

        result.data[0] = 1.0 - 2.0 * (y * y + z * z);
        result.data[1] = 2.0 * (x * y + z * w);
        result.data[2] = 2.0 * (x * z - y * w);
        result.data[4] = 2.0 * (x * y - z * w);
        result.data[5] = 1.0 - 2.0 * (x * x + z * z);
        result.data[6] = 2.0 * (y * z + x * w);
        result.data[8] = 2.0 * (x * z + y * w);
        result.data[9] = 2.0 * (y * z - x * w);
        result.data[10] = 1.0 - 2.0 * (x * x + y * y);
        result
    }
    pub fn new_rotate_quaternion_4f(x: f32, y: f32, z: f32, w: f32) -> Self {
        let mut result = Matrix::new();
        result.data[0] = 1.0 - 2.0 * (y * y + z * z);
        result.data[1] = 2.0 * (x * y + z * w);
        result.data[2] = 2.0 * (x * z - y * w);
        result.data[4] = 2.0 * (x * y - z * w);
        result.data[5] = 1.0 - 2.0 * (x * x + z * z);
        result.data[6] = 2.0 * (y * z + x * w);
        result.data[8] = 2.0 * (x * z + y * w);
        result.data[9] = 2.0 * (y * z - x * w);
        result.data[10] = 1.0 - 2.0 * (x * x + y * y);
        result
    }

    pub fn new_projection(fov_y: f32, aspect: f32, near: f32, far: f32) -> Matrix {
        let mut result = Matrix::new();

        let f = 1.0 / (fov_y / 2.0).tan();

        result.data[0] = f / aspect;
        result.data[5] = -f;

        result.data[10] = far / (near - far);
        result.data[11] = -1.0;
        result.data[14] = (far * near) / (near - far);
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
        result.data[10] = 1.0 / f_n;
        result.data[12] = -(right + left) / rl;
        result.data[13] = -(top + bottom) / tb;
        result.data[14] = -near / f_n;
        result.data[15] = 1.0;

        result
    }

    pub fn new_view(pos: &Vector, rot: &Vector) -> Self {
        let t = Matrix::new_translation_vec3(&pos.mul_by_vec(&Vector::new_vec3(-1.0, -1.0, 1.0)));
        let r = Matrix::new_rotate_euler_vec3(&rot.mul_by_vec(&Vector::new_vec3(-1.0,1.0,-1.0)));

        let result = r.mul_mat4(&t);

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