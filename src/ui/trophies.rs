//! Options > Trophies: every trophy with what it takes, how far along you are, and when each was
//! earned. A hidden trophy is just a grey "?" until it is earned (see `trophies.rs`).
use crate::config::Config;
use crate::trophies::{self, Line, Tier};
use eframe::egui::{self, vec2, Color32, RichText, Stroke};

/// The cup of one trophy: a coin in the tier's colour once earned, a grey ring while it is not,
/// and a "?" in a ring while it is hidden. `size` is how wide it is.
fn cup(ui: &mut egui::Ui, tier: Tier, earned: bool, secret: bool, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    let painter = ui.painter();
    let c = rect.center();
    let k = size / 26.0;
    if earned {
        let [r, g, b] = tier.colour();
        let colour = Color32::from_rgb(r, g, b);
        painter.circle_filled(c, 11.0 * k, colour);
        painter.circle_stroke(c, 8.0 * k, Stroke::new(1.5 * k, Color32::from_black_alpha(90)));
        painter.circle_filled(c, 3.0 * k, Color32::from_black_alpha(110));
        painter.circle_stroke(c, 11.0 * k, Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.6)));
    } else {
        painter.circle_stroke(c, 10.5 * k, Stroke::new(1.5, crate::theme::border_light()));
        if secret {
            painter.text(c, egui::Align2::CENTER_CENTER, "?", egui::FontId::proportional(14.0 * k), crate::theme::text_dim());
        }
    }
}

fn row(ui: &mut egui::Ui, line: &Line) {
    ui.horizontal(|ui| {
        cup(ui, line.tier, line.earned, line.secret, 26.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            let name = RichText::new(&line.name).strong();
            let name = if line.earned { name.color(crate::theme::text()) } else { name.color(crate::theme::text_dim()) };
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(name).selectable(false));
                if !line.secret {
                    ui.add(egui::Label::new(RichText::new(line.tier.label()).small().color(crate::theme::text_dim())).selectable(false));
                }
            });
            ui.add(egui::Label::new(RichText::new(&line.description).small().color(Color32::from_gray(150))).wrap().selectable(false));
            if let Some(when) = line.when.filter(|_| line.earned) {
                ui.add(egui::Label::new(RichText::new(format!("Earned {}", trophies::date(when))).small().color(crate::theme::accent_stroke())).selectable(false));
            }
        });
    });
    ui.add_space(4.0);
}

/// The progress bar, the note about hidden trophies and every trophy, in one list. It is the
/// body of the Options drop-down and of the Trophies tab.
fn list(ui: &mut egui::Ui, cfg: &Config, earned: usize, all: usize) {
    ui.add(egui::ProgressBar::new(earned as f32 / all.max(1) as f32).desired_height(8.0).fill(crate::theme::accent()));
    ui.add_space(4.0);
    let hidden = cfg.trophies.lines().iter().filter(|l| l.secret).count();
    let text = if hidden == 0 {
        "Every trophy is revealed.".to_string()
    } else {
        format!("{hidden} hidden: they show their name and what they take once you have earned them.")
    };
    ui.add(egui::Label::new(RichText::new(text).small().color(Color32::from_gray(140))).wrap().selectable(false));
    ui.add_space(6.0);
    for line in cfg.trophies.lines() {
        row(ui, &line);
    }
    ui.add(egui::Label::new(RichText::new("Test tools: Options > Trophy testing.").small().color(Color32::from_gray(110))).selectable(false));
}

/// The Trophies drop-down. Not there until the first trophy has been earned.
pub fn render_trophies_section(ui: &mut egui::Ui, cfg: &Config) {
    if cfg.trophies.count() == 0 {
        return;
    }
    let (earned, all) = cfg.trophies.progress();
    crate::theme::collapsing(ui, "options_trophies", format!("Trophies  {earned} / {all}"), true, |ui| {
        list(ui, cfg, earned, all);
    });
}

fn rank(t: Tier) -> u8 {
    match t {
        Tier::Platinum => 3,
        Tier::Gold => 2,
        Tier::Silver => 1,
        Tier::Bronze => 0,
    }
}

fn tier_colour(t: Tier) -> Color32 {
    let [r, g, b] = t.colour();
    Color32::from_rgb(r, g, b)
}

/// A medal: a coin in the tier's colour once earned, a ring that hints at the tier while it is not,
/// and a plain "?" ring while it is hidden. The letter on it is a placeholder for the trophy's own
/// icon (see the design in the Trophy Icons canvas).
fn paint_medal(painter: &egui::Painter, c: egui::Pos2, r: f32, line: &Line) {
    let initial: String = line.name.chars().next().map(|ch| ch.to_string()).unwrap_or_default();
    let font = egui::FontId::proportional(r * 1.0);
    if line.earned {
        painter.circle_filled(c, r, tier_colour(line.tier));
        painter.circle_stroke(c, r * 0.76, Stroke::new((r * 0.12).max(1.0), Color32::from_black_alpha(90)));
        painter.circle_stroke(c, r, Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.6)));
        painter.text(c, egui::Align2::CENTER_CENTER, initial, font, Color32::from_black_alpha(150));
    } else {
        painter.circle_filled(c, r, crate::theme::bg());
        let ring = if line.secret { crate::theme::border_light() } else { tier_colour(line.tier).gamma_multiply(0.5) };
        painter.circle_stroke(c, r * 0.96, Stroke::new(1.5, ring));
        let text = if line.secret { "?".to_string() } else { initial };
        painter.text(c, egui::Align2::CENTER_CENTER, text, font, crate::theme::text_dim());
    }
}

/// One group of medals, wrapped to the width of the page. Clicking one selects it.
fn medal_grid(ui: &mut egui::Ui, heading: &str, items: &[&Line], selected: &mut Option<String>) {
    if items.is_empty() {
        return;
    }
    ui.add_space(6.0);
    ui.add(egui::Label::new(RichText::new(heading).small().strong().color(crate::theme::text_dim())).selectable(false));
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
        for line in items {
            let (rect, resp) = ui.allocate_exact_size(vec2(58.0, 58.0), egui::Sense::click());
            let is_selected = selected.as_deref() == Some(line.id);
            let fill = if resp.hovered() { crate::theme::surface_2() } else { crate::theme::surface() };
            let edge = if is_selected { Stroke::new(2.0, crate::theme::accent()) } else { Stroke::new(1.0, crate::theme::border()) };
            let painter = ui.painter();
            painter.rect_filled(rect, 8.0, fill);
            painter.rect_stroke(rect, 8.0, edge, egui::StrokeKind::Inside);
            paint_medal(painter, rect.center(), 20.0, line);
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                *selected = Some(line.id.to_string());
            }
            resp.on_hover_text(if line.secret { "???" } else { line.name.as_str() });
        }
    });
}

/// The selected medal, large, with what it is and what it takes.
fn medal_detail(ui: &mut egui::Ui, line: &Line) {
    egui::Frame::new()
        .fill(crate::theme::surface())
        .stroke(Stroke::new(1.0, crate::theme::border()))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(84.0, 84.0), egui::Sense::hover());
                paint_medal(ui.painter(), rect.center(), 38.0, line);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    let (tier_text, tier_col) = if line.secret {
                        ("HIDDEN".to_string(), crate::theme::text_dim())
                    } else {
                        (line.tier.label().to_uppercase(), tier_colour(line.tier))
                    };
                    ui.add(egui::Label::new(RichText::new(tier_text).small().strong().color(tier_col)).selectable(false));
                    let name = if line.secret { "???" } else { line.name.as_str() };
                    ui.add(egui::Label::new(RichText::new(name).size(18.0).strong().color(crate::theme::text())).selectable(false));
                    let what = if line.secret {
                        "A hidden trophy. It shows its name and what it takes once you have earned it."
                    } else {
                        line.description.as_str()
                    };
                    ui.add(egui::Label::new(RichText::new(what).color(Color32::from_gray(170))).wrap().selectable(false));
                    if line.earned {
                        let when = line.when.map(|w| format!("Earned {}", trophies::date(w))).unwrap_or_else(|| "Earned".to_string());
                        ui.add(egui::Label::new(RichText::new(when).small().color(crate::theme::accent_stroke())).selectable(false));
                    } else {
                        ui.add(egui::Label::new(RichText::new("Not earned yet").small().color(crate::theme::text_dim())).selectable(false));
                    }
                });
            });
        });
}

/// The Trophies tab: how far along you are, then the medals you have earned (newest first) and the
/// ones still to earn (best first), and the one you have picked, large, with what it takes.
pub fn render_page(ui: &mut egui::Ui, cfg: &Config) {
    let (earned, all) = cfg.trophies.progress();
    let lines = cfg.trophies.lines();
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(RichText::new("Trophies").size(17.0).strong().color(crate::theme::text())).selectable(false));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(RichText::new(format!("{earned} of {all} earned")).color(crate::theme::text_dim())).selectable(false));
        });
    });
    ui.add(egui::ProgressBar::new(earned as f32 / all.max(1) as f32).desired_height(8.0).fill(crate::theme::accent()));
    ui.add_space(2.0);
    // how many of each tier: a hidden trophy that is not earned does not say which tier it is
    let hidden = lines.iter().filter(|l| l.secret).count();
    ui.horizontal_wrapped(|ui| {
        for tier in [Tier::Bronze, Tier::Silver, Tier::Gold, Tier::Platinum] {
            let shown: Vec<&Line> = lines.iter().filter(|l| l.tier == tier && !l.secret).collect();
            if shown.is_empty() {
                continue;
            }
            let got = shown.iter().filter(|l| l.earned).count();
            ui.add(egui::Label::new(RichText::new(format!("{} {}/{}", tier.label(), got, shown.len())).small().color(tier_colour(tier))).selectable(false));
            ui.add_space(6.0);
        }
        if hidden > 0 {
            ui.add(egui::Label::new(RichText::new(format!("{hidden} hidden")).small().color(crate::theme::text_dim())).selectable(false));
        }
    });

    let mut got: Vec<&Line> = lines.iter().filter(|l| l.earned).collect();
    got.sort_by(|a, b| b.when.cmp(&a.when));
    let mut open: Vec<&Line> = lines.iter().filter(|l| !l.earned).collect();
    open.sort_by_key(|l| (l.secret, std::cmp::Reverse(rank(l.tier))));

    let sel_id = egui::Id::new("trophies_selected");
    let mut selected: Option<String> = ui.data(|d| d.get_temp::<String>(sel_id));
    if !selected.as_deref().is_some_and(|id| lines.iter().any(|l| l.id == id)) {
        selected = got.first().or(open.first()).map(|l| l.id.to_string());
    }
    medal_grid(ui, "EARNED", &got, &mut selected);
    medal_grid(ui, "STILL TO EARN", &open, &mut selected);
    ui.add_space(10.0);
    if let Some(line) = lines.iter().find(|l| Some(l.id) == selected.as_deref()) {
        medal_detail(ui, line);
    }
    if let Some(id) = selected {
        ui.data_mut(|d| d.insert_temp(sel_id, id));
    }
}

/// One switch of the picture of Options > Settings > Tabs in the notice below: not clickable.
/// `mark` draws a frame round the row.
fn mock_switch(ui: &mut egui::Ui, label: &str, on: bool, mark: bool) {
    let stroke = if mark { Stroke::new(1.5, crate::theme::accent()) } else { Stroke::NONE };
    egui::Frame::new().stroke(stroke).corner_radius(4).inner_margin(egui::Margin::symmetric(6, 2)).show(ui, |ui| {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(36.0, 20.0), egui::Sense::hover());
            let track = if on { crate::theme::accent() } else { crate::theme::toggle_off() };
            let r = rect.height() / 2.0;
            ui.painter().rect_filled(rect, r, track);
            let knob_x = if on { rect.right() - r } else { rect.left() + r };
            ui.painter().circle_filled(egui::pos2(knob_x, rect.center().y), r - 3.0, if on { Color32::WHITE } else { crate::theme::text_dim() });
            let text = RichText::new(label).color(if mark { crate::theme::text() } else { crate::theme::text_dim() });
            ui.add(egui::Label::new(if mark { text.strong() } else { text }).selectable(false));
        });
    });
}

/// The notice that the Trophies tab can be switched on, shown once after the first trophy. It has
/// to be closed with "Okay" (it blocks the window behind it), and shows where the switch is.
/// True once "Okay" is clicked.
pub fn render_tab_notice(ctx: &egui::Context) -> bool {
    let mut okay = false;
    egui::Modal::new(egui::Id::new("trophies_tab_notice")).show(ctx, |ui| {
        ui.set_max_width(360.0);
        ui.vertical_centered(|ui| {
            ui.add(egui::Label::new(RichText::new("A new tab: Trophies").size(20.0).strong().color(crate::theme::accent_stroke())).selectable(false));
        });
        ui.add_space(6.0);
        ui.add(egui::Label::new(RichText::new("The new Trophies tab can be selected inside the options under Settings\\Tabs").size(15.0)).wrap().selectable(false));
        ui.add_space(8.0);
        // a picture of the place: Options > Settings > Tabs, with the new switch marked
        egui::Frame::new()
            .fill(crate::theme::bg())
            .stroke(Stroke::new(1.0, crate::theme::border()))
            .corner_radius(6)
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.add(egui::Label::new(RichText::new("Options").small().color(crate::theme::text_dim())).selectable(false));
                ui.add(egui::Label::new(RichText::new("\u{25be} Settings").strong()).selectable(false));
                ui.indent("trophies_notice_tabs", |ui| {
                    ui.add(egui::Label::new(RichText::new("\u{25be} Tabs").strong()).selectable(false));
                    ui.indent("trophies_notice_rows", |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for name in ["Lighting", "Sensors", "Files", "Changelog", "Info"] {
                            mock_switch(ui, name, true, false);
                        }
                        mock_switch(ui, "Trophies", false, true);
                    });
                });
            });
        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            if ui.button("   Okay   ").clicked() {
                okay = true;
            }
        });
    });
    okay
}

/// How long the announcement stays up if it is not clicked.
pub const TOAST_SECONDS: f32 = 6.0;

/// The announcement of a trophy that has just been earned: a card that slides down from the top of
/// the window, stays a few seconds and fades. `age` is how long it has been up. Returns true when it
/// was clicked (the caller then takes it away). The caller decides when it is a good time to show it.
pub fn render_toast(ctx: &egui::Context, trophy: &trophies::Trophy, age: f32) -> bool {
    let slide = (1.0 - (age / 0.35).clamp(0.0, 1.0)).powi(3);
    let fade = ((TOAST_SECONDS - age) / 0.4).clamp(0.0, 1.0);
    let mut clicked = false;
    egui::Area::new(egui::Id::new("trophy_toast"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_TOP, vec2(0.0, 14.0 - 110.0 * slide))
        .show(ctx, |ui| {
            ui.set_opacity(fade);
            let platinum = trophy.id == trophies::PLATINUM;
            let frame = egui::Frame::new()
                .fill(crate::theme::surface())
                .stroke(Stroke::new(1.5, crate::theme::accent_stroke()))
                .corner_radius(egui::CornerRadius::same(10))
                .inner_margin(egui::Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.set_max_width(360.0);
                    ui.horizontal(|ui| {
                        cup(ui, trophy.tier, true, false, 40.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            let heading = if platinum { "PLATINUM TROPHY EARNED" } else { "TROPHY EARNED" };
                            ui.add(egui::Label::new(RichText::new(heading).small().strong().color(crate::theme::accent_stroke())).selectable(false));
                            ui.add(egui::Label::new(RichText::new(trophy.name).strong().size(17.0).color(crate::theme::text())).selectable(false));
                            ui.add(egui::Label::new(RichText::new(trophy.description).small().color(crate::theme::text_dim())).wrap().selectable(false));
                            let hint = if platinum { "The Platinum skin is yours: Options > UI Options > Skins." } else { "Options > Trophies has them all." };
                            ui.add(egui::Label::new(RichText::new(hint).small().color(Color32::from_gray(140))).wrap().selectable(false));
                        });
                    });
                });
            if frame.response.interact(egui::Sense::click()).clicked() {
                clicked = true;
            }
        });
    clicked
}
