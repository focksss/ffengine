use std::cell::RefCell;
use std::sync::Arc;
use crate::math::matrix::Matrix;
use crate::math::Vector;
use crate::physics::hitboxes::bounding_box::BoundingBox;
use crate::physics::hitboxes::capsule::Capsule;
use crate::physics::hitboxes::hitbox::{Hitbox, HitboxType};
use crate::physics::hitboxes::mesh::{Bvh, MeshCollider};
use crate::physics::hitboxes::sphere::Sphere;
use crate::physics::physics_engine::{AxisType, ContactInformation, ContactPoint, PhysicsEngine};
use crate::world::scene::{Node, Scene, Vertex};

#[derive(Clone)]
pub struct RigidBodyPointer {
    pub physics_engine: Arc<RefCell<PhysicsEngine>>,
    pub index: usize
}
pub struct RigidBody {
    pub owned_by_player: bool,

    pub coupled_with_scene_object: Option<(usize, usize)>, // model index, node index

    pub hitbox: Hitbox,
    pub(crate) is_static: bool,
    pub restitution_coefficient: f32,
    pub friction_coefficient: f32,
    pub(crate) mass: f32,
    pub(crate) inv_mass: f32,

    pub force: Vector,
    pub torque: Vector,

    pub position: Vector,
    pub velocity: Vector,
    pub orientation: Vector, // quaternion
    pub angular_velocity: Vector,
    center_of_mass: Vector,

    inertia_tensor: Matrix, // 3x3
    inv_inertia_tensor: Matrix,
}

impl RigidBody {
    pub fn new_from_node(node: &Node, couple_with_node: Option<(usize, usize)>, hitbox: Hitbox) -> Self {
        let mut body = RigidBody::default();
        body.coupled_with_scene_object = couple_with_node;
        body.position = node.world_transform * Vector::new_vec4(0.0, 0.0, 0.0, 1.0);
        body.orientation = node.world_transform.extract_quaternion();
        body.hitbox = hitbox;
        body.update_shape_properties();
        body
    }
    pub fn update_this_according_to_coupled(&mut self, world: &Scene)  {
        if let Some(coupled_object) = self.coupled_with_scene_object {
            let node = &world.models[coupled_object.0].nodes[coupled_object.1];

            self.position = node.world_transform * Vector::new_vec4(0.0, 0.0, 0.0, 1.0);
            self.orientation = node.world_transform.extract_quaternion();
            self.hitbox = Hitbox::get_hitbox_from_node(node, match self.hitbox {
                Hitbox::OBB(_) => 0,
                Hitbox::MESH(_) => 1,
                Hitbox::CAPSULE(_) => 2,
                Hitbox::SPHERE(_) => 3,
            }).unwrap();
            self.update_shape_properties();
        }
    }
    pub fn update_coupled_according_to_this(&self, world: &mut Scene) {
        if let Some(coupled_object) = self.coupled_with_scene_object {
            let node = &mut world.models[coupled_object.0].nodes[coupled_object.1];

            node.translation = self.position;
            node.rotation = self.orientation;
            node.needs_update = true;
        }
    }
    pub fn set_mass(&mut self, mass: f32) {
        self.mass = mass;
        self.inv_mass = 1.0 / mass;
        self.update_shape_properties();
    }
    pub fn set_static(&mut self, is_static: bool) {
        self.is_static = is_static;
        if is_static {
            self.inv_mass = 0.0;
            self.mass = 999.0
        } else {
            self.inv_mass = 1.0 / self.mass;
        }
        self.update_shape_properties();
    }

    pub fn update_shape_properties(&mut self) {
        match self.hitbox {
            Hitbox::OBB(obb) => {
                let a = obb.half_extents.x * 2.0;
                let b = obb.half_extents.y * 2.0;
                let c = obb.half_extents.z * 2.0;

                self.inertia_tensor = Matrix::new();
                self.inertia_tensor.set(0, 0, (1.0 / 12.0) * (b * b + c * c));
                self.inertia_tensor.set(1, 1, (1.0 / 12.0) * (a * a + c * c));
                self.inertia_tensor.set(2, 2, (1.0 / 12.0) * (a * a + b * b));

                let d = -obb.center;
                let d2 = d.dot3(&d);

                // parallel axis theorem tensor
                let pat = Matrix::new_manual([
                    d2 - d.x*d.x, -d.x*d.y,     -d.x*d.z,     0.0,
                    -d.y*d.x,     d2 - d.y*d.y, -d.y*d.z,     0.0,
                    -d.z*d.x,     -d.z*d.y,     d2 - d.z*d.z, 0.0,
                    0.0,          0.0,          0.0,          0.0
                ]);

                self.inertia_tensor += pat;

                self.center_of_mass = obb.center
            }
            Hitbox::MESH(_) => {
                //TODO inertia tensor and center of mass
            }
            Hitbox::CAPSULE(capsule) => {
                //TODO inertia tensor

                self.center_of_mass = (capsule.a + capsule.b) * 0.5
            }
            Hitbox::SPHERE(sphere) => {
                let r2 = sphere.radius * sphere.radius;
                let c = 2.0 / 5.0;
                let v = c * r2;
                self.inertia_tensor = Matrix::new();
                self.inertia_tensor.set(0, 0, v);
                self.inertia_tensor.set(1, 1, v);
                self.inertia_tensor.set(2, 2, v);

                self.center_of_mass = sphere.center
            }
        }
        self.inv_inertia_tensor = self.inertia_tensor.inverse3().mul_float_into3(self.inv_mass);
    }
    pub fn get_inverse_inertia_tensor_world_space(&self) -> Matrix {
        let rot = Matrix::new_rotate_quaternion_vec4(&self.orientation);
        rot * self.inv_inertia_tensor * rot.transpose3()
    }
    pub fn get_center_of_mass_world_space(&self) -> Vector {
        self.position + Matrix::new_rotate_quaternion_vec4(&self.orientation) * self.center_of_mass
    }

    pub fn update(&mut self, delta_time: f32) {
        self.position += self.velocity * delta_time;

        let c = self.get_center_of_mass_world_space();
        let c_to_pos = self.position - c;

        let rot = Matrix::new_rotate_quaternion_vec4(&self.orientation);
        let inertia_world = rot * self.inertia_tensor * rot.transpose3();
        let inv_inertia_world = rot * self.inv_inertia_tensor * rot.transpose3();
        let torque = self.angular_velocity.cross(&(inertia_world * self.angular_velocity));
        let alpha = inv_inertia_world * torque;
        self.angular_velocity += alpha * delta_time;

        let d_theta = self.angular_velocity * delta_time;
        let dq = Vector::axis_angle_quat(&d_theta, d_theta.magnitude_3d());
        self.orientation = dq.combine(&self.orientation).normalize_4d();

        self.position = c + Matrix::new_rotate_quaternion_vec4(&dq) * c_to_pos;
    }

    pub fn apply_impulse(&mut self, impulse: Vector, point: Vector) {
        if self.inv_mass == 0.0 { return }

        self.velocity += impulse * self.inv_mass;

        let c = self.get_center_of_mass_world_space();
        let r = point - c;
        let dl = r.cross(&impulse);
        if self.owned_by_player { return }
        self.apply_angular_impulse(dl);
    }
    pub fn apply_angular_impulse(&mut self, impulse: Vector) {
        if self.inv_mass == 0.0 { return }
        self.angular_velocity += self.get_inverse_inertia_tensor_world_space() * impulse;

        const MAX_ANGULAR_SPEED: f32 = 30.0;
        if self.angular_velocity.magnitude_3d() > MAX_ANGULAR_SPEED {
            self.angular_velocity = self.angular_velocity.normalize_3d() * MAX_ANGULAR_SPEED;
        }
    }

    pub fn will_collide_with(&mut self, other: &mut RigidBody, dt: f32) -> Option<ContactInformation> {
        let other_type = other.hitbox.get_type();

        match other_type {
            HitboxType::SPHERE => self.intersects_sphere(other, dt),
            HitboxType::OBB => self.intersects_obb(other, dt),
            _ => { panic!("{:?}-{:?} intersection not implemented", self.hitbox.get_type(), other.hitbox.get_type()) }
        }
    }

    fn intersects_sphere(&mut self, other: &mut RigidBody, dt: f32) -> Option<ContactInformation> {
        if let Hitbox::SPHERE(sphere) = other.hitbox {
            return match self.hitbox {
                Hitbox::OBB(obb) => {
                    let obb_position = self.position;
                    let sphere_position = other.position;
                    let obb_orientation = self.orientation;
                    let sphere_orientation = other.orientation;
                    let rot = Matrix::new_rotate_quaternion_vec4(&obb_orientation);

                    let sphere_center = sphere.center.rotate_by_quat(&sphere_orientation) + sphere_position;
                    let obb_center = (rot * obb.center.with('w', 1.0)) + obb_position;

                    let axes = [
                        rot * Vector::new_vec4(1.0, 0.0, 0.0, 1.0),
                        rot * Vector::new_vec4(0.0, 1.0, 0.0, 1.0),
                        rot * Vector::new_vec4(0.0, 0.0, 1.0, 1.0),
                    ];

                    let delta = sphere_center - obb_center;
                    let local_sphere_center = Vector::new_vec3(delta.dot3(&axes[0]), delta.dot3(&axes[1]), delta.dot3(&axes[2]));

                    let closest_local = local_sphere_center.clamp3(&(-obb.half_extents), &obb.half_extents);
                    let closest_world = obb_center + (axes[0] * closest_local.x) + (axes[1] * closest_local.y) + (axes[2] * closest_local.z);

                    let diff = sphere_center - closest_world;
                    let dist_sq = diff.dot3(&diff);
                    let radius_sq = sphere.radius * sphere.radius;
                    if dist_sq > radius_sq {
                        return None
                    }
                    let dist = dist_sq.sqrt();

                    let (normal, penetration, point_on_obb) = if dist < 1e-6 {
                        let penetrations = [
                            (obb.half_extents.x - local_sphere_center.x.abs(), 0, local_sphere_center.x.signum()),
                            (obb.half_extents.y - local_sphere_center.y.abs(), 1, local_sphere_center.y.signum()),
                            (obb.half_extents.z - local_sphere_center.z.abs(), 2, local_sphere_center.z.signum()),
                        ];
                        let mut min_pen = penetrations[0];
                        for &pen in &penetrations[1..] {
                            if pen.0 < min_pen.0 {
                                min_pen = pen;
                            }
                        }

                        let axis_idx = min_pen.1;
                        let sign = min_pen.2;
                        let normal = axes[axis_idx] * sign;
                        let penetration = min_pen.0 + sphere.radius;

                        let mut local_point = local_sphere_center;
                        match axis_idx {
                            0 => local_point.x = obb.half_extents.x * sign,
                            1 => local_point.y = obb.half_extents.y * sign,
                            2 => local_point.z = obb.half_extents.z * sign,
                            _ => unreachable!(),
                        }

                        let point_on_obb = obb_center + (
                            axes[0] * local_point.x + axes[1] * local_point.y + axes[2] * local_point.z
                        );

                        (normal, penetration, point_on_obb)
                    } else {
                        let normal = diff * (1.0 / dist);
                        let penetration = sphere.radius - dist;
                        (normal, penetration, closest_world)
                    };

                    let point_on_sphere = sphere_center - (normal * sphere.radius);

                    let tolerance = 1e-4;
                    let mut contact_points = vec![ContactPoint {
                        point_on_a: point_on_obb,
                        point_on_b: point_on_sphere,
                        penetration,
                    }];

                    let on_face_x = (local_sphere_center.x.abs() - obb.half_extents.x).abs() < tolerance;
                    let on_face_y = (local_sphere_center.y.abs() - obb.half_extents.y).abs() < tolerance;
                    let on_face_z = (local_sphere_center.z.abs() - obb.half_extents.z).abs() < tolerance;

                    let faces_count = on_face_x as u8 + on_face_y as u8 + on_face_z as u8;

                    // additional contact points for edge/corner
                    if faces_count >= 2 && dist > 1e-6 {
                        let tangent1 = if normal.x.abs() < 0.9 {
                            Vector::new_vec3(1.0, 0.0, 0.0).cross(&normal).normalize_3d()
                        } else {
                            Vector::new_vec3(0.0, 1.0, 0.0).cross(&normal).normalize_3d()
                        };
                        let tangent2 = normal.cross(&tangent1).normalize_3d();

                        // contact points in a circle around the main contact
                        let num_additional = if faces_count == 3 { 3 } else { 2 }; // corner/edge
                        for i in 1..=num_additional {
                            let angle = (i as f32) * std::f32::consts::PI * 2.0 / (num_additional + 1) as f32;
                            let offset = tangent1 * (angle.cos() * 0.01) + tangent2 * (angle.sin() * 0.01);

                            contact_points.push(ContactPoint {
                                point_on_a: point_on_obb + offset,
                                point_on_b: point_on_sphere + offset,
                                penetration,
                            });
                        }
                    }

                    Some(ContactInformation {
                        contact_points,
                        normal,
                        time_of_impact: 0.0
                    })
                }
                Hitbox::SPHERE(a) => {
                    let p_a = a.center.rotate_by_quat(&self.orientation) + self.position;
                    let p_b = sphere.center.rotate_by_quat(&other.orientation) + other.position;

                    if let Some((mut point, time_of_impact)) = {
                        let relative_vel = self.velocity - other.velocity;

                        let end_pt_a = p_a + relative_vel * dt;
                        let dir = end_pt_a - p_a;

                        let mut t0 = 0.0;
                        let mut t1 = 0.0;
                        if dir.magnitude_3d() < 0.001 {
                            let ab = p_b - p_a;
                            let radius = a.radius + sphere.radius + 0.001;
                            if ab.magnitude_3d() > radius {
                                return None
                            }
                        } else if let Some((i_t0, i_t1)) = Sphere::ray_sphere(&p_a, &dir, &p_b, a.radius + sphere.radius) {
                            t0 = i_t0;
                            t1 = i_t1;
                        } else {
                            return None
                        }

                        // convert 0-1 to 0-dt
                        t0 *= dt;
                        t1 *= dt;

                        // collision happened in past
                        if t1 < 0.0 { return None }

                        let toi = if t0 < 0.0 { 0.0 } else { t0 };

                        // collision happens past dt
                        if toi > dt { return None }

                        let new_pos_a = p_a + self.velocity * toi;
                        let new_pos_b = p_b + other.velocity * toi;

                        let ab = (new_pos_b - new_pos_a).normalize_3d();

                        Some((ContactPoint {
                            point_on_a: new_pos_a + ab * a.radius,
                            point_on_b: new_pos_b - ab * sphere.radius,

                            penetration: 0.0,
                        }, toi))
                    } {
                        // there will be a collision
                        self.update(time_of_impact);
                        other.update(time_of_impact);

                        let normal = (self.position - other.position).normalize_3d();

                        self.update(-time_of_impact);
                        other.update(-time_of_impact);

                        let ab = other.position - self.position;
                        let r = ab.magnitude_3d() - (a.radius + sphere.radius);

                        Some(ContactInformation {
                            contact_points: vec![point],
                            time_of_impact,
                            normal,
                        })
                    } else {
                        None
                    }
                }
                _ => { None }
            }
        }
        None
    }
    pub fn intersects_obb(&mut self, other: &mut RigidBody, dt: f32) -> Option<ContactInformation> {
        if let Hitbox::OBB(obb) = other.hitbox {
            return match self.hitbox {
                Hitbox::OBB(this_obb) => {
                    let (a, b) = (this_obb, obb);

                    let a_center = a.center.rotate_by_quat(&self.orientation) + self.position;
                    let b_center = b.center.rotate_by_quat(&other.orientation) + other.position;

                    let a_quat = self.orientation;
                    let b_quat = other.orientation;

                    let a_axes = [
                        Vector::new_vec3(1.0, 0.0, 0.0).rotate_by_quat(&a_quat),
                        Vector::new_vec3(0.0, 1.0, 0.0).rotate_by_quat(&a_quat),
                        Vector::new_vec3(0.0, 0.0, 1.0).rotate_by_quat(&a_quat),
                    ];
                    let b_axes = [
                        Vector::new_vec3(1.0, 0.0, 0.0).rotate_by_quat(&b_quat),
                        Vector::new_vec3(0.0, 1.0, 0.0).rotate_by_quat(&b_quat),
                        Vector::new_vec3(0.0, 0.0, 1.0).rotate_by_quat(&b_quat),
                    ];

                    let t = b_center - a_center;

                    let mut min_penetration = f32::MAX;
                    let mut collision_normal = Vector::new_empty();
                    let mut best_axis_type = AxisType::FaceA(0);
                    for i in 0..3 {
                        { // a_normals
                            let axis = a_axes[i];
                            let penetration = Self::test_axis(&axis, &t, &a.half_extents, &b.half_extents, &a_axes, &b_axes, true);
                            if penetration < 0.0 {
                                return None
                            }
                            if penetration < min_penetration {
                                min_penetration = penetration;
                                collision_normal = axis;
                                best_axis_type = AxisType::FaceA(i);
                            }
                        }
                        { // b_normals
                            let axis = b_axes[i];
                            let penetration = Self::test_axis(&axis, &t, &a.half_extents, &b.half_extents, &a_axes, &b_axes, true);
                            if penetration < 0.0 {
                                return None
                            }
                            if penetration < min_penetration {
                                min_penetration = penetration;
                                collision_normal = axis;
                                best_axis_type = AxisType::FaceB(i);
                            }
                        }
                        for j in 0..3 { // edge-edge cross-products
                            let axis = a_axes[i].cross(&b_axes[j]);
                            let axis_length = axis.magnitude_3d();

                            if axis_length < 1e-6 {
                                continue;
                            }

                            let axis_normalized = axis / axis_length;
                            let penetration = Self::test_axis_cross(&axis_normalized, &t, &a.half_extents, &b.half_extents, &a_axes, &b_axes);

                            if penetration < 0.0 {
                                return None;
                            }

                            if penetration < min_penetration {
                                min_penetration = penetration;
                                collision_normal = axis_normalized;
                                best_axis_type = AxisType::Edge(i, j);
                            }
                        }
                    }

                    if collision_normal.dot(&t) < 0.0 {
                        collision_normal = -collision_normal;
                    }

                    Some(ContactInformation {
                        contact_points: vec![ContactPoint {
                            point_on_a: a_center + collision_normal * (1.0 - min_penetration),
                            point_on_b: b_center - collision_normal * (1.0 - min_penetration),
                            penetration: min_penetration
                        }],
                        normal: collision_normal,
                        time_of_impact: 0.0,
                    })
                }
                Hitbox::SPHERE(_) => {
                    if let Some(contact) = other.intersects_sphere(self, dt) {
                        Some(contact.flip())
                    } else { None }
                }
                _ => None
            }
        }
        None
    }

    fn test_axis(
        axis: &Vector,
        t: &Vector,
        half_a: &Vector,
        half_b: &Vector,
        axes_a: &[Vector; 3],
        axes_b: &[Vector; 3],
        is_a: bool,
    ) -> f32 {
        let ra = if is_a {
            half_a.x * axes_a[0].dot3(axis).abs() +
                half_a.y * axes_a[1].dot3(axis).abs() +
                half_a.z * axes_a[2].dot3(axis).abs()
        } else {
            half_a.x * axes_a[0].dot3(axis).abs() +
                half_a.y * axes_a[1].dot3(axis).abs() +
                half_a.z * axes_a[2].dot3(axis).abs()
        };
        let rb = half_b.x * axes_b[0].dot3(axis).abs() +
            half_b.y * axes_b[1].dot3(axis).abs() +
            half_b.z * axes_b[2].dot3(axis).abs();
        let distance = t.dot3(axis).abs();
        ra + rb - distance
    }
    fn test_axis_cross(
        axis: &Vector,
        t: &Vector,
        half_a: &Vector,
        half_b: &Vector,
        axes_a: &[Vector; 3],
        axes_b: &[Vector; 3],
    ) -> f32 {
        let ra = half_a.x * axes_a[0].dot3(axis).abs() +
            half_a.y * axes_a[1].dot3(axis).abs() +
            half_a.z * axes_a[2].dot3(axis).abs();

        let rb = half_b.x * axes_b[0].dot3(axis).abs() +
            half_b.y * axes_b[1].dot3(axis).abs() +
            half_b.z * axes_b[2].dot3(axis).abs();

        let distance = t.dot3(axis).abs();

        ra + rb - distance
    }
}
impl Default for RigidBody {
    fn default() -> Self {
        Self {
            owned_by_player: false,
            coupled_with_scene_object: None,
            hitbox: Hitbox::OBB(BoundingBox {
                center: Vector::new_vec(0.0),
                half_extents: Vector::new_vec(1.0),
            }),
            is_static: true,
            restitution_coefficient: 0.5,
            friction_coefficient: 0.5,
            mass: 999.0,
            inv_mass: 0.0,
            force: Default::default(),
            torque: Default::default(),
            position: Default::default(),
            velocity: Default::default(),
            orientation: Vector::new_empty_quat(),
            angular_velocity: Default::default(),
            center_of_mass: Vector::new_empty_quat(),
            inertia_tensor: Matrix::new(),
            inv_inertia_tensor: Matrix::new(),
        }
    }
}