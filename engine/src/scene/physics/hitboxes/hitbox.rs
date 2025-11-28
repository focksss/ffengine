use crate::scene::physics::hitboxes::capsule::Capsule;
use crate::scene::physics::hitboxes::sphere::Sphere;
use crate::scene::world::scene::{Node, World};
use crate::math::Vector;
use crate::scene::physics::hitboxes::bounding_box::BoundingBox;
use crate::scene::physics::hitboxes::convex_hull::ConvexHull;
use crate::scene::physics::hitboxes::hitbox;
use crate::scene::physics::hitboxes::mesh::MeshCollider;

pub enum Hitbox {
    OBB(BoundingBox, ConvexHull),
    Mesh(MeshCollider),
    Capsule(Capsule),
    Sphere(Sphere),
    ConvexHull(ConvexHull),
}
impl Hitbox {
    pub fn get_hitbox_from_node(world: &World, node: &Node, hitbox_type: usize) -> Option<(Hitbox, Vector)> {
        assert!(hitbox_type < 5);
        if let Some(mesh_index) = &node.mesh {
            let mesh = &world.meshes[*mesh_index];
            let scale = node.scale * node.user_scale;
            let (min, max) = mesh.get_min_max();
            let half_extent = (max - min) * 0.5 * scale;

            return Some((match hitbox_type {
                0 => {
                    let bounds = BoundingBox {
                        center: (min + max) * scale * 0.5,
                        half_extents: half_extent,
                    };
                    Hitbox::OBB(bounds, ConvexHull::from_bounds(&bounds))
                }
                1 => {
                    let mesh_collider = MeshCollider::new(mesh.clone(), scale);
                    Hitbox::Mesh(mesh_collider)
                }
                2 => {
                    let mid = (min + max) * 0.5 * scale;
                    let radius = half_extent.with('y', 0.0).magnitude3();

                    let min_s = min * scale;
                    let max_s = max * scale;

                    Hitbox::Capsule(Capsule {
                        a: Vector::new3(mid.x, max_s.y - radius, mid.z),
                        b: Vector::new3(mid.x, min_s.y + radius, mid.z),
                        radius,
                    })
                }
                3 => {
                    Hitbox::Sphere(Sphere {
                        center: (min + max) * scale * 0.5,
                        radius: half_extent.max_of(),
                    })
                }
                4 => {
                    let mut vertices = Vec::new();

                    for primitive in &mesh.primitives {
                        for vertex in primitive.vertex_data.iter() {
                            vertices.push(scale * Vector::from_array(&vertex.position));
                        }
                    }
                    Hitbox::ConvexHull(ConvexHull::new(vertices))
                }
                _ => unreachable!()
            }, scale))
        }
        None
    }

    pub fn get_type(&self) -> HitboxType {
        match self {
            Hitbox::OBB(_, _) => HitboxType::OBB,
            Hitbox::Mesh(_) => HitboxType::MESH,
            Hitbox::Capsule(_) => HitboxType::CAPSULE,
            Hitbox::Sphere(_) => HitboxType::SPHERE,
            Hitbox::ConvexHull(_) => HitboxType::CONVEX
        }
    }

    /// Direction must be normalized
    pub fn get_furthest_point(&self, direction: &Vector, position: &Vector, bias: f32) -> Vector {
        match self {
            Hitbox::Sphere(sphere) => {
                position + sphere.center + direction * (sphere.radius + bias)
            }
            Hitbox::OBB(_, convex) => {
                position + ConvexHull::furthest_point(&convex.points, direction).0
            }
            Hitbox::ConvexHull(convex) => {
                position + ConvexHull::furthest_point(&convex.points, direction).0
            }
            _ => *position
        }
    }

    pub fn fastest_linear_speed(&self, center_of_mass: &Vector, angular_velocity: &Vector, direction: &Vector) -> f32 {
        match self {
            Hitbox::Sphere(sphere) => {
                0.0
            }
            Hitbox::OBB(_, convex) => {
                convex.largest_linear_speed(center_of_mass, angular_velocity, direction)
            }
            Hitbox::ConvexHull(convex) => {
                convex.largest_linear_speed(center_of_mass, angular_velocity, direction)
            }
            _ => 0.0
        }
    }
}
impl Clone for Hitbox {
    fn clone(&self) -> Self {
        match self {
            Hitbox::OBB(bounding_box, convex_hull) => Hitbox::OBB(bounding_box.clone(), convex_hull.clone()),
            Hitbox::Mesh(collider) => Hitbox::Mesh(collider.clone()),
            Hitbox::Capsule(capsule) => Hitbox::Capsule(capsule.clone()),
            Hitbox::Sphere(sphere) => Hitbox::Sphere(sphere.clone()),
            Hitbox::ConvexHull(convex) => Hitbox::ConvexHull(convex.clone()),
        }
    }
}
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum HitboxType {
    OBB,
    MESH,
    CAPSULE,
    SPHERE,
    CONVEX
}