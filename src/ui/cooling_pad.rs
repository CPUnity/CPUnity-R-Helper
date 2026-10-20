use crate::config::{clean_preset_name, snap_rpm, Config, PadProfile, PAD_PROFILES_MAX};
use eframe::egui::{self, RichText};
use librazer::command::STANDARD_SPEED;
use librazer::cooling_pad::{
    percent_for_rpm, rpm_for_percent, PadLightingMode, MAX_RPM, MIN_RPM, RPM_STEP,
};

/// What the user did in the Cooling Pad card this frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PadAction {
    /// "Toggle App Control" for the fan was switched on / off.
    pub control_toggled: Option<bool>,
    /// Fan switched to Off.
    pub fan_off: bool,
    /// Fan switched to Manual (at the current speed value).
    pub fan_manual: bool,
    /// A new fan speed was chosen while in Manual (dropdown or exact slider).
    pub fan_rpm: Option<u16>,
    /// Apply this fan profile (index into `cfg.pad_profiles`).
    pub apply_profile: Option<usize>,
    /// A profile was added, renamed, re-timed or deleted (needs saving).
    pub profiles_changed: bool,
    /// "Toggle App Lighting Control" was switched on / off (lighting card).
    pub lighting_toggled: Option<bool>,
    /// A lighting mode button was clicked.
    pub light_mode: Option<PadLightingMode>,
    /// "Apply" next to the color button was clicked.
    pub apply_color: bool,
    /// Brightness slider released / changed (percent, 0-100).
    pub brightness: Option<u8>,
}

/// Renders the Cooling Pad fan card: a drop-down with the opt-in switch, the fan controls and up
/// to ten named fan profiles. Only shown while a pad is connected.
///
/// * `fan_mode` - "", "off" or "manual" ("" = never touched by the app)
pub fn render_cooling_pad_section(
    ui: &mut egui::Ui,
    fan_mode: &str,
    cfg: &mut Config,
) -> PadAction {
    let mut action = PadAction::default();

    crate::theme::card(ui, |ui| {
        let status = if !cfg.pad_control {
            "Connected".to_string()
        } else {
            match fan_mode {
                "manual" => format!("{} RPM", cfg.pad_rpm),
                "off" => "Fan off".to_string(),
                _ => "Connected".to_string(),
            }
        };
        let title = format!("\u{2744} Cooling Pad  \u{b7}  {}", status);
        crate::theme::collapsing(
            ui,
            "pad_fan_card",
            crate::theme::section_title(&title),
            true,
            |ui| {
                render_fan_body(ui, fan_mode, cfg, &mut action);
            },
        );
    });

    action
}

fn render_fan_body(ui: &mut egui::Ui, fan_mode: &str, cfg: &mut Config, action: &mut PadAction) {
    let response = crate::theme::toggle(ui, &mut cfg.pad_control, "Toggle App Control")
        .on_hover_text(
            "Off: R-Helper never changes the pad's fan (the pad / Synapse keep control).\n\
         On: the fan only changes when you click here, and the last fan setting is put back\n\
         when the app starts or the pad is plugged in again.",
        );
    if response.changed() {
        action.control_toggled = Some(cfg.pad_control);
    }
    if !cfg.pad_control {
        return;
    }

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
            let rpm = cfg.pad_rpm;
            let pct = percent_for_rpm(rpm);
            let exact = rpm_for_percent(pct) == rpm;
            let shown = if exact {
                format!("{}% \u{b7} {} RPM", pct, rpm)
            } else {
                format!("Custom \u{b7} {} RPM", rpm)
            };
            egui::ComboBox::from_id_salt("pad_fan_speed").selected_text(shown).show_ui(ui, |ui| {
                for step in (15u8..=100).step_by(5) {
                    let r = rpm_for_percent(step);
                    let selected = exact && pct == step;
                    let label = format!("{}% \u{b7} {} RPM", step, r);
                    if ui.selectable_label(selected, label).clicked() && !selected {
                        cfg.pad_rpm = r;
                        action.fan_rpm = Some(r);
                    }
                }
            });
        });

        egui::CollapsingHeader::new("Exact RPM").default_open(false).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Label::new("RPM:").selectable(false));
                let response = ui.add(
                    egui::Slider::new(&mut cfg.pad_rpm, MIN_RPM..=MAX_RPM).step_by(RPM_STEP as f64),
                );
                if response.drag_stopped() || (response.changed() && !response.dragged()) {
                    action.fan_rpm = Some(cfg.pad_rpm);
                }
            });
        });
    }

    render_profiles(ui, fan_mode, cfg, action);
}

/// The named fan profiles: a drop-down that lists them, each one a drop-down of its own.
fn render_profiles(ui: &mut egui::Ui, fan_mode: &str, cfg: &mut Config, action: &mut PadAction) {
    let title = format!("Fan profiles ({} of {})", cfg.pad_profiles.len(), PAD_PROFILES_MAX);
    crate::theme::collapsing(ui, "pad_profiles", title, false, |ui| {
        let mut remove: Option<usize> = None;
        for i in 0..cfg.pad_profiles.len() {
            let running = fan_mode == "manual" && cfg.pad_profiles[i].rpm == cfg.pad_rpm;
            let header = format!(
                "{} \u{b7} {} RPM{}",
                cfg.pad_profiles[i].name,
                cfg.pad_profiles[i].rpm,
                if running { "  (running)" } else { "" }
            );
            egui::CollapsingHeader::new(header).id_salt(("pad_profile", i)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Apply").on_hover_text("Run the fan at this speed").clicked() {
                        action.apply_profile = Some(i);
                    }
                    if ui
                        .button("Use current")
                        .on_hover_text("Store the fan's current speed here")
                        .clicked()
                    {
                        cfg.pad_profiles[i].rpm = snap_rpm(cfg.pad_rpm);
                        action.profiles_changed = true;
                    }
                    if ui.button("X").on_hover_text("Delete this profile").clicked() {
                        remove = Some(i);
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new("Name:").selectable(false));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut cfg.pad_profiles[i].name)
                            .char_limit(20)
                            .desired_width(150.0),
                    );
                    if response.lost_focus() && response.changed() {
                        let cleaned = clean_preset_name(&cfg.pad_profiles[i].name);
                        cfg.pad_profiles[i].name =
                            if cleaned.is_empty() { "Profile".to_string() } else { cleaned };
                        action.profiles_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new("RPM:").selectable(false));
                    let response = ui.add(
                        egui::DragValue::new(&mut cfg.pad_profiles[i].rpm)
                            .range(MIN_RPM..=MAX_RPM)
                            .speed(10.0)
                            .suffix(" RPM"),
                    );
                    if response.drag_stopped() || (response.changed() && !response.dragged()) {
                        cfg.pad_profiles[i].rpm = snap_rpm(cfg.pad_profiles[i].rpm);
                        action.profiles_changed = true;
                    }
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("steps of {}", RPM_STEP)).small().weak(),
                        )
                        .selectable(false),
                    );
                });
            });
        }
        if let Some(i) = remove {
            cfg.pad_profiles.remove(i);
            action.profiles_changed = true;
        }

        if cfg.pad_profiles.is_empty() {
            ui.add(
                egui::Label::new(RichText::new("No profiles saved yet.").weak()).selectable(false),
            );
        }

        // ---- add a profile from the current fan speed ----
        if cfg.pad_profiles.len() < PAD_PROFILES_MAX {
            let id = egui::Id::new("pad_profile_new_name");
            let mut name: String = ui.data(|d| d.get_temp(id)).unwrap_or_default();
            ui.horizontal(|ui| {
                ui.add(egui::Label::new("New:").selectable(false));
                ui.add(
                    egui::TextEdit::singleline(&mut name)
                        .hint_text("profile name")
                        .char_limit(20)
                        .desired_width(130.0),
                );
                if ui
                    .button(format!("Save {} RPM", cfg.pad_rpm))
                    .on_hover_text("Save the fan's current speed as a new profile")
                    .clicked()
                {
                    let cleaned = clean_preset_name(&name);
                    let name_to_use = if cleaned.is_empty() {
                        format!("Profile {}", cfg.pad_profiles.len() + 1)
                    } else {
                        cleaned
                    };
                    cfg.pad_profiles
                        .push(PadProfile { name: name_to_use, rpm: snap_rpm(cfg.pad_rpm) });
                    name.clear();
                    action.profiles_changed = true;
                }
            });
            ui.data_mut(|d| d.insert_temp(id, name));
        } else {
            ui.add(
                egui::Label::new(
                    RichText::new("All profiles used: delete one to add another.").small().weak(),
                )
                .selectable(false),
            );
        }
    });
}

/// Renders the cooling pad lighting card (Lighting tab). Only shown while a pad is connected.
///
/// * `cfg.pad_lighting` - opt-in switch: when off, R-Helper never touches the pad's lights
/// * `light_mode` - "", "Static", "Breathing" or "Off" (what the app last applied this session)
pub fn render_pad_lighting_section(
    ui: &mut egui::Ui,
    cfg: &mut Config,
    light_mode: &str,
    lighting_available: bool,
) -> PadAction {
    let mut action = PadAction::default();

    crate::theme::card(ui, |ui| {
        crate::theme::collapsing(
            ui,
            "pad_lighting_header",
            crate::theme::section_title("❄ Cooling Pad Lighting"),
            true,
            |ui| {
                // The sync card drives the pad while Sync is on.
                if cfg.light_sync {
                    ui.disable();
                }

                // Opt-in. Off = R-Helper never touches the pad's lights.
                let response = crate::theme::toggle(ui, &mut cfg.pad_lighting, "Toggle App Lighting Control").on_hover_text(
            "Off: R-Helper leaves the pad's lights alone (Razer Chroma / Synapse keep control).\n\
             On: the lights only change when you click here (or restore at startup if you chose that).",
        );
                if response.changed() {
                    action.lighting_toggled = Some(cfg.pad_lighting);
                }

                if !cfg.pad_lighting {
                    return;
                }
                if !lighting_available {
                    ui.add(
                        egui::Label::new(RichText::new("Pad lighting interface not found").weak())
                            .selectable(false),
                    );
                    return;
                }

                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::Label::new("Light:").selectable(false));
                    for mode in [
                        PadLightingMode::Static,
                        PadLightingMode::Breathing,
                        PadLightingMode::Fire,
                        PadLightingMode::Tidal,
                        PadLightingMode::Starlight,
                        PadLightingMode::Spectrum,
                        PadLightingMode::Dipp,
                        PadLightingMode::Rainbow,
                        PadLightingMode::Wheel,
                        PadLightingMode::Off,
                    ] {
                        let selected = light_mode == mode.name();
                        if ui.selectable_label(selected, mode.name()).clicked() && !selected {
                            action.light_mode = Some(mode);
                        }
                    }
                });

                if light_mode == "Fire" {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("Hot:").selectable(false));
                        let hot = ui.color_edit_button_srgb(&mut cfg.pad_fire_hot);
                        ui.add(egui::Label::new("Cold:").selectable(false));
                        let cold = ui.color_edit_button_srgb(&mut cfg.pad_fire_cold);
                        if hot.changed() || cold.changed() {
                            action.apply_color = true;
                        }
                        if ui
                            .button("Razer's colors")
                            .on_hover_text("Orange and red, the colors of Razer Chroma's own Fire")
                            .clicked()
                        {
                            cfg.pad_fire_hot = [255, 110, 0];
                            cfg.pad_fire_cold = [255, 0, 0];
                            action.apply_color = true;
                        }
                    });
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "Made like Razer Chroma's Fire, along the pad's light strip (the \
                                 keyboard's Fire has colors of its own unless Sync is on). \
                                 R-Helper draws it, so it only burns while it is open.",
                            )
                            .small()
                            .weak(),
                        )
                        .selectable(false),
                    );
                }

                if light_mode == "Color Dipp" {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("From:").selectable(false));
                        let a = ui.color_edit_button_srgb(&mut cfg.pad_dipp_a);
                        ui.add(egui::Label::new("To:").selectable(false));
                        let b = ui.color_edit_button_srgb(&mut cfg.pad_dipp_b);
                        if a.changed() || b.changed() {
                            action.apply_color = true;
                        }
                    });
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "The whole strip slides from the first color to the second \
                                 through every hue in between (red to violet passes orange, \
                                 yellow, green and blue), rests a moment, and slides back. The \
                                 keyboard's Color Dipp has colors of its own unless Sync is on. \
                                 R-Helper draws it, so it only runs while it is open.",
                            )
                            .small()
                            .weak(),
                        )
                        .selectable(false),
                    );
                }

                if light_mode == "Rainbow" {
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "Every color of the rainbow at once along the pad's light strip, \
                                 scrolling along. It has no colors to pick. R-Helper draws it, so \
                                 it only runs while it is open.",
                            )
                            .small()
                            .weak(),
                        )
                        .selectable(false),
                    );
                }

                if light_mode == "Wheel" {
                    ui.horizontal_wrapped(|ui| {
                        if crate::theme::toggle(ui, &mut cfg.pad_wheel_custom, "Choose colors")
                            .on_hover_text("Off: Razer's own wheel, the rainbow, which cannot be given colors. On: R-Helper draws the wheel going from one color you pick to another.")
                            .changed()
                        {
                            action.apply_color = true;
                        }
                        if cfg.pad_wheel_custom {
                            ui.add(egui::Label::new("From:").selectable(false));
                            let a = ui.color_edit_button_srgb(&mut cfg.pad_wheel_a);
                            ui.add(egui::Label::new("To:").selectable(false));
                            let b = ui.color_edit_button_srgb(&mut cfg.pad_wheel_b);
                            if a.changed() || b.changed() {
                                action.apply_color = true;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("Direction:").selectable(false));
                        for (name, ccw) in [("Clockwise", false), ("Counter-clockwise", true)] {
                            let selected = cfg.pad_wheel_ccw == ccw;
                            if ui.selectable_label(selected, name).clicked() && !selected {
                                cfg.pad_wheel_ccw = ccw;
                                action.apply_color = true;
                            }
                        }
                    });
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "Made like Razer's Wheel: the rainbow turns round once a second, \
                                 the pad's two halves half a rainbow apart. With Choose colors it \
                                 goes from one color to the other instead. R-Helper draws it, so \
                                 it only runs while it is open.",
                            )
                            .small()
                            .weak(),
                        )
                        .selectable(false),
                    );
                }

                if light_mode == "Spectrum" {
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "Made like Razer Chroma's Spectrum: the whole strip goes slowly \
                                 round the rainbow (about 38 seconds a turn). It has no colors to \
                                 pick. R-Helper draws it, so it only runs while it is open.",
                            )
                            .small()
                            .weak(),
                        )
                        .selectable(false),
                    );
                }

                if light_mode == "Starlight" {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("Color:").selectable(false));
                        let a = ui.color_edit_button_srgb(&mut cfg.pad_starlight_a);
                        let two = crate::theme::toggle(ui, &mut cfg.pad_starlight_two, "Two colors");
                        let mut b_changed = false;
                        if cfg.pad_starlight_two {
                            b_changed = ui.color_edit_button_srgb(&mut cfg.pad_starlight_b).changed();
                        }
                        if a.changed() || two.changed() || b_changed {
                            action.apply_color = true;
                        }
                        if ui
                            .button("Razer's color")
                            .on_hover_text("Green, the color Razer Chroma's Starlight was captured with")
                            .clicked()
                        {
                            cfg.pad_starlight_a = [0, 255, 0];
                            action.apply_color = true;
                        }
                    });
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "Made like Razer Chroma's Starlight: stars twinkle along the pad's \
                                 light strip. With two colors every star is followed by the same \
                                 star in the second color. The keyboard's Starlight has colors of \
                                 its own unless Sync is on. R-Helper draws it, so it only twinkles \
                                 while it is open.",
                            )
                            .small()
                            .weak(),
                        )
                        .selectable(false),
                    );
                }

                if light_mode == "Tidal" {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("Colors:").selectable(false));
                        let a = ui.color_edit_button_srgb(&mut cfg.pad_tidal_a);
                        let b = ui.color_edit_button_srgb(&mut cfg.pad_tidal_b);
                        if a.changed() || b.changed() {
                            action.apply_color = true;
                        }
                        if ui
                            .button("Razer's colors")
                            .on_hover_text("Green and blue, the colors Razer Chroma's Tidal was captured with")
                            .clicked()
                        {
                            cfg.pad_tidal_a = [0, 255, 0];
                            cfg.pad_tidal_b = [0, 0, 255];
                            action.apply_color = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("Direction:").selectable(false));
                        for (name, inward) in [("Out", false), ("In", true)] {
                            let selected = cfg.pad_tidal_in == inward;
                            if ui.selectable_label(selected, name).clicked() && !selected {
                                cfg.pad_tidal_in = inward;
                                action.apply_color = true;
                            }
                        }
                    });
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "Made like Razer Chroma's Tidal: the waves run out from the middle \
                                 of the pad's strip (or come in from the ends). The keyboard's \
                                 Tidal has colors of its own unless Sync is on. R-Helper draws it, \
                                 so it only runs while it is open.",
                            )
                            .small()
                            .weak(),
                        )
                        .selectable(false),
                    );
                }

                {
                    let speed = match light_mode {
                        "Fire" => Some(&mut cfg.pad_fire_speed),
                        "Tidal" => Some(&mut cfg.pad_tidal_speed),
                        "Starlight" => Some(&mut cfg.pad_starlight_speed),
                        "Spectrum" => Some(&mut cfg.pad_spectrum_speed),
                        "Color Dipp" => Some(&mut cfg.pad_dipp_speed),
                        "Rainbow" => Some(&mut cfg.pad_rainbow_speed),
                        "Wheel" => Some(&mut cfg.pad_wheel_speed),
                        _ => None,
                    };
                    if let Some(speed) = speed {
                        if super::lighting::speed_row(ui, speed) {
                            action.apply_color = true;
                        }
                    }
                }

                if light_mode == "Static" || light_mode == "Breathing" {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("Color:").selectable(false));
                        ui.color_edit_button_srgb(&mut cfg.pad_color);
                        if crate::theme::toggle(ui, &mut cfg.pad_two, "Two colors")
                    .on_hover_text(
                        "Breathing alternates between both colors, or fades both halves / a\n\
                         gradient at once (see Layout); Static shows both.",
                    )
                    .changed()
                {
                    action.apply_color = true;
                }
                        if cfg.pad_two {
                            ui.color_edit_button_srgb(&mut cfg.pad_color2);
                        }
                        if ui
                            .button("Apply")
                            .on_hover_text("Send the picked colors to the pad (it blinks briefly)")
                            .clicked()
                        {
                            action.apply_color = true;
                        }
                    });

                    if cfg.pad_two && light_mode == "Static" {
                        ui.horizontal(|ui| {
                            ui.add(egui::Label::new("Layout:").selectable(false));
                            for (index, name) in [(0u8, "Two halves"), (1u8, "Gradient")] {
                                let selected = cfg.pad_split == index;
                                if ui.selectable_label(selected, name).clicked() && !selected {
                                    cfg.pad_split = index;
                                    action.apply_color = true;
                                }
                            }
                        });
                        ui.add(
                            egui::Label::new(
                                RichText::new("Two-color Static on the pad is experimental.")
                                    .small()
                                    .weak(),
                            )
                            .selectable(false),
                        );
                    }

                    if light_mode == "Breathing" {
                        if super::lighting::speed_row(ui, &mut cfg.pad_breath_speed) {
                            action.apply_color = true;
                        }
                        if cfg.pad_breath_speed != STANDARD_SPEED {
                            ui.add(
                        egui::Label::new(
                            RichText::new(
                                "A speed other than 100% is drawn by R-Helper while it is open (experimental).",
                            )
                            .small()
                            .weak(),
                        )
                        .selectable(false),
                    );
                        }
                    }

                    if cfg.pad_two && light_mode == "Breathing" {
                        ui.horizontal(|ui| {
                            ui.add(egui::Label::new("Layout:").selectable(false));
                            for (index, name) in
                                [(0u8, "Alternate"), (1u8, "Two halves"), (2u8, "Gradient")]
                            {
                                let selected = cfg.pad_breath_split == index;
                                if ui.selectable_label(selected, name).clicked() && !selected {
                                    cfg.pad_breath_split = index;
                                    action.apply_color = true;
                                }
                            }
                        });
                        if cfg.pad_breath_split != 0 {
                            ui.add(
                        egui::Label::new(
                            RichText::new(
                                "Halves and gradient fade in and out while R-Helper is running (experimental).",
                            )
                            .small()
                            .weak(),
                        )
                        .selectable(false),
                    );
                        }
                    }

                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("Brightness:").selectable(false));
                        let response =
                            ui.add(egui::Slider::new(&mut cfg.pad_brightness, 0..=100).suffix("%"));
                        if response.drag_stopped() || (response.changed() && !response.dragged()) {
                            action.brightness = Some(cfg.pad_brightness);
                        }
                    });
                }
            },
        );
    });

    action
}
