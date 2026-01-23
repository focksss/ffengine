use crate::math::matrix::Matrix;
use crate::scene::scene::Scene;

pub struct SkinComponent {
    pub(crate) joints: Vec<usize>, // entity indices
    pub(crate) inverse_bind_matrices: Vec<Matrix>
}
impl SkinComponent {
    pub fn update(&self, scene: &Scene, joints: &mut Vec<Matrix>) {
        for (i, joint_entity_index) in self.joints.iter().enumerate() {
            let world_transform = scene.transforms[scene.entities[*joint_entity_index].transform].world;
            joints.push(world_transform * self.inverse_bind_matrices[i]);
        }
    }
}