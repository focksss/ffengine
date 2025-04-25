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
    pub fn new_rotate(x: f32, y: f32, z: f32) -> Self {
        let rx = Matrix::new_rotate_x(x);
        let ry = Matrix::new_rotate_y(y);
        let rz = Matrix::new_rotate_z(z);
        rz.mul_mat4(&ry).mul_mat4(&rx)
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
}
/*
class Matrix4 {

  static rotateQuat(x, y, z, w) {
    const mat = new Matrix4();
    mat.identity();
    mat.data[0] = 1 - 2*(y*y + z*z);
    mat.data[1] = 2*(x*y - z*w);
    mat.data[2] = 2*(x*z + y*w);
    mat.data[4] = 2*(x*y + z*w);
    mat.data[5] = 1 - 2*(x*x + z*z);
    mat.data[6] = 2*(y*z - x*w);
    mat.data[8] = 2*(x*z - y*w);
    mat.data[9] = 2*(y*z + x*w);
    mat.data[10] = 1 - 2*(x*x + y*y);
    return mat;
  }

  static scale(x, y, z) {
    const mat = new Matrix4();
    mat.identity();
    mat.data[0] = x;
    mat.data[5] = y;
    mat.data[10] = z;
    return mat;
  }

  static projection(fov, aspect, near, far) {
    const mat = new Matrix4();
    mat.identity();
    const f = 1.0 / Math.tan(fov / 2);

    mat.data[0] = f / aspect;
    mat.data[5] = f;
    mat.data[10] = (far + near) / (near - far);
    mat.data[11] = -1;
    mat.data[14] = (2 * far * near) / (near - far);
    mat.data[15] = 0;

    return mat;
  }

  static ortho(left, right, bottom, top, near, far) {
    const mat = new Matrix4();
    mat.identity();

    const rl = right - left;
    const tb = top - bottom;
    const fn = far - near;

    mat.data[0] = 2 / rl;
    mat.data[5] = 2 / tb;
    mat.data[10] = -2 / fn;
    mat.data[12] = -(right + left) / rl;
    mat.data[13] = -(top + bottom) / tb;
    mat.data[14] = -(far + near) / fn;
    mat.data[15] = 1;

    return mat;
  }

  static view(rot, pos) {
    const rotate = Matrix4.rotation(rot.x, rot.y, rot.z);
    const mat = Matrix4.translation(-pos.x, -pos.y, -pos.z);
    return mat.multiply(rotate);
  }

  static lookAt(position, target, up) {
    
    const f = target.sub(position).normalize();
    const s = f.cross(up).normalize();
    const u = s.cross(f).normalize();

    const mat = new Matrix4();
    mat.data.fill(1);

    mat.data[0] = s.x;
    mat.data[4] = s.y;
    mat.data[8] = s.z;

    mat.data[1] = u.x;
    mat.data[5] = u.y;
    mat.data[9] = u.z;

    mat.data[2] = -f.x;
    mat.data[6] = -f.y;
    mat.data[10] = -f.z;

    mat.data[3] = 0.0;
    mat.data[7] = 0.0;
    mat.data[11] = 0.0;

    mat.data[12] = -s.dot(position);
    mat.data[13] = -u.dot(position);
    mat.data[14] = f.dot(position);
    mat.data[15] = 1.0;
    return mat;
    
    // const rotate = Matrix4.rotation(target.x, target.y, target.z);
    // const mat = Matrix4.translation(-position.x, -position.y, -position.z);
    // return mat.multiply(rotate);
  }

  toArray() {
    return this.data;
  }
}
 */