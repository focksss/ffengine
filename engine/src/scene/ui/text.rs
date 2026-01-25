use crate::render::text::text_render::TextInformation;
use crate::math::Vector;

pub struct Text {
    pub text_information: Option<TextInformation>,
    pub(crate) font_index: usize,
    pub(crate) color: Vector,
}
impl Text {
    fn destroy(&self) {
        if let Some(text_information) = &self.text_information {
            text_information.destroy();
        }
    }
}