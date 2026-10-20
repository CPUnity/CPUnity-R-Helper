// Log viewer: pick one of R-Helper's CSV logs and see it as graphs plus a min / average / max
// table, instead of rows of numbers.
use crate::log_view::Loaded;
use crate::sensor_history::{Series, SERIES_COUNT};
use crate::theme;
use eframe::egui::{self, RichText};
use std::path::PathBuf;

/// What the viewer shows and has been told.
#[derive(Default)]
pub struct LogView {
    /// The logs in the logs folder, newest first.
    pub files: Vec<PathBuf>,
    /// The folder was listed.
    pub listed: bool,
    pub loaded: Option<Loaded>,
    /// The file `loaded` was read from.
    pub path: Option<PathBuf>,
    pub error: Option<String>,
}

/// What was clicked.
#[derive(Debug, Default, PartialEq)]
pub struct LogViewAction {
    /// Open this log.
    pub open: Option<PathBuf>,
    /// "Browse..." was clicked.
    pub browse: bool,
    /// "Refresh" was clicked: list the folder again.
    pub refresh: bool,
    /// "Open full viewer" was clicked: show the loaded log in its window again.
    pub open_window: bool,
    /// "Open folder" was clicked.
    pub open_folder: bool,
}

fn number(value: f32, series: Series) -> String {
    match series.unit() {
        "W" => format!("{:.1} W", value),
        unit => format!("{:.0} {}", value, unit),
    }
}

fn duration_text(seconds: f64) -> String {
    let s = seconds.round() as u64;
    match s {
        0..=59 => format!("{} s", s),
        60..=3599 => format!("{} min {} s", s / 60, s % 60),
        _ => format!("{} h {} min", s / 3600, s % 3600 / 60),
    }
}

pub fn render_log_view(ui: &mut egui::Ui, view: &mut LogView, picking: bool) -> LogViewAction {
    let mut action = LogViewAction::default();
    {
        theme::collapsing(ui, "log_viewer", RichText::new("Log viewer").strong(), false, |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new("Pick one of the logs R-Helper wrote, or browse to a file. It opens in a window of its own.")
                    .small()
                    .weak(),
                )
                .wrap()
                .selectable(false),
            );
            ui.horizontal(|ui| {
                let shown = view
                    .loaded
                    .as_ref()
                    .map(|l| l.name.clone())
                    .unwrap_or_else(|| "Choose a log...".to_string());
                egui::ComboBox::from_id_salt("log_view_file")
                    .selected_text(shown)
                    .width(250.0)
                    .show_ui(ui, |ui| {
                        if view.files.is_empty() {
                            ui.label(RichText::new("No logs yet. Switch Logging on first.").weak());
                        }
                        for path in &view.files {
                            let name = path
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            if ui.selectable_label(false, name).clicked() {
                                action.open = Some(path.clone());
                            }
                        }
                    });
                if ui
                    .add_enabled(!picking, egui::Button::new("\u{1f4c2}"))
                    .on_hover_text("Browse... (opens a file picker)")
                    .clicked()
                {
                    action.browse = true;
                }
                if ui.small_button("Refresh").on_hover_text("List the logs folder again").clicked()
                {
                    action.refresh = true;
                }
                if ui.small_button("Open folder").clicked() {
                    action.open_folder = true;
                }
            });

            if let Some(error) = &view.error {
                ui.add(
                    egui::Label::new(
                        RichText::new(format!("Could not show it: {}", error))
                            .color(egui::Color32::from_rgb(230, 160, 60)),
                    )
                    .wrap()
                    .selectable(false),
                );
            }
            let Some(loaded) = &view.loaded else {
                return;
            };
            ui.add_space(6.0);
            ui.add(
                egui::Label::new(
                    RichText::new(format!(
                        "{}  to  {}   ({}, {} readings)",
                        loaded.start,
                        loaded.end,
                        duration_text(loaded.history.duration()),
                        loaded.rows
                    ))
                    .strong(),
                )
                .selectable(false),
            );
            ui.add_space(4.0);
            egui::Grid::new("log_view_stats")
                .num_columns(4)
                .spacing([18.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    for name in ["Reading", "Lowest", "Average", "Highest"] {
                        ui.add(egui::Label::new(RichText::new(name).weak()).selectable(false));
                    }
                    ui.end_row();
                    for series in Series::ALL {
                        let Some((min, avg, max)) = loaded.history.stats(series) else {
                            continue;
                        };
                        ui.add(egui::Label::new(series.full_label()).selectable(false));
                        for value in [min, avg, max] {
                            ui.add(egui::Label::new(number(value, series)).selectable(false));
                        }
                        ui.end_row();
                    }
                });
            ui.add_space(6.0);
            if ui
                .button("Open the full viewer")
                .on_hover_text("The graphs in a window of their own")
                .clicked()
            {
                action.open_window = true;
            }
        });
    }
    // (SERIES_COUNT keeps the stats table in step with the columns that are logged)
    debug_assert_eq!(Series::ALL.len(), SERIES_COUNT);
    action
}
