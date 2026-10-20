// The cat before the crown: the scene you talk to him in (the way into Game 1).
//
// A night market under the clock tower. He sits in the middle of it in a wine-red frock coat, ill
// and composed, half aged by what it cost him to climb out of Time: the age spreads from the eye
// under his monocle, down his right cheek (ours, on the right) and into the fang on that side. Now
// and then he coughs, and sometimes the cough starts with the twitch the dead Monarch King has
// later. The clock's light swells now and then: Time, calling for him.
//
// What he says, the choice bars and the cough are worked out in `cat_talk.rs`; this is only the
// picture. His face is the Monarch King's (`king::cat_scene`) in two-tone fur, with no crown.
use super::backdrop::soft_ellipse;
use super::dead_king::{centre, fill_fan};
use crate::cat_talk::{self, Ask, Cough, CoughPose, Mood, Outcome, Speaker, Talk};
use crate::king;
use eframe::egui::{self, pos2, vec2, Color32, Pos2, Rect, Shape, Stroke};
use std::sync::Arc;

// two-tone fur: a lighter ash-black over dark bronze
const BLACK: [f32; 3] = [84.0, 82.0, 100.0];
const BRONZE: [f32; 3] = [150.0, 100.0, 46.0];
const BRONZE_D: [f32; 3] = [104.0, 66.0, 30.0];
const BRONZE_L: [f32; 3] = [206.0, 150.0, 70.0];
const ASH: [f32; 3] = [206.0, 202.0, 198.0];
const OUT: Color32 = Color32::from_rgb(18, 14, 14);
const FANG: Color32 = Color32::from_rgb(240, 228, 196);
const NOSE: Color32 = Color32::from_rgb(214, 120, 104);
const GOLD: Color32 = Color32::from_rgb(244, 200, 80);
const GOLD_D: Color32 = Color32::from_rgb(120, 80, 20);
const WINE: Color32 = Color32::from_rgb(30, 25, 31);
const WINE_D: Color32 = Color32::from_rgb(17, 14, 19);
const WINE_L: Color32 = Color32::from_rgb(150, 30, 48);

/// The red that is left of the old wine-red coat: linings and little touches only.
const RED_TRIM: Color32 = Color32::from_rgb(112, 22, 38);

/// The scene: the talk, his cough, and when things happened (egui time).
pub struct Scene {
    talk: Talk,
    cough: Cough,
    opened: f64,
    line_since: f64,
    /// you said yes: he stands straighter
    agreed: bool,
    /// the last thing he said (it stays up while the bars are)
    last_cat: Option<&'static str>,
    /// the look on his face with it
    last_mood: Mood,
    /// how angry he looks now, 0..1 (it eases in and out)
    anger: f32,
}

/// What the scene wants the app to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    None,
    /// Walked away (Esc): nothing changes.
    Leave,
    /// The talk is over.
    Done(Outcome),
}

impl Scene {
    /// `declines`: how many times you have said no to him before.
    pub fn new(declines: u8, seed: u32, now: f64) -> Scene {
        Scene {
            talk: Talk::new(declines),
            cough: Cough::new(seed),
            opened: now,
            line_since: now,
            agreed: false,
            last_cat: None,
            last_mood: Mood::Calm,
            anger: 0.0,
        }
    }
}

fn rgb(c: [f32; 3]) -> Color32 {
    Color32::from_rgb(
        c[0].clamp(0.0, 255.0) as u8,
        c[1].clamp(0.0, 255.0) as u8,
        c[2].clamp(0.0, 255.0) as u8,
    )
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn alpha(c: [u8; 3], a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// A tiny repeatable random number (for things that must not flicker from frame to frame).
fn hash(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x % 10_000) as f32 / 10_000.0
}

/// Draws the scene and handles the clicks and keys.
pub fn show(ui: &mut egui::Ui, scene: &mut Scene, animated: bool) -> Exit {
    let rect = ui.max_rect();
    if rect.width() < 200.0 || rect.height() < 160.0 {
        return Exit::None;
    }
    let now = ui.input(|i| i.time);
    let t = (now - scene.opened) as f32;
    scene.cough.update(t);
    let (w, h) = (rect.width(), rect.height());
    let background = ui.interact(rect, ui.id().with("cat_talk_background"), egui::Sense::click());

    // what is being said
    let line = scene.talk.line();
    if let Some(l) = line {
        if l.who == Speaker::Cat {
            scene.last_cat = Some(l.text);
            scene.last_mood = l.mood;
        }
    }
    let since = (now - scene.line_since) as f32;
    let typing = line.is_some_and(|l| cat_talk::revealed(l.text, since) < l.text.chars().count());
    let cat_talking = typing && line.is_some_and(|l| l.who == Speaker::Cat);

    // the cat
    // he stands back from you: the whole of him, boots and all, fits in the picture
    let k = (h / 820.0).min(w * 0.26 / 430.0).min(h / 1500.0);
    let hx = rect.left() + w * 0.27;
    let hy = rect.top() + h * 0.08 + 170.0 * k;
    let s = 0.5 * k;
    let cp = if animated { scene.cough.pose(t) } else { CoughPose::default() };
    let painter = ui.painter_at(rect);
    // the brows come down (and go back up) over about a third of a second
    let anger_target = if scene.last_mood == Mood::Annoyed { 1.0 } else { 0.0 };
    let dt = ui.input(|i| i.stable_dt).min(0.1);
    scene.anger += (anger_target - scene.anger) * (1.0 - (-9.0 * dt).exp());
    if (anger_target - scene.anger).abs() < 0.01 {
        scene.anger = anger_target;
    }
    let look = CatLook {
        agreed: scene.agreed,
        talking: cat_talking,
        interested: false,
        gaze: 0.0,
        head: 0.5,
        cp,
        anger: scene.anger,
    };
    paint_backdrop(&painter, rect, t, animated, Some(pos2(hx, hy)), k);
    paint_cat(&painter, hx, hy, k, &look, t, animated);

    // ------------------------------------------------------------------------- what is said
    let col_left = rect.left() + w * 0.50;
    let col_width = (w * 0.45).max(200.0);
    let fs = (h * 0.03).clamp(15.0, 30.0);
    let pad = fs * 0.8;
    let font = egui::FontId::proportional(fs);
    let mut next = false;
    let mut choice: Option<Ask> = None;
    let mut y = rect.top() + h * 0.09;

    // his bubble: what he is saying, or the last thing he said while the bars are up
    let cat_text: Option<(&'static str, bool)> = match line {
        Some(l) if l.who == Speaker::Cat => Some((l.text, false)),
        _ => scene.last_cat.map(|t| (t, true)),
    };
    if let Some((text, dim)) = cat_text {
        let shown_n = if dim { text.chars().count() } else { cat_talk::revealed(text, since) };
        let ink =
            if dim { Color32::from_rgb(190, 178, 206) } else { Color32::from_rgb(250, 240, 214) };
        let r = speech(
            &painter,
            text,
            shown_n,
            &font,
            ink,
            pos2(col_left, y),
            col_width,
            pad,
            Color32::from_rgb(18, 12, 34),
            if dim { GOLD.gamma_multiply(0.5) } else { GOLD },
            Some(pos2(hx + 330.0 * s, hy + 110.0 * s)),
        );
        y = r.bottom() + pad;
        if !dim && !typing {
            blink_arrow(&painter, pos2(r.right() - pad, r.bottom() - pad * 0.7), pad * 0.45, t);
        }
    }
    // your words
    if let Some(l) = line {
        if l.who == Speaker::You {
            let shown_n = cat_talk::revealed(l.text, since);
            let r = speech(
                &painter,
                l.text,
                shown_n,
                &font,
                Color32::from_rgb(255, 232, 244),
                pos2(col_left + col_width * 0.18, y),
                col_width * 0.82,
                pad,
                Color32::from_rgb(30, 12, 44),
                Color32::from_rgb(255, 100, 170),
                None,
            );
            if !typing {
                blink_arrow(&painter, pos2(r.right() - pad, r.bottom() - pad * 0.7), pad * 0.45, t);
            }
        }
    }
    // a click moves on (the first one finishes the typing)
    if line.is_some() {
        let confirm = background.clicked()
            || ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
        if confirm {
            if typing {
                scene.line_since = now - 1000.0;
            } else {
                next = true;
            }
        }
        if background.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }

    // the choice bars
    if scene.talk.choosing() {
        let options = scene.talk.options();
        let bar_h = (h * 0.062).clamp(34.0, 56.0);
        let gap = bar_h * 0.18;
        let total = options.len() as f32 * (bar_h + gap);
        let top = rect.bottom() - h * 0.06 - total;
        let keys = [
            egui::Key::Num1,
            egui::Key::Num2,
            egui::Key::Num3,
            egui::Key::Num4,
            egui::Key::Num5,
            egui::Key::Num6,
        ];
        for (i, ask) in options.iter().enumerate() {
            let bar = Rect::from_min_size(
                pos2(col_left, top + i as f32 * (bar_h + gap)),
                vec2(col_width, bar_h),
            );
            let response =
                ui.interact(bar, ui.id().with(("cat_talk_bar", i)), egui::Sense::click());
            let hot = response.hovered();
            let (fill, edge, ink) = match ask {
                Ask::Help => (
                    if hot { GOLD } else { Color32::from_rgb(70, 50, 14) },
                    GOLD,
                    if hot {
                        Color32::from_rgb(40, 24, 8)
                    } else {
                        Color32::from_rgb(255, 232, 160)
                    },
                ),
                Ask::Decline => (
                    if hot { Color32::from_rgb(60, 44, 90) } else { Color32::from_rgb(30, 20, 50) },
                    Color32::from_rgb(120, 120, 170),
                    Color32::from_rgb(214, 214, 238),
                ),
                _ => (
                    if hot { Color32::from_rgb(54, 30, 74) } else { Color32::from_rgb(30, 14, 46) },
                    Color32::from_rgb(255, 100, 170),
                    Color32::from_rgb(255, 224, 240),
                ),
            };
            painter.rect_filled(bar, 8.0, fill);
            painter.rect_stroke(
                bar,
                8.0,
                Stroke::new(if hot { 2.4 } else { 1.4 }, edge),
                egui::StrokeKind::Inside,
            );
            painter.text(
                pos2(bar.left() + pad, bar.center().y),
                egui::Align2::LEFT_CENTER,
                format!("{}   {}", i + 1, ask.label()),
                egui::FontId::proportional(fs * 0.95),
                ink,
            );
            if hot {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked()
                || keys.get(i).is_some_and(|key| ui.input(|inp| inp.key_pressed(*key)))
            {
                choice = Some(*ask);
            }
        }
    }
    // a way out
    let leave =
        Rect::from_min_size(pos2(rect.right() - 150.0, rect.top() + 8.0), vec2(142.0, 26.0));
    let leave_response = ui.interact(leave, ui.id().with("cat_talk_leave"), egui::Sense::click());
    painter.text(
        leave.right_center(),
        egui::Align2::RIGHT_CENTER,
        "Esc: walk away",
        egui::FontId::proportional(13.0),
        Color32::from_rgba_unmultiplied(
            200,
            190,
            230,
            if leave_response.hovered() { 230 } else { 110 },
        ),
    );
    if leave_response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    // ------------------------------------------------------------------------- what happened
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) || leave_response.clicked() {
        return Exit::Leave;
    }
    if let Some(ask) = choice {
        if ask == Ask::Help {
            scene.agreed = true;
        }
        scene.talk.choose(ask);
        scene.line_since = now;
    } else if next {
        scene.talk.advance();
        scene.line_since = now;
    }
    if let Some(o) = scene.talk.outcome() {
        return Exit::Done(o);
    }
    if animated {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }
    Exit::None
}

/// How he carries himself right now.
struct CatLook {
    /// you said yes: he stands straighter
    agreed: bool,
    /// he is speaking (his jaw moves)
    talking: bool,
    /// the cursor is on him (his brows go up)
    interested: bool,
    /// degrees his head turns toward the cursor (he watches you)
    gaze: f32,
    /// how big the head is drawn, as a fraction of the coat's scale (0.5 in the talk; less on the page, where he is small)
    head: f32,
    cp: CoughPose,
    /// 0..1: slightly angry (the brows down, the eyes narrowed a little)
    anger: f32,
}

/// The cat himself: coat, head, cough and all. `hx`, `hy` is the middle of his head; `k` is how many
/// screen points one point of the coat drawing is (his head is drawn at half that).
#[allow(clippy::too_many_arguments)]
fn paint_cat(
    painter: &egui::Painter,
    hx: f32,
    hy: f32,
    k: f32,
    look: &CatLook,
    t: f32,
    animated: bool,
) {
    let s = look.head * k;
    let cp = look.cp;
    let blink = animated && (t * 0.213 + 0.5).fract() < 0.022;
    let breathe = if animated { (t * 1.3).sin() } else { 0.0 };
    let (base_tilt, base_brow, glow) = if look.agreed {
        (-2.0, 0.0, 0.85)
    } else {
        (2.0, if look.interested { 0.3 } else { 0.12 }, 0.5)
    };
    let pose = king::Pose {
        bob: breathe * 2.0 * k,
        tilt: base_tilt + cp.tilt + look.gaze,
        jaw: cp.jaw + if look.talking { 0.05 + 0.05 * (t * 20.0).sin().abs() } else { 0.0 },
        squint: if blink { 1.0 } else { cp.squint + look.anger * 0.18 },
        brow: base_brow + cp.brow * 0.8 - look.anger * 0.55,
        shoulders: cp.shake * k,
        crown_dx: 0.0,
        crown_tilt: 0.0,
        glow,
    };
    let polys: Vec<(Vec<Pos2>, king::Poly)> = king::cat_scene(&pose, (hx, hy), s)
        .into_iter()
        .map(|poly| (poly.points.iter().map(|&p| pos2(p.0, p.1)).collect(), poly))
        .collect();
    let body_dy = -pose.shoulders + pose.bob * 0.35;
    let neck_y = hy + 380.0 * s + body_dy;
    let head_at = |q: (f32, f32)| {
        let p = king::head_point(&pose, (hx, hy), s, q);
        pos2(p.0, p.1)
    };
    paint_body(painter, hx, neck_y, k, cp.hanky);
    let age = Age {
        path: cat_talk::spread_path(8)
            .into_iter()
            .map(|q| king::head_point(&pose, (hx, hy), s, q))
            .collect(),
        s,
        top: hy - 330.0 * s,
        height: 700.0 * s,
    };
    paint_head(painter, &polys, &age, hx, s, k, &head_at, &pose, t, animated);
    if cp.hanky > 0.02 {
        paint_cough_arm(painter, hx, hy, neck_y, s, k, cp.hanky);
    }
    if animated && cp.shake.abs() > 0.5 {
        // the shake, in little arcs either side of the head
        for sx in [-1.0f32, 1.0] {
            for i in 0..3 {
                let c = pos2(
                    hx + sx * (470.0 + i as f32 * 30.0) * s,
                    hy + (40.0 - i as f32 * 26.0) * s,
                );
                let r = 16.0 * s;
                let pts: Vec<Pos2> = (0..8)
                    .map(|j| {
                        let a = if sx < 0.0 {
                            200.0 + 140.0 * j as f32 / 7.0
                        } else {
                            -20.0 + 140.0 * j as f32 / 7.0
                        };
                        c + vec2(a.to_radians().cos(), a.to_radians().sin()) * r
                    })
                    .collect();
                painter.add(Shape::line(
                    pts,
                    Stroke::new(
                        (2.0 * k).max(1.0),
                        Color32::from_rgba_unmultiplied(230, 220, 250, 150),
                    ),
                ));
            }
        }
    }
}

/// The cat himself drawn for another scene (the Ally's archway): `hx`, `hy` is the middle of his head
/// and `k` the scale, as in the talk. `talking` moves his jaw.
pub fn paint_cat_portrait(
    painter: &egui::Painter,
    hx: f32,
    hy: f32,
    k: f32,
    t: f32,
    animated: bool,
    talking: bool,
) {
    let look = CatLook {
        agreed: false,
        talking,
        interested: false,
        gaze: 0.0,
        head: 0.5,
        cp: CoughPose::default(),
        anger: 0.0,
    };
    paint_cat(painter, hx, hy, k, &look, t, animated);
}

// ------------------------------------------------------------------------------- speech bubbles
/// A speech bubble with `text` typed out to `shown` characters. Returns its rectangle.
#[allow(clippy::too_many_arguments)]
pub(super) fn speech(
    painter: &egui::Painter,
    text: &str,
    shown: usize,
    font: &egui::FontId,
    ink: Color32,
    at: Pos2,
    width: f32,
    pad: f32,
    fill: Color32,
    edge: Color32,
    tail: Option<Pos2>,
) -> Rect {
    let wrap = (width - 2.0 * pad).max(40.0);
    let full: Arc<egui::Galley> = painter.layout(text.to_string(), font.clone(), ink, wrap);
    let part: String = text.chars().take(shown).collect();
    let typed: Arc<egui::Galley> = painter.layout(part, font.clone(), ink, wrap);
    let r = Rect::from_min_size(at, full.size() + vec2(2.0 * pad, 2.0 * pad));
    if let Some(tip) = tail {
        let a = pos2(r.left() + 1.0, r.top() + r.height() * 0.30);
        let b = pos2(r.left() + 1.0, r.top() + r.height() * 0.66);
        painter.add(Shape::convex_polygon(vec![a, b, tip], fill, Stroke::new(1.6, edge)));
    }
    painter.rect_filled(r, 12.0, fill);
    painter.rect_stroke(r, 12.0, Stroke::new(2.0, edge), egui::StrokeKind::Inside);
    painter.galley(r.min + vec2(pad, pad), typed, ink);
    r
}

fn blink_arrow(painter: &egui::Painter, at: Pos2, size: f32, t: f32) {
    let a = (0.5 + 0.5 * (t * 5.0).sin()).clamp(0.0, 1.0);
    painter.add(Shape::convex_polygon(
        vec![
            at + vec2(-size, -size * 0.6),
            at + vec2(size, -size * 0.6),
            at + vec2(0.0, size * 0.7),
        ],
        Color32::from_rgba_unmultiplied(244, 200, 80, (90.0 + 165.0 * a) as u8),
        Stroke::NONE,
    ));
}

// ------------------------------------------------------------------------------- the backdrop
/// A rectangle with a colour at each corner (top-left, top-right, bottom-right, bottom-left).
fn gradient(painter: &egui::Painter, r: Rect, c: [Color32; 4]) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(r.left_top(), c[0]);
    mesh.colored_vertex(r.right_top(), c[1]);
    mesh.colored_vertex(r.right_bottom(), c[2]);
    mesh.colored_vertex(r.left_bottom(), c[3]);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

/// The night market under the clock tower, whose light swells now and then.
fn paint_backdrop(
    painter: &egui::Painter,
    r: Rect,
    t: f32,
    animated: bool,
    head: Option<Pos2>,
    k: f32,
) {
    let (w, h) = (r.width(), r.height());
    let at = |x: f32, y: f32| pos2(r.left() + x * w, r.top() + y * h);
    let tm = if animated { t } else { 0.0 };
    // Circuit City: the sky and the skyline are parts of a circuit board
    let base = r.bottom() - h * 0.12;
    super::circuit_city::paint_city(painter, r, base, tm, animated);
    let focus = head.unwrap_or_else(|| at(0.5, 0.3));
    soft_ellipse(
        painter,
        pos2(focus.x, focus.y + 120.0 * k),
        vec2(w * 0.34, h * 0.5),
        [60, 200, 190],
        0.14,
    );
    // the clock tower, lit, with a locked door at its foot
    let hh = base - (r.top() + h * 0.06);
    paint_clock_tower(
        painter,
        r.left() + w * 0.075,
        base,
        r.top() + h * 0.06,
        (w * 0.03).max(hh * 0.055),
        tm,
        animated,
    );
    // the ground
    let ground = Rect::from_min_max(pos2(r.left(), base), r.right_bottom());
    gradient(
        painter,
        ground,
        [
            Color32::from_rgb(8, 26, 30),
            Color32::from_rgb(8, 26, 30),
            Color32::from_rgb(3, 10, 14),
            Color32::from_rgb(3, 10, 14),
        ],
    );
    super::circuit_city::paint_street_traces(painter, ground, tm, animated);
    // string lights over the market
    for (row, (y0, sag)) in [(0.035f32, 0.05f32), (0.11, 0.045)].into_iter().enumerate() {
        let bulbs = 16;
        let mut prev: Option<Pos2> = None;
        for b in 0..=bulbs {
            let u = b as f32 / bulbs as f32;
            let sway = if animated { (tm * 0.8 + u * 4.0 + row as f32).sin() * 2.0 } else { 0.0 };
            let p = at(u, y0 + sag * (u * std::f32::consts::PI).sin()) + vec2(0.0, sway);
            if let Some(q) = prev {
                painter.line_segment([q, p], Stroke::new(1.2, alpha([40, 30, 50], 0.9)));
            }
            prev = Some(p);
            let hue = [[255, 200, 110], [255, 120, 180], [130, 220, 255], [255, 220, 150]]
                [(b + row * 2) % 4];
            let flick =
                if animated { 0.75 + 0.25 * (tm * 2.0 + b as f32 * 1.7).sin() } else { 1.0 };
            soft_ellipse(painter, p + vec2(0.0, 6.0), vec2(w * 0.016, w * 0.016), hue, 0.5 * flick);
            painter.circle_filled(p + vec2(0.0, 5.0), 3.0, alpha(hue, 1.0));
        }
    }
    // pocket watches drifting in the air
    for i in 0..7u32 {
        let fx = 0.46 + 0.5 * hash(i + 400);
        let fy = 0.2 + 0.5 * hash(i + 500);
        let bob = if animated { (tm * 0.7 + i as f32 * 1.9).sin() * h * 0.012 } else { 0.0 };
        let c = at(fx, fy) + vec2(0.0, bob);
        let wr = h * (0.012 + 0.01 * hash(i + 600));
        painter.add(Shape::line(
            vec![c + vec2(0.0, -wr), c + vec2(wr * 0.4, -wr * 2.0), c + vec2(-wr * 0.3, -wr * 3.0)],
            Stroke::new(1.0, alpha([244, 200, 80], 0.5)),
        ));
        painter.circle_filled(c, wr, alpha([244, 200, 80], 0.8));
        painter.circle_filled(c, wr * 0.78, alpha([250, 238, 204], 0.8));
        let a = tm * (0.6 + hash(i + 700)) + i as f32;
        painter.line_segment(
            [c, c + vec2(a.sin(), -a.cos()) * wr * 0.6],
            Stroke::new(1.2, alpha([60, 40, 20], 0.8)),
        );
        painter.line_segment(
            [c, c + vec2((a * 0.083).sin(), -(a * 0.083).cos()) * wr * 0.4],
            Stroke::new(1.6, alpha([60, 40, 20], 0.8)),
        );
    }
    // the edges go dark
    let dark = Color32::from_rgba_unmultiplied(4, 2, 10, 210);
    let clear = Color32::from_rgba_unmultiplied(4, 2, 10, 0);
    gradient(painter, Rect::from_min_size(r.min, vec2(w * 0.16, h)), [dark, clear, clear, dark]);
    gradient(
        painter,
        Rect::from_min_max(pos2(r.right() - w * 0.16, r.top()), r.right_bottom()),
        [clear, dark, dark, clear],
    );
    gradient(painter, Rect::from_min_size(r.min, vec2(w, h * 0.12)), [dark, dark, clear, clear]);
    gradient(
        painter,
        Rect::from_min_max(pos2(r.left(), r.bottom() - h * 0.14), r.right_bottom()),
        [clear, clear, dark, dark],
    );
}

// ------------------------------------------------------------------------------- his clothes
fn poly(painter: &egui::Painter, pts: &[Pos2], fill: Color32, edge: Option<(Color32, f32)>) {
    fill_fan(painter, pts, fill, vec2(0.0, 0.0));
    if let Some((c, wd)) = edge {
        painter.add(Shape::closed_line(pts.to_vec(), Stroke::new(wd, c)));
    }
}

/// How wide his body is next to the drawing's own measures: the coat was cut for a much broader
/// gentleman, and he looked as if he wore an overcoat three sizes too big.
const BODY_W: f32 = 0.58;

/// How much longer the body is from the collar to the belt than the first drawing made it (he
/// was stubby): the waistcoat is stretched, and everything below the belt moves down with it.
const TALL: f32 = 90.0;

/// How much longer his trousers are than the first drawing made them (he was stubby there too).
const LEGS: f32 = 90.0;

/// Where his feet are below his neck, in the coat's own units (the boots end at 752 in the drawing).
const FEET_BELOW_NECK: f32 = 752.0 + TALL + LEGS;

/// Where a height in the drawing falls once the body has been made taller.
fn body_y(y: f32) -> f32 {
    if y <= 200.0 {
        y
    } else if y <= 388.0 {
        200.0 + (y - 200.0) * (188.0 + TALL) / 188.0
    } else if y <= 520.0 {
        y + TALL
    } else if y <= 640.0 {
        // the trouser legs are stretched; the boots keep their shape and move down
        520.0 + TALL + (y - 520.0) * (120.0 + LEGS) / 120.0
    } else {
        y + TALL + LEGS
    }
}

fn body_pt(hx: f32, neck_y: f32, k: f32, x: f32, y: f32) -> Pos2 {
    pos2(hx + x * k * BODY_W, neck_y + body_y(y) * k)
}

/// The arm that hangs at his side (ours, the right), and comes up for the cough: where the
/// shoulder, elbow, wrist and paw are. `up` is how far it has come (0 hangs, 1 holds a cloth to his
/// mouth) and `raised_paw` is where the paw is then.
struct Arm {
    shoulder: Pos2,
    elbow: Pos2,
    wrist: Pos2,
    paw: Pos2,
}

/// How long his arm is (the same for both): shoulder to elbow, and elbow to the middle of the paw.
const UPPER_ARM: f32 = 185.0;
const FOREARM: f32 = 210.0;

/// Where the elbow is for a shoulder and a paw: of the two places two bones of his arm's lengths can
/// reach the paw from, the one nearer `pole`. A paw too far for the arm is stopped at its reach, and
/// that place is returned with the elbow. The bones keep their length at every angle.
fn bend_arm(shoulder: Pos2, paw: Pos2, k: f32, pole: Pos2) -> (Pos2, Pos2) {
    let (l1, l2) = (UPPER_ARM * k, FOREARM * k);
    let to_paw = paw - shoulder;
    let dist = to_paw.length();
    let d = dist.clamp((l1 - l2).abs() + 1.0, l1 + l2 - 1.0);
    let dir = if dist > 0.001 { to_paw / dist } else { vec2(0.0, 1.0) };
    let reach = shoulder + dir * d;
    let a = (l1 * l1 - l2 * l2 + d * d) / (2.0 * d);
    let h = (l1 * l1 - a * a).max(0.0).sqrt();
    let perp = vec2(-dir.y, dir.x);
    let c1 = shoulder + dir * a + perp * h;
    let c2 = shoulder + dir * a - perp * h;
    let elbow = if (c1 - pole).length() < (c2 - pole).length() { c1 } else { c2 };
    (elbow, reach)
}

fn finish_arm(shoulder: Pos2, elbow: Pos2, paw: Pos2, k: f32) -> Arm {
    let wrist = paw + (elbow - paw).normalized() * 50.0 * k;
    Arm { shoulder, elbow, wrist, paw }
}

fn right_arm(hx: f32, neck_y: f32, k: f32, up: f32, raised_paw: Pos2) -> Arm {
    // measured in the drawing's units straight down from the neck (the arm is not stretched with the waistcoat)
    let at = |x: f32, y: f32| pos2(hx + x * k * BODY_W, neck_y + y * k);
    let shoulder = at(312.0, 100.0);
    // at rest the paw is in his pocket at the waist: it is behind the coat, the elbow turned a little out
    let paw_rest = at(185.0, 462.0);
    let pole = at(380.0, 320.0);
    // the paw travels, and the arm is worked out from where it is, so it is never longer or shorter
    let (_, reach) = bend_arm(shoulder, raised_paw, k, pole);
    let u = up.clamp(0.0, 1.0);
    let wanted = paw_rest + (reach - paw_rest) * u;
    let (elbow, paw) = bend_arm(shoulder, wanted, k, pole);
    finish_arm(shoulder, elbow, paw, k)
}

/// His other arm (ours, the left), which rests a paw on the knob of his cane at the hip.
fn cane_arm(hx: f32, neck_y: f32, k: f32) -> Arm {
    let at = |x: f32, y: f32| pos2(hx + x * k * BODY_W, neck_y + y * k);
    let shoulder = at(-312.0, 100.0);
    let (elbow, paw) = bend_arm(shoulder, at(-332.0, 470.0), k, at(-392.0, 320.0));
    finish_arm(shoulder, elbow, paw, k)
}

/// A sleeve from the shoulder through the elbow to the wrist, with a darker edge.
fn paint_sleeve(painter: &egui::Painter, pts: &[Pos2; 3], k: f32) {
    for (w, c) in [(54.0f32, WINE_D), (45.0, WINE)] {
        painter.add(Shape::line(pts.to_vec(), Stroke::new((w * k).max(1.0), c)));
        for p in pts {
            painter.circle_filled(*p, (w * 0.5 * k).max(0.5), c);
        }
    }
}

/// The mouth of the side pocket on his coat, slanted, with a gold welt; `c` is where the paw goes in
/// (the arm that goes into it is drawn behind the coat, so the coat hides the hand).
fn paint_pocket(painter: &egui::Painter, c: Pos2, k: f32) {
    let top = c + vec2(30.0 * k, -4.0 * k);
    let bottom = c + vec2(-18.0 * k, 26.0 * k);
    let capsule = |width: f32, colour: Color32| {
        painter.add(Shape::line(vec![top, bottom], Stroke::new((width * k).max(1.0), colour)));
        painter.circle_filled(top, (width * 0.5 * k).max(0.5), colour);
        painter.circle_filled(bottom, (width * 0.5 * k).max(0.5), colour);
    };
    capsule(17.0, GOLD_D);
    capsule(12.0, Color32::from_rgb(8, 6, 10));
}

/// The gold cuff across the wrist.
fn paint_cuff(painter: &egui::Painter, elbow: Pos2, wrist: Pos2, k: f32) {
    let dir = (elbow - wrist).normalized();
    let side = vec2(-dir.y, dir.x) * 26.0 * k;
    painter
        .add(Shape::line(vec![wrist - side, wrist + side], Stroke::new((12.0 * k).max(1.0), GOLD)));
    painter.add(Shape::line(
        vec![wrist - side, wrist + side],
        Stroke::new((3.0 * k).max(1.0), GOLD_D),
    ));
}

/// A paw. The toes show on the lower edge while it hangs and on the upper edge when it grips.
fn paint_paw(painter: &egui::Painter, c: Pos2, k: f32, grips: bool) {
    painter.add(Shape::ellipse_filled(c, vec2(26.0 * k, 29.0 * k), rgb(BRONZE)));
    painter.add(Shape::ellipse_stroke(
        c,
        vec2(26.0 * k, 29.0 * k),
        Stroke::new((2.0 * k).max(1.0), OUT),
    ));
    let sign = if grips { -1.0 } else { 1.0 };
    for dx in [-16.0f32, 0.0, 16.0] {
        let x = dx * 0.6 * k;
        painter.add(Shape::line(
            vec![c + vec2(x, sign * 16.0 * k), c + vec2(x, sign * 30.0 * k)],
            Stroke::new((2.0 * k).max(1.0), rgb(BRONZE_D)),
        ));
    }
}

/// The frock coat, waistcoat, cravat and brooch, the epaulettes, the watch chain, the cane. `arm_up`
/// is how far his right arm has come up for the cough; once it has, `paint_cough_arm` draws it.
fn paint_body(painter: &egui::Painter, hx: f32, neck_y: f32, k: f32, arm_up: f32) {
    let pt = |x: f32, y: f32| body_pt(hx, neck_y, k, x, y);
    let ps = |v: &[(f32, f32)]| -> Vec<Pos2> { v.iter().map(|&(x, y)| pt(x, y)).collect() };
    let line = |v: &[(f32, f32)], c: Color32, wd: f32| {
        painter.add(Shape::line(ps(v), Stroke::new((wd * k).max(1.0), c)));
    };
    let dot = |x: f32, y: f32, r: f32, c: Color32| {
        painter.circle_filled(pt(x, y), (r * k).max(1.0), c);
    };
    let edge_gold = Some((GOLD_D, (1.5 * k).max(1.0)));
    // below the waist (seen when he is standing in the market): trousers, boots and a belt
    let pants = Color32::from_rgb(34, 30, 44);
    let pants_l = Color32::from_rgb(54, 48, 66);
    let leather = Color32::from_rgb(46, 30, 24);
    let boot = Color32::from_rgb(26, 20, 24);
    let boot_l = Color32::from_rgb(92, 72, 76);
    poly(
        painter,
        &ps(&[(-200.0, 388.0), (200.0, 388.0), (176.0, 520.0), (-176.0, 520.0)]),
        pants,
        None,
    );
    for sx in [-1.0f32, 1.0] {
        // a long slim trouser leg, with its crease
        poly(
            painter,
            &ps(&[(sx * 6.0, 500.0), (sx * 172.0, 500.0), (sx * 150.0, 670.0), (sx * 54.0, 670.0)]),
            pants,
            None,
        );
        line(&[(sx * 104.0, 440.0), (sx * 102.0, 668.0)], pants_l, 2.0);
        // the boot: a shaft with a polished cuff, a strap and a gold band and buckle, and a foot with the toe turned out
        poly(
            painter,
            &ps(&[
                (sx * 52.0, 640.0),
                (sx * 152.0, 640.0),
                (sx * 154.0, 712.0),
                (sx * 52.0, 712.0),
            ]),
            boot,
            None,
        );
        poly(
            painter,
            &ps(&[
                (sx * 48.0, 704.0),
                (sx * 154.0, 704.0),
                (sx * 214.0, 730.0),
                (sx * 222.0, 746.0),
                (sx * 48.0, 746.0),
            ]),
            boot,
            None,
        );
        line(&[(sx * 52.0, 642.0), (sx * 152.0, 642.0)], boot_l, 5.0);
        line(&[(sx * 78.0, 652.0), (sx * 78.0, 700.0)], boot_l, 3.0);
        line(&[(sx * 52.0, 676.0), (sx * 154.0, 676.0)], GOLD_D, 9.0);
        line(&[(sx * 52.0, 693.0), (sx * 154.0, 693.0)], GOLD_D, 5.0);
        dot(sx * 103.0, 676.0, 8.0, GOLD);
        line(&[(sx * 48.0, 748.0), (sx * 224.0, 748.0)], OUT, 8.0);
    }
    // the belt, with a gold buckle
    poly(
        painter,
        &ps(&[(-236.0, 388.0), (236.0, 388.0), (236.0, 428.0), (-236.0, 428.0)]),
        leather,
        Some((GOLD_D, (1.5 * k).max(1.0))),
    );
    poly(
        painter,
        &ps(&[(-36.0, 384.0), (36.0, 384.0), (36.0, 432.0), (-36.0, 432.0)]),
        GOLD,
        Some((GOLD_D, (1.5 * k).max(1.0))),
    );
    poly(
        painter,
        &ps(&[(-18.0, 396.0), (18.0, 396.0), (18.0, 420.0), (-18.0, 420.0)]),
        leather,
        None,
    );
    // the stand-up collar behind
    poly(
        painter,
        &ps(&[(-110.0, -20.0), (-200.0, -120.0), (-262.0, -80.0), (-236.0, 56.0)]),
        RED_TRIM,
        None,
    );
    poly(
        painter,
        &ps(&[(110.0, -20.0), (200.0, -120.0), (262.0, -80.0), (236.0, 56.0)]),
        RED_TRIM,
        None,
    );
    // the sleeves, behind the coat: the coat is the front of him and goes over them
    let right = right_arm(hx, neck_y, k, 0.0, pos2(hx, neck_y));
    let left = cane_arm(hx, neck_y, k);
    paint_sleeve(painter, &[left.shoulder, left.elbow, left.wrist], k);
    if arm_up <= 0.02 {
        // his other hand is in his pocket: the arm goes in behind the coat, which hides the paw
        paint_sleeve(painter, &[right.shoulder, right.elbow, right.wrist], k);
        paint_sleeve(painter, &[right.elbow, right.wrist, right.paw], k);
    }
    // the coat, cut close
    poly(
        painter,
        &ps(&[
            (-150.0, 0.0),
            (150.0, 0.0),
            (300.0, 50.0),
            (310.0, 130.0),
            (280.0, 230.0),
            (244.0, 300.0),
            (230.0, 392.0),
            (-230.0, 392.0),
            (-244.0, 300.0),
            (-280.0, 230.0),
            (-310.0, 130.0),
            (-300.0, 50.0),
        ]),
        WINE,
        None,
    );
    line(&[(-230.0, 390.0), (230.0, 390.0)], GOLD, 4.0);
    // the long skirts of the frock coat, open at the front so the trousers show between them
    for sx in [-1.0f32, 1.0] {
        poly(
            painter,
            &ps(&[
                (sx * 58.0, 388.0),
                (sx * 232.0, 388.0),
                (sx * 244.0, 480.0),
                (sx * 262.0, 585.0),
                (sx * 104.0, 592.0),
                (sx * 80.0, 480.0),
            ]),
            WINE,
            None,
        );
        poly(
            painter,
            &ps(&[(sx * 58.0, 388.0), (sx * 76.0, 388.0), (sx * 110.0, 592.0), (sx * 98.0, 592.0)]),
            RED_TRIM,
            None,
        );
        line(&[(sx * 168.0, 400.0), (sx * 196.0, 586.0)], WINE_D, 3.0);
        line(
            &[
                (sx * 232.0, 390.0),
                (sx * 244.0, 480.0),
                (sx * 262.0, 585.0),
                (sx * 104.0, 592.0),
                (sx * 80.0, 480.0),
                (sx * 58.0, 390.0),
            ],
            GOLD,
            3.0,
        );
    }
    // shirt and waistcoat (in the colour the UI has chosen, darkened), then the white V
    poly(
        painter,
        &ps(&[(-130.0, 8.0), (130.0, 8.0), (96.0, 388.0), (-96.0, 388.0)]),
        crate::theme::mix(crate::theme::accent(), Color32::BLACK, 0.5),
        None,
    );
    poly(
        painter,
        &ps(&[(-92.0, 6.0), (92.0, 6.0), (0.0, 196.0)]),
        Color32::from_rgb(240, 236, 226),
        None,
    );
    // the cravat and its ruby brooch
    poly(
        painter,
        &ps(&[
            (-84.0, -6.0),
            (84.0, -6.0),
            (66.0, 50.0),
            (24.0, 120.0),
            (0.0, 96.0),
            (-24.0, 120.0),
            (-66.0, 50.0),
        ]),
        Color32::from_rgb(250, 248, 240),
        None,
    );
    for dx in [-30.0f32, -8.0, 14.0, 36.0] {
        line(&[(dx, 0.0), (dx * 0.7, 80.0)], Color32::from_rgb(214, 208, 196), 2.0);
    }
    dot(0.0, 60.0, 11.0, GOLD);
    dot(0.0, 60.0, 6.0, Color32::from_rgb(210, 30, 60));
    // the velvet lapels, edged in gold
    poly(
        painter,
        &ps(&[
            (-130.0, 4.0),
            (-92.0, 6.0),
            (0.0, 200.0),
            (-60.0, 380.0),
            (-170.0, 380.0),
            (-186.0, 230.0),
        ]),
        WINE_D,
        None,
    );
    poly(
        painter,
        &ps(&[
            (130.0, 4.0),
            (92.0, 6.0),
            (0.0, 200.0),
            (60.0, 380.0),
            (170.0, 380.0),
            (186.0, 230.0),
        ]),
        WINE_D,
        None,
    );
    for sx in [-1.0f32, 1.0] {
        line(&[(sx * 130.0, 4.0), (sx * 186.0, 230.0), (sx * 170.0, 380.0)], GOLD, 4.0);
        line(&[(sx * 92.0, 6.0), (0.0, 200.0)], GOLD, 3.0);
    }
    // waistcoat buttons, the chain across, the watch in its pocket
    for y in [220.0f32, 262.0, 304.0, 346.0] {
        dot(0.0, y, 6.0, GOLD);
    }
    line(&[(-84.0, 252.0), (-40.0, 276.0), (0.0, 284.0), (40.0, 276.0), (84.0, 252.0)], GOLD, 3.0);
    dot(-92.0, 262.0, 15.0, GOLD);
    dot(-92.0, 262.0, 10.0, Color32::from_rgb(250, 238, 204));
    line(&[(-92.0, 262.0), (-92.0, 255.0)], Color32::from_rgb(60, 40, 20), 2.0);
    line(&[(-92.0, 262.0), (-87.0, 265.0)], Color32::from_rgb(60, 40, 20), 2.0);
    // the little white M pinned to his lapel
    dot(-150.0, 112.0, 14.0, Color32::from_rgb(244, 240, 232));
    line(
        &[(-160.0, 120.0), (-160.0, 104.0), (-150.0, 114.0), (-140.0, 104.0), (-140.0, 120.0)],
        Color32::from_rgb(40, 30, 36),
        3.0,
    );
    // the pocket square
    poly(
        painter,
        &ps(&[(-260.0, 156.0), (-246.0, 128.0), (-232.0, 154.0), (-218.0, 126.0), (-202.0, 156.0)]),
        RED_TRIM,
        None,
    );
    // the hands, in front of the coat: the cane in his left paw (ours, the left); the other arm hangs by his side
    let feet = pt(0.0, 748.0).y;
    let knob = left.paw + vec2(0.0, -46.0 * k);
    painter.add(Shape::line(
        vec![knob, pos2(left.paw.x, feet)],
        Stroke::new((12.0 * k).max(1.0), Color32::from_rgb(24, 20, 26)),
    ));
    painter.circle_filled(knob, (21.0 * k).max(1.0), GOLD);
    painter.circle_stroke(knob, (21.0 * k).max(2.0), Stroke::new((2.0 * k).max(1.0), GOLD_D));
    paint_cuff(painter, left.elbow, left.wrist, k);
    paint_paw(painter, left.paw, k, false);
    paint_pocket(painter, right.paw, k);
    if arm_up <= 0.02 {
        paint_cuff(painter, right.elbow, right.wrist, k);
    }
    // the gold along the edge of the coat, over the sleeves, and the epaulettes
    for sx in [-1.0f32, 1.0] {
        line(
            &[
                (sx * 300.0, 50.0),
                (sx * 310.0, 130.0),
                (sx * 280.0, 230.0),
                (sx * 244.0, 300.0),
                (sx * 230.0, 390.0),
            ],
            GOLD,
            4.0,
        );
        poly(
            painter,
            &ps(&[
                (sx * 296.0 - 46.0, 56.0),
                (sx * 296.0 + 46.0, 56.0),
                (sx * 296.0 + 62.0, 84.0),
                (sx * 296.0 - 62.0, 84.0),
            ]),
            GOLD,
            edge_gold,
        );
        for i in -3..=3 {
            line(
                &[
                    (sx * 296.0 + i as f32 * 16.0, 84.0),
                    (sx * 296.0 + i as f32 * 16.0 + 2.0, 102.0),
                ],
                GOLD,
                3.0,
            );
        }
        line(&[(sx * 296.0, 56.0), (sx * 150.0, 0.0)], WINE_L, 4.0);
    }
}

// ------------------------------------------------------------------------------- his head
/// Where the age is: a path down his face (screen points), and the shading of the head.
struct Age {
    path: Vec<(f32, f32)>,
    s: f32,
    top: f32,
    height: f32,
}

impl Age {
    /// The fur colour at `p` for the tone `base`: shaded from light at the top to dark at the chin, and drained of
    /// colour along the spread, with a frost-white heart.
    fn colour(&self, base: [f32; 3], p: Pos2) -> Color32 {
        let shade = (1.18 - 0.40 * ((p.y - self.top) / self.height.max(1.0))).clamp(0.78, 1.18);
        let mut c = [base[0] * shade, base[1] * shade, base[2] * shade];
        let (d, reach) = cat_talk::nearest_on_path(&self.path, (p.x, p.y));
        let ripple = ((p.x * 0.045).sin() + (p.y * 0.06 + p.x * 0.02).sin()) * 0.25;
        let d = d / self.s.max(0.001);
        let amount = cat_talk::age_amount(d, reach, ripple);
        if amount > 0.0 {
            c = lerp3(
                c,
                [
                    ASH[0] * (0.92 + 0.14 * (ripple + 0.5)),
                    ASH[1] * (0.92 + 0.14 * (ripple + 0.5)),
                    ASH[2] * 0.97,
                ],
                amount,
            );
            c = lerp3(c, [236.0, 236.0, 236.0], cat_talk::age_core(d, reach, ripple));
        }
        rgb(c)
    }
}

/// Fills a polygon with the fur colour, a little finer than one flat colour: each wedge of the fan is cut into a grid
/// of smaller triangles, coloured where they are, so the shading and the age run smoothly through it.
fn fill_fur(painter: &egui::Painter, pts: &[Pos2], base: [f32; 3], age: &Age) {
    let n = pts.len();
    if n < 3 {
        return;
    }
    const N: usize = 5;
    let c = centre(pts);
    let mut mesh = egui::Mesh::default();
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let start = mesh.vertices.len() as u32;
        for r in 0..=N {
            for q in 0..=r {
                let fr = r as f32 / N as f32;
                let fq = if r == 0 { 0.0 } else { q as f32 / r as f32 };
                let p = c + (a - c) * fr + (b - a) * (fr * fq);
                mesh.colored_vertex(p, age.colour(base, p));
            }
        }
        let idx = |r: usize, q: usize| start + (r * (r + 1) / 2 + q) as u32;
        for r in 0..N {
            for q in 0..=r {
                mesh.add_triangle(idx(r, q), idx(r + 1, q), idx(r + 1, q + 1));
                if q < r {
                    mesh.add_triangle(idx(r, q), idx(r + 1, q + 1), idx(r, q + 1));
                }
            }
        }
    }
    painter.add(Shape::mesh(mesh));
}

fn centroid_x(pts: &[Pos2]) -> f32 {
    pts.iter().map(|p| p.x).sum::<f32>() / pts.len().max(1) as f32
}

#[allow(clippy::too_many_arguments)]
fn paint_head(
    painter: &egui::Painter,
    polys: &[(Vec<Pos2>, king::Poly)],
    age: &Age,
    hx: f32,
    s: f32,
    k: f32,
    head_at: &dyn Fn((f32, f32)) -> Pos2,
    pose: &king::Pose,
    t: f32,
    animated: bool,
) {
    let line_w = |poly: &king::Poly| (poly.line.1 * s * 1.5).max(1.0);
    for (pts, poly) in polys {
        if pts.len() < 2 {
            continue;
        }
        let n = pts.len();
        match poly.fill {
            king::Tone::Fur => {
                // ears and the upper head are the lighter ash-black, the muzzle and jaw bronze
                let base = if n == 20 { BRONZE } else { BLACK };
                fill_fur(painter, pts, base, age);
                painter.add(Shape::closed_line(pts.clone(), Stroke::new(line_w(poly), OUT)));
            }
            king::Tone::Mane => {
                fill_fur(painter, pts, if n == 5 { BRONZE_D } else { BRONZE }, age);
                painter.add(Shape::closed_line(pts.clone(), Stroke::new(line_w(poly), OUT)));
            }
            king::Tone::Mouth => {
                fill_fan(painter, pts, Color32::from_rgb(58, 14, 24), vec2(0.0, 0.0));
                painter.add(Shape::closed_line(pts.clone(), Stroke::new(line_w(poly), OUT)));
            }
            king::Tone::Gem => {
                // the nose is the one on the middle of the face; the rest are fangs and the lower teeth that show when the
                // mouth is open (the one on the aged side is done with the face). The nose is not told from them by its
                // place in the list: with the mouth open the teeth come first, and the nose used to vanish when he talked.
                let is_nose = (centroid_x(pts) - hx).abs() < 50.0 * s;
                let aged_fang = !is_nose && centroid_x(pts) > hx;
                if !aged_fang {
                    fill_fan(painter, pts, if is_nose { NOSE } else { FANG }, vec2(0.0, 0.0));
                    painter.add(Shape::closed_line(pts.clone(), Stroke::new(line_w(poly), OUT)));
                }
            }
            king::Tone::None => {
                let lw = poly.line.1;
                if (lw - 2.4).abs() < 0.01 {
                    painter.add(Shape::line(pts.clone(), Stroke::new(line_w(poly), rgb(BRONZE_L))));
                } else if (lw - 2.6).abs() < 0.01 {
                    painter.add(Shape::line(
                        pts.clone(),
                        Stroke::new(line_w(poly) * 1.1, rgb(BRONZE_L)),
                    ));
                } else if lw < 2.0 {
                    painter.add(Shape::line(
                        pts.clone(),
                        Stroke::new(line_w(poly), Color32::from_rgb(34, 26, 24)),
                    ));
                }
            }
            _ => {}
        }
    }
    paint_face(painter, polys, age, hx, s, k, head_at, pose, t, animated);
}

fn bbox(pts: &[Pos2]) -> Rect {
    let mut r = Rect::from_min_max(pts[0], pts[0]);
    for p in pts {
        r.extend_with(*p);
    }
    r
}

#[allow(clippy::too_many_arguments)]
fn paint_face(
    painter: &egui::Painter,
    polys: &[(Vec<Pos2>, king::Poly)],
    age: &Age,
    hx: f32,
    s: f32,
    k: f32,
    head_at: &dyn Fn((f32, f32)) -> Pos2,
    pose: &king::Pose,
    t: f32,
    animated: bool,
) {
    let px = |v: f32| (v * k).max(1.0);
    // the eyes: the shapes when open, the lid lines when shut
    let eyes: Vec<&Vec<Pos2>> = polys
        .iter()
        .filter(|(p, q)| q.fill == king::Tone::Eye && p.len() >= 3)
        .map(|(p, _)| p)
        .collect();
    let lids: Vec<&Vec<Pos2>> = polys
        .iter()
        .filter(|(p, q)| q.fill == king::Tone::None && q.line.1 > 3.0 && p.len() >= 2)
        .map(|(p, _)| p)
        .collect();
    let shut = eyes.is_empty();
    let mut shapes: Vec<&Vec<Pos2>> = if shut { lids.clone() } else { eyes };
    shapes.sort_by(|a, b| centroid_x(a).total_cmp(&centroid_x(b)));
    if shapes.len() < 2 {
        return;
    }
    let (good, aged) = (shapes[0], shapes[shapes.len() - 1]);

    // the good eye: bright, with a slit pupil, glowing
    let gb = bbox(good);
    let gc = gb.center();
    if !shut {
        soft_ellipse(
            painter,
            gc,
            vec2(gb.width() * 1.3, gb.width() * 1.3),
            [255, 190, 70],
            0.35 * pose.glow,
        );
        fill_fan(painter, good, Color32::from_rgb(255, 196, 70), vec2(0.0, 0.0));
        painter.add(Shape::ellipse_filled(
            gc,
            vec2(gb.width() * 0.3, gb.height() * 0.35),
            Color32::from_rgb(255, 232, 140),
        ));
        painter.add(Shape::convex_polygon(
            vec![
                gc + vec2(0.0, -gb.height() * 0.62),
                gc + vec2(gb.width() * 0.07, 0.0),
                gc + vec2(0.0, gb.height() * 0.62),
                gc + vec2(-gb.width() * 0.07, 0.0),
            ],
            Color32::from_rgb(22, 12, 10),
            Stroke::NONE,
        ));
        painter.add(Shape::closed_line(
            good.clone(),
            Stroke::new(px(2.4), Color32::from_rgb(24, 16, 12)),
        ));
    } else {
        painter.add(Shape::line(good.clone(), Stroke::new(px(5.0), Color32::from_rgb(18, 12, 12))));
        painter.add(Shape::line(
            good.iter().take(3).map(|p| *p + vec2(0.0, px(8.0))).collect(),
            Stroke::new(px(2.0), Color32::from_rgb(80, 56, 40)),
        ));
    }
    // heavier lid lines
    if !shut {
        for l in &lids {
            painter.add(Shape::line(
                (*l).clone(),
                Stroke::new(px(5.0), Color32::from_rgb(18, 12, 12)),
            ));
        }
    }
    // the mouth, closed: composed (or set, after he has been told yes)
    if pose.jaw < 0.03 {
        let (path, wd): (&[(f32, f32)], f32) = if pose.glow > 0.7 {
            (&[(-176.0, 210.0), (-90.0, 194.0), (40.0, 190.0), (140.0, 196.0), (206.0, 218.0)], 5.0)
        } else {
            (&[(-170.0, 196.0), (-60.0, 190.0), (40.0, 188.0), (130.0, 194.0), (200.0, 208.0)], 4.5)
        };
        painter.add(Shape::line(
            path.iter().map(|&q| head_at(q)).collect(),
            Stroke::new(px(wd), Color32::from_rgb(24, 14, 12)),
        ));
    }
    // the fang on the aged side: yellowed, greying at the root, cracked, with a seam of gold
    if let Some((pts, _)) = polys
        .iter()
        .filter(|(p, q)| q.fill == king::Tone::Gem && p.len() == 3)
        .filter(|(p, _)| centroid_x(p) > hx)
        .last()
    {
        let tip = *pts.iter().max_by(|a, b| a.y.total_cmp(&b.y)).unwrap_or(&pts[0]);
        let base: Vec<Pos2> = pts.iter().copied().filter(|p| *p != tip).collect();
        if base.len() == 2 {
            painter.add(Shape::convex_polygon(
                pts.clone(),
                Color32::from_rgb(204, 188, 150),
                Stroke::NONE,
            ));
            let m1 = base[0].lerp(tip, 0.5);
            let m2 = base[1].lerp(tip, 0.5);
            painter.add(Shape::convex_polygon(
                vec![base[0], base[1], m2, m1],
                Color32::from_rgb(168, 160, 142),
                Stroke::NONE,
            ));
            painter.add(Shape::closed_line(
                pts.clone(),
                Stroke::new(px(2.0), Color32::from_rgb(60, 50, 40)),
            ));
            let crack = Color32::from_rgb(92, 78, 60);
            painter.line_segment(
                [base[0] + vec2(px(2.0), 0.0), m1 + vec2(0.0, px(14.0))],
                Stroke::new(px(1.6), crack),
            );
            painter.line_segment(
                [base[1] - vec2(px(2.0), 0.0), m2 + vec2(0.0, px(10.0))],
                Stroke::new(px(1.4), crack),
            );
            painter.line_segment(
                [m1 + vec2(px(3.0), px(6.0)), tip - vec2(px(1.0), px(12.0))],
                Stroke::new(px(1.4), crack),
            );
            painter.line_segment(
                [base[1] - vec2(px(4.0), -px(6.0)), m2 + vec2(px(1.0), px(4.0))],
                Stroke::new(px(1.6), Color32::from_rgb(255, 206, 100)),
            );
        }
    }
    // the age: furrows, crow's feet, a hollow under the cheek, grey whiskers, thin cracks of gold light
    let ab = bbox(aged);
    let (ecx, ecy) = (ab.center().x, ab.center().y);
    let furrow = Color32::from_rgb(86, 78, 78);
    for (dx, dy) in [(70.0f32, -46.0f32), (82.0, -6.0), (66.0, 34.0)] {
        painter.line_segment(
            [
                pos2(ecx + ab.width() * 0.55, ecy + px(4.0)),
                pos2(ecx + ab.width() * 0.55 + dx * k, ecy + dy * k),
            ],
            Stroke::new(px(2.4), furrow),
        );
    }
    painter.add(Shape::line(
        vec![head_at((150.0, 70.0)), head_at((205.0, 130.0)), head_at((212.0, 190.0))],
        Stroke::new(px(3.0), furrow),
    ));
    painter.add(Shape::line(
        vec![head_at((110.0, 40.0)), head_at((176.0, 92.0)), head_at((186.0, 150.0))],
        Stroke::new(px(2.0), Color32::from_rgb(96, 88, 88)),
    ));
    soft_ellipse(painter, head_at((260.0, 90.0)), vec2(120.0 * k, 64.0 * k), [30, 26, 36], 0.32);
    // grey whiskers on the aged side
    for i in 0..22u32 {
        let (sx, sy) = cat_talk::SPREAD[(hash(i + 900) * 7.0) as usize % cat_talk::SPREAD.len()];
        let b = head_at((sx + (hash(i + 910) - 0.5) * 80.0, sy + (hash(i + 920) - 0.5) * 60.0));
        painter.line_segment(
            [b, b + vec2(px(6.0 + 8.0 * hash(i + 930)), px(-4.0 + 12.0 * hash(i + 940)))],
            Stroke::new(px(1.4), Color32::from_rgb(222, 218, 214)),
        );
    }
    // light seeping out along the spread
    let path = cat_talk::spread_path(3);
    for strand in 0..5u32 {
        let mut pts = Vec::new();
        for (idx, q) in
            path.iter().enumerate().skip(2).step_by(3).take(path.len().saturating_sub(8) / 3)
        {
            let fade = 1.0 - idx as f32 / path.len() as f32;
            let ox = (hash(strand * 50 + idx as u32) - 0.5) * 68.0 * fade;
            let oy = (hash(strand * 50 + idx as u32 + 7) - 0.5) * 60.0 * fade;
            pts.push(head_at((q.0 + ox, q.1 + oy)));
        }
        if pts.len() > 2 {
            let flicker = if animated { 0.7 + 0.3 * (t * 2.0 + strand as f32).sin() } else { 1.0 };
            painter.add(Shape::line(
                pts.clone(),
                Stroke::new(px(4.0), alpha([255, 196, 90], 0.12 * flicker)),
            ));
            painter
                .add(Shape::line(pts, Stroke::new(px(1.5), alpha([255, 210, 120], 0.8 * flicker))));
        }
    }
    // the gold ring in the good side's ear
    if let Some((ear, _)) = polys
        .iter()
        .filter(|(p, q)| q.fill == king::Tone::Fur && p.len() == 5)
        .min_by(|a, b| centroid_x(&a.0).total_cmp(&centroid_x(&b.0)))
    {
        let tip = *ear.iter().min_by(|a, b| a.y.total_cmp(&b.y)).unwrap_or(&ear[0]);
        let out = *ear.iter().min_by(|a, b| a.x.total_cmp(&b.x)).unwrap_or(&ear[0]);
        let c = tip.lerp(out, 0.45);
        painter.add(Shape::ellipse_stroke(
            c + vec2(0.0, px(8.0)),
            vec2(px(8.0), px(10.0)),
            Stroke::new(px(3.0), GOLD),
        ));
    }
    // whiskers
    for (sx, col) in
        [(1.0f32, Color32::from_rgb(222, 218, 214)), (-1.0, Color32::from_rgb(236, 226, 206))]
    {
        let ruff = polys
            .iter()
            .filter(|(p, q)| q.fill == king::Tone::Mane && p.len() == 7)
            .find(|(p, _)| (centroid_x(p) > hx) == (sx > 0.0));
        if let Some((p, _)) = ruff {
            let base = p[0];
            for (i, (dx, dy)) in
                [(74.0f32, -12.0f32), (86.0, 8.0), (70.0, 28.0)].into_iter().enumerate()
            {
                let start = pos2(base.x - sx * px(18.0), base.y + px(14.0 + i as f32 * 6.0));
                painter.add(Shape::line(
                    vec![
                        start,
                        start + vec2(sx * dx * k, dy * 0.6 * k),
                        start + vec2(sx * dx * 1.7 * k, dy * k),
                    ],
                    Stroke::new(px(1.6), col),
                ));
            }
        }
    }
    paint_monocle(painter, aged, shut, k, pos2(ecx, ecy), ab, hx_chain(hx, polys, k));
}

/// Where the monocle's chain ends: the waistcoat pocket on his right (ours, the right). It hangs from the lens clasp.
fn hx_chain(hx: f32, polys: &[(Vec<Pos2>, king::Poly)], k: f32) -> Pos2 {
    // the lowest point of the jaw tells how long his head is; the pocket is a fixed distance below the middle of the head
    let jaw_bottom = polys
        .iter()
        .filter(|(p, _)| p.len() == 20)
        .flat_map(|(p, _)| p.iter().map(|q| q.y))
        .fold(f32::MIN, f32::max);
    pos2(hx + 90.0 * k * BODY_W, jaw_bottom + 150.0 * k)
}

/// The monocle on the aged eye: the lens makes the eye bigger and shows what is wrong with it.
fn paint_monocle(
    painter: &egui::Painter,
    eye: &[Pos2],
    shut: bool,
    k: f32,
    c: Pos2,
    eb: Rect,
    chain_to: Pos2,
) {
    let px = |v: f32| (v * k).max(1.0);
    let r = (eb.width() * 0.78).max(46.0 * k);
    // the white of the aged eye, yellowed
    painter.circle_filled(c, r, Color32::from_rgb(236, 226, 196));
    if !shut {
        let mag = (1.7 * r / eb.width().max(1.0)).min(1.9);
        let big: Vec<Pos2> = eye.iter().map(|p| c + (*p - c) * mag).collect();
        fill_fan(painter, &big, Color32::from_rgb(228, 208, 150), vec2(0.0, 0.0));
        let bb = bbox(&big);
        let (ix, iy) = (c.x, c.y + bb.height() * 0.02);
        let ir = (bb.width() * 0.30).min(bb.height() * 0.9);
        let ip = pos2(ix, iy);
        let ell = |scale: f32, col: Color32| {
            painter.add(Shape::ellipse_filled(ip, vec2(ir * scale, ir * 0.95 * scale), col))
        };
        // the iris: a gold ring that is going out, dull gold on one side and ash on the other
        ell(1.0, Color32::from_rgb(176, 142, 66));
        let half: Vec<Pos2> = (0..=10)
            .map(|i| {
                let a = (20.0 + 180.0 * i as f32 / 10.0).to_radians();
                ip + vec2(a.cos() * ir, a.sin() * ir * 0.95)
            })
            .collect();
        fill_fan(painter, &half, Color32::from_rgb(158, 150, 142), vec2(0.0, 0.0));
        // the pupil is not black any more: grey, clouded over
        ell(0.56, Color32::from_rgb(166, 172, 182));
        ell(0.30, Color32::from_rgb(190, 196, 204));
        for i in 0..9 {
            let a = i as f32 * std::f32::consts::TAU / 9.0 + 0.3;
            painter.line_segment(
                [
                    ip + vec2(a.cos() * ir * 0.62, a.sin() * ir * 0.58),
                    ip + vec2(a.cos() * ir * 0.95, a.sin() * ir * 0.9),
                ],
                Stroke::new(px(1.4), Color32::from_rgb(110, 90, 50)),
            );
        }
        painter.add(Shape::ellipse_stroke(
            ip,
            vec2(ir * 0.52, ir * 0.5),
            Stroke::new(px(1.6), Color32::from_rgb(96, 98, 108)),
        ));
        painter.add(Shape::ellipse_filled(
            ip + vec2(-ir * 0.23, -ir * 0.35),
            vec2(ir * 0.11, ir * 0.11),
            Color32::from_rgba_unmultiplied(250, 250, 250, 220),
        ));
        painter
            .add(Shape::closed_line(big.clone(), Stroke::new(1.0, Color32::from_rgb(96, 70, 50))));
        // a heavy lid across the top, so the eye looks tired: a strip between the rim and a drooping edge
        let steps = 14;
        let lower = |u: f32| -> Pos2 {
            let pts = [(-1.0f32, -0.05f32), (-0.5, -0.38), (0.0, -0.5), (0.5, -0.38), (1.0, -0.05)];
            let f = (u * 4.0).min(3.999);
            let i = f as usize;
            let tt = f - i as f32;
            let (a, b) = (pts[i], pts[i + 1]);
            let x = a.0 + (b.0 - a.0) * tt;
            let y = a.1 + (b.1 - a.1) * tt;
            // x is in lens radii; y in lens radii for the ends and in half eye heights for the droop
            c + vec2(x * r, if y.abs() < 0.1 { y * r } else { y * bb.height() })
        };
        let mut mesh = egui::Mesh::default();
        let lid = Color32::from_rgb(150, 130, 112);
        for i in 0..=steps {
            let u = i as f32 / steps as f32;
            let a = std::f32::consts::PI + std::f32::consts::PI * u;
            mesh.colored_vertex(c + vec2(a.cos() * r, a.sin() * r), lid);
            mesh.colored_vertex(lower(u), lid);
        }
        for i in 0..steps as u32 {
            let b = i * 2;
            mesh.add_triangle(b, b + 1, b + 2);
            mesh.add_triangle(b + 1, b + 3, b + 2);
        }
        painter.add(Shape::mesh(mesh));
    } else {
        let arc: Vec<Pos2> = (0..=10)
            .map(|i| {
                let a = (200.0 + 140.0 * i as f32 / 10.0).to_radians();
                c + vec2(a.cos() * r * 0.7, a.sin() * r * 0.4 + r * 0.2)
            })
            .collect();
        painter.add(Shape::line(arc, Stroke::new(px(6.0), Color32::from_rgb(24, 16, 14))));
    }
    // glass: a faint sheen and a glint, then the gold frame, the bag under it, the clasp and the chain
    painter.circle_filled(c, r, Color32::from_rgba_unmultiplied(200, 230, 255, 26));
    let glint: Vec<Pos2> = (0..=6)
        .map(|i| {
            let a = (200.0 + 60.0 * i as f32 / 6.0).to_radians();
            c + vec2(a.cos(), a.sin()) * r * 0.82
        })
        .collect();
    painter.add(Shape::line(
        glint,
        Stroke::new(px(3.0), Color32::from_rgba_unmultiplied(255, 255, 255, 190)),
    ));
    painter.circle_stroke(c, r, Stroke::new(px(5.0), GOLD));
    painter.circle_stroke(c, r + px(2.0), Stroke::new(px(1.5), GOLD_D));
    let bag: Vec<Pos2> = (0..=10)
        .map(|i| {
            let a = (20.0 + 140.0 * i as f32 / 10.0).to_radians();
            pos2(c.x + a.cos() * r * 0.95, c.y + r * 1.1 + a.sin() * r * 0.4)
        })
        .collect();
    painter.add(Shape::line(bag, Stroke::new(px(3.0), Color32::from_rgb(96, 86, 88))));
    let clasp = c + vec2(r * 0.7, r * 0.7);
    painter.circle_filled(clasp, px(4.0), GOLD);
    let start = clasp + vec2(0.0, px(4.0));
    let links: Vec<Pos2> = (0..=20)
        .map(|i| {
            let u = i as f32 / 20.0;
            let p = start + (chain_to - start) * u;
            p + vec2(
                (u * std::f32::consts::PI).sin() * px(26.0),
                (u * std::f32::consts::PI).sin() * px(18.0),
            )
        })
        .collect();
    painter.add(Shape::line(links.clone(), Stroke::new(px(2.6), GOLD)));
    for (i, p) in links.iter().enumerate() {
        if i % 2 == 0 {
            painter.circle_filled(*p, px(2.4), Color32::from_rgb(255, 226, 130));
        }
    }
}

/// The cough: his arm comes up and a paw holds a handkerchief to his mouth, with a speck of red on
/// it. The cloth is simply there as soon as the paw is up (it pops into the paw), so nothing comes
/// from nowhere: the arm that hangs at his side is the one that does it.
fn paint_cough_arm(
    painter: &egui::Painter,
    hx: f32,
    hy: f32,
    neck_y: f32,
    s: f32,
    k: f32,
    up: f32,
) {
    let hp = |x: f32, y: f32| pos2(hx + x * s, hy + y * s);
    let target = hp(280.0, 410.0);
    let arm = right_arm(hx, neck_y, k, up, target);
    paint_sleeve(painter, &[arm.shoulder, arm.elbow, arm.wrist], k);
    paint_cuff(painter, arm.elbow, arm.wrist, k);
    let pop = ((up - 0.2) / 0.15).clamp(0.0, 1.0);
    if pop > 0.0 {
        let grow = 0.55 + 0.45 * pop;
        let at = |x: f32, y: f32| arm.paw + (hp(x, y) - target) * grow;
        let ps = |v: &[(f32, f32)]| -> Vec<Pos2> { v.iter().map(|&(x, y)| at(x, y)).collect() };
        let edge = (2.0 * k).max(1.0);
        let cloth = ps(&[
            (-100.0, 180.0),
            (220.0, 150.0),
            (300.0, 330.0),
            (180.0, 420.0),
            (-40.0, 380.0),
            (-130.0, 300.0),
        ]);
        poly(
            painter,
            &cloth,
            Color32::from_rgb(244, 240, 232),
            Some((Color32::from_rgb(150, 142, 132), edge)),
        );
        for (a, b) in [
            ((0.0f32, 200.0f32), (40.0f32, 360.0f32)),
            ((90.0, 180.0), (140.0, 400.0)),
            ((-60.0, 230.0), (-30.0, 370.0)),
        ] {
            painter.line_segment(
                [at(a.0, a.1), at(b.0, b.1)],
                Stroke::new(edge, Color32::from_rgb(206, 200, 190)),
            );
        }
        painter.add(Shape::ellipse_filled(
            at(110.0, 300.0),
            vec2(28.0 * s * grow, 20.0 * s * grow),
            Color32::from_rgb(150, 26, 40),
        ));
        painter.circle_filled(
            at(160.0, 340.0),
            (8.0 * s * grow).max(1.5),
            Color32::from_rgb(150, 26, 40),
        );
    }
    paint_paw(painter, arm.paw, k, up > 0.5);
}

// ------------------------------------------------------------------------------- as a backdrop
/// The cat as a backdrop: what changes between frames.
pub struct Backdrop {
    cough: Cough,
    opened: Option<f64>,
}

impl Backdrop {
    pub fn new(seed: u32) -> Backdrop {
        Backdrop { cough: Cough::new(seed), opened: None }
    }
}

/// Puts the street of Circuit City at the top of the page (on the Info and Changelog tabs), in the
/// logo's place, with the cat standing on the left of the market near the clock tower: small, in
/// the glow of the circuits and dimmed into the crowd so he does not stand out, watching you (his
/// head turns a little toward the cursor) and coughing now and then. Cats walk by in front of him, a
/// couple sleep in the alley. `animate` is false with the window minimized; `motion` is the Animate
/// backdrop switch (off: everything stands still). Returns (he was clicked, the alley was clicked): the
/// caller opens the talk, or the Ally of Darkness. The alley only answers when `alley_open` (you have
/// said yes to him), and then it flickers a little, the only invitation.
pub fn paint_cat_backdrop(
    ui: &mut egui::Ui,
    state: &mut Backdrop,
    animate: bool,
    motion: bool,
    alley_open: bool,
) -> (bool, bool) {
    let available = ui.available_width();
    if available < 160.0 {
        return (false, false);
    }
    let h = (available * 0.5).clamp(230.0, 330.0);
    let (stage, _) = ui.allocate_exact_size(vec2(available, h), egui::Sense::hover());
    let now = ui.input(|i| i.time);
    let t = (now - *state.opened.get_or_insert(now)) as f32;
    let live = animate && motion;
    // him: about as tall as the cats walking by
    let ground = stage.bottom() - h * 0.16;
    let head_ratio = 0.42f32;
    let k = h * 0.24 / 1050.0;
    let hx = stage.left() + available * CAT_X;
    let feet = ground + h * 0.05;
    let hy = feet - (380.0 * head_ratio + FEET_BELOW_NECK) * k;
    let hit =
        Rect::from_min_max(pos2(hx - 500.0 * k, hy - 170.0 * k), pos2(hx + 500.0 * k, feet + 4.0));
    let response = ui.interact(hit, ui.id().with("cat_backdrop"), egui::Sense::click());
    let hovered = response.hovered();
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    state.cough.update(t);
    let cp = if live { state.cough.pose(t) } else { CoughPose::default() };
    let gaze = if live {
        ui.input(|i| i.pointer.hover_pos())
            .map_or(0.0, |p| ((p.x - hx) / available).clamp(-0.5, 0.5) * 10.0)
    } else {
        0.0
    };
    let painter = ui.painter_at(stage);
    let tt = if live { t } else { 0.0 };
    let look = CatLook {
        agreed: false,
        talking: false,
        interested: hovered,
        gaze,
        head: head_ratio,
        cp,
        anger: 0.0,
    };
    // drawn between the alley and the cats walking by, so they cross in front of him
    let cat = || {
        // lit by the circuits round him, then dimmed into the street so that he blends in
        soft_ellipse(
            &painter,
            pos2(hx, hy + 440.0 * k),
            vec2(560.0 * k, 620.0 * k),
            [60, 220, 200],
            0.16,
        );
        paint_cat(&painter, hx, hy, k, &look, tt, live);
        soft_ellipse(
            &painter,
            pos2(hx, hy + 430.0 * k),
            vec2(700.0 * k, 760.0 * k),
            [3, 14, 18],
            0.34,
        );
        soft_ellipse(
            &painter,
            pos2(hx, hy + 430.0 * k),
            vec2(640.0 * k, 700.0 * k),
            [40, 200, 190],
            0.10,
        );
        if hovered {
            let pulse = if live { 0.6 + 0.4 * (t * 4.0).sin() } else { 1.0 };
            painter.add(Shape::ellipse_stroke(
                pos2(hx, hy + 440.0 * k),
                vec2(520.0 * k, 600.0 * k),
                Stroke::new(1.0, alpha([96, 220, 255], 0.3 * pulse)),
            ));
        }
    };
    paint_street(&painter, stage, tt, live, &cat);
    // the alley: once he has asked you for help it flickers, and it can be walked into
    let mut alley_clicked = false;
    if alley_open {
        let mouth = Rect::from_min_max(
            pos2(stage.left() + available * (ALLEY_AT - 0.075), stage.top() + h * 0.22),
            pos2(stage.left() + available * (ALLEY_AT + 0.095), ground),
        );
        let alley = ui.interact(mouth, ui.id().with("ally_mouth"), egui::Sense::click());
        if alley.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let flicker = if live { 0.5 + 0.5 * (tt * 3.1).sin() * (tt * 1.7 + 1.0).sin() } else { 0.5 };
        soft_ellipse(&painter, pos2(mouth.center().x, mouth.top() + mouth.height() * 0.25), vec2(mouth.width() * 0.5, mouth.height() * 0.3), [255, 196, 100], 0.04 + 0.08 * flicker.abs());
        alley_clicked = alley.clicked();
    }
    if live {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }
    (response.clicked(), alley_clicked)
}

/// Where the alley between the two stalls is across the page backdrop, as a fraction of its width.
const ALLEY_AT: f32 = 0.64;
/// Where the cat stands: on the left of the market, near the clock tower, among the crowd.
const CAT_X: f32 = 0.265;

/// Coats, and fur, for the cats out for a walk (`cat_talk::Walker` picks from them).
const COATS: [[u8; 3]; 8] = [
    [78, 104, 84],
    [64, 80, 128],
    [168, 128, 54],
    [58, 112, 118],
    [110, 58, 96],
    [132, 132, 140],
    [150, 78, 62],
    [70, 66, 90],
];
const FURS: [[u8; 3]; 5] =
    [[196, 150, 96], [120, 120, 130], [226, 214, 196], [84, 70, 66], [176, 112, 60]];

/// The hands of the clock at the computer's own time, in degrees clockwise from 12: (hour, minute, second).
fn clock_hand_angles() -> (f32, f32, f32) {
    let now = crate::sensor_log::now();
    crate::info::hand_angles(now.hour, now.minute, now.second)
}

/// A cat asleep, curled up, seen from the side, with a few z's drifting up. `size` is half its
/// length; `flip` is 1 when its head is on the right, -1 when it is on the left.
fn paint_sleeper(painter: &egui::Painter, base: Pos2, size: f32, fur_i: usize, flip: f32, t: f32) {
    let fur = tint(FURS[fur_i % FURS.len()], 0.78);
    let dark = tint(FURS[fur_i % FURS.len()], 0.55);
    let ink = Color32::from_rgb(12, 8, 18);
    let breathe = 1.0 + 0.05 * (t * 1.5 + fur_i as f32 * 2.0).sin();
    let p = |x: f32, y: f32| pos2(base.x + flip * x * size, base.y - y * size);
    painter.add(Shape::ellipse_filled(
        p(0.0, 0.0),
        vec2(1.3 * size, 0.28 * size),
        alpha([0, 0, 0], 0.4),
    ));
    // the tail curls round the front
    painter.add(Shape::line(
        vec![p(-0.95, 0.3), p(-0.8, 0.0), p(-0.2, -0.1), p(0.5, 0.0), p(0.85, 0.15)],
        Stroke::new(0.32 * size, dark),
    ));
    // the body, the head tucked in at one end
    painter.add(Shape::ellipse_filled(p(0.0, 0.5), vec2(size, 0.55 * size * breathe), fur));
    painter.add(Shape::ellipse_stroke(
        p(0.0, 0.5),
        vec2(size, 0.55 * size * breathe),
        Stroke::new(1.0, ink),
    ));
    let head = p(0.8, 0.4);
    for ex in [0.58f32, 0.95] {
        painter.add(Shape::convex_polygon(
            vec![p(ex - 0.12, 0.7), p(ex + 0.1, 0.72), p(ex, 1.02)],
            fur,
            Stroke::new(1.0, ink),
        ));
    }
    painter.circle_filled(head, 0.4 * size, fur);
    painter.circle_stroke(head, 0.4 * size, Stroke::new(1.0, ink));
    painter.line_segment([p(0.78, 0.45), p(0.98, 0.43)], Stroke::new(1.2, ink));
    // z z z
    for i in 0..3u32 {
        let ph = (t * 0.3 + i as f32 / 3.0 + fur_i as f32 * 0.37).fract();
        let at = p(0.95 + 0.5 * ph, 1.2 + 1.6 * ph);
        painter.text(
            at,
            egui::Align2::CENTER_CENTER,
            "z",
            egui::FontId::proportional(size * (0.45 + 0.35 * ph)),
            alpha([214, 224, 255], 0.8 * (1.0 - ph)),
        );
    }
}

/// A colour made darker (or lighter) by `k`.
fn tint(c: [u8; 3], k: f32) -> Color32 {
    let f = |v: u8| (v as f32 * k).clamp(0.0, 255.0) as u8;
    Color32::from_rgb(f(c[0]), f(c[1]), f(c[2]))
}

/// The face of the clock, showing the time on the computer like Father Time's: cream dial, gold
/// bezel, hour marks, hands (the second hand red).
fn paint_clock_face(painter: &egui::Painter, c: Pos2, r: f32, light: cat_talk::ClockLight) {
    painter.circle_filled(c, r * 1.16, Color32::from_rgb(58, 42, 22));
    painter.circle_stroke(c, r * 1.16, Stroke::new((r * 0.08).max(1.0), GOLD));
    painter.circle_filled(c, r, Color32::from_rgb(236, 226, 196));
    painter.circle_filled(c, r * 0.96, alpha([255, 236, 200], 0.4 * light.glow));
    painter.circle_stroke(c, r, Stroke::new((r * 0.1).max(1.5), GOLD));
    for m in 0..12 {
        let a = std::f32::consts::TAU * m as f32 / 12.0;
        let d = vec2(a.sin(), -a.cos());
        painter.line_segment(
            [c + d * r * 0.8, c + d * r * 0.94],
            Stroke::new(if m % 3 == 0 { 1.8 } else { 1.0 }, Color32::from_rgb(60, 40, 20)),
        );
    }
    let (hour_deg, minute_deg, second_deg) = clock_hand_angles();
    for (deg, len, wd, col) in [
        (hour_deg, 0.5f32, (r * 0.1).max(2.0), Color32::from_rgb(40, 24, 18)),
        (minute_deg, 0.78, (r * 0.06).max(1.4), Color32::from_rgb(40, 24, 18)),
        (second_deg, 0.86, 0.9, Color32::from_rgb(200, 40, 50)),
    ] {
        let a = deg.to_radians();
        painter.line_segment([c, c + vec2(a.sin(), -a.cos()) * r * len], Stroke::new(wd, col));
    }
    painter.circle_filled(c, r * 0.08, Color32::from_rgb(200, 40, 50));
}

/// The clock tower, made to stand out from the circuit board around it: dark stone with gold trim,
/// a lit clock, a belfry, a spire with a beacon, and at its foot a locked door whose keyhole glows
/// (it waits for a key). `tx` is its middle, `ground` its foot, `top` the tip of the spire and
/// `half` half the width of the shaft.
fn paint_clock_tower(
    painter: &egui::Painter,
    tx: f32,
    ground: f32,
    top: f32,
    half: f32,
    t: f32,
    animated: bool,
) {
    let hh = (ground - top).max(40.0);
    let y = |f: f32| top + hh * f;
    let light = if animated { cat_talk::clock_light(t) } else { cat_talk::ClockLight::default() };
    let stone = Color32::from_rgb(38, 30, 38);
    let stone_l = Color32::from_rgb(62, 50, 56);
    let amber = [255u8, 184, 84];
    let block = |l: f32, tp: f32, r: f32, b: f32, fill: Color32| {
        painter.rect_filled(Rect::from_min_max(pos2(l, tp), pos2(r, b)), 0.0, fill);
    };
    let band = |half_w: f32, at: f32, wd: f32| {
        painter.line_segment([pos2(tx - half_w, at), pos2(tx + half_w, at)], Stroke::new(wd, GOLD));
    };
    // a warm glow behind it, so it reads against the cool board
    soft_ellipse(
        painter,
        pos2(tx, y(0.42)),
        vec2(half * 3.6, hh * 0.52),
        amber,
        0.07 + 0.05 * light.glow,
    );
    // the plinth, two steps
    block(tx - half * 1.6, y(0.965), tx + half * 1.6, ground + 2.0, stone_l);
    block(tx - half * 1.35, y(0.93), tx + half * 1.35, y(0.965), stone);
    band(half * 1.6, y(0.965), 1.2);
    band(half * 1.35, y(0.93), 1.0);
    // the shaft
    block(tx - half, y(0.50), tx + half, y(0.93), stone);
    painter.line_segment(
        [pos2(tx - half, y(0.50)), pos2(tx - half, y(0.93))],
        Stroke::new(1.0, alpha([255, 200, 120], 0.35)),
    );
    painter.line_segment(
        [pos2(tx + half, y(0.50)), pos2(tx + half, y(0.93))],
        Stroke::new(1.0, alpha([255, 200, 120], 0.2)),
    );
    super::circuit_city::paint_vias(
        painter,
        Rect::from_min_max(pos2(tx - half, y(0.54)), pos2(tx + half, y(0.70))),
        5,
        10.0,
        t,
        animated,
        0.9,
    );
    band(half, y(0.72), 1.4);
    // the locked door: an arch, dark, with iron straps, a glowing keyhole and a thin edge of light
    let rr = half * 0.62;
    let door_top = y(0.76);
    let door_bot = y(0.93);
    let arch_c = pos2(tx, door_top + rr);
    let mut arch = vec![pos2(tx - rr, door_bot), pos2(tx - rr, arch_c.y)];
    for i in 1..8 {
        let a = std::f32::consts::PI * (1.0 + i as f32 / 8.0);
        arch.push(arch_c + vec2(a.cos(), a.sin()) * rr);
    }
    arch.push(pos2(tx + rr, arch_c.y));
    arch.push(pos2(tx + rr, door_bot));
    painter.add(Shape::convex_polygon(
        arch.clone(),
        Color32::from_rgb(8, 6, 12),
        Stroke::new(1.6, GOLD),
    ));
    for f in [0.30f32, 0.72] {
        let by = door_top + (door_bot - door_top) * f;
        painter.line_segment(
            [pos2(tx - rr, by), pos2(tx + rr, by)],
            Stroke::new(1.4, Color32::from_rgb(70, 66, 76)),
        );
    }
    painter.line_segment(
        [pos2(tx, arch_c.y - rr * 0.9), pos2(tx, door_bot)],
        Stroke::new(1.0, alpha([255, 184, 84], 0.25)),
    );
    let key_at = pos2(tx, door_top + (door_bot - door_top) * 0.56);
    let pulse = 0.55 + 0.45 * light.glow;
    soft_ellipse(painter, key_at, vec2(rr * 1.5, rr * 1.5), amber, 0.30 * pulse);
    painter.circle_filled(
        key_at,
        (half * 0.12).max(1.6),
        alpha([255, 214, 130], 0.5 + 0.5 * pulse),
    );
    painter.add(Shape::convex_polygon(
        vec![
            key_at + vec2(-half * 0.05, 0.0),
            key_at + vec2(half * 0.05, 0.0),
            key_at + vec2(half * 0.08, half * 0.26),
            key_at + vec2(-half * 0.08, half * 0.26),
        ],
        alpha([255, 214, 130], 0.5 + 0.5 * pulse),
        Stroke::NONE,
    ));
    // the clock stage, wider than the shaft, with gold corners
    block(tx - half * 1.4, y(0.28), tx + half * 1.4, y(0.50), stone_l);
    painter.add(Shape::closed_line(
        vec![
            pos2(tx - half * 1.4, y(0.28)),
            pos2(tx + half * 1.4, y(0.28)),
            pos2(tx + half * 1.4, y(0.50)),
            pos2(tx - half * 1.4, y(0.50)),
        ],
        Stroke::new(1.4, GOLD),
    ));
    band(half * 1.5, y(0.50), 2.0);
    band(half * 1.5, y(0.28), 2.0);
    for sx in [-1.0f32, 1.0] {
        for f in [0.31f32, 0.47] {
            painter.circle_filled(pos2(tx + sx * half * 1.22, y(f)), (half * 0.07).max(1.0), GOLD);
        }
    }
    let face_c = pos2(tx, y(0.39));
    let face_r = (half * 1.0).min(hh * 0.095);
    paint_clock_light(painter, face_c, face_r, light);
    paint_clock_face(painter, face_c, face_r, light);
    // the belfry: three lit openings under the roof
    block(tx - half * 0.95, y(0.18), tx + half * 0.95, y(0.28), stone);
    for dx in [-0.55f32, 0.0, 0.55] {
        let o =
            Rect::from_center_size(pos2(tx + dx * half, y(0.23)), vec2(half * 0.34, hh * 0.065));
        painter.rect_filled(o, 0.0, alpha(amber, 0.55 + 0.3 * light.glow));
        painter.rect_stroke(o, 0.0, Stroke::new(0.8, GOLD), egui::StrokeKind::Outside);
    }
    painter.line_segment(
        [pos2(tx - half * 0.95, y(0.28)), pos2(tx + half * 0.95, y(0.28))],
        Stroke::new(1.6, GOLD),
    );
    // the spire and its beacon
    painter.add(Shape::convex_polygon(
        vec![
            pos2(tx - half * 1.2, y(0.18)),
            pos2(tx + half * 1.2, y(0.18)),
            pos2(tx, top + hh * 0.025),
        ],
        Color32::from_rgb(78, 56, 30),
        Stroke::new(1.2, GOLD),
    ));
    painter.line_segment(
        [pos2(tx, y(0.18)), pos2(tx, top + hh * 0.025)],
        Stroke::new(1.0, alpha([255, 214, 130], 0.4)),
    );
    soft_ellipse(
        painter,
        pos2(tx, top + hh * 0.02),
        vec2(half * 2.4, half * 2.4),
        amber,
        0.20 + 0.50 * light.glow,
    );
    painter.circle_filled(
        pos2(tx, top + hh * 0.02),
        (half * 0.16).max(2.0),
        alpha([255, 230, 170], 0.9),
    );
}

/// The clock's light: its own dim glow, and every so often a swell with a ring of light leaving the
/// clock (`cat_talk::clock_light`). Drawn under the clock face.
fn paint_clock_light(painter: &egui::Painter, c: Pos2, r: f32, light: cat_talk::ClockLight) {
    soft_ellipse(painter, c, vec2(r * 2.3, r * 2.3), [255, 196, 96], 0.18 + 0.32 * light.glow);
    if light.glow > 0.01 {
        soft_ellipse(painter, c, vec2(r * 7.0, r * 6.0), [255, 170, 70], 0.16 * light.glow);
    }
    if let Some(u) = light.ring {
        for (lag, strength) in [(0.0f32, 0.55f32), (0.2, 0.3)] {
            let v = u - lag;
            if v > 0.0 {
                painter.circle_stroke(
                    c,
                    r * (1.15 + 5.0 * v),
                    Stroke::new(1.6, alpha([255, 216, 140], strength * (1.0 - v))),
                );
            }
        }
    }
}

/// A cat seen from the side in ordinary clothes: coat, trousers, shoes, sometimes a cap or a
/// bowler, a scarf, a bag. `feet` is where it stands, `height` how tall (ears and hat included),
/// `dir` which way it faces (1 right, -1 left), `step` where it is in its stride (radians; `None`
/// stands still), `shade` how dark the street makes it.
#[allow(clippy::too_many_arguments)]
fn paint_stroller(
    painter: &egui::Painter,
    feet: Pos2,
    height: f32,
    dir: f32,
    step: Option<f32>,
    who: &cat_talk::Walker,
    shade: f32,
    t: f32,
) {
    let u = height / 108.0;
    let bob = step.map_or(0.0, |s| s.cos().abs() * 1.6);
    let p = |x: f32, y: f32| pos2(feet.x + dir * x * u, feet.y - (y + bob) * u);
    let q = |x: f32, y: f32| pos2(feet.x + dir * x * u, feet.y - y * u);
    let fur = tint(FURS[who.fur as usize % FURS.len()], shade);
    let fur_l = tint(FURS[who.fur as usize % FURS.len()], shade * 1.2);
    let coat = tint(COATS[who.coat as usize % COATS.len()], shade);
    let accent = tint(COATS[(who.coat as usize + 3) % COATS.len()], shade * 1.1);
    let hat_col = tint(COATS[(who.coat as usize + 5) % COATS.len()], shade * 0.7);
    let trousers = tint([44, 38, 58], shade);
    let ink = tint([12, 8, 18], shade.max(0.6));
    let line = (0.9 * u).max(1.0);
    let swing = step.map_or(0.0, |s| s.sin() * 0.55);
    // legs: the one behind, the tail, the coat, the one in front
    let leg = |hip_x: f32, a: f32, col: Color32| {
        let foot = (hip_x + 38.0 * a.sin(), 40.0 - 38.0 * a.cos());
        painter.line_segment([q(hip_x, 40.0), q(foot.0, foot.1)], Stroke::new(8.0 * u, col));
        painter.add(Shape::ellipse_filled(
            q(foot.0 + 3.0, foot.1 + 3.4),
            vec2(6.0 * u, 3.4 * u),
            ink,
        ));
    };
    leg(-4.0, -swing - 0.07, tint([44, 38, 58], shade * 0.75));
    let sway = (t * if step.is_some() { 2.0 } else { 1.2 } + who.x0 * 9.0).sin() * 3.0;
    let tail: Vec<Pos2> = [
        (-14.0, 46.0),
        (-24.0, 50.0),
        (-31.0 - sway * 0.3, 62.0),
        (-30.0 + sway, 76.0),
        (-24.0 + sway * 1.4, 85.0),
    ]
    .iter()
    .map(|&(x, y)| p(x, y))
    .collect();
    painter.add(Shape::line(tail.clone(), Stroke::new(8.0 * u, ink)));
    painter.add(Shape::line(tail, Stroke::new(5.6 * u, fur)));
    poly(
        painter,
        &[p(-15.0, 75.0), p(14.0, 75.0), p(18.0, 38.0), p(-19.0, 38.0)],
        coat,
        Some((ink, line)),
    );
    painter.line_segment(
        [p(-19.0, 41.0), p(18.0, 41.0)],
        Stroke::new(1.6 * u, tint(COATS[who.coat as usize % COATS.len()], shade * 0.7)),
    );
    leg(4.0, swing + 0.07, trousers);
    // the arm swings against the front leg, and may carry a bag
    let hand = (2.0 - 24.0 * swing.sin(), 70.0 - 24.0 * swing.cos());
    painter.line_segment([p(1.0, 70.0), p(hand.0, hand.1)], Stroke::new(9.0 * u, ink));
    painter.line_segment([p(1.0, 70.0), p(hand.0, hand.1)], Stroke::new(6.6 * u, coat));
    if who.bag {
        painter.line_segment(
            [p(hand.0, hand.1), p(hand.0 - 4.0, hand.1 - 6.0)],
            Stroke::new(1.2 * u, ink),
        );
        poly(
            painter,
            &[
                p(hand.0 - 7.0, hand.1 - 6.0),
                p(hand.0 + 7.0, hand.1 - 6.0),
                p(hand.0 + 7.0, hand.1 - 19.0),
                p(hand.0 - 7.0, hand.1 - 19.0),
            ],
            tint([120, 78, 42], shade),
            Some((ink, line)),
        );
    }
    painter.circle_filled(p(hand.0, hand.1), 3.8 * u, fur);
    // the head: ears, face, scarf, hat
    poly(
        painter,
        &[p(3.0, 93.0), p(10.0, 96.0), p(4.0, 107.0)],
        tint(FURS[who.fur as usize % FURS.len()], shade * 0.8),
        Some((ink, line)),
    );
    poly(painter, &[p(10.0, 95.0), p(18.0, 94.0), p(15.0, 108.0)], fur, Some((ink, line)));
    painter.circle_filled(p(14.0, 86.0), 11.0 * u, fur);
    painter.circle_stroke(p(14.0, 86.0), 11.0 * u, Stroke::new(line, ink));
    painter.add(Shape::ellipse_filled(p(24.0, 81.5), vec2(6.5 * u, 4.6 * u), fur_l));
    painter.circle_filled(p(29.5, 83.0), 1.7 * u, tint([214, 120, 104], shade));
    painter.circle_filled(p(19.0, 88.0), 1.7 * u, ink);
    if who.scarf {
        poly(
            painter,
            &[p(6.0, 77.0), p(18.0, 79.0), p(19.0, 71.0), p(5.0, 68.0)],
            accent,
            Some((ink, line)),
        );
        poly(
            painter,
            &[p(4.0, 71.0), p(10.0, 69.0), p(9.0, 52.0), p(3.0, 53.0)],
            accent,
            Some((ink, line)),
        );
    }
    match who.hat {
        1 => {
            poly(
                painter,
                &[p(5.0, 94.0), p(8.0, 102.0), p(18.0, 103.0), p(24.0, 95.0)],
                hat_col,
                Some((ink, line)),
            );
            poly(
                painter,
                &[p(22.0, 95.0), p(31.0, 93.0), p(30.0, 96.5), p(23.0, 98.0)],
                hat_col,
                Some((ink, line)),
            );
        }
        2 => {
            poly(
                painter,
                &[
                    p(6.0, 95.0),
                    p(7.0, 103.0),
                    p(12.0, 108.0),
                    p(19.0, 108.0),
                    p(23.0, 103.0),
                    p(23.0, 95.0),
                ],
                hat_col,
                Some((ink, line)),
            );
            painter.line_segment([p(2.0, 95.0), p(27.0, 95.0)], Stroke::new(3.2 * u, ink));
            painter.line_segment([p(2.5, 95.0), p(26.5, 95.0)], Stroke::new(2.0 * u, hat_col));
        }
        _ => {}
    }
}

/// A cat selling things, seen from the front over the top of her counter. `base` is the middle of
/// the counter's top edge, `u` how big one point of the drawing is (the vendor is about 90 high).
/// One of them has a fez and one a headband; they both keep an eye on the gentleman in the alley.
fn paint_vendor(painter: &egui::Painter, base: Pos2, u: f32, which: usize, t: f32, animated: bool) {
    let tm = if animated { t } else { 0.0 };
    let hb = (tm * 1.6 + which as f32 * 2.0).sin() * 1.2;
    let p = |x: f32, y: f32| pos2(base.x + x * u, base.y - y * u);
    let (fur, shirt, apron, hat) = if which == 0 {
        ([196u8, 150, 96], [88u8, 110, 150], [226u8, 214, 190], [170u8, 40, 52])
    } else {
        ([120, 120, 130], [120, 70, 96], [70, 120, 92], [214, 170, 60])
    };
    let (fur, fur_l, shirt, apron, hat) =
        (tint(fur, 1.0), tint(fur, 1.2), tint(shirt, 1.0), tint(apron, 1.0), tint(hat, 1.0));
    let ink = Color32::from_rgb(12, 8, 18);
    let line = (0.9 * u).max(1.0);
    let side = if which == 0 { 1.0f32 } else { -1.0 };
    // the body, with an apron, and two arms: one on the counter, one calling people over
    poly(
        painter,
        &[p(-22.0, -4.0), p(22.0, -4.0), p(24.0, 42.0), p(-24.0, 42.0)],
        shirt,
        Some((ink, line)),
    );
    poly(
        painter,
        &[p(-15.0, -4.0), p(15.0, -4.0), p(11.0, 36.0), p(-11.0, 36.0)],
        apron,
        Some((ink, line)),
    );
    let wave = (tm * 3.0 + which as f32).sin();
    let hand = (side * (35.0 + 4.0 * wave), 58.0 + 4.0 * (tm * 3.0 + which as f32).cos());
    painter.line_segment([p(side * 23.0, 40.0), p(hand.0, hand.1)], Stroke::new(10.0 * u, ink));
    painter.line_segment([p(side * 23.0, 40.0), p(hand.0, hand.1)], Stroke::new(7.4 * u, shirt));
    painter.circle_filled(p(hand.0, hand.1), 4.8 * u, fur);
    painter.line_segment([p(-side * 23.0, 40.0), p(-side * 29.0, 3.0)], Stroke::new(10.0 * u, ink));
    painter
        .line_segment([p(-side * 23.0, 40.0), p(-side * 29.0, 3.0)], Stroke::new(7.4 * u, shirt));
    painter.circle_filled(p(-side * 29.0, 3.0), 4.8 * u, fur);
    // the head
    for sx in [-1.0f32, 1.0] {
        poly(
            painter,
            &[p(sx * 17.0, 66.0 + hb), p(sx * 7.0, 73.0 + hb), p(sx * 16.0, 91.0 + hb)],
            fur,
            Some((ink, line)),
        );
    }
    painter.circle_filled(p(0.0, 62.0 + hb), 16.0 * u, fur);
    painter.circle_stroke(p(0.0, 62.0 + hb), 16.0 * u, Stroke::new(line, ink));
    painter.add(Shape::ellipse_filled(p(0.0, 55.0 + hb), vec2(8.0 * u, 5.5 * u), fur_l));
    painter.circle_filled(p(0.0, 58.0 + hb), 2.0 * u, Color32::from_rgb(214, 120, 104));
    for sx in [-1.0f32, 1.0] {
        painter.line_segment([p(0.0, 56.0 + hb), p(sx * 3.0, 53.0 + hb)], Stroke::new(line, ink));
        for dy in [4.0f32, 0.0, -4.0] {
            painter.line_segment(
                [p(sx * 9.0, 56.0 + hb), p(sx * 24.0, 56.0 + dy + hb)],
                Stroke::new(0.8, alpha([230, 220, 200], 0.6)),
            );
        }
        let eye = p(sx * 6.5, 65.0 + hb);
        if animated && (tm * 0.3 + which as f32 * 0.4).fract() < 0.03 {
            painter.line_segment(
                [eye - vec2(2.4 * u, 0.0), eye + vec2(2.4 * u, 0.0)],
                Stroke::new(line, ink),
            );
        } else {
            painter.circle_filled(eye, 2.8 * u, Color32::from_rgb(244, 236, 210));
            painter.circle_filled(eye + vec2(side * 1.1 * u, 0.0), 1.5 * u, ink);
        }
    }
    if which == 0 {
        poly(
            painter,
            &[p(-6.0, 75.0 + hb), p(6.0, 75.0 + hb), p(5.0, 88.0 + hb), p(-5.0, 88.0 + hb)],
            hat,
            Some((ink, line)),
        );
        painter.line_segment([p(5.0, 88.0 + hb), p(12.0, 81.0 + hb)], Stroke::new(1.2 * u, GOLD));
    } else {
        poly(
            painter,
            &[p(-15.5, 69.0 + hb), p(15.5, 69.0 + hb), p(15.0, 75.0 + hb), p(-15.0, 75.0 + hb)],
            hat,
            Some((ink, line)),
        );
        painter.circle_filled(p(15.0, 72.0 + hb), 3.2 * u, hat);
    }
}

/// One of the two stalls that make the alley's mouth, with its vendor. `which` 0 is on the left of
/// the cat, 1 on the right.
fn paint_stall(painter: &egui::Painter, r: Rect, which: usize, t: f32, animated: bool) {
    let (w, h) = (r.width(), r.height());
    let ground = r.bottom() - h * 0.16;
    let cx = r.left() + w * (ALLEY_AT + if which == 0 { -0.17 } else { 0.19 });
    let sw = w * 0.15;
    let top = ground - h * 0.34;
    let awning = h * 0.075;
    let counter = ground - h * 0.09;
    // the back of the stall, lit from inside, with a shelf of jars
    painter.rect_filled(
        Rect::from_min_max(pos2(cx - sw * 0.5, top + awning * 0.5), pos2(cx + sw * 0.5, ground)),
        0.0,
        Color32::from_rgb(34, 22, 40),
    );
    soft_ellipse(
        painter,
        pos2(cx, counter - h * 0.1),
        vec2(sw * 0.62, h * 0.17),
        [255, 170, 90],
        0.30,
    );
    let shelf = top + awning + h * 0.02;
    painter.line_segment(
        [pos2(cx - sw * 0.46, shelf), pos2(cx + sw * 0.46, shelf)],
        Stroke::new(1.5, Color32::from_rgb(90, 60, 44)),
    );
    for j in 0..5u32 {
        let jx = cx - sw * 0.38 + sw * 0.19 * j as f32;
        let tone =
            [[230u8, 160, 70], [200, 80, 110], [120, 190, 220], [244, 220, 150], [150, 220, 140]]
                [((j + which as u32 * 2) % 5) as usize];
        painter.rect_filled(
            Rect::from_min_max(
                pos2(jx - sw * 0.04, shelf - h * 0.035),
                pos2(jx + sw * 0.04, shelf),
            ),
            1.5,
            alpha(tone, 0.8),
        );
    }
    paint_vendor(painter, pos2(cx, counter), h * 0.0017, which, t, animated);
    // the counter, with its goods
    painter.rect_filled(
        Rect::from_min_max(pos2(cx - sw * 0.5, counter), pos2(cx + sw * 0.5, ground)),
        0.0,
        Color32::from_rgb(58, 36, 48),
    );
    painter.rect_filled(
        Rect::from_min_max(
            pos2(cx - sw * 0.54, counter),
            pos2(cx + sw * 0.54, counter + h * 0.014),
        ),
        1.0,
        Color32::from_rgb(110, 76, 52),
    );
    for g in 0..4u32 {
        let gx = cx - sw * 0.38 + sw * 0.25 * g as f32;
        let tone = [[230u8, 160, 70], [200, 80, 110], [120, 190, 220], [244, 220, 150]]
            [((g + which as u32) % 4) as usize];
        painter.circle_filled(pos2(gx, counter - h * 0.012), h * 0.014, alpha(tone, 0.9));
    }
    // the awning, striped, with a lantern on the side facing the alley
    for st in 0..6usize {
        let x0 = cx - sw * 0.55 + sw * 1.1 * st as f32 / 6.0;
        let x1 = cx - sw * 0.55 + sw * 1.1 * (st + 1) as f32 / 6.0;
        let col = if (st + which) % 2 == 0 {
            Color32::from_rgb(132, 38, 56)
        } else {
            Color32::from_rgb(206, 186, 150)
        };
        painter.add(Shape::convex_polygon(
            vec![
                pos2(x0, top),
                pos2(x1, top),
                pos2(x1 + 3.0, top + awning),
                pos2(x0 - 3.0, top + awning),
            ],
            col.gamma_multiply(0.8),
            Stroke::NONE,
        ));
    }
    let sway = if animated { (t * 1.1 + which as f32 * 2.0).sin() * 1.5 } else { 0.0 };
    let inner = if which == 0 { 1.0 } else { -1.0 };
    let lantern = pos2(cx + inner * sw * 0.46 + sway, top + awning + h * 0.05);
    painter.line_segment(
        [pos2(lantern.x - sway, top + awning), lantern],
        Stroke::new(1.0, alpha([60, 40, 40], 0.9)),
    );
    soft_ellipse(painter, lantern, vec2(h * 0.10, h * 0.10), [255, 180, 90], 0.5);
    painter.circle_filled(lantern, h * 0.015, alpha([255, 210, 130], 1.0));
}

/// The alley the cat stands at the mouth of: a building either side with lit windows, the far end
/// dim, and a lamp on a rope across it.
fn paint_alley(painter: &egui::Painter, r: Rect, t: f32, animated: bool) {
    let (w, h) = (r.width(), r.height());
    let ground = r.bottom() - h * 0.16;
    let x = |f: f32| r.left() + w * f;
    let (c_out, c_in) = (Color32::from_rgb(12, 40, 46), Color32::from_rgb(4, 16, 20));
    // the far end of the alley
    gradient(
        painter,
        Rect::from_min_max(pos2(x(ALLEY_AT - 0.10), r.top()), pos2(x(ALLEY_AT + 0.12), ground)),
        [
            Color32::from_rgb(10, 32, 38),
            Color32::from_rgb(10, 32, 38),
            Color32::from_rgb(5, 16, 20),
            Color32::from_rgb(5, 16, 20),
        ],
    );
    // the two buildings, darker toward the alley, with their windows
    for (wi, (x0, x1)) in [(ALLEY_AT - 0.30, ALLEY_AT - 0.075), (ALLEY_AT + 0.095, ALLEY_AT + 0.30)]
        .into_iter()
        .enumerate()
    {
        let wall = Rect::from_min_max(pos2(x(x0), r.top()), pos2(x(x1), ground));
        let corners = if wi == 0 { [c_out, c_in, c_in, c_out] } else { [c_in, c_out, c_out, c_in] };
        gradient(painter, wall, corners);
        super::circuit_city::paint_vias(
            painter,
            wall,
            7000 + wi as u32 * 500,
            h * 0.078,
            t,
            animated,
            0.9,
        );
        // the lamp light catches the edge of the wall that faces the alley
        let edge = if wi == 0 { wall.right() } else { wall.left() };
        painter.line_segment(
            [pos2(edge, wall.top()), pos2(edge, wall.bottom())],
            Stroke::new(1.0, alpha([96, 220, 255], 0.22)),
        );
    }
    // a lamp on a rope across the alley
    let (a, b) = (
        pos2(x(ALLEY_AT - 0.075), r.top() + h * 0.22),
        pos2(x(ALLEY_AT + 0.095), r.top() + h * 0.22),
    );
    let mid = pos2((a.x + b.x) * 0.5, a.y + h * 0.03);
    painter.add(Shape::line(
        vec![
            a,
            pos2((a.x + mid.x) * 0.5, mid.y - h * 0.008),
            mid,
            pos2((mid.x + b.x) * 0.5, mid.y - h * 0.008),
            b,
        ],
        Stroke::new(1.0, alpha([60, 44, 56], 0.95)),
    ));
    let sway = if animated { (t * 0.9).sin() * 1.2 } else { 0.0 };
    let lamp = mid + vec2(sway, h * 0.045);
    painter.line_segment([mid, lamp], Stroke::new(1.0, alpha([60, 44, 56], 0.95)));
    soft_ellipse(painter, lamp, vec2(h * 0.20, h * 0.20), [255, 190, 100], 0.5);
    painter.circle_filled(lamp, h * 0.017, alpha([255, 214, 140], 1.0));
}

/// The strip of night market at the top of the page: a skyline with lit windows and the clock tower
/// (whose light swells every so often), an alley between two stalls with a cat selling at each, cats
/// in ordinary clothes queueing at the stalls and walking by in two lines, the cobbles and the string
/// lights. The gentleman stands at the mouth of the alley, drawn by the caller on top of all this.
fn paint_street(painter: &egui::Painter, r: Rect, t: f32, animated: bool, cat: &dyn Fn()) {
    let (w, h) = (r.width(), r.height());
    let at = |x: f32, y: f32| pos2(r.left() + x * w, r.top() + y * h);
    let ground = r.bottom() - h * 0.16;
    super::circuit_city::paint_city(painter, r, ground, t, animated);
    // the clock tower on the left: lit, tall, with a locked door at its foot and a light that swells now and then
    let tower_top = r.top() + h * 0.03;
    let tower_half = (w * 0.026).max((ground - tower_top) * 0.055);
    paint_clock_tower(painter, r.left() + w * 0.13, ground, tower_top, tower_half, t, animated);
    // the alley, with a stall and its vendor either side of the mouth
    paint_alley(painter, r, t, animated);
    for which in 0..2 {
        paint_stall(painter, r, which, t, animated);
    }
    // the street, with traces running along it
    let street = Rect::from_min_max(pos2(r.left(), ground), r.right_bottom());
    gradient(
        painter,
        street,
        [
            Color32::from_rgb(8, 26, 30),
            Color32::from_rgb(8, 26, 30),
            Color32::from_rgb(3, 10, 14),
            Color32::from_rgb(3, 10, 14),
        ],
    );
    super::circuit_city::paint_street_traces(painter, street, t, animated);
    // two cats asleep in the alley, and the one who stands there
    for (sx, fur, flip, size) in
        [(ALLEY_AT - 0.075, 0usize, 1.0f32, 0.042f32), (ALLEY_AT + 0.09, 1, -1.0, 0.04)]
    {
        paint_sleeper(
            painter,
            pos2(r.left() + sx * w, ground + h * 0.045),
            h * size,
            fur,
            flip,
            if animated { t } else { 0.0 },
        );
    }
    cat();
    // the queues at the stalls: one waiting on the left, two on the right, all facing the vendor
    let queue = [
        (0.372f32, 1.0f32, 2u8, 0u8, 1u8, true, false),
        (0.93, -1.0, 5, 3, 0, false, true),
        (0.965, -1.0, 4, 2, 2, true, false),
    ];
    for (qi, (fx, dir, coat, fur, hat, scarf, bag)) in queue.into_iter().enumerate() {
        let who =
            cat_talk::Walker { lane: 0, x0: fx + qi as f32 * 0.1, coat, fur, hat, scarf, bag };
        paint_stroller(
            painter,
            pos2(r.left() + fx * w, ground + h * 0.03),
            h * 0.24,
            dir,
            None,
            &who,
            0.8,
            if animated { t } else { 0.0 },
        );
    }
    // everyone walking by, in two single files: the far one smaller and darker, going right, then the near one going left
    for lane in 0..cat_talk::LANES.len() {
        let (lift, height, shade) =
            if lane == 0 { (0.055f32, 0.22f32, 0.72f32) } else { (0.095, 0.26, 0.92) };
        for who in cat_talk::WALKERS.iter().filter(|wk| wk.lane == lane) {
            let fx = cat_talk::walker_x(who, if animated { t } else { 0.0 });
            let dir = cat_talk::LANES[lane].dir;
            let stride = h * height * 0.84;
            let phase = fx * dir * w / stride * std::f32::consts::TAU;
            paint_stroller(
                painter,
                pos2(r.left() + fx * w, ground + h * lift),
                h * height,
                dir,
                Some(phase),
                who,
                shade,
                if animated { t } else { 0.0 },
            );
        }
    }
    // string lights over the market
    for (row, (y0, sag)) in [(0.05f32, 0.07f32), (0.13, 0.06)].into_iter().enumerate() {
        let bulbs = 14;
        let mut prev: Option<Pos2> = None;
        for b in 0..=bulbs {
            let u = b as f32 / bulbs as f32;
            let sway = if animated { (t * 0.8 + u * 4.0 + row as f32).sin() * 1.5 } else { 0.0 };
            let p = at(u, y0 + sag * (u * std::f32::consts::PI).sin()) + vec2(0.0, sway);
            if let Some(q) = prev {
                painter.line_segment([q, p], Stroke::new(1.0, alpha([40, 30, 50], 0.9)));
            }
            prev = Some(p);
            let hue = [[255, 200, 110], [255, 120, 180], [130, 220, 255], [255, 220, 150]]
                [(b + row * 2) % 4];
            let flick = if animated { 0.75 + 0.25 * (t * 2.0 + b as f32 * 1.7).sin() } else { 1.0 };
            soft_ellipse(painter, p + vec2(0.0, 4.0), vec2(h * 0.045, h * 0.045), hue, 0.5 * flick);
            painter.circle_filled(p + vec2(0.0, 3.0), 2.2, alpha(hue, 1.0));
        }
    }
    // the edges go dark
    let dark = Color32::from_rgba_unmultiplied(4, 2, 10, 200);
    let clear = Color32::from_rgba_unmultiplied(4, 2, 10, 0);
    gradient(painter, Rect::from_min_size(r.min, vec2(w * 0.1, h)), [dark, clear, clear, dark]);
    gradient(
        painter,
        Rect::from_min_max(pos2(r.right() - w * 0.1, r.top()), r.right_bottom()),
        [clear, dark, dark, clear],
    );
}

/// Circuit City, drawn over the whole panel behind the tabs, for the Circuit City backdrop style.
/// `t` is the time in seconds (0 holds it still); `strength` is how strong the backdrop is.
pub fn paint_market(ui: &egui::Ui, t: f32, strength: f32) {
    super::circuit_city::paint_window(ui, t, strength);
}
