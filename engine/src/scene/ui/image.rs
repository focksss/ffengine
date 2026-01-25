use ash::vk;
use crate::math::Vector;

pub struct Image {
    pub index: usize,
    pub uri: String,
    pub alpha_threshold: f32,
    pub additive_tint: Vector,
    pub multiplicative_tint: Vector,
    pub corner_radius: f32,
    pub aspect_ratio: Option<f32>,

    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
}
impl Image {
    fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_sampler(self.sampler, None);
            device.destroy_image_view(self.image_view, None);
            device.destroy_image(self.image, None);
            device.free_memory(self.memory, None);
        }
    }
}