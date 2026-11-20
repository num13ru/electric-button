use crate::electric_button_state::ElectricButtonState;
use egui::{Color32, Rect};

/// Visual parameters for the electric button effect.
#[derive(Clone, Debug)]
pub struct ElectricButtonStyle {
    /// Corner radius of the rounded rectangle.
    pub corner_radius: f32,
    /// Border line thickness (in screen pixels).
    pub border_thickness: f32,
    /// Base border color.
    pub color: Color32,
    /// Glow spread radius around the border.
    pub glow_spread: f32,
    /// Animation speed multiplier.
    pub speed: f32,
    /// Intensity multiplier for the electric effect.
    pub intensity: f32,
    /// Whether reduced-motion mode is enabled.
    pub reduced_motion: bool,
    /// State-specific intensity overrides (0.0 = disabled).
    pub state: ElectricButtonStateIntensity,
}

#[derive(Clone, Debug)]
pub struct ElectricButtonStateIntensity {
    /// Intensity multiplier when button is idle.
    pub idle: f32,
    /// Intensity multiplier when hovered.
    pub hovered: f32,
    /// Intensity multiplier when pressed.
    pub pressed: f32,
    /// Intensity multiplier when focused.
    pub focused: f32,
    /// Intensity when disabled (always 0 visually).
    pub disabled: f32,
}

impl Default for ElectricButtonStyle {
    fn default() -> Self {
        Self {
            corner_radius: 8.0,
            border_thickness: 2.0,
            color: Color32::from_rgba_unmultiplied(0, 200, 255, 200),
            glow_spread: 6.0,
            speed: 1.0,
            intensity: 1.0,
            reduced_motion: false,
            state: ElectricButtonStateIntensity {
                idle: 0.3,
                hovered: 1.0,
                pressed: 1.5,
                focused: 1.0,
                disabled: 0.0,
            },
        }
    }
}

impl ElectricButtonStyle {
    /// Compute the effective intensity for a given state.
    pub fn effective_intensity(&self, state: ElectricButtonState) -> f32 {
        let base = match state {
            ElectricButtonState::Idle => self.state.idle,
            ElectricButtonState::Hovered => self.state.hovered,
            ElectricButtonState::Pressed => self.state.pressed,
            ElectricButtonState::Focused => self.state.focused,
            ElectricButtonState::Disabled => self.state.disabled,
        };
        if self.reduced_motion {
            if matches!(state, ElectricButtonState::Focused) {
                return base * 0.8;
            }
            if matches!(state, ElectricButtonState::Hovered) {
                return base * 0.5;
            }
            return base * 0.2;
        }
        base * self.intensity
    }

    /// Padding to add around the button rect for glow effects.
    pub fn glow_padding(&self) -> f32 {
        self.glow_spread + self.border_thickness
    }

    /// Compute the effect rect (body + glow padding).
    pub fn effect_rect(&self, body_rect: Rect) -> Rect {
        let pad = self.glow_padding();
        body_rect.expand(pad)
    }
}
