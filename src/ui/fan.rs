use eframe::egui::{self, Align, Color32, Layout, RichText};

const MIN_RPM_FOR_COLOR: f32 = 1900.0;
const MAX_RPM_FOR_COLOR: f32 = 5000.0;
const MIN_MANUAL_RPM: u16 = 2000;
pub const MAX_MANUAL_RPM: u16 = 5500;
const RPM_STEP: f64 = 100.0;
const DARK_GREEN_MAX: u8 = 120;
const ORANGE_MAX: u8 = 100;

#[derive(Debug, Clone, PartialEq)]
pub enum FanAction {
    None,
    SetAutoMode,
    SetManualMode(u16),
    /// Manual mode with the slider pushed to its maximum.
    SetMaxManual,
    SetManualRpm(u16),
    SliderDragging(u16),
}

pub fn render_fan_section(
    ui: &mut egui::Ui,
    fan_speed: &str,
    fan_actual_rpm: Option<u16>,
    fan_set_rpm: Option<u16>,
    manual_fan_rpm: &mut u16,
    show_status_messages: bool,
    custom_mode_active: bool,
    max_fan_speed_enabled: bool,
) -> (FanAction, bool) {
    let mut action = FanAction::None;
    let mut toggle_max = max_fan_speed_enabled;

    crate::theme::card(ui, |ui| {
        let mode = if fan_speed.eq_ignore_ascii_case("manual") { "Manual" } else { "Auto" };
        let rpm = fan_actual_rpm.map_or("N/A".to_string(), |r| format!("{} RPM", r));
        let title = format!("\u{1f300} Fan Control  \u{b7}  {}  \u{b7}  {}", mode, rpm);
        crate::theme::collapsing(ui, "fan_card", crate::theme::section_title(&title), true, |ui| {
        if show_status_messages {
            let set = fan_set_rpm.map_or("Set: Auto".to_string(), |r| format!("Set: {} RPM", r));
            ui.add(egui::Label::new(RichText::new(set).small().color(Color32::LIGHT_GRAY)).selectable(false));
        }
        // Fan Mode Selection row with Max on the right
        let available_width = ui.available_width();
        ui.allocate_ui_with_layout(
            egui::Vec2::new(available_width, ui.spacing().interact_size.y),
            Layout::left_to_right(Align::Center),
            |ui| {
                // Use two columns for clean right alignment
                ui.columns(2, |cols| {
                    // Left column: Auto / Manual
                    cols[0].horizontal(|ui| {
                        let auto_selected = fan_speed.eq_ignore_ascii_case("auto");
                        if ui.selectable_label(auto_selected, "Auto").clicked() && !auto_selected {
                            action = FanAction::SetAutoMode;
                        }
                        let manual_selected = fan_speed.eq_ignore_ascii_case("manual");
                        if ui.selectable_label(manual_selected, "Manual").clicked()
                            && !manual_selected
                        {
                            action = FanAction::SetManualMode(*manual_fan_rpm);
                        }
                    });
                    // Right column: Max (toggle) - only when Custom mode AND in-app Debug are enabled
                    cols[1].with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Rightmost: one click = Manual mode + slider at maximum.
                        if ui
                            .button("Max Fan")
                            .on_hover_text("Switch to Manual and set the fan slider to maximum")
                            .clicked()
                        {
                            action = FanAction::SetMaxManual;
                        }
                        if custom_mode_active && show_status_messages {
                            let max_selected = toggle_max;
                            let response = ui.selectable_label(max_selected, "Max");
                            if response.clicked() {
                                toggle_max = !toggle_max;
                            }
                        }
                    });
                });
            },
        );

        // Manual RPM Slider (shown only in manual mode)
        if fan_speed.eq_ignore_ascii_case("manual") {
            if let Some(manual_action) = render_manual_fan_controls(ui, manual_fan_rpm) {
                action = manual_action;
            }
        }

        render_current_status(ui, fan_speed);
        });
    });

    (action, toggle_max)
}

// (Removed old separate render_fan_mode_controls; integrated directly for alignment needs)

fn render_manual_fan_controls(ui: &mut egui::Ui, manual_fan_rpm: &mut u16) -> Option<FanAction> {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new("RPM:").selectable(false));
        let fan_response = ui.add(
            egui::Slider::new(manual_fan_rpm, MIN_MANUAL_RPM..=MAX_MANUAL_RPM).step_by(RPM_STEP),
        );

        if fan_response.dragged() || fan_response.has_focus() {
            Some(FanAction::SliderDragging(*manual_fan_rpm))
        } else if fan_response.drag_stopped() || fan_response.lost_focus() {
            Some(FanAction::SetManualRpm(*manual_fan_rpm))
        } else {
            None
        }
    })
    .inner
}

fn render_current_status(ui: &mut egui::Ui, fan_speed: &str) {
    ui.add(egui::Label::new(format!("Current: {}", fan_speed)).selectable(false));
}

#[allow(dead_code)]
fn calculate_rpm_color(actual_rpm: u16) -> Color32 {
    let normalized_rpm = ((actual_rpm as f32 - MIN_RPM_FOR_COLOR)
        / (MAX_RPM_FOR_COLOR - MIN_RPM_FOR_COLOR))
        .clamp(0.0, 1.0);
    let green_component = ((1.0 - normalized_rpm) * DARK_GREEN_MAX as f32) as u8;
    let red_component = (normalized_rpm * 255.0) as u8;
    let orange_component = (normalized_rpm * 165.0) as u8;

    Color32::from_rgb(red_component, green_component, orange_component.min(ORANGE_MAX))
}
