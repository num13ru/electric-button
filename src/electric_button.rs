use crate::electric_button_renderer::{ElectricButtonCallback, ElectricButtonUniforms};
use crate::electric_button_state::ElectricButtonState;
use crate::electric_button_style::ElectricButtonStyle;
use egui::{Color32, Rect, Ui, Vec2};

/// A clickable button widget with GPU-rendered electric border.
#[derive(Clone, Debug, Default)]
pub struct ElectricButton {
    text: String,
    desired_size: Vec2,
    style: ElectricButtonStyle,
    enabled: Option<bool>,
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
        self,
        ui: &mut Ui,
        global_time: f32,
    ) -> (egui::Response, Rect, ElectricButtonState, ElectricButtonStyle) {
        let enabled = self.enabled.unwrap_or(true);
        let button = self.clone();
        let response = if enabled {
            button.show_impl(ui, global_time)
        } else {
            ui.add_enabled_ui(false, |ui| button.show_impl(ui, global_time)).inner
        };
        let body_rect = response.rect;
        let state = ElectricButton::determine_state(&response);
        let style = self.style;
        (response, body_rect, state, style)
    }

    fn show_impl(mut self, ui: &mut Ui, global_time: f32) -> egui::Response {
        let text = std::mem::take(&mut self.text);
        let style = self.style;

        // Measure text.
        let font_id = egui::TextStyle::Body.resolve(&ui.style());
        let galley = ui
            .painter()
            .layout_no_wrap(text.clone(), font_id, Color32::PLACEHOLDER);
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
            Color32::LIGHT_GRAY
        } else {
            Color32::DARK_GRAY
        };
        ui.painter().text(
            response.rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(14.0),
            label_color,
        );
        // Expand the callback rect to cover the glow ring around the body.
        // Only the clip region is expanded — the hit target stays response.rect
        // (Shape::Callback does not affect hit testing).
        let expanded = response
            .rect
            .expand(style.glow_padding() / ui.ctx().pixels_per_point());

        // Build GPU uniforms and callback.
        let uniforms = ElectricButtonUniforms::new(
            response.rect,
            expanded,
            ui.ctx().pixels_per_point(),
            style.corner_radius,
            style.border_thickness,
            style.color,
            style.glow_spread,
            global_time,
            !matches!(
                ElectricButton::determine_state(&response),
                ElectricButtonState::Disabled
            ),
        );
        let callback = ElectricButtonCallback::new(uniforms);
        let paint_callback = egui_wgpu::Callback::new_paint_callback(expanded, callback);
        ui.painter().add(egui::epaint::Shape::Callback(paint_callback));

        response
    }

    fn determine_state(response: &egui::Response) -> ElectricButtonState {
        if !response.enabled() {
            return ElectricButtonState::Disabled;
        }
        if response.has_focus() {
            return ElectricButtonState::Focused;
        }
        if response.is_pointer_button_down_on() {
            return ElectricButtonState::Pressed;
        }
        if response.hovered() {
            return ElectricButtonState::Hovered;
        }
        ElectricButtonState::Idle
    }
}

fn fmax(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}
