pub mod layout;
pub mod interaction;
pub mod quad;
pub mod image;
pub mod texture;
pub mod text;
mod ui_layout_manager;

#[derive(Clone)]
pub enum Size {
    Absolute(f32), // pixels
    Factor(f32), // factor of parent size
    FillFactor(f32), // factor of final remaining space, after allocation of other Size types.
    Auto, // fit content,
    Copy,
}
impl Default for Size {
    fn default() -> Size {
        Size::FillFactor(1.0)
    }
}

#[derive(Clone, Debug)]
pub enum Offset {
    Pixels(f32),
    Factor(f32),
}

#[derive(Clone)]
pub enum PackingMode {
    Start, // top if vertical,
    End, // bottom if vertical
    Center,
    SpaceIncludeEdge,
    SpaceExcludeEdge,
}
impl Default for PackingMode {
    fn default() -> Self {
        PackingMode::Start
    }
}

#[derive(Clone)]
pub struct Padding {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}
impl Default for Padding {
    fn default() -> Padding {
        Padding {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
        }
    }
}

#[derive(Clone)]
pub enum StackDirection {
    Reverse,
    Normal,
    Alternating
}
impl Default for StackDirection {
    fn default() -> Self {
StackDirection::Normal
    }
}

#[derive(Clone)]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
}
impl Default for Alignment {
    fn default() -> Alignment {
        Alignment::Start
    }
}

#[derive(Clone)]
#[derive(Debug)]
pub enum AnchorPoint {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}
impl Default for AnchorPoint {
    fn default() -> AnchorPoint {
        AnchorPoint::TopLeft
    }
}

#[derive(Clone, Debug)]
pub enum DockMode {
    Left,
    Right,
    Top,
    Bottom,
}
impl Default for DockMode {
    fn default() -> DockMode {
        DockMode::Top
    }
}