use crate::physics::hitboxes::capsule::Capsule;
use crate::physics::hitboxes::sphere::Sphere;
use crate::world::scene::Node;
use crate::math::Vector;
use crate::physics::hitboxes::bounding_box::BoundingBox;
use crate::physics::hitboxes::mesh::MeshCollider;

pub enum Hitbox {
    OBB(BoundingBox),
    MESH(MeshCollider),
    CAPSULE(Capsule),
    SPHERE(Sphere),
}
impl Hitbox {
    pub fn get_hitbox_from_node(node: &Node, hitbox_type: usize) -> Option<Hitbox> {
        assert!(hitbox_type < 4);
        if let Some(mesh) = &node.mesh {
            let scale = node.scale * node.user_scale;
            let (min, max) = mesh.borrow().get_min_max();
            let half_extent = (max - min) * 0.5 * scale;

            return Some(match hitbox_type {
                0 => {
                    Hitbox::OBB(BoundingBox {
                        center: (min + max) * scale * 0.5,
                        half_extents: half_extent,
                    })
                }
                1 => {
                    let mesh_collider = MeshCollider::new(mesh.clone(), scale);
                    Hitbox::MESH(mesh_collider)
                }
                2 => {
                    let mid = (min + max) * 0.5 * scale;
                    let radius = half_extent.with('y', 0.0).magnitude_3d();

                    let min_s = min * scale;
                    let max_s = max * scale;

                    Hitbox::CAPSULE(Capsule {
                        a: Vector::new_vec3(mid.x, max_s.y - radius, mid.z),
                        b: Vector::new_vec3(mid.x, min_s.y + radius, mid.z),
                        radius,
                    })
                }
                3 => {
                    Hitbox::SPHERE(Sphere {
                        center: (min + max) * scale * 0.5,
                        radius: half_extent.max_of(),
                    })
                }
                _ => unreachable!()
            })
        }
        None
    }

    pub fn get_type(&self) -> HitboxType {
        match self {
            Hitbox::OBB(_) => HitboxType::OBB,
            Hitbox::MESH(_) => HitboxType::MESH,
            Hitbox::CAPSULE(_) => HitboxType::CAPSULE,
            Hitbox::SPHERE(_) => HitboxType::SPHERE,
        }
    }
}
impl Clone for Hitbox {
    fn clone(&self) -> Self {
        match self {
            Hitbox::OBB(bounding_box) => Hitbox::OBB(bounding_box.clone()),
            Hitbox::MESH(collider) => Hitbox::MESH(collider.clone()),
            Hitbox::CAPSULE(capsule) => Hitbox::CAPSULE(capsule.clone()),
            Hitbox::SPHERE(sphere) => Hitbox::SPHERE(sphere.clone()),
        }
    }
}
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum HitboxType {
    OBB,
    MESH,
    CAPSULE,
    SPHERE,
}