// Sensors tab: opt-in live readings with current / average / maximum.
use crate::log_plan::{Plan, Start};
use crate::sensor_ext::{Link, SOURCE_COUNT, SOURCE_NAMES};
use crate::sensor_history::History;
use crate::sensors::{
    LogInfo, Readings, Stat, DEFAULT_INTERVAL_MS, MAX_INTERVAL_MS, MIN_INTERVAL_MS,
};
use eframe::egui::{self, RichText};

/// What the user did in the sensors card this frame.
#[derive(Debug, Default, PartialEq)]
pub struct SensorsAction {
    /// The reading switch was flipped.
    pub toggled: Option<bool>,
    /// "Reset" (averages and maximums) was clicked.
    pub reset: bool,
    /// The logging switch was flipped.
    pub logging: Option<bool>,
    /// A different graph time span was picked.
    pub window_changed: bool,
    /// "Open folder" was clicked.
    pub open_logs: bool,
    /// The log file name was edited (the new text).
    pub log_name: Option<String>,
    /// Editing the name is finished: save it.
    pub log_name_commit: bool,
    /// What was clicked in the Log viewer (inside the Logging card).
    pub log_view: crate::ui::log_view::LogViewAction,
    /// When the log starts and stops was changed (the new settings; apply them now).
    pub log_settings: Option<LogSettings>,
    /// The new settings are final: save them.
    pub log_settings_commit: bool,
    /// "View" was clicked on the log being written or the last one finished: open it in the viewer.
    pub view_log: Option<std::path::PathBuf>,
    /// The list of running programs was opened: look them up.
    pub refresh_programs: bool,
    /// The graphs switch was flipped.
    pub graphs: Option<bool>,
    /// The refresh interval (milliseconds) was changed; apply it now.
    pub interval: Option<u64>,
    /// The new interval is final (slider released, or "Default" clicked): save it.
    pub interval_commit: bool,
}

/// What the Sensors page shows besides its own switches.
pub struct SensorsView<'a> {
    pub readings: &'a Readings,
    pub history: &'a History,
    pub log: &'a LogInfo,
    /// The programs running now, for the picker of the program that starts a log.
    pub programs: &'a [String],
}

/// When a log starts and stops, as the Logging card edits it (see `log_plan`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogSettings {
    pub start: Start,
    /// As typed.
    pub program: String,
    pub limit_secs: u64,
    pub stop_with_program: bool,
    /// The saved file names (name presets).
    pub name_presets: Vec<String>,
    /// The built-in file name preset (Date, Time, Date and time, None).
    pub stamp: crate::sensor_log::Stamp,
}

impl LogSettings {
    pub fn of(cfg: &crate::config::Config) -> LogSettings {
        LogSettings {
            start: cfg.sensor_log_start,
            program: cfg.sensor_log_program.clone(),
            limit_secs: cfg.sensor_log_limit_secs,
            stop_with_program: cfg.sensor_log_stop_with_program,
            name_presets: cfg.sensor_log_name_presets.clone(),
            stamp: cfg.sensor_log_stamp,
        }
    }

    pub fn apply(&self, cfg: &mut crate::config::Config) {
        cfg.sensor_log_start = self.start;
        cfg.sensor_log_program = self.program.clone();
        cfg.sensor_log_limit_secs = self.limit_secs;
        cfg.sensor_log_stop_with_program = self.stop_with_program;
        cfg.sensor_log_name_presets = self.name_presets.clone();
        cfg.sensor_log_stamp = self.stamp;
    }

    pub fn plan(&self) -> Plan {
        Plan {
            start: self.start,
            program: crate::log_plan::clean_program(&self.program),
            limit_secs: self.limit_secs,
            stop_with_program: self.stop_with_program,
            stamp: self.stamp,
        }
    }
}

fn show(value: Option<f32>, decimals: usize, unit: &str, missing: &str) -> String {
    match value {
        Some(v) => format!("{:.*} {}", decimals, v, unit),
        None => missing.to_string(),
    }
}

/// One table row. A value that has not shown up in the first few readings is "n/a" (this
/// machine does not provide it); "--" is a reading that is just missing right now.
fn row(ui: &mut egui::Ui, label: &str, stat: &Stat, samples: u64, decimals: usize, unit: &str) {
    let missing = if !stat.seen() && samples >= 3 { "n/a" } else { "--" };
    ui.add(egui::Label::new(label).selectable(false));
    ui.add(
        egui::Label::new(RichText::new(show(stat.now, decimals, unit, missing)).strong())
            .selectable(false),
    );
    ui.add(egui::Label::new(show(stat.avg, decimals, unit, missing)).selectable(false));
    ui.add(egui::Label::new(show(stat.max, decimals, unit, missing)).selectable(false));
    ui.end_row();
}

/// The heading of a group of rows. Clicking it folds the group away (remembered between
/// launches under `key`); returns whether the rows are shown.
fn header(ui: &mut egui::Ui, title: &str, key: &str) -> bool {
    let open = crate::theme::get_open(key).unwrap_or(true);
    let mut clicked = false;
    ui.horizontal(|ui| {
        let (arrow, response) =
            ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::click());
        let colour = crate::theme::accent_stroke();
        let c = arrow.center();
        let points = if open {
            vec![c + egui::vec2(-4.5, -2.5), c + egui::vec2(4.5, -2.5), c + egui::vec2(0.0, 3.5)]
        } else {
            vec![c + egui::vec2(-2.5, -4.5), c + egui::vec2(-2.5, 4.5), c + egui::vec2(3.5, 0.0)]
        };
        ui.painter().add(egui::Shape::convex_polygon(points, colour, egui::Stroke::NONE));
        let label = ui
            .add(
                egui::Label::new(RichText::new(title).color(colour).strong())
                    .selectable(false)
                    .sense(egui::Sense::click()),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        clicked = response.clicked() || label.clicked();
    });
    if clicked {
        crate::theme::set_open(key, !open);
    }
    for name in ["Now", "Average", "Maximum"] {
        let text = if open { RichText::new(name).weak() } else { RichText::new("") };
        ui.add(egui::Label::new(text).selectable(false));
    }
    ui.end_row();
    open
}

/// Renders the sensors card.
///
/// * `on` - the reading switch (nothing is read while it is off)
pub fn render_sensors_section(
    ui: &mut egui::Ui,
    on: &mut bool,
    logging: &mut bool,
    log_name: &mut String,
    settings: &mut LogSettings,
    graphs: &mut bool,
    window: &mut u8,
    interval_ms: &mut u64,
    log_view: &mut crate::ui::log_view::LogView,
    picking: bool,
    view: &SensorsView,
) -> SensorsAction {
    let mut action = SensorsAction::default();
    let readings = view.readings;

    crate::theme::card(ui, |ui| {
        ui.add(egui::Label::new(crate::theme::section_title("Sensors")).selectable(false));
        ui.separator();

        let response = crate::theme::toggle(ui, on, "Read sensors").on_hover_text(
            "Off: nothing is read and no background work is done.\n\
             On: the readings below are taken every refresh interval (set below), from the\n\
             moment you switch this on (they keep running while you look at the other tabs).",
        );
        if response.changed() {
            action.toggled = Some(*on);
        }

        // ---- refresh interval ----
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Refresh every").selectable(false));
            let mut seconds = *interval_ms as f32 / 1000.0;
            let slider = ui
                .add(
                    egui::Slider::new(
                        &mut seconds,
                        MIN_INTERVAL_MS as f32 / 1000.0..=MAX_INTERVAL_MS as f32 / 1000.0,
                    )
                    .step_by(0.25)
                    .fixed_decimals(2)
                    .suffix(" s"),
                )
                .on_hover_text(
                    "How often the sensors are read. Faster gives finer graphs and a quicker \
                     response; slower uses less CPU. Applies immediately.",
                );
            if slider.changed() {
                *interval_ms =
                    ((seconds * 1000.0).round() as u64).clamp(MIN_INTERVAL_MS, MAX_INTERVAL_MS);
                action.interval = Some(*interval_ms);
                action.interval_commit = !slider.dragged();
            }
            if slider.drag_stopped() {
                action.interval_commit = true;
            }
            let is_default = *interval_ms == DEFAULT_INTERVAL_MS;
            if ui
                .add_enabled(!is_default, egui::Button::new("Default"))
                .on_hover_text("Back to the factory setting (1 second)")
                .clicked()
            {
                *interval_ms = DEFAULT_INTERVAL_MS;
                action.interval = Some(DEFAULT_INTERVAL_MS);
                action.interval_commit = true;
            }
        });

        if !*on && readings.samples == 0 {
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "Switch it on to start reading. Averages and maximums count from then on.",
                    )
                    .small()
                    .weak(),
                )
                .selectable(false),
            );
            return;
        }

        ui.add_space(4.0);
        egui::Grid::new("sensors_grid").num_columns(4).spacing([18.0, 6.0]).striped(true).show(
            ui,
            |ui| {
                let n = readings.samples;
                if header(ui, "CPU", "sensors_cpu") {
                    row(ui, "Package temperature", &readings.cpu_temp, n, 0, "\u{b0}C");
                    row(ui, "Core temperature (hottest)", &readings.core_temp, n, 0, "\u{b0}C");
                    row(ui, "PL1 (long term limit)", &readings.pl1, n, 0, "W");
                    row(ui, "PL2 (short term limit)", &readings.pl2, n, 0, "W");
                    row(ui, "Package power", &readings.cpu_power, n, 1, "W");
                    row(ui, "Clock (all threads)", &readings.cpu_clock, n, 0, "MHz");
                    row(ui, "Clock (fastest core)", &readings.cpu_clock_top, n, 0, "MHz");
                    row(ui, "Clock (P-cores average)", &readings.cpu_clock_p, n, 0, "MHz");
                    row(ui, "Clock (E-cores average)", &readings.cpu_clock_e, n, 0, "MHz");
                    row(ui, "Load", &readings.cpu_load, n, 0, "%");
                    row(ui, "Hottest thermal zone", &readings.cpu_zone_temp, n, 0, "\u{b0}C");
                }

                if header(ui, "GPU", "sensors_gpu") {
                    row(ui, "Temperature", &readings.gpu_temp, n, 0, "\u{b0}C");
                    row(ui, "Memory junction", &readings.gpu_mem_temp, n, 0, "\u{b0}C");
                    row(ui, "Power", &readings.gpu_power, n, 1, "W");
                    row(ui, "Clock", &readings.gpu_clock, n, 0, "MHz");
                    row(ui, "Load", &readings.gpu_load, n, 0, "%");
                }
            },
        );

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("Reset average / max").clicked() {
                action.reset = true;
            }
            let state = if *on {
                format!("{} readings", readings.samples)
            } else {
                format!("Stopped after {} readings", readings.samples)
            };
            ui.add(egui::Label::new(RichText::new(state).small().weak()).selectable(false));
        });
        // where the readings come from, folded away unless something needs attention
        let mut lines: Vec<(String, bool)> = Vec::new();
        for source in 1..SOURCE_COUNT {
            let state = readings.sources[source];
            let name = SOURCE_NAMES[source];
            match (state.link, state.detail.is_empty()) {
                (Link::None, _) => {}
                (Link::Connected, true) => lines.push((format!("{}: connected", name), true)),
                (Link::Connected, false) => {
                    lines.push((format!("{}: {}", name, state.detail), false))
                }
                (Link::NotFound, _) => lines.push((format!("{}: {}", name, state.detail), false)),
            }
        }
        if !lines.is_empty() {
            let attention = lines.iter().filter(|(_, ok)| !ok).count();
            let title = if attention == 0 {
                RichText::new("Sources").strong()
            } else {
                RichText::new(format!("Sources ({} to look at)", attention))
                    .strong()
                    .color(egui::Color32::from_rgb(230, 160, 60))
            };
            crate::theme::collapsing(ui, "sensors_sources", title, attention > 0, |ui| {
                for (text, ok) in &lines {
                    let rich = RichText::new(text.as_str());
                    let rich = if *ok {
                        rich.weak()
                    } else {
                        rich.color(egui::Color32::from_rgb(230, 160, 60))
                    };
                    ui.add(egui::Label::new(rich).wrap().selectable(false));
                }
            });
        }

        ui.add_space(2.0);
        crate::ui::help::link(ui, "sensors");
    });

    // ---- logging ----
    ui.add_space(8.0);
    crate::theme::card(ui, |ui| {
        ui.add(egui::Label::new(crate::theme::section_title("Logging")).selectable(false));
        ui.separator();
        // the name is chosen before logging starts (it is used when the file is created)
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("File name").selectable(false));
            let edit = ui
                .add_enabled(
                    !*logging,
                    egui::TextEdit::singleline(log_name)
                        .hint_text("sensors-<time>")
                        .desired_width(200.0),
                )
                .on_hover_text("Switch logging off to change the name.");
            if edit.changed() {
                action.log_name = Some(log_name.clone());
            }
            if edit.lost_focus() {
                action.log_name_commit = true;
            }
        });
        ui.add_space(4.0);
        let before = settings.clone();
        let mut commit = false;

        // ---- the built-in presets: what goes around the name ----
        // (they work on top of the saved names below: Date and time with a name of your own is
        // `<date>_<name>_<time>.csv`)
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Built-in preset").selectable(false));
            ui.add_enabled_ui(!*logging, |ui| {
                egui::ComboBox::from_id_salt("log_stamp").selected_text(settings.stamp.label()).width(170.0).show_ui(ui, |ui| {
                    for stamp in crate::sensor_log::Stamp::ALL {
                        if ui.selectable_value(&mut settings.stamp, stamp, stamp.label()).clicked() {
                            commit = true;
                        }
                    }
                });
            });
        });

        // ---- the saved file names ----
        // pick one to name the next log with it, or "No preset" for no name of your own
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Saved preset").selectable(false));
            let at = crate::log_plan::name_preset_index(&settings.name_presets, log_name);
            let shown = at.map(|i| settings.name_presets[i].clone()).unwrap_or_else(|| "No preset".to_string());
            ui.add_enabled_ui(!*logging, |ui| {
                egui::ComboBox::from_id_salt("log_name_preset").selected_text(shown).width(170.0).show_ui(ui, |ui| {
                    if ui.selectable_label(at.is_none() && log_name.trim().is_empty(), "No preset").clicked() {
                        log_name.clear();
                        action.log_name = Some(String::new());
                        action.log_name_commit = true;
                    }
                    for (i, preset) in settings.name_presets.iter().enumerate() {
                        if ui.selectable_label(at == Some(i), preset).clicked() {
                            *log_name = preset.clone();
                            action.log_name = Some(preset.clone());
                            action.log_name_commit = true;
                        }
                    }
                });
                let can_save = at.is_none() && !crate::sensor_log::clean_name(log_name).is_empty() && settings.name_presets.len() < crate::log_plan::MAX_NAME_PRESETS;
                if ui.add_enabled(can_save, egui::Button::new("Save name")).on_hover_text("Keep the name in the File name box as a preset.").clicked()
                    && crate::log_plan::add_name_preset(&mut settings.name_presets, log_name)
                {
                    commit = true;
                }
                if ui.add_enabled(at.is_some(), egui::Button::new("Remove")).on_hover_text("Take the shown preset out of the list.").clicked()
                    && crate::log_plan::remove_name_preset(&mut settings.name_presets, log_name)
                {
                    commit = true;
                }
            });
        });
        ui.add(
            egui::Label::new(
                RichText::new(format!(
                    "Saved as {}",
                    crate::sensor_log::file_name_stamped(crate::sensor_log::now(), log_name, settings.stamp)
                ))
                .small()
                .weak(),
            )
            .selectable(false),
        );
        ui.add_space(4.0);

        // ---- when it starts ----
        ui.horizontal(|ui| {
            ui.add(egui::Label::new("Start").selectable(false));
            egui::ComboBox::from_id_salt("log_start")
                .selected_text(match settings.start {
                    Start::Now => "Right away",
                    Start::Program => "When a program starts",
                })
                .show_ui(ui, |ui| {
                    commit |= ui
                        .selectable_value(&mut settings.start, Start::Now, "Right away")
                        .clicked();
                    commit |= ui
                        .selectable_value(
                            &mut settings.start,
                            Start::Program,
                            "When a program starts",
                        )
                        .clicked();
                });
        });
        if settings.start == Start::Program {
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::Label::new("Program").selectable(false));
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut settings.program)
                        .hint_text("Cinebench.exe")
                        .desired_width(170.0),
                );
                if edit.lost_focus() {
                    commit = true;
                }
                let current = crate::log_plan::clean_program(&settings.program);
                let pick = egui::ComboBox::from_id_salt("log_program_pick")
                    .selected_text("Pick a running program")
                    .show_ui(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                            if view.programs.is_empty() {
                                ui.add(
                                    egui::Label::new(RichText::new("None found.").weak())
                                        .selectable(false),
                                );
                            }
                            for name in view.programs {
                                if ui
                                    .selectable_label(
                                        current.eq_ignore_ascii_case(name),
                                        name.as_str(),
                                    )
                                    .clicked()
                                {
                                    settings.program = name.clone();
                                    commit = true;
                                }
                            }
                        });
                    });
                if pick.response.clicked() {
                    action.refresh_programs = true;
                }
            });
            commit |= crate::theme::toggle(
                ui,
                &mut settings.stop_with_program,
                "Stop when the program closes",
            )
            .changed();
        }

        // ---- how long it runs ----
        let mut limited = settings.limit_secs > 0;
        let remembered = egui::Id::new("log_last_limit");
        if crate::theme::toggle(ui, &mut limited, "Stop after a set time").changed() {
            if limited {
                settings.limit_secs =
                    ui.data(|d| d.get_temp::<u64>(remembered)).filter(|s| *s > 0).unwrap_or(600);
            } else {
                let last = settings.limit_secs;
                ui.data_mut(|d| d.insert_temp(remembered, last));
                settings.limit_secs = 0;
            }
            commit = true;
        }
        if limited {
            ui.indent("log_length", |ui| {
                let (mut h, mut m, mut sec) = crate::log_plan::limit_parts(settings.limit_secs);
                ui.horizontal(|ui| {
                    let mut moved = false;
                    for (value, max, suffix) in
                        [(&mut h, 2376, " h"), (&mut m, 59, " min"), (&mut sec, 59, " s")]
                    {
                        let r = ui.add(egui::DragValue::new(value).range(0..=max).suffix(suffix));
                        moved |= r.changed();
                        commit |= r.drag_stopped() || r.lost_focus();
                    }
                    if moved {
                        settings.limit_secs = crate::log_plan::limit_from_parts(h, m, sec).max(1);
                    }
                });
                ui.horizontal(|ui| {
                    for (name, secs) in [
                        ("1 min", 60),
                        ("5 min", 300),
                        ("10 min", 600),
                        ("30 min", 1800),
                        ("1 h", 3600),
                        ("2 h", 7200),
                    ] {
                        if ui.small_button(name).clicked() {
                            settings.limit_secs = secs;
                            commit = true;
                        }
                    }
                });
            });
        } else {
            ui.add(
                egui::Label::new(
                    RichText::new("No limit: it logs until you switch it off.").small().weak(),
                )
                .selectable(false),
            );
        }
        if *settings != before {
            action.log_settings = Some(settings.clone());
            action.log_settings_commit = commit;
        } else if commit {
            action.log_settings_commit = true;
        }

        ui.add_space(4.0);
        let label = match settings.start {
            Start::Now => "Log readings to a CSV file",
            Start::Program => "Log readings to a CSV file when the program starts",
        };
        let response =
            crate::theme::toggle(ui, logging, label).on_hover_text("Needs Read sensors to be on.");
        if response.changed() {
            action.logging = Some(*logging);
        }
        let plan = settings.plan();
        let folder = crate::sensor_log::log_dir().display().to_string();
        let file_of = |p: &std::path::Path| {
            p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        };
        let writing = view.log.path.as_deref().map(file_of);
        let elapsed = view
            .log
            .since_unix
            .map(|s| (crate::sensor_log::unix_now() - s).max(0) as u64)
            .unwrap_or(0);
        let last = view.log.last.as_ref().map(|(p, why)| (file_of(p.as_path()), *why));
        let status = crate::log_plan::status(&crate::log_plan::Facts {
            armed: *logging,
            reading: *on,
            plan: &plan,
            writing: writing.as_deref().map(|n| (n, elapsed)),
            rows: view.log.rows,
            waiting: view.log.waiting,
            failed: view.log.failed,
            last: last.as_ref().map(|(n, why)| (n.as_str(), *why)),
            folder: &folder,
        });
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(RichText::new(status).small().weak()).wrap().selectable(false));
            if ui.small_button("Open folder").clicked() {
                action.open_logs = true;
            }
            // the log being written (it can be watched while it grows), or else the last one finished
            let viewable =
                view.log.path.clone().or_else(|| view.log.last.as_ref().map(|(p, _)| p.clone()));
            if let Some(path) = viewable {
                let text = if view.log.path.is_some() { "View live" } else { "View it" };
                if ui.small_button(text).on_hover_text("Open it in the log viewer window").clicked()
                {
                    action.view_log = Some(path);
                }
            }
        });
        crate::ui::help::link(ui, "sensor_logging");
        // look at a finished log as graphs (a drop-down under the logging switch)
        ui.add_space(4.0);
        action.log_view = crate::ui::log_view::render_log_view(ui, log_view, picking);
    });

    // ---- graphs ----
    ui.add_space(8.0);
    crate::theme::card(ui, |ui| {
        let response = crate::theme::toggle(ui, graphs, "Live graphs").on_hover_text(
            "Off: nothing is recorded for the graphs and nothing is drawn.\n\
             On: the readings are kept for the last hour and drawn in the drop-down below.\n\
             Drawing is the only part that costs noticeable CPU, and only while the drop-down is\n\
             open and visible; closing it stops the drawing.",
        );
        if response.changed() {
            action.graphs = Some(*graphs);
        }
        if *graphs {
            crate::theme::collapsing(
                ui,
                "sensor_graphs",
                crate::theme::section_title("Graphs"),
                true,
                |ui| {
                    if crate::ui::graph::render_graphs(ui, view.history, window) {
                        action.window_changed = true;
                    }
                },
            );
        } else {
            ui.add(
                egui::Label::new(
                    RichText::new("Off: no history is recorded and no graph is drawn.")
                        .small()
                        .weak(),
                )
                .selectable(false),
            );
        }
    });

    action
}
