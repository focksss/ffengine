use crate::math::Vector;

#[derive(Debug, Copy, Clone)]
pub struct Sphere {
    pub center: Vector,
    pub radius: f32,
}
impl Sphere {
    pub fn ray_sphere(o: &Vector, d: &Vector, center: &Vector, radius: f32) -> Option<(f32, f32)> {
        let m = center - o;
        let a = d.dot3(d);
        let b = m.dot3(d);
        let c = m.dot3(&m) - radius * radius;

        let delta = b*b - a*c;
        let inv_a = 1.0 / a;

        if delta < 0.0 { return None }

        let delta_root = delta.sqrt();
        let t1 = inv_a * ( b - delta_root );
        let t2 = inv_a * ( b + delta_root );

        Some((t1, t2))
    }
}