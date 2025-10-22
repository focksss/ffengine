#![allow(dead_code)]

use std::ops::{Add, Div, Mul, Sub};

#[derive(Clone, Debug, Copy)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub null: bool,
}

impl Vector {
    //<editor-fold desc = "constructors">
    pub fn new_vec4(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w, null: false }
    }
    pub fn new_vec3(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, w: 1.0, null: false }
    }
    pub fn new_vec2(x: f32, y: f32) -> Self {
        Self { x, y, z: 1.0, w: 1.0, null: false}
    }
    pub fn new_vec(v: f32) -> Self {
        Self { x: v, y: v, z: v, w: v, null: false }
    }
    pub fn new_empty() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, w: 0.0, null: false }
    }

    pub fn new_null() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, w: 0.0, null: false }
    }

    pub fn new_from_vec(vals: &Vec<f32>) -> Self {
        match vals.len() {
            1 => Vector::new_vec(vals[0]),
            2 => Vector::new_vec2(vals[0], vals[1]),
            3 => Vector::new_vec3(vals[0], vals[1], vals[2]),
            4 => Vector::new_vec4(vals[0], vals[1], vals[2], vals[3]),
            _ => {
                eprintln!("\n--- PROBLEM ---\ninvalid number of values for new_from_vec: {}\nfrom {:?}\n", vals.len(), vals);
                Vector::new_empty()
            }
        }
    }
    pub fn new_from_array(vals: &[f32]) -> Self {
        match vals.len() {
            1 => Vector::new_vec(vals[0]),
            2 => Vector::new_vec2(vals[0], vals[1]),
            3 => Vector::new_vec3(vals[0], vals[1], vals[2]),
            4 => Vector::new_vec4(vals[0], vals[1], vals[2], vals[3]),
            _ => {
                eprintln!("\n--- PROBLEM ---\ninvalid number of values for new_from_array: {}\nfrom {:?}\n", vals.len(), vals);
                Vector::new_empty()
            }
        }
    }
    //</editor-fold>

    //<editor-fold desc = "to array">
    pub fn to_array4(&self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }
    pub fn to_array3(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
    pub fn to_array2(&self) -> [f32; 2] {
        [self.x, self.y]
    }
    //</editor-fold>

    //<editor-fold desc = "vector operations"
    pub fn magnitude_3d(&self) -> f32 {
        (
            self.x * self.x +
            self.y * self.y +
            self.z * self.z
            ).sqrt().max(1e-10)
    }
    pub fn magnitude_4d(&self) -> f32 {
        (
            self.x * self.x +
            self.y * self.y +
            self.z * self.z +
            self.w * self.w
            ).sqrt().max(1e-10)
    }

    pub fn normalize_3d(&self) -> Vector {
        self / self.magnitude_3d()
    }
    pub fn normalize_4d(&self) -> Vector {
        self / self.magnitude_4d()
    }

    pub fn euler_to_quat(&self) -> Vector {
        let cr = (self.x * 0.5).cos();
        let sr = (self.x * 0.5).sin();
        let cp = (self.y * 0.5).cos();
        let sp = (self.y * 0.5).sin();
        let cy = (self.z * 0.5).cos();
        let sy = (self.z * 0.5).sin();

        Vector::new_vec4(
            sr * cp * cy - cr * sp * sy,
            cr * sp * cy + sr * cp * sy,
            cr * cp * sy - sr * sp * cy,
            cr * cp * cy + sr * sp * sy
        )
    }
    //</editor-fold>

    //<editor-fold desc = "vector vector operations">
    pub fn dot(&self, other: &Vector) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }
    pub fn dot3(&self, other: &Vector) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
    pub fn cross(&self, other: &Vector) -> Vector {
        Vector::new_vec3(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x)
    }

    pub fn add_vec_to_self(&mut self, vec: &Vector) {
        let temp = Vector::new_vec4(self.x + vec.x, self.y + vec.y, self.z + vec.z, self.w + vec.w);
        self.x = temp.x;
        self.y = temp.y;
        self.z = temp.z;
        self.w = temp.w;
    }
    pub fn sub_vec(&self, vec: &Vector) -> Vector {
        Vector::new_vec4(self.x - vec.x, self.y - vec.y, self.z - vec.z, self.w - vec.w)
    }
    pub fn mul_by_vec(&self, vec: &Vector) -> Vector {
        Vector::new_vec4(self.x * vec.x, self.y * vec.y, self.z * vec.z, self.w * vec.w)
    }

    pub fn mul_by_vec_to_self(&mut self, vec: &Vector) {
        let temp = Vector::new_vec4(self.x * vec.x, self.y * vec.y, self.z * vec.z, self.w * vec.w);
        self.x = temp.x;
        self.y = temp.y;
        self.z = temp.z;
        self.w = temp.w;
    }
    pub fn div_by_vec(&self, vec: &Vector) -> Vector {
        Vector::new_vec4(self.x / vec.x, self.y / vec.y, self.z / vec.z, self.w / vec.w)
    }

    pub fn combine(&self, other: &Vector) -> Vector {
        Vector::new_vec4(
            self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
            self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
        )
    }
    
    pub fn combine_to_self(&mut self, other: &Vector) {
        let temp = self.clone().combine(other);
        self.x = temp.x;
        self.y = temp.y;
        self.z = temp.z;
        self.w = temp.w;
    }
    
    pub fn max(a: &Vector, b: &Vector) -> Vector {
        return Vector::new_vec4(
            a.x.max(b.x),
            a.y.max(b.y),
            a.z.max(b.z),
            a.w.max(b.w)
        )
    }
    pub fn min(a: &Vector, b: &Vector) -> Vector {
        return Vector::new_vec4(
            a.x.min(b.x),
            a.y.min(b.y),
            a.z.min(b.z),
            a.w.min(b.w)
        )
    }

    pub fn mix(a: &Vector, b: &Vector, t: f32) -> Vector {
        Vector::new_vec4(
            a.x + (b.x - a.x) * t,
            a.y + (b.y - a.y) * t,
            a.z + (b.z - a.z) * t,
            a.w + (b.w - a.w) * t
        )
    }

    pub fn spherical_lerp(a: &Vector, b: &Vector, t: f32) -> Vector {
        let a = a.normalize_4d();
        let mut b = b.normalize_4d();

        let mut dot = a.dot(&b).clamp(-1.0, 1.0);

        if dot < 0.0 {
            b = b * -1.0;
            dot = -dot;
        }

        let theta = dot.acos();
        let sin_theta = theta.sin();

        if sin_theta < 1e-10 {
            return Vector::mix(&a, &b, t).normalize_4d();
        }

        let w1 = ((1.0 - t) * theta).sin() / sin_theta;
        let w2 = (t * theta).sin() / sin_theta;

        (a * w1 + b * w2).normalize_4d()
    }

    pub fn rotate(&self, rot: &Vector) -> Vector {
        let rx = rot.x;
        let ry = rot.y;
        let rz = rot.z;

        let cos_x = rx.cos();
        let sin_x = rx.sin();
        let mut new_y = cos_x * self.y - sin_x * self.z;
        let mut new_z = sin_x * self.y + cos_x * self.z;
        let y = new_y;
        let z = new_z;
        
        let cos_y = ry.cos();
        let sin_y = ry.sin();
        let mut new_x = cos_y * self.x + sin_y * z;
        new_z = -sin_y * self.x + cos_y * z;
        let x = new_x;
            
        let cos_z = rz.cos();
        let sin_z = rz.sin();
        new_x = cos_z * x - sin_z * y;
        new_y = sin_z * x + cos_z * y;
    
        Vector::new_vec3(new_x, new_y, new_z)
    }
    //</editor-fold>

    //<editor-fold desc = "vector float operations"
    pub fn add_float(&self, v: f32) -> Vector {
        Vector::new_vec4(self.x + v, self.y + v, self.z + v, self.w + v)
    }
    pub fn sub_float(&self, v: f32) -> Vector {
        Vector::new_vec4(self.x - v, self.y - v, self.z - v, self.w - v)
    }
    //</editor-fold>
    
    pub fn println(&self) {
        println!("{:?}", self)
    }
}

impl<'a, 'b> Add<&'b Vector> for &'a Vector {
    type Output = Vector;

    fn add(self, other: &'b Vector) -> Vector {
        Vector::new_vec4(self.x + other.x, self.y + other.y, self.z + other.z, self.w + other.w)
    }
}
impl<'a> Add<Vector> for &'a Vector {
    type Output = Vector;
    fn add(self, other: Vector) -> Vector {
        self + &other
    }
}
impl<'b> Add<&'b Vector> for Vector {
    type Output = Vector;
    fn add(self, other: &'b Vector) -> Vector {
        &self + other
    }
}
impl Add<Vector> for Vector {
    type Output = Vector;
    fn add(self, other: Vector) -> Vector {
        &self + &other
    }
}

impl<'a, 'b> Sub<&'b Vector> for &'a Vector {
    type Output = Vector;

    fn sub(self, other: &'b Vector) -> Vector {
        Vector::new_vec4(self.x - other.x, self.y - other.y, self.z - other.z, self.w - other.w)
    }
}
impl<'a> Sub<Vector> for &'a Vector {
    type Output = Vector;
    fn sub(self, other: Vector) -> Vector {
        self - &other
    }
}
impl<'b> Sub<&'b Vector> for Vector {
    type Output = Vector;
    fn sub(self, other: &'b Vector) -> Vector {
        &self - other
    }
}
impl Sub<Vector> for Vector {
    type Output = Vector;
    fn sub(self, other: Vector) -> Vector {
        &self - &other
    }
}

impl<'a, 'b> Mul<&'b Vector> for &'a Vector {
    type Output = Vector;

    fn mul(self, other: &'b Vector) -> Vector {
        Vector::new_vec4(self.x * other.x, self.y * other.y, self.z * other.z, self.w * other.w)
    }
}
impl<'a> Mul<Vector> for &'a Vector {
    type Output = Vector;
    fn mul(self, other: Vector) -> Vector {
        self * &other
    }
}
impl<'b> Mul<&'b Vector> for Vector {
    type Output = Vector;
    fn mul(self, other: &'b Vector) -> Vector {
        &self * other
    }
}
impl Mul<Vector> for Vector {
    type Output = Vector;
    fn mul(self, other: Vector) -> Vector {
        &self * &other
    }
}
impl<'a> Mul<f32> for &'a Vector {
    type Output = Vector;
    fn mul(self, scalar: f32) -> Vector {
        Vector::new_vec4(self.x * scalar, self.y * scalar, self.z * scalar, self.w * scalar)
    }
}
impl Mul<f32> for Vector {
    type Output = Vector;
    fn mul(self, scalar: f32) -> Vector { &self * scalar }
}
impl<'a> Mul<&'a Vector> for f32 {
    type Output = Vector;
    fn mul(self, vector: &'a Vector) -> Vector {
        Vector::new_vec4(vector.x * self, vector.y * self, vector.z * self, vector.w * self)
    }
}
impl Mul<Vector> for f32 {
    type Output = Vector;
    fn mul(self, vector: Vector) -> Vector { self * &vector }
}

impl<'a, 'b> Div<&'b Vector> for &'a Vector {
    type Output = Vector;

    fn div(self, other: &'b Vector) -> Vector {
        Vector::new_vec4(self.x / other.x, self.y / other.y, self.z / other.z, self.w / other.w)
    }
}
impl<'a> Div<Vector> for &'a Vector {
    type Output = Vector;
    fn div(self, other: Vector) -> Vector {
        self / &other
    }
}
impl<'b> Div<&'b Vector> for Vector {
    type Output = Vector;
    fn div(self, other: &'b Vector) -> Vector {
        &self / other
    }
}
impl Div<Vector> for Vector {
    type Output = Vector;
    fn div(self, other: Vector) -> Vector {
        &self / &other
    }
}
impl<'a> Div<f32> for &'a Vector {
    type Output = Vector;
    fn div(self, scalar: f32) -> Vector {
        Vector::new_vec4(self.x / scalar, self.y / scalar, self.z / scalar, self.w / scalar)
    }
}
impl Div<f32> for Vector {
    type Output = Vector;
    fn div(self, scalar: f32) -> Vector { &self / scalar }
}
impl<'a> Div<&'a Vector> for f32 {
    type Output = Vector;
    fn div(self, vector: &'a Vector) -> Vector {
        Vector::new_vec4(vector.x / self, vector.y / self, vector.z / self, vector.w / self)
    }
}
impl Div<Vector> for f32 {
    type Output = Vector;
    fn div(self, vector: Vector) -> Vector { self / &vector }
}