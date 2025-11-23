use crate::math::Vector;
use crate::physics::hitboxes::bounding_box::BoundingBox;

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
    
    pub fn new(mut points: Vec<Vector>) -> ConvexHull {
        let dist_from_line = |point: &Vector, a: &Vector, b: &Vector| -> f32 {
            let ab = (b - a).normalize_3d();
            let ray = point - a;
            let proj = ab * ray.dot3(&ab);
            let perp = ray - proj;
            perp.magnitude_3d()
        };
        let furthest_point_from_line = |pts: &Vec<Vector>, a: &Vector, b: &Vector| -> Vector {
            let mut furthest_point = &pts[0];
            let mut furthest_dist = dist_from_line(&furthest_point, a, b);
            for i in 1..pts.len() {
                let point = &pts[i];
                let dist = dist_from_line(&point, a, b);
                if dist > furthest_dist {
                    furthest_dist = dist;
                    furthest_point = point;
                }
            }
            furthest_point.clone()
        };
        let furthest_point_from_tri = |pts: &Vec<Vector>, tri: (&Vector, &Vector, &Vector)| -> Vector {
            let mut furthest_point = &pts[0];
            let mut furthest_dist = Self::dist_from_tri(&furthest_point, tri);
            for i in 1..pts.len() {
                let point = &pts[i];
                let dist = Self::dist_from_tri(&point, tri);
                if dist > furthest_dist {
                    furthest_dist = dist;
                    furthest_point = point;
                }
            }
            furthest_point.clone()
        };

        // construct tetrahedron
        let mut hull_points = Vec::new();
        hull_points.push(Self::furthest_point(&points, &Vector::new_vec3(0.0, 1.0, 0.0)).0);
        hull_points.push(Self::furthest_point(&points, &-hull_points[0]).0);
        hull_points.push(furthest_point_from_line(&points, &hull_points[0], &hull_points[1]));
        hull_points.push(furthest_point_from_tri(&points, (&hull_points[0], &hull_points[1], &hull_points[2])));
        // ensure ccw
        let dist = Self::dist_from_tri(&hull_points[3], (&hull_points[0], &hull_points[1], &hull_points[2]));
        if dist > 0.0 { hull_points.swap(0, 1); }

        let mut hull_tris = Vec::new();
        hull_tris.push((0, 1, 2));
        hull_tris.push((0, 2, 3));
        hull_tris.push((2, 1, 3));
        hull_tris.push((1, 0, 3));

        // expand tetrahedron into full hull
        // prune contained points (make a hull convex)
        Self::remove_internal_points(&mut points, &mut hull_points, &hull_tris);
        // expand
        while points.len() > 0 {
            println!("points left: {}", points.len());
            let (point, point_index) = Self::furthest_point(&points, &points[0]);

            points.swap_remove(point_index);
            Self::add_point(&mut hull_points, &mut hull_tris, point);
            Self::remove_internal_points(&mut points, &mut hull_points, &hull_tris);
        }
        // remove unused points
        let mut i = 0;
        while i < hull_points.len() {
            let mut used = false;
            for tri in hull_tris.iter() {
                if tri.0 == i || tri.1 == i || tri.2 == i {
                    used = true;
                    break;
                }
            }
            if used {
                i += 1;
                continue;
            }
            // not used, remove
            for tri in hull_tris.iter_mut() {
                if tri.0 > i {
                    tri.0 -= 1;
                }
                if tri.1 > i {
                    tri.1 -= 1;
                }
                if tri.2 > i {
                    tri.2 -= 1;
                }
            }
            hull_points.swap_remove(i);
        }

        ConvexHull {
            points: hull_points,
            triangle_vert_indices: hull_tris,
        }
    }
    fn dist_from_tri(point: &Vector, tri: (&Vector, &Vector, &Vector)) -> f32 {
        let ab = tri.1 - tri.0;
        let ac = tri.2 - tri.0;
        let n = ab.cross(&ac).normalize_3d();
        let ray = point - tri.0;
        ray.dot3(&n)
    }
    fn edge_unique_test(hull_tris: &Vec<(usize, usize, usize)>, tris_facing: &Vec<usize>, ignore_tri_facing: usize, edge: &Edge) -> bool {
        for tri_facing in tris_facing {
            if *tri_facing == ignore_tri_facing { continue; }
            let tri = hull_tris[*tri_facing];

            let edges = [
                Edge { a: tri.0, b: tri.1 },
                Edge { a: tri.1, b: tri.2 },
                Edge { a: tri.2, b: tri.0 },
            ];
            for tri_edge in &edges {
                if edge == tri_edge {
                    return false
                }
            }
        }
        true
    }
    fn remove_internal_points(points: &mut Vec<Vector>, hull_points: &mut Vec<Vector>, hull_tris: &Vec<(usize, usize, usize)>) {
        let mut i = 0;
        while i < points.len() {
            let point = &points[i];

            let mut inside = true;
            for tri_vert_indices in hull_tris {
                let a = &hull_points[tri_vert_indices.0];
                let b = &hull_points[tri_vert_indices.1];
                let c = &hull_points[tri_vert_indices.2];

                let dist = Self::dist_from_tri(point, (a, b, c));
                if dist > 0.0 { inside = false; break; }
            }
            if inside {
                points.swap_remove(i);
            } else {
                i += 1;
            }
        }
        // prune "overlapping" points
        let mut i = 0;
        while i < points.len() {
            let point = &points[i];

            let mut too_close = false;
            for hull_point in hull_points.iter() {
                if (hull_point - point).magnitude_3d() < 0.01 {
                    too_close = true;
                    break;
                }
            }
            if too_close {
                points.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }
    fn add_point(hull_points: &mut Vec<Vector>, hull_tris: &mut Vec<(usize, usize, usize)>, point: Vector) {
        let mut tris_facing = Vec::new();
        for tri_idx in 0..hull_tris.len() {
            let tri = &hull_tris[tri_idx];
            let a = &hull_points[tri.0];
            let b = &hull_points[tri.1];
            let c = &hull_points[tri.2];

            let dist = Self::dist_from_tri(&point, (a, b, c));
            if dist > 0.0 { tris_facing.push(tri_idx); }
        }
        let mut unique_edges = Vec::new();
        for tri_facing_idx in 0..tris_facing.len() {
            let tri_facing = &tris_facing[tri_facing_idx];
            let tri = &hull_tris[*tri_facing];
            let edges = [
                Edge { a: tri.0, b: tri.1 },
                Edge { a: tri.1, b: tri.2 },
                Edge { a: tri.2, b: tri.0 },
            ];
            for edge in edges {
                if Self::edge_unique_test(&hull_tris, &tris_facing, *tri_facing, &edge) {
                    unique_edges.push(edge);
                }
            }
        }
        // remove old tris facing point
        let tris_facing_set: std::collections::HashSet<_> = tris_facing.into_iter().collect();
        *hull_tris = hull_tris.iter()
            .enumerate()
            .filter(|(i, _)| !tris_facing_set.contains(i))
            .map(|(_, tri)| *tri)
            .collect();
        // add
        let new_id = hull_points.len();
        hull_points.push(point);
        // add tris for unique edges
        for edge in unique_edges {
            hull_tris.push((edge.a, edge.b, new_id))
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