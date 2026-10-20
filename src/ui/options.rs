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
    /// "Close button hides to the tray" was toggled.
    pub tray_changed: bool,
    /// "Preview Gold Edition" was clicked: unlock it for now and show the cat king.
    pub gold_preview: bool,
    /// The sensor source (or its address / file) was changed.
    pub sensor_source_changed: bool,
    /// "Restart as administrator now" was clicked.
    pub admin_restart: bool,
    /// "Set up..." (start at login without the prompt) was clicked: open the confirmation.
    pub login_task_setup: bool,
    /// "Remove" (the login task) was clicked.
    pub login_task_remove: bool,
    /// "Break the staff" was clicked: skip the moon and go straight to Game 3.
    pub break_staff: bool,
    /// "Take life" was clicked (a test switch): straight to the end scene of Game 3.
    pub take_life: bool,
    /// "Start Game 1 at part two" was clicked (Game testing).
    pub game1_part2: bool,
    /// "Start Game 2" was clicked (Game testing).
    pub game2_start: bool,
    /// "Talk to the cat" was clicked (Game testing): the talk before Game 1.
    pub cat_talk: bool,
    /// "Enter the Ally" was clicked (Game testing): in, whether or not the alley is open.
    pub ally_enter: bool,
    /// "Sit at the Devil's Table" was clicked (the buy-in is taken by the caller).
    pub poker_sit: bool,
    /// The free test seat at the Devil's Table was clicked.
    pub poker_test: bool,
    /// Something was bought in the shop (its id).
    pub shop_bought: Option<&'static str>,
    /// Trophies given by hand in Shop testing (their ids), to be announced like earned ones.
    pub trophies_given: Vec<&'static str>,
}

/// Explanation of a sensor source (shown as a tooltip and under its switch).
fn source_hint(source: usize) -> &'static str {
    match source {
        crate::sensor_ext::HWINFO => {
            "Needs HWiNFO running with its Sensors window open and Shared Memory Support \
             switched on (HWiNFO > Sensors > Settings). Gives CPU package temperature, package \
             power, PL1 / PL2 and the GPU values. A list of what it shares is written to \
             %APPDATA%\\CPUnity-R-Helper\\hwinfo_sensors.txt."
        }
        crate::sensor_ext::LHM => {
            "Needs LibreHardwareMonitor running with Options > Remote Web Server switched on. \
             Gives CPU package temperature and power, plus the GPU values."
        }
        crate::sensor_ext::LOG => {
            "ThrottleStop and Intel XTU have no live interface, only log files. Turn on their CSV \
             logging (or HWiNFO's) and give the path. The newest line is read once a second; \
             columns are recognized by name (temperature, power, PL1, PL2)."
        }
        crate::sensor_ext::PAWN => {
            "Needs the PawnIO driver (pawnio.eu), its signed IntelMSR.bin module and R-Helper \
             started as administrator (see \"Always run as administrator\" above). Reads package \
             and hottest-core temperature, PL1 / PL2 and package power straight from the (Intel) CPU. With \
             Nvidia.bin next to it, also the GPU memory junction temperature. The modules are \
             looked for next to R-Helper and in Program Files\\PawnIO and its modules folder."
        }
        _ => "Windows counters and the NVIDIA driver: always on, nothing to install.",
    }
}

/// A labelled text box for the sensor source. The sensors restart (once) when editing ends, not
/// on every key.
fn source_field(
    ui: &mut egui::Ui,
    res: &mut OptionsResult,
    label: &str,
    text: &mut String,
    hint: &str,
    width: f32,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let edit = ui.add(egui::TextEdit::singleline(text).hint_text(hint).desired_width(width));
        let edited = edit.id.with("edited");
        if edit.changed() {
            res.changed = true;
            ui.data_mut(|d| d.insert_temp(edited, true));
        }
        if edit.lost_focus() && ui.data_mut(|d| d.remove_temp::<bool>(edited)).unwrap_or(false) {
            res.sensor_source_changed = true;
        }
    });
}

/// "Sensor sources" section: which programs the Sensors tab may read from, and where each
/// reading comes from.
fn render_sensor_source(ui: &mut egui::Ui, cfg: &mut Config, res: &mut OptionsResult) {
    use crate::sensor_ext::{Reading, AUTO, BUILT_IN, SOURCE_COUNT, SOURCE_NAMES, SOURCE_TITLES};

    ui.add(
        egui::Label::new(
            egui::RichText::new(
                "Built-in readings are always on. Switch on more sources, then choose below \
                 where each reading comes from. Sources are only read while \"Read sensors\" is on.",
            )
            .small()
            .color(Color32::from_gray(150)),
        )
        .selectable(false),
    );
    for source in 1..SOURCE_COUNT {
        let response =
            crate::theme::toggle(ui, &mut cfg.sensor_enabled[source], SOURCE_TITLES[source])
                .on_hover_text(source_hint(source));
        if response.changed() {
            res.changed = true;
            res.sensor_source_changed = true;
        }
        if cfg.sensor_enabled[source] {
            ui.indent(("sensor_source_settings", source), |ui| match source {
                crate::sensor_ext::LHM => source_field(
                    ui,
                    res,
                    "Address",
                    &mut cfg.sensor_lhm_addr,
                    crate::sensor_ext::DEFAULT_LHM_ADDR,
                    160.0,
                ),
                crate::sensor_ext::LOG => source_field(
                    ui,
                    res,
                    "Log file",
                    &mut cfg.sensor_log_path,
                    "C:\\path\\to\\log.csv",
                    240.0,
                ),
                crate::sensor_ext::PAWN => source_field(
                    ui,
                    res,
                    "IntelMSR.bin",
                    &mut cfg.sensor_module_path,
                    "IntelMSR.bin or its folder (Nvidia.bin next to it); empty = usual places",
                    240.0,
                ),
                _ => {}
            });
        }
    }

    ui.add_space(4.0);
    egui::CollapsingHeader::new("Where each reading comes from").default_open(false).show(
        ui,
        |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Auto takes the first source that has the value: PawnIO, HWiNFO, \
                         LibreHardwareMonitor, log file, then built-in. Only sources that can \
                         provide a reading are listed for it.",
                    )
                    .small()
                    .color(Color32::from_gray(150)),
                )
                .selectable(false),
            );
            let current = cfg.sensor_source_config();
            egui::Grid::new("sensor_pick_grid").num_columns(2).spacing([14.0, 6.0]).show(
                ui,
                |ui| {
                    for reading in Reading::ALL {
                        ui.add(egui::Label::new(reading.label()).selectable(false));
                        let shown = match current.pick(reading) {
                            Some(source) => SOURCE_NAMES[source],
                            None => "Auto",
                        };
                        egui::ComboBox::from_id_salt(("sensor_pick", reading.index()))
                            .selected_text(shown)
                            .width(190.0)
                            .show_ui(ui, |ui| {
                                let slot = &mut cfg.sensor_picks[reading.index()];
                                let mut picked = ui.selectable_value(slot, AUTO, "Auto").changed();
                                for source in 0..SOURCE_COUNT {
                                    let on = source == BUILT_IN || cfg.sensor_enabled[source];
                                    if on && reading.provided_by(source) {
                                        let slot = &mut cfg.sensor_picks[reading.index()];
                                        picked |= ui
                                            .selectable_value(
                                                slot,
                                                source as u8,
                                                SOURCE_NAMES[source],
                                            )
                                            .changed();
                                    }
                                }
                                if picked {
                                    res.changed = true;
                                    res.sensor_source_changed = true;
                                }
                            });
                        ui.end_row();
                    }
                },
            );
        },
    );
}

/// How wide the Options window is, so that every panel in it is as wide as the next.
const PANEL_WIDTH: f32 = 500.0;

/// A back panel for one group of settings: darker than the window, with a border of its own and,
/// when given, a heading. Each group sits on its own, so they are easy to tell apart.
fn panel<R>(ui: &mut egui::Ui, title: Option<&str>, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    // on Platinum the back panels are brushed steel too, painted behind what they hold
    let steel = crate::theme::skin() == crate::theme::Skin::Platinum;
    let slot = ui.painter().add(egui::Shape::Noop);
    let shown = egui::Frame::new()
        .fill(if steel { Color32::TRANSPARENT } else { crate::theme::bg() })
        .stroke(egui::Stroke::new(1.0, crate::theme::border()))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(title) = title {
                ui.label(crate::theme::section_title(title));
                ui.add_space(2.0);
            }
            add_contents(ui)
        });
    if steel {
        ui.painter().set(slot, crate::theme::steel_shape(shown.response.rect, 8.0));
    }
    ui.add_space(6.0);
    shown.inner
}

/// One row of Pairs: the preview box, the name and what it needs, and the button that puts on both.
fn pair_row(ui: &mut egui::Ui, cfg: &mut Config, res: &mut OptionsResult, pair: &crate::pairs::Pair) {
    let missing = crate::pairs::missing(cfg, pair);
    let on = crate::pairs::is_on(cfg, pair);
    ui.horizontal(|ui| {
        crate::ui::preview::mini_toggle(ui, crate::ui::preview::Open::pair(cfg, pair));
        ui.vertical(|ui| {
            ui.set_max_width(250.0);
            ui.label(egui::RichText::new(pair.name).strong());
            let note = if missing.is_empty() { pair.blurb.to_string() } else { format!("Needs {}.", missing.join(" and ")) };
            ui.add(egui::Label::new(egui::RichText::new(note).small().color(Color32::from_gray(140))).wrap().selectable(false));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if on {
                ui.label(egui::RichText::new("On").color(crate::theme::accent_stroke()));
            } else if ui
                .add_enabled(missing.is_empty(), egui::Button::new("Equip both"))
                .on_disabled_hover_text(format!("Needs {}.", missing.join(" and ")))
                .clicked()
            {
                crate::pairs::equip(cfg, pair);
                res.changed = true;
            }
        });
    });
    ui.add_space(2.0);
}

/// Options > UI Options: how the program looks. Three drop-downs: Skins (put on one of the skins that
/// were bought, or none), Pairs (with the earned worlds in a Worlds drop-down) and Backdrop (the picture behind every tab).
fn render_ui_options(ui: &mut egui::Ui, cfg: &mut Config, res: &mut OptionsResult) {
    crate::theme::collapsing(ui, "options_ui", "UI Options", false, |ui| {
        crate::theme::collapsing(ui, "options_ui_skins", "Skins", true, |ui| {
            let looks = crate::pairs::owned_looks(cfg);
            for wear in &looks {
                let wear = *wear;
                ui.horizontal(|ui| {
                    crate::ui::preview::mini_toggle(ui, crate::ui::preview::Open::look(wear));
                    let on = wear.is_on(cfg);
                    if ui.selectable_label(on, wear.name()).on_hover_text(wear.blurb()).clicked() && !on {
                        wear.equip(cfg);
                        res.changed = true;
                    }
                });
            }
            if crate::pairs::Wear::Skin("platinum").is_on(cfg)
                && crate::theme::toggle(ui, &mut cfg.platinum_banner, "Platinum banner")
                    .on_hover_text("The steel banner under the title. Off: the usual header.")
                    .changed()
            {
                res.changed = true;
            }
            if crate::pairs::Wear::Skin("platinum").is_on(cfg)
                && crate::theme::toggle(ui, &mut cfg.platinum_effects, "Platinum light and glints")
                    .on_hover_text("The band of light that sweeps over the window, the glimmering facets and the glints. Off: the look stands still.")
                    .changed()
            {
                res.changed = true;
            }
            let hint = if looks.len() == 1 {
                "No skins yet. The shop sells them, and the worlds you earn show up here."
            } else if !crate::release::story() {
                "Click a box for a preview. Putting a skin on switches the last one off."
            } else {
                "Click a box for a preview. Putting a look on switches the last one off (the earned ones are not lost)."
            };
            ui.add(egui::Label::new(egui::RichText::new(hint).small().color(Color32::from_gray(140))).selectable(false));
        });
        crate::theme::collapsing(ui, "options_ui_pairs", "Pairs", true, |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new("A look and the backdrop made for it, put on together with one click.")
                        .small()
                        .color(Color32::from_gray(140)),
                )
                .selectable(false),
            );
            // the earned worlds sit together in their own drop-down (a build without the story has none)
            if crate::release::story() {
                crate::theme::collapsing(ui, "options_ui_pairs_worlds", "Worlds", true, |ui| {
                    for pair in crate::pairs::PAIRS.iter().filter(|p| crate::pairs::is_world(p)) {
                        pair_row(ui, cfg, res, pair);
                    }
                });
            }
            for pair in crate::pairs::PAIRS.iter().filter(|p| !crate::pairs::is_world(p)) {
                if !crate::pairs::shown(cfg, pair) {
                    continue;
                }
                pair_row(ui, cfg, res, pair);
            }
        });
        crate::theme::collapsing(ui, "options_ui_backdrop", "Backdrop", true, |ui| {
            if crate::theme::toggle(ui, &mut cfg.backdrop, "Backdrop")
                .on_hover_text(
                    "A faint picture in the accent color behind every tab.\n\
                     Off: plain dark background.",
                )
                .changed()
            {
                res.changed = true;
            }
            if cfg.backdrop {
                let look_now = crate::pairs::current(cfg);
                // the angel cats go by another name on Platinum
                let soul = look_now.soul_cats();
                ui.horizontal(|ui| {
                    ui.label("Backdrop style");
                    egui::ComboBox::from_id_salt("backdrop_style")
                        .selected_text(cfg.shown_backdrop_style().label_in(soul))
                        .show_ui(ui, |ui| {
                            let void_on = cfg.void_active();
                            let underworld_on = cfg.underworld_active();
                            let owned: Vec<String> = crate::shop::CATALOG.iter().filter(|i| cfg.shop.owns(i.id)).map(|i| i.id.to_string()).collect();
                            for style in crate::info::BackdropStyle::available_with(cfg.gold_unlocked, cfg.void_unlocked, |id| owned.iter().any(|o| o == id)) {
                                // (the dead king is only offered inside The Void itself)
                                if style.inside_void_only() && !void_on {
                                    continue;
                                }
                                // (and Soul Whispers only inside the Underworld)
                                if style.inside_underworld_only() && !underworld_on {
                                    continue;
                                }
                                ui.horizontal(|ui| {
                                    crate::ui::preview::mini_toggle(ui, crate::ui::preview::Open::backdrop(look_now, style));
                                    if ui.selectable_value(&mut cfg.backdrop_style, style, style.label_in(soul)).changed() {
                                        res.changed = true;
                                    }
                                });
                            }
                        });
                });
                if cfg.backdrop_style.is_animated()
                    && crate::theme::toggle(ui, &mut cfg.backdrop_animated, "Animate backdrop").changed()
                {
                    res.changed = true;
                }
            }
        });
    });
}

pub fn render_options(
    ui: &mut egui::Ui,
    cfg: &mut Config,
    startup: &mut bool,
    _modes: &[PerfMode],
    debug_mode: &mut bool,
    login_task: Option<bool>,
    coin: Option<&egui::TextureHandle>,
) -> OptionsResult {
    let mut res = OptionsResult::default();
    // one width for the whole window, so that every panel is as wide as the next
    ui.set_width(PANEL_WIDTH);

    if crate::release::story() && cfg.shop.visible() {
        panel(ui, None, |ui| {
            let shop = crate::ui::shop::render_shop_section(ui, cfg, coin);
            res.changed |= shop.changed;
            res.shop_bought = shop.bought;
        });
    }
    if crate::release::story() && cfg.trophies.count() > 0 {
        panel(ui, None, |ui| crate::ui::trophies::render_trophies_section(ui, cfg));
    }
    if crate::release::test_tools() {
        panel(ui, None, |ui| render_shop_testing(ui, cfg, &mut res));
        panel(ui, None, |ui| render_trophy_testing(ui, cfg, &mut res));
        panel(ui, None, |ui| render_game_testing(ui, cfg, &mut res));
    }
    panel(ui, None, |ui| render_ui_options(ui, cfg, &mut res));
    if crate::release::story() && cfg.games_section_shown() {
        panel(ui, None, |ui| render_games_section(ui, cfg, &mut res));
    }

    panel(ui, None, |ui| {
        crate::theme::collapsing(ui, "options_appearance", "Appearance", false, |ui| {
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
            ui.separator();
            ui.label("Accent color");
            render_color_section(ui, cfg, &mut res);
        });
    });

    // the three worlds, each its own drop-down under one
    if crate::release::story() {
        panel(ui, None, |ui| {
        crate::theme::collapsing(ui, "options_worlds", "World Controls", false, |ui| {
            render_gold_section(ui, cfg, &mut res);
            // The Void is not mentioned until Gold has been earned (or the Void itself)
            if cfg.gold_unlocked || cfg.void_unlocked {
                ui.add_space(4.0);
                render_void_section(ui, cfg, &mut res);
            }
            // the Underworld is not mentioned until The Void has been earned (or the Underworld itself)
            if cfg.underworld_out() && (cfg.void_unlocked || cfg.games.underworld()) {
                ui.add_space(4.0);
                render_underworld_section(ui, cfg, &mut res);
            }
        });
        });
    }

    // the settings that are set once and left: a panel of their own with one drop-down, and each
    // inside it still on a panel of its own
    panel(ui, None, |ui| {
        crate::theme::collapsing(ui, "options_settings", "Settings", false, |ui| {
            panel(ui, None, |ui| {
                crate::theme::collapsing(ui, "options_tabs", "Tabs", false, |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new("Main is always there. Switch the others off for a plainer window.")
                                .small()
                                .color(Color32::from_gray(140)),
                        )
                        .selectable(false),
                    );
                    for (key, name) in crate::config::TOGGLE_TABS {
                        let mut shown = cfg.tab_shown(key);
                        if crate::theme::toggle(ui, &mut shown, name).changed() {
                            cfg.set_tab_shown(key, shown);
                            res.changed = true;
                        }
                    }
                    // the Trophies tab can be switched on once a trophy has been earned (it is off
                    // until then, and not even listed)
                    if cfg.trophies_tab_available() && crate::theme::toggle(ui, &mut cfg.trophies_tab, "Trophies").changed() {
                        res.changed = true;
                    }
                });
            });

            panel(ui, Some("Window and startup"), |ui| {
                if crate::theme::toggle(ui, &mut cfg.always_on_top, "Keep window on top").changed() {
                    res.changed = true;
                    res.on_top_changed = true;
                }

                if crate::theme::toggle(ui, startup, "Launch at Windows startup").changed() {
                    res.startup_toggled = Some(*startup);
                }

                if crate::theme::toggle(ui, &mut cfg.start_minimized, "Start minimized to tray")
                    .on_hover_text(
                        "On: CR-Helper starts hidden next to the clock instead of opening its window.\n\
                         Click the tray icon to open it; right-click it for Show / Quit.\n\
                         Handy together with \"Launch at Windows startup\". The tray icon stays while\n\
                         CR-Helper runs. Off: the window opens as usual.",
                    )
                    .changed()
                {
                    res.changed = true;
                    res.tray_changed = true;
                }

                if crate::theme::toggle(ui, &mut cfg.close_to_tray, "Close button minimizes to tray")
                    .on_hover_text(
                        "On: the X button hides the window and leaves R-Helper running next to the clock.\n\
                         Click the tray icon to show it again; right-click it for Show / Quit.\n\
                         Off: the X button quits.",
                    )
                    .changed()
                {
                    res.changed = true;
                    res.tray_changed = true;
                }

                if crate::theme::toggle(ui, &mut cfg.run_as_admin, "Always run as administrator")
                    .on_hover_text(
                        "On: R-Helper restarts itself with administrator rights when it starts (Windows asks\n\
                         for permission each time, also with \"Launch at Windows startup\").\n\
                         Needed for the PawnIO source (CPU temperature, power limits, GPU memory\n\
                         temperature) and for some other sensor sources. Off: normal rights.\n\
                         If you decline the permission prompt, R-Helper just keeps running normally.",
                    )
                    .changed()
                {
                    res.changed = true;
                }
                ui.horizontal(|ui| {
                    let elevated = crate::admin::is_elevated();
                    let text = if elevated {
                        "Running as administrator now."
                    } else {
                        "Not running as administrator now."
                    };
                    ui.add(egui::Label::new(egui::RichText::new(text).small().weak()).selectable(false));
                    if !elevated && ui.small_button("Restart as administrator now").clicked() {
                        res.admin_restart = true;
                    }
                });
                if cfg.run_as_admin {
                    crate::theme::collapsing(
                        ui,
                        "admin_login",
                        "Skip the permission prompt at login",
                        true,
                        |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(
                                        "Windows asks for permission each time R-Helper starts as \
                                     administrator. A scheduled task can start it at login with the same \
                                     rights and no prompt. Nothing is changed until you confirm in the \
                                     next window.",
                                    )
                                    .small()
                                    .weak(),
                                )
                                .wrap()
                                .selectable(false),
                            );
                            let present = login_task == Some(true);
                            let status = match login_task {
                                Some(true) => "Set up: R-Helper starts at login without a prompt.",
                                Some(false) => "Not set up.",
                                None => "Checking...",
                            };
                            ui.horizontal(|ui| {
                                ui.add(egui::Label::new(egui::RichText::new(status).small()).selectable(false));
                                let label = if present { "Set up again..." } else { "Set up..." };
                                if ui.small_button(label).clicked() {
                                    res.login_task_setup = true;
                                }
                                if present
                                    && ui
                                        .add_enabled(
                                            crate::admin::is_elevated(),
                                            egui::Button::new("Remove").small(),
                                        )
                                        .on_disabled_hover_text("Removing it needs administrator rights.")
                                        .clicked()
                                {
                                    res.login_task_remove = true;
                                }
                            });
                        },
                    );
                }

            });

            panel(ui, Some("Sensor sources"), |ui| render_sensor_source(ui, cfg, &mut res));
        });
    });

    panel(ui, None, |ui| {
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
            crate::theme::toggle(ui, debug_mode, "🐛 Debug mode");
            ui.add_space(4.0);
            ui.add(
                egui::Label::new(egui::RichText::new("Keyboard effect tuning").strong())
                    .selectable(false),
            );
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Only if an effect ignores the color or looks wrong. Click the effect again \
                         after changing one of these.",
                    )
                    .small()
                    .weak(),
                )
                .selectable(false),
            );
            for (flag, text, tip) in [
                (
                    &mut cfg.kbd_static_frame,
                    "Paint Static as a custom frame",
                    "On (default): Static fills every key with the color. Off: use the firmware's Static effect, which ignores the color on some laptops.",
                ),
                (
                    &mut cfg.kbd_legacy,
                    "Use the older command set (0x030A)",
                    "Send effects with the older standard matrix effect command.",
                ),
                (
                    &mut cfg.kbd_random_id,
                    "Use a random transaction id",
                    "Off (default): fixed id 0x1F, which newer Razer firmware expects for lighting.",
                ),
                (
                    &mut cfg.kbd_clear_first,
                    "Clear the keyboard before Static / Breathing",
                    "On (default): sends an Off effect first, for firmware that ignores a new color while the same effect is showing.",
                ),
            ] {
                if crate::theme::toggle(ui, flag, text).on_hover_text(tip).changed() {
                    res.changed = true;
                }
            }
        });
    });

    // the preview that was opened with one of the tiny boxes
    crate::ui::preview::render_window(ui.ctx(), ui.clip_rect().right_top() + egui::vec2(12.0, 0.0));

    res
}

/// The AC / battery profile, a card of the main tab (under Razer Synapse Standby).
pub fn render_power_profile(ui: &mut egui::Ui, cfg: &mut Config, modes: &[PerfMode]) -> OptionsResult {
    let mut res = OptionsResult::default();
    crate::theme::card(ui, |ui| {
        crate::theme::collapsing(ui, "power_profile_card", crate::theme::section_title("\u{26a1} AC / battery profile"), true, |ui| {
            power_profile_body(ui, cfg, modes, &mut res);
        });
    });
    res
}

fn power_profile_body(ui: &mut egui::Ui, cfg: &mut Config, modes: &[PerfMode], res: &mut OptionsResult) {
        ui.add(egui::Label::new(egui::RichText::new("At launch and on power change.").small().color(Color32::from_gray(150))).selectable(false));
        // ---- on AC power
        let mut ac_changed = false;
        ui.add_enabled_ui(!cfg.ac_last, |ui| {
            ac_changed = mode_combo(ui, "ac_mode_combo", "On AC power:", &mut cfg.ac_mode, modes);
        });
        if !cfg.ac_last && cfg.ac_mode.as_deref() == Some("Custom") {
            ui.indent("ac_custom_boosts", |ui| {
                if boost_combo(ui, "ac_cpu_combo", "CPU:", &mut cfg.ac_cpu, &CPU_BOOSTS) {
                    ac_changed = true;
                }
                if boost_combo(ui, "ac_gpu_combo", "GPU:", &mut cfg.ac_gpu, &GPU_BOOSTS) {
                    ac_changed = true;
                }
            });
        }
        if last_known_toggle(ui, &mut cfg.ac_last, &mut cfg.ac_last_state, "Last Known Settings on AC power") {
            ac_changed = true;
        }
        if ac_changed {
            res.changed = true;
            res.modes_changed = true;
        }
        ui.add_space(4.0);
        // ---- on battery
        let mut bat_changed = false;
        ui.add_enabled_ui(!cfg.battery_last, |ui| {
            bat_changed = mode_combo(ui, "bat_mode_combo", "On battery:", &mut cfg.battery_mode, modes);
        });
        if !cfg.battery_last && cfg.battery_mode.as_deref() == Some("Custom") {
            ui.indent("bat_custom_boosts", |ui| {
                if boost_combo(ui, "bat_cpu_combo", "CPU:", &mut cfg.battery_cpu, &CPU_BOOSTS) {
                    bat_changed = true;
                }
                if boost_combo(ui, "bat_gpu_combo", "GPU:", &mut cfg.battery_gpu, &GPU_BOOSTS) {
                    bat_changed = true;
                }
            });
        }
        if last_known_toggle(ui, &mut cfg.battery_last, &mut cfg.battery_last_state, "Last Known Settings on battery") {
            bat_changed = true;
        }
        if bat_changed {
            res.changed = true;
            res.modes_changed = true;
        }
        ui.add_space(2.0);
        if ui.button("Apply current power profile now").clicked() {
            res.apply_now = true;
        }
        crate::ui::help::link(ui, "power_profile");
}

const CPU_BOOSTS: [&str; 5] = ["Low", "Medium", "High", "Boost", "Overclock"];
const GPU_BOOSTS: [&str; 3] = ["Low", "Medium", "High"];

/// A drop-down for the CPU or GPU level of a Custom profile. `None` is Default (the levels last
/// picked on the main page). Returns true when the choice changed.
fn boost_combo(ui: &mut egui::Ui, id: &str, label: &str, value: &mut Option<String>, names: &[&str]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let current = value.clone().unwrap_or_else(|| "Default".to_string());
        egui::ComboBox::from_id_salt(id).selected_text(current).show_ui(ui, |ui| {
            if ui.selectable_label(value.is_none(), "Default").clicked() {
                *value = None;
                changed = true;
            }
            for n in names {
                let selected = value.as_deref() == Some(*n);
                if ui.selectable_label(selected, *n).clicked() {
                    *value = Some((*n).to_string());
                    changed = true;
                }
            }
        });
    });
    changed
}

/// The "Last Known Settings" switch of one power source. Switching it clears what was saved, so it
/// starts again from what the laptop is running then. Returns true when it was switched.
fn last_known_toggle(ui: &mut egui::Ui, on: &mut bool, saved: &mut Option<String>, label: &str) -> bool {
    let changed = crate::theme::toggle(ui, on, label).changed();
    if changed {
        *saved = None;
    }
    changed
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
        let rows: [(&str, &str); 11] = [
            ("Fatalution", "R-Helper, the app this is built on"),
            ("blauzim & tdakhran", "razer-ctl / librazer, the device layer"),
            ("Razer-Linux laptop-control contributors", "protocol groundwork"),
            ("NxRoot", "SynapseKiller, the idea behind Synapse Standby"),
            ("Robak08", "Razer Laptop Cooling Pad support, the groundwork for the pad card"),
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

/// Options > Shop testing: for trying the shop out. Edit the CR-Token balance, take back what was
/// bought, empty the balance, give and take trophies by hand, or start the wallet over as if nothing had ever been earned. It is
/// always there, even before the first token.
fn render_shop_testing(ui: &mut egui::Ui, cfg: &mut Config, res: &mut OptionsResult) {
    crate::theme::collapsing(ui, "options_shop_testing", "Shop testing", false, |ui| {
        ui.add(
            egui::Label::new(
                egui::RichText::new("Test tools. They change the saved tokens and purchases straight away.")
                    .small()
                    .color(Color32::from_gray(140)),
            )
            .selectable(false),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("Balance");
            let mut balance = cfg.shop.balance();
            if ui.add(egui::DragValue::new(&mut balance).range(0..=1_000_000).speed(1.0)).changed() {
                cfg.shop.set_balance(balance);
                res.changed = true;
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button("Reset purchases").on_hover_text("Takes back everything bought and the skin that was on. No refund.").clicked() {
                cfg.shop.reset_purchases();
                cfg.shop_preview = None;
                res.changed = true;
            }
            if ui.button("Reset balance").on_hover_text("Sets the balance to 0. Purchases stay.").clicked() {
                cfg.shop.reset_balance();
                res.changed = true;
            }
            if ui
                .button("Reset everything")
                .on_hover_text("Tokens, purchases and first-win bonuses: as if no token had ever been earned. The shop hides again until the next one.")
                .clicked()
            {
                cfg.shop.reset_all();
                cfg.shop_preview = None;
                res.changed = true;
            }
        });
    });
}

/// Options > Game testing: an invincibility switch for Games 1 and 2 (never saved, and a run with it pays no
/// tokens), and buttons that go straight to Game 1's part two or to Game 2.
fn render_game_testing(ui: &mut egui::Ui, cfg: &mut Config, res: &mut OptionsResult) {
    crate::theme::collapsing(ui, "options_game_testing", "Game testing".to_string(), false, |ui| {
        if crate::theme::toggle(ui, &mut cfg.dev_invincible, "Invincible (Games 1 and 2)")
            .on_hover_text("Test switch. Nothing costs a life; you still flash when hit. A run with it pays no tokens. It is off every time the program starts.")
            .changed()
        {
            res.changed = true;
        }
        if crate::theme::toggle(ui, &mut cfg.dev_show_underworld, "Show the Underworld")
            .on_hover_text("Test switch. This build holds the Underworld back (its look, the Devil's Table, its trophies). Switch it on to work on it. Saved.")
            .changed()
        {
            res.changed = true;
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .button("Game 1: skip to part two")
                .on_hover_text("Starts Game 1 at its second part (the one after the checkpoint). The saved checkpoint is not changed.")
                .clicked()
            {
                res.game1_part2 = true;
            }
            if ui
                .button("Talk to the cat")
                .on_hover_text("The talk before Game 1 (the cat before the crown). Saying yes starts Game 1 and counts as helping him; no changes nothing but how he greets you next time.")
                .clicked()
            {
                res.cat_talk = true;
            }
            if ui
                .button("Reset the cat")
                .on_hover_text("As if you had never spoken to him: he greets you for the first time again, and you are no longer counted as having helped or turned him down. Game 1's checkpoint is not changed.")
                .clicked()
            {
                cfg.games.reset_cat();
                res.changed = true;
            }
            if ui.button("Enter the Ally").on_hover_text("Goes into the alley whether or not you have said yes to the cat, keeping its progress.").clicked() {
                res.ally_enter = true;
            }
            if ui.button("Reset the Ally").on_hover_text("Forgets everything in the alley (lamps, hands, key, the friend). Whether you helped the cat is not changed.").clicked() {
                cfg.games.ally_mut().reset();
                res.changed = true;
            }
            if ui.button("Game 2: start now").on_hover_text("Skips straight to Game 2 (the staff).").clicked() {
                res.game2_start = true;
            }
        });
    });
}

/// Options > Trophy testing (under Shop testing, always there): give and take trophies one at a time, to see how they and their
/// announcement look. With "Earn trophies by playing" on, a trophy that the game already satisfies
/// comes straight back after it is taken; switch it off to hold them where you put them.
fn render_trophy_testing(ui: &mut egui::Ui, cfg: &mut Config, res: &mut OptionsResult) {
    let (earned, all) = cfg.trophies.progress();
    crate::theme::collapsing(ui, "options_trophy_testing", format!("Trophy testing  {earned} / {all}"), false, |ui| {
        let mut auto = !cfg.trophies.manual();
        if crate::theme::toggle(ui, &mut auto, "Earn trophies by playing")
            .on_hover_text("On: trophies are earned as the game allows. Off: only the buttons below give or take them, so a test stays as you set it.")
            .changed()
        {
            cfg.trophies.set_manual(!auto);
            res.changed = true;
        }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        ui.horizontal_wrapped(|ui| {
            if ui.button("Give all").on_hover_text("Gives every trophy, one announcement each, and the Platinum skin.").clicked() {
                let held = cfg.trophies.holds_underworld();
                for t in crate::trophies::TROPHIES.iter().filter(|t| t.id != crate::trophies::PLATINUM && !crate::release::trophy_held(t.id, held)).chain(crate::trophies::TROPHIES.iter().filter(|t| t.id == crate::trophies::PLATINUM)) {
                    if let Some(new) = cfg.give_trophy(t.id, now) {
                        res.trophies_given.push(new.id);
                    }
                }
                res.changed = true;
            }
            if ui.button("Take all").on_hover_text("Takes every trophy away again, and the Platinum skin with it.").clicked() {
                cfg.trophies.clear();
                cfg.shop.revoke("platinum");
                cfg.shop_preview = None;
                res.changed = true;
            }
        });
        let held = cfg.trophies.holds_underworld();
        for t in crate::trophies::TROPHIES.iter().filter(|t| !crate::release::trophy_held(t.id, held)) {
            let has = cfg.trophies.has(t.id);
            ui.horizontal(|ui| {
                let label = if t.hidden { format!("{} (hidden)", t.name) } else { t.name.to_string() };
                ui.add(egui::Label::new(egui::RichText::new(label).color(if has { crate::theme::text() } else { crate::theme::text_dim() })).selectable(false));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if has {
                        if ui.button("Take").clicked() {
                            cfg.take_trophy(t.id);
                            res.changed = true;
                        }
                    } else if ui.button("Give").clicked() {
                        if let Some(new) = cfg.give_trophy(t.id, now) {
                            res.trophies_given.push(new.id);
                        }
                        res.changed = true;
                    }
                });
            });
        }
    });
}

/// Options > Games, at the very top: how hard the hidden game of the look you are in is. It is not
/// there at all until that game has killed you 10 times; then it shows the death total and Hard,
/// with Normal and Easy joining at 20 and 35 deaths (see `games.rs`). It only has that one game's
/// setting.
fn render_games_section(ui: &mut egui::Ui, cfg: &mut Config, res: &mut OptionsResult) {
    if !cfg.games_section_shown() {
        return;
    }
    let game = cfg.edition().game();
    crate::theme::collapsing(ui, "options_games", "Games", true, |ui| {
        ui.add(egui::Label::new(egui::RichText::new(crate::games::game_name(game)).strong()).selectable(false));
        ui.add(
            egui::Label::new(
                egui::RichText::new(format!("Deaths, all time: {}", cfg.games.deaths(game))).small().color(Color32::from_gray(140)),
            )
            .selectable(false),
        );
        ui.horizontal(|ui| {
            for d in cfg.games.available(game) {
                let on = cfg.games.difficulty(game) == d;
                if ui.selectable_label(on, d.label()).clicked() && !on {
                    cfg.games.set_difficulty(game, d);
                    res.changed = true;
                }
            }
        });
        let text = crate::games::describe(game, cfg.games.difficulty(game));
        ui.add(egui::Label::new(egui::RichText::new(text).small().color(Color32::from_gray(140))).selectable(false));
        // the cheats this game has earned (50, 75 or 100 deaths), each a switch that starts off
        let cheats = cfg.games.cheats_unlocked(game);
        if !cheats.is_empty() {
            ui.add_space(4.0);
            ui.add(egui::Label::new(egui::RichText::new("Cheats").strong()).selectable(false));
            for c in cheats {
                let mut on = cfg.games.cheat_on(c);
                if crate::theme::toggle(ui, &mut on, c.label()).on_hover_text(c.blurb()).changed() {
                    cfg.games.set_cheat(c, on);
                    res.changed = true;
                }
                ui.add(egui::Label::new(egui::RichText::new(c.blurb()).small().color(Color32::from_gray(140))).selectable(false));
            }
        }
        // the rhythm game's sound: how loud, and how late the speakers are
        if game == 2 {
            ui.add_space(4.0);
            ui.add(egui::Label::new(egui::RichText::new("Sound").strong()).selectable(false));
            let mut volume = cfg.games.volume() as i32;
            if ui.add(egui::Slider::new(&mut volume, 0..=100).text("Volume")).changed() {
                cfg.games.set_volume(volume.clamp(0, 100) as u8);
                res.changed = true;
            }
            let mut delay = cfg.games.latency_ms();
            if ui
                .add(egui::Slider::new(&mut delay, -150..=150).suffix(" ms").text("Audio delay"))
                .on_hover_text("If you keep hitting the notes late (Bluetooth speakers, say), raise this; if early, lower it.")
                .changed()
            {
                cfg.games.set_latency_ms(delay);
                res.changed = true;
            }
        }
    });
}

/// Options > Gold Edition: locked until earned (beating Father Time), then switches for each part.
fn render_gold_section(ui: &mut egui::Ui, cfg: &mut Config, res: &mut OptionsResult) {
    crate::theme::collapsing(ui, "options_gold", "Gold Edition", cfg.gold_unlocked, |ui| {
        if !cfg.gold_unlocked {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Locked. A golden look for the whole program, earned by beating Father Time.",
                    )
                    .small()
                    .color(Color32::from_gray(140)),
                )
                .selectable(false),
            );
            if ui
                .button("Preview Gold Edition")
                .on_hover_text("Unlocks it now so you can see it. (The game that earns it is not in yet.)")
                .clicked()
            {
                cfg.gold_unlocked = true;
                cfg.gold_on = true;
                res.changed = true;
                res.gold_preview = true;
            }
            return;
        }
        if crate::theme::toggle(ui, &mut cfg.gold_on, "Use Gold Edition").changed() {
            res.changed = true;
            if cfg.gold_on {
                // one look at a time
                cfg.void_on = false;
            }
        }
        if cfg.gold_on {
            if crate::theme::toggle(ui, &mut cfg.gold_banner, "Gold banner").changed() {
                res.changed = true;
            }
            if crate::theme::toggle(ui, &mut cfg.gold_sparkles, "Sun rays and sparkles").changed() {
                res.changed = true;
            }
            if crate::theme::toggle(ui, &mut cfg.gold_glitch, "Glitch when the king laughs")
                .on_hover_text("The window breaks into pixels for a second. Off: no glitch.")
                .changed()
            {
                res.changed = true;
            }
            ui.horizontal(|ui| {
                if ui.button("Make the king laugh").clicked() {
                    res.gold_preview = true;
                }
                if ui.button("Lock Gold Edition again").clicked() {
                    cfg.lock_gold();
                    res.changed = true;
                }
            });
        }
    });
}

/// Options > The Void: locked until Game 2 is won (the cat king), then switches for each part. It
/// takes the place of the Gold Edition while it is on.
fn render_underworld_section(ui: &mut egui::Ui, cfg: &mut Config, res: &mut OptionsResult) {
    let open = cfg.games.underworld();
    crate::theme::collapsing(ui, "options_underworld", "The Underworld", open, |ui| {
        if !open {
            ui.add(
                egui::Label::new(
                    egui::RichText::new("Locked. Something below is waiting.").small().color(Color32::from_gray(140)),
                )
                .selectable(false),
            );
            if ui
                .button("Preview the Underworld")
                .on_hover_text("Opens it now so you can see it, without playing for it.")
                .clicked()
            {
                cfg.games.open_underworld();
                cfg.underworld_on = true;
                res.changed = true;
            }
            return;
        }
        let now = crate::poker_save::unix_now();
        let locked = cfg.poker.locked(now);
        if locked {
            let left = crate::poker_save::left_text(cfg.poker.remaining(now));
            ui.add(
                egui::Label::new(egui::RichText::new(crate::poker_story::locked_banner(&left)).color(Color32::from_rgb(255, 150, 110)))
                    .selectable(false),
            );
        }
        let toggled = ui
            .add_enabled_ui(!locked, |ui| {
                crate::theme::toggle(ui, &mut cfg.underworld_on, "Use the Underworld look").on_hover_text(
                    "Rock slabs lying on moving lava, with souls rising and a line of skeletons walking by.\n\
                     It takes the place of The Void and the Gold Edition while it is on.\n\
                     The backdrop animation switch also stops its movement.",
                )
            })
            .inner
            .changed();
        if toggled {
            res.changed = true;
        }
        // the Devil's Table: Texas Hold'em against the ones down here
        ui.add_space(4.0);
        let cost = crate::poker_game::BUY_IN;
        if ui
            .add_enabled(!locked, egui::Button::new(format!("Sit at the Devil's Table ({cost} CR-Tokens)")))
            .on_hover_text(if locked {
                "Closed while your soul is held."
            } else {
                "Texas Hold'em against the dead King, the Demon Bear, Father Time and, sooner or later, the Shadow Monarch. First place pays 80, second 20."
            })
            .clicked()
        {
            res.poker_sit = true;
        }
        ui.add(
            egui::Label::new(
                egui::RichText::new(format!(
                    "Games played {} · won {} · souls taken {}",
                    cfg.poker.played(),
                    cfg.poker.won(),
                    cfg.poker.takes()
                ))
                .small()
                .color(Color32::from_gray(140)),
            )
            .selectable(false),
        );
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::Label::new(egui::RichText::new("Testing:").small().color(Color32::from_gray(140))).selectable(false));
            if ui.add_enabled(!locked, egui::Button::new("Sit (free)")).on_hover_text("A seat that costs nothing and still counts: the Monarch can still take your soul.").clicked() {
                res.poker_test = true;
            }
            if ui.add_enabled(!locked, egui::Button::new("Lock my soul (2 min)")).on_hover_text("Holds the Underworld still for two minutes, as if the Monarch had won.").clicked() {
                cfg.poker.lock_for_test(now);
                res.changed = true;
            }
            if ui.add_enabled(locked, egui::Button::new("Free my soul")).on_hover_text("Ends the lock now; the break-free scene plays.").clicked() {
                cfg.poker.free();
                res.changed = true;
            }
        });
        if cfg.underworld_active() {
            ui.add(
                egui::Label::new(
                    egui::RichText::new("Backdrop style > Soul Whispers: souls that come to your cursor and whisper (or put it on with Pairs).")
                        .small()
                        .color(Color32::from_gray(140)),
                )
                .selectable(false),
            );
        }
    });
}

fn render_void_section(ui: &mut egui::Ui, cfg: &mut Config, res: &mut OptionsResult) {
    crate::theme::collapsing(ui, "options_void", "The Void", cfg.void_unlocked, |ui| {
        if !cfg.void_unlocked {
            ui.add(
                egui::Label::new(
                    egui::RichText::new("Locked. Something in the sky is holding it.")
                        .small()
                        .color(Color32::from_gray(140)),
                )
                .selectable(false),
            );
            if ui
                .button("Preview The Void")
                .on_hover_text("Unlocks it now so you can see it, without playing for it.")
                .clicked()
            {
                cfg.unlock_void_by_winning();
                res.changed = true;
            }
            return;
        }
        if crate::theme::toggle(ui, &mut cfg.void_on, "Use The Void").changed() {
            res.changed = true;
            if cfg.void_on {
                cfg.gold_on = false;
                cfg.backdrop = true;
                cfg.backdrop_style = crate::info::BackdropStyle::VoidStars;
            } else {
                if cfg.backdrop_style.inside_void_only() {
                    cfg.backdrop_style = crate::info::BackdropStyle::default();
                }
                if cfg.gold_unlocked {
                    cfg.gold_on = true;
                }
            }
        }
        if cfg.void_on {
            if crate::theme::toggle(ui, &mut cfg.void_banner, "Torn banner").changed() {
                res.changed = true;
            }
            if crate::theme::toggle(ui, &mut cfg.void_script, "Script on the Info tab")
                .on_hover_text("The Info tab is written in the Void's own signs; hover a word to read it. Off: the plain text.")
                .changed()
            {
                res.changed = true;
            }
            if crate::theme::toggle(ui, &mut cfg.void_pretend_night, "Moon: pretend it is night")
                .on_hover_text("Test switch. The moon in the corner of the glitching stars only comes out from 7pm to 5am; this shows it all day.")
                .changed()
            {
                res.changed = true;
            }
            if cfg.void_staff_broken {
                if ui
                    .button("Mend the staff")
                    .on_hover_text("The moon has struck it. This puts it back, and the moon comes back with it.")
                    .clicked()
                {
                    cfg.void_staff_broken = false;
                    res.changed = true;
                }
            } else if ui
                .button("Break the staff")
                .on_hover_text("Skips the moon and goes straight to the game. Mend the staff puts everything back.")
                .clicked()
            {
                cfg.void_staff_broken = true;
                res.changed = true;
                res.break_staff = true;
            }
            if crate::release::test_tools()
                && ui
                    .button("Take life")
                    .on_hover_text("Test switch, to go: skips to the end of the second song, so the ending plays and the rest opens.")
                    .clicked()
            {
                cfg.void_staff_broken = true;
                res.changed = true;
                res.take_life = true;
            }
            ui.add(
                egui::Label::new(
                    egui::RichText::new("The glitching stars are in Backdrop style above.")
                        .small()
                        .color(Color32::from_gray(140)),
                )
                .selectable(false),
            );
        }
        if ui.button("Lock The Void again").clicked() {
            cfg.lock_void();
            res.changed = true;
        }
    });
}
