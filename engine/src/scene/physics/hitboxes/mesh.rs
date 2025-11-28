use std::cell::RefCell;
use std::rc::Rc;
use crate::math::Vector;
use crate::scene::physics::hitboxes::bounding_box::BoundingBox;
use crate::scene::world::scene::{Mesh, Vertex};

///* Does not support non-uniform scaling, as is the standard(?) with physics engines. Must call .rescale() to rescale the bvh to be nonuniform.
pub struct MeshCollider {
    pub current_scale_multiplier: f32,
    pub current_scale_factor: Vector,
    pub bvh: Rc<RefCell<Bvh>>,
}
impl MeshCollider {
    pub fn new(mesh: &Mesh, scale: Vector) -> Self {
        MeshCollider {
            current_scale_factor: scale,
            current_scale_multiplier: 1.0,
            bvh: Rc::new(RefCell::new(Bvh::new(mesh, scale))),
        }
    }

    pub fn rescale_bvh(&mut self, new_scale: Vector) {
        if self.bvh.borrow().active_scale_factor.equals(&new_scale, 1e-6) { return }
        self.bvh.borrow_mut().rescale_bvh_bounds(&new_scale, &self.current_scale_factor);
        self.current_scale_factor = new_scale;
    }
}
impl Clone for MeshCollider {
    fn clone(&self) -> Self {
        Self {
            current_scale_factor: self.current_scale_factor.clone(),
            current_scale_multiplier: self.current_scale_multiplier,
            bvh: self.bvh.clone()
        }
    }
}
pub struct Bvh {
    pub active_scale_factor: Vector,
    pub(crate) bounds: BoundingBox,
    pub(crate) left_child: Option<Rc<RefCell<Bvh>>>,
    pub(crate) right_child: Option<Rc<RefCell<Bvh>>>,
    pub(crate) triangle_indices: Option<Vec<usize>>,
}

impl Bvh {
    pub fn get_bounds_info(bvh: &Rc<RefCell<Bvh>>) -> Vec<(Vector, Vector)> { // centers, half extents
        let mut constants = Vec::new();
        Bvh::bounds_stack_add_bvh(&bvh, &mut constants);
        constants
    }
    fn bounds_stack_add_bvh(bvh: &Rc<RefCell<Bvh>>, constants: &mut Vec<(Vector, Vector)>) {
        let bvh = bvh.borrow();
        constants.push((
            bvh.bounds.center.clone(),
            bvh.bounds.half_extents.clone()
        ));
        if let Some(left_child) = &bvh.left_child {
            Bvh::bounds_stack_add_bvh(left_child, constants);
        }
        if let Some(right_child) = &bvh.right_child {
            Bvh::bounds_stack_add_bvh(right_child, constants);
        }
    }

    pub fn rescale_bvh_bounds(&mut self, new_scale: &Vector, old_scale: &Vector) {
        self.bounds.half_extents = (self.bounds.half_extents / old_scale) * new_scale;
        self.bounds.center = (self.bounds.center / old_scale) * new_scale;
        if let Some(left_child) = self.left_child.clone() {
            left_child.borrow_mut().rescale_bvh_bounds(&new_scale, &old_scale);
        }
        if let Some(right_child) = self.right_child.clone() {
            right_child.borrow_mut().rescale_bvh_bounds(&new_scale, &old_scale);
        }
        self.active_scale_factor = new_scale.clone();
    }

    pub fn new(mesh: &Mesh, scale: Vector) -> Bvh {
        let mut triangles = Vec::new();

        for primitive in &mesh.primitives {
            let indices: Vec<u32> = if primitive.index_data_u8.len() > 0 {
                primitive.index_data_u8.iter().map(|i| *i as u32).collect()
            } else if primitive.index_data_u16.len() > 0 {
                primitive.index_data_u16.iter().map(|i| *i as u32).collect()
            } else if primitive.index_data_u32.len() > 0 {
                primitive.index_data_u32.clone()
            } else {
                panic!("mesh does not have indices")
            };

            for i in (0..indices.len()).step_by(3) {
                let v0 = &primitive.vertex_data[indices[i] as usize];
                let v1 = &primitive.vertex_data[indices[i + 1] as usize];
                let v2 = &primitive.vertex_data[indices[i + 2] as usize];

                let centroid = Self::centroid(v0, v1, v2);
                triangles.push((i / 3, centroid));
            }
        }

        let num_tris = triangles.len();
        Self::split(&mesh, &mut triangles, 0, num_tris, &scale)
    }

    fn split(
        mesh: &Mesh,
        triangles: &mut [(usize, Vector)],
        start: usize,
        end: usize,
        scale: &Vector,
    ) -> Bvh {
        let (mut min, mut max) = Self::min_max(mesh, triangles, start, end);
        min = min * scale; max = max * scale;
        let num_triangles = end - start;

        const MAX_LEAF_SIZE: usize = 4;

        if num_triangles <= MAX_LEAF_SIZE {
            let triangle_indices: Vec<usize> = triangles[start..end]
                .iter()
                .map(|(idx, _)| *idx)
                .collect();

            return Bvh {
                active_scale_factor: scale.clone(),
                bounds: BoundingBox::from_min_max(min, max),
                left_child: None,
                right_child: None,
                triangle_indices: Some(triangle_indices),
            };
        }

        let extent = max - min;
        let axis = if extent.x > extent.y && extent.x > extent.z {
            'x'
        } else if extent.y > extent.z {
            'y'
        } else {
            'z'
        };

        Self::sort_triangles_by_axis(&mut triangles[start..end], axis);

        let mid = start + num_triangles / 2;

        let left_child = Some(Rc::new(RefCell::new(Self::split(
            mesh.clone(),
            triangles,
            start,
            mid,
            scale
        ))));

        let right_child = Some(Rc::new(RefCell::new(Self::split(
            mesh.clone(),
            triangles,
            mid,
            end,
            scale
        ))));

        Bvh {
            active_scale_factor: scale.clone(),
            bounds: BoundingBox::from_min_max(min, max),
            left_child,
            right_child,
            triangle_indices: None,
        }
    }

    fn min_max(
        mesh: &Mesh,
        triangles: &[(usize, Vector)],
        start: usize,
        end: usize
    ) -> (Vector, Vector) {
        let mut min = Vector::fill(f32::MAX);
        let mut max = Vector::fill(f32::MIN);

        for (triangle_idx, _) in &triangles[start..end] {
            let (v0, v1, v2) = Self::get_triangle_vertices(&mesh, *triangle_idx, None);

            min = Vector::min(&min, &Vector::from_array(&v0.position));
            min = Vector::min(&min, &Vector::from_array(&v1.position));
            min = Vector::min(&min, &Vector::from_array(&v2.position));

            max = Vector::max(&max, &Vector::from_array(&v0.position));
            max = Vector::max(&max, &Vector::from_array(&v1.position));
            max = Vector::max(&max, &Vector::from_array(&v2.position));
        }

        (min, max)
    }

    pub fn get_triangle_vertices(mesh: &Mesh, triangle_index: usize, scale_factor: Option<Vector>) -> (Vertex, Vertex, Vertex) {
        let primitive = &mesh.primitives[0];

        let idx0;
        let idx1;
        let idx2;

        if primitive.index_data_u8.len() > 0 {
            idx0 = primitive.index_data_u8[3 * triangle_index] as usize;
            idx1 = primitive.index_data_u8[3 * triangle_index + 1] as usize;
            idx2 = primitive.index_data_u8[3 * triangle_index + 2] as usize;
        } else if primitive.index_data_u16.len() > 0 {
            idx0 = primitive.index_data_u16[3 * triangle_index] as usize;
            idx1 = primitive.index_data_u16[3 * triangle_index + 1] as usize;
            idx2 = primitive.index_data_u16[3 * triangle_index + 2] as usize;
        } else if primitive.index_data_u32.len() > 0 {
            idx0 = primitive.index_data_u32[3 * triangle_index] as usize;
            idx1 = primitive.index_data_u32[3 * triangle_index + 1] as usize;
            idx2 = primitive.index_data_u32[3 * triangle_index + 2] as usize;
        } else {
            panic!("mesh does not have indices")
        }

        let mut v0 = primitive.vertex_data[idx0].clone();
        let mut v1 = primitive.vertex_data[idx1].clone();
        let mut v2 = primitive.vertex_data[idx2].clone();

        if let Some(scale) = scale_factor {
            v0.position = (Vector::from_array(&v0.position) * scale).to_array3();
            v0.normal = (Vector::from_array(&v0.normal) * scale).normalize3().to_array3();

            v1.position = (Vector::from_array(&v1.position) * scale).to_array3();
            v1.normal = (Vector::from_array(&v1.normal) * scale).normalize3().to_array3();

            v2.position = (Vector::from_array(&v2.position) * scale).to_array3();
            v2.normal = (Vector::from_array(&v2.normal) * scale).normalize3().to_array3();
        }

        (v0, v1, v2)
    }

    fn sort_triangles_by_axis(triangles: &mut [(usize, Vector)], axis: char) {
        match axis {
            'x' => triangles.sort_by(|a, b| a.1.x.partial_cmp(&b.1.x).unwrap()),
            'y' => triangles.sort_by(|a, b| a.1.y.partial_cmp(&b.1.y).unwrap()),
            'z' => triangles.sort_by(|a, b| a.1.z.partial_cmp(&b.1.z).unwrap()),
            _ => panic!("Unknown axis"),
        }
    }

    fn centroid(a: &Vertex, b: &Vertex, c: &Vertex) -> Vector {
        (Vector::from_array(&a.position)
            + Vector::from_array(&b.position)
            + Vector::from_array(&c.position)) / 3.0
    }
}
impl Clone for Bvh {
    fn clone(&self) -> Self {
        Self {
            active_scale_factor: self.active_scale_factor.clone(),
            bounds: self.bounds.clone(),
            left_child: self.left_child.clone(),
            right_child: self.right_child.clone(),
            triangle_indices: self.triangle_indices.clone(),
        }
    }
}