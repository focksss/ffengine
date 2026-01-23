use crate::scripting::lua_engine::Field;

#[derive(Clone)]
pub struct InteractionInformation {
    method: String,
    script: usize,
    args: Vec<Field>
}
#[derive(Clone)]
pub struct UiInteractableInformation {
    was_initially_left_pressed: bool,
    was_initially_right_pressed: bool,

    passive_actions: Vec<crate::gui::gui::InteractionInformation>,
    pub hover_actions: Vec<crate::gui::gui::InteractionInformation>,
    unhover_actions: Vec<crate::gui::gui::InteractionInformation>,
    pub left_up_actions: Vec<crate::gui::gui::InteractionInformation>,
    pub left_down_actions: Vec<crate::gui::gui::InteractionInformation>,
    pub left_hold_actions: Vec<crate::gui::gui::InteractionInformation>,
    pub(crate) right_up_actions: Vec<crate::gui::gui::InteractionInformation>,
    right_down_actions: Vec<crate::gui::gui::InteractionInformation>,
    right_hold_actions: Vec<crate::gui::gui::InteractionInformation>,
}
impl Default for UiInteractableInformation {
    fn default() -> Self {
        Self {
            was_initially_left_pressed: false,
            was_initially_right_pressed: false,
            passive_actions: Vec::new(),
            hover_actions: Vec::new(),
            unhover_actions: Vec::new(),
            left_hold_actions: Vec::new(),
            left_up_actions: Vec::new(),
            left_down_actions: Vec::new(),
            right_hold_actions: Vec::new(),
            right_down_actions: Vec::new(),
            right_up_actions: Vec::new(),
        }
    }
}