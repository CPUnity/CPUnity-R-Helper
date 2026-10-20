// Log viewer window: a sensor log in a window of its own. Real clock times on the time axis, drag to
// move, scroll to zoom, a crosshair that follows the pointer across every graph, a switch for each
// line with its lowest / average / highest for the part on screen, and an overview strip of the
// whole log. The maths of moving and zooming is in `log_chart` (tested); this file draws it.
use crate::log_chart::{self as lc, Span};
use crate::log_options::{self as lo, Dots, Options, Preset, Side, Wheel, YMode};
use crate::log_view::Loaded;
use crate::sensor_history::{self as hist, Series};
use crate::theme;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, RichText, Sense, Shape, Stroke,
    StrokeKind,
};
use std::path::PathBuf;

const ORANGE: Color32 = Color32::from_rgb(255, 159, 67);
const CYAN: Color32 = Color32::from_rgb(60, 200, 255);
const VIOLET: Color32 = Color32::from_rgb(190, 110, 255);
const AMBER: Color32 = Color32::from_rgb(255, 214, 84);
const CORAL: Color32 = Color32::from_rgb(255, 99, 99);
const LIME: Color32 = Color32::from_rgb(150, 220, 100);

/// Margins round the plot, at the standard text size (they grow with the text of the graphs).
const PLOT_RIGHT: f32 = 14.0;
/// How much bigger than normal the text of this window is (the same as the Info tab).
const TEXT_SCALE: f32 = crate::ui::info::TEXT_SCALE;
/// Standard sizes of the text drawn on the graphs.
const TICK_FONT: f32 = 13.5;
const TITLE_FONT: f32 = 16.0;
const READOUT_FONT: f32 = 15.0;
/// The graphs in the order they are stacked, by the unit their lines share.
/// How small the numbers panel makes its text to fit a narrow panel (below that it scrolls).
const SIDE_MIN_FIT: f32 = 0.75;
const UNITS: [&str; 4] = ["\u{b0}C", "W", "MHz", "%"];
/// How long a followed live log waits between two reloads (seconds).
const FOLLOW_EVERY: f64 = 4.0;

/// One line of the log: every value it has, as (seconds since the first reading, value).
struct Trace {
    series: Series,
    /// Exactly what the log says (degrees Celsius).
    raw: Vec<(f64, f32)>,
    /// What is shown (the temperatures in Fahrenheit when that is chosen).
    points: Vec<(f64, f32)>,
}

pub struct LogWindow {
    pub path: PathBuf,
    name: String,
    start_unix: i64,
    start_text: String,
    end_text: String,
    rows: usize,
    total: f64,
    traces: Vec<Trace>,
    span: Span,
    /// The moment under the pointer, shared by every graph.
    cursor: Option<f64>,
    /// The time axis shows clock times (otherwise time since the first reading).
    clock: bool,
    /// Set by the main window: this log is the one being written right now.
    pub live: bool,
    was_live: bool,
    /// Reload it every few seconds while it is being written.
    follow: bool,
    /// All the settings of this window.
    pub opts: Options,
    /// A setting changed since the main window last saved them.
    opts_changed: bool,
    options_open: bool,
    /// `points` of the traces are in Fahrenheit.
    converted_f: bool,
    /// How wide the numbers table is at the full size of its text (the panel shrinks it to fit).
    side_need: f32,
    /// The line whose name the pointer is over (the others are dimmed).
    hover: Option<Series>,
    /// The last "always on top" state sent to the system.
    sent_on_top: Option<bool>,
    reload: bool,
    last_reload: f64,
    /// A drag on the overview strip started inside the shown part (moves it) or outside (centres it).
    overview_drag: Option<bool>,
}

/// The colour of a line: the one the user picked, or the standard one.
fn colour(series: Series, opts: &Options) -> Color32 {
    match opts.colour_of(series.column()) {
        Some([r, g, b]) => Color32::from_rgb(r, g, b),
        None => default_colour(series),
    }
}

fn default_colour(series: Series) -> Color32 {
    match series {
        Series::CpuTemp | Series::CpuPower | Series::CpuClock | Series::CpuLoad => {
            theme::accent_stroke()
        }
        Series::CpuCoreTemp => LIME,
        Series::CpuZoneTemp => ORANGE,
        Series::GpuTemp | Series::GpuPower | Series::GpuClock | Series::GpuLoad => CYAN,
        Series::CpuClockTop => AMBER,
        Series::CpuClockP => LIME,
        Series::CpuClockE => VIOLET,
        Series::GpuMemTemp => VIOLET,
        Series::Pl1 => AMBER,
        Series::Pl2 => CORAL,
    }
}

fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

fn dashed(series: Series) -> bool {
    matches!(series, Series::Pl1 | Series::Pl2)
}

fn group_title(unit: &str) -> &'static str {
    match unit {
        "W" => "Power",
        "MHz" => "Clock speed",
        "%" => "Load",
        _ => "Temperature",
    }
}

/// The least the vertical axis spans, so that a flat line does not fill the height with noise.
fn min_span(unit: &str, opts: &Options) -> f32 {
    match unit {
        "W" => 20.0,
        "MHz" => 400.0,
        "%" => 1.0,
        _ if opts.fahrenheit => 12.0 * 1.8,
        _ => 12.0,
    }
}

/// The unit as written (the temperature graph says \u{b0}F when that is chosen).
fn unit_name<'a>(unit: &'a str, opts: &Options) -> &'a str {
    if unit == "\u{b0}C" && opts.fahrenheit {
        "\u{b0}F"
    } else {
        unit
    }
}

fn value_text(value: f32, unit: &str, opts: &Options) -> String {
    let decimals = match unit {
        "W" => opts.power_decimals,
        "\u{b0}C" => opts.temp_decimals,
        _ => 0,
    };
    format!("{} {}", lo::number_text(value, decimals), unit_name(unit, opts))
}

fn tick_text(value: f32) -> String {
    if (value - value.round()).abs() < 0.01 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
}

fn hidden_key(series: Series) -> String {
    format!("logwin_hidden/{}", series.column())
}

/// A line that was switched off (remembered between launches like the drop-down states).
fn is_hidden(series: Series) -> bool {
    theme::get_open(&hidden_key(series)).unwrap_or(false)
}

impl LogWindow {
    pub fn new(loaded: &Loaded, path: PathBuf, opts: Options) -> LogWindow {
        let mut win = LogWindow {
            path,
            name: loaded.name.clone(),
            start_unix: loaded.start_unix,
            start_text: String::new(),
            end_text: String::new(),
            rows: 0,
            total: lc::MIN_SPAN,
            traces: Vec::new(),
            span: Span::all(lc::MIN_SPAN),
            cursor: None,
            clock: true,
            live: false,
            was_live: false,
            follow: true,
            opts,
            opts_changed: false,
            options_open: false,
            converted_f: false,
            hover: None,
            side_need: 380.0,
            sent_on_top: None,
            reload: false,
            last_reload: 0.0,
            overview_drag: None,
        };
        win.fill(loaded);
        win.span = Span::all(win.total);
        win
    }

    fn fill(&mut self, loaded: &Loaded) {
        self.name = loaded.name.clone();
        self.start_unix = loaded.start_unix;
        self.start_text = loaded.start.clone();
        self.end_text = loaded.end.clone();
        self.rows = loaded.rows;
        self.total = loaded.history.duration().max(lc::MIN_SPAN);
        self.traces = Series::ALL
            .iter()
            .copied()
            .map(|series| {
                let raw = loaded.history.all(series);
                Trace { series, points: Vec::new(), raw }
            })
            .filter(|t| !t.raw.is_empty())
            .collect();
        self.convert();
    }

    /// Fills what is shown from what the log says, in the unit that is chosen.
    fn convert(&mut self) {
        let fahrenheit = self.opts.fahrenheit;
        for trace in &mut self.traces {
            trace.points = if fahrenheit && trace.series.unit() == "\u{b0}C" {
                trace.raw.iter().map(|(t, v)| (*t, lo::fahrenheit(*v))).collect()
            } else {
                trace.raw.clone()
            };
        }
        self.converted_f = fahrenheit;
    }

    /// The settings as text, once after they changed (the main window saves them).
    pub fn take_options(&mut self) -> Option<String> {
        if self.opts_changed {
            self.opts_changed = false;
            Some(self.opts.encode())
        } else {
            None
        }
    }

    /// Replaces the data with a newer read of the same file. A window that was looking at the end
    /// keeps looking at the end.
    pub fn reloaded(&mut self, loaded: &Loaded) {
        let at_end = self.span.hi >= self.total - 0.5;
        let width = self.span.width();
        self.fill(loaded);
        self.span = if at_end {
            Span { lo: self.total - width, hi: self.total }.clamped(self.total)
        } else {
            self.span.clamped(self.total)
        };
    }

    /// Whether the main window should read the file again now.
    pub fn due_reload(&mut self, now: f64) -> bool {
        let finished = self.was_live && !self.live;
        self.was_live = self.live;
        let due = self.reload
            || finished
            || (self.live && self.follow && now - self.last_reload >= FOLLOW_EVERY);
        if due {
            self.reload = false;
            self.last_reload = now;
        }
        due
    }

    fn clock_origin(&self) -> i64 {
        if self.clock {
            self.start_unix
        } else {
            0
        }
    }

    /// `04:27:41 (+1:05)` for the time at `t`.
    fn moment_text(&self, t: f64) -> String {
        let date = if self.total > 3600.0 * 12.0 {
            format!("{} ", lc::date_text(self.start_unix, t))
        } else {
            String::new()
        };
        format!("{}{}  ({})", date, lc::clock_text(self.start_unix, t, true), lc::elapsed_text(t))
    }
}

/// Shows the window (a window of its own, or inside the main one where the system cannot make
/// more). Returns false once it has been closed.
pub fn show(ctx: &egui::Context, win: &mut LogWindow) -> bool {
    let mut keep = true;
    let id = egui::ViewportId::from_hash_of("cpunity_log_viewer");
    let builder = egui::ViewportBuilder::default()
        .with_title(format!("Log viewer - {}", win.name))
        .with_inner_size([1280.0, 780.0])
        .with_min_inner_size([760.0, 480.0]);
    ctx.show_viewport_immediate(id, builder, |ctx, class| {
        if class == egui::ViewportClass::Embedded {
            let mut open = true;
            egui::Window::new(format!("Log viewer - {}", win.name))
                .id(egui::Id::new("cpunity_log_viewer_window"))
                .default_size([980.0, 640.0])
                .open(&mut open)
                .show(ctx, |ui| body(ui, win));
            keep &= open;
        } else {
            if win.sent_on_top != Some(win.opts.on_top) {
                win.sent_on_top = Some(win.opts.on_top);
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if win.opts.on_top {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                }));
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(theme::bg()).inner_margin(egui::Margin::same(12)))
                .show(ctx, |ui| body(ui, win));
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            keep = false;
        }
    });
    if win.live && win.follow {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(FOLLOW_EVERY));
    }
    keep
}

/// Runs `add` with every text style of the `ui` made `factor` times the standard size of this
/// window (which is the Info tab's size).
fn scaled<R>(ui: &mut egui::Ui, factor: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.scope(|ui| {
        for font in ui.style_mut().text_styles.values_mut() {
            font.size *= TEXT_SCALE * factor;
        }
        add(ui)
    })
    .inner
}

fn body(ui: &mut egui::Ui, win: &mut LogWindow) {
    win.cursor = None;
    win.hover = None;
    if win.converted_f != win.opts.fahrenheit {
        win.convert();
    }
    keys(ui, win);
    let before = win.opts.clone();
    let o = before.clone();

    egui::TopBottomPanel::top("logwin_top")
        .show_separator_line(false)
        .show_inside(ui, |ui| scaled(ui, o.factor(o.size_toolbar), |ui| toolbar(ui, win)));
    if o.overview {
        egui::TopBottomPanel::bottom("logwin_overview")
            .exact_height(o.overview_height + 8.0)
            .show_separator_line(false)
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                overview(ui, win);
            });
    }
    // the options panel is the outermost on the right, the numbers panel sits inside it
    if win.options_open {
        egui::SidePanel::right("logwin_options")
            .resizable(true)
            .default_width(420.0)
            .width_range(300.0..=800.0)
            .show_inside(ui, |ui| {
                scaled(ui, o.factor(o.size_options), |ui| options_panel(ui, win))
            });
    }
    // as narrow as the table can get with its text shrunk as far as it goes
    let body_size = ui.style().text_styles[&egui::TextStyle::Body].size;
    let metrics = side_metrics(ui, win, body_size * TEXT_SCALE * o.factor(o.size_side));
    let narrowest = metrics.table().0.min(metrics.stacked().0);
    win.side_need = narrowest;
    let min_side = (narrowest * SIDE_MIN_FIT).max(150.0);
    let panel = match o.side {
        Side::Left => Some(egui::SidePanel::left("logwin_side")),
        Side::Right => Some(egui::SidePanel::right("logwin_side_right")),
        Side::Hidden => None,
    };
    if let Some(panel) = panel {
        panel
            .resizable(true)
            .default_width(470.0)
            .width_range(min_side..=900.0)
            .show_inside(ui, |ui| scaled(ui, o.factor(o.size_side), |ui| side(ui, win)));
    }
    egui::CentralPanel::default().show_inside(ui, |ui| charts(ui, win));

    if win.opts != before {
        win.opts_changed = true;
    }
}

/// Arrow keys move, plus and minus zoom, Home shows everything, End jumps to the end.
fn keys(ui: &egui::Ui, win: &mut LogWindow) {
    use egui::Key;
    let (left, right, plus, minus, home, end, shift) = ui.input(|i| {
        (
            i.key_pressed(Key::ArrowLeft),
            i.key_pressed(Key::ArrowRight),
            i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals),
            i.key_pressed(Key::Minus),
            i.key_pressed(Key::Home),
            i.key_pressed(Key::End),
            i.modifiers.shift,
        )
    });
    let step = win.span.width() * if shift { 0.5 } else { 0.1 };
    let centre = (win.span.lo + win.span.hi) / 2.0;
    if left {
        win.span = win.span.panned(-step, win.total);
    }
    if right {
        win.span = win.span.panned(step, win.total);
    }
    if plus {
        win.span = win.span.zoomed(0.7, centre, win.total);
    }
    if minus {
        win.span = win.span.zoomed(1.0 / 0.7, centre, win.total);
    }
    if home {
        win.span = Span::all(win.total);
    }
    if end {
        win.span = Span { lo: win.total - win.span.width(), hi: win.total }.clamped(win.total);
    }
}

fn toolbar(ui: &mut egui::Ui, win: &mut LogWindow) {
    let k = win.opts.factor(win.opts.size_toolbar);
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::Label::new(
                RichText::new(&win.name).size(17.0 * k).strong().color(theme::accent_stroke()),
            )
            .selectable(false),
        );
        ui.add(
            egui::Label::new(
                RichText::new(format!(
                    "{}  to  {}   \u{b7}   {}   \u{b7}   {} readings",
                    win.start_text,
                    win.end_text,
                    lc::length_text(win.total),
                    win.rows
                ))
                .weak(),
            )
            .selectable(false),
        );
    });
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        let total = win.total;
        let centre = (win.span.lo + win.span.hi) / 2.0;
        if ui
            .button("Show all")
            .on_hover_text("The whole log (double-click a graph, or Home)")
            .clicked()
        {
            win.span = Span::all(total);
        }
        if ui.button("Zoom in").on_hover_text("Scroll up over a graph, or +").clicked() {
            win.span = win.span.zoomed(0.5, centre, total);
        }
        if ui.button("Zoom out").on_hover_text("Scroll down over a graph, or -").clicked() {
            win.span = win.span.zoomed(2.0, centre, total);
        }
        if ui.button("\u{25c0}").on_hover_text("Move earlier (left arrow)").clicked() {
            win.span = win.span.panned(-win.span.width() * 0.5, total);
        }
        if ui.button("\u{25b6}").on_hover_text("Move later (right arrow)").clicked() {
            win.span = win.span.panned(win.span.width() * 0.5, total);
        }
        if ui.button("End").on_hover_text("Jump to the newest readings").clicked() {
            win.span = Span { lo: total - win.span.width(), hi: total }.clamped(total);
        }
        ui.add_space(10.0);
        ui.add(egui::Label::new(RichText::new("Times").weak()).selectable(false));
        if ui.selectable_label(win.clock, "Clock").clicked() {
            win.clock = true;
        }
        if ui.selectable_label(!win.clock, "Since start").clicked() {
            win.clock = false;
        }
        ui.add_space(10.0);
        ui.add(
            egui::Label::new(
                RichText::new(format!(
                    "Showing {} to {}  ({})",
                    lc::clock_text(win.start_unix, win.span.lo, true),
                    lc::clock_text(win.start_unix, win.span.hi, true),
                    lc::length_text(win.span.width())
                ))
                .weak(),
            )
            .selectable(false),
        );
        if win.live {
            ui.add_space(10.0);
            ui.checkbox(&mut win.follow, "Follow live")
                .on_hover_text("This log is still being written: read it again every few seconds.");
        }
        if ui.button("Reload").on_hover_text("Read the file again").clicked() {
            win.reload = true;
        }
        ui.add_space(10.0);
        let bar = win.opts.side != Side::Hidden;
        if ui
            .selectable_label(bar, "Numbers")
            .on_hover_text("The side bar with the switches and the lowest / average / highest")
            .clicked()
        {
            win.opts.side = if bar { Side::Hidden } else { Side::Left };
        }
        if ui
            .selectable_label(win.options_open, "Options")
            .on_hover_text("Text sizes, what is drawn, units, colours and more")
            .clicked()
        {
            win.options_open = !win.options_open;
        }
    });
    ui.add_space(4.0);
}

/// Lowest, average, highest and latest of a line between two moments.
fn stats_row(points: &[(f64, f32)], from: f64, to: f64) -> Option<[f32; 4]> {
    let (low, avg, high) = lc::stats_in(points, from, to)?;
    let end = points.partition_point(|p| p.0 <= to);
    let last = end
        .checked_sub(1)
        .and_then(|i| points.get(i))
        .filter(|p| p.0 >= from)
        .map_or(high, |p| p.1);
    Some([low, avg, high, last])
}

/// The columns of numbers that are switched on: (heading, index into `stats_row`).
fn columns(o: &Options) -> Vec<(&'static str, usize)> {
    [
        ("Lowest", o.col_low, 0),
        ("Average", o.col_avg, 1),
        ("Highest", o.col_high, 2),
        ("Latest", o.col_last, 3),
    ]
    .into_iter()
    .filter(|(_, on, _)| *on)
    .map(|(name, _, index)| (name, index))
    .collect()
}

/// The numbers as text that a spreadsheet takes (tab separated).
fn numbers_text(win: &LogWindow, from: f64, to: f64) -> String {
    let o = &win.opts;
    let columns = columns(o);
    let mut text = String::from("Line");
    for (name, _) in &columns {
        text.push('\t');
        text.push_str(name);
    }
    text.push('\n');
    for trace in &win.traces {
        text.push_str(trace.series.label());
        let row = stats_row(&trace.points, from, to);
        for (_, index) in &columns {
            text.push('\t');
            if let Some(row) = row {
                text.push_str(&value_text(row[*index], trace.series.unit(), o));
            }
        }
        text.push('\n');
    }
    text
}

/// What the numbers table is made of, measured with text of one size.
struct SideMetrics {
    /// The widest name (with its dot) or group title.
    names: f32,
    /// The widest cell of each column of numbers (its heading included).
    cols: Vec<f32>,
    groups: f32,
    rows: f32,
    size: f32,
    row_spacing: f32,
}

impl SideMetrics {
    /// Width and height of the numbers as a table (names left, one column for each number).
    fn table(&self) -> (f32, f32) {
        let width =
            self.names + self.cols.iter().sum::<f32>() + 18.0 * self.cols.len() as f32 + 22.0;
        let line = self.size * 1.3 + self.row_spacing;
        let height = (self.groups + self.rows) * line
            + self.groups * (4.0 + self.row_spacing)
            + 3.0 * (self.size * 1.6 + 8.0);
        (width.max(120.0), height.max(120.0))
    }

    /// Width and height with each line's numbers on a row of their own under its name (narrow but
    /// tall): the text can stay big in a thin panel.
    fn stacked(&self) -> (f32, f32) {
        let numbers =
            16.0 + self.cols.iter().sum::<f32>() + 14.0 * self.cols.len().saturating_sub(1) as f32;
        let width = self.names.max(numbers) + 22.0;
        let line = self.size * 1.3 + 3.0;
        let height = (1.0 + self.groups + 2.0 * self.rows) * line
            + self.groups * 8.0
            + 3.0 * (self.size * 1.6 + 8.0);
        (width.max(120.0), height.max(120.0))
    }
}

/// Measures the numbers table with text of the size `size`, from the text itself, so that it is
/// right at once (not a frame late).
fn side_metrics(ui: &egui::Ui, win: &LogWindow, size: f32) -> SideMetrics {
    let o = &win.opts;
    let (from, to) =
        if o.stats_whole { (0.0, win.total.max(lc::MIN_SPAN)) } else { (win.span.lo, win.span.hi) };
    let font = FontId::proportional(size);
    let painter = ui.painter();
    let measure = |text: &str| {
        painter.layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE).size().x + 1.0
    };
    let columns = columns(o);
    let mut names = 0.0f32;
    let mut cols: Vec<f32> = columns.iter().map(|(name, _)| measure(*name)).collect();
    let (mut groups, mut rows) = (0.0f32, 0.0f32);
    for unit in UNITS {
        let mut any = false;
        for trace in win.traces.iter().filter(|t| t.series.unit() == unit) {
            any = true;
            rows += 1.0;
            // dot, its gap, the name
            names = names.max(22.0 + measure(trace.series.label()));
            if let Some(row) = stats_row(&trace.points, from, to) {
                for (width, (_, index)) in cols.iter_mut().zip(columns.iter()) {
                    *width = width.max(measure(value_text(row[*index], unit, o).as_str()));
                }
            }
        }
        if any {
            groups += 1.0;
            names = names.max(measure(group_title(unit)));
        }
    }
    SideMetrics { names, cols, groups, rows, size, row_spacing: o.row_spacing }
}

/// The switch for each line, with its lowest, average and highest. The panel makes its text
/// smaller to fit when it is made narrow or short (down to a point, then it scrolls), and when it
/// is thin it puts the numbers of each line on a row under its name, which keeps the text bigger.
fn side(ui: &mut egui::Ui, win: &mut LogWindow) {
    let size = ui.style().text_styles[&egui::TextStyle::Body].size;
    let metrics = side_metrics(ui, win, size);
    let (avail_w, avail_h) = (ui.available_width(), ui.available_height());
    let fit_of = |(width, height): (f32, f32)| {
        (avail_w / width).min(avail_h / height).clamp(SIDE_MIN_FIT, 1.0)
    };
    let (fit_table, fit_stacked) = (fit_of(metrics.table()), fit_of(metrics.stacked()));
    // the table, unless stacking is clearly the bigger text
    let stacked = fit_stacked > fit_table + 0.04;
    let fit = if stacked { fit_stacked } else { fit_table };
    ui.scope(|ui| {
        for font in ui.style_mut().text_styles.values_mut() {
            font.size *= fit;
        }
        let spacing = ui.spacing_mut();
        spacing.item_spacing *= fit;
        spacing.button_padding *= fit;
        side_content(ui, win, fit, stacked);
    });
}

/// The dot and name of a line: clicking them switches the line off or on. Returns whether they
/// were clicked.
fn name_cell(
    ui: &mut egui::Ui,
    series: Series,
    off: bool,
    ink: Color32,
    fit: f32,
    hover: &mut Option<Series>,
) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let (dot, response) = ui.allocate_exact_size(vec2(14.0 * fit, 14.0 * fit), Sense::click());
        if off {
            ui.painter().circle_stroke(
                dot.center(),
                5.5 * fit,
                Stroke::new(1.5, with_alpha(ink, 140)),
            );
        } else {
            ui.painter().circle_filled(dot.center(), 6.0 * fit, ink);
        }
        let text_colour = if off { theme::text_dim().gamma_multiply(0.7) } else { theme::text() };
        let label = ui.add(
            egui::Label::new(RichText::new(series.label()).color(text_colour))
                .selectable(false)
                .sense(Sense::click()),
        );
        clicked = response.clicked() || label.clicked();
        if (response.hovered() || label.hovered()) && !off {
            *hover = Some(series);
        }
    });
    if clicked {
        theme::set_open(&hidden_key(series), !off);
    }
    clicked
}

/// The numbers of one line as text (`-` where it has none).
fn number_text(
    row: Option<[f32; 4]>,
    index: usize,
    unit: &str,
    o: &Options,
    off: bool,
) -> RichText {
    match row {
        Some(row) => {
            let text = RichText::new(value_text(row[index], unit, o));
            if off {
                text.weak()
            } else {
                text.strong()
            }
        }
        None => RichText::new("-").weak(),
    }
}

/// The numbers with each line's on a row of their own under its name.
#[allow(clippy::too_many_arguments)]
fn stacked_rows(
    ui: &mut egui::Ui,
    win: &LogWindow,
    o: &Options,
    columns: &[(&'static str, usize)],
    from: f64,
    to: f64,
    fit: f32,
    hover: &mut Option<Series>,
) {
    let size = ui.style().text_styles[&egui::TextStyle::Body].size;
    let metrics = side_metrics(ui, win, size);
    let indent = 16.0 * fit;
    let line = ui.text_style_height(&egui::TextStyle::Body);
    let cell = |ui: &mut egui::Ui, width: f32, text: RichText| {
        ui.allocate_ui_with_layout(
            vec2(width, line),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add(egui::Label::new(text).selectable(false).extend());
            },
        );
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0 * fit;
        ui.add_space(indent);
        for ((name, _), width) in columns.iter().zip(metrics.cols.iter()) {
            cell(ui, *width, RichText::new(*name).weak());
        }
    });
    for unit in UNITS {
        if !win.traces.iter().any(|t| t.series.unit() == unit) {
            continue;
        }
        ui.add_space(4.0 * fit);
        ui.add(
            egui::Label::new(
                RichText::new(group_title(unit)).strong().color(theme::accent_stroke()),
            )
            .selectable(false),
        );
        for trace in win.traces.iter().filter(|t| t.series.unit() == unit) {
            let off = is_hidden(trace.series);
            name_cell(ui, trace.series, off, colour(trace.series, o), fit, hover);
            let row = stats_row(&trace.points, from, to);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0 * fit;
                ui.add_space(indent);
                for ((_, index), width) in columns.iter().zip(metrics.cols.iter()) {
                    cell(ui, *width, number_text(row, *index, unit, o, off));
                }
            });
        }
    }
}

fn side_content(ui: &mut egui::Ui, win: &mut LogWindow, fit: f32, stacked: bool) {
    let o = win.opts.clone();
    let (from, to) =
        if o.stats_whole { (0.0, win.total.max(lc::MIN_SPAN)) } else { (win.span.lo, win.span.hi) };
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button("All on").clicked() {
            for t in &win.traces {
                theme::set_open(&hidden_key(t.series), false);
            }
        }
        if ui.button("All off").clicked() {
            for t in &win.traces {
                theme::set_open(&hidden_key(t.series), true);
            }
        }
        if ui.button("Copy numbers").on_hover_text("Copy the table (for a spreadsheet)").clicked() {
            ui.ctx().copy_text(numbers_text(win, from, to));
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Numbers for").weak());
        ui.selectable_value(&mut win.opts.stats_whole, false, "On screen");
        ui.selectable_value(&mut win.opts.stats_whole, true, "Whole log");
    });
    ui.add_space(2.0);
    let columns = columns(&o);
    let mut hover: Option<Series> = None;
    let panel_width = ui.available_width();
    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        if stacked {
            stacked_rows(ui, win, &o, &columns, from, to, fit, &mut hover);
        } else {
            egui::Grid::new("logwin_stats")
                .num_columns(1 + columns.len())
                .spacing([18.0 * fit, o.row_spacing * fit])
                .show(ui, |ui| {
                    for unit in UNITS {
                        if !win.traces.iter().any(|t| t.series.unit() == unit) {
                            continue;
                        }
                        ui.add(
                            egui::Label::new(
                                RichText::new(group_title(unit))
                                    .strong()
                                    .color(theme::accent_stroke()),
                            )
                            .selectable(false),
                        );
                        for (name, _) in &columns {
                            ui.add(egui::Label::new(RichText::new(*name).weak()).selectable(false));
                        }
                        ui.end_row();
                        for trace in win.traces.iter().filter(|t| t.series.unit() == unit) {
                            let off = is_hidden(trace.series);
                            name_cell(
                                ui,
                                trace.series,
                                off,
                                colour(trace.series, &o),
                                fit,
                                &mut hover,
                            );
                            let row = stats_row(&trace.points, from, to);
                            for (_, index) in &columns {
                                let text = number_text(row, *index, unit, &o, off);
                                ui.add(egui::Label::new(text).selectable(false));
                            }
                            ui.end_row();
                        }
                        // a little room before the next group
                        ui.add_space(4.0 * fit);
                        ui.end_row();
                    }
                });
        }
        if o.side_hint {
            ui.add_space(10.0);
            ui.scope(|ui| {
                ui.set_max_width(panel_width);
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "Drag a graph to move. Scroll to zoom. Double-click to show everything. \
                             Arrow keys move, + and - zoom. Drag the strip underneath to move \
                             through the log. Click a name to switch its line off or on.",
                        )
                        .weak(),
                    )
                    .wrap()
                    .selectable(false),
                );
            });
        }
    });
    win.hover = hover;
}

/// The graphs that have a line switched on: (unit, its series that are shown).
fn groups(win: &LogWindow) -> Vec<(&'static str, Vec<usize>)> {
    UNITS
        .into_iter()
        .enumerate()
        .filter(|(index, _)| win.opts.graph_on[*index])
        .filter_map(|(_, unit)| {
            let shown: Vec<usize> = win
                .traces
                .iter()
                .enumerate()
                .filter(|(_, t)| t.series.unit() == unit && !is_hidden(t.series))
                .map(|(i, _)| i)
                .collect();
            (!shown.is_empty()).then_some((unit, shown))
        })
        .collect()
}

/// The part of a graph's rectangle that holds the lines. The margins grow with the text.
fn plot_of(rect: Rect, last: bool, o: &Options) -> Rect {
    let axis = o.factor(o.size_axis);
    let titles = o.factor(o.size_titles);
    let left = 20.0 + 48.0 * axis;
    let top = 14.0 + 18.0 * titles;
    let bottom = if last { 12.0 + 16.0 * axis } else { 8.0 };
    Rect::from_min_max(
        pos2(rect.left() + left, rect.top() + top),
        pos2(rect.right() - PLOT_RIGHT, rect.bottom() - bottom),
    )
}

fn charts(ui: &mut egui::Ui, win: &mut LogWindow) {
    let o = win.opts.clone();
    let groups = groups(win);
    if groups.is_empty() {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.add(
                egui::Label::new(
                    RichText::new("Nothing to show. Switch a line on in the numbers panel.").weak(),
                )
                .selectable(false),
            );
        });
        return;
    }
    let n = groups.len();
    let gap = o.chart_gap;
    let height = ((ui.available_height() - gap * (n as f32 - 1.0)) / n as f32).max(90.0);
    let width = ui.available_width();
    ui.spacing_mut().item_spacing.y = 0.0;

    // first the places and what is done to them, so that every graph can show the shared crosshair
    let mut cells: Vec<(Rect, egui::Response)> = Vec::with_capacity(n);
    for _ in 0..n {
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click_and_drag());
        cells.push((rect, response));
        ui.add_space(gap);
    }
    let span = win.span;
    let total = win.total;
    let pointer = ui.input(|i| i.pointer.hover_pos());
    let (scroll, zoom) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
    let sign = if o.wheel_invert { -1.0f32 } else { 1.0 };
    let speed = f64::from(o.zoom_speed / 100.0 * sign);
    let mut pan_px = 0.0f32;
    let mut factor = 1.0f64;
    let mut around = (span.lo + span.hi) / 2.0;
    for (i, (rect, response)) in cells.iter().enumerate() {
        let plot = plot_of(*rect, i + 1 == n, &o);
        if plot.width() < 40.0 {
            continue;
        }
        let time_at = |x: f32| {
            span.lo + f64::from(((x - plot.left()) / plot.width()).clamp(0.0, 1.0)) * span.width()
        };
        // where the crosshair stands: under the pointer, or on the nearest reading of the first line
        let snapped = |t: f64| -> f64 {
            if !o.snap {
                return t;
            }
            groups[i]
                .1
                .first()
                .and_then(|&index| lc::nearest(&win.traces[index].points, t))
                .map_or(t, |(tt, _)| tt)
        };
        if let Some(p) = pointer.filter(|p| rect.contains(*p)) {
            let t = snapped(time_at(p.x));
            win.cursor = Some(t);
            around = time_at(p.x);
            // the wheel zooms or moves (a pinch and ctrl + wheel zoom either way)
            if scroll.y != 0.0 {
                match o.wheel {
                    Wheel::Zoom => factor *= (-f64::from(scroll.y) * 0.004 * speed).exp(),
                    Wheel::Move => pan_px -= scroll.y * (o.zoom_speed / 100.0) * sign,
                }
            }
            if zoom != 1.0 {
                factor /= f64::from(zoom);
            }
            pan_px -= scroll.x;
        }
        if response.dragged() {
            pan_px -= response.drag_delta().x;
            if let Some(p) = response.interact_pointer_pos() {
                let t = snapped(time_at(p.x));
                win.cursor = Some(t);
            }
        }
        if response.double_clicked() {
            win.span = Span::all(total);
        }
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(if response.dragged() {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::Grab
            });
        }
    }
    let plot_width = plot_of(cells[0].0, n == 1, &o).width().max(1.0);
    if pan_px != 0.0 {
        win.span = win.span.panned(f64::from(pan_px) / f64::from(plot_width) * span.width(), total);
    }
    if (factor - 1.0).abs() > 1e-6 {
        win.span = win.span.zoomed(factor, around, total);
    }

    // then the drawing
    let span = win.span;
    for (i, ((rect, _), (unit, shown))) in cells.iter().zip(groups.iter()).enumerate() {
        draw_chart(ui, win, *rect, i + 1 == n, unit, shown, span);
    }
}

fn draw_chart(
    ui: &egui::Ui,
    win: &LogWindow,
    area: Rect,
    last: bool,
    unit: &str,
    shown: &[usize],
    span: Span,
) {
    let o = &win.opts;
    let tick_font = FontId::proportional(TICK_FONT * o.factor(o.size_axis));
    let painter = ui.painter_at(area);
    let level = o.chart_bg as u8;
    painter.rect_filled(
        area,
        CornerRadius::same(o.chart_corner as u8),
        Color32::from_rgb(level, level.saturating_add(2), level.saturating_add(6)),
    );
    painter.text(
        area.left_top() + vec2(12.0, 7.0),
        Align2::LEFT_TOP,
        format!("{} ({})", group_title(unit), unit_name(unit, o)),
        FontId::proportional(TITLE_FONT * o.factor(o.size_titles)),
        theme::text(),
    );
    let plot = plot_of(area, last, o);
    if plot.width() < 40.0 || plot.height() < 20.0 {
        return;
    }
    // the colour of a line, dimmed while the pointer is over another one's name
    let ink = |series: Series| -> Color32 {
        let base = colour(series, o);
        if o.hover_highlight && win.hover.is_some_and(|h| h != series) {
            with_alpha(base, 50)
        } else {
            base
        }
    };

    // what is on screen of each line, thinned to about a point per pixel
    struct Drawn<'a> {
        series: Series,
        visible: &'a [(f64, f32)],
        thin: Vec<(f64, f32)>,
        /// What the line goes through (steps when that is chosen).
        line: Vec<(f64, f32)>,
    }
    let drawn: Vec<Drawn> = shown
        .iter()
        .map(|&i| {
            let trace = &win.traces[i];
            let visible = lc::visible(&trace.points, span.lo, span.hi);
            let thin = lc::downsample(visible, plot.width() as usize);
            let line = if o.step { lo::stepped(&thin) } else { thin.clone() };
            Drawn { series: trace.series, visible, thin, line }
        })
        .collect();

    // ---- vertical axis ----
    let axis = if unit == "%" {
        hist::Axis { lo: 0.0, hi: 100.0, ticks: vec![0.0, 25.0, 50.0, 75.0, 100.0] }
    } else {
        let (mut low, mut high) = (f32::MAX, f32::MIN);
        for (d, &i) in drawn.iter().zip(shown.iter()) {
            let points: &[(f64, f32)] =
                if o.y_mode == YMode::Whole { &win.traces[i].points } else { d.visible };
            if let Some((l, h)) = hist::value_range(points) {
                low = low.min(l);
                high = high.max(h);
            }
        }
        if low > high {
            (low, high) = (0.0, 1.0);
        }
        if o.y_mode == YMode::Zero {
            low = low.min(0.0);
        }
        hist::axis(low, high, 4, min_span(unit, o))
    };
    let vspan = (axis.hi - axis.lo).max(f32::EPSILON);
    let x_of = |t: f64| plot.left() + (((t - span.lo) / span.width()) as f32) * plot.width();
    let y_of = |v: f32| plot.bottom() - ((v - axis.lo) / vspan).clamp(0.0, 1.0) * plot.height();

    // ---- grid and labels ----
    let grid = |base: f32| {
        Stroke::new(
            1.0,
            Color32::from_white_alpha((base * o.grid_percent / 100.0).min(255.0) as u8),
        )
    };
    for tick in &axis.ticks {
        let y = y_of(*tick);
        if o.grid_h {
            painter.line_segment([pos2(plot.left(), y), pos2(plot.right(), y)], grid(16.0));
        }
        painter.text(
            pos2(plot.left() - 6.0, y),
            Align2::RIGHT_CENTER,
            tick_text(*tick),
            tick_font.clone(),
            theme::text_dim(),
        );
    }
    let wanted = ((plot.width() / (90.0 * o.factor(o.size_axis).max(0.8))) as usize).max(2);
    let ticks = lc::time_ticks(win.clock_origin(), span.lo, span.hi, wanted);
    let step = lc::tick_step(span.width(), wanted);
    for t in &ticks {
        let x = x_of(*t);
        if o.grid_v {
            painter.line_segment([pos2(x, plot.top()), pos2(x, plot.bottom())], grid(9.0));
        }
        if last {
            let text = if win.clock {
                lc::clock_text(win.start_unix, *t, step < 60.0 || o.seconds)
            } else {
                lc::elapsed_text(*t)
            };
            painter.text(
                pos2(x, plot.bottom() + 4.0),
                Align2::CENTER_TOP,
                text,
                tick_font.clone(),
                theme::text_dim(),
            );
        }
    }

    // ---- lines: fills first so that every line stays on top ----
    let clipped = painter.with_clip_rect(plot.expand2(vec2(2.0, 4.0)));
    let screen: Vec<(&Drawn, Vec<egui::Pos2>)> = drawn
        .iter()
        .map(|d| (d, d.line.iter().map(|(t, v)| pos2(x_of(*t), y_of(*v))).collect::<Vec<_>>()))
        .collect();
    let fill_alpha = (o.fill_percent / 100.0 * 255.0) as u8;
    if o.fill && fill_alpha > 0 {
        for (d, points) in &screen {
            if dashed(d.series) || points.len() < 2 {
                continue;
            }
            let base = ink(d.series);
            let mut mesh = egui::Mesh::default();
            for p in points {
                mesh.colored_vertex(*p, with_alpha(base, fill_alpha));
                mesh.colored_vertex(pos2(p.x, plot.bottom()), with_alpha(base, 0));
            }
            for i in 0..points.len() - 1 {
                let a = (i * 2) as u32;
                mesh.add_triangle(a, a + 1, a + 2);
                mesh.add_triangle(a + 1, a + 3, a + 2);
            }
            clipped.add(Shape::mesh(mesh));
        }
    }
    for (d, points) in &screen {
        let base = ink(d.series);
        let boost = if o.hover_highlight && win.hover == Some(d.series) { 1.7 } else { 1.0 };
        let width = o.line_width * boost;
        if dashed(d.series) {
            clipped.extend(Shape::dashed_line(points, Stroke::new(width * 0.85, base), 6.0, 4.0));
            continue;
        }
        if points.len() >= 2 {
            if o.glow {
                clipped.add(Shape::line(
                    points.clone(),
                    Stroke::new(width * 2.8, with_alpha(base, 34)),
                ));
            }
            clipped.add(Shape::line(points.clone(), Stroke::new(width, base)));
        }
        // dots on the readings
        let crowded = d.visible.len() as f32 * 7.0 >= plot.width();
        let dots = match o.dots {
            Dots::Auto => !crowded,
            Dots::Always => d.thin.len() <= 4000,
            Dots::Never => false,
        };
        if dots {
            for (t, v) in &d.thin {
                clipped.circle_filled(pos2(x_of(*t), y_of(*v)), o.dot_size, base);
            }
        }
    }

    // ---- the average, and the highest and lowest reading of each line ----
    if o.avg_line {
        for (d, &i) in drawn.iter().zip(shown.iter()) {
            if let Some((_, avg, _)) = lc::stats_in(&win.traces[i].points, span.lo, span.hi) {
                let y = y_of(avg);
                let base = ink(d.series);
                clipped.extend(Shape::dashed_line(
                    &[pos2(plot.left(), y), pos2(plot.right(), y)],
                    Stroke::new(1.0, with_alpha(base, 150)),
                    2.0,
                    5.0,
                ));
                painter.text(
                    pos2(plot.right() - 4.0, y - 2.0),
                    Align2::RIGHT_BOTTOM,
                    format!("avg {}", value_text(avg, unit, o)),
                    FontId::proportional(TICK_FONT * o.factor(o.size_axis) * 0.9),
                    with_alpha(base, 220),
                );
            }
        }
    }
    if o.peaks {
        let small = FontId::proportional(TICK_FONT * o.factor(o.size_axis) * 0.9);
        for d in &drawn {
            let Some((low, high)) = lo::extremes(d.visible) else {
                continue;
            };
            if (high.1 - low.1).abs() < f32::EPSILON {
                continue;
            }
            let base = ink(d.series);
            for (point, above) in [(high, true), (low, false)] {
                if point.0 < span.lo || point.0 > span.hi {
                    continue;
                }
                let p = pos2(x_of(point.0), y_of(point.1));
                clipped.circle_stroke(p, 5.0, Stroke::new(1.5, base));
                painter.text(
                    p + vec2(0.0, if above { -8.0 } else { 8.0 }),
                    if above { Align2::CENTER_BOTTOM } else { Align2::CENTER_TOP },
                    value_text(point.1, unit, o),
                    small.clone(),
                    base,
                );
            }
        }
    }

    // ---- the horizontal guide under the pointer, with its value on the axis ----
    let hover_pos = ui.input(|i| i.pointer.hover_pos());
    let inside = hover_pos.is_some_and(|p| area.contains(p));
    if o.guide_h && inside {
        if let Some(p) = hover_pos.filter(|p| plot.contains(*p)) {
            painter.line_segment(
                [pos2(plot.left(), p.y), pos2(plot.right(), p.y)],
                Stroke::new(1.0, Color32::from_white_alpha(60)),
            );
            let value = axis.lo + (plot.bottom() - p.y) / plot.height() * vspan;
            let font = tick_font.clone();
            let text = tick_text_fine(value);
            let size = painter.layout_no_wrap(text.clone(), font.clone(), theme::text()).size();
            let at = Rect::from_min_size(
                pos2(plot.left() - 8.0 - size.x, p.y - size.y / 2.0 - 1.0),
                size + vec2(4.0, 2.0),
            );
            painter.rect_filled(at, CornerRadius::same(3), Color32::from_rgb(40, 44, 52));
            painter.text(at.center(), Align2::CENTER_CENTER, text, font, theme::text());
        }
    }

    // ---- the shared crosshair and what it reads ----
    if !o.crosshair && !o.readout {
        return;
    }
    let Some(t) = win.cursor.filter(|t| *t >= span.lo && *t <= span.hi) else {
        return;
    };
    let x = x_of(t);
    if o.crosshair {
        clipped.line_segment(
            [pos2(x, plot.top()), pos2(x, plot.bottom())],
            Stroke::new(1.0, Color32::from_white_alpha(90)),
        );
    }
    let typical_gap = drawn
        .iter()
        .filter(|d| d.visible.len() > 1)
        .map(|d| (d.visible[d.visible.len() - 1].0 - d.visible[0].0) / (d.visible.len() - 1) as f64)
        .fold(f64::MAX, f64::min);
    let reach = (span.width() / f64::from(plot.width()) * 14.0).max(if typical_gap.is_finite() {
        typical_gap * 1.5
    } else {
        0.0
    });
    let mut rows: Vec<(Color32, String)> = Vec::new();
    for d in &drawn {
        if let Some((tt, v)) = lc::nearest(d.visible, t).filter(|(tt, _)| (tt - t).abs() <= reach) {
            let p = pos2(x_of(tt), y_of(v));
            if o.crosshair {
                clipped.circle_filled(p, 4.0, ink(d.series));
                clipped.circle_stroke(p, 5.5, Stroke::new(1.0, Color32::from_white_alpha(160)));
            }
            rows.push((
                colour(d.series, o),
                format!("{}  {}", d.series.label(), value_text(v, unit, o)),
            ));
        }
    }
    // only the graph under the pointer gets the read-out box
    if o.readout && inside {
        let k = o.factor(o.size_readout);
        let head = win.moment_text(t);
        let font = FontId::proportional(READOUT_FONT * k);
        let line_h = 21.0 * k;
        let mut width = painter.layout_no_wrap(head.clone(), font.clone(), theme::text()).size().x;
        for (_, text) in &rows {
            width = width.max(
                painter.layout_no_wrap(text.clone(), font.clone(), theme::text()).size().x + 20.0,
            );
        }
        let size = vec2(width + 22.0, line_h * (rows.len() as f32 + 1.0) + 12.0);
        let mut min = pos2(x + 14.0, plot.top() + 4.0);
        if min.x + size.x > plot.right() {
            min.x = x - 14.0 - size.x;
        }
        let boxed = Rect::from_min_size(min, size);
        painter.rect_filled(
            boxed,
            CornerRadius::same(6),
            Color32::from_rgba_unmultiplied(12, 14, 18, (o.readout_opacity / 100.0 * 255.0) as u8),
        );
        painter.rect_stroke(
            boxed,
            CornerRadius::same(6),
            Stroke::new(1.0, Color32::from_white_alpha(40)),
            StrokeKind::Inside,
        );
        painter.text(
            boxed.left_top() + vec2(9.0, 6.0),
            Align2::LEFT_TOP,
            head,
            font.clone(),
            theme::text(),
        );
        for (i, (base, text)) in rows.iter().enumerate() {
            let y = boxed.top() + 5.0 + line_h * (i as f32 + 1.0);
            painter.circle_filled(pos2(boxed.left() + 13.0, y + line_h / 2.0), 4.0, *base);
            painter.text(
                pos2(boxed.left() + 26.0, y),
                Align2::LEFT_TOP,
                text,
                font.clone(),
                theme::text(),
            );
        }
    }
}

/// The value under the guide, with a decimal where the axis is fine.
fn tick_text_fine(value: f32) -> String {
    format!("{:.1}", value)
}

/// The whole log in one thin strip; the part on screen is the lit window in it. Drag it to move,
/// click or drag outside it to jump.
fn overview(ui: &mut egui::Ui, win: &mut LogWindow) {
    let o = win.opts.clone();
    let k = o.factor(o.size_overview);
    // room under the strip for the two times
    let room = 8.0 + 14.0 * k;
    let width = ui.available_width();
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, o.overview_height), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    let level = o.chart_bg as u8;
    painter.rect_filled(
        rect,
        CornerRadius::same(6),
        Color32::from_rgb(level, level.saturating_add(2), level.saturating_add(6)),
    );
    let plot = Rect::from_min_max(
        pos2(rect.left() + 20.0 + 48.0 * o.factor(o.size_axis), rect.top() + 6.0),
        pos2(rect.right() - PLOT_RIGHT, rect.bottom() - room),
    );
    if plot.width() < 40.0 {
        return;
    }
    let total = win.total;
    let x_of = |t: f64| plot.left() + ((t / total) as f32).clamp(0.0, 1.0) * plot.width();
    let t_of = |x: f32| f64::from(((x - plot.left()) / plot.width()).clamp(0.0, 1.0)) * total;

    // every line that is on, each scaled to its own range so that all of them are seen
    for trace in win.traces.iter().filter(|t| !is_hidden(t.series)) {
        let Some((lo, hi)) = hist::value_range(&trace.points) else {
            continue;
        };
        let range = (hi - lo).max(f32::EPSILON);
        let thin = lc::downsample(&trace.points, (plot.width() / 2.0) as usize);
        let points: Vec<egui::Pos2> = thin
            .iter()
            .map(|(t, v)| pos2(x_of(*t), plot.bottom() - ((*v - lo) / range) * plot.height()))
            .collect();
        if points.len() >= 2 {
            painter.add(Shape::line(
                points,
                Stroke::new(1.0, with_alpha(colour(trace.series, &o), 150)),
            ));
        }
    }

    let shown = Rect::from_min_max(
        pos2(x_of(win.span.lo), rect.top() + 2.0),
        pos2(x_of(win.span.hi).max(x_of(win.span.lo) + 4.0), rect.bottom() - room + 2.0),
    );
    // what is off screen is dimmed
    let dim = Color32::from_rgba_unmultiplied(0, 0, 0, 120);
    painter.rect_filled(
        Rect::from_min_max(pos2(plot.left(), shown.top()), pos2(shown.left(), shown.bottom())),
        0.0,
        dim,
    );
    painter.rect_filled(
        Rect::from_min_max(pos2(shown.right(), shown.top()), pos2(plot.right(), shown.bottom())),
        0.0,
        dim,
    );
    painter.rect_stroke(
        shown,
        CornerRadius::same(3),
        Stroke::new(1.5, theme::accent_stroke()),
        StrokeKind::Inside,
    );
    for (x, align) in [(plot.left(), Align2::LEFT_TOP), (plot.right(), Align2::RIGHT_TOP)] {
        let t = if align == Align2::LEFT_TOP { 0.0 } else { total };
        let text =
            if win.clock { lc::clock_text(win.start_unix, t, true) } else { lc::elapsed_text(t) };
        painter.text(
            pos2(x, rect.bottom() - room + 3.0),
            align,
            text,
            FontId::proportional(TICK_FONT * k),
            theme::text_dim(),
        );
    }

    // ---- moving it ----
    if response.drag_started() {
        let inside = response.interact_pointer_pos().is_some_and(|p| shown.contains(p));
        win.overview_drag = Some(inside);
    }
    if response.dragged() {
        match (win.overview_drag, response.interact_pointer_pos()) {
            (Some(true), _) => {
                let seconds = f64::from(response.drag_delta().x) / f64::from(plot.width()) * total;
                win.span = win.span.panned(seconds, total);
            }
            (_, Some(p)) => win.span = win.span.centred(t_of(p.x), total),
            _ => {}
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if response.clicked() {
        if let Some(p) = response.interact_pointer_pos() {
            win.span = win.span.centred(t_of(p.x), total);
        }
    }
    if response.drag_stopped() {
        win.overview_drag = None;
    }
    if response.double_clicked() {
        win.span = Span::all(total);
    }
}

// ---- the options panel ----

/// A text size, in percent.
fn size_slider(ui: &mut egui::Ui, value: &mut f32, label: &str) {
    ui.add(
        egui::Slider::new(value, lo::PERCENT_MIN..=lo::PERCENT_MAX)
            .suffix(" %")
            .integer()
            .text(label),
    );
}

fn slider(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
    label: &str,
) {
    ui.add(egui::Slider::new(value, range).suffix(suffix).text(label));
}

fn note(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).weak()).wrap().selectable(false));
}

fn options_panel(ui: &mut egui::Ui, win: &mut LogWindow) {
    let t = &mut win.opts;
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::Label::new(RichText::new("Options").strong().color(theme::accent_stroke()))
                .selectable(false),
        );
        if ui
            .button("Reset everything")
            .on_hover_text("All of these back to how they were")
            .clicked()
        {
            *t = Options::default();
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("All text").weak());
        for (name, preset) in [
            ("Compact", Preset::Compact),
            ("Standard", Preset::Standard),
            ("Large", Preset::Large),
            ("Huge", Preset::Huge),
        ] {
            if ui.button(name).clicked() {
                t.preset(preset);
            }
        }
    });
    ui.add_space(4.0);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().slider_width = 160.0;

        theme::collapsing(ui, "logwin_opt/text", "Text sizes", true, |ui| {
            note(ui, "Each part has its own size. \"Everything\" scales all of them together.");
            size_slider(ui, &mut t.size_all, "Everything");
            size_slider(ui, &mut t.size_side, "Numbers panel");
            size_slider(ui, &mut t.size_toolbar, "Buttons on top");
            size_slider(ui, &mut t.size_options, "This options panel");
            size_slider(ui, &mut t.size_titles, "Graph titles");
            size_slider(ui, &mut t.size_axis, "Numbers and times on the axes");
            size_slider(ui, &mut t.size_readout, "Crosshair read-out");
            size_slider(ui, &mut t.size_overview, "Strip under the graphs");
        });

        theme::collapsing(ui, "logwin_opt/side", "Numbers panel", true, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Sits");
                ui.selectable_value(&mut t.side, Side::Left, "Left");
                ui.selectable_value(&mut t.side, Side::Right, "Right");
                ui.selectable_value(&mut t.side, Side::Hidden, "Hidden");
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Numbers for");
                ui.selectable_value(&mut t.stats_whole, false, "The part on screen");
                ui.selectable_value(&mut t.stats_whole, true, "The whole log");
            });
            ui.label("Columns");
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut t.col_low, "Lowest");
                ui.checkbox(&mut t.col_avg, "Average");
                ui.checkbox(&mut t.col_high, "Highest");
                ui.checkbox(&mut t.col_last, "Latest");
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Temperature decimals");
                for n in 0u8..=2 {
                    ui.selectable_value(&mut t.temp_decimals, n, n.to_string());
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Power decimals");
                for n in 0u8..=2 {
                    ui.selectable_value(&mut t.power_decimals, n, n.to_string());
                }
            });
            slider(ui, &mut t.row_spacing, 0.0..=24.0, " px", "Space between rows");
            ui.checkbox(&mut t.side_hint, "Show the how-to text under the numbers");
            ui.checkbox(&mut t.hover_highlight, "Pointing at a name lights its line up");
        });

        theme::collapsing(ui, "logwin_opt/units", "Units and times", false, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Temperature");
                ui.selectable_value(&mut t.fahrenheit, false, "\u{b0}C");
                ui.selectable_value(&mut t.fahrenheit, true, "\u{b0}F");
            });
            ui.checkbox(&mut t.seconds, "Always show seconds on the time axis");
        });

        theme::collapsing(ui, "logwin_opt/graphs", "Graphs", false, |ui| {
            ui.label("Which graphs");
            ui.horizontal_wrapped(|ui| {
                for (index, unit) in UNITS.into_iter().enumerate() {
                    ui.checkbox(&mut t.graph_on[index], group_title(unit));
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Vertical axis");
                ui.selectable_value(&mut t.y_mode, YMode::Visible, "Fits what is on screen");
                ui.selectable_value(&mut t.y_mode, YMode::Whole, "Fits the whole log");
                ui.selectable_value(&mut t.y_mode, YMode::Zero, "Reaches down to zero");
            });
            ui.checkbox(&mut t.grid_h, "Horizontal grid lines");
            ui.checkbox(&mut t.grid_v, "Vertical grid lines");
            slider(ui, &mut t.grid_percent, 0.0..=300.0, " %", "Grid strength");
            slider(ui, &mut t.chart_gap, 0.0..=30.0, " px", "Space between graphs");
            slider(ui, &mut t.chart_corner, 0.0..=24.0, " px", "Rounded corners");
            slider(ui, &mut t.chart_bg, 0.0..=60.0, "", "Background brightness");
        });

        theme::collapsing(ui, "logwin_opt/lines", "Lines", false, |ui| {
            slider(ui, &mut t.line_width, 0.5..=5.0, " px", "Line thickness");
            ui.checkbox(&mut t.glow, "Soft glow round the lines");
            ui.checkbox(&mut t.fill, "Colour under the lines");
            if t.fill {
                slider(ui, &mut t.fill_percent, 0.0..=100.0, " %", "Colour strength");
            }
            ui.checkbox(&mut t.step, "Steps instead of slopes (a reading holds until the next)");
            ui.horizontal_wrapped(|ui| {
                ui.label("Dots on readings");
                ui.selectable_value(&mut t.dots, Dots::Auto, "When zoomed in");
                ui.selectable_value(&mut t.dots, Dots::Always, "Always");
                ui.selectable_value(&mut t.dots, Dots::Never, "Never");
            });
            slider(ui, &mut t.dot_size, 1.0..=8.0, " px", "Dot size");
            ui.checkbox(&mut t.avg_line, "A line at the average of each line");
            ui.checkbox(&mut t.peaks, "Mark the highest and lowest reading of each line");
        });

        theme::collapsing(ui, "logwin_opt/cross", "Crosshair", false, |ui| {
            ui.checkbox(&mut t.crosshair, "Vertical line and dots following the pointer");
            ui.checkbox(&mut t.readout, "Read-out box with the exact time and values");
            ui.checkbox(&mut t.guide_h, "Horizontal line with its value on the axis");
            ui.checkbox(&mut t.snap, "Jump to the nearest reading");
            slider(ui, &mut t.readout_opacity, 30.0..=100.0, " %", "Read-out box opacity");
        });

        theme::collapsing(ui, "logwin_opt/strip", "Strip under the graphs", false, |ui| {
            ui.checkbox(&mut t.overview, "Show the strip");
            slider(ui, &mut t.overview_height, 40.0..=200.0, " px", "Height");
        });

        theme::collapsing(ui, "logwin_opt/mouse", "Mouse", false, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("The wheel over a graph");
                ui.selectable_value(&mut t.wheel, Wheel::Zoom, "Zooms");
                ui.selectable_value(&mut t.wheel, Wheel::Move, "Moves");
            });
            slider(ui, &mut t.zoom_speed, 20.0..=400.0, " %", "Wheel speed");
            ui.checkbox(&mut t.wheel_invert, "Turn the wheel the other way");
        });

        theme::collapsing(ui, "logwin_opt/colours", "Line colours", false, |ui| {
            for series in Series::ALL {
                ui.horizontal(|ui| {
                    let now = colour(series, t);
                    let mut rgb = [now.r(), now.g(), now.b()];
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        t.set_colour(series.column(), rgb);
                    }
                    ui.label(series.label());
                    if t.colour_of(series.column()).is_some()
                        && ui
                            .small_button("Standard")
                            .on_hover_text("The standard colour again")
                            .clicked()
                    {
                        t.reset_colour(series.column());
                    }
                });
            }
        });

        theme::collapsing(ui, "logwin_opt/window", "Window", false, |ui| {
            ui.checkbox(&mut t.on_top, "Keep this window above the others");
        });
        ui.add_space(12.0);
    });
}
