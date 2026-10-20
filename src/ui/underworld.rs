// The Underworld look, drawn: moving lava behind everything with bubbles bursting on it, a rock
// slab behind every card, a causeway of skeletons under the title, and
// blue-white souls that rise out of the lava in spurts. The numbers behind all of it (outlines,
// textures, timings) are in `underworld.rs`; this file only paints them.
//
// The slabs float on the lava and what is written on them floats with them (see `slab_frame`).
use crate::underworld as uw;
use eframe::egui;
use egui::{pos2, vec2, Color32, Pos2, Rect, Shape, Stroke};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

/// The three pictures the look is made from. Keep them alive while the look is on.
pub struct Textures {
    pub rock: egui::TextureHandle,
    pub crust: egui::TextureHandle,
    pub far: egui::TextureHandle,
}

impl Textures {
    /// Makes the lava and the rock (a moment's work, done once).
    pub fn load(ctx: &egui::Context) -> Self {
        let n = uw::TEXTURE_SIZE;
        let options = egui::TextureOptions { wrap_mode: egui::epaint::textures::TextureWrapMode::Repeat, ..egui::TextureOptions::LINEAR };
        let make = |name: &str, pixels: Vec<[u8; 4]>| -> egui::TextureHandle {
            let bytes: Vec<u8> = pixels.iter().flat_map(|p| p.iter().copied()).collect();
            ctx.load_texture(name, egui::ColorImage::from_rgba_unmultiplied([n, n], &bytes), options)
        };
        Textures {
            rock: make("cpunity_uw_rock", uw::rock_rgba(n, 3)),
            crust: make("cpunity_uw_crust", uw::lava_crust_rgba(n, 11)),
            far: make("cpunity_uw_far", uw::lava_crust_far_rgba(n, 11)),
        }
    }
}

static ROCK: Mutex<Option<egui::TextureId>> = Mutex::new(None);
static SLAB_COUNT: AtomicU32 = AtomicU32::new(0);
static SLABS: Mutex<Vec<Rect>> = Mutex::new(Vec::new());
static MOTION: AtomicBool = AtomicBool::new(true);
static BUBBLES: OnceLock<Vec<uw::Bubble>> = OnceLock::new();
static SPURTS: OnceLock<Vec<uw::Spurt>> = OnceLock::new();

/// Tells the cards which picture the rock is (so a card far from the loading code can use it).
pub fn set_rock(id: egui::TextureId) {
    if let Ok(mut g) = ROCK.lock() {
        *g = Some(id);
    }
}

/// Whether things move (the animation switch in Options, and not while the window is hidden).
pub fn set_motion(on: bool) {
    MOTION.store(on, Ordering::Relaxed);
}

fn rock_texture() -> Option<egui::TextureId> {
    ROCK.lock().ok().and_then(|g| *g)
}

fn a8(x: f32) -> u8 {
    (x.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, a8(a))
}

/// The clock the movement runs on: seconds, or 0 (everything still) while movement is off.
fn clock(ui: &egui::Ui) -> f32 {
    if MOTION.load(Ordering::Relaxed) {
        (ui.input(|i| i.time) % 3600.0) as f32
    } else {
        0.0
    }
}

/// The start of a frame: the slabs are counted again from the first.
fn begin_frame() {
    SLAB_COUNT.store(0, Ordering::Relaxed);
    if let Ok(mut s) = SLABS.lock() {
        s.clear();
    }
}

// ---- the lava ----

/// Paints the lava over the whole panel. Call it first in the panel, before the scroll area;
/// the cards sit on top of it and it stays put while they scroll.
pub fn paint_lava(ui: &egui::Ui, tex: &Textures, t: f32) {
    begin_frame();
    let area = ui.clip_rect();
    if area.width() < 2.0 || area.height() < 2.0 {
        return;
    }
    let painter = ui.painter();
    // the molten lava itself: bright at the top, deeper red at the bottom, glowing a little
    let pulse = 0.94 + 0.06 * (t * 0.9).sin();
    let shade = |r: f32, g: f32, b: f32| Color32::from_rgb((r * pulse).min(255.0) as u8, (g * pulse).min(255.0) as u8, (b * pulse).min(255.0) as u8);
    let (top, bottom) = (shade(255.0, 190.0, 80.0), shade(188.0, 60.0, 12.0));
    let mut base = egui::epaint::Mesh::default();
    let white = egui::epaint::WHITE_UV;
    for (p, c) in [(area.left_top(), top), (area.right_top(), top), (area.right_bottom(), bottom), (area.left_bottom(), bottom)] {
        base.vertices.push(egui::epaint::Vertex { pos: p, uv: white, color: c });
    }
    base.add_triangle(0, 1, 2);
    base.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(base));
    // hot patches drifting sideways
    let drift = (t * 0.11).sin();
    for (fx, fy, rw, a) in [(0.28, 0.14, 0.34, 0.5), (0.74, 0.36, 0.30, 0.42), (0.36, 0.6, 0.36, 0.42), (0.7, 0.82, 0.32, 0.4)] {
        let centre = pos2(area.left() + (fx + 0.06 * drift) * area.width(), area.top() + fy * area.height());
        crate::ui::backdrop::soft_ellipse(painter, centre, vec2(rw * area.width(), 0.06 * area.height()), [255, 226, 140], a);
    }
    // the crust, drifting over it in two layers that go different ways
    let tile_a = uw::TEXTURE_SIZE as f32 * 1.9;
    let tile_b = uw::TEXTURE_SIZE as f32 * 3.0;
    let uv = |tile: f32, ox: f32, oy: f32| Rect::from_min_size(pos2(ox, oy), vec2(area.width() / tile, area.height() / tile));
    painter.image(tex.crust.id(), area, uv(tile_a, t * 0.0045, t * 0.002), rgba(255, 255, 255, 0.9));
    painter.image(tex.far.id(), area, uv(tile_b, 0.3 - t * 0.003, 0.1 + t * 0.0022), Color32::WHITE);
    // the bubbles
    let bubbles = BUBBLES.get_or_init(|| uw::bubbles(2026, 46));
    for b in bubbles {
        let look = uw::bubble_look(b, t);
        let centre = pos2(area.left() + b.x * area.width(), area.top() + b.y * area.height());
        if look.alpha > 0.01 && look.scale > 0.01 {
            let r = b.radius * look.scale;
            painter.circle_filled(centre, r * 1.35, rgba(255, 120, 30, 0.22 * look.alpha));
            painter.circle_filled(centre, r, rgba(255, 150, 40, 0.92 * look.alpha));
            painter.circle_filled(centre + vec2(-r * 0.12, -r * 0.14), r * 0.62, rgba(255, 196, 80, 0.9 * look.alpha));
            painter.circle_filled(centre + vec2(-r * 0.28, -r * 0.3), r * 0.3, rgba(255, 244, 190, 0.95 * look.alpha));
        }
        if look.ring_alpha > 0.01 {
            painter.circle_stroke(centre, b.radius * look.ring, Stroke::new(2.0_f32, rgba(255, 226, 150, look.ring_alpha)));
        }
    }
}

// ---- the rock slabs ----

/// The margin inside a slab: wide enough that the spikes of its edge stay out of the text.
const MARGIN: egui::Margin = egui::Margin { left: 18, right: 18, top: 16, bottom: 18 };

/// A card: a slab of rock lying on the lava, with `add_contents` on it. (The card in `theme.rs`
/// calls this while the Underworld look is on.)
pub fn rock_card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    slab_frame(ui, add_contents)
}

/// A slab behind whatever `add_contents` puts in it.
pub fn slab_frame<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    let index = SLAB_COUNT.fetch_add(1, Ordering::Relaxed);
    let seed = 1000 + index as u64 * 7919;
    let t = clock(ui);
    // The slab floats on the lava (and breathes a little, see `slab_shapes`), and what is on it floats
    // with it: the contents are moved by the same amount, only moved and never scaled, so the text
    // stays crisp.
    let (dx, dy) = uw::float(seed, t);
    let off = vec2(dx, dy);
    let back = ui.painter().add(Shape::Noop);
    let shown = egui::Frame::new().inner_margin(MARGIN).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.with_visual_transform(egui::emath::TSTransform::from_translation(off), add_contents).inner
    });
    let rect = shown.response.rect;
    if let Ok(mut s) = SLABS.lock() {
        s.push(rect.translate(off));
    }
    let outer = Rect::from_min_max(
        pos2(rect.left() - uw::ZONE_SIDE, rect.top() - uw::ZONE_TOP),
        pos2(rect.right() + uw::ZONE_SIDE, rect.bottom() + uw::ZONE_BOTTOM),
    )
    .translate(off);
    let mut shapes = slab_shapes(outer, seed, t);
    // the cracks run across the rock, under the text so that they never get in the way of reading
    let (qw, qh) = (rect.width().max(40.0), rect.height().max(40.0));
    for line in uw::cracks(seed, qw, qh, 2) {
        let points: Vec<Pos2> = line.iter().map(|&(x, y)| pos2(rect.left() + x, rect.top() + y) + off).collect();
        shapes.push(Shape::line(points.clone(), Stroke::new(3.4_f32, rgba(255, 100, 24, 0.28))));
        shapes.push(Shape::line(points, Stroke::new(1.5_f32, Color32::from_rgb(8, 3, 3))));
    }
    ui.painter().set(back, Shape::Vec(shapes));
    shown
}

/// The colour the rock picture is tinted with, from the top of a slab (lit) to the bottom (in
/// shadow, with the lava's red coming up under it). `f` is 0 at the top and 1 at the bottom.
fn rock_tint(f: f32) -> Color32 {
    let f = f.clamp(0.0, 1.0);
    let mix = |a: f32, b: f32| a + (b - a) * f;
    Color32::from_rgb(mix(255.0, 214.0) as u8, mix(244.0, 120.0) as u8, mix(232.0, 92.0) as u8)
}

/// Everything painted behind a slab: the glow round its edge, the rock, the flat planes on its
/// face and its dark outline. `outer` is the slab with its spiky margin.
fn slab_shapes(outer: Rect, seed: u64, t: f32) -> Vec<Shape> {
    // the outline is made at a size rounded to 8 points, so the shape does not shiver when the
    // window grows by a fraction of a point
    let round = |v: f32| ((v / 8.0).round() * 8.0).max(40.0);
    let (qw, qh) = (round(outer.width()), round(outer.height()));
    let local = uw::rock_outline(seed, qw, qh);
    let (sx, sy) = (outer.width() / qw, outer.height() / qh);
    let breathe = uw::breathe(seed, t);
    let centre = outer.center();
    let at = |x: f32, y: f32| -> Pos2 {
        let p = pos2(outer.min.x + x * sx, outer.min.y + y * sy);
        centre + (p - centre) * breathe
    };
    let points: Vec<Pos2> = local.iter().map(|&(x, y)| at(x, y)).collect();
    let glow = uw::glow(seed, t);
    let mut shapes: Vec<Shape> = Vec::new();
    for (width, alpha) in [(13.0_f32, 0.07_f32), (8.0, 0.14), (3.5, 0.3)] {
        shapes.push(Shape::closed_line(points.clone(), Stroke::new(width, rgba(255, 104, 26, alpha * glow))));
    }
    shapes.push(rock_mesh(&points, outer, seed));
    for f in uw::facets(seed, qw, qh, 5) {
        let corners: Vec<Pos2> = f.points.iter().map(|&(x, y)| at(x, y)).collect();
        let fill = if f.light { rgba(255, 205, 160, 0.08) } else { rgba(0, 0, 0, 0.32) };
        shapes.push(Shape::convex_polygon(corners, fill, Stroke::NONE));
    }
    shapes.push(Shape::closed_line(points, Stroke::new(1.0_f32, Color32::from_rgb(14, 6, 5))));
    shapes
}

/// The rock itself: the outline filled as a fan from its middle, with the rock picture on it.
fn rock_mesh(points: &[Pos2], outer: Rect, seed: u64) -> Shape {
    let texture = rock_texture();
    let mut mesh = match texture {
        Some(id) => egui::epaint::Mesh::with_texture(id),
        None => egui::epaint::Mesh::default(),
    };
    let tile = uw::TEXTURE_SIZE as f32 * 1.5;
    let (ox, oy) = ((seed % 97) as f32 / 97.0, ((seed / 97) % 89) as f32 / 89.0);
    let uv_of = |p: Pos2| -> Pos2 {
        match texture {
            Some(_) => pos2(p.x / tile + ox, p.y / tile + oy),
            None => egui::epaint::WHITE_UV,
        }
    };
    // without the picture the tint would make it nearly white, so it is darkened
    let darker = |c: Color32| -> Color32 {
        if texture.is_some() {
            c
        } else {
            Color32::from_rgb((c.r() as f32 * 0.38) as u8, (c.g() as f32 * 0.3) as u8, (c.b() as f32 * 0.28) as u8)
        }
    };
    let centre = outer.center();
    mesh.vertices.push(egui::epaint::Vertex { pos: centre, uv: uv_of(centre), color: darker(rock_tint(0.45)) });
    let height = outer.height().max(1.0);
    for p in points {
        let f = (p.y - outer.top()) / height;
        mesh.vertices.push(egui::epaint::Vertex { pos: *p, uv: uv_of(*p), color: darker(rock_tint(f)) });
    }
    let n = points.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    Shape::mesh(mesh)
}

// ---- the causeway ----

/// The stone causeway across the lava under the title: a chain along each side, and a line of
/// skeletons walking along it. Takes the full width.
pub fn causeway(ui: &mut egui::Ui, t: f32) {
    let width = ui.available_width();
    let height = 66.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    // the stone: bridges the whole width, spikes and all
    let outer = Rect::from_min_max(
        pos2(rect.left() - uw::ZONE_SIDE, rect.top() - 3.0),
        pos2(rect.right() + uw::ZONE_SIDE, rect.bottom() + 4.0),
    );
    let painter = ui.painter();
    for s in slab_shapes(outer, 424_242, t) {
        painter.add(s);
    }
    // a chain along each edge
    let link = rgba(168, 150, 138, 0.8);
    for y in [rect.top() + 8.0, rect.bottom() - 8.0] {
        let mut x = rect.left() + 6.0;
        while x < rect.right() - 4.0 {
            painter.rect_filled(Rect::from_center_size(pos2(x, y), vec2(9.0, 3.2)), egui::CornerRadius::same(2), link);
            x += 13.0;
        }
    }
    // the line of skeletons
    let lane = Rect::from_min_max(pos2(rect.left(), rect.top() + 12.0), pos2(rect.right(), rect.bottom() - 12.0));
    let clipped = painter.with_clip_rect(lane.expand2(vec2(0.0, 1.0)));
    for (x, step) in uw::walkers(t, lane.width(), 64.0, 18.0) {
        let fade = uw::walker_fade(x, lane.width());
        if fade > 0.02 {
            let bob = (t * 5.0 + x * 0.05).sin().abs() * 1.2;
            draw_skeleton(&clipped, pos2(lane.left() + x, lane.bottom() - bob), lane.height(), step, fade, None);
        }
    }
    // (one skeleton used to carry a beating heart here; it is gone for now, and `draw_skeleton` can
    // still give one something to carry)
}

/// One skeleton standing on `feet` and `height` tall, in the given step; `heart` is the size of
/// the beat if it carries one.
fn draw_skeleton(painter: &egui::Painter, feet: Pos2, height: f32, step: usize, fade: f32, heart: Option<f32>) {
    let s = height / 58.0;
    let origin = pos2(feet.x - 15.0 * s, feet.y - 56.0 * s);
    let at = |p: (f32, f32)| origin + vec2(p.0 * s, p.1 * s);
    let bone = rgba(207, 191, 171, fade);
    for b in uw::skeleton(step) {
        painter.line_segment([at(b.a), at(b.b)], Stroke::new((b.w * s).max(1.0), bone));
    }
    painter.circle_filled(at(uw::SKULL.0), uw::SKULL.1 .1 * s, bone);
    for eye in uw::EYES {
        painter.circle_filled(at(eye), 1.3 * s, rgba(255, 122, 42, 0.9 * fade));
    }
    if let Some(beat) = heart {
        let c = at(uw::CHEST);
        let r = 3.6 * s * beat;
        let red = rgba(255, 42, 60, fade);
        painter.circle_filled(c, r * 2.4, rgba(255, 42, 60, 0.16 * fade));
        painter.circle_filled(c + vec2(-0.5 * r, -0.3 * r), 0.6 * r, red);
        painter.circle_filled(c + vec2(0.5 * r, -0.3 * r), 0.6 * r, red);
        painter.add(Shape::convex_polygon(
            vec![c + vec2(-1.04 * r, 0.04 * r), c + vec2(1.04 * r, 0.04 * r), c + vec2(0.0, 1.2 * r)],
            red,
            Stroke::NONE,
        ));
    }
}

// ---- the souls ----

/// The heights (from the top of the screen) of the middles of the gaps between the slabs drawn this
/// frame, where the lava shows; the bottom of the panel if there are no gaps.
fn gap_spots_abs(area: Rect) -> Vec<f32> {
    let mut slabs: Vec<Rect> = SLABS.lock().map(|s| s.clone()).unwrap_or_default();
    slabs.sort_by(|a, b| a.top().partial_cmp(&b.top()).unwrap_or(std::cmp::Ordering::Equal));
    let mut spots: Vec<f32> = Vec::new();
    for pair in slabs.windows(2) {
        let gap = pair[1].top() - pair[0].bottom();
        if gap > 12.0 {
            spots.push(pair[0].bottom() + gap * 0.5);
        }
    }
    if spots.is_empty() {
        spots.push(area.bottom() - 24.0);
    }
    spots
}

/// The same, from the top of `area`, for the souls of Soul Whispers.
pub fn gap_spots(area: Rect) -> Vec<f32> {
    gap_spots_abs(area).into_iter().map(|y| y - area.top()).filter(|y| *y > 0.0 && *y < area.height()).collect()
}

/// The souls that rise out of the lava in spurts. Call it last in the panel, after the scroll
/// area, so they float up over the cards. They come up in the gaps between the cards (where the
/// lava shows), found from the slabs drawn this frame.
pub fn paint_souls(ui: &egui::Ui, t: f32) {
    let area = ui.clip_rect();
    let spots = gap_spots_abs(area);
    let spurts = SPURTS.get_or_init(|| {
        uw::spurts(77, &[(0.12, 0.0), (0.78, 0.0), (0.45, 0.0), (0.9, 0.0), (0.25, 0.0), (0.62, 0.0), (0.05, 0.0)])
    });
    let painter = ui.painter().with_clip_rect(area);
    for (i, spurt) in spurts.iter().enumerate() {
        let y = spots[i % spots.len()];
        let x = area.left() + spurt.x * area.width();
        if y < area.top() || y > area.bottom() {
            continue;
        }
        for soul in &spurt.souls {
            let Some(look) = uw::soul_look(spurt, soul, t) else { continue };
            let origin = pos2(x + soul.jitter.0, y + soul.jitter.1);
            if look.splash_alpha > 0.01 {
                painter.circle_stroke(origin, 13.0 * look.splash, Stroke::new(1.5_f32, rgba(205, 232, 255, look.splash_alpha)));
            }
            let c = origin + vec2(look.dx, look.dy);
            let (s, a) = (look.scale, look.alpha);
            // a soft glow, the head, and a tail that fades away below it
            painter.circle_filled(c, 20.0 * s, rgba(110, 180, 255, 0.11 * a));
            painter.circle_filled(c, 11.0 * s, rgba(130, 196, 255, 0.27 * a));
            for k in 0..3 {
                let top = (5.0 + k as f32 * 7.0) * s;
                painter.line_segment(
                    [c + vec2(0.0, top), c + vec2(0.0, top + 8.0 * s)],
                    Stroke::new((3.4 - k as f32) * s, rgba(190, 225, 255, a * (0.7 - 0.2 * k as f32))),
                );
            }
            painter.circle_filled(c, 5.6 * s, rgba(120, 185, 255, 0.95 * a));
            painter.circle_filled(c + vec2(0.0, -0.8 * s), 3.2 * s, rgba(255, 255, 255, a));
        }
    }
}

// ---- Soul Whispers ----

/// Paints the souls of Soul Whispers (see `whispers.rs`) over the whole panel: the souls, then the
/// words of the ones that whisper on top of them all. Call it where `paint_souls` goes, last in the
/// panel, so they float over the cards.
pub fn paint_whispers(ui: &egui::Ui, field: &crate::whispers::Field) {
    let area = ui.clip_rect();
    let painter = ui.painter().with_clip_rect(area);
    let looks = field.looks();
    paint_whisper_looks(&painter, area, &looks);
}

fn paint_whisper_looks(painter: &egui::Painter, area: Rect, looks: &[crate::whispers::Look]) {
    for look in looks {
        draw_wisp(painter, area.min, look);
    }
    for look in looks {
        if let Some((text, a)) = look.words {
            draw_words(painter, area, area.min + vec2(look.pos.0, look.pos.1), text, a * look.alpha);
        }
    }
}

/// One soul: a soft glow, a tail of fading beads, the head, and when it listens a pair of eyes and
/// a ring that breathes.
fn draw_wisp(painter: &egui::Painter, origin: Pos2, look: &crate::whispers::Look) {
    let (s, a) = (look.scale, look.alpha);
    if a <= 0.01 {
        return;
    }
    let c = origin + vec2(look.pos.0, look.pos.1);
    let n = look.trail.len();
    // a soul that has gone red goes from blue-white to a dull red
    let tone = |blue: [u8; 3], red: [u8; 3], alpha: f32| {
        let m = |i: usize| (blue[i] as f32 + (red[i] as f32 - blue[i] as f32) * look.red) as u8;
        rgba(m(0), m(1), m(2), alpha)
    };
    for (k, p) in look.trail.iter().enumerate().skip(1) {
        let f = 1.0 - k as f32 / n as f32;
        painter.circle_filled(origin + vec2(p.0, p.1), 5.2 * s * (0.3 + 0.7 * f), tone([150, 205, 255], [210, 60, 50], 0.34 * a * f));
    }
    let lit = 1.0 + 0.7 * look.eyes;
    painter.circle_filled(c, 20.0 * s * lit, tone([110, 180, 255], [230, 50, 40], 0.11 * a));
    painter.circle_filled(c, 11.0 * s * lit, tone([130, 196, 255], [235, 70, 50], (0.27 + 0.1 * look.eyes) * a));
    painter.circle_filled(c, 5.6 * s, tone([120, 185, 255], [225, 55, 45], 0.95 * a));
    painter.circle_filled(c + vec2(0.0, -0.8 * s), 3.2 * s, tone([255, 255, 255], [255, 170, 150], a));
    if look.eyes > 0.01 {
        let e = look.eyes;
        for dx in [-1.9_f32, 1.9] {
            painter.circle_filled(c + vec2(dx * s, -0.2 * s), 0.95 * s * e, rgba(24, 40, 84, 0.9 * a));
        }
        painter.circle_stroke(c, 13.0 * s * (0.8 + 0.2 * e), Stroke::new(1.0_f32, rgba(205, 232, 255, 0.35 * e * a)));
    }
}

/// A line of whispered words beside a soul at `c`, on a dark pill so that they read over the cards
/// and the lava alike, on the side with room.
fn draw_words(painter: &egui::Painter, area: Rect, c: Pos2, text: &str, alpha: f32) {
    if alpha <= 0.01 {
        return;
    }
    let galley = painter.layout_no_wrap(text.to_string(), egui::FontId::proportional(13.0), rgba(222, 240, 255, alpha));
    let size = galley.size();
    let mut x = c.x + 18.0;
    if x + size.x + 10.0 > area.right() {
        x = c.x - 18.0 - size.x;
    }
    x = x.clamp(area.left() + 8.0, (area.right() - size.x - 8.0).max(area.left() + 8.0));
    let y = (c.y - size.y * 0.5).clamp(area.top() + 4.0, (area.bottom() - size.y - 4.0).max(area.top() + 4.0));
    let pos = pos2(x, y);
    painter.rect_filled(Rect::from_min_size(pos, size).expand2(vec2(7.0, 3.0)), egui::CornerRadius::same(7), rgba(8, 4, 12, 0.66 * alpha));
    painter.galley(pos, galley, rgba(222, 240, 255, alpha));
}

/// A still-moving picture of Soul Whispers for the preview boxes: a few souls rising, one of them
/// leaning in with its words. `area` is the box; `t` the time (0 holds it still).
pub fn paint_whispers_preview(ui: &egui::Ui, t: f32) {
    let area = ui.clip_rect();
    if area.width() < 4.0 || area.height() < 4.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(area);
    // a glow of lava along the bottom, so the souls have somewhere to come from
    painter.rect_filled(Rect::from_min_max(pos2(area.left(), area.bottom() - area.height() * 0.16), area.right_bottom()), 0.0, rgba(255, 106, 30, 0.35));
    let scale = (area.width() / 260.0).clamp(0.35, 1.3);
    let mut looks = Vec::new();
    for i in 0..7 {
        let f = i as f32;
        let phase = (t * 0.07 + f * 0.143).rem_euclid(1.0);
        let x = area.width() * (0.1 + 0.8 * ((f * 0.37 + 0.11).rem_euclid(1.0))) + (t * 0.8 + f).sin() * 6.0 * scale;
        let y = area.height() * (0.95 - 0.9 * phase);
        let alpha = (phase / 0.15).min(1.0).min((1.0 - phase) / 0.2);
        let mut trail = [(x, y); crate::whispers::TRAIL];
        for (k, p) in trail.iter_mut().enumerate() {
            *p = (x, y + k as f32 * 4.0 * scale);
        }
        looks.push(crate::whispers::Look { pos: (x, y), scale: scale * (0.8 + 0.1 * f.rem_euclid(3.0)), alpha, eyes: 0.0, red: 0.0, trail, words: None });
    }
    // the listener
    let (x, y) = (area.width() * 0.3, area.height() * 0.45);
    let mut trail = [(x, y); crate::whispers::TRAIL];
    for (k, p) in trail.iter_mut().enumerate() {
        *p = (x, y + k as f32 * 2.0 * scale);
    }
    let words = if area.width() >= 150.0 { Some((crate::whispers::LINES[11].text, 1.0)) } else { None };
    looks.push(crate::whispers::Look { pos: (x, y), scale: scale * 1.1, alpha: 1.0, eyes: 1.0, red: 0.0, trail, words });
    paint_whisper_looks(&painter, area, &looks);
}
