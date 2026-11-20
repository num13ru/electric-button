//! Static outline renderer for the electric button.
//!
//! Phase 2: draws a rounded-rectangle border using egui's built-in
//! painter (epaint), not a custom WGSL shader. This is simpler and
//! fully integrates with egui's clipping, DPI scaling, and z-ordering.
//!
//! Phase 3 will replace this with a WGSL-based GPU shader for
//! animated lightning effects.

use crate::electric_button_state::ElectricButtonState;
use egui::epaint::RectShape;
use egui::{Color32, Rect, Shape, Stroke, Ui};

/// Draw the static electric border for a button body rect.
/// The border intensity adjusts based on the button's current state.
pub fn draw_static_border(
    ui: &mut Ui,
    body_rect: Rect,
    style: &crate::electric_button_style::ElectricButtonStyle,
    state: ElectricButtonState,
) {
    // Adjust alpha based on state.
    let effective_alpha = match state {
        ElectricButtonState::Disabled => 25,
        ElectricButtonState::Focused => 180,
        ElectricButtonState::Pressed => 200,
        ElectricButtonState::Hovered => 160,
        ElectricButtonState::Idle => 80,
    };

    // Outer glow (wide, soft, low-alpha)
    let glow_color =
        Color32::from_rgba_unmultiplied(style.color.r(), style.color.g(), style.color.b(), effective_alpha / 3);
    let glow_stroke = Stroke::new(style.glow_spread + style.border_thickness, glow_color);
    ui.painter().add(Shape::Rect(RectShape::stroke(
        body_rect,
        style.corner_radius,
        glow_stroke,
        egui::StrokeKind::Outside,
    )));

    // Main border (thin, semi-transparent)
    let border_color =
        Color32::from_rgba_unmultiplied(style.color.r(), style.color.g(), style.color.b(), effective_alpha);
    let border_stroke = Stroke::new(style.border_thickness, border_color);
    ui.painter().add(Shape::Rect(RectShape::stroke(
        body_rect,
        style.corner_radius,
        border_stroke,
        egui::StrokeKind::Outside,
    )));

    // Inner highlight (thin, bright core) - only if not disabled
    if !matches!(state, ElectricButtonState::Disabled) {
        let inner_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(200, 255, 255, effective_alpha / 2));
        let inner_rect = body_rect.shrink(style.border_thickness * 0.5);
        ui.painter().add(Shape::Rect(RectShape::stroke(
            inner_rect,
            fmax(style.corner_radius - style.border_thickness * 0.5, 0.0),
            inner_stroke,
            egui::StrokeKind::Inside,
        )));
    }
}

fn fmax(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}
