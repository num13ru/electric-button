#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElectricButtonState {
    Idle,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}
