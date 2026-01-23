use crate::gui::text::text_render::TextInformation;
use crate::math::Vector;

pub struct Text {
    text_information: Option<TextInformation>,
    font_index: usize,
    color: Vector,
}
impl Text {
    fn destroy(&self) {
        if let Some(text_information) = &self.text_information {
            text_information.destroy();
        }
    }
}