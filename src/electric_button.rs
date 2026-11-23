use crate::electric_button_renderer::{
    ButtonResources, ElectricButtonCallback, ElectricButtonUniforms,
};
use crate::electric_button_state::ElectricButtonState;
use crate::electric_button_style::ElectricButtonStyle;
use egui::{Color32, Rect, Ui, Vec2};
use std::sync::{Arc, Mutex};

/// A clickable button widget with GPU-rendered electric border.
///
/// The widget owns a GPU-resource handle shared across frames, so the
/// uniform buffer and bind group are allocated once instead of per frame.
/// Buttons should be created once and reused across frames (the demo app
/// stores them on the app struct).
pub struct ElectricButton {
    text: String,
    desired_size: Vec2,
    style: ElectricButtonStyle,
    enabled: Option<bool>,
    resource: Arc<Mutex<Option<ButtonResources>>>,
}

impl Default for ElectricButton {
    fn default() -> Self {
        Self {
            text: String::new(),
            desired_size: Vec2::ZERO,
            style: ElectricButtonStyle::default(),
            enabled: None,
            resource: Arc::new(Mutex::new(None)),
        }
    }
}

impl ElectricButton {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }

    pub fn desired_size(mut self, size: Vec2) -> Self {
        self.desired_size = size;
        self
    }

    pub fn style(mut self, style: ElectricButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    pub fn show(
        &self,
        ui: &mut Ui,
        global_time: f32,
    ) -> (
        egui::Response,
        Rect,
        ElectricButtonState,
        ElectricButtonStyle,
    ) {
        let enabled = self.enabled.unwrap_or(true);
        let response = if enabled {
            self.show_impl(ui, global_time)
        } else {
            ui.add_enabled_ui(false, |ui| self.show_impl(ui, global_time))
                .inner
        };
        let body_rect = response.rect;
        let state = ElectricButton::determine_state(&response);
        (response, body_rect, state, self.style.clone())
    }

    fn show_impl(&self, ui: &mut Ui, global_time: f32) -> egui::Response {
        let style = &self.style;

        // Measure text.
        let font_id = egui::TextStyle::Body.resolve(ui.style());
        let galley = ui
            .painter()
            .layout_no_wrap(self.text.clone(), font_id, Color32::PLACEHOLDER);
        let text_w = galley.rect.width();
        let text_h = galley.rect.height();

        // Size.
        let padding = Vec2::new(24.0, 12.0);
        let (bw, bh) = if self.desired_size.x > 0.0 || self.desired_size.y > 0.0 {
            (
                fmax(self.desired_size.x, text_w + padding.x),
                fmax(self.desired_size.y, text_h + padding.y),
            )
        } else {
            (text_w + padding.x, text_h + padding.y)
        };

        let response = ui.allocate_response(Vec2::new(bw, bh), egui::Sense::click());

        // Draw text label.
        let label_color = if response.enabled() {
            ui.style().visuals.text_color()
        } else {
            let tc = ui.style().visuals.text_color();
            Color32::from_rgba_unmultiplied(tc.r(), tc.g(), tc.b(), 150)
        };
        let label_top_left = response.rect.center() - Vec2::new(text_w / 2.0, text_h / 2.0);
        ui.painter().galley(label_top_left, galley, label_color);
        // Expand the callback rect to cover the glow ring around the body.
        // Only the clip region is expanded — the hit target stays response.rect
        // (Shape::Callback does not affect hit testing).
        let expanded = response
            .rect
            .expand(style.glow_padding() / ui.ctx().pixels_per_point());

        // Build GPU uniforms and callback.
        let state = ElectricButton::determine_state(&response);
        let uniforms = ElectricButtonUniforms::new(
            response.rect,
            expanded,
            ui.ctx().pixels_per_point(),
            style.corner_radius,
            style.border_thickness,
            style.color,
            style.glow_spread,
            global_time,
            !matches!(state, ElectricButtonState::Disabled),
            state.intensity(),
        );
        let callback = ElectricButtonCallback::new(uniforms, self.resource.clone());
        let paint_callback = egui_wgpu::Callback::new_paint_callback(expanded, callback);
        ui.painter()
            .add(egui::epaint::Shape::Callback(paint_callback));

        response
    }

    fn determine_state(response: &egui::Response) -> ElectricButtonState {
        ElectricButton::state_from_signals(
            response.enabled(),
            response.has_focus(),
            response.is_pointer_button_down_on(),
            response.hovered(),
        )
    }

    /// Pure state derivation, extracted so it can be unit-tested without a
    /// live egui context. Priority: Disabled > Focused > Pressed > Hovered.
    fn state_from_signals(
        enabled: bool,
        focused: bool,
        pressed: bool,
        hovered: bool,
    ) -> ElectricButtonState {
        if !enabled {
            return ElectricButtonState::Disabled;
        }
        if focused {
            return ElectricButtonState::Focused;
        }
        if pressed {
            return ElectricButtonState::Pressed;
        }
        if hovered {
            return ElectricButtonState::Hovered;
        }
        ElectricButtonState::Idle
    }
}

fn fmax(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_beats_all_other_signals() {
        // Priority: Disabled must win even when other signals are true.
        assert_eq!(
            ElectricButton::state_from_signals(false, true, true, true),
            ElectricButtonState::Disabled
        );
        assert_eq!(
            ElectricButton::state_from_signals(false, false, false, false),
            ElectricButtonState::Disabled
        );
        assert_eq!(
            ElectricButton::state_from_signals(false, false, true, true),
            ElectricButtonState::Disabled
        );
    }

    #[test]
    fn focused_beats_pressed_and_hovered() {
        assert_eq!(
            ElectricButton::state_from_signals(true, true, true, true),
            ElectricButtonState::Focused
        );
        assert_eq!(
            ElectricButton::state_from_signals(true, true, true, false),
            ElectricButtonState::Focused
        );
        assert_eq!(
            ElectricButton::state_from_signals(true, true, false, true),
            ElectricButtonState::Focused
        );
    }

    #[test]
    fn pressed_beats_hovered() {
        assert_eq!(
            ElectricButton::state_from_signals(true, false, true, true),
            ElectricButtonState::Pressed
        );
        assert_eq!(
            ElectricButton::state_from_signals(true, false, true, false),
            ElectricButtonState::Pressed
        );
    }

    #[test]
    fn hovered_beats_idle() {
        assert_eq!(
            ElectricButton::state_from_signals(true, false, false, true),
            ElectricButtonState::Hovered
        );
    }

    #[test]
    fn no_signals_means_idle() {
        assert_eq!(
            ElectricButton::state_from_signals(true, false, false, false),
            ElectricButtonState::Idle
        );
    }

    #[test]
    fn intensity_ordering_is_distinct_and_disabled_is_zero() {
        assert_eq!(ElectricButtonState::Disabled.intensity(), 0.0);
        assert!(ElectricButtonState::Hovered.intensity() > ElectricButtonState::Idle.intensity());
        assert!(
            ElectricButtonState::Pressed.intensity() > ElectricButtonState::Hovered.intensity()
        );
        assert!(ElectricButtonState::Focused.intensity() > ElectricButtonState::Idle.intensity());
        assert!(ElectricButtonState::Idle.intensity() > 0.0);
    }
}
