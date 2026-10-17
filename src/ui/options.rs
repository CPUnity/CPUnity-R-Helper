// Options window: accent color, window behavior, startup, AC/battery auto-switch modes.
use crate::config::Config;
use eframe::egui::{self, Color32};
use librazer::types::PerfMode;

const PRESETS: &[(&str, [u8; 3])] = &[
    ("Green", [0, 120, 60]),
    ("Teal", [0, 128, 128]),
    ("Blue", [30, 100, 200]),
    ("Purple", [120, 60, 180]),
    ("Magenta", [190, 40, 120]),
    ("Orange", [200, 100, 0]),
    ("Red", [180, 40, 40]),
];

#[derive(Debug, Default)]
pub struct OptionsResult {
    /// Any config value changed (caller should apply + save).
    pub changed: bool,
    /// Accent color changed.
    pub accent_changed: bool,
    /// UI font changed.
    pub font_changed: bool,
    /// "Always on top" changed.
    pub on_top_changed: bool,
    /// "Launch at startup" was toggled to this value.
    pub startup_toggled: Option<bool>,
    /// AC / battery auto-switch mode (or AC overclock) changed.
    pub modes_changed: bool,
    /// "Apply current profile now" was clicked.
    pub apply_now: bool,
}

pub fn render_options(
    ui: &mut egui::Ui,
    cfg: &mut Config,
    startup: &mut bool,
    modes: &[PerfMode],
    debug_mode: &mut bool,
) -> OptionsResult {
    let mut res = OptionsResult::default();

    ui.label("Font");
    egui::ComboBox::from_id_salt("font_combo").selected_text(cfg.font.clone()).show_ui(ui, |ui| {
        for (name, _) in crate::theme::FONT_CHOICES {
            if ui.selectable_value(&mut cfg.font, name.to_string(), *name).changed() {
                res.changed = true;
                res.font_changed = true;
            }
        }
    });
    ui.add(
        egui::Label::new(
            egui::RichText::new("Custom: put font.ttf (Orbitron, Rajdhani, ...) next to the exe.")
                .small()
                .color(Color32::from_gray(140)),
        )
        .selectable(false),
    );
    ui.add_space(6.0);

    render_custom_image_section(ui);
    ui.add_space(6.0);

    ui.label("Accent color");
    render_color_section(ui, cfg, &mut res);

    ui.separator();

    if ui.checkbox(&mut cfg.always_on_top, "Keep window on top").changed() {
        res.changed = true;
        res.on_top_changed = true;
    }

    if ui.checkbox(startup, "Launch at Windows startup").changed() {
        res.startup_toggled = Some(*startup);
    }

    ui.separator();
    ui.label("AC / battery profile (at launch and on power change)");

    let mut ac_changed = false;
    ui.add_enabled_ui(!cfg.ac_overclock, |ui| {
        ac_changed = mode_combo(ui, "ac_mode_combo", "On AC power:", &mut cfg.ac_mode, modes);
    });
    if ac_changed {
        res.changed = true;
        res.modes_changed = true;
    }
    if ui
        .checkbox(&mut cfg.ac_overclock, "Overclock on AC power (Custom mode + CPU Overclock)")
        .changed()
    {
        res.changed = true;
        res.modes_changed = true;
    }
    if mode_combo(ui, "bat_mode_combo", "On battery:", &mut cfg.battery_mode, modes) {
        res.changed = true;
        res.modes_changed = true;
    }
    ui.add_space(2.0);
    if ui.button("Apply current power profile now").clicked() {
        res.apply_now = true;
    }
    ui.label(
        egui::RichText::new("Default = Performance on AC / Battery on battery when you plug or unplug.\nAt launch only an explicit choice is applied; Default leaves the mode as it is.")
            .small()
            .color(Color32::from_gray(150)),
    );

    ui.separator();
    render_shoutouts(ui);
    egui::CollapsingHeader::new("Advanced (developer)").default_open(false).show(ui, |ui| {
        ui.add(
            egui::Label::new(
                egui::RichText::new(
                    "Warning: Debug lets you select CPU/GPU boost combinations the firmware \
                     table forbids, shows the firmware Max-fan toggle in Custom mode, and \
                     prints extra status messages. Not saved; resets on restart.",
                )
                .small()
                .color(Color32::from_rgb(230, 160, 60)),
            )
            .selectable(false),
        );
        ui.checkbox(debug_mode, "🐛 Debug mode");
    });

    res
}

fn mode_combo(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    value: &mut Option<String>,
    modes: &[PerfMode],
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let current = value.clone().unwrap_or_else(|| "Default".to_string());
        egui::ComboBox::from_id_salt(id).selected_text(current).show_ui(ui, |ui| {
            if ui.selectable_label(value.is_none(), "Default").clicked() {
                *value = None;
                changed = true;
            }
            for m in modes {
                let name = format!("{:?}", m);
                let selected = value.as_deref() == Some(name.as_str());
                if ui.selectable_label(selected, name.clone()).clicked() {
                    *value = Some(name);
                    changed = true;
                }
            }
        });
    });
    changed
}

fn hex_of(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

fn set_accent(cfg: &mut Config, res: &mut OptionsResult, rgb: [u8; 3]) {
    if cfg.accent != rgb {
        cfg.accent = rgb;
        res.changed = true;
        res.accent_changed = true;
    }
}

/// Accent color: presets, exact RGB / hex entry, a visual picker, and 10 saved slots.
fn render_color_section(ui: &mut egui::Ui, cfg: &mut Config, res: &mut OptionsResult) {
    // Presets
    ui.horizontal_wrapped(|ui| {
        for (name, rgb) in PRESETS {
            let col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            let resp = ui
                .add(egui::Button::new("   ").fill(col).min_size(egui::vec2(24.0, 20.0)))
                .on_hover_text(*name);
            if resp.clicked() {
                set_accent(cfg, res, *rgb);
            }
        }
    });

    // Exact values: R / G / B (0-255) and hex.
    ui.horizontal(|ui| {
        let mut rgb = [cfg.accent[0] as i32, cfg.accent[1] as i32, cfg.accent[2] as i32];
        let mut edited = false;
        for (label, v) in ["R", "G", "B"].iter().zip(rgb.iter_mut()) {
            ui.label(*label);
            edited |= ui.add(egui::DragValue::new(v).range(0..=255).speed(1)).changed();
        }
        if edited {
            set_accent(cfg, res, [rgb[0] as u8, rgb[1] as u8, rgb[2] as u8]);
        }

        ui.label("Hex");
        let id = egui::Id::new("accent_hex_text");
        let mut text: String =
            ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_else(|| hex_of(cfg.accent));
        let resp = ui.add(egui::TextEdit::singleline(&mut text).desired_width(70.0));
        if resp.changed() {
            if let Some(v) = parse_hex(&text) {
                set_accent(cfg, res, v);
            }
        }
        if !resp.has_focus() {
            text = hex_of(cfg.accent); // keep in sync with the other controls
        }
        ui.data_mut(|d| d.insert_temp(id, text));
    });

    // Visual picker (hue / saturation / brightness)
    egui::CollapsingHeader::new("Visual color picker").default_open(false).show(ui, |ui| {
        let mut c = Color32::from_rgb(cfg.accent[0], cfg.accent[1], cfg.accent[2]);
        if egui::color_picker::color_picker_color32(ui, &mut c, egui::color_picker::Alpha::Opaque) {
            set_accent(cfg, res, [c.r(), c.g(), c.b()]);
        }
    });

    // 10 saved colors
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new("Saved colors: click to use, right-click to delete")
            .small()
            .color(Color32::from_gray(150)),
    );
    let mut delete: Option<usize> = None;
    let mut apply: Option<[u8; 3]> = None;
    ui.horizontal_wrapped(|ui| {
        for i in 0..10usize {
            match cfg.saved_colors.get(i) {
                Some(rgb) => {
                    let col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                    let resp = ui
                        .add(egui::Button::new("   ").fill(col).min_size(egui::vec2(24.0, 24.0)))
                        .on_hover_text(hex_of(*rgb));
                    if resp.clicked() {
                        apply = Some(*rgb);
                    }
                    if resp.secondary_clicked() {
                        delete = Some(i);
                    }
                }
                None => {
                    ui.add_enabled(
                        false,
                        egui::Button::new("   ").min_size(egui::vec2(24.0, 24.0)),
                    )
                    .on_disabled_hover_text("Empty slot");
                }
            }
        }
    });
    if let Some(rgb) = apply {
        set_accent(cfg, res, rgb);
    }
    if let Some(i) = delete {
        cfg.saved_colors.remove(i);
        res.changed = true;
    }
    if ui.button("💾 Save current color").clicked() && !cfg.saved_colors.contains(&cfg.accent) {
        if cfg.saved_colors.len() >= 10 {
            cfg.saved_colors.remove(0); // full: replace the oldest
        }
        cfg.saved_colors.push(cfg.accent);
        res.changed = true;
    }
}

/// Collapsible "Custom Image" section: how to add a banner and/or a logo.
fn render_custom_image_section(ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Custom Image").default_open(false).show(ui, |ui| {
        let dim = |t: String| {
            egui::Label::new(egui::RichText::new(t).small().color(Color32::from_gray(140)))
                .selectable(false)
        };

        ui.label("Banner");
        let banner_text = match crate::ui::header::banner_slot() {
            Some((w, h)) => {
                let ppp = ui.ctx().pixels_per_point();
                let (pw, ph) = ((w * ppp).round() as u32, (h * ppp).round() as u32);
                format!(
                    "Name it banner.png (or .jpg). It fills the header space right of the title.\n\
                     Free space: {:.0} x {:.0} pt = {} x {} px on this screen (ratio {:.2}:1).\n\
                     Make it about 3x that ({} x {} px) and keep key art centred; the edges \
                     get cropped to fit.",
                    w,
                    h,
                    pw,
                    ph,
                    w / h,
                    (w * 3.0).round() as u32,
                    (h * 3.0).round() as u32
                )
            }
            None => "Name it banner.png (or .jpg). Window too narrow right now - widen it to \
                     see the banner size."
                .to_string(),
        };
        ui.add(dim(banner_text));
        ui.add_space(6.0);

        ui.label("Logo");
        ui.add(dim("Name it logo.png (or .jpg). A small square image (128 x 128 px or larger, \
             transparent PNG works best) shown left of the title. It uses part of the banner's \
             width, so skip it if your banner already has the branding."
            .to_string()));
        ui.add_space(6.0);

        ui.add(dim(
            "Put the files next to the exe or in %APPDATA%\\CPUnity-R-Helper, then restart. \
             To bake them into the exe, put them in the project's assets folder and rebuild."
                .to_string(),
        ));
    });
}

/// Collapsible "Shoutouts" section: credits for everyone this project stands on.
fn render_shoutouts(ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Shoutouts").default_open(false).show(ui, |ui| {
        ui.set_max_width(400.0); // keep long lines wrapping instead of widening the window
        let dim = |t: &str| {
            egui::Label::new(egui::RichText::new(t).small().color(Color32::from_gray(150)))
                .selectable(false)
        };
        ui.add(dim("Huge thanks to everyone who made this possible:"));
        ui.add_space(4.0);
        let rows: [(&str, &str); 10] = [
            ("Fatalution", "R-Helper, the app this is built on"),
            ("blauzim & tdakhran", "razer-ctl / librazer, the device layer"),
            ("Razer-Linux laptop-control contributors", "protocol groundwork"),
            ("NxRoot", "SynapseKiller, the idea behind Synapse Standby"),
            ("egui, image-rs, hidapi-rs, windows-rs & more", "the Rust crates that carry the app"),
            ("The Rust project and community", "the language and tooling"),
            ("CPUnity", "the project and its vision"),
            ("Volk-M\u{e4}rchen:VonV_T", "testing, art direction, relentless bug-hunting"),
            ("Claude (Anthropic)", "development assistant"),
            ("ChatGPT (OpenAI)", "logo and banner art"),
        ];
        for (who, what) in rows {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add(egui::Label::new(egui::RichText::new(who).strong()).selectable(false));
                ui.add(dim(&format!("- {}", what)));
            });
        }
        ui.add_space(6.0);
        ui.add(dim("Please support NxRoot too: if Synapse Standby saves you time, give their \
             SynapseKiller a star on GitHub."));
        ui.add(
            egui::Label::new(
                egui::RichText::new("github.com/NxRoot/SynapseKiller")
                    .small()
                    .color(Color32::from_gray(190)),
            )
            .selectable(true),
        );
    });
}
