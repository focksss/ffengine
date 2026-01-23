use crate::scene::ui::{Alignment, PackingMode, Padding, StackDirection};

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