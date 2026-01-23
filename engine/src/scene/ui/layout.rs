use crate::math::Vector;
use crate::scene::ui::{Alignment, AnchorPoint, DockMode, Offset, PackingMode, Padding, Size, StackDirection};

#[derive(Clone)]
pub enum Container {
    Stack {
        horizontal: bool, // else vertical
        spacing: f32,
        padding: Padding,
        packing: PackingMode,
        stack_direction: StackDirection,
        alignment: Alignment,
    },
    Dock,
}

impl Default for Container {
    fn default() -> Self {
        Container::Dock
    }
}
impl PartialEq for Container {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Container::Dock, Container::Dock) => true,
            (Container::Stack { .. }, Container::Stack { .. }) => true,
            _ => false,
        }
    }
}

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

pub struct UiNodeLayout {
    pub container: Container,
    pub parent_relation: Option<ParentRelation>,
    pub width: Size,
    pub height: Size,

    pub hidden: bool,
    pub clipping: bool, // clips to parent, recursively affects clipping enabled children

    // computed during format pass
    pub position: Vector,
    pub size: Vector,
    pub clip_min: Vector,
    pub clip_max: Vector,
}