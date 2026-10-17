use eframe::egui::{self, Align, Color32, Layout, RichText};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Renders the application footer with version info and controls
pub fn render_footer(ui: &mut egui::Ui, show_options: &mut bool, device_label: &str) {
    // Add vertical padding for better spacing
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.add(
            egui::Label::new(RichText::new(device_label).small().color(Color32::from_gray(150)))
                .selectable(false),
        );
        ui.separator();
        render_version_info(ui);

        // Options button on the right side
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button("⚙ Options").clicked() {
                *show_options = !*show_options;
            }
        });
    });

    // Add bottom padding for balance
    ui.add_space(8.0);
}

/// Renders the application version information
fn render_version_info(ui: &mut egui::Ui) {
    let text = format!("{} • R-Helper continuation from CPUnity", APP_VERSION);
    ui.add(
        egui::Label::new(RichText::new(text).small().color(Color32::from_gray(150)))
            .selectable(false),
    );
}
