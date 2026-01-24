use ash::vk;
use crate::math::Vector;

pub struct Image {
    pub(crate) index: usize,
    uri: String,
    alpha_threshold: f32,
    pub(crate) additive_tint: Vector,
    pub(crate) multiplicative_tint: Vector,
    pub(crate) corner_radius: f32,
    pub(crate) aspect_ratio: Option<f32>,

    image_view: vk::ImageView,
    sampler: vk::Sampler,
    image: vk::Image,
    memory: vk::DeviceMemory,
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