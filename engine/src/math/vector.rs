#![allow(dead_code)]

use std::any::{Any, TypeId};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use mlua::{FromLua, IntoLua, Lua, UserData, UserDataFields, UserDataMethods, Value};
use mlua::prelude::{LuaError, LuaResult, LuaValue};

#[derive(Clone, Debug, Copy)]
#[derive(Default)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vector {
    //<editor-fold desc = "constructors">
    pub fn new4(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
    pub fn new() -> Self {
        Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }
    pub fn new3(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, w: 1.0 }
    }
    pub fn new2(x: f32, y: f32) -> Self {
        Self { x, y, z: 1.0, w: 1.0 }
    }
    pub fn fill(v: f32) -> Self {
        Self { x: v, y: v, z: v, w: v }
    }
    pub fn empty() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, w: 0.0 }
    }

    pub fn from_vec(vals: &Vec<f32>) -> Self {
        match vals.len() {
            1 => Vector::fill(vals[0]),
            2 => Vector::new2(vals[0], vals[1]),
            3 => Vector::new3(vals[0], vals[1], vals[2]),
            4 => Vector::new4(vals[0], vals[1], vals[2], vals[3]),
            _ => {
                eprintln!("\n--- PROBLEM ---\ninvalid number of values for new_from_vec: {}\nfrom {:?}\n", vals.len(), vals);
                Vector::empty()
            }
        }
    }
    pub fn from_array(vals: &[f32]) -> Self {
        match vals.len() {
            1 => Vector::fill(vals[0]),
            2 => Vector::new2(vals[0], vals[1]),
            3 => Vector::new3(vals[0], vals[1], vals[2]),
            4 => Vector::new4(vals[0], vals[1], vals[2], vals[3]),
            _ => {
                eprintln!("\n--- PROBLEM ---\ninvalid number of values for new_from_array: {}\nfrom {:?}\n", vals.len(), vals);
                Vector::empty()
            }
        }
    }

    pub fn axis_angle_quat(n: &Vector, theta: f32) -> Vector {
        let half_angle = theta * 0.5;
        let half_sine = half_angle.sin();
        let a = n.normalize3();
        Vector::new4(
            a.x * half_sine,
            a.y * half_sine,
            a.z * half_sine,
            half_angle.cos(),
        )
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
    pub fn magnitude3(&self) -> f32 {
        (
            self.x * self.x +
            self.y * self.y +
            self.z * self.z
            ).sqrt().max(1e-10)
    }
    pub fn magnitude3_sq(&self) -> f32 {
        (
            self.x * self.x +
            self.y * self.y +
            self.z * self.z
        ).max(1e-10)
    }
    pub fn magnitude4(&self) -> f32 {
        (
            self.x * self.x +
            self.y * self.y +
            self.z * self.z +
            self.w * self.w
            ).sqrt().max(1e-10)
    }
    pub fn magnitude4_sq(&self) -> f32 {
        (
            self.x * self.x +
            self.y * self.y +
            self.z * self.z +
            self.w * self.w
        ).max(1e-10)
    }

    pub fn normalize3(&self) -> Vector {
        self / self.magnitude3()
    }
    pub fn normalize4(&self) -> Vector {
        self / self.magnitude4()
    }

    pub fn euler_to_quat(&self) -> Vector {
        let cr = (self.x * 0.5).cos();
        let sr = (self.x * 0.5).sin();
        let cp = (self.y * 0.5).cos();
        let sp = (self.y * 0.5).sin();
        let cy = (self.z * 0.5).cos();
        let sy = (self.z * 0.5).sin();

        Vector::new4(
            sr * cp * cy - cr * sp * sy,
            cr * sp * cy + sr * cp * sy,
            cr * cp * sy - sr * sp * cy,
            cr * cp * cy + sr * sp * sy
        )
    }
    pub fn quat_to_euler(&self) -> Vector {
        let qx = self.x;
        let qy = self.y;
        let qz = self.z;
        let qw = self.w;

        let sinr_cosp = 2.0 * (qw * qx + qy * qz);
        let cosr_cosp = 1.0 - 2.0 * (qx * qx + qy * qy);
        let roll = sinr_cosp.atan2(cosr_cosp);

        let sinp = 2.0 * (qw * qy - qz * qx);
        let pitch = if sinp.abs() >= 1.0 {
            sinp.signum() * std::f32::consts::FRAC_PI_2
        } else {
            sinp.asin()
        };

        let siny_cosp = 2.0 * (qw * qz + qx * qy);
        let cosy_cosp = 1.0 - 2.0 * (qy * qy + qz * qz);
        let yaw = siny_cosp.atan2(cosy_cosp);

        Vector::new3(roll, pitch, yaw)
    }

    pub fn conjugate(&self) -> Vector {
        Vector::new4(-self.x, -self.y, -self.z, self.w)
    }

    pub fn inverse_quat(&self) -> Vector { self.conjugate() / self.dot4(self) }

    pub fn unitize_w(mut self) -> Self {
        self.w = 1.0;
        self
    }
    
    pub fn get<T: AxisKey>(&self, axis: T) -> f32 {
        axis.get_component(self)
    }
    pub fn set<T: AxisKey>(&mut self, axis: T, v: f32) -> &mut Self {
        axis.set_component(self, v);
        self
    }
    pub fn with<T: AxisKey>(&self, axis: T, v: f32) -> Vector {
        let mut ret = self.clone();
        axis.set_component(&mut ret, v);
        ret
    }

    pub fn max_of(&self) -> f32 {
        self.x.max(self.y).max(self.z).max(self.w)
    }
    pub fn min_of(&self) -> f32 {
        self.x.min(self.y).min(self.z).min(self.w)
    }
    //</editor-fold>

    //<editor-fold desc = "vector vector operations">
    pub fn dot4(&self, other: &Vector) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }
    pub fn dot3(&self, other: &Vector) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
    pub fn cross(&self, other: &Vector) -> Vector {
        Vector::new3(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x)
    }

    pub fn add_vec_to_self(&mut self, vec: &Vector) {
        let temp = Vector::new4(self.x + vec.x, self.y + vec.y, self.z + vec.z, self.w + vec.w);
        self.x = temp.x;
        self.y = temp.y;
        self.z = temp.z;
        self.w = temp.w;
    }
    pub fn sub_vec(&self, vec: &Vector) -> Vector {
        Vector::new4(self.x - vec.x, self.y - vec.y, self.z - vec.z, self.w - vec.w)
    }
    pub fn mul_by_vec(&self, vec: &Vector) -> Vector {
        Vector::new4(self.x * vec.x, self.y * vec.y, self.z * vec.z, self.w * vec.w)
    }

    pub fn mul_by_vec_to_self(&mut self, vec: &Vector) {
        let temp = Vector::new4(self.x * vec.x, self.y * vec.y, self.z * vec.z, self.w * vec.w);
        self.x = temp.x;
        self.y = temp.y;
        self.z = temp.z;
        self.w = temp.w;
    }
    pub fn div_by_vec(&self, vec: &Vector) -> Vector {
        Vector::new4(self.x / vec.x, self.y / vec.y, self.z / vec.z, self.w / vec.w)
    }

    /**
    * self * other
     */
    pub fn combine(&self, other: &Vector) -> Vector {
        Vector::new4(
            self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
            self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
        )
    }
    
    pub fn combine_to_self(&mut self, other: &Vector) {
        self.x = self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y;
        self.y = self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x;
        self.z = self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w;
        self.w = self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z;
    }
    
    pub fn max(a: &Vector, b: &Vector) -> Vector {
        return Vector::new4(
            a.x.max(b.x),
            a.y.max(b.y),
            a.z.max(b.z),
            a.w.max(b.w)
        )
    }
    pub fn min(a: &Vector, b: &Vector) -> Vector {
        return Vector::new4(
            a.x.min(b.x),
            a.y.min(b.y),
            a.z.min(b.z),
            a.w.min(b.w)
        )
    }

    pub fn mix(a: &Vector, b: &Vector, t: f32) -> Vector {
        Vector::new4(
            a.x + (b.x - a.x) * t,
            a.y + (b.y - a.y) * t,
            a.z + (b.z - a.z) * t,
            a.w + (b.w - a.w) * t
        )
    }

    pub fn spherical_lerp(a: &Vector, b: &Vector, t: f32) -> Vector {
        let a = a.normalize4();
        let mut b = b.normalize4();

        let mut dot = a.dot4(&b).clamp(-1.0, 1.0);

        if dot < 0.0 {
            b = b * -1.0;
            dot = -dot;
        }

        let theta = dot.acos();
        let sin_theta = theta.sin();

        if sin_theta < 1e-10 {
            return Vector::mix(&a, &b, t).normalize4();
        }

        let w1 = ((1.0 - t) * theta).sin() / sin_theta;
        let w2 = (t * theta).sin() / sin_theta;

        (a * w1 + b * w2).normalize4()
    }

    pub fn rotate_by_euler(&self, rot: &Vector) -> Vector {
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
    
        Vector::new3(new_x, new_y, new_z)
    }
    pub fn rotate_by_quat(&self, rot: &Vector) -> Vector {
        let qw = rot.w;
        let qx = rot.x;
        let qy = rot.y;
        let qz = rot.z;

        let vx = self.x;
        let vy = self.y;
        let vz = self.z;

        let ix =  qw * vx + qy * vz - qz * vy;
        let iy =  qw * vy + qz * vx - qx * vz;
        let iz =  qw * vz + qx * vy - qy * vx;
        let iw = -qx * vx - qy * vy - qz * vz;

        let new_x = ix * qw + iw * -qx + iy * -qz - iz * -qy;
        let new_y = iy * qw + iw * -qy + iz * -qx - ix * -qz;
        let new_z = iz * qw + iw * -qz + ix * -qy - iy * -qx;

        Vector::new3(new_x, new_y, new_z)
    }

    pub fn project_onto_plane(&self, other: &Vector) -> Vector {
        let n = other.normalize3();
        self - n * self.dot3(&n)
    }

    pub fn clamp3(&self, min: &Vector, max: &Vector) -> Vector {
        Vector::new3(
            self.x.clamp(min.x, max.x),
            self.y.clamp(min.y, max.y),
            self.z.clamp(min.z, max.z),
        )
    }
    pub fn clamp4(&self, min: &Vector, max: &Vector) -> Vector {
        Vector::new4(
            self.x.clamp(min.x, max.x),
            self.y.clamp(min.y, max.y),
            self.z.clamp(min.z, max.z),
            self.w.clamp(min.w, max.w)
        )
    }

    pub fn equals(&self, other: &Vector, threshold: f32) -> bool {
        (self.x - other.x).abs() <= threshold &&
        (self.y - other.y).abs() <= threshold && 
        (self.z - other.z).abs() <= threshold &&
        (self.w - other.w).abs() <= threshold
    }
    //</editor-fold>

    //<editor-fold desc = "vector float operations"
    pub fn add_float(&self, v: f32) -> Vector {
        Vector::new4(self.x + v, self.y + v, self.z + v, self.w + v)
    }
    pub fn sub_float(&self, v: f32) -> Vector {
        Vector::new4(self.x - v, self.y - v, self.z - v, self.w - v)
    }

    ///* Threshold is not direction dependent.
    pub fn nullify_threshold(&self, threshold: f32) -> Vector {
        Vector::new4(
            if self.x.abs() > threshold { self.x } else { 0.0 },
            if self.y.abs() > threshold { self.y } else { 0.0 },
            if self.z.abs() > threshold { self.z } else { 0.0 },
            if self.w.abs() > threshold { self.w } else { 0.0 },
        )
    }
    ///* Threshold is not direction dependent.
    pub fn nullify_horizontal_threshold(&self, threshold: f32) -> Vector {
        Vector::new4(
            if self.x.abs() > threshold { self.x } else { 0.0 },
            self.y,
            if self.z.abs() > threshold { self.z } else { 0.0 },
            self.w
        )
    }
    //</editor-fold>
    
    pub fn println(&self) {
        println!("{:?}", self)
    }
}

impl<'a, 'b> Add<&'b Vector> for &'a Vector {
    type Output = Vector;

    fn add(self, other: &'b Vector) -> Vector {
        Vector::new4(self.x + other.x, self.y + other.y, self.z + other.z, self.w + other.w)
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

impl AddAssign<&Vector> for Vector {
    fn add_assign(&mut self, other: &Vector) {
        self.add_vec_to_self(other);
    }
}
impl AddAssign<Vector> for Vector {
    fn add_assign(&mut self, other: Vector) {
        *self += &other;
    }
}

impl<'a, 'b> Sub<&'b Vector> for &'a Vector {
    type Output = Vector;

    fn sub(self, other: &'b Vector) -> Vector {
        Vector::new4(self.x - other.x, self.y - other.y, self.z - other.z, self.w - other.w)
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

impl SubAssign<&Vector> for Vector {
    fn sub_assign(&mut self, other: &Vector) {
        self.x -= other.x;
        self.y -= other.y;
        self.z -= other.z;
        self.w -= other.w;
    }
}
impl SubAssign<Vector> for Vector {
    fn sub_assign(&mut self, other: Vector) {
        *self -= &other;
    }
}

impl<'a, 'b> Mul<&'b Vector> for &'a Vector {
    type Output = Vector;

    fn mul(self, other: &'b Vector) -> Vector {
        Vector::new4(self.x * other.x, self.y * other.y, self.z * other.z, self.w * other.w)
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
        Vector::new4(self.x * scalar, self.y * scalar, self.z * scalar, self.w * scalar)
    }
}
impl Mul<f32> for Vector {
    type Output = Vector;
    fn mul(self, scalar: f32) -> Vector { &self * scalar }
}
impl<'a> Mul<&'a Vector> for f32 {
    type Output = Vector;
    fn mul(self, vector: &'a Vector) -> Vector {
        Vector::new4(vector.x * self, vector.y * self, vector.z * self, vector.w * self)
    }
}
impl Mul<Vector> for f32 {
    type Output = Vector;
    fn mul(self, vector: Vector) -> Vector { self * &vector }
}

impl MulAssign<&Vector> for Vector {
    fn mul_assign(&mut self, other: &Vector) {
        self.mul_by_vec_to_self(other);
    }
}
impl MulAssign<Vector> for Vector {
    fn mul_assign(&mut self, other: Vector) {
        *self *= &other;
    }
}

impl<'a, 'b> Div<&'b Vector> for &'a Vector {
    type Output = Vector;

    fn div(self, other: &'b Vector) -> Vector {
        Vector::new4(self.x / other.x, self.y / other.y, self.z / other.z, self.w / other.w)
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
        Vector::new4(self.x / scalar, self.y / scalar, self.z / scalar, self.w / scalar)
    }
}
impl Div<f32> for Vector {
    type Output = Vector;
    fn div(self, scalar: f32) -> Vector { &self / scalar }
}
impl<'a> Div<&'a Vector> for f32 {
    type Output = Vector;
    fn div(self, vector: &'a Vector) -> Vector {
        Vector::new4(vector.x / self, vector.y / self, vector.z / self, vector.w / self)
    }
}
impl Div<Vector> for f32 {
    type Output = Vector;
    fn div(self, vector: Vector) -> Vector { self / &vector }
}

impl DivAssign<&Vector> for Vector {
    fn div_assign(&mut self, other: &Vector) {
        self.x /= &other.x;
        self.y /= &other.y;
        self.z /= &other.z;
        self.w /= &other.w;
    }
}
impl DivAssign<Vector> for Vector {
    fn div_assign(&mut self, other: Vector) {
        *self /= &other;
    }
}

impl Neg for Vector {
    type Output = Vector;
    fn neg(self) -> Vector {
        Vector::new4(-self.x, -self.y, -self.z, -self.w)
    }
}

pub enum Axis {
    X,
    Y,
    Z,
    W,
}

pub trait AxisKey {
    fn get_component(&self, v: &Vector) -> f32;
    fn set_component(&self, v: &mut Vector, f: f32);
}
impl AxisKey for Axis {
    fn get_component(&self, v: &Vector) -> f32 {
        match self {
            Axis::X => v.x,
            Axis::Y => v.y,
            Axis::Z => v.z,
            Axis::W => v.w,
        }
    }
    fn set_component(&self, v: &mut Vector, f: f32) {
        match self {
            Axis::X => v.x = f,
            Axis::Y => v.y = f,
            Axis::Z => v.z = f,
            Axis::W => v.w = f,
        }
    }
}
impl AxisKey for char {
    fn get_component(&self, v: &Vector) -> f32 {
        match self {
            'x' | 'X' => v.x,
            'y' | 'Y' => v.y,
            'z' | 'Z' => v.z,
            'w' | 'W' => v.w,
            _ => panic!("invalid axis '{}'", self),
        }
    }
    fn set_component(&self, v: &mut Vector, f: f32) {
        match self {
            'x' | 'X' => v.x = f,
            'y' | 'Y' => v.y = f,
            'z' | 'Z' => v.z = f,
            'w' | 'W' => v.w = f,
            _ => panic!("invalid axis '{}'", self),
        }
    }
}
impl AxisKey for usize {
    fn get_component(&self, v: &Vector) -> f32 {
        match *self {
            0 => v.x,
            1 => v.y,
            2 => v.z,
            3 => v.w,
            _ => panic!("invalid axis index {}", self),
        }
    }
    fn set_component(&self, v: &mut Vector, f: f32) {
        match *self {
            0 => v.x = f,
            1 => v.y = f,
            2 => v.z = f,
            3 => v.w = f,
            _ => panic!("invalid axis index {}", self),
        }
    }
}