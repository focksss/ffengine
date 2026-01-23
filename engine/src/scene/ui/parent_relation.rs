use crate::scene::ui::{AnchorPoint, DockMode, Offset};

#[derive(Clone, Debug)]
pub enum ParentRelation {
    Docking(DockMode),
    Independent {
        relative: bool,
        anchor: AnchorPoint,
        offset_x: Offset,
        offset_y: Offset,
    }
}