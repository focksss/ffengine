use crate::math::Vector;
use crate::scene::physics::hitboxes::bounding_box::BoundingBox;
use parry3d::transformation::convex_hull;
use parry3d::math::Point;
use rand::prelude::SliceRandom;
use rand::{rng, thread_rng};
use crate::math::matrix::Matrix;

#[derive(Clone)]
pub struct ConvexHull {
    pub points: Vec<Vector>,
    pub triangle_vert_indices: Vec<(usize, usize, usize)>,
    pub min_max: (Vector, Vector),
}
impl ConvexHull {
    pub fn from_bounds(bounds: &BoundingBox) -> ConvexHull {
        let min = bounds.center - bounds.half_extents;
        let max = bounds.center + bounds.half_extents;
        Self {
            points: vec![
                min,
                Vector::new3(max.x, min.y, min.z),
                Vector::new3(max.x, min.y, max.z),
                Vector::new3(min.x, min.y, max.z),
                Vector::new3(min.x, max.y, min.z),
                Vector::new3(max.x, max.y, min.z),
                max,
                Vector::new3(min.x, max.y, max.z),
            ],
            triangle_vert_indices: Vec::new(),
            min_max: (min, max),
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

    pub fn center_of_mass(&self, max_samples: usize) -> Vector {
        assert!(self.points.len() > 0);
        let min = &self.min_max.0;
        let extent = self.min_max.1 - min;
        let dx = extent.x / max_samples as f32;
        let dy = extent.y / max_samples as f32;
        let dz = extent.z / max_samples as f32;
        let dp = dx.max(dy.max(dz));
        let max_x_iters = (extent.x / dp).ceil() as usize;
        let max_y_iters = (extent.y / dp).ceil() as usize;
        let max_z_iters = (extent.z / dp).ceil() as usize;

        let mut point_sum = Vector::empty();
        let mut internal_points = 0;
        for x_iter in 0..max_x_iters {
            for y_iter in 0..max_y_iters {
                for z_iter in 0..max_z_iters {
                    let point = Vector::new3(
                        min.x + dp * (x_iter as f32 + 0.5),
                        min.y + dp * (y_iter as f32 + 0.5),
                        min.z + dp * (z_iter as f32 + 0.5),
                    );
                    if Self::is_contained(&point, &self.points, &self.triangle_vert_indices) {
                        point_sum += point;
                        internal_points += 1;
                    }
                }
            }
        }
        if internal_points > 0 {
            point_sum / internal_points as f32
        } else {
            min + extent * 0.5
        }
    }
    pub fn inertia_tensor(&self, center_of_mass: &Vector, max_samples: usize) -> Matrix {
        assert!(self.points.len() > 0);
        let min = &self.min_max.0;
        let extent = self.min_max.1 - min;
        let dx = extent.x / max_samples as f32;
        let dy = extent.y / max_samples as f32;
        let dz = extent.z / max_samples as f32;
        let dp = dx.max(dy.max(dz));
        let max_x_iters = (extent.x / dp).ceil() as usize;
        let max_y_iters = (extent.y / dp).ceil() as usize;
        let max_z_iters = (extent.z / dp).ceil() as usize;

        let mut tensor_sum = Matrix::new();
        let mut internal_points = 0;
        for x_iter in 0..max_x_iters {
            for y_iter in 0..max_y_iters {
                for z_iter in 0..max_z_iters {
                    let mut point = Vector::new3(
                        min.x + dp * (x_iter as f32 + 0.5),
                        min.y + dp * (y_iter as f32 + 0.5),
                        min.z + dp * (z_iter as f32 + 0.5),
                    );
                    if Self::is_contained(&point, &self.points, &self.triangle_vert_indices) {
                        point -= center_of_mass;

                        tensor_sum.set(0, 0, tensor_sum.get(0, 0) + point.y*point.y + point.z*point.z);
                        tensor_sum.set(1, 1, tensor_sum.get(1, 1) + point.x*point.x + point.z*point.z);
                        tensor_sum.set(2, 2, tensor_sum.get(2, 2) + point.x*point.x + point.y*point.y);

                        tensor_sum.set(0, 1, tensor_sum.get(0, 1) - point.x*point.y);
                        tensor_sum.set(0, 2, tensor_sum.get(0, 2) - point.x*point.z);
                        tensor_sum.set(1, 2, tensor_sum.get(1, 2) - point.y*point.z);

                        tensor_sum.set(1, 0, tensor_sum.get(1, 0) - point.x*point.y);
                        tensor_sum.set(2, 0, tensor_sum.get(2, 0) - point.x*point.z);
                        tensor_sum.set(2, 1, tensor_sum.get(2, 1) - point.y*point.z);

                        internal_points += 1;
                    }
                }
            }
        }
        tensor_sum /= internal_points as f32;
        tensor_sum
    }

    pub fn new(points: Vec<Vector>) -> ConvexHull {
        let parry_points: Vec<Point<f32>> = points.iter()
            .map(|v| Point::new(v.x, v.y, v.z))
            .collect();

        let (vertices, indices) = convex_hull(&parry_points);

        let hull_points: Vec<Vector> = vertices.iter()
            .map(|p| Vector::new3(p.x, p.y, p.z))
            .collect();

        let hull_tris: Vec<(usize, usize, usize)> = indices.iter()
            .map(|tri| (tri[0] as usize, tri[1] as usize, tri[2] as usize))
            .collect();

        let mut min = hull_points[0];
        let mut max = hull_points[0];
        for point in &hull_points {
            if point.x < min.x { min.x = point.x; }
            if point.y < min.y { min.y = point.y; }
            if point.z < min.z { min.z = point.z; }
            if point.x > max.x { max.x = point.x; }
            if point.y > max.y { max.y = point.y; }
            if point.z > max.z { max.z = point.z; }
        }

        ConvexHull {
            points: hull_points,
            triangle_vert_indices: hull_tris,
            min_max: (min, max),
        }
    }
    fn dist_from_tri(point: &Vector, tri: (&Vector, &Vector, &Vector)) -> f32 {
        let ab = tri.1 - tri.0;
        let ac = tri.2 - tri.0;
        let n = ab.cross(&ac).normalize3();
        let ray = point - tri.0;
        ray.dot3(&n)
    }
    fn is_contained(point: &Vector, hull_points: &Vec<Vector>, hull_tris: &Vec<(usize, usize, usize)>) -> bool {
        let mut inside = true;
        for tri_vert_indices in hull_tris {
            let a = &hull_points[tri_vert_indices.0];
            let b = &hull_points[tri_vert_indices.1];
            let c = &hull_points[tri_vert_indices.2];

            let dist = Self::dist_from_tri(point, (a, b, c));
            if dist > 0.0 {
                inside = false;
                break;
            }
        }
        inside
    }
}