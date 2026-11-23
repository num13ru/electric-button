use crate::electric_button::ElectricButton;
use crate::electric_button_state::ElectricButtonState;
use crate::electric_button_style::ElectricButtonStyle;
use egui::{Color32, RichText, Vec2};

/// Maximum elapsed time (seconds) applied to the animation per frame.
/// Above this, uncontrolled jumps (e.g. after suspension or long stalls)
/// are avoided; the animation effectively slows to a crawl instead.
const MAX_FRAME_DT: f32 = 0.1;

/// Demo app showing multiple ElectricButton instances with
/// GPU-rendered lightning animation.
pub struct ElectricDemoApp {
    main_click_count: u32,
    main_state: ElectricButtonState,
    speed: f32,
    /// Global animation time in seconds (updated each frame).
    time: f64,
    /// Cached status label; rebuilt only when click count or state changes.
    status_text: egui::RichText,
}

impl ElectricDemoApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            main_click_count: 0,
            main_state: ElectricButtonState::Idle,
            speed: 1.0,
            time: 0.0,
            status_text: egui::RichText::new("").size(14.0),
        }
    }

    /// Advance the global animation clock by the real elapsed frame time,
    /// scaled by the Speed slider. `dt` must already be clamped by the caller.
    fn advance_time(&mut self, dt: f32) {
        let dt = dt.min(MAX_FRAME_DT);
        self.time += f64::from(dt) * f64::from(self.speed);
    }

    /// Time to feed the GPU shader. The CPU keeps a full-precision f64
    /// accumulator; the shader only needs a bounded f32 value.
    fn shader_time(&self) -> f32 {
        self.time as f32
    }
}

impl eframe::App for ElectricDemoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Advance animation time using the real elapsed frame time
        // (egui's stable_dt), clamped to avoid jumps after suspension.
        let dt = ui.ctx().input(|i| i.stable_dt).min(MAX_FRAME_DT);
        self.advance_time(dt);

        // Request continuous repaints so the animation runs — but only while
        // the animation is actually advancing (speed > 0).
        if self.speed > 0.0 {
            ui.ctx().request_repaint();
        }

        // Top bar with settings.
        egui::Panel::top("settings_panel").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Electric Button Demo").size(16.0).strong());
                ui.add_space(16.0);
                ui.label("Speed:");
                ui.add(egui::Slider::new(&mut self.speed, 0.1..=3.0).text("Speed"));
                ui.add_space(8.0);
                ui.separator();
            });
        });

        // Central content area.
        egui::CentralPanel::default().show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(32.0);

                // Main interactive button.
                let style = ElectricButtonStyle::default();

                let (response, _body_rect, state, _style) = ElectricButton::new("Electric Button")
                    .desired_size(Vec2::new(220.0, 60.0))
                    .style(style)
                    .show(ui, self.shader_time());

                if state != self.main_state {
                    self.main_state = state;
                }

                if response.clicked() {
                    self.main_click_count += 1;
                }

                let new_status = egui::RichText::new(format!(
                    "Clicks: {} | State: {:?}",
                    self.main_click_count, self.main_state
                ))
                .size(14.0);
                if self.status_text != new_status {
                    self.status_text = new_status;
                }
                ui.add_space(16.0);
                ui.label(self.status_text.clone());

                ui.add_space(32.0);
                ui.separator();
                ui.add_space(16.0);

                // Multi-instance buttons.
                ui.horizontal(|ui| {
                    ui.label("Multi-instance:");
                });
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    let style = ElectricButtonStyle {
                        color: Color32::from_rgb(255, 165, 0),
                        ..Default::default()
                    };
                    ElectricButton::new("Amber")
                        .desired_size(Vec2::new(120.0, 45.0))
                        .style(style)
                        .show(ui, self.shader_time());

                    let style = ElectricButtonStyle {
                        color: Color32::from_rgb(255, 100, 100),
                        corner_radius: 24.0,
                        ..Default::default()
                    };
                    ElectricButton::new("Violet")
                        .desired_size(Vec2::new(120.0, 45.0))
                        .style(style)
                        .show(ui, self.shader_time());

                    let style = ElectricButtonStyle {
                        corner_radius: 4.0,
                        ..Default::default()
                    };
                    ElectricButton::new("Sharp")
                        .desired_size(Vec2::new(120.0, 45.0))
                        .style(style)
                        .show(ui, self.shader_time());
                });

                ui.add_space(32.0);
                ui.separator();
                ui.add_space(16.0);

                // Disabled button.
                ui.horizontal(|ui| {
                    ui.label("Disabled:");
                    let style = ElectricButtonStyle {
                        corner_radius: 8.0,
                        ..Default::default()
                    };
                    ElectricButton::new("Disabled")
                        .desired_size(Vec2::new(120.0, 45.0))
                        .style(style)
                        .enabled(false)
                        .show(ui, self.shader_time());
                });

                ui.add_space(16.0);
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> ElectricDemoApp {
        ElectricDemoApp {
            main_click_count: 0,
            main_state: ElectricButtonState::Idle,
            speed: 1.0,
            time: 0.0,
            status_text: egui::RichText::new("").size(14.0),
        }
    }

    #[test]
    fn time_advances_with_real_elapsed_dt_not_constant() {
        let mut app2 = app();
        app2.advance_time(0.016);
        assert!((app2.time - 0.016).abs() < 1e-9);

        let mut app2 = app();
        app2.advance_time(0.05);
        assert!((app2.time - 0.05).abs() < 1e-9);
    }

    #[test]
    fn same_elapsed_duration_clocks_up_identically_at_30_60_120_fps() {
        for fps in [30u32, 60, 120] {
            let mut app = app();
            let dt = 1.0 / fps as f32;
            // Simulate 2 seconds of wall time.
            let frames = fps * 2;
            for _ in 0..frames {
                app.advance_time(dt);
            }
            let expected = 2.0; // seconds
            assert!(
                (app.time - expected).abs() < 0.05,
                "time drifted at {fps} FPS: {}",
                app.time
            );
        }
    }

    #[test]
    fn suspension_dt_is_clamped_to_max_frame_dt() {
        let mut app = app();
        app.advance_time(10.0); // e.g. after app suspension
        assert!((app.time - MAX_FRAME_DT as f64).abs() < 1e-9);
    }

    #[test]
    fn speed_slider_scales_time_advance() {
        let mut app = app();
        app.speed = 2.5;
        app.advance_time(0.1);
        assert!((app.time - 0.25).abs() < 1e-6);
    }

    #[test]
    fn long_session_time_stays_finite() {
        let mut app = app();
        for _ in 0..20_000 {
            app.advance_time(1.0 / 60.0);
        }
        assert!(app.time.is_finite());
        assert!((app.time - 20_000.0 / 60.0).abs() < 0.5);
        assert!(app.shader_time().is_finite());
    }

    #[test]
    fn speed_zero_does_not_advance_time() {
        let mut app = app();
        app.speed = 0.0;
        app.advance_time(0.1);
        assert_eq!(app.time, 0.0);
    }
}
