use crate::config::{Config, LightPreset, PRESET_SLOTS};
use eframe::egui::{self, Align, Color32, Layout, RichText};
use librazer::command::KeyboardEffect;

/// Which bank of presets is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bank {
    Keyboard,
    Pad,
    Both,
}

impl Bank {
    pub fn label(self) -> &'static str {
        match self {
            Bank::Keyboard => "Keyboard",
            Bank::Pad => "Cooling Pad",
            Bank::Both => "Both together",
        }
    }
}

/// UI-only state of the presets card (not saved).
pub struct PresetsUiState {
    pub bank: Bank,
    /// Name typed for the next save.
    pub name: String,
    /// "Both" bank: save the keyboard part / the pad part.
    pub include_kbd: bool,
    pub include_pad: bool,
}

impl Default for PresetsUiState {
    fn default() -> Self {
        Self { bank: Bank::Keyboard, name: String::new(), include_kbd: true, include_pad: true }
    }
}

/// What the user did in the presets card this frame.
#[derive(Debug, Default)]
pub struct PresetsAction {
    pub apply: Option<(Bank, usize)>,
    pub save: Option<(Bank, usize)>,
    pub clear: Option<(Bank, usize)>,
    /// "Restore lighting at startup" was switched on / off.
    pub resume_toggled: Option<bool>,
}

fn bank_slots(cfg: &Config, bank: Bank) -> &Vec<Option<LightPreset>> {
    match bank {
        Bank::Keyboard => &cfg.kbd_presets,
        Bank::Pad => &cfg.pad_light_presets,
        Bank::Both => &cfg.combo_presets,
    }
}

fn kbd_summary(p: &crate::config::KbdPart) -> String {
    KeyboardEffect::from_key(&p.effect).map(|e| e.name().to_string()).unwrap_or(p.effect.clone())
}

fn summary(p: &LightPreset) -> String {
    match (&p.kbd, &p.pad) {
        (Some(k), Some(d)) => format!("Keyboard {} + Pad {}", kbd_summary(k), d.mode),
        (Some(k), None) => kbd_summary(k),
        (None, Some(d)) => d.mode.clone(),
        (None, None) => String::new(),
    }
}

/// First color worth showing as a swatch (Static / Breathing parts only).
fn swatch(p: &LightPreset) -> Option<[u8; 3]> {
    if let Some(k) = &p.kbd {
        if KeyboardEffect::from_key(&k.effect).map_or(false, |e| e.uses_color()) {
            return Some(k.color);
        }
    }
    if let Some(d) = &p.pad {
        if d.mode == "Static" || d.mode == "Breathing" {
            return Some(d.color);
        }
    }
    None
}

/// Renders the lighting presets card: restore-at-startup switch, bank tabs and ten slots.
///
/// * `resume` - "Restore lighting at startup" (opt-in)
/// * `pad_present` - a cooling pad is connected right now
pub fn render_presets_card(
    ui: &mut egui::Ui,
    state: &mut PresetsUiState,
    cfg: &Config,
    resume: &mut bool,
    pad_present: bool,
) -> PresetsAction {
    let mut action = PresetsAction::default();

    crate::theme::card(ui, |ui| {
        ui.add(
            egui::Label::new(crate::theme::section_title("💡 Lighting Presets")).selectable(false),
        );
        ui.separator();

        let response = ui.checkbox(resume, "Restore lighting at startup").on_hover_text(
            "Off (default): the keyboard and pad lights are left alone when the app starts.\n\
             On: the last keyboard effect and pad lighting you applied here are put back at\n\
             launch (and when the pad is plugged in again). Needs \"Toggle App Lighting Control\"\n\
             switched on for that device.",
        );
        if response.changed() {
            action.resume_toggled = Some(*resume);
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Bank:").selectable(false));
            for bank in [Bank::Keyboard, Bank::Pad, Bank::Both] {
                let selected = state.bank == bank;
                if ui.selectable_label(selected, bank.label()).clicked() {
                    state.bank = bank;
                }
            }
        });

        if state.bank == Bank::Both {
            ui.horizontal(|ui| {
                ui.add(egui::Label::new("Save:").selectable(false));
                ui.checkbox(&mut state.include_kbd, "Keyboard");
                ui.checkbox(&mut state.include_pad, "Cooling pad");
            });
        }
        if state.bank != Bank::Keyboard && !pad_present {
            ui.add(
                egui::Label::new(
                    RichText::new("No cooling pad connected: pad parts can't be saved or applied.")
                        .small()
                        .weak(),
                )
                .selectable(false),
            );
        }

        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Name:").selectable(false));
            ui.add(
                egui::TextEdit::singleline(&mut state.name)
                    .hint_text("name for the next save")
                    .char_limit(20)
                    .desired_width(170.0),
            );
        });
        ui.add(
            egui::Label::new(
                RichText::new(
                    "Set up the lights above, type a name (optional), then press Save on a slot.",
                )
                .small()
                .weak(),
            )
            .selectable(false),
        );
        ui.add_space(2.0);

        let slots = bank_slots(cfg, state.bank);
        for i in 0..PRESET_SLOTS {
            let preset = slots.get(i).and_then(|p| p.as_ref());
            ui.horizontal(|ui| {
                ui.add(
                    egui::Label::new(RichText::new(format!("{:>2}.", i + 1)).weak())
                        .selectable(false),
                );
                match preset {
                    Some(p) => {
                        if let Some(c) = swatch(p) {
                            ui.add(
                                egui::Label::new(
                                    RichText::new("\u{25a0}")
                                        .color(Color32::from_rgb(c[0], c[1], c[2])),
                                )
                                .selectable(false),
                            );
                        }
                        ui.add(egui::Label::new(RichText::new(&p.name).strong()).selectable(false));
                        ui.add(
                            egui::Label::new(RichText::new(summary(p)).small().weak())
                                .selectable(false),
                        );
                    }
                    None => {
                        ui.add(egui::Label::new(RichText::new("(empty)").weak()).selectable(false));
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_enabled(preset.is_some(), egui::Button::new("X"))
                        .on_hover_text("Clear this slot")
                        .clicked()
                    {
                        action.clear = Some((state.bank, i));
                    }
                    if ui
                        .button("Save")
                        .on_hover_text(
                            "Save the current lighting here (replaces what is in this slot)",
                        )
                        .clicked()
                    {
                        action.save = Some((state.bank, i));
                    }
                    if ui
                        .add_enabled(preset.is_some(), egui::Button::new("Apply"))
                        .on_hover_text("Apply this preset now")
                        .clicked()
                    {
                        action.apply = Some((state.bank, i));
                    }
                });
            });
        }
    });

    action
}
