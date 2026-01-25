use crate::scripting::lua_engine::Field;

#[derive(Clone)]
pub struct InteractionInformation {
    pub method: &'static str,
    pub script: usize,
    pub args: Vec<Field>
}
#[derive(Clone)]
pub struct UiInteractableInformation {
    pub was_initially_left_pressed: bool,
    pub was_initially_right_pressed: bool,

    pub passive_actions: Vec<InteractionInformation>,
    pub hover_actions: Vec<InteractionInformation>,
    pub unhover_actions: Vec<InteractionInformation>,
    pub left_up_actions: Vec<InteractionInformation>,
    pub left_down_actions: Vec<InteractionInformation>,
    pub left_hold_actions: Vec<InteractionInformation>,
    pub right_up_actions: Vec<InteractionInformation>,
    pub right_down_actions: Vec<InteractionInformation>,
    pub right_hold_actions: Vec<InteractionInformation>,
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