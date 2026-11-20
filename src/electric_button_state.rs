#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElectricButtonState {
    Idle,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}

impl ElectricButtonState {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Hovered | Self::Pressed | Self::Focused)
    }

    pub fn requires_animation(&self) -> bool {
        matches!(self, Self::Hovered | Self::Pressed | Self::Focused)
    }
}
