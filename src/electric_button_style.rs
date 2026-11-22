use egui::Color32;

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
}

impl Default for ElectricButtonStyle {
    fn default() -> Self {
        Self {
            corner_radius: 8.0,
            border_thickness: 2.0,
            color: Color32::from_rgba_unmultiplied(0, 200, 255, 200),
            glow_spread: 6.0,
        }
    }
}

impl ElectricButtonStyle {
    /// Padding to add around the button rect for glow effects.
    pub fn glow_padding(&self) -> f32 {
        self.glow_spread + self.border_thickness
    }
}
