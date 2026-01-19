use crate::math::Vector;

const MAX_ITERATIONS: usize = 5;
const MIN_MOVE_THRESHOLD: f32 = 0.001;

pub struct PhysicsEngine {
    pub gravity: Vector,
    pub air_resistance_coefficient: f32,
    pub player_horiz_const_resistance: f32,
}

impl PhysicsEngine {
    pub fn new(gravity: Vector, air_resistance_coefficient: f32, player_horiz_const_resistance: f32) -> Self {
        Self {
            gravity,
            air_resistance_coefficient,
            player_horiz_const_resistance,
        }
    }

    fn axis_angle_to_quat(axis: &Vector, angle: f32) -> Vector {
        let half_angle = angle * 0.5;
        let s = half_angle.sin();
        Vector::new4(
            axis.x * s,
            axis.y * s,
            axis.z * s,
            half_angle.cos()
        )
    }
}
#[derive(Clone, Copy)]
pub enum AxisType {
    FaceA(usize),
    FaceB(usize),
    Edge(usize, usize),
}

struct CastInformation {
    distance: f32,
    contacts: Vec<ContactInformation>,
}
pub struct ContactInformation {
    pub contact_points: Vec<ContactPoint>,
    pub time_of_impact: f32,
    pub normal: Vector,
}
impl ContactInformation {
    pub fn flip(mut self) -> ContactInformation {
        for point in &mut self.contact_points { point.flip(); }
        self.normal = -self.normal;
        self
    }
}
#[derive(Debug)]
pub struct ContactPoint {
    pub point_on_a: Vector,
    pub point_on_b: Vector,

    pub penetration: f32,
}
impl ContactPoint {
    fn flip(&mut self) {
        let temp = self.point_on_b;
        self.point_on_b = self.point_on_a;
        self.point_on_a = temp;
    }
}