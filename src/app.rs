use crate::electric_button::ElectricButton;
use crate::electric_button_state::ElectricButtonState;
use crate::electric_button_style::ElectricButtonStyle;
use egui::{Color32, RichText, Vec2};

/// Demo app showing multiple ElectricButton instances with
/// GPU-rendered lightning animation.
pub struct ElectricDemoApp {
    main_click_count: u32,
    main_state: ElectricButtonState,
    reduced_motion: bool,
    speed: f32,
    intensity: f32,
    /// Global animation time (updated each frame).
    time: f32,
}

impl ElectricDemoApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            main_click_count: 0,
            main_state: ElectricButtonState::Idle,
            reduced_motion: false,
            speed: 1.0,
            intensity: 1.0,
            time: 0.0,
        }
    }
}

impl eframe::App for ElectricDemoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Update global time.
        self.time += 0.016 * self.speed;

        // Request continuous repaints so the animation runs.
        ui.ctx().request_repaint();

        // Top bar with settings.
        egui::Panel::top("settings_panel").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("⚡ Electric Button Demo").size(16.0).strong());
                ui.add_space(16.0);
                ui.checkbox(&mut self.reduced_motion, "Reduced motion");
                ui.add_space(8.0);
                ui.label("Speed:");
                ui.add(egui::Slider::new(&mut self.speed, 0.1..=3.0).text("Speed"));
                ui.add_space(8.0);
                ui.label("Intensity:");
                ui.add(egui::Slider::new(&mut self.intensity, 0.1..=2.0).text("Intensity"));
                ui.separator();
                ui.label(format!("Clicks: {}", self.main_click_count));
                ui.label(format!("State: {:?}", self.main_state));
            });
        });

        // Central content area.
        egui::CentralPanel::default().show(ui, |ui| {
            ui.with_layout(
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.add_space(32.0);

                    // Main interactive button.
                    let style = ElectricButtonStyle {
                        reduced_motion: self.reduced_motion,
                        speed: self.speed,
                        intensity: self.intensity,
                        ..Default::default()
                    };

                    let (response, _body_rect, state, _style) = ElectricButton::new("")
                        .desired_size(Vec2::new(220.0, 60.0))
                        .style(style)
                        .show(ui, self.time);

                    if state != self.main_state {
                        self.main_state = state;
                    }

                    if response.clicked() {
                        self.main_click_count += 1;
                    }

                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(format!(
                            "Clicks: {} | State: {:?}",
                            self.main_click_count, self.main_state
                        ))
                        .size(14.0),
                    );

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
                            .show(ui, self.time);

                        let style = ElectricButtonStyle {
                            color: Color32::from_rgb(255, 100, 100),
                            corner_radius: 24.0,
                            ..Default::default()
                        };
                        ElectricButton::new("Violet")
                            .desired_size(Vec2::new(120.0, 45.0))
                            .style(style)
                            .show(ui, self.time);

                        let style = ElectricButtonStyle {
                            corner_radius: 4.0,
                            ..Default::default()
                        };
                        ElectricButton::new("Sharp")
                            .desired_size(Vec2::new(120.0, 45.0))
                            .style(style)
                            .show(ui, self.time);
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
                            .show(ui, self.time);
                    });

                    ui.add_space(16.0);
                },
            );
        });
    }
}
