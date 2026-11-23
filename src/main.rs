use electric_button::app::ElectricDemoApp;

fn main() {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 480.0]),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native(
        "Electric Button",
        native_options,
        Box::new(|cc| Ok(Box::new(ElectricDemoApp::new(cc)))),
    ) {
        eprintln!("electric-button exited with error: {e}");
    }
}
