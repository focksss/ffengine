use crate::math::matrix::Matrix;
use crate::math::Vector;

pub struct Transform {
    is_identity: bool,
    pub owner: usize,

    pub local_translation: Vector,
    pub local_rotation: Vector,
    pub local_scale: Vector,

    pub local: Matrix,

    pub world_translation: Vector,
    pub world_rotation: Vector,
    pub world_scale: Vector,

    pub world: Matrix
}
impl Transform {
    pub(crate) fn update_local_matrix(&mut self) {
        let rotate = Matrix::new_rotate_quaternion_vec4(&self.local_rotation);
        let scale = Matrix::new_scale_vec3(&self.local_scale);
        let translate = Matrix::new_translation_vec3(&self.local_translation);

        self.local = translate * rotate * scale;
    }
    pub(crate) fn update_world_matrix(&mut self, parent_transform: &Transform, animated_transform: Option<&Transform>) {
        if let Some(animated) = animated_transform {
            self.world_scale = parent_transform.world_scale * animated.local_scale;
            self.world_rotation = parent_transform.world_rotation.combine(&animated.local_rotation);
            self.world_translation = parent_transform.world_translation +
                (animated.local_translation * parent_transform.world_scale)
                    .rotate_by_quat(&parent_transform.world_rotation);
            self.world = parent_transform.world * animated.local;
        } else {
            self.world_scale = parent_transform.world_scale * self.local_scale;
            self.world_rotation = parent_transform.world_rotation.combine(&self.local_rotation);
            self.world_translation = parent_transform.world_translation +
                (self.local_translation * parent_transform.world_scale)
                    .rotate_by_quat(&parent_transform.world_rotation);
            self.world = parent_transform.world * self.local;
        }
    }

    // fn local_to_world_position(&self, position: Vector) -> Vector {
    //     self.world_translation + (position * self.world_scale).rotate_by_quat(&self.world_rotation)
    // }
    pub(crate) fn world_to_local_position(&self, position: Vector, parent: &Transform) -> Vector {
        ((position - parent.world_translation) / parent.world_scale).rotate_by_quat(&parent.world_rotation.inverse_quat())
    }

    // fn local_to_world_rotation(&self, rotation: Vector) -> Vector {
    //     self.world_rotation * rotation
    // }
    pub(crate) fn world_to_local_rotation(&self, rotation: Vector, parent: &Transform) -> Vector {
        parent.world_rotation.inverse_quat().combine(&rotation)
    }
}
impl Default for Transform {
    fn default() -> Self {
        Transform {
            owner: 0,
            is_identity: true,

            local_translation: Vector::new(),
            local_rotation: Vector::new4(0.0, 0.0, 0.0, 1.0),
            local_scale: Vector::fill(1.0),

            local: Matrix::new(),

            world_translation: Vector::new(),
            world_rotation: Vector::new4(0.0, 0.0, 0.0, 1.0),
            world_scale: Vector::fill(1.0),

            world: Matrix::new(),
        }
    }
}