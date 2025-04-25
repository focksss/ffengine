#[derive(Debug)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vector {
    //<editor-fold desc = "constructors">
    pub fn new_vec4(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
    pub fn new_vec3(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, w: 1.0 }
    }
    pub fn new_vec2(x: f32, y: f32) -> Self {
        Self { x, y, z: 1.0, w: 1.0}
    }
    pub fn new_vec(v: f32) -> Self {
        Self { x: v, y: v, z: v, w: v }
    }
    pub fn new_empty() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, w: 0.0 }
    }
    pub fn new_from_vec(vals: Vec<f32>) -> Self {
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
    
    //<editor-fold desc = "vector vector operations">
    pub fn dot(&self, other: Vector) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }
    pub fn cross(&self, other: Vector) -> Vector {
        Vector::new_vec3(
            self.y * other.z - self.z * other.y, 
            self.z * other.x - self.x * other.z, 
            self.x * other.y - self.y * other.x)
    }
    pub fn add_vec(&self, vec: Vector) -> Vector {
        Vector::new_vec4(self.x + vec.x, self.y + vec.y, self.z + vec.z, self.w + vec.w)
    }
    pub fn sub_vec(&self, vec: Vector) -> Vector {
        Vector::new_vec4(self.x - vec.x, self.y - vec.y, self.z - vec.z, self.w - vec.w)
    }
    pub fn mul_by_vec(&self, vec: Vector) -> Vector {
        Vector::new_vec4(self.x * vec.x, self.y * vec.y, self.z * vec.z, self.w * vec.w)
    }
    pub fn div_by_vec(&self, vec: Vector) -> Vector {
        Vector::new_vec4(self.x / vec.x, self.y / vec.y, self.z / vec.z, self.w / vec.w)
    }
    //</editor-fold>
    
    //<editor-fold desc = "vector float operations"
    pub fn add_float(&self, v: f32) -> Vector {
        Vector::new_vec4(self.x + v, self.y + v, self.z + v, self.w + v)
    }
    pub fn sub_float(&self, v: f32) -> Vector {
        Vector::new_vec4(self.x - v, self.y - v, self.z - v, self.w - v)
    }
    pub fn mul_float(&self, v: f32) -> Vector {
        Vector::new_vec4(self.x * v, self.y * v, self.z * v, self.w * v)
    }
    pub fn div_float(&self, v: f32) -> Vector {
        Vector::new_vec4(self.x / v, self.y / v, self.z / v, self.w / v)
    }
    //</editor-fold>
}