use eframe::egui::{self, Align, Layout, RichText};
use librazer::cooling_pad::{
    percent_for_rpm, rpm_for_percent, PadLightingMode, MAX_RPM, MIN_RPM, RPM_STEP,
};

/// What the user did in the Cooling Pad card this frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PadAction {
    /// Fan switched to Off.
    pub fan_off: bool,
    /// Fan switched to Manual (at the current speed value).
    pub fan_manual: bool,
    /// A new fan speed was chosen while in Manual (dropdown, preset or exact slider).
    pub fan_rpm: Option<u16>,
    /// A preset slot was overwritten with the current speed.
    pub presets_saved: bool,
    /// "Toggle App Lighting Control" was switched on / off (lighting card).
    pub lighting_toggled: Option<bool>,
    /// A lighting mode button was clicked.
    pub light_mode: Option<PadLightingMode>,
    /// "Apply" next to the color button was clicked.
    pub apply_color: bool,
    /// Brightness slider released / changed (percent, 0-100).
    pub brightness: Option<u8>,
}

/// Renders the Cooling Pad fan card. Only shown while a pad is connected.
///
/// * `fan_mode` - "", "off" or "manual" ("" = never touched by the app)
/// * `presets` - the three saved fan speeds, in percent of the pad's maximum
pub fn render_cooling_pad_section(
    ui: &mut egui::Ui,
    fan_mode: &str,
    rpm: &mut u16,
    presets: &mut [u8; 3],
) -> PadAction {
    let mut action = PadAction::default();

    crate::theme::card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.add(
                egui::Label::new(crate::theme::section_title("❄ Cooling Pad")).selectable(false),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let text = match fan_mode {
                    "manual" => format!("{} RPM", *rpm),
                    "off" => "Fan off".to_string(),
                    _ => "Connected".to_string(),
                };
                ui.add(egui::Label::new(RichText::new(text).weak()).selectable(false));
            });
        });
        ui.separator();

        // ---- Fan: Manual / Off ----
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Fan:").selectable(false));
            let manual = fan_mode == "manual";
            if ui.selectable_label(manual, "Manual").clicked() && !manual {
                action.fan_manual = true;
            }
            let off = fan_mode == "off";
            if ui.selectable_label(off, "Off").clicked() && !off {
                action.fan_off = true;
            }
        });

        if fan_mode == "manual" {
            // Speed dropdown: 5% steps of the pad's maximum.
            ui.horizontal(|ui| {
                ui.add(egui::Label::new("Speed:").selectable(false));
                let pct = percent_for_rpm(*rpm);
                let exact = rpm_for_percent(pct) == *rpm;
                let shown = if exact {
                    format!("{}% \u{b7} {} RPM", pct, *rpm)
                } else {
                    format!("Custom \u{b7} {} RPM", *rpm)
                };
                egui::ComboBox::from_id_salt("pad_fan_speed").selected_text(shown).show_ui(
                    ui,
                    |ui| {
                        for step in (15u8..=100).step_by(5) {
                            let r = rpm_for_percent(step);
                            let selected = exact && pct == step;
                            let label = format!("{}% \u{b7} {} RPM", step, r);
                            if ui.selectable_label(selected, label).clicked() && !selected {
                                *rpm = r;
                                action.fan_rpm = Some(r);
                            }
                        }
                    },
                );
            });
        }

        // ---- Three saveable speed presets ----
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Presets:").selectable(false));
            for i in 0..3 {
                let preset_rpm = rpm_for_percent(presets[i]);
                let active = fan_mode == "manual" && *rpm == preset_rpm;
                let response =
                    ui.selectable_label(active, format!("{}%", presets[i])).on_hover_text(format!(
                        "Click: fan at {}% ({} RPM)\nRight-click: save the current speed here",
                        presets[i], preset_rpm
                    ));
                if response.clicked() {
                    *rpm = preset_rpm;
                    if fan_mode == "manual" {
                        action.fan_rpm = Some(preset_rpm);
                    } else {
                        action.fan_manual = true;
                    }
                }
                if response.secondary_clicked() && fan_mode == "manual" {
                    presets[i] = percent_for_rpm(*rpm).max(15);
                    action.presets_saved = true;
                }
            }
            if fan_mode == "manual" {
                ui.add(
                    egui::Label::new(RichText::new("right-click = save current").small().weak())
                        .selectable(false),
                );
            }
        });

        if fan_mode == "manual" {
            egui::CollapsingHeader::new("Exact RPM").default_open(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new("RPM:").selectable(false));
                    let response =
                        ui.add(egui::Slider::new(rpm, MIN_RPM..=MAX_RPM).step_by(RPM_STEP as f64));
                    if response.drag_stopped() || (response.changed() && !response.dragged()) {
                        action.fan_rpm = Some(*rpm);
                    }
                });
            });
        }
    });

    action
}

/// Renders the cooling pad lighting card (Lighting tab). Only shown while a pad is connected.
///
/// * `lighting_enabled` - opt-in switch: when off, R-Helper never touches the pad's lights
/// * `light_mode` - "", "Static", "Breathing" or "Off" (what the app last applied this session)
pub fn render_pad_lighting_section(
    ui: &mut egui::Ui,
    lighting_enabled: &mut bool,
    light_mode: &str,
    color: &mut [u8; 3],
    brightness_pct: &mut u8,
    lighting_available: bool,
) -> PadAction {
    let mut action = PadAction::default();

    crate::theme::card(ui, |ui| {
        ui.add(
            egui::Label::new(crate::theme::section_title("❄ Cooling Pad Lighting"))
                .selectable(false),
        );
        ui.separator();

        // Opt-in. Off = R-Helper never touches the pad's lights.
        let response = ui.checkbox(lighting_enabled, "Toggle App Lighting Control").on_hover_text(
            "Off: R-Helper leaves the pad's lights alone (Razer Chroma / Synapse keep control).\n\
             On: the lights only change when you click here (or restore at startup if you chose that).",
        );
        if response.changed() {
            action.lighting_toggled = Some(*lighting_enabled);
        }

        if !*lighting_enabled {
            return;
        }
        if !lighting_available {
            ui.add(
                egui::Label::new(RichText::new("Pad lighting interface not found").weak())
                    .selectable(false),
            );
            return;
        }

        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Light:").selectable(false));
            for mode in [PadLightingMode::Static, PadLightingMode::Breathing, PadLightingMode::Off]
            {
                let selected = light_mode == mode.name();
                if ui.selectable_label(selected, mode.name()).clicked() && !selected {
                    action.light_mode = Some(mode);
                }
            }
        });

        if light_mode == "Static" || light_mode == "Breathing" {
            ui.horizontal(|ui| {
                ui.add(egui::Label::new("Color:").selectable(false));
                ui.color_edit_button_srgb(color);
                if ui
                    .button("Apply")
                    .on_hover_text("Send the picked color to the pad (it blinks briefly)")
                    .clicked()
                {
                    action.apply_color = true;
                }
            });

            ui.horizontal(|ui| {
                ui.add(egui::Label::new("Brightness:").selectable(false));
                let response = ui.add(egui::Slider::new(brightness_pct, 0..=100).suffix("%"));
                if response.drag_stopped() || (response.changed() && !response.dragged()) {
                    action.brightness = Some(*brightness_pct);
                }
            });
        }
    });

    action
}
