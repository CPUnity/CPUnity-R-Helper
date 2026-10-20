use crate::config::{Config, LightPreset, PRESET_SLOTS};
use eframe::egui::{self, Color32, RichText};
use librazer::command::KeyboardEffect;
use librazer::keymap::KeyLayout;

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
    /// The WorldSets tab (the earned worlds' presets) is showing instead of a bank.
    pub game: bool,
    /// The Shop Purchases tab (the presets of the skins bought in the shop) is showing instead of a bank.
    pub shop: bool,
    /// The Presets tab (the ready-made ones that need no look) is showing instead of a bank.
    pub ready: bool,
}

impl Default for PresetsUiState {
    fn default() -> Self {
        Self { bank: Bank::Keyboard, name: String::new(), include_kbd: true, include_pad: true, game: false, shop: false, ready: false }
    }
}

/// What the user did in the presets card this frame.
#[derive(Debug, Default)]
pub struct PresetsAction {
    pub apply: Option<(Bank, usize)>,
    pub save: Option<(Bank, usize)>,
    /// "Save as new": store the current lighting in the first empty slot of this bank.
    pub save_new: Option<Bank>,
    pub clear: Option<(Bank, usize)>,
    /// "Restore lighting at startup" was switched on / off.
    pub resume_toggled: Option<bool>,
    /// A game preset's Apply button (its id).
    pub apply_game: Option<&'static str>,
    /// "Put it on when a look is unlocked" was switched on / off.
    pub game_auto_toggled: Option<bool>,
}

fn bank_slots(cfg: &Config, bank: Bank) -> &Vec<Option<LightPreset>> {
    match bank {
        Bank::Keyboard => &cfg.kbd_presets,
        Bank::Pad => &cfg.pad_light_presets,
        Bank::Both => &cfg.combo_presets,
    }
}

/// How many slots of `bank` hold a preset.
pub fn saved_count(cfg: &Config, bank: Bank) -> usize {
    bank_slots(cfg, bank).iter().filter(|p| p.is_some()).count()
}

/// The first empty slot of `bank`, if there is one.
pub fn first_free_slot(cfg: &Config, bank: Bank) -> Option<usize> {
    let slots = bank_slots(cfg, bank);
    (0..PRESET_SLOTS).find(|&i| slots.get(i).map_or(true, |p| p.is_none()))
}

fn kbd_summary(p: &crate::config::KbdPart) -> String {
    let effect = KeyboardEffect::from_key(&p.effect);
    let name = effect.map(|e| e.name().to_string()).unwrap_or(p.effect.clone());
    if effect == Some(KeyboardEffect::Custom) {
        let keys = p
            .layout
            .as_deref()
            .and_then(KeyLayout::decode)
            .map(|layout| layout.keys.len())
            .unwrap_or(0);
        return format!("{} ({} keys)", name, keys);
    }
    if matches!(effect, Some(KeyboardEffect::WaveRight | KeyboardEffect::WaveLeft)) && p.split == 3 {
        return format!("{} (own colors)", name);
    }
    if effect == Some(KeyboardEffect::Wheel) && p.split & 2 == 2 {
        return format!("{} (own colors)", name);
    }
    if p.color2.is_some() && !matches!(
        effect,
        Some(KeyboardEffect::Fire | KeyboardEffect::Tidal | KeyboardEffect::Dipp | KeyboardEffect::Wheel)
    ) {
        format!("{} (2 colors)", name)
    } else {
        name
    }
}

fn pad_summary(p: &crate::config::PadPart) -> String {
    if p.color2.is_some() && !matches!(p.mode.as_str(), "Fire" | "Tidal" | "Color Dipp") {
        format!("{} (2 colors)", p.mode)
    } else {
        p.mode.clone()
    }
}

fn summary(p: &LightPreset) -> String {
    match (&p.kbd, &p.pad) {
        (Some(k), Some(d)) => format!("Keyboard {} + Pad {}", kbd_summary(k), pad_summary(d)),
        (Some(k), None) => kbd_summary(k),
        (None, Some(d)) => pad_summary(d),
        (None, None) => String::new(),
    }
}

/// Colors worth showing as swatches (Static / Breathing parts only): up to two.
fn swatches(p: &LightPreset) -> Vec<[u8; 3]> {
    if let Some(k) = &p.kbd {
        let effect = KeyboardEffect::from_key(&k.effect);
        if effect == Some(KeyboardEffect::Custom) {
            // the first two different key colors
            let mut colors: Vec<[u8; 3]> = Vec::new();
            if let Some(layout) = k.layout.as_deref().and_then(KeyLayout::decode) {
                for (_, c) in layout.keys {
                    if colors.len() < 2 && !colors.contains(&c) {
                        colors.push(c);
                    }
                }
            }
            return colors;
        }
        let own_wave = (matches!(effect, Some(KeyboardEffect::WaveRight | KeyboardEffect::WaveLeft)) && k.split == 3)
            || (effect == Some(KeyboardEffect::Wheel) && k.split & 2 == 2);
        if own_wave
            || effect.map_or(false, |e| e.uses_color()
                || matches!(
                    e,
                    KeyboardEffect::Fire
                        | KeyboardEffect::Tidal
                        | KeyboardEffect::Starlight
                        | KeyboardEffect::Dipp
                )) {
            return std::iter::once(k.color).chain(k.color2).collect();
        }
    }
    if let Some(d) = &p.pad {
        if matches!(d.mode.as_str(), "Static" | "Breathing" | "Fire" | "Tidal" | "Starlight" | "Color Dipp")
            || (d.mode == "Wheel" && d.color2.is_some())
        {
            return std::iter::once(d.color).chain(d.color2).collect();
        }
    }
    Vec::new()
}

/// Renders the lighting presets card: a drop-down with the restore-at-startup switch, bank tabs
/// and the presets that have been saved (up to `PRESET_SLOTS` per bank).
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
        let title = format!(
            "\u{1F4A1} Lighting Presets ({} of {} saved)",
            saved_count(cfg, state.bank),
            PRESET_SLOTS
        );
        crate::theme::collapsing(
            ui,
            "lighting_presets",
            crate::theme::section_title(&title),
            false,
            |ui| {
                render_presets_body(ui, state, cfg, resume, pad_present, &mut action);
            },
        );
    });

    action
}

fn render_presets_body(
    ui: &mut egui::Ui,
    state: &mut PresetsUiState,
    cfg: &Config,
    resume: &mut bool,
    pad_present: bool,
    action: &mut PresetsAction,
) {
    let response = crate::theme::toggle(ui, resume, "Restore lighting at startup").on_hover_text(
        "Off (default): the keyboard and pad lights are left alone when the app starts.\n\
         On: the last keyboard effect and pad lighting you applied here are put back at\n\
         launch (and when the pad is plugged in again). Needs \"Toggle App Lighting Control\"\n\
         switched on for that device.",
    );
    if response.changed() {
        action.resume_toggled = Some(*resume);
    }

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::Label::new("Bank:").selectable(false));
        for bank in [Bank::Keyboard, Bank::Pad, Bank::Both] {
            let selected = state.bank == bank && !state.game && !state.ready && !state.shop;
            let label = format!("{} {}/{}", bank.label(), saved_count(cfg, bank), PRESET_SLOTS);
            if ui.selectable_label(selected, label).clicked() {
                state.bank = bank;
                state.game = false;
                state.ready = false;
                state.shop = false;
            }
        }
        let label = format!("Presets {}", crate::game_presets::available_ready(cfg).len());
        if ui.selectable_label(state.ready, label).clicked() {
            state.ready = true;
            state.game = false;
            state.shop = false;
        }
        // (the next two tabs only exist once there is something in them)
        let shop = crate::game_presets::available_shop(cfg).len();
        if shop > 0 {
            // (a build without the shop calls the tab by what is in it)
            let name = if crate::release::story() { "Shop Purchases" } else { "Skin Sets" };
            let label = format!("{} {}", name, shop);
            if ui.selectable_label(state.shop, label).clicked() {
                state.shop = true;
                state.ready = false;
                state.game = false;
            }
        }
        let worlds = crate::game_presets::available_worlds(cfg).len();
        if worlds > 0 {
            let label = format!("WorldSets {}", worlds);
            if ui.selectable_label(state.game, label).clicked() {
                state.game = true;
                state.ready = false;
                state.shop = false;
            }
        }
    });
    if state.game && crate::game_presets::available_worlds(cfg).is_empty() {
        state.game = false;
    }
    if state.shop && crate::game_presets::available_shop(cfg).is_empty() {
        state.shop = false;
    }

    if state.ready {
        render_ready_presets(ui, &crate::game_presets::available_ready(cfg), None, pad_present, action);
        return;
    }
    if state.shop {
        render_ready_presets(ui, &crate::game_presets::available_shop(cfg), Some(cfg.game_preset_auto), pad_present, action);
        return;
    }
    if state.game {
        render_ready_presets(ui, &crate::game_presets::available_worlds(cfg), Some(cfg.game_preset_auto), pad_present, action);
        return;
    }

    if state.bank == Bank::Both {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Save:").selectable(false));
            crate::theme::toggle(ui, &mut state.include_kbd, "Keyboard");
            crate::theme::toggle(ui, &mut state.include_pad, "Cooling pad");
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

    let saved = saved_count(cfg, state.bank);
    let free = first_free_slot(cfg, state.bank);
    ui.horizontal(|ui| {
        ui.add(egui::Label::new("Name:").selectable(false));
        ui.add(
            egui::TextEdit::singleline(&mut state.name)
                .hint_text("name for the next save")
                .char_limit(20)
                .desired_width(150.0),
        );
        if ui
            .add_enabled(free.is_some(), egui::Button::new("Save as new"))
            .on_hover_text("Save the current lighting as a new preset")
            .clicked()
        {
            action.save_new = Some(state.bank);
        }
    });
    ui.add(
        egui::Label::new(
            RichText::new(if free.is_some() {
                format!(
                    "{} of {} saved. Set up the lights above, then press Save as new.",
                    saved, PRESET_SLOTS
                )
            } else {
                format!(
                    "{} of {} saved: full. Update or remove one to make room.",
                    saved, PRESET_SLOTS
                )
            })
            .small()
            .weak(),
        )
        .selectable(false),
    );
    ui.add_space(2.0);

    if saved == 0 {
        ui.add(egui::Label::new(RichText::new("No presets saved yet.").weak()).selectable(false));
        return;
    }

    let slots = bank_slots(cfg, state.bank);
    for i in 0..PRESET_SLOTS {
        let Some(p) = slots.get(i).and_then(|p| p.as_ref()) else {
            continue;
        };
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::Label::new(RichText::new(format!("{:>2}.", i + 1)).weak()).selectable(false),
            );
            for c in swatches(p) {
                ui.add(
                    egui::Label::new(
                        RichText::new("\u{25a0}").color(Color32::from_rgb(c[0], c[1], c[2])),
                    )
                    .selectable(false),
                );
            }
            ui.add(egui::Label::new(RichText::new(&p.name).strong()).selectable(false));
            if ui.button("Apply").on_hover_text("Apply this preset now").clicked() {
                action.apply = Some((state.bank, i));
            }
            if ui
                .button("Update")
                .on_hover_text("Replace this preset with the current lighting (a typed name renames it)")
                .clicked()
            {
                action.save = Some((state.bank, i));
            }
            if ui.button("X").on_hover_text("Delete this preset").clicked() {
                action.clear = Some((state.bank, i));
            }
        });
        ui.add(egui::Label::new(RichText::new(summary(p)).small().weak()).selectable(false));
    }
}

/// A tab of ready-made presets: the Presets tab (`auto` = `None`) or the WorldSets and Shop Purchases tabs, which
/// also have the switch that puts a look's preset on when it is unlocked (`auto` = its state).
fn render_ready_presets(
    ui: &mut egui::Ui,
    presets: &[&'static crate::game_presets::GamePreset],
    auto: Option<bool>,
    pad_present: bool,
    action: &mut PresetsAction,
) {
    if let Some(mut auto) = auto {
        if crate::theme::toggle(ui, &mut auto, "Apply a look's preset when it unlocks")
            .on_hover_text(
                "On: when a look of the program is unlocked or switched on, its preset is applied \
                 to the keyboard and the cooling pad, if their Toggle App Lighting Control is on.\n\
                 Off: presets are only applied with their Apply button.",
            )
            .changed()
        {
            action.game_auto_toggled = Some(auto);
        }
    }
    if !pad_present {
        ui.add(
            egui::Label::new(
                RichText::new("No cooling pad connected: only the keyboard part is applied.")
                    .small()
                    .weak(),
            )
            .selectable(false),
        );
    }
    ui.add_space(2.0);
    for preset in presets.iter().copied() {
        // (shown with its colors; the brightness here is only a placeholder)
        let built = crate::game_presets::build(preset, 255, 100);
        ui.horizontal_wrapped(|ui| {
            for c in swatches(&built) {
                ui.add(
                    egui::Label::new(
                        RichText::new("\u{25a0}").color(Color32::from_rgb(c[0], c[1], c[2])),
                    )
                    .selectable(false),
                );
            }
            ui.add(egui::Label::new(RichText::new(preset.name).strong()).selectable(false));
            if ui.button("Apply").on_hover_text(preset.blurb).clicked() {
                action.apply_game = Some(preset.id);
            }
        });
        ui.add(egui::Label::new(RichText::new(summary(&built)).small().weak()).selectable(false));
        ui.add(egui::Label::new(RichText::new(preset.blurb).small().weak()).selectable(false));
    }
}
