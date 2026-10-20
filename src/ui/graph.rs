// Sensors graphs: glowing lines over a gradient fill, a pulsing "now" dot, a hover read-out and a
// legend whose entries switch a line on and off.
use crate::config::GRAPH_WINDOWS;
use crate::sensor_history::{self as hist, History, Series};
use crate::theme;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, FontId, Rect, RichText, Sense, Shape, Stroke,
    StrokeKind,
};
use std::time::Duration;

/// Longest polyline drawn per line (older points are averaged together).
const MAX_DRAWN: usize = 360;
const CHART_HEIGHT: f32 = 150.0;
const LEFT_MARGIN: f32 = 40.0;
const BOTTOM_MARGIN: f32 = 16.0;

const ORANGE: Color32 = Color32::from_rgb(255, 159, 67);
const CYAN: Color32 = Color32::from_rgb(60, 200, 255);
const VIOLET: Color32 = Color32::from_rgb(190, 110, 255);
const AMBER: Color32 = Color32::from_rgb(255, 214, 84);
const CORAL: Color32 = Color32::from_rgb(255, 99, 99);
const LIME: Color32 = Color32::from_rgb(150, 220, 100);

struct Line {
    label: &'static str,
    color: Color32,
    /// Limits (PL1 / PL2) are drawn dashed and without fill.
    dashed: bool,
    points: Vec<(f64, f32)>,
}

struct Chart {
    title: &'static str,
    unit: &'static str,
    /// Fixed vertical range (percentages), otherwise it follows the data.
    fixed: Option<(f32, f32)>,
    min_span: f32,
    lines: Vec<Line>,
}

fn value_text(value: f32, unit: &str) -> String {
    match unit {
        "W" => format!("{:.1} {}", value, unit),
        _ => format!("{:.0} {}", value, unit),
    }
}

fn tick_text(value: f32) -> String {
    if (value - value.round()).abs() < 0.01 {
        format!("{:.0}", value)
    } else {
        format!("{:.1}", value)
    }
}

fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

fn build_charts(history: &History, seconds: f64) -> Vec<Chart> {
    let line = |series: Series, label: &'static str, color: Color32, dashed: bool| Line {
        label,
        color,
        dashed,
        points: history.series(series, seconds),
    };
    let accent = theme::accent_stroke();
    let charts = vec![
        Chart {
            title: "Temperature",
            unit: "\u{b0}C",
            fixed: None,
            min_span: 12.0,
            lines: vec![
                line(Series::CpuTemp, "CPU package", accent, false),
                line(Series::CpuCoreTemp, "Hottest core", LIME, false),
                line(Series::CpuZoneTemp, "Thermal zone", ORANGE, false),
                line(Series::GpuTemp, "GPU", CYAN, false),
                line(Series::GpuMemTemp, "GPU memory junction", VIOLET, false),
            ],
        },
        Chart {
            title: "Power",
            unit: "W",
            fixed: None,
            min_span: 20.0,
            lines: vec![
                line(Series::CpuPower, "CPU package", accent, false),
                line(Series::GpuPower, "GPU", CYAN, false),
                line(Series::Pl1, "PL1", AMBER, true),
                line(Series::Pl2, "PL2", CORAL, true),
            ],
        },
        Chart {
            title: "Clock speed",
            unit: "MHz",
            fixed: None,
            min_span: 400.0,
            lines: vec![
                line(Series::CpuClock, "CPU", accent, false),
                line(Series::CpuClockTop, "Fastest core", AMBER, false),
                line(Series::CpuClockP, "P-cores", LIME, false),
                line(Series::CpuClockE, "E-cores", VIOLET, false),
                line(Series::GpuClock, "GPU", CYAN, false),
            ],
        },
        Chart {
            title: "Load",
            unit: "%",
            fixed: Some((0.0, 100.0)),
            min_span: 1.0,
            lines: vec![
                line(Series::CpuLoad, "CPU", accent, false),
                line(Series::GpuLoad, "GPU", CYAN, false),
            ],
        },
    ];
    // a chart none of whose values exist on this machine is left out
    charts.into_iter().filter(|c| c.lines.iter().any(|l| !l.points.is_empty())).collect()
}

/// The graphs card. Returns true when the time span was changed.
pub fn render_graphs(ui: &mut egui::Ui, history: &History, window: &mut u8) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for (i, (label, _)) in GRAPH_WINDOWS.iter().enumerate() {
            if ui.selectable_label(usize::from(*window) == i, *label).clicked() {
                *window = i as u8;
                changed = true;
            }
        }
    });
    ui.separator();

    let seconds = f64::from(GRAPH_WINDOWS[usize::from(*window).min(GRAPH_WINDOWS.len() - 1)].1);
    draw_all(ui, history, seconds, true);
    changed
}

/// The graphs of a whole loaded log (`seconds` = its length). Nothing animates or repaints.
pub fn render_log_graphs(ui: &mut egui::Ui, history: &History, seconds: f64) {
    draw_all(ui, history, seconds.max(1.0), false);
}

/// The charts for the last `seconds` of `history`. `live`: the sensors are running (pulsing dot,
/// steady repaints); otherwise the picture is still.
fn draw_all(ui: &mut egui::Ui, history: &History, seconds: f64, live: bool) {
    let charts = build_charts(history, seconds);
    if charts.is_empty() {
        ui.add(
            egui::Label::new(
                RichText::new("Graphs appear once the first readings are in.").small().weak(),
            )
            .selectable(false),
        );
        return;
    }
    for chart in &charts {
        draw_chart(ui, chart, seconds, live);
        ui.add_space(8.0);
    }
    let note = if live {
        "Hover a graph for exact values. Click a name above a graph to hide or show \
         its line. The graphs keep the last hour while the sensors are on. Closing \
         this drop-down stops the drawing; the switch above stops the recording too."
    } else {
        "Hover a graph for exact values. Click a name above a graph to hide or show \
         its line. Times are counted back from the end of the log."
    };
    ui.add(egui::Label::new(RichText::new(note).small().weak()).selectable(false));
}

fn draw_chart(ui: &mut egui::Ui, chart: &Chart, window: f64, live: bool) {
    // ---- legend (also the on / off switches of the lines) ----
    let mut hidden = Vec::with_capacity(chart.lines.len());
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::Label::new(RichText::new(chart.title).strong()).selectable(false));
        ui.add_space(6.0);
        for line in &chart.lines {
            if line.points.is_empty() {
                hidden.push(true);
                continue;
            }
            // remembered between launches like the drop-down states ("open" = line hidden)
            let key = format!("graph_hidden/{}/{}", chart.title, line.label);
            let off = theme::get_open(&key).unwrap_or(false);
            hidden.push(off);
            let latest = line.points.last().map(|p| p.1).unwrap_or(0.0);
            let (dot, dot_response) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::click());
            let dot_color = if off { with_alpha(line.color, 60) } else { line.color };
            ui.painter().circle_filled(dot.center(), 4.0, dot_color);
            let text_color =
                if off { theme::text_dim().gamma_multiply(0.6) } else { theme::text() };
            let label = ui.add(
                egui::Label::new(
                    RichText::new(format!("{} {}", line.label, value_text(latest, chart.unit)))
                        .small()
                        .color(text_color),
                )
                .selectable(false)
                .sense(Sense::click()),
            );
            if dot_response.clicked() || label.clicked() {
                theme::set_open(&key, !off);
            }
            ui.add_space(8.0);
        }
    });

    // ---- plot area ----
    let (response, painter) =
        ui.allocate_painter(vec2(ui.available_width(), CHART_HEIGHT), Sense::hover());
    let area = response.rect;
    painter.rect_filled(area, CornerRadius::same(8), Color32::from_rgb(16, 18, 22));
    let plot = Rect::from_min_max(
        pos2(area.left() + LEFT_MARGIN, area.top() + 8.0),
        pos2(area.right() - 10.0, area.bottom() - BOTTOM_MARGIN),
    );
    if plot.width() < 40.0 {
        return;
    }

    let shown: Vec<&Line> = chart
        .lines
        .iter()
        .zip(hidden.iter())
        .filter(|(l, off)| !**off && !l.points.is_empty())
        .map(|(l, _)| l)
        .collect();
    if shown.is_empty() {
        painter.text(
            plot.center(),
            Align2::CENTER_CENTER,
            "nothing to show - turn a line on above",
            FontId::proportional(12.0),
            theme::text_dim(),
        );
        return;
    }

    // ---- vertical axis ----
    let axis = match chart.fixed {
        Some((lo, hi)) => hist::Axis { lo, hi, ticks: vec![lo, 25.0, 50.0, 75.0, hi] },
        None => {
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for line in &shown {
                if let Some((l, h)) = hist::value_range(&line.points) {
                    lo = lo.min(l);
                    hi = hi.max(h);
                }
            }
            hist::axis(lo, hi, 4, chart.min_span)
        }
    };
    let span = (axis.hi - axis.lo).max(f32::EPSILON);
    let x_of = |t: f64| plot.right() - ((-t / window) as f32).clamp(0.0, 1.0) * plot.width();
    let y_of = |v: f32| plot.bottom() - ((v - axis.lo) / span).clamp(0.0, 1.0) * plot.height();

    // ---- grid and labels ----
    let faint = Stroke::new(1.0, Color32::from_white_alpha(16));
    let fainter = Stroke::new(1.0, Color32::from_white_alpha(8));
    for tick in &axis.ticks {
        let y = y_of(*tick);
        painter.line_segment([pos2(plot.left(), y), pos2(plot.right(), y)], faint);
        painter.text(
            pos2(plot.left() - 6.0, y),
            Align2::RIGHT_CENTER,
            tick_text(*tick),
            FontId::proportional(10.0),
            theme::text_dim(),
        );
    }
    for step in 0..=4 {
        let fraction = step as f32 / 4.0;
        let x = plot.left() + fraction * plot.width();
        if step > 0 && step < 4 {
            painter.line_segment([pos2(x, plot.top()), pos2(x, plot.bottom())], fainter);
        }
        let (anchor, label_x) = match step {
            0 => (Align2::LEFT_TOP, x),
            4 => (Align2::RIGHT_TOP, x),
            _ => (Align2::CENTER_TOP, x),
        };
        painter.text(
            pos2(label_x, plot.bottom() + 3.0),
            anchor,
            hist::ago_label(-window * f64::from(1.0 - fraction)),
            FontId::proportional(10.0),
            theme::text_dim(),
        );
    }

    // ---- lines: fills first so that every line stays on top ----
    let drawn: Vec<(&Line, Vec<egui::Pos2>)> = shown
        .iter()
        .map(|line| {
            let points = hist::decimate(&line.points, MAX_DRAWN)
                .into_iter()
                .map(|(t, v)| pos2(x_of(t), y_of(v)))
                .collect();
            (*line, points)
        })
        .collect();
    let painter = painter.with_clip_rect(area);
    for (line, points) in &drawn {
        if line.dashed || points.len() < 2 {
            continue;
        }
        let mut mesh = egui::Mesh::default();
        for p in points {
            mesh.colored_vertex(*p, with_alpha(line.color, 70));
            mesh.colored_vertex(pos2(p.x, plot.bottom()), with_alpha(line.color, 0));
        }
        for i in 0..points.len() - 1 {
            let a = (i * 2) as u32;
            mesh.add_triangle(a, a + 1, a + 2);
            mesh.add_triangle(a + 1, a + 3, a + 2);
        }
        painter.add(Shape::mesh(mesh));
    }
    let time = ui.input(|i| i.time);
    for (line, points) in &drawn {
        if line.dashed {
            painter.extend(Shape::dashed_line(points, Stroke::new(1.5, line.color), 6.0, 4.0));
            continue;
        }
        if points.len() >= 2 {
            painter.add(Shape::line(points.clone(), Stroke::new(6.0, with_alpha(line.color, 38))));
            painter.add(Shape::line(points.clone(), Stroke::new(2.0, line.color)));
        }
        if let Some(last) = points.last() {
            // the "now" dot with a ring that spreads and fades
            if live {
                let phase = ((time * 1.1) % 1.0) as f32;
                let ring = Stroke::new(1.5, with_alpha(line.color, ((1.0 - phase) * 140.0) as u8));
                painter.circle_stroke(*last, 4.0 + 9.0 * phase, ring);
            }
            painter.circle_filled(*last, 3.5, line.color);
        }
    }
    if live {
        ui.ctx().request_repaint_after(Duration::from_millis(60));
    }

    // ---- hover read-out ----
    let Some(cursor) = response.hover_pos().filter(|p| plot.contains(*p)) else {
        return;
    };
    let t_cursor = -f64::from((plot.right() - cursor.x) / plot.width()) * window;
    let reach = (window / f64::from(plot.width()) * 12.0).max(2.5);
    painter.line_segment(
        [pos2(cursor.x, plot.top()), pos2(cursor.x, plot.bottom())],
        Stroke::new(1.0, Color32::from_white_alpha(60)),
    );
    let mut rows: Vec<(Color32, String)> = Vec::new();
    for line in &shown {
        if let Some((t, v)) = hist::value_near(&line.points, t_cursor, reach) {
            let at = pos2(x_of(t), y_of(v));
            painter.circle_filled(at, 4.0, line.color);
            painter.circle_stroke(at, 4.5, Stroke::new(1.0, Color32::WHITE));
            rows.push((line.color, format!("{}  {}", line.label, value_text(v, chart.unit))));
        }
    }
    if rows.is_empty() {
        return;
    }
    let row_height = 15.0;
    let size = vec2(170.0, 22.0 + row_height * rows.len() as f32);
    let mut left = cursor.x + 14.0;
    if left + size.x > plot.right() {
        left = cursor.x - 14.0 - size.x;
    }
    let top = (cursor.y - size.y / 2.0).clamp(plot.top(), (plot.bottom() - size.y).max(plot.top()));
    let tip = Rect::from_min_size(pos2(left, top), size);
    painter.rect(
        tip,
        CornerRadius::same(6),
        Color32::from_black_alpha(225),
        Stroke::new(1.0, theme::border_light()),
        StrokeKind::Inside,
    );
    painter.text(
        pos2(tip.left() + 8.0, tip.top() + 6.0),
        Align2::LEFT_TOP,
        hist::ago_label(t_cursor),
        FontId::proportional(10.0),
        theme::text_dim(),
    );
    for (i, (color, text)) in rows.iter().enumerate() {
        let y = tip.top() + 24.0 + row_height * i as f32 + row_height / 2.0;
        painter.circle_filled(pos2(tip.left() + 12.0, y), 3.5, *color);
        painter.text(
            pos2(tip.left() + 22.0, y),
            Align2::LEFT_CENTER,
            text,
            FontId::proportional(11.0),
            theme::text(),
        );
    }
}
