// The moon that waits in the corner of The Void at night, the staff in the sand, and what happens
// when the moon is flicked into it (the logic is in moon.rs). It is drawn above the page and only
// takes the pointer where the moon is.
use super::backdrop::soft_ellipse;
use crate::moon::{self, Moon, Phase, Pt};
use crate::theme;
use crate::void;
use eframe::egui::{self, pos2, vec2, Color32, CursorIcon, Pos2, Rect, Shape, Stroke};

/// What the moon wants the program to do.
pub struct Events {
    /// The strike has played out: the staff is broken, Game 3 begins.
    pub finished: bool,
    /// The player skipped ahead in the scene (so its sound is to be cut).
    pub skipped: bool,
    /// How much everything is glitching because of the moon (0 to 1).
    pub glitch: f32,
    /// While the window is to be shaken: how far from where it was. None when it is still.
    pub shake: Option<egui::Vec2>,
}

const GOLD: [u8; 3] = [201, 162, 74];
const SAND: [[u8; 3]; 4] = [[255, 241, 184], [241, 210, 122], [232, 199, 106], [201, 162, 74]];
const LIGHT_GOLD: [u8; 3] = [241, 210, 122];
const PLATE: [u8; 3] = [216, 180, 90];
/// The window shakes for this long after the hit.
const SHAKE_SECS: f32 = 1.3;

fn rgba(c: [u8; 3], a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// Takes the pointer where the moon is, moves it, and draws it all inside `rect` (the area the
/// backdrop covers). Call it every frame while the moon is shown.
pub fn show(ui: &mut egui::Ui, moon: &mut Moon, rect: Rect, animated: bool) -> Events {
    let ctx = ui.ctx().clone();
    let now = ctx.input(|i| i.time);
    let area: Pt = (rect.width(), rect.height());
    let local = |p: Pos2| -> Pt { (p.x - rect.min.x, p.y - rect.min.y) };
    let screen = |p: Pt| -> Pos2 { pos2(rect.min.x + p.0, rect.min.y + p.1) };
    moon.tick(now, area);

    // ---- the hand ----
    if matches!(moon.phase(), Phase::Tucked | Phase::Drifting | Phase::Held) {
        let reach = (moon.radius() + 8.0).max(moon::HIT_R);
        // held, the whole page is the hand's (it can be pulled anywhere); otherwise only the moon is
        let hit = if moon.phase() == Phase::Held {
            rect
        } else {
            Rect::from_center_size(screen(moon.centre()), vec2(reach * 2.0, reach * 2.0)).intersect(rect)
        };
        if hit.width() > 0.0 && hit.height() > 0.0 {
            let response = ui.interact(hit, ui.id().with("void_moon"), egui::Sense::drag());
            if response.drag_started() {
                // where the pointer went down, not where it was when the drag was noticed
                let origin = ctx.input(|i| i.pointer.press_origin()).or(response.interact_pointer_pos());
                if let Some(p) = origin {
                    moon.press(now, local(p), area);
                }
            }
            if response.dragged() {
                if let Some(p) = ctx.pointer_interact_pos() {
                    moon.drag(now, local(p), area);
                }
            }
            if response.drag_stopped() {
                moon.release(now, area);
                moon.tick(now, area);
            }
            if moon.phase() == Phase::Held {
                ctx.set_cursor_icon(CursorIcon::Grabbing);
            } else if response.hover_pos().map_or(false, |p| moon.hovered(local(p), area)) {
                ctx.set_cursor_icon(CursorIcon::Grab);
            }
        }
    }

    // let go outside the window, or in any way the drag was not told about: the moon is let go too
    if moon.phase() == Phase::Held && !ctx.input(|i| i.pointer.primary_down()) {
        moon.release(now, area);
        moon.tick(now, area);
    }

    // ---- the scene: nothing under it can be clicked, and any click or key skips ahead ----
    let mut skipped = false;
    if moon.phase() == Phase::Struck {
        let blocker = egui::Area::new(egui::Id::new("void_moon_blocker"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(&ctx, |ui| ui.allocate_rect(rect, egui::Sense::click_and_drag()));
        let key = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Key { pressed: true, .. })));
        if blocker.inner.clicked() || blocker.inner.drag_started() || key {
            skipped = moon.skip(now);
        }
    }

    // ---- the picture ----
    let painter = ctx
        .layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("void_moon_layer")))
        .with_clip_rect(rect);
    let t = (now % 3600.0) as f32;
    let since = moon.struck_for(now);
    let strike = since.map(moon::strike);
    let head = screen(moon::staff_head(area));

    if let Some(s) = &strike {
        paint_trails(&painter, rect, s.trails);
        paint_streams(&painter, rect, area, since.unwrap_or(0.0));
        paint_dunes(&painter, rect, area, s.flood, 0);
        paint_grains(&painter, rect, area, since.unwrap_or(0.0));
        paint_dunes(&painter, rect, area, s.flood, 1);
    }
    let reveal = moon.staff_reveal(area);
    if reveal > 0.0 {
        let split = strike.map_or(0.0, |s| s.split);
        // before the strike the staff is only half there; the hit makes it solid
        paint_staff(&painter, rect, area, 0.9 * reveal, split, t);
    }

    match moon.phase() {
        Phase::Tucked | Phase::Held | Phase::Drifting | Phase::Falling => {
            let c = screen(moon.centre());
            if matches!(moon.phase(), Phase::Held | Phase::Drifting) {
                paint_thread(&painter, screen(moon::corner(area)), c, 1.0 - moon.pull() * 0.6);
            }
            if moon.phase() == Phase::Falling {
                // the streak it leaves behind, one fringe colour on each side
                let to = head - c;
                let len = to.length().max(1.0);
                let back = -to / len * 46.0;
                painter.line_segment([c + back + vec2(-2.0, 0.0), c], Stroke::new(1.5, theme::VOID_CYAN.gamma_multiply(0.5)));
                painter.line_segment([c + back + vec2(2.0, 0.0), c], Stroke::new(1.5, theme::VOID_MAGENTA.gamma_multiply(0.5)));
            }
            let glow = if moon.phase() == Phase::Tucked { 0.7 + 0.15 * (t * 2.0).sin() } else { 1.0 };
            paint_moon(&painter, c, moon.radius(), moon::fringe(t), glow);
        }
        Phase::Struck => {}
    }

    if let (Some(s), Some(since)) = (&strike, since) {
        paint_hit(&painter, head, s);
        paint_tears(&painter, rect, since);
        // the glitch dies away, the sands mend time, and a void opens: the shadow monarch
        let scene = crate::monarch::scene_time(since);
        super::monarch::paint(&painter, rect, area, scene, t);
        super::monarch::paint_hint(&painter, rect, scene);
    }

    // the whole window glitches at the hit, which would hide the staff just as it breaks, so a calm
    // patch is kept round it, above the glitch (the layer above every other one)
    if let (Some(s), Some(since)) = (&strike, since) {
        let clear = moon::staff_clear(since);
        if clear > 0.0 {
            let (middle, reach) = moon::staff_zone(area);
            let zone = Rect::from_center_size(screen(middle), vec2(reach.0, reach.1) * 2.0).intersect(rect);
            let calm = ctx
                .layer_painter(egui::LayerId::new(egui::Order::Debug, egui::Id::new("void_staff_clear")))
                .with_clip_rect(zone);
            let dark = [3, 1, 9];
            soft_ellipse(&calm, screen(middle), vec2(reach.0, reach.1) * 1.45, dark, 0.95 * clear);
            soft_ellipse(&calm, screen(middle), vec2(reach.0, reach.1) * 1.0, dark, 0.95 * clear);
            paint_grains(&calm, rect, area, since);
            paint_staff(&calm, rect, area, 0.9 * clear * reveal, s.split, t);
            paint_hit(&calm, head, s);
        }
    }

    if moon.phase() != Phase::Tucked || animated {
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
    Events {
        skipped,
        glitch: moon.glitch_level(now, area),
        finished: moon.finished(now),
        // the hit shakes the window, and later each lurch of the monarch and his lunge
        shake: since.and_then(|s| {
            if s < SHAKE_SECS {
                let (x, y) = moon::strike(s).shake;
                Some(vec2(x, y))
            } else {
                crate::monarch::shake(crate::monarch::scene_time(s)).map(|(x, y)| vec2(x, y))
            }
        }),
    }
}

/// The flash and the shockwave rings over the hourglass at the moment of the hit.
fn paint_hit(painter: &egui::Painter, head: Pos2, s: &moon::Strike) {
    if s.flash > 0.0 {
        soft_ellipse(painter, head, vec2(120.0, 120.0), SAND[0], 0.6 * s.flash);
        painter.circle_filled(head, 16.0 * s.flash.min(1.0), Color32::from_white_alpha((s.flash * 230.0) as u8));
    }
    for (radius, alpha) in s.rings {
        if alpha > 0.0 {
            painter.circle_stroke(head, radius, Stroke::new(2.5, rgba(SAND[0], alpha)));
        }
    }
}

/// The moon: pale, with craters, a soft glow, and a cyan and a magenta ghost of itself a little to
/// each side.
fn paint_moon(painter: &egui::Painter, c: Pos2, r: f32, fringe: f32, glow: f32) {
    if r <= 0.0 {
        return;
    }
    soft_ellipse(painter, c, vec2(r * 1.9, r * 1.9), [185, 166, 255], 0.5 * glow);
    painter.circle_filled(c - vec2(fringe, 0.0), r, theme::VOID_CYAN.gamma_multiply(0.55));
    painter.circle_filled(c + vec2(fringe, 0.0), r, theme::VOID_MAGENTA.gamma_multiply(0.55));
    painter.circle_filled(c, r, Color32::from_rgb(230, 222, 250));
    painter.circle_filled(c - vec2(r * 0.22, r * 0.25), r * 0.62, Color32::from_white_alpha(110));
    if r > 14.0 {
        for (a, b, k) in [(-0.35, -0.25, 0.22), (0.28, -0.1, 0.15), (0.05, 0.38, 0.2), (-0.3, 0.3, 0.1), (0.4, 0.35, 0.09)] {
            painter.circle_filled(c + vec2(a * r, b * r), k * r, Color32::from_rgba_unmultiplied(150, 135, 200, 120));
        }
    }
    // a thin slice that has slipped sideways
    let slice = Rect::from_min_size(pos2(c.x - r * 0.8 + fringe * 2.0, c.y - r * 0.08), vec2(r * 1.6, (r * 0.07).max(1.5)));
    painter.rect_filled(slice, 0.0, theme::VOID_CYAN.gamma_multiply(0.3));
}

/// The dotted thread that still ties the moon to its corner.
fn paint_thread(painter: &egui::Painter, from: Pos2, to: Pos2, alpha: f32) {
    let n = ((to - from).length() / 9.0).floor().max(1.0) as usize;
    for k in 1..n {
        let p = from + (to - from) * (k as f32 / n as f32);
        painter.circle_filled(p, 1.3, Color32::from_rgba_unmultiplied(185, 166, 255, (alpha.clamp(0.0, 1.0) * 130.0) as u8));
    }
}

fn ellipse_points(c: Pos2, rx: f32, ry: f32) -> Vec<Pos2> {
    (0..24)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / 24.0;
            c + vec2(a.cos() * rx, a.sin() * ry)
        })
        .collect()
}

/// The staff of time planted in the sand: a rod with the hourglass on top. `split` (0 to 1) is how
/// far the strike has broken it: the hourglass in two halves falling apart, the rod cracked.
fn paint_staff(painter: &egui::Painter, rect: Rect, area: Pt, alpha: f32, split: f32, t: f32) {
    let at = |p: Pt| pos2(rect.min.x + p.0, rect.min.y + p.1);
    let head = at(moon::staff_head(area));
    let base = at(moon::staff_base(area));
    let rod_top = head + vec2(0.0, 20.0);
    soft_ellipse(painter, head, vec2(46.0, 46.0), LIGHT_GOLD, 0.3 * alpha * (0.85 + 0.15 * (t * 3.0).sin()));
    for (rx, ry) in [(34.0, 5.0), (18.0, 3.0)] {
        painter.add(Shape::closed_line(
            ellipse_points(base + vec2(0.0, -6.0), rx, ry),
            Stroke::new(1.0, rgba(GOLD, 0.45 * alpha)),
        ));
    }
    painter.line_segment([rod_top, base + vec2(0.0, 4.0)], Stroke::new(4.0, rgba(GOLD, alpha)));
    painter.line_segment([rod_top + vec2(1.5, 0.0), base + vec2(1.5, 4.0)], Stroke::new(1.0, rgba(LIGHT_GOLD, 0.6 * alpha)));
    let mut y = rod_top.y + 30.0;
    while y < base.y {
        painter.line_segment([pos2(base.x - 3.0, y), pos2(base.x + 3.0, y)], Stroke::new(1.0, rgba([122, 93, 34], alpha)));
        y += 22.0;
    }
    let dark = rgba([26, 18, 4], alpha);
    let edge = Stroke::new(1.6, rgba([232, 199, 106], alpha));
    if split <= 0.0 {
        let p = |x: f32, y: f32| head + vec2(x, y);
        for (a, b, c) in [((-13.0, -14.0), (13.0, -14.0), (0.0, 0.0)), ((0.0, 0.0), (13.0, 14.0), (-13.0, 14.0))] {
            painter.add(Shape::convex_polygon(vec![p(a.0, a.1), p(b.0, b.1), p(c.0, c.1)], dark, edge));
        }
        for y in [-17.0, 17.0] {
            painter.rect_filled(Rect::from_center_size(p(0.0, y), vec2(34.0, 5.0)), 0.0, rgba(PLATE, alpha));
        }
        painter.add(Shape::convex_polygon(
            vec![p(-8.0, 14.0), p(8.0, 14.0), p(0.0, 6.0)],
            rgba(LIGHT_GOLD, alpha),
            Stroke::NONE,
        ));
        painter.line_segment([p(0.0, -6.0), p(0.0, 6.0)], Stroke::new(1.2, rgba(LIGHT_GOLD, alpha)));
        return;
    }
    // the two halves tilt away from each other and drop a little; the right one is the left one
    // mirrored
    for side in [-1.0_f32, 1.0] {
        let (sin, cos) = (side * 0.55 * split).sin_cos();
        let place = |x: f32, y: f32| -> Pos2 {
            let x = x * -side;
            head + vec2(x * cos - y * sin + side * 24.0 * split, x * sin + y * cos + 6.0 * split)
        };
        for tri in [[(-13.0, -14.0), (0.0, -14.0), (0.0, 0.0)], [(0.0, 0.0), (0.0, 14.0), (-13.0, 14.0)]] {
            let pts: Vec<Pos2> = tri.iter().map(|(x, y)| place(*x, *y)).collect();
            painter.add(Shape::convex_polygon(pts, rgba([90, 68, 18], alpha), Stroke::new(2.0, rgba(LIGHT_GOLD, alpha))));
        }
        for y in [-17.0_f32, 17.0] {
            let pts: Vec<Pos2> = [(-15.0, y - 2.5), (0.0, y - 2.5), (0.0, y + 2.5), (-15.0, y + 2.5)]
                .iter()
                .map(|(x, y)| place(*x, *y))
                .collect();
            painter.add(Shape::convex_polygon(pts, rgba(PLATE, alpha), Stroke::NONE));
        }
    }
    let crack = [(0.0, 0.0), (6.0, 10.0), (-4.0, 20.0), (5.0, 32.0), (0.0, 44.0)]
        .iter()
        .map(|(x, y)| rod_top + vec2(*x, *y))
        .collect::<Vec<_>>();
    painter.add(Shape::line(crack, Stroke::new(3.0, rgba(LIGHT_GOLD, alpha * split.min(1.0)))));
}

fn paint_grains(painter: &egui::Painter, rect: Rect, area: Pt, since: f32) {
    let head = moon::staff_head(area);
    let ground = area.1 - 14.0;
    for i in 0..moon::GRAINS {
        let Some(g) = moon::grain(i, since, head, ground, area.0) else { continue };
        let p = pos2(rect.min.x + g.pos.0, rect.min.y + g.pos.1);
        painter.rect_filled(Rect::from_center_size(p, vec2(g.size, g.size)), 0.0, rgba(SAND[g.tone as usize % 4], g.alpha));
    }
}

fn paint_streams(painter: &egui::Painter, rect: Rect, area: Pt, since: f32) {
    for (top, bottom, alpha, width) in moon::streams(since, area) {
        if alpha <= 0.0 {
            continue;
        }
        let a = pos2(rect.min.x + top.0, rect.min.y + top.1);
        let b = pos2(rect.min.x + bottom.0, rect.min.y + bottom.1);
        painter.line_segment([a, b], Stroke::new(width, rgba([232, 199, 106], alpha)));
    }
}

/// The sand piling up along the bottom, in columns.
fn paint_dunes(painter: &egui::Painter, rect: Rect, area: Pt, flood: f32, layer: usize) {
    if flood <= 0.0 {
        return;
    }
    let colour = if layer == 0 { rgba(GOLD, 0.55) } else { rgba([232, 199, 106], 0.6) };
    let step = 8.0;
    let mut x = 0.0;
    while x < area.0 {
        let h = moon::dune(layer, (x + step / 2.0) / area.0, flood, area.1);
        if h > 0.5 {
            painter.rect_filled(
                Rect::from_min_max(pos2(rect.min.x + x, rect.max.y - h), pos2(rect.min.x + (x + step + 1.0).min(area.0), rect.max.y)),
                0.0,
                colour,
            );
        }
        x += step;
    }
}

/// The stars stretch into slow trails as time comes loose.
fn paint_trails(painter: &egui::Painter, rect: Rect, amount: f32) {
    if amount <= 0.0 {
        return;
    }
    for i in 0..void::STARS {
        let s = void::star(i);
        let p = pos2(rect.min.x + s.x * rect.width(), rect.min.y + s.y * rect.height());
        painter.line_segment(
            [p, p + vec2(0.0, amount * 14.0 * s.size)],
            Stroke::new(s.size * 0.8, Color32::from_rgba_unmultiplied(244, 239, 255, (amount * 110.0) as u8)),
        );
    }
}

/// Bars of cyan and magenta tearing across the page in the first moments after the hit.
fn paint_tears(painter: &egui::Painter, rect: Rect, since: f32) {
    if since >= 0.6 {
        return;
    }
    let fade = 1.0 - since / 0.6;
    let frame = (since * 40.0) as u32;
    for k in 0..5u32 {
        let y = ((k * 7919 + frame * 131) % 1000) as f32 / 1000.0 * rect.height();
        let h = 3.0 + (k % 3) as f32 * 2.0;
        let colour = if k % 2 == 0 { theme::VOID_CYAN } else { theme::VOID_MAGENTA };
        painter.rect_filled(
            Rect::from_min_size(pos2(rect.min.x, rect.min.y + y), vec2(rect.width(), h)),
            0.0,
            colour.gamma_multiply(0.3 * fade),
        );
    }
}
