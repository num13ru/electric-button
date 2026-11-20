use crate::electric_button_renderer::draw_static_border;
use crate::electric_button_state::ElectricButtonState;
use crate::electric_button_style::ElectricButtonStyle;
use egui::{Color32, Rect, Ui, Vec2};

/// A clickable button widget that reserves layout space and returns
/// an egui Response with standard hover/press semantics.
///
/// The GPU electric effect is rendered separately behind the label.
/// The interaction rect, visual body rect, and effect rect are
/// kept distinct so glow never enlarges the clickable area.
#[derive(Clone, Debug, Default)]
pub struct ElectricButton {
    /// Button label text.
    text: String,
    /// Desired size of the button (Zero = auto-size based on label).
    desired_size: Vec2,
    /// Visual style overrides.
    style: ElectricButtonStyle,
}

impl ElectricButton {
    /// Create a new ElectricButton with the given label.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }

    /// Set the desired button size.
    pub fn desired_size(mut self, size: Vec2) -> Self {
        self.desired_size = size;
        self
    }

    /// Set the style.
    pub fn style(mut self, style: ElectricButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// Show the button in a UI. Returns the egui Response and the
    /// computed body rect for the renderer to draw behind it.
    pub fn show(
        self,
        ui: &mut Ui,
    ) -> (egui::Response, Rect, ElectricButtonState, ElectricButtonStyle) {
        let button = self.clone();
        let (response, body_rect) = button.show_impl(ui);
        let state = Self::determine_state(&response);
        let style = self.style;
        (response, body_rect, state, style)
    }

    fn show_impl(mut self, ui: &mut Ui) -> (egui::Response, Rect) {
        let text = std::mem::take(&mut self.text);
        let style = self.style;

        // Measure text size.
        let font_id = egui::TextStyle::Body.resolve(&ui.style());
        let galley = ui
            .painter()
            .layout_no_wrap(text.clone(), font_id, Color32::PLACEHOLDER);
        let text_w = galley.rect.width();
        let text_h = galley.rect.height();

        // Body size: at least the text size + padding, or the requested size.
        let padding = Vec2::new(24.0, 12.0);
        let (body_width, body_height) =
            if self.desired_size.x > 0.0 || self.desired_size.y > 0.0 {
                (
                    fmax(self.desired_size.x, text_w + padding.x),
                    fmax(self.desired_size.y, text_h + padding.y),
                )
            } else {
                (text_w + padding.x, text_h + padding.y)
            };

        let body_size = Vec2::new(body_width, body_height);

        // Get response (clickable area).
        let response = ui.allocate_response(body_size, egui::Sense::click());

        // Compute the body rect centered in the allocated space.
        let body_rect =
            Rect::from_center_size(response.rect.center(), Vec2::new(body_width, body_height));

        // Draw the electric border using the renderer (Phase 2).
        draw_static_border(ui, body_rect, &style, ElectricButton::determine_state(&response));

        // Draw the text label centered in the body rect.
        if !text.is_empty() {
            let label_color = if response.enabled() {
                Color32::LIGHT_GRAY
            } else {
                Color32::DARK_GRAY
            };
            ui.painter().text(
                body_rect.center(),
                egui::Align2::CENTER_CENTER,
                text.clone(),
                egui::FontId::proportional(14.0),
                label_color,
            );
        }

        (response, body_rect)
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
    if a > b { a } else { b }
}
