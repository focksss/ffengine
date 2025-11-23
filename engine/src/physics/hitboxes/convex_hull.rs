use crate::math::Vector;
use crate::physics::hitboxes::bounding_box::BoundingBox;
use parry3d::transformation::convex_hull;
use parry3d::math::Point;

#[derive(Clone)]
pub struct ConvexHull {
    pub points: Vec<Vector>,
    pub triangle_vert_indices: Vec<(usize, usize, usize)>,
}
impl ConvexHull {
    pub fn from_bounds(bounds: &BoundingBox) -> ConvexHull {
        let min = bounds.center - bounds.half_extents;
        let max = bounds.center + bounds.half_extents;
        Self {
            points: vec![
                min,
                Vector::new_vec3(max.x, min.y, min.z),
                Vector::new_vec3(max.x, min.y, max.z),
                Vector::new_vec3(min.x, min.y, max.z),
                Vector::new_vec3(min.x, max.y, min.z),
                Vector::new_vec3(max.x, max.y, min.z),
                max,
                Vector::new_vec3(min.x, max.y, max.z),
            ],
            triangle_vert_indices: Vec::new()
        }
    }

    pub fn largest_linear_speed(&self, center_of_mass: &Vector, angular_velocity: &Vector, direction: &Vector) -> f32 {
        let mut max_speed = 0.0;
        for point in &self.points {
            let r = point - center_of_mass;
            let linear_velocity = angular_velocity.cross(&r);
            let speed = direction.dot3(&linear_velocity);
            if speed > max_speed {
                max_speed = speed;
            }
        }
        max_speed
    }

    pub fn furthest_point(points: &Vec<Vector>, direction: &Vector) -> (Vector, usize) {
        let mut furthest_point = &points[0];
        let mut furthest_index = 0;
        let mut furthest_distance = direction.dot3(&furthest_point);
        for i in 1..points.len() {
            let point = &points[i];
            let dist = direction.dot3(point);
            if dist > furthest_distance {
                furthest_distance = dist;
                furthest_point = point;
                furthest_index = i;
            }
        }
        (furthest_point.clone(), furthest_index)
    }

    pub fn new(points: Vec<Vector>) -> ConvexHull {
        let parry_points: Vec<Point<f32>> = points.iter()
            .map(|v| Point::new(v.x, v.y, v.z))
            .collect();
        
        let (vertices, indices) = convex_hull(&parry_points);
        
        let hull_points: Vec<Vector> = vertices.iter()
            .map(|p| Vector::new_vec3(p.x, p.y, p.z))
            .collect();
        
        let hull_tris: Vec<(usize, usize, usize)> = indices.iter()
            .map(|tri| (tri[0] as usize, tri[1] as usize, tri[2] as usize))
            .collect();

        ConvexHull {
            points: hull_points,
            triangle_vert_indices: hull_tris,
        }
    }
}
struct Edge {
    a: usize,
    b: usize,
}
impl PartialEq for Edge {
    fn eq(&self, other: &Self) -> bool {
        (self.a == other.a && self.b == other.b) || (self.a == other.b && self.b == other.a)
    }
}