use eframe::egui::{self, Align, Color32, Layout, RichText};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Renders the application footer with version info and controls
///
/// `tokens` is the CR-Token balance and the coin picture, once the shop is open (the first token has
/// been earned); clicking it opens Options, where the shop is.
pub fn render_footer(
    ui: &mut egui::Ui,
    show_options: &mut bool,
    device_label: &str,
    tokens: Option<(u32, Option<&egui::TextureHandle>)>,
) {
    // Add vertical padding for better spacing
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        // The left side gets what the Options button and the coin leave, and its text is cut short with
        // an ellipsis if it does not fit (a wide typeface would otherwise run under the coin).
        let right = if tokens.is_some() { 175.0 } else { 115.0 };
        let left = (ui.available_width() - right).max(80.0);
        ui.allocate_ui_with_layout(egui::vec2(left, ui.spacing().interact_size.y), Layout::left_to_right(Align::Center), |ui| {
            ui.set_max_width(left);
            ui.add(
                egui::Label::new(RichText::new(device_label).small().color(Color32::from_gray(150)))
                    .truncate()
                    .selectable(false),
            );
            ui.separator();
            render_version_info(ui);
        });

        // Options button on the right side
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button("⚙ Options").clicked() {
                *show_options = !*show_options;
            }
            if let Some((balance, coin)) = tokens {
                ui.add_space(6.0);
                if crate::ui::shop::balance_chip(ui, coin, balance).clicked() {
                    *show_options = true;
                }
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
            .truncate()
            .selectable(false),
    );
}
