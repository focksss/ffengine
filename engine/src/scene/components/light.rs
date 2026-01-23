use crate::math::Vector;
use crate::scene::components::transform::Transform;
use crate::scene::world::world::LightSendable;

pub struct LightComponent {
    pub owner: usize,
    pub transform: usize,

    pub color: Vector,
    pub light_type: u32,
    pub quadratic_falloff: f32,
    pub linear_falloff: f32,
    pub constant_falloff: f32,
    pub inner_cutoff: f32,
    pub outer_cutoff: f32,
}
impl LightComponent {
    pub fn new(position: Vector, direction: Vector, color: Vector) -> LightComponent {
        LightComponent {
            owner: 0,
            transform: 0,
            color,
            light_type: 0,
            quadratic_falloff: 0.1,
            linear_falloff: 0.1,
            constant_falloff: 0.1,
            inner_cutoff: 0.0,
            outer_cutoff: 0.0,
        }
    }
    pub fn to_sendable(&self, transform: &Transform) -> LightSendable {
        LightSendable {
            position: transform.world_translation.to_array3(),
            _pad0: 0u32,
            direction: Vector::new3(0.0, 0.0, 1.0).rotate_by_quat(&transform.world_rotation).to_array3(),
            light_type: self.light_type,
            attenuation_values:
            if self.light_type == 0 { [self.quadratic_falloff, self.linear_falloff, self.constant_falloff] }
            else { [self.inner_cutoff, self.outer_cutoff, 0.0] },
            _pad1: 0u32,
            color: self.color.to_array3(),
            _pad2: 0u32,
        }
    }
}