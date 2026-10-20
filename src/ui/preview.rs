//! Small previews of looks and backdrops (Options > UI Options): a tiny box next to each, and a
//! bigger window that opens when a box is clicked and closes when the same box is clicked again or
//! the window's X is. A preview is a miniature of the program (two cards on the look's colours, with
//! its trim, its art and its backdrop, the picture ones too), so nothing has to be put on to see it.
use crate::info::BackdropStyle;
use crate::pairs::{Pair, Wear};
use crate::theme::{self, LookKind};
use crate::config::Config;
use eframe::egui::{self, pos2, vec2, Color32, CornerRadius, Rect, Stroke, StrokeKind, TextureHandle};

/// The look to draw for something that can be worn.
pub fn look_kind(wear: Wear) -> LookKind {
    match wear {
        Wear::Plain => LookKind::Standard,
        Wear::Skin(id) => LookKind::Skin(theme::Skin::from_id(Some(id))),
        Wear::Gold => LookKind::Gold,
        Wear::Void => LookKind::Void,
        Wear::Underworld => LookKind::Underworld,
    }
}

/// The pictures of a picture backdrop as textures, made once and kept (shrunk, since they only show
/// in a small box). Empty for the backdrops that are drawn live. `void` picks the Void's version of
/// the angel cats.
fn textures(ctx: &egui::Context, style: BackdropStyle, void: bool, soul: bool) -> Vec<TextureHandle> {
    let pngs = style.pngs_with(void, soul);
    if pngs.is_empty() {
        return Vec::new();
    }
    let id = egui::Id::new(("ui_options_preview_textures", style.key(), void, soul));
    if let Some(found) = ctx.data(|d| d.get_temp::<Vec<TextureHandle>>(id)) {
        return found;
    }
    let loaded: Vec<TextureHandle> = pngs
        .iter()
        .enumerate()
        .filter_map(|(i, png)| {
            let img = image::load_from_memory(png).ok()?;
            let img = if img.width().max(img.height()) > PICTURE_SIDE { img.resize(PICTURE_SIDE, PICTURE_SIDE, image::imageops::FilterType::Triangle) } else { img };
            let img = img.to_rgba8();
            let size = [img.width() as usize, img.height() as usize];
            Some(ctx.load_texture(
                format!("cpunity_preview_{}_{}_{}_{}", style.key(), void, soul, i),
                egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
                egui::TextureOptions::LINEAR,
            ))
        })
        .collect();
    ctx.data_mut(|d| d.insert_temp(id, loaded.clone()));
    loaded
}

/// The longest side of a preview picture, in pixels.
const PICTURE_SIDE: u32 = 640;

/// Draws a backdrop into the ui's clip rectangle: the live-drawn ones, and the pictures (with the
/// motion they have when `t` is above 0; Father Time is a still without his clock hands).
fn paint_backdrop(ui: &egui::Ui, style: BackdropStyle, t: f32, void: bool, soul: bool) {
    use crate::ui::backdrop as b;
    let moving = t > 0.0;
    match style {
        BackdropStyle::Embers => b::paint_embers(ui, t, 0.9),
        BackdropStyle::DataRain => b::paint_data_rain(ui, t, 0.8),
        BackdropStyle::Northern => b::paint_northern(ui, t, 0.9),
        BackdropStyle::Petals => b::paint_petals(ui, t, 0.9),
        BackdropStyle::SynthGrid => b::paint_synth_grid(ui, t, 0.9),
        BackdropStyle::Bubbles => b::paint_bubbles(ui, t, 0.9),
        BackdropStyle::Scanlines => b::paint_scanlines(ui, t, 0.9),
        BackdropStyle::Fireflies => b::paint_fireflies(ui, t, 0.9),
        BackdropStyle::Snowfall => b::paint_snowfall(ui, t, 0.9),
        BackdropStyle::Storm => b::paint_storm(ui, t, 0.9),
        BackdropStyle::Constellations => b::paint_constellations(ui, t, 0.9),
        BackdropStyle::Sunbeams => b::paint_sunbeams(ui, t, 0.9),
        BackdropStyle::SoulWhispers => crate::ui::underworld::paint_whispers_preview(ui, t),
        BackdropStyle::VoidStars | BackdropStyle::DeadKing => b::paint_void_stars(ui, t),
        BackdropStyle::Cat => crate::ui::cat_talk::paint_market(ui, t, 0.8),
        BackdropStyle::Circuit | BackdropStyle::Predator | BackdropStyle::Cats => b::paint(ui, &textures(ui.ctx(), style, void, soul), 0.8),
        BackdropStyle::Dragons => b::paint_dragons(ui, &textures(ui.ctx(), style, void, soul), 0.8, false, moving),
        BackdropStyle::AngelCats => b::paint_angels(ui, &textures(ui.ctx(), style, void, soul), 0.9, false, moving, void, soul),
        BackdropStyle::FatherTime => b::paint_father_still(ui, &textures(ui.ctx(), style, void, soul), 0.8),
    }
}

/// A miniature of the program in `look`, with `backdrop` behind it, filling `rect`. `t` is the time in
/// seconds (0 holds everything still).
pub fn paint(ui: &mut egui::Ui, rect: Rect, look: LookKind, backdrop: Option<BackdropStyle>, t: f32) {
    let sw = theme::swatch(look);
    let area = rect.intersect(ui.clip_rect());
    if area.width() <= 0.0 || area.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(area);
    painter.rect_filled(rect, 4.0, sw.bg);
    // the art and the backdrops draw into the clip rectangle of a ui, so they get one of their own
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    child.set_clip_rect(area);
    let saved = theme::skin();
    let skin = match look {
        LookKind::Skin(skin) => skin,
        _ => theme::Skin::None,
    };
    theme::set_skin(skin);
    match skin {
        theme::Skin::Sakura => crate::ui::backdrop::paint_blossom_branches(&child, t, 0.9),
        theme::Skin::Frost => crate::ui::backdrop::paint_pine_forest(&child, t, 0.9),
        theme::Skin::Descent => crate::ui::backdrop::paint_descent(&child, t, 0.9),
        theme::Skin::Platinum => {
            crate::ui::backdrop::paint_platinum(&child, t, 0.9);
            if area.width() >= 200.0 {
                crate::ui::backdrop::paint_platinum_sparkles(&child, t);
            }
        }
        _ => {}
    }
    if let Some(style) = backdrop {
        paint_backdrop(&child, style, t, look == LookKind::Void, look == LookKind::Skin(theme::Skin::Platinum));
    }
    theme::set_skin(saved);
    mock_cards(&painter, rect, &sw, look, t);
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, sw.border_light), StrokeKind::Inside);
}

/// Two cards on the look's colours, with a heading, buttons and lines of text, and the look's trim.
fn mock_cards(painter: &egui::Painter, rect: Rect, sw: &theme::Swatch, look: LookKind, t: f32) {
    let (w, h) = (rect.width(), rect.height());
    let u = (w / 200.0).clamp(0.2, 2.0);
    let line = u.max(0.6);
    let radius = CornerRadius::same(((sw.radius as f32) * u).min(8.0).round() as u8);
    let card = |top: f32, bottom: f32| Rect::from_min_max(pos2(rect.left() + 0.07 * w, rect.top() + top * h), pos2(rect.right() - 0.07 * w, rect.top() + bottom * h));
    let cards = [card(0.10, 0.47), card(0.55, 0.90)];
    for c in cards {
        if look == LookKind::Skin(theme::Skin::Platinum) {
            painter.add(theme::steel_shape(c, radius.nw as f32));
        } else {
            painter.rect_filled(c, radius, sw.surface);
        }
        painter.rect_stroke(c, radius, Stroke::new(line, sw.border), StrokeKind::Inside);
    }
    let (c1, c2) = (cards[0], cards[1]);
    // a heading and three buttons on the first card
    painter.rect_filled(Rect::from_min_size(c1.left_top() + vec2(0.05 * w, 0.05 * h), vec2(0.28 * w, (0.06 * h).max(1.0))), 1.0, sw.accent);
    if w >= 120.0 {
        for i in 0..3 {
            let chip = Rect::from_min_size(c1.left_top() + vec2(0.05 * w + i as f32 * 0.17 * w, 0.17 * h), vec2(0.15 * w, 0.1 * h));
            painter.rect_filled(chip, 3.0 * u, sw.surface_2);
            painter.rect_stroke(chip, 3.0 * u, Stroke::new(line, sw.border_light), StrokeKind::Inside);
        }
    }
    // two lines of text and a filled button on the second
    for (i, frac) in [0.6, 0.42].into_iter().enumerate() {
        painter.rect_filled(Rect::from_min_size(c2.left_top() + vec2(0.05 * w, 0.06 * h + i as f32 * 0.07 * h), vec2(frac * w, (0.025 * h).max(1.0))), 1.0, sw.text_dim);
    }
    painter.rect_filled(Rect::from_min_size(c2.right_bottom() - vec2(0.05 * w + 0.18 * w, 0.05 * h + 0.1 * h), vec2(0.18 * w, 0.1 * h)), 3.0 * u, sw.accent);

    let arm = 5.0 * u;
    let bracket = |p: egui::Pos2, dx: f32, dy: f32, colour: Color32| {
        painter.line_segment([p, p + vec2(dx * arm, 0.0)], Stroke::new(1.3 * u.max(0.7), colour));
        painter.line_segment([p, p + vec2(0.0, dy * arm)], Stroke::new(1.3 * u.max(0.7), colour));
    };
    match look {
        LookKind::Skin(theme::Skin::Sakura) => {
            painter.line_segment([c1.left_top() + vec2(0.05 * w, 0.5), c1.left_top() + vec2(0.17 * w, 0.5)], Stroke::new(2.0 * u.max(0.6), theme::SAKURA_VERMILION));
            for (dx, dy, r) in [(-0.04, 0.04, 0.024), (-0.09, 0.075, 0.018), (-0.02, 0.10, 0.016)] {
                painter.circle_filled(c1.right_top() + vec2(dx * w, dy * w), r * w, Color32::from_rgb(238, 170, 194));
            }
        }
        LookKind::Skin(theme::Skin::Synthwave) => {
            for c in cards {
                painter.line_segment([c.left_top() + vec2(3.0 * u, 0.5), c.right_top() + vec2(-3.0 * u, 0.5)], Stroke::new(1.5 * u.max(0.7), theme::SYNTH_MAGENTA));
                painter.line_segment([c.left_bottom() + vec2(3.0 * u, -0.5), c.right_bottom() + vec2(-3.0 * u, -0.5)], Stroke::new(1.5 * u.max(0.7), theme::SYNTH_CYAN));
            }
        }
        LookKind::Skin(skin @ (theme::Skin::Prism | theme::Skin::Aurora)) => {
            let colour: fn(f32, f32) -> [u8; 3] = if skin == theme::Skin::Prism { crate::skins::prism_edge } else { crate::skins::aurora_edge };
            for c in cards {
                for (a, b, along) in crate::skins::outline_pieces(c.width(), c.height(), 40) {
                    let [r, g, bl] = colour(along, t);
                    painter.line_segment([c.min + vec2(a.0, a.1), c.min + vec2(b.0, b.1)], Stroke::new(1.5 * u.max(0.7), Color32::from_rgb(r, g, bl)));
                }
            }
        }
        LookKind::Skin(theme::Skin::DeepSea) => {
            for c in cards {
                for k in 0..5 {
                    let r = (1.2 + ((k * 7) % 3) as f32 * 0.6) * u.max(0.5);
                    let centre = pos2(c.left() + (0.1 + 0.17 * k as f32) * c.width(), c.bottom() - r - 1.0);
                    painter.circle_stroke(centre, r, Stroke::new(line, sw.accent.gamma_multiply(0.7)));
                }
            }
        }
        LookKind::Skin(theme::Skin::Frost) => {
            let snow = Color32::from_rgb(236, 246, 255);
            for c in cards {
                painter.rect_filled(Rect::from_min_size(c.left_top() + vec2(0.04 * w, -0.5 * u), vec2(c.width() - 0.08 * w, (2.5 * u).max(1.0))), 1.0, snow);
            }
            for k in 0..5 {
                let x = c1.left() + (0.12 + 0.17 * k as f32) * c1.width();
                painter.add(egui::Shape::convex_polygon(
                    vec![pos2(x - 1.5 * u, c1.bottom()), pos2(x + 1.5 * u, c1.bottom()), pos2(x, c1.bottom() + (3.0 + (k % 3) as f32) * u.max(0.6))],
                    Color32::from_rgb(170, 214, 240),
                    Stroke::NONE,
                ));
            }
        }
        LookKind::Skin(theme::Skin::Terminal) => {
            for c in cards {
                bracket(c.left_top() + vec2(1.0, 1.0), 1.0, 1.0, sw.accent);
                bracket(c.right_top() + vec2(-1.0, 1.0), -1.0, 1.0, sw.accent);
                bracket(c.left_bottom() + vec2(1.0, -1.0), 1.0, -1.0, sw.accent);
                bracket(c.right_bottom() + vec2(-1.0, -1.0), -1.0, -1.0, sw.accent);
            }
        }
        LookKind::Skin(theme::Skin::Descent) => {
            for c in cards {
                painter.line_segment([c.left_top() + vec2(0.0, 0.5), c.left_top() + vec2(0.1 * w, 0.5)], Stroke::new(2.0 * u.max(0.6), sw.accent));
            }
        }
        LookKind::Skin(theme::Skin::Platinum) => {
            for c in cards {
                // the bevel: lit along the top and left, dark along the bottom and right
                let i = c.shrink(1.0);
                painter.line_segment([i.left_bottom(), i.left_top()], Stroke::new(line, Color32::WHITE.gamma_multiply(0.55)));
                painter.line_segment([i.left_top(), i.right_top()], Stroke::new(line, Color32::WHITE.gamma_multiply(0.75)));
                painter.line_segment([i.right_top(), i.right_bottom()], Stroke::new(line, Color32::from_black_alpha(150)));
                painter.line_segment([i.right_bottom(), i.left_bottom()], Stroke::new(line, Color32::from_black_alpha(160)));
                painter.rect_stroke(c.shrink(3.0 * u), CornerRadius::same(2), Stroke::new(line, sw.accent.gamma_multiply(0.25)), StrokeKind::Inside);
                // a gem on the top edge and at each corner
                let gem = |p: egui::Pos2, r: f32| {
                    for (tri, shade) in crate::platinum::gem_facets(r) {
                        let colour = theme::mix(Color32::from_rgb(64, 74, 92), Color32::from_rgb(246, 250, 255), shade);
                        painter.add(egui::Shape::convex_polygon(tri.iter().map(|(x, y)| p + vec2(*x, *y)).collect(), colour, Stroke::NONE));
                    }
                };
                gem(c.center_top(), 4.5 * u.max(0.6));
                for (corner, dx, dy) in [(c.left_top(), 1.0, 1.0), (c.right_top(), -1.0, 1.0), (c.left_bottom(), 1.0, -1.0), (c.right_bottom(), -1.0, -1.0)] {
                    gem(corner + vec2(dx * 6.0 * u, dy * 6.0 * u), 2.8 * u.max(0.6));
                }
            }
        }
        LookKind::Skin(theme::Skin::Midnight) => {
            for c in cards {
                painter.line_segment([c.left_top() + vec2(3.0 * u, 0.5), c.right_top() + vec2(-3.0 * u, 0.5)], Stroke::new(1.3 * u.max(0.7), sw.accent));
                painter.circle_filled(c.right_top() + vec2(-5.0 * u, 3.5 * u), 1.1 * u.max(0.6), Color32::from_rgb(230, 236, 255));
            }
        }
        LookKind::Skin(theme::Skin::Copper) => {
            for c in cards {
                bracket(c.left_top() + vec2(1.0, 1.0), 1.0, 1.0, sw.accent);
                bracket(c.right_bottom() + vec2(-1.0, -1.0), -1.0, -1.0, sw.accent);
            }
        }
        LookKind::Skin(theme::Skin::Fraying) => {
            for k in 0..3 {
                let x = c1.left() + (0.2 + 0.25 * k as f32) * c1.width();
                painter.add(egui::Shape::convex_polygon(
                    vec![pos2(x - 2.0 * u, c1.top()), pos2(x + 2.0 * u, c1.top()), pos2(x, c1.top() + 2.5 * u.max(0.6))],
                    sw.bg,
                    Stroke::NONE,
                ));
            }
        }
        LookKind::Gold => {
            let bright = Color32::from_rgb(240, 194, 79);
            for c in cards {
                bracket(c.left_top() + vec2(1.0, 1.0), 1.0, 1.0, bright);
                bracket(c.right_bottom() + vec2(-1.0, -1.0), -1.0, -1.0, bright);
            }
        }
        LookKind::Void => {
            for c in cards {
                painter.line_segment([c.left_top() + vec2(0.04 * w, 1.0), c.left_top() + vec2(0.14 * w, 1.0)], Stroke::new(2.0 * u.max(0.6), Color32::from_rgb(63, 240, 255)));
                painter.line_segment([c.right_bottom() + vec2(-0.04 * w, -1.0), c.right_bottom() + vec2(-0.14 * w, -1.0)], Stroke::new(2.0 * u.max(0.6), Color32::from_rgb(255, 63, 200)));
            }
        }
        LookKind::Underworld => {
            painter.rect_filled(Rect::from_min_max(pos2(rect.left(), rect.bottom() - 0.1 * h), rect.right_bottom()), 0.0, Color32::from_rgba_unmultiplied(255, 106, 30, 150));
        }
        _ => {}
    }
}

/// The tiny box next to a look or a backdrop (a still picture of its preview). Clicking it is how the
/// preview is opened or closed.
pub fn mini(ui: &mut egui::Ui, look: LookKind, backdrop: Option<BackdropStyle>) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(40.0, 28.0), egui::Sense::click());
    paint(ui, rect, look, backdrop, 0.0);
    response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Click for a preview")
}

// ---- the preview that is open ----

/// What the open preview shows: the look and backdrop to draw, and what it is.
#[derive(Debug, Clone)]
pub struct Open {
    pub look: Wear,
    pub backdrop: Option<BackdropStyle>,
    pub title: String,
    /// What kind of thing it is ("Look", "Backdrop", "Pair").
    pub kind: &'static str,
    /// What it is, in a sentence or two.
    pub blurb: String,
    /// Further lines (the parts of a pair, where it stands).
    pub details: Vec<String>,
}

/// Two are the same preview when they show the same thing; the words round it may change as
/// things are bought or put on, which does not make it another preview.
impl PartialEq for Open {
    fn eq(&self, other: &Open) -> bool {
        self.look == other.look && self.backdrop == other.backdrop && self.title == other.title && self.kind == other.kind
    }
}

impl Open {
    /// A look on its own (no backdrop).
    pub fn look(wear: Wear) -> Open {
        let title = if wear == Wear::Plain { "The program's own look".to_string() } else { wear.name().to_string() };
        Open { look: wear, backdrop: None, title, kind: "Look", blurb: wear.blurb().to_string(), details: Vec::new() }
    }

    /// Whether the angel cats are the Soul Catsters on `look` (they are on Platinum).
    fn soul_on(look: Wear) -> bool {
        look.soul_cats()
    }

    /// A backdrop, shown on `look` (the one the program has now).
    pub fn backdrop(look: Wear, style: BackdropStyle) -> Open {
        let mut details = vec![format!("Shown here on: {}.", if look == Wear::Plain { "the program's own look" } else { look.name() })];
        if style.needs_gold() && crate::release::story() {
            details.push("Comes with the Gold Edition.".to_string());
        } else if style.needs_void() {
            details.push("Comes with The Void.".to_string());
        } else if style.inside_underworld_only() {
            details.push("Only inside The Underworld.".to_string());
        } else if let (true, Some(item)) = (crate::release::story(), style.shop_id().and_then(crate::shop::item)) {
            details.push(format!("Sold in the shop for {} tokens.", item.price));
        } else if style.is_ambient() {
            details.push("Comes with the program.".to_string());
        }
        if style.is_animated() {
            details.push("Moves (Animate backdrop turns that off).".to_string());
        }
        let soul = Open::soul_on(look);
        Open { look, backdrop: Some(style), title: style.label_in(soul).to_string(), kind: "Backdrop", blurb: style.blurb_in(soul).to_string(), details }
    }

    /// A pair: the look and the backdrop made for it, with where it stands.
    pub fn pair(cfg: &Config, pair: &Pair) -> Open {
        let details = vec![
            format!("Look: {}. {}", pair.look.name(), pair.look.blurb()),
            format!("Backdrop: {}. {}", pair.backdrop.label_in(Open::soul_on(pair.look)), pair.backdrop.blurb_in(Open::soul_on(pair.look))),
            crate::pairs::status(cfg, pair),
        ];
        Open { look: pair.look, backdrop: Some(pair.backdrop), title: pair.name.to_string(), kind: "Pair: a look and a backdrop, put on together", blurb: pair.blurb.to_string(), details }
    }
}

fn state_id() -> egui::Id {
    egui::Id::new("ui_options_preview_state")
}

fn current(ctx: &egui::Context) -> Option<Open> {
    ctx.data(|d| d.get_temp::<Option<Open>>(state_id())).flatten()
}

fn set(ctx: &egui::Context, value: Option<Open>) {
    ctx.data_mut(|d| d.insert_temp(state_id(), value));
}

/// Opens this preview, or closes it if it is the one that is open.
pub fn toggle(ctx: &egui::Context, open: Open) {
    if current(ctx).as_ref() == Some(&open) {
        set(ctx, None);
    } else {
        set(ctx, Some(open));
    }
}

/// The tiny box for `open`, which opens its preview when clicked (and closes it when clicked again).
/// The box of the preview that is open has a ring round it.
pub fn mini_toggle(ui: &mut egui::Ui, open: Open) {
    let is_open = current(ui.ctx()).as_ref() == Some(&open);
    let response = mini(ui, look_kind(open.look), open.backdrop);
    if is_open {
        set(ui.ctx(), Some(open.clone()));
        ui.painter().rect_stroke(response.rect.expand(1.5), 5.0, Stroke::new(1.5, theme::accent()), StrokeKind::Outside);
    }
    if response.clicked() {
        toggle(ui.ctx(), open);
    }
}

/// Shows the open preview, if there is one, in a window of its own that moves and has an X. Call it
/// every frame while Options is showing. `near` is where the window first appears.
pub fn render_window(ctx: &egui::Context, near: egui::Pos2) {
    let Some(open) = current(ctx) else {
        return;
    };
    let mut keep = true;
    egui::Window::new("Preview")
        .id(egui::Id::new("ui_options_preview_window"))
        .open(&mut keep)
        .collapsible(false)
        .resizable(false)
        .default_pos(near)
        .show(ctx, |ui| {
            let weak = |text: &str| egui::Label::new(egui::RichText::new(text).small().weak()).wrap().selectable(false);
            ui.label(egui::RichText::new(&open.title).strong().size(15.0));
            ui.add(weak(open.kind));
            ui.add_space(4.0);
            let (rect, _) = ui.allocate_exact_size(vec2(300.0, 190.0), egui::Sense::hover());
            let t = (ui.input(|i| i.time) % 3600.0) as f32 + 0.01;
            paint(ui, rect, look_kind(open.look), open.backdrop, t);
            ui.add_space(4.0);
            ui.set_max_width(300.0);
            ui.add(egui::Label::new(open.blurb.as_str()).wrap().selectable(false));
            for line in &open.details {
                ui.add(weak(line));
            }
            ui.add_space(2.0);
            ui.add(weak("Click its box again, or the X, to close this."));
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
        });
    if !keep {
        set(ctx, None);
    }
}
