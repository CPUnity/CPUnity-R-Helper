// Files tab: big tiles, one per place R-Helper stores data. Click a tile to open it in Explorer.
use crate::config::Config;
use crate::picker::Target;
use crate::places::{
    explorer_args, open_url, path_status, places, Link, PathStatus, Place, PAWN_LINKS,
};
use crate::sensor_ext::{self, DEFAULT_LHM_ADDR, HWINFO, LHM, LOG, PAWN};
use crate::theme;
use eframe::egui::{self, vec2, Color32, CornerRadius, RichText, Sense, Stroke, StrokeKind};
use std::path::Path;

const ORANGE: Color32 = Color32::from_rgb(230, 160, 60);
const GREEN: Color32 = Color32::from_rgb(110, 200, 120);

/// What happened in the Files tab this frame.
#[derive(Debug, Default)]
pub struct FilesAction {
    /// A setting changed (save the config).
    pub changed: bool,
    /// A sensor source setting changed for good: restart the readings with the new one.
    pub restart: bool,
    /// "Browse..." was clicked.
    pub pick: Option<Target>,
}

const TILE_HEIGHT: f32 = 112.0;
const GAP: f32 = 10.0;

/// Opens `place` in File Explorer (folders that do not exist yet are created first).
fn open(place: &Place) {
    let exists = place.path.exists();
    if !place.is_file {
        let _ = std::fs::create_dir_all(&place.path);
    }
    let _ = std::process::Command::new("explorer").args(explorer_args(place, exists)).spawn();
}

/// One tile; true when it was clicked.
fn tile(ui: &mut egui::Ui, width: f32, place: &Place) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(width, TILE_HEIGHT), Sense::click());
    let hovered = response.hovered();
    let painter = ui.painter();
    let fill = if hovered { theme::surface_2() } else { theme::surface() };
    let stroke = if hovered { theme::accent_stroke() } else { theme::border() };
    painter.rect_filled(rect, CornerRadius::same(10), fill);
    painter.rect_stroke(rect, CornerRadius::same(10), Stroke::new(1.0, stroke), StrokeKind::Inside);

    let inner = rect.shrink(12.0);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    child.set_clip_rect(inner.intersect(ui.clip_rect()));
    child.add(
        egui::Label::new(
            RichText::new(format!("{}  {}", place.icon, place.title))
                .strong()
                .size(17.0)
                .color(theme::accent_stroke()),
        )
        .selectable(false),
    );
    child
        .add(egui::Label::new(RichText::new(place.detail).small().weak()).wrap().selectable(false));
    child.add(
        egui::Label::new(RichText::new(place.path.display().to_string()).monospace().small())
            .truncate()
            .selectable(false),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

pub fn render_files(ui: &mut egui::Ui, cfg: &mut Config, picking: bool) -> FilesAction {
    let mut action = FilesAction::default();
    let exe = std::env::current_exe().ok();
    let list = places(&crate::config::config_dir(), &crate::sensor_log::log_dir(), exe.as_deref());

    ui.add(
        egui::Label::new(
            RichText::new(
                "Everything R-Helper stores on this PC. Click a tile to open it in File Explorer.",
            )
            .weak(),
        )
        .wrap()
        .selectable(false),
    );
    ui.add_space(6.0);

    let width = ((ui.available_width() - GAP) / 2.0).floor().max(120.0);
    let mut clicked = None;
    for pair in list.chunks(2) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            for place in pair {
                if tile(ui, width, place) {
                    clicked = Some(place.clone());
                }
            }
        });
        ui.add_space(GAP - 4.0);
    }
    if let Some(place) = clicked {
        open(&place);
    }

    ui.add(
        egui::Label::new(
            RichText::new(
                "Besides these files, R-Helper only adds a Windows startup entry when \"Launch at \
                 Windows startup\" is on, and a Task Scheduler task if you set up the login \
                 task in Options.",
            )
            .small()
            .weak(),
        )
        .wrap()
        .selectable(false),
    );

    // ---- files of other programs ----
    ui.add_space(12.0);
    ui.add(
        egui::Label::new(theme::section_title("Where R-Helper looks for other programs"))
            .selectable(false),
    );
    ui.add(
        egui::Label::new(
            RichText::new(
                "Sensor sources read data from these programs. Open one to say where its file \
                 is: type the location or click the folder button to pick it.",
            )
            .small()
            .weak(),
        )
        .wrap()
        .selectable(false),
    );
    ui.add_space(4.0);
    theme::collapsing(
        ui,
        "files_log",
        "Log file: ThrottleStop / Intel XTU / HWiNFO",
        false,
        |ui| {
            use_source(ui, cfg, LOG, &mut action);
            note(
            ui,
            "These programs have no live interface, so R-Helper reads the newest line of the CSV \
             log they write. ThrottleStop: Options > Log File. Intel XTU: Settings > Logging. \
             HWiNFO: the sensors window's logging button.",
        );
            let mut text = std::mem::take(&mut cfg.sensor_log_path);
            path_row(
                ui,
                &mut text,
                "C:\\path\\to\\log.csv",
                Some(Target::LogFile),
                picking,
                &mut action,
            );
            cfg.sensor_log_path = text;
            match path_status(&cfg.sensor_log_path) {
                PathStatus::Empty => status(ui, "No file chosen.", false),
                PathStatus::Missing => status(ui, "That file was not found.", false),
                PathStatus::Folder => {
                    status(ui, "That is a folder: choose the log file itself.", false)
                }
                PathStatus::File => status(ui, "File found.", true),
            }
            open_folder_button(ui, Path::new(cfg.sensor_log_path.trim().trim_matches('"')));
        },
    );
    theme::collapsing(ui, "files_pawn", "PawnIO modules: IntelMSR.bin, Nvidia.bin", false, |ui| {
        use_source(ui, cfg, PAWN, &mut action);
        note(
            ui,
            "PawnIO reads CPU temperature, power limits and the GPU memory temperature straight \
             from the hardware. It needs the PawnIO driver (pawnio.eu) and the signed module \
             files from the PawnIO.Modules releases. Name IntelMSR.bin or its folder; Nvidia.bin \
             is looked for next to it. Empty = next to R-Helper and in Program Files\\PawnIO.",
        );
        theme::collapsing(ui, "files_pawn_get", "Get Modules Here", false, |ui| {
            note(ui, "Two downloads, opened in your browser. R-Helper downloads nothing itself.");
            for link in PAWN_LINKS {
                link_row(ui, link);
            }
        });
        let mut text = std::mem::take(&mut cfg.sensor_module_path);
        path_row(
            ui,
            &mut text,
            "IntelMSR.bin or its folder",
            Some(Target::ModuleFile),
            picking,
            &mut action,
        );
        cfg.sensor_module_path = text;
        for name in ["IntelMSR.bin", "Nvidia.bin"] {
            match sensor_ext::find_module(&cfg.sensor_module_path, name) {
                Some(found) => {
                    status(ui, &format!("{}: found at {}", name, found.display()), true);
                    open_folder_button(ui, &found);
                }
                None => status(ui, &format!("{}: not found", name), false),
            }
        }
        match sensor_ext::pawnio_driver_dir() {
            Some(dir) => {
                status(ui, &format!("PawnIO driver: found at {}", dir.display()), true);
                open_folder_button(ui, &dir.join("PawnIOLib.dll"));
            }
            None => status(ui, "PawnIO driver: not installed (get it at pawnio.eu).", false),
        }
    });
    theme::collapsing(ui, "files_lhm", "LibreHardwareMonitor: web server address", false, |ui| {
        use_source(ui, cfg, LHM, &mut action);
        note(
            ui,
            "Nothing to locate: R-Helper asks LibreHardwareMonitor's built-in web server. In \
             LibreHardwareMonitor: Options > Remote Web Server > Run. Default port 8085.",
        );
        ui.horizontal(|ui| {
            let mut text = std::mem::take(&mut cfg.sensor_lhm_addr);
            text_edit(ui, &mut text, DEFAULT_LHM_ADDR, 170.0, &mut action);
            cfg.sensor_lhm_addr = text;
            if ui
                .add_enabled(cfg.sensor_lhm_addr != DEFAULT_LHM_ADDR, egui::Button::new("Default"))
                .on_hover_text(format!("Back to {}", DEFAULT_LHM_ADDR))
                .clicked()
            {
                cfg.sensor_lhm_addr = DEFAULT_LHM_ADDR.to_string();
                action.changed = true;
                action.restart = true;
            }
        });
    });
    theme::collapsing(ui, "files_hwinfo", "HWiNFO: shared memory", false, |ui| {
        use_source(ui, cfg, HWINFO, &mut action);
        note(
            ui,
            "Nothing to locate: HWiNFO hands its readings over through Windows shared memory. In \
             HWiNFO's Settings tick \"Shared Memory Support\" and keep the sensors window running. \
             A listing of everything it shares is written to hwinfo_sensors.txt in the settings \
             folder (first tile).",
        );
    });
    action
}

/// A world button that opens `link` in the browser, with where it leads and what to download.
fn link_row(ui: &mut egui::Ui, link: Link) {
    ui.horizontal_top(|ui| {
        if ui
            .button("\u{1f310}")
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(format!("Open {} in your browser", link.url))
            .clicked()
        {
            open_url(link.url);
        }
        ui.vertical(|ui| {
            ui.add(
                egui::Label::new(RichText::new(link.title).strong().color(theme::accent_stroke()))
                    .selectable(false),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(format!("Leads to {}", link.url)).small().monospace(),
                )
                .truncate()
                .selectable(false),
            );
            note(ui, link.get);
        });
    });
    ui.add_space(4.0);
}

fn note(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).small().weak()).wrap().selectable(false));
}

fn status(ui: &mut egui::Ui, text: &str, ok: bool) {
    let rich = RichText::new(text).small();
    ui.add(
        egui::Label::new(if ok { rich.color(GREEN) } else { rich.color(ORANGE) })
            .wrap()
            .selectable(false),
    );
}

/// The "use this source" switch (the same one as in Options).
fn use_source(ui: &mut egui::Ui, cfg: &mut Config, source: usize, action: &mut FilesAction) {
    let title = sensor_ext::SOURCE_TITLES[source];
    if theme::toggle(ui, &mut cfg.sensor_enabled[source], title).changed() {
        action.changed = true;
        action.restart = true;
    }
}

/// A single-line text box; a finished edit restarts the readings once (not on every key).
fn text_edit(
    ui: &mut egui::Ui,
    text: &mut String,
    hint: &str,
    width: f32,
    action: &mut FilesAction,
) {
    let edit = ui.add(egui::TextEdit::singleline(text).hint_text(hint).desired_width(width));
    let edited = edit.id.with("edited");
    if edit.changed() {
        action.changed = true;
        ui.data_mut(|d| d.insert_temp(edited, true));
    }
    if edit.lost_focus() && ui.data_mut(|d| d.remove_temp::<bool>(edited)).unwrap_or(false) {
        action.restart = true;
    }
}

/// A location box with a folder button that opens the Windows file picker.
fn path_row(
    ui: &mut egui::Ui,
    text: &mut String,
    hint: &str,
    target: Option<Target>,
    picking: bool,
    action: &mut FilesAction,
) {
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 48.0).max(120.0);
        text_edit(ui, text, hint, width, action);
        if let Some(target) = target {
            if ui
                .add_enabled(!picking, egui::Button::new("\u{1f4c2}"))
                .on_hover_text("Browse... (opens a file picker)")
                .clicked()
            {
                action.pick = Some(target);
            }
        }
    });
}

/// "Open folder" for a file or folder path (the file is shown selected).
fn open_folder_button(ui: &mut egui::Ui, path: &Path) {
    if path.as_os_str().is_empty() || !path.exists() {
        return;
    }
    if ui.small_button("Open folder").clicked() {
        let place = Place {
            icon: "",
            title: "",
            detail: "",
            path: path.to_path_buf(),
            is_file: path.is_file(),
        };
        open(&place);
    }
}
