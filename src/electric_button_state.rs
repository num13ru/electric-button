#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElectricButtonState {
    Idle,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}

impl ElectricButtonState {
    /// Brightness multiplier for the state. Disabled is 0 (the shader
    /// draws a dim gray ghost outline instead of the electric border).
    pub fn intensity(self) -> f32 {
        match self {
            ElectricButtonState::Idle => 1.0,
            ElectricButtonState::Hovered => 1.3,
            ElectricButtonState::Pressed => 1.5,
            ElectricButtonState::Focused => 1.15,
            ElectricButtonState::Disabled => 0.0,
        }
    }
}
