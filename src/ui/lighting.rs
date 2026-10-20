use crate::config::Config;
use eframe::egui;
use librazer::command::{KeyboardEffect, KeyboardSplit, SPEED_MAX, SPEED_MIN, STANDARD_SPEED};
use librazer::cooling_pad::PadLightingMode;
use librazer::keymap::{KeyFx, COLS, KEYS, QUICK_SETS, ROWS};

// Discrete brightness levels that the keyboard actually supports
// Based on testing with Fn+F10/F11 brightness keys
const BRIGHTNESS_LEVELS: &[u8] = &[
    0,   // Step 0
    13,  // Step 1
    28,  // Step 2
    43,  // Step 3
    59,  // Step 4
    74,  // Step 5
    89,  // Step 6
    105, // Step 7
    120, // Step 8
    133, // Step 9
    148, // Step 10
    163, // Step 11
    179, // Step 12
    194, // Step 13
    209, // Step 14
    225, // Step 15
];

/// Actions that can be triggered from the lighting UI
#[derive(Debug, Clone, PartialEq)]
pub struct LightingAction {
    /// Logo lighting mode to set
    pub logo_mode: Option<String>,
    /// Brightness value to set (0-255 raw value)
    pub brightness: Option<u8>,
    /// Whether the lights always on setting was toggled
    pub lights_always_on: bool,
    /// Whether the brightness slider is currently being interacted with
    pub slider_active: Option<bool>,
    /// "Toggle App Lighting Control" for the keyboard was switched on / off
    pub kbd_lighting_toggled: Option<bool>,
    /// A keyboard effect button was clicked
    pub kbd_effect: Option<KeyboardEffect>,
    /// "Apply" next to the keyboard color was clicked
    pub kbd_apply_color: bool,
    /// A key, the "other keys" color or the key map shift of Custom Keys was changed
    pub kbd_custom_changed: bool,
}

impl Default for LightingAction {
    fn default() -> Self {
        Self {
            logo_mode: None,
            brightness: None,
            lights_always_on: false,
            slider_active: None,
            kbd_lighting_toggled: None,
            kbd_effect: None,
            kbd_apply_color: false,
            kbd_custom_changed: false,
        }
    }
}

/// Renders the lighting section UI
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `logo_mode` - The current logo lighting mode
/// * `temp_brightness_step` - Mutable reference to brightness step index (0-15)
/// * `lights_always_on` - Mutable reference to lights always on setting
///
/// # Returns
/// The action requested by the user, if any
pub fn render_lighting_section(
    ui: &mut egui::Ui,
    logo_mode: &str,
    temp_brightness_step: &mut usize,
    lights_always_on: &mut bool,
    cfg: &mut Config,
    kbd_effect: &str,
    sync_active: bool,
) -> LightingAction {
    let mut action = LightingAction::default();

    crate::theme::card(ui, |ui| {
        crate::theme::collapsing(
            ui,
            "kbd_logo_header",
            crate::theme::section_title("💡 Keyboard & Logo"),
            true,
            |ui| {
                // Logo Mode Selection
                render_logo_mode_selection(ui, logo_mode, &mut action);

                // Brightness Slider
                render_brightness_controls(ui, temp_brightness_step, &mut action);

                // Lights Always On Toggle
                render_always_on_toggle(ui, lights_always_on, &mut action);

                // Keyboard effects (opt-in) in a drop-down of their own. While the sync card drives the
                // keyboard they are greyed out, and switching sync on folds them away.
                crate::theme::collapsing(
                    ui,
                    "kbd_lighting_header",
                    "Keyboard lighting",
                    true,
                    |ui| {
                        if sync_active {
                            ui.disable();
                        }
                        render_keyboard_effects(ui, cfg, kbd_effect, &mut action);
                    },
                );
            },
        );
    });

    action
}

/// What the user did in the sync card this frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SyncAction {
    /// The sync switch was flipped.
    pub toggled: Option<bool>,
    /// An effect button (Static / Breathing / Off) was clicked: set it on both devices.
    pub effect: Option<KeyboardEffect>,
    /// The colors, layout or speed changed (or Apply was clicked): send them to both devices.
    pub apply: bool,
    /// New cooling pad brightness (percent).
    pub pad_brightness: Option<u8>,
}

/// Renders the sync card: one set of controls (effect, colors, layout, speed) that is applied to
/// the keyboard and the cooling pad together. The keyboard's own settings are the shared ones;
/// the pad's copies are filled in when they are applied.
pub fn render_sync_section(ui: &mut egui::Ui, cfg: &mut Config, kbd_effect: &str) -> SyncAction {
    let mut action = SyncAction::default();

    crate::theme::card(ui, |ui| {
        crate::theme::collapsing(
            ui,
            "kbd_sync_header",
            crate::theme::section_title("Sync Keyboard + Cooling Pad"),
            true,
            |ui| {
                let response =
            crate::theme::toggle(ui, &mut cfg.light_sync, "Sync keyboard and cooling pad")
                .on_hover_text(
                "On: the controls below set the keyboard and the cooling pad together, and the\n\
                 separate keyboard effect and cooling pad lighting controls are greyed out.\n\
                 Turning it on also switches on Toggle App Lighting Control for both.",
            );
                if response.changed() {
                    action.toggled = Some(cfg.light_sync);
                }
                if !cfg.light_sync {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(
                                "Off: the keyboard and the pad are set separately.",
                            )
                            .small()
                            .weak(),
                        )
                        .selectable(false),
                    );
                    return;
                }

                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::Label::new("Effect:").selectable(false));
                    for effect in [
                        KeyboardEffect::Static,
                        KeyboardEffect::Breathing,
                        KeyboardEffect::Fire,
                        KeyboardEffect::Tidal,
                        KeyboardEffect::Starlight,
                        KeyboardEffect::Spectrum,
                        KeyboardEffect::Dipp,
                        KeyboardEffect::Rainbow,
                        KeyboardEffect::Off,
                    ] {
                        let selected = kbd_effect == effect.name();
                        if ui.selectable_label(selected, effect.name()).clicked() && !selected {
                            action.effect = Some(effect);
                        }
                    }
                });

                let effect = KeyboardEffect::from_name(kbd_effect);
                let colored =
                    matches!(effect, Some(KeyboardEffect::Static | KeyboardEffect::Breathing));
                if colored {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("Color:").selectable(false));
                        ui.color_edit_button_srgb(&mut cfg.kbd_color);
                        if crate::theme::toggle(ui, &mut cfg.kbd_two, "Two colors").changed() {
                            action.apply = true;
                        }
                        if cfg.kbd_two {
                            ui.color_edit_button_srgb(&mut cfg.kbd_color2);
                        }
                        if ui
                            .button("Apply")
                            .on_hover_text("Send the picked colors to both")
                            .clicked()
                        {
                            action.apply = true;
                        }
                    });

                    if cfg.kbd_two && effect == Some(KeyboardEffect::Static) {
                        ui.horizontal_wrapped(|ui| {
                            ui.add(egui::Label::new("Layout:").selectable(false));
                            for split in KeyboardSplit::ALL {
                                let selected = cfg.kbd_split == split.index();
                                if ui.selectable_label(selected, split.name()).clicked()
                                    && !selected
                                {
                                    cfg.kbd_split = split.index();
                                    action.apply = true;
                                }
                            }
                        });
                    }
                    if cfg.kbd_two && effect == Some(KeyboardEffect::Breathing) {
                        ui.horizontal_wrapped(|ui| {
                            ui.add(egui::Label::new("Layout:").selectable(false));
                            for split in KeyboardSplit::BREATHING {
                                let selected = cfg.kbd_breath_split == split.breath_index();
                                if ui.selectable_label(selected, split.name()).clicked()
                                    && !selected
                                {
                                    cfg.kbd_breath_split = split.breath_index();
                                    action.apply = true;
                                }
                            }
                        });
                    }
                    if cfg.kbd_two {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(
                                    "The cooling pad shows Top | Bottom as two halves.",
                                )
                                .small()
                                .weak(),
                            )
                            .selectable(false),
                        );
                    }
                }

                if effect == Some(KeyboardEffect::Breathing)
                    && speed_row(ui, &mut cfg.kbd_breath_speed)
                {
                    action.apply = true;
                }

                if effect == Some(KeyboardEffect::Fire) {
                    // One pair of colors for both devices (apart from Sync, each has its own).
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("Hot:").selectable(false));
                        let hot = ui.color_edit_button_srgb(&mut cfg.fire_hot);
                        ui.add(egui::Label::new("Cold:").selectable(false));
                        let cold = ui.color_edit_button_srgb(&mut cfg.fire_cold);
                        if hot.changed() || cold.changed() {
                            action.apply = true;
                        }
                        if ui
                            .button("Razer's colors")
                            .on_hover_text("Orange and red, the colors of Razer Chroma's own Fire")
                            .clicked()
                        {
                            cfg.fire_hot = [255, 110, 0];
                            cfg.fire_cold = [255, 0, 0];
                            action.apply = true;
                        }
                    });
                    ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Both burn in these colors while Sync is on. Switch Sync off and each \
                         has colors of its own. R-Helper draws the fire, so it only burns while \
                         it is open.",
                    )
                    .small()
                    .weak(),
                )
                .selectable(false),
            );
                }

                if effect == Some(KeyboardEffect::Dipp) {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("From:").selectable(false));
                        let a = ui.color_edit_button_srgb(&mut cfg.dipp_a);
                        ui.add(egui::Label::new("To:").selectable(false));
                        let b = ui.color_edit_button_srgb(&mut cfg.dipp_b);
                        if a.changed() || b.changed() {
                            action.apply = true;
                        }
                    });
                    ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Both slide between these colors while Sync is on. Switch Sync off and \
                         each has colors of its own.",
                    )
                    .small()
                    .weak(),
                )
                .selectable(false),
            );
                }

                if effect == Some(KeyboardEffect::Starlight) {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("Color:").selectable(false));
                        let a = ui.color_edit_button_srgb(&mut cfg.starlight_a);
                        let two = crate::theme::toggle(ui, &mut cfg.starlight_two, "Two colors");
                        let mut b_changed = false;
                        if cfg.starlight_two {
                            b_changed = ui.color_edit_button_srgb(&mut cfg.starlight_b).changed();
                        }
                        if a.changed() || two.changed() || b_changed {
                            action.apply = true;
                        }
                    });
                    ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Stars twinkle on both while Sync is on. Switch Sync off and each has \
                         colors of its own.",
                    )
                    .small()
                    .weak(),
                )
                .selectable(false),
            );
                }

                if effect == Some(KeyboardEffect::Tidal) {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("Colors:").selectable(false));
                        let a = ui.color_edit_button_srgb(&mut cfg.tidal_a);
                        let b = ui.color_edit_button_srgb(&mut cfg.tidal_b);
                        if a.changed() || b.changed() {
                            action.apply = true;
                        }
                        if ui
                            .button("Razer's colors")
                            .on_hover_text(
                                "Green and blue, the colors Razer Chroma's Tidal was captured with",
                            )
                            .clicked()
                        {
                            cfg.tidal_a = [0, 255, 0];
                            cfg.tidal_b = [0, 0, 255];
                            action.apply = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("Direction:").selectable(false));
                        for (name, inward) in [("Out", false), ("In", true)] {
                            let selected = cfg.tidal_in == inward;
                            if ui.selectable_label(selected, name).clicked() && !selected {
                                cfg.tidal_in = inward;
                                action.apply = true;
                            }
                        }
                    });
                    ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Both run these colors while Sync is on (out from a line across the \
                         keyboard and from the middle of the pad's strip). Switch Sync off and \
                         each has colors of its own.",
                    )
                    .small()
                    .weak(),
                )
                .selectable(false),
            );
                }

                if let Some(speed) = effect_speed(cfg, effect) {
                    if speed_row(ui, speed) {
                        action.apply = true;
                    }
                }

                ui.horizontal(|ui| {
                    ui.add(egui::Label::new("Pad brightness:").selectable(false));
                    let response =
                        ui.add(egui::Slider::new(&mut cfg.pad_brightness, 0..=100).suffix("%"));
                    if response.drag_stopped() || (response.changed() && !response.dragged()) {
                        action.pad_brightness = Some(cfg.pad_brightness);
                    }
                });
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(
                            "Keyboard brightness: use the slider above. Wave and Custom Keys need \
                     Sync off.",
                        )
                        .small()
                        .weak(),
                    )
                    .selectable(false),
                );
            },
        );
    });

    action
}

/// A Speed row: a slider from 25% to 250% of the standard speed, and a button that puts it back.
/// Returns true when a new speed was chosen (when the slider is let go, so the effect is not
/// restarted over and over while it is dragged).
pub fn speed_row(ui: &mut egui::Ui, value: &mut u8) -> bool {
    let mut chosen = false;
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::Label::new("Speed:").selectable(false));
        let mut speed = (*value).clamp(SPEED_MIN, SPEED_MAX);
        let response = ui.add(egui::Slider::new(&mut speed, SPEED_MIN..=SPEED_MAX).suffix("%"));
        *value = speed;
        if response.drag_stopped() || (response.changed() && !response.dragged()) {
            chosen = true;
        }
        if ui.small_button("Standard").on_hover_text("Back to the standard speed (100%)").clicked()
            && *value != STANDARD_SPEED
        {
            *value = STANDARD_SPEED;
            chosen = true;
        }
    });
    chosen
}

/// The keyboard's speed setting for an effect that is drawn by R-Helper (Breathing's is handled
/// where it is shown).
fn effect_speed(cfg: &mut Config, effect: Option<KeyboardEffect>) -> Option<&mut u8> {
    match effect? {
        KeyboardEffect::Fire => Some(&mut cfg.fire_speed),
        KeyboardEffect::Tidal => Some(&mut cfg.tidal_speed),
        KeyboardEffect::Starlight => Some(&mut cfg.starlight_speed),
        KeyboardEffect::Spectrum => Some(&mut cfg.spectrum_speed),
        KeyboardEffect::Dipp => Some(&mut cfg.dipp_speed),
        KeyboardEffect::Rainbow => Some(&mut cfg.rainbow_speed),
        KeyboardEffect::Wheel => Some(&mut cfg.wheel_speed),
        KeyboardEffect::WaveRight | KeyboardEffect::WaveLeft if cfg.wave_custom => {
            Some(&mut cfg.wave_speed)
        }
        _ => None,
    }
}

/// The cooling pad mode that matches a synced keyboard effect.
pub fn pad_mode_for(effect: KeyboardEffect) -> Option<PadLightingMode> {
    match effect {
        KeyboardEffect::Static => Some(PadLightingMode::Static),
        KeyboardEffect::Breathing => Some(PadLightingMode::Breathing),
        KeyboardEffect::Fire => Some(PadLightingMode::Fire),
        KeyboardEffect::Tidal => Some(PadLightingMode::Tidal),
        KeyboardEffect::Starlight => Some(PadLightingMode::Starlight),
        KeyboardEffect::Spectrum => Some(PadLightingMode::Spectrum),
        KeyboardEffect::Dipp => Some(PadLightingMode::Dipp),
        KeyboardEffect::Rainbow => Some(PadLightingMode::Rainbow),
        KeyboardEffect::Wheel => Some(PadLightingMode::Wheel),
        KeyboardEffect::Off => Some(PadLightingMode::Off),
        _ => None,
    }
}

/// Renders the logo mode selection controls
fn render_logo_mode_selection(ui: &mut egui::Ui, logo_mode: &str, action: &mut LightingAction) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new("Logo Mode:").selectable(false));
        const LOGO_MODES: &[&str] = &["Static", "Breathing", "Off"];

        for mode in LOGO_MODES {
            let selected = logo_mode == *mode;
            if ui.selectable_label(selected, *mode).clicked() && !selected {
                action.logo_mode = Some(mode.to_string());
            }
        }
    });
}

/// Renders the brightness control slider
fn render_brightness_controls(
    ui: &mut egui::Ui,
    temp_brightness_step: &mut usize,
    action: &mut LightingAction,
) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new("Keyboard Brightness:").selectable(false));

        // Ensure step index is within bounds
        *temp_brightness_step = (*temp_brightness_step).min(BRIGHTNESS_LEVELS.len() - 1);

        let mut step_index = *temp_brightness_step;
        let brightness_response = ui.add(
            egui::Slider::new(&mut step_index, 0..=(BRIGHTNESS_LEVELS.len() - 1))
                .custom_formatter(|val, _| format!("{}", val as usize))
                .custom_parser(|s| s.parse::<f64>().ok()),
        );

        // Check if the value actually changed
        let value_changed = step_index != *temp_brightness_step;
        *temp_brightness_step = step_index;

        // Track slider interaction state
        if brightness_response.dragged() || brightness_response.has_focus() {
            action.slider_active = Some(true);
            // Send brightness command immediately when value changes during interaction
            if value_changed {
                action.brightness = Some(BRIGHTNESS_LEVELS[*temp_brightness_step]);
            }
        } else if brightness_response.drag_stopped() || brightness_response.lost_focus() {
            action.slider_active = Some(false);
            // Send the final brightness value when interaction ends
            if value_changed {
                action.brightness = Some(BRIGHTNESS_LEVELS[*temp_brightness_step]);
            }
        } else if value_changed {
            // Handle cases where value changed without drag (e.g., clicking on slider track)
            action.brightness = Some(BRIGHTNESS_LEVELS[*temp_brightness_step]);
        }
    });
}

/// Renders the always on toggle control
fn render_always_on_toggle(
    ui: &mut egui::Ui,
    lights_always_on: &mut bool,
    action: &mut LightingAction,
) {
    ui.horizontal(|ui| {
        if crate::theme::toggle(ui, lights_always_on, "Keyboard Backlight Always On").clicked() {
            action.lights_always_on = true;
        }
    });
}

/// Renders the opt-in keyboard effect controls.
///
/// While "Toggle App Lighting Control" is off, R-Helper never sends a keyboard color or effect.
fn render_keyboard_effects(
    ui: &mut egui::Ui,
    cfg: &mut Config,
    kbd_effect: &str,
    action: &mut LightingAction,
) {
    ui.add_space(4.0);
    ui.separator();

    let response = crate::theme::toggle(ui, &mut cfg.kbd_lighting, "Toggle App Lighting Control").on_hover_text(
        "Off: R-Helper leaves the keyboard's color and effect alone (Razer Chroma / Synapse keep control).\n\
         On: the keyboard effect only changes when you click here (or restore at startup if you chose that).",
    );
    if response.changed() {
        action.kbd_lighting_toggled = Some(cfg.kbd_lighting);
    }
    if !cfg.kbd_lighting {
        return;
    }

    ui.horizontal_wrapped(|ui| {
        ui.add(egui::Label::new("Effect:").selectable(false));
        for effect in KeyboardEffect::ALL {
            let selected = kbd_effect == effect.name();
            if ui.selectable_label(selected, effect.name()).clicked() && !selected {
                action.kbd_effect = Some(effect);
            }
        }
    });

    let effect = KeyboardEffect::from_name(kbd_effect);
    if effect.map_or(false, |e| e.uses_color()) {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Color:").selectable(false));
            ui.color_edit_button_srgb(&mut cfg.kbd_color);
            if crate::theme::toggle(ui, &mut cfg.kbd_two, "Two colors")
                .on_hover_text(
                    "Breathing alternates between both colors, or fades both halves / a gradient\n\
                     at once (see Layout); Static shows both.",
                )
                .changed()
            {
                action.kbd_apply_color = true;
            }
            if cfg.kbd_two {
                ui.color_edit_button_srgb(&mut cfg.kbd_color2);
            }
            if ui.button("Apply").on_hover_text("Send the picked colors to the keyboard").clicked()
            {
                action.kbd_apply_color = true;
            }
        });

        if cfg.kbd_two && effect == Some(KeyboardEffect::Static) {
            ui.horizontal(|ui| {
                ui.add(egui::Label::new("Layout:").selectable(false));
                for split in KeyboardSplit::ALL {
                    let selected = cfg.kbd_split == split.index();
                    if ui.selectable_label(selected, split.name()).clicked() && !selected {
                        cfg.kbd_split = split.index();
                        action.kbd_apply_color = true;
                    }
                }
            });
        }

        if effect == Some(KeyboardEffect::Breathing) {
            if speed_row(ui, &mut cfg.kbd_breath_speed) {
                action.kbd_apply_color = true;
            }
            if cfg.kbd_breath_speed != STANDARD_SPEED {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(
                            "A speed other than 100% is drawn by R-Helper, so it only runs while it is open.",
                        )
                        .small()
                        .weak(),
                    )
                    .selectable(false),
                );
            }
        }

        if cfg.kbd_two && effect == Some(KeyboardEffect::Breathing) {
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::Label::new("Layout:").selectable(false));
                for split in KeyboardSplit::BREATHING {
                    let selected = cfg.kbd_breath_split == split.breath_index();
                    if ui.selectable_label(selected, split.name()).clicked() && !selected {
                        cfg.kbd_breath_split = split.breath_index();
                        action.kbd_apply_color = true;
                    }
                }
            });
            if cfg.kbd_breath_split != 0 {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(
                            "Halves and gradient fade in and out while R-Helper is running.",
                        )
                        .small()
                        .weak(),
                    )
                    .selectable(false),
                );
            }
        }
    }

    if effect == Some(KeyboardEffect::Custom) {
        render_custom_keys(ui, cfg, action);
    }
    if effect == Some(KeyboardEffect::Fire) {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Hot:").selectable(false));
            let hot = ui
                .color_edit_button_srgb(&mut cfg.fire_hot)
                .on_hover_text("The brightest, hottest keys (bottom rows)");
            ui.add(egui::Label::new("Cold:").selectable(false));
            let cold = ui
                .color_edit_button_srgb(&mut cfg.fire_cold)
                .on_hover_text("The dim, cold keys (top rows)");
            if hot.changed() || cold.changed() {
                action.kbd_apply_color = true;
            }
            if ui
                .button("Razer's colors")
                .on_hover_text("Orange and red, the colors of Razer Chroma's own Fire")
                .clicked()
            {
                cfg.fire_hot = [255, 110, 0];
                cfg.fire_cold = [255, 0, 0];
                action.kbd_apply_color = true;
            }
        });
        ui.add(
            egui::Label::new(
                egui::RichText::new(
                    "Made like Razer Chroma's Fire: dim and cold along the top of the keyboard, hot and bright \
                     at the bottom, every key flickering. R-Helper draws it, so it only burns while it is open.",
                )
                .small()
                .weak(),
            )
            .selectable(false),
        );
    }
    if effect == Some(KeyboardEffect::Rainbow) {
        ui.add(
            egui::Label::new(
                egui::RichText::new(
                    "Every color of the rainbow at once across the keyboard, scrolling along to \
                     the right. It has no colors to pick. R-Helper draws it, so it only runs \
                     while it is open.",
                )
                .small()
                .weak(),
            )
            .selectable(false),
        );
    }
    if effect == Some(KeyboardEffect::Spectrum) {
        ui.add(
            egui::Label::new(
                egui::RichText::new(
                    "Made like Razer Chroma's Spectrum: every key goes slowly round the rainbow \
                     together (about 38 seconds a turn) and rests a few seconds on a blue. It has \
                     no colors to pick. R-Helper draws it, so it only runs while it is open.",
                )
                .small()
                .weak(),
            )
            .selectable(false),
        );
    }
    if matches!(effect, Some(KeyboardEffect::WaveRight | KeyboardEffect::WaveLeft)) {
        ui.horizontal_wrapped(|ui| {
            if crate::theme::toggle(ui, &mut cfg.wave_custom, "Choose colors")
                .on_hover_text("Off: Razer's own wave, a rainbow, which cannot be given colors. On: R-Helper draws the wave in the colors you pick.")
                .changed()
            {
                action.kbd_apply_color = true;
            }
            if cfg.wave_custom {
                let a = ui
                    .color_edit_button_srgb(&mut cfg.wave_a)
                    .on_hover_text("One color of the wave");
                let b = ui
                    .color_edit_button_srgb(&mut cfg.wave_b)
                    .on_hover_text("The other color (pick a very dark one for bands of one color on dark)");
                if a.changed() || b.changed() {
                    action.kbd_apply_color = true;
                }
            }
        });
        ui.add(
            egui::Label::new(
                egui::RichText::new(if cfg.wave_custom {
                    "The keys wash from one color to the other as each band passes. R-Helper draws it, so it only runs while it is open."
                } else {
                    "Razer's own wave, in every color of the rainbow. It runs on the keyboard itself."
                })
                .small()
                .weak(),
            )
            .selectable(false),
        );
    }
    if effect == Some(KeyboardEffect::Wheel) {
        ui.horizontal_wrapped(|ui| {
            if crate::theme::toggle(ui, &mut cfg.wheel_custom, "Choose colors")
                .on_hover_text("Off: Razer's own wheel, the rainbow turning round, which cannot be given colors. On: R-Helper draws the wheel going from one color you pick to another.")
                .changed()
            {
                action.kbd_apply_color = true;
            }
            if cfg.wheel_custom {
                ui.add(egui::Label::new("From:").selectable(false));
                let a = ui
                    .color_edit_button_srgb(&mut cfg.wheel_a)
                    .on_hover_text("One color of the wheel");
                ui.add(egui::Label::new("To:").selectable(false));
                let b = ui
                    .color_edit_button_srgb(&mut cfg.wheel_b)
                    .on_hover_text("The other color; the hues between show along the way");
                if a.changed() || b.changed() {
                    action.kbd_apply_color = true;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Direction:").selectable(false));
            for (name, ccw) in [("Clockwise", false), ("Counter-clockwise", true)] {
                let selected = cfg.wheel_ccw == ccw;
                if ui.selectable_label(selected, name).clicked() && !selected {
                    cfg.wheel_ccw = ccw;
                    action.kbd_apply_color = true;
                }
            }
        });
        ui.add(
            egui::Label::new(
                egui::RichText::new(if cfg.wheel_custom {
                    "The wheel in two colors: it goes from the first to the second through the \
                     hues between (as Color Dipp does) over half a turn and back over the other \
                     half, and turns once a second. R-Helper draws it, so it only runs while it \
                     is open."
                } else {
                    "Made like Razer's Wheel: the rainbow is laid round a point on the Shift row \
                     and turns once a second, so each key shows the color that is at its angle. \
                     R-Helper draws it, so it only runs while it is open. Sync also turns the \
                     cooling pad's lights."
                })
                .small()
                .weak(),
            )
            .selectable(false),
        );
    }
    if effect == Some(KeyboardEffect::Dipp) {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("From:").selectable(false));
            let a = ui
                .color_edit_button_srgb(&mut cfg.dipp_a)
                .on_hover_text("The color it starts on and comes back to");
            ui.add(egui::Label::new("To:").selectable(false));
            let b =
                ui.color_edit_button_srgb(&mut cfg.dipp_b).on_hover_text("The color it slides to");
            if a.changed() || b.changed() {
                action.kbd_apply_color = true;
            }
        });
        ui.add(
            egui::Label::new(
                egui::RichText::new(
                    "Color Dipp: every key slides from the first color to the second through \
                     every hue in between (red to violet passes orange, yellow, green and blue), \
                     rests a moment, and slides back. R-Helper draws it, so it only runs while it \
                     is open.",
                )
                .small()
                .weak(),
            )
            .selectable(false),
        );
    }
    if effect == Some(KeyboardEffect::Starlight) {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Color:").selectable(false));
            let a = ui
                .color_edit_button_srgb(&mut cfg.starlight_a)
                .on_hover_text("The color of the stars");
            let two = crate::theme::toggle(ui, &mut cfg.starlight_two, "Two colors")
                .on_hover_text("Every star is followed by the same star in the second color");
            let mut b_changed = false;
            if cfg.starlight_two {
                b_changed = ui
                    .color_edit_button_srgb(&mut cfg.starlight_b)
                    .on_hover_text("The color that follows each star")
                    .changed();
            }
            if a.changed() || two.changed() || b_changed {
                action.kbd_apply_color = true;
            }
            if ui
                .button("Razer's color")
                .on_hover_text("Green, the color Razer Chroma's Starlight was captured with")
                .clicked()
            {
                cfg.starlight_a = [0, 255, 0];
                action.kbd_apply_color = true;
            }
        });
        ui.add(
            egui::Label::new(
                egui::RichText::new(
                    "Made like Razer Chroma's Starlight: stars twinkle on random keys, each \
                     fading up and down. With two colors every star is followed by the same \
                     star in the second color. R-Helper draws it, so it only twinkles while it \
                     is open.",
                )
                .small()
                .weak(),
            )
            .selectable(false),
        );
    }
    if effect == Some(KeyboardEffect::Tidal) {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Colors:").selectable(false));
            let a = ui
                .color_edit_button_srgb(&mut cfg.tidal_a)
                .on_hover_text("The first color of each wave");
            let b = ui
                .color_edit_button_srgb(&mut cfg.tidal_b)
                .on_hover_text("The second color of each wave");
            if a.changed() || b.changed() {
                action.kbd_apply_color = true;
            }
            if ui
                .button("Razer's colors")
                .on_hover_text("Green and blue, the colors Razer Chroma's Tidal was captured with")
                .clicked()
            {
                cfg.tidal_a = [0, 255, 0];
                cfg.tidal_b = [0, 0, 255];
                action.kbd_apply_color = true;
            }
        });
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Direction:").selectable(false));
            for (name, inward) in [("Out", false), ("In", true)] {
                let selected = cfg.tidal_in == inward;
                if ui.selectable_label(selected, name).clicked() && !selected {
                    cfg.tidal_in = inward;
                    action.kbd_apply_color = true;
                }
            }
        });
        ui.add(
            egui::Label::new(
                egui::RichText::new(
                    "Made like Razer Chroma's Tidal: each key shows the first color, rests, then \
                     the second, and the waves run diagonally across the keyboard, jetting out \
                     from a line or coming in towards it. R-Helper draws it, so it only runs \
                     while it is open.",
                )
                .small()
                .weak(),
            )
            .selectable(false),
        );
    }
    if let Some(speed) = effect_speed(cfg, effect) {
        if speed_row(ui, speed) {
            action.kbd_apply_color = true;
        }
    }
    ui.add(
        egui::Label::new(
            egui::RichText::new("Brightness: use the Keyboard Brightness slider above.")
                .small()
                .weak(),
        )
        .selectable(false),
    );
}

/// True when black text reads better than white on this color.
fn is_light(c: [u8; 3]) -> bool {
    0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32 > 150.0
}

/// The short mark that follows a key's name in the editor when the key moves.
fn fx_mark(fx: KeyFx) -> &'static str {
    match fx {
        KeyFx::Static => "",
        KeyFx::Breathing => "~",
        KeyFx::Fire => "^",
        KeyFx::Spectrum => "*",
        KeyFx::Flash => "!",
        KeyFx::Starlight => "+",
        KeyFx::Dipp => "%",
        KeyFx::Wheel => "@",
    }
}

/// What the brush's second color does for this effect.
fn second_hint(fx: KeyFx) -> &'static str {
    match fx {
        KeyFx::Breathing => "The key breathes in the first color, then in the second, and so on.",
        KeyFx::Fire => "The flame goes between the second color (low) and the first (high).",
        KeyFx::Starlight => "Each star is followed by the same star in the second color.",
        KeyFx::Dipp => {
            "The key goes from the first color to the second through every hue between, and back."
        }
        KeyFx::Wheel => "The tail of the hand turns from the first color into the second.",
        _ => "",
    }
}

/// The Custom Keys editor: brush and background colors, quick sets and a clickable keyboard.
fn render_custom_keys(ui: &mut egui::Ui, cfg: &mut Config, action: &mut LightingAction) {
    let brush_fx = KeyFx::from_index(cfg.kbd_brush_fx);
    // the second color goes with the key when the effect can use one and it is on (Color Dipp always)
    let two = brush_fx.can_two() && (cfg.kbd_brush_two || brush_fx.needs_two());
    let second = two.then_some(cfg.kbd_brush2);

    ui.horizontal_wrapped(|ui| {
        ui.add(egui::Label::new("Brush:").selectable(false));
        ui.color_edit_button_srgb(&mut cfg.kbd_brush)
            .on_hover_text("The color a click on a key paints");
        if brush_fx.can_two() {
            if !brush_fx.needs_two() {
                crate::theme::toggle(ui, &mut cfg.kbd_brush_two, "Two colors")
                    .on_hover_text(second_hint(brush_fx));
            }
            if two {
                ui.color_edit_button_srgb(&mut cfg.kbd_brush2).on_hover_text(second_hint(brush_fx));
            }
        }
        ui.add(egui::Label::new("Other keys:").selectable(false));
        if ui
            .color_edit_button_srgb(&mut cfg.kbd_custom.background)
            .on_hover_text("The color of every key you did not pick (black = off)")
            .changed()
        {
            action.kbd_custom_changed = true;
        }
    });
    if brush_fx.can_two() && !second_hint(brush_fx).is_empty() {
        ui.add(
            egui::Label::new(egui::RichText::new(second_hint(brush_fx)).small().weak())
                .selectable(false),
        );
    }

    ui.horizontal_wrapped(|ui| {
        ui.add(egui::Label::new("Effect:").selectable(false));
        for fx in KeyFx::ALL {
            let selected = brush_fx == fx;
            if ui
                .selectable_label(selected, fx.name())
                .on_hover_text("What a key does when you click it with the brush")
                .clicked()
                && !selected
            {
                cfg.kbd_brush_fx = fx.index();
            }
        }
    });
    if cfg.kbd_custom.is_animated() || brush_fx != KeyFx::Static {
        if speed_row(ui, &mut cfg.kbd_breath_speed) {
            action.kbd_custom_changed = true;
        }
    }

    ui.horizontal_wrapped(|ui| {
        ui.add(egui::Label::new("Quick sets:").selectable(false));
        for (name, ids) in QUICK_SETS.iter() {
            if ui
                .button(*name)
                .on_hover_text("Light only these keys, in the brush color and effect")
                .clicked()
            {
                cfg.kbd_custom.clear_keys();
                cfg.kbd_custom.set_many_with_second(ids, cfg.kbd_brush, second, brush_fx);
                action.kbd_custom_changed = true;
            }
        }
        if ui
            .button("All keys")
            .on_hover_text("Light every key in the brush color and effect")
            .clicked()
        {
            let brush = cfg.kbd_brush;
            for key in KEYS.iter() {
                cfg.kbd_custom.set_with_second(key.id, brush, second, brush_fx);
            }
            action.kbd_custom_changed = true;
        }
        if ui.button("Clear").on_hover_text("Turn every key off").clicked() {
            cfg.kbd_custom.clear_keys();
            action.kbd_custom_changed = true;
        }
    });

    // The keyboard: one button per key, as wide as the cells the key covers. The keys are painted at
    // exactly that width (a button grows to fit its text, and a row that grows past the card
    // stretches the card and everything below it).
    ui.scope(|ui| {
        let gap = 2.0_f32;
        ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
        let unit = ((ui.available_width() - gap * COLS as f32) / COLS as f32)
            .floor()
            .clamp(18.0, 36.0);
        for row in 0..ROWS as u8 {
            ui.horizontal(|ui| {
                let mut next_col: u8 = 0;
                for key in KEYS.iter().filter(|key| key.row == row) {
                    if key.col > next_col {
                        ui.add_space((key.col - next_col) as f32 * (unit + gap));
                    }
                    next_col = key.col + key.span;

                    let lit = cfg.kbd_custom.color_of(key.id);
                    let key_fx = cfg.kbd_custom.fx_of(key.id);
                    let key_second = cfg.kbd_custom.second_of(key.id);
                    let fill = match lit {
                        Some(c) => egui::Color32::from_rgb(c[0], c[1], c[2]),
                        None => egui::Color32::from_gray(38),
                    };
                    let text_color = match lit {
                        Some(c) if is_light(c) => egui::Color32::BLACK,
                        Some(_) => egui::Color32::WHITE,
                        None => egui::Color32::from_gray(190),
                    };
                    let width = unit * key.span as f32 + gap * (key.span as f32 - 1.0);
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::click());
                    let response = response.on_hover_text(format!(
                        "{}{}: click lights it in the brush color and effect (click again: off), right-click: off",
                        key.label,
                        if lit.is_some() { format!(" ({})", key_fx.name()) } else { String::new() }
                    ));
                    if ui.is_rect_visible(rect) {
                        let painter = ui.painter();
                        let radius = egui::CornerRadius::same(4);
                        painter.rect_filled(rect, radius, fill);
                        let edge = if response.hovered() {
                            ui.visuals().widgets.hovered.bg_stroke
                        } else {
                            ui.visuals().widgets.inactive.bg_stroke
                        };
                        painter.rect_stroke(rect, radius, edge, egui::StrokeKind::Inside);
                        // a second color shows as a strip along the bottom of the key
                        if let Some(c) = key_second {
                            let strip = egui::Rect::from_min_max(
                                egui::pos2(rect.left() + 3.0, rect.bottom() - 6.0),
                                egui::pos2(rect.right() - 3.0, rect.bottom() - 3.0),
                            );
                            painter.rect_filled(strip, 1.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
                        }
                        // the name, and a small mark in the corner when the key moves (nothing can
                        // spill out of the key)
                        let inside = painter.with_clip_rect(rect);
                        let lift = if key_second.is_some() { -2.0 } else { 0.0 };
                        inside.text(
                            rect.center() + egui::vec2(0.0, lift),
                            egui::Align2::CENTER_CENTER,
                            key.label,
                            egui::FontId::proportional(10.5),
                            text_color,
                        );
                        let mark = fx_mark(key_fx);
                        if lit.is_some() && !mark.is_empty() {
                            inside.text(
                                rect.right_top() + egui::vec2(-2.5, 1.0),
                                egui::Align2::RIGHT_TOP,
                                mark,
                                egui::FontId::proportional(9.0),
                                text_color,
                            );
                        }
                    }
                    if response.clicked() {
                        if lit == Some(cfg.kbd_brush) && key_fx == brush_fx && key_second == second {
                            cfg.kbd_custom.clear_key(key.id);
                        } else {
                            cfg.kbd_custom.set_with_second(key.id, cfg.kbd_brush, second, brush_fx);
                        }
                        action.kbd_custom_changed = true;
                    }
                    if response.secondary_clicked() {
                        cfg.kbd_custom.clear_key(key.id);
                        action.kbd_custom_changed = true;
                    }
                }
            });
        }
    });

    ui.add(
        egui::Label::new(
            egui::RichText::new(
                "Marks: ~ breathing, ^ fire, * spectrum, ! flash, + starlight, % color dipp, @ wheel. A \
                 strip along a key is its second color. Keys that move are drawn by R-Helper, so they \
                 only move while it is open.",
            )
            .small()
            .weak(),
        )
        .selectable(false),
    );

    egui::CollapsingHeader::new("Key map fine-tune").default_open(false).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Shift columns:").selectable(false));
            let mut shift = cfg.kbd_key_shift as i32;
            if ui.add(egui::DragValue::new(&mut shift).range(-3..=3)).changed() {
                cfg.kbd_key_shift = shift as i8;
                action.kbd_custom_changed = true;
            }
        });
        ui.add(
            egui::Label::new(
                egui::RichText::new(
                    "If a different key lights than the one you clicked, shift by 1 or -1.",
                )
                .small()
                .weak(),
            )
            .selectable(false),
        );
    });
}

/// Converts raw brightness (0-255) to the closest supported step index
pub fn raw_brightness_to_step_index(brightness: u8) -> usize {
    BRIGHTNESS_LEVELS
        .iter()
        .enumerate()
        .min_by_key(|(_, &level)| (level as i16 - brightness as i16).abs())
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}
