use eframe::egui::{self, Align, Layout, RichText};
use librazer::command::{BATTERY_LIMIT_MAX, BATTERY_LIMIT_MIN};

// Battery UI actions
#[derive(Debug, Clone, PartialEq)]
pub enum BatteryAction {
    // No action
    None,
    // Toggle battery care (Battery Health Optimizer on/off)
    ToggleBatteryCare,
    // Change the charge limit (percent, 50..=80 in steps of 5)
    SetLimit(u8),
}

pub fn render_battery_section(
    ui: &mut egui::Ui,
    battery_care: &mut bool,
    limit: &mut u8,
) -> BatteryAction {
    let mut action = BatteryAction::None;

    crate::theme::card(ui, |ui| {
        ui.add(egui::Label::new(crate::theme::section_title("🔋 Battery")).selectable(false));
        ui.separator();

        ui.horizontal(|ui| {
            if ui.checkbox(battery_care, "Battery Health Optimizer").clicked() {
                action = BatteryAction::ToggleBatteryCare;
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_enabled_ui(*battery_care, |ui| {
                    egui::ComboBox::from_id_salt("battery_limit_combo")
                        .selected_text(format!("Limit {}%", *limit))
                        .show_ui(ui, |ui| {
                            let mut pct = BATTERY_LIMIT_MIN;
                            while pct <= BATTERY_LIMIT_MAX {
                                if ui.selectable_value(limit, pct, format!("{}%", pct)).changed() {
                                    action = BatteryAction::SetLimit(pct);
                                }
                                pct += 5;
                            }
                        });
                });
            });
        });

        render_battery_status(ui, *battery_care, *limit);
    });

    action
}

fn render_battery_status(ui: &mut egui::Ui, battery_care_enabled: bool, limit: u8) {
    ui.horizontal(|ui| {
        let status_text = if battery_care_enabled {
            format!("Active: charging stops at {}%", limit)
        } else {
            "Disabled: charges to 100%".to_string()
        };
        ui.add(egui::Label::new(RichText::new(status_text)).selectable(false));
    });
}
