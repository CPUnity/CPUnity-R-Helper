// The people at the Hold'em table, painted from the pictures and shapes the rest of the program
// already has: the dead king (the Void's backdrop, laughing with his head going right round), Father
// Time (his own layers, smoking, with his beard that falls off), the Shadow Monarch (his painted
// body and two faces) and the croupier, Mammon, drawn by hand. The Demon Bear is in poker_bear.rs.
//
// Every function draws one person where you tell it, at the size you tell it, and clips nothing:
// the table screen puts a clip rectangle round each seat so they sit behind the felt.
use super::backdrop::{soft_ellipse, tint};
use super::dead_king::{dead_colour, fill_fan, paint_dead_eye};
use super::monarch_art::{self, Art};
use super::poker_bear::{ellipse, lerp, norm, rgba, Draw, Pt};
use crate::{father, gold, king, smoker, theme};
use eframe::egui::{self, pos2, vec2, Color32, ColorImage, Context, Id, Painter, Pos2, Rect, Shape, Stroke, TextureHandle, TextureOptions, Vec2};
use std::f32::consts::TAU;
use std::sync::Arc;

fn alpha(c: [u8; 3], a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

// ---- the dead king ------------------------------------------------------------------------------

/// How much of the King's aura is there at time `t` (0 to 1): two slow waves that add up, shaped so that
/// it is full for a while, thins, is gone for a few seconds, and comes back. `phase` keeps two lobes apart.
fn aura_veil(t: f32, phase: f32) -> f32 {
    let w = (t * 0.29 + phase).sin() + 0.7 * (t * 0.113 + phase * 1.7 + 1.0).sin();
    let x = ((w + 0.55) / 1.45).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The dead king, with the middle of his head at `head`, one picture pixel being `scale` points.
/// `laugh` is how long ago the current cackle began (None when he is not laughing): his jaw hangs
/// open and shuts in slow "ha"s while his head turns all the way round on his neck.
pub fn paint_king(painter: &Painter, head: Pos2, scale: f32, t: f32, laugh: Option<f32>) {
    let pose = king::dead_pose(t, laugh, true);
    let origin = head - vec2(700.0, 462.0) * scale;
    let at = |p: (f32, f32)| pos2(origin.x + p.0 * scale, origin.y + p.1 * scale);
    let beat = king::dead_beat(t);
    // death: a cold dark bloom behind him. It drifts and swells and thins, and now and then it is gone
    // altogether, so that the room behind him shows through (two lobes with their own slow rhythms)
    let veil = aura_veil(t, 0.0);
    let veil2 = aura_veil(t, 2.3);
    let drift = vec2(22.0 * (t * 0.21).sin() + 12.0 * (t * 0.13 + 1.0).sin(), 14.0 * (t * 0.17 + 2.0).sin()) * scale;
    let swell = 0.92 + 0.08 * (t * 0.23).sin();
    soft_ellipse(painter, at(king::HEART) + vec2(0.0, -20.0 * scale) + drift, vec2(520.0, 540.0) * scale * swell, king::DEATH_DEEP, (0.40 + 0.12 * beat) * veil);
    soft_ellipse(painter, at(king::HEART) + vec2(0.0, 70.0 * scale) - drift * 1.7, vec2(360.0, 400.0) * scale * (1.02 - 0.06 * (t * 0.19 + 1.0).sin()), king::DEATH_DEEP, 0.26 * veil2);
    let polys: Vec<(Vec<Pos2>, king::Poly)> = king::scene(&pose)
        .into_iter()
        .map(|poly| {
            let points = poly.points.iter().map(|&p| at(p)).collect();
            (points, poly)
        })
        .collect();
    // the glow hugging his outline: violet further out, mint close in, swelling with the heart he lacks
    for (points, poly) in &polys {
        if poly.fill == king::Tone::None || poly.fill == king::Tone::Eye || points.len() < 3 {
            continue;
        }
        for (colour, width, strength, swell) in [(king::DEATH, 40.0, 0.05, 1.0), (king::DEATH, 20.0, 0.07, 1.0), (king::LIFE, 14.0, 0.05, 0.55 + 0.9 * beat), (king::LIFE, 5.0, 0.12, 0.55 + 0.9 * beat)] {
            // (the violet along his outline comes and goes with the aura; the mint keeps a trace of itself)
            let hold = if colour == king::DEATH { veil } else { 0.3 + 0.7 * veil };
            painter.add(Shape::closed_line(points.clone(), Stroke::new((width * scale).max(1.0), alpha(colour, strength * swell * hold))));
        }
    }
    for (points, poly) in &polys {
        if points.len() < 2 {
            continue;
        }
        if poly.fill == king::Tone::Eye && points.len() >= 3 {
            paint_dead_eye(painter, points, (poly.line.1 * scale).max(1.0), scale);
            continue;
        }
        if poly.fill != king::Tone::None && points.len() >= 3 {
            fill_fan(painter, points, dead_colour(poly.fill.value(), 1.0), vec2(0.0, 0.0));
        }
        if poly.line.1 > 0.0 {
            let stroke = Stroke::new((poly.line.1 * scale).max(1.0), dead_colour(poly.line.0 * 0.8, 1.0));
            painter.add(if poly.closed { Shape::closed_line(points.clone(), stroke) } else { Shape::line(points.clone(), stroke) });
        }
    }
    // the slow broken laughs round his head
    if let Some(s) = laugh {
        for (text, pos, strength, size) in king::dead_has(s) {
            let font = egui::FontId::proportional((size * scale).max(10.0));
            let spot = at(pos);
            painter.text(spot + vec2(-2.0, 0.0), egui::Align2::CENTER_CENTER, text, font.clone(), theme::VOID_CYAN.gamma_multiply(0.5 * strength));
            painter.text(spot + vec2(2.0, 0.0), egui::Align2::CENTER_CENTER, text, font.clone(), theme::VOID_MAGENTA.gamma_multiply(0.5 * strength));
            painter.text(spot, egui::Align2::CENTER_CENTER, text, font, Color32::from_rgba_unmultiplied(170, 140, 235, (strength * 255.0) as u8));
        }
    }
}

// ---- Father Time ----------------------------------------------------------------------------------

/// Father Time's two layers (his solid body, and him in lines without his staff), at full size: the
/// camera comes in close on him. The clock that hangs behind his head on the backdrop is left
/// out here: at the table it was cut off by the edge of his seat.
pub struct FatherTex {
    mask: TextureHandle,
    front: TextureHandle,
}

fn load_father(ctx: &Context) -> Option<FatherTex> {
    let layers: [&[u8]; 2] = [gold::STAFF_PNGS[1], gold::STAFF_PNGS[0]];
    let mut out = Vec::new();
    for (i, png) in layers.iter().enumerate() {
        let img = image::load_from_memory(png).ok()?.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        out.push(ctx.load_texture(format!("cpunity_poker_father_{i}"), ColorImage::from_rgba_unmultiplied(size, img.as_raw()), TextureOptions::LINEAR));
    }
    let front = out.pop()?;
    let mask = out.pop()?;
    Some(FatherTex { mask, front })
}

/// The pictures, made the first time they are asked for (None if any could not be read).
pub fn father_tex(ctx: &Context) -> Option<Arc<FatherTex>> {
    let id = Id::new("cpunity_poker_father_tex");
    if let Some(done) = ctx.data(|d| d.get_temp::<Option<Arc<FatherTex>>>(id)) {
        return done;
    }
    let made = load_father(ctx).map(Arc::new);
    ctx.data_mut(|d| d.insert_temp(id, made.clone()));
    made
}

/// What Father Time is going through.
#[derive(Debug, Clone, Copy, Default)]
pub struct FatherState {
    /// Seconds since his beard fell off (None: he has it).
    pub beard_off: Option<f32>,
    /// Seconds since he stuck it back on.
    pub beard_back: Option<f32>,
    /// He is quivering.
    pub rattled: bool,
    /// Seconds since he began to breathe smoke at you (None: he is not).
    pub blow: Option<f32>,
}

/// Father Time, smoking, with the middle of his face (picture point 600, 700) at `face` and one
/// picture pixel being `scale` points.
pub fn paint_father(painter: &Painter, tex: &FatherTex, face: Pos2, scale: f32, t: f32, st: &FatherState) {
    let shake = if st.rattled { vec2((TAU * 11.0 * t).sin() * 1.6, (TAU * 9.0 * t).cos() * 1.1) } else { vec2(0.0, 0.0) };
    let origin = face - vec2(600.0, 700.0) * scale + shake;
    let at = |p: (f32, f32)| pos2(origin.x + p.0 * scale, origin.y + p.1 * scale);
    let colour = tint(0.95);
    let background = theme::bg();
    let rect = Rect::from_min_max(at((0.0, 0.0)), at((father::PICTURE.0, father::PICTURE.1)));
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    painter.image(tex.mask.id(), rect, uv, background);
    painter.image(tex.front.id(), rect, uv, colour);

    // his eyes over the ones in the picture: heavy-lidded, or wide when he is rattled
    let points = |pts: &[(f32, f32)]| -> Vec<Pos2> { pts.iter().map(|&p| at(p)).collect() };
    for cover in smoker::eye_covers() {
        painter.add(Shape::convex_polygon(points(&cover), background, Stroke::NONE));
    }
    let bright = theme::mix(colour, Color32::WHITE, 0.45);
    let look = smoker::face(if st.rattled { smoker::Mood::Shock } else { smoker::Mood::Droop });
    for eye in &look.eyes {
        painter.add(Shape::convex_polygon(points(&eye.points), bright.gamma_multiply(eye.fill * 0.9), Stroke::new(1.4, colour.gamma_multiply(eye.edge))));
    }
    for &(pos, radius) in &look.pupils {
        painter.circle_filled(at(pos), (radius * scale).max(1.5), background);
    }
    for line in &look.strokes {
        painter.add(Shape::line(points(&line.points), Stroke::new((line.width * scale).max(1.2), colour.gamma_multiply(line.alpha))));
    }

    // the beard: painted over when it is gone, with the pieces of it flying off as pixels
    let mut beard_gone: f32 = 0.0;
    let mut drop: f32 = 0.0;
    if let Some(s) = st.beard_off {
        if let Some(r) = father::reaction(s.min(3.5)) {
            beard_gone = r.beard_gone;
            if s < 3.0 {
                for bit in r.bits {
                    let half = (bit.size * scale * 0.5).max(1.0);
                    painter.rect_filled(Rect::from_center_size(at(bit.pos), vec2(half * 2.0, half * 2.0)), 0.0, colour.gamma_multiply(bit.strength));
                }
            }
        }
        drop = father::jaw(s) * if s < father::REACTION { 1.0 } else { 0.0 };
    } else if let Some(b) = st.beard_back {
        if let Some(r) = father::reaction(4.2 + b) {
            beard_gone = r.beard_gone;
        }
    }
    if beard_gone > 0.01 {
        let beard: Vec<Pos2> = father::BEARD.iter().map(|&p| at(p)).collect();
        fill_fan(painter, &beard, background.gamma_multiply(beard_gone.clamp(0.0, 1.0)), vec2(0.0, 0.0));
    }
    if st.rattled && drop < 20.0 {
        drop = drop.max(18.0);
    }
    // his mouth opens to breathe out
    if let Some(a) = st.blow {
        drop = drop.max(22.0 * crate::poker_cam::bump(a, 0.5, 0.9, 1.9, 2.6, 1.0));
    }
    // his jaw hangs open
    for poly in father::face(drop) {
        let pts = points(&poly.points);
        let fill = match poly.fill {
            father::Fill::None => None,
            father::Fill::Background => Some(background),
            father::Fill::Dim => Some(colour.gamma_multiply(0.18)),
            father::Fill::Bright => Some(colour.gamma_multiply(0.85)),
        };
        if let Some(f) = fill {
            fill_fan(painter, &pts, f, vec2(0.0, 0.0));
        }
        if poly.line > 0.0 {
            let stroke = Stroke::new((poly.line * 1.6 * scale).max(1.0), colour);
            painter.add(if poly.closed { Shape::closed_line(pts, stroke) } else { Shape::line(pts, stroke) });
        }
    }

    // the smoke, behind the cigarette, and the cigarette: a filter, the paper and the ember
    for puff in smoker::smoke(smoker::REST_TIP, t, 0.8) {
        if puff.alpha > 0.004 {
            painter.circle_filled(at(puff.pos), (puff.radius * scale).max(0.8), Color32::from_rgb(235, 215, 170).gamma_multiply(puff.alpha));
        }
    }
    let (b, e) = (at(smoker::MOUTH), at(smoker::REST_TIP));
    let along = e - b;
    if along.length() > 1.0 {
        let dir = along.normalized();
        let side = vec2(-dir.y, dir.x) * (smoker::HALF_WIDTH * scale).max(1.2);
        let edge = Stroke::new(1.0, Color32::from_rgb(150, 104, 26));
        painter.add(Shape::convex_polygon(vec![b + side, e + side, e - side, b - side], Color32::from_rgb(255, 244, 214), edge));
        let filter_end = b + dir * (26.0 * scale);
        painter.add(Shape::convex_polygon(vec![b + side, filter_end + side, filter_end - side, b - side], Color32::from_rgb(214, 150, 60), edge));
    }
    let ember = 0.75 + 0.25 * (TAU * 0.45 * t).sin();
    painter.circle_filled(e, (15.0 * scale).max(3.0), Color32::from_rgb(255, 190, 90).gamma_multiply(0.22 * ember));
    painter.circle_filled(e, (6.5 * scale).max(2.0), Color32::from_rgb(255, 168, 60).gamma_multiply(ember));
    painter.circle_filled(e, (3.2 * scale).max(1.0), Color32::from_rgb(255, 236, 170).gamma_multiply(ember));
}

/// Where Father Time's mouth is on the screen (the cigarette is in its corner).
pub fn father_mouth(face: Pos2, scale: f32) -> Pos2 {
    face + (vec2(smoker::MOUTH.0, smoker::MOUTH.1) - vec2(600.0, 700.0)) * scale
}

fn jitter(i: usize, k: f32) -> f32 {
    ((i as f32 * 12.9898 + k * 78.233).sin() * 43758.547).fract().abs()
}

/// The smoke Father Time breathes at you, painted over the whole screen `screen` (it comes at the
/// camera, so it grows as it comes and ends as a haze). `age` is the seconds since he began.
pub fn paint_blow(painter: &Painter, from: Pos2, screen: Rect, age: f32) {
    use crate::poker_cam::{bump, BLOW_LEN};
    if !(0.0..BLOW_LEN).contains(&age) {
        return;
    }
    let to = pos2(screen.center().x, screen.top() + screen.height() * 0.55);
    let big = screen.height() * 0.55;
    const N: usize = 30;
    for i in 0..N {
        let born = 0.7 + i as f32 * 0.045;
        let a = (age - born) / 1.9;
        if !(0.0..1.0).contains(&a) {
            continue;
        }
        let e = a * a * (3.0 - 2.0 * a);
        let (jx, jy, jr) = (jitter(i, 1.0) - 0.5, jitter(i, 2.0) - 0.5, jitter(i, 3.0));
        let pos = from + (to - from) * e + vec2(jx * screen.width() * 0.3 * a, jy * screen.height() * 0.25 * a);
        let r = (6.0 + (big - 6.0) * a.powf(0.8)) * (0.7 + 0.6 * jr);
        soft_ellipse(painter, pos, vec2(r, r * 0.85), [226, 206, 168], (std::f32::consts::PI * a).sin() * 0.16);
    }
    let haze = bump(age, 1.0, 1.9, 2.6, BLOW_LEN, 1.0);
    if haze > 0.0 {
        painter.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(222, 203, 166, (haze * 0.34 * 255.0) as u8));
    }
}

// ---- the Shadow Monarch ---------------------------------------------------------------------------

const VIOLET: [u8; 3] = [138, 92, 245];

/// Seconds the Shadow takes to come through the void: it tears open, he climbs out of it, it closes behind him.
pub const VOID_LEN: f32 = 6.0;

fn smooth01(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// How dark the room goes (0 to 1) `age` seconds after the void opens.
pub fn void_dim(age: f32) -> f32 {
    crate::poker_cam::bump(age, 0.0, 1.2, 4.2, VOID_LEN, 1.0)
}

/// How open the void is, `age` seconds in: it tears open, holds, and then closes to a dark scar behind him.
fn void_open(age: f32) -> f32 {
    if age < 1.4 {
        smooth01(age / 1.4)
    } else if age < 4.6 {
        1.0
    } else {
        0.34 + 0.66 * (1.0 - smooth01((age - 4.6) / 1.6))
    }
}

/// A hole in the room: a violet rim, black going down into black, and slow arms of light turning into it.
fn paint_void(painter: &Painter, c: Pos2, rx: f32, ry: f32, t: f32, open: f32) {
    if open <= 0.01 {
        return;
    }
    soft_ellipse(painter, c, vec2(rx * 1.4, ry * 1.3) * open, VIOLET, 0.30 * open);
    for (f, col) in [(1.0, [118, 70, 226]), (0.93, [48, 18, 104]), (0.82, [14, 4, 32]), (0.6, [3, 0, 9])] {
        painter.add(Shape::ellipse_filled(c, vec2(rx * f, ry * f) * open, Color32::from_rgba_unmultiplied(col[0], col[1], col[2], (255.0 * open) as u8)));
    }
    for arm in 0..3 {
        for i in 0..24 {
            let u = i as f32 / 24.0;
            let ang = -t * 0.9 + arm as f32 * TAU / 3.0 + u * 5.2;
            let rad = (1.0 - u).powf(0.8) * 0.92;
            let at = c + vec2(ang.cos() * rx * rad, ang.sin() * ry * rad) * open;
            painter.circle_filled(at, rx * 0.028 * (0.35 + u), alpha([186, 160, 255], (0.15 + 0.6 * u) * open));
        }
    }
}

/// The Shadow Monarch with the middle of his head at `head`, the head being `head_pts` points tall, `age`
/// seconds after he came through the void (which tears open at his chest, and he climbs out of it, larger
/// than life; once he is out he leans slowly, black smoke rises off him, and every so often he twitches
/// and the second face flickers through). `second_face` shows the face that tears out of the first.
pub fn paint_monarch(painter: &Painter, art: &Art, head: Pos2, head_pts: f32, t: f32, age: f32, second_face: bool) {
    let (x0, y0, x1, y1) = art.head_a.units();
    let s = head_pts / (y1 - y0).max(1.0);
    let open = void_open(age);
    let rise = smooth01((age - 1.5) / 3.2);
    let a = smooth01((age - 1.6) / 1.6);
    paint_void(painter, head + vec2(0.0, head_pts * 1.0), head_pts * 1.5, head_pts * 1.9, t, open);
    if a <= 0.0 {
        return;
    }
    let sink = (1.0 - rise) * head_pts * 1.8;
    let loom = 1.0 + 0.012 * (TAU * 0.11 * t).sin();
    let bob = (TAU * 0.13 * t).sin() * 2.5;
    let origin = head + vec2(-(x0 + x1) * 0.5 * s, -(y0 + y1) * 0.5 * s + bob + sink);
    let map = move |p: Pt| pos2(origin.x + p.0 * s, origin.y + p.1 * s * loom);
    // the cold light round him, and a darkness that is not shadow
    let pulse = 0.85 + 0.15 * (TAU * 0.35 * t).sin();
    soft_ellipse(painter, map((0.0, 120.0)), vec2(430.0, 480.0) * s, [6, 0, 16], 0.50 * a);
    soft_ellipse(painter, map((0.0, 120.0)), vec2(330.0, 400.0) * s, VIOLET, 0.28 * a * pulse);
    soft_ellipse(painter, map((0.0, -20.0)), vec2(150.0, 170.0) * s, VIOLET, 0.2 * a * pulse);
    // black smoke rising off his shoulders
    for i in 0..5 {
        let base = head + vec2((i as f32 - 2.0) * head_pts * 0.55, head_pts * 0.9 + sink);
        for j in 0..14 {
            let u = j as f32 / 14.0;
            let sway = (t * 0.6 + i as f32 * 1.7 + u * 3.0).sin() * head_pts * 0.16 * u;
            let rise_t = ((t * 0.18 + i as f32 * 0.37) % 1.0) * 0.4;
            let at = base + vec2(sway, -(u + rise_t) * head_pts * 1.5);
            painter.circle_filled(at, head_pts * 0.075 * (1.0 - u * 0.7), alpha([4, 0, 10], 0.34 * a * (1.0 - u)));
        }
    }
    art.body.draw(painter, &map, monarch_art::fade(a));
    // the head leans slowly to one side and the other, and now and then jerks
    let ph = t % 9.0;
    let jerk = if ph < 0.3 { (t * 90.0).sin() * 3.0 * (1.0 - ph / 0.3) } else { 0.0 };
    let flick = ph < 0.12 && !second_face;
    let lean = 0.05 * (TAU * 0.07 * t).sin() + if ph < 0.3 { 0.03 * (1.0 - ph / 0.3) } else { 0.0 };
    let head_map = move |p: Pt| {
        let q = map(p);
        pos2(q.x + lean * (y1 - p.1) * s + jerk, q.y)
    };
    let face = if second_face || flick { &art.head_b } else { &art.head_a };
    face.draw(painter, &head_map, monarch_art::fade(a));
}

// ---- Mammon, the croupier -------------------------------------------------------------------------

/// The demon who deals. He is drawn in the same units as the Bear (a table edge at y 640, the middle
/// at x 400) by `Draw`; `bow` (0 to 1) lowers his head for a moment when he deals a card.
pub fn paint_mammon(painter: &Painter, at: Pos2, scale: f32, t: f32, bow: f32, away: f32) {
    let d = Draw { painter, at, s: scale, ox: 400.0 };
    let dy = 8.0 * bow + 2.0 * (TAU * 0.2 * t).sin();
    let ink = rgba([14, 10, 16], 1.0);
    let ember = rgba([255, 106, 26], 0.55);
    let skin_mid = rgba([158, 38, 42], 1.0);
    let skin_dark = rgba([62, 12, 20], 1.0);
    // the shoulders in a black coat
    let coat = [(240.0, 640.0), (252.0, 524.0), (306.0, 470.0), (362.0, 450.0), (438.0, 450.0), (494.0, 470.0), (548.0, 524.0), (560.0, 640.0)];
    d.fan(&coat, ink);
    d.outline(&coat, ember, 2.0);
    // the shirt, the lapels, the bow tie
    d.fan(&[(374.0, 452.0), (400.0, 566.0), (426.0, 452.0)], rgba([226, 214, 196], 1.0));
    for side in [-1.0_f32, 1.0] {
        let x = |v: f32| 400.0 + side * (400.0 - v);
        let lapel = [(x(360.0), 450.0), (x(374.0), 452.0), (x(400.0), 566.0), (x(374.0), 604.0), (x(326.0), 516.0)];
        d.fan(&lapel, rgba([34, 26, 36], 1.0));
        d.outline(&lapel, rgba([255, 138, 58], 0.45), 1.4);
        d.fan(&[(400.0, 470.0 + dy), (x(364.0), 452.0 + dy), (x(364.0), 488.0 + dy)], rgba([160, 24, 34], 1.0));
    }
    d.dot((400.0, 470.0 + dy), 7.0, rgba([200, 36, 48], 1.0));
    // the neck
    d.fan(&ellipse((400.0, 436.0 + dy * 0.5), 30.0, 30.0, 0.0, 16), skin_dark);
    paint_mammon_arm(&d, t, away);

    // his horns: two heavy curls of bone
    for side in [-1.0_f32, 1.0] {
        let x = |v: f32| 400.0 + side * (400.0 - v);
        let curl = [(x(358.0), 322.0 + dy), (x(334.0), 296.0 + dy), (x(302.0), 286.0 + dy), (x(282.0), 306.0 + dy), (x(290.0), 336.0 + dy), (x(316.0), 344.0 + dy), (x(326.0), 324.0 + dy)];
        let widths = [28.0, 25.0, 21.0, 17.0, 13.0, 9.0, 5.0];
        for i in 0..curl.len() - 1 {
            d.limb(curl[i], curl[i + 1], widths[i], widths[i + 1], rgba([214, 196, 160], 1.0), rgba([120, 96, 60], 1.0));
        }
        // the ear
        d.fan(&[(x(340.0), 356.0 + dy), (x(296.0), 322.0 + dy), (x(344.0), 338.0 + dy)], skin_mid);
    }
    // the head
    let head: Vec<Pt> = [(400.0, 300.0), (448.0, 322.0), (466.0, 372.0), (450.0, 420.0), (420.0, 452.0), (400.0, 458.0), (380.0, 452.0), (350.0, 420.0), (334.0, 372.0), (352.0, 322.0)].iter().map(|&(x, y)| (x, y + dy)).collect();
    d.glow_fan(&head, (400.0, 360.0 + dy), skin_mid, skin_dark);
    d.outline(&head, rgba([255, 138, 58], 0.6), 2.0);
    // the eyes: yellow slits that glow, the brows slanted over them
    for side in [-1.0_f32, 1.0] {
        let c = (400.0 + side * 28.0, 366.0 + dy);
        d.soft(c, 30.0, 18.0, [255, 190, 40], 0.4);
        d.fan(&ellipse(c, 13.0, 6.0, side * -8.0, 14), rgba([255, 214, 70], 1.0));
        d.fan(&ellipse(c, 2.2, 6.0, 0.0, 8), rgba([10, 4, 4], 1.0));
        d.line(&[(c.0 - side * 18.0, c.1 - 10.0 - 3.0 * side.abs()), (c.0 + side * 18.0, c.1 - 18.0)], rgba([10, 4, 4], 0.95), 4.0);
    }
    // a thin moustache, a goatee and a small, pleased smile
    d.line(&[(398.0, 408.0 + dy), (380.0, 404.0 + dy), (360.0, 412.0 + dy)], ink, 3.0);
    d.line(&[(402.0, 408.0 + dy), (420.0, 404.0 + dy), (440.0, 412.0 + dy)], ink, 3.0);
    d.fan(&[(386.0, 436.0 + dy), (414.0, 436.0 + dy), (400.0, 486.0 + dy)], ink);
    d.line(&[(380.0, 424.0 + dy), (400.0, 432.0 + dy), (420.0, 424.0 + dy)], rgba([20, 4, 8], 0.9), 2.4);
    d.line(&[(396.0, 392.0 + dy), (400.0, 402.0 + dy), (404.0, 392.0 + dy)], rgba([40, 8, 14], 0.8), 2.0);
}

// ---- the dealer's mechanical arm and the hand that leaves it -----------------------------------------

/// Where Mammon's wrist is in his own picture (the socket the hand sits in).
const WRIST: Pt = (504.0, 606.0);

/// Where the socket is on the screen, for Mammon drawn at `at` (the table edge, in the middle of him)
/// with one picture unit being `scale` points.
pub fn mammon_dock(at: Pos2, scale: f32) -> Pos2 {
    pos2(at.x + (WRIST.0 - 400.0) * scale, at.y + (WRIST.1 - 640.0) * scale)
}

/// His right arm: a black sleeve to the elbow, then a forearm of steel that ends in a socket. When the
/// hand has gone (`away` 1) the socket is open and lit, with sparks and a cable hanging out of it.
fn paint_mammon_arm(d: &Draw, t: f32, away: f32) {
    let (shoulder, elbow, wrist) = ((520.0, 492.0), (556.0, 572.0), WRIST);
    d.limb(shoulder, elbow, 46.0, 38.0, rgba([22, 16, 24], 1.0), rgba([255, 138, 58], 0.35));
    let steel = rgba([112, 118, 130], 1.0);
    let steel_dark = rgba([58, 62, 74], 1.0);
    d.limb(elbow, wrist, 30.0, 24.0, steel, rgba([255, 150, 70], 0.7));
    // bands across the forearm, and a piston along it
    let n = norm(elbow, wrist);
    for f in [0.3, 0.62] {
        let c = lerp(elbow, wrist, f);
        d.line(&[(c.0 - n.0 * 13.0, c.1 - n.1 * 13.0), (c.0 + n.0 * 13.0, c.1 + n.1 * 13.0)], steel_dark, 3.0);
    }
    d.line(&[(lerp(elbow, wrist, 0.1).0 + n.0 * 5.0, lerp(elbow, wrist, 0.1).1 + n.1 * 5.0), (lerp(elbow, wrist, 0.85).0 + n.0 * 4.0, lerp(elbow, wrist, 0.85).1 + n.1 * 4.0)], rgba([200, 204, 212], 0.7), 2.0);
    // the elbow, with an ember in it
    d.dot(elbow, 12.0, steel_dark);
    d.dot(elbow, 5.0, rgba([255, 120, 40], 0.55 + 0.25 * (t * 2.0).sin()));
    // the socket
    d.dot(wrist, 16.0, steel_dark);
    d.outline(&ellipse(wrist, 16.0, 16.0, 0.0, 14), rgba([255, 150, 70], 0.5), 1.4);
    d.dot(wrist, 11.0, rgba([20, 14, 18], 1.0));
    if away > 0.01 {
        d.soft(wrist, 26.0, 26.0, [255, 120, 40], 0.45 * away);
        d.dot(wrist, 6.0, rgba([255, 190, 90], 0.8 * away));
        // a cable hanging from it, and sparks
        d.line(&[wrist, (wrist.0 + 8.0, wrist.1 + 16.0), (wrist.0 - 2.0, wrist.1 + 30.0), (wrist.0 + 6.0, wrist.1 + 40.0)], rgba([10, 8, 12], away), 3.0);
        for i in 0..3 {
            let ph = (t * 7.0 + i as f32 * 2.1).fract();
            let a = (1.0 - ph) * away;
            let ang = i as f32 * 2.2 + (t * 3.0).floor();
            let (dx, dy) = (ang.cos() * (8.0 + ph * 22.0), ang.sin().abs() * (6.0 + ph * 24.0));
            d.line(&[(wrist.0 + dx * 0.5, wrist.1 + dy * 0.5), (wrist.0 + dx, wrist.1 + dy)], rgba([255, 214, 120], a), 1.8);
        }
    }
}

/// The dealer's robotic hand, in the air or at rest. `at` is the middle of its palm, `dir` the way its
/// fingers point (a unit vector), `s` points to one unit of the hand (it is about 62 units long),
/// `thrust` how hard the rocket in its wrist burns (0 to 1) and `grip` how far its fingers are closed.
pub fn paint_robot_hand(p: &Painter, at: Pos2, dir: Vec2, s: f32, thrust: f32, t: f32, grip: f32) {
    let f = dir;
    let nrm = vec2(-dir.y, dir.x);
    let m = |x: f32, y: f32| at + f * (x * s) + nrm * (y * s);
    let steel = Color32::from_rgb(120, 126, 138);
    let steel_dark = Color32::from_rgb(58, 62, 74);
    let line = Stroke::new((1.0 * s).max(0.6), Color32::from_rgb(24, 22, 30));
    let ember = Color32::from_rgb(255, 130, 44);
    // the rocket: a nozzle on the back of the wrist and, when it burns, a flame
    if thrust > 0.02 {
        let flick = 0.88 + 0.12 * (t * 55.0).sin();
        let len = (12.0 + 62.0 * thrust) * flick;
        let bulge = 5.5 + 1.8 * (t * 41.0).sin().abs();
        soft_ellipse(p, m(-40.0 - len * 0.35, 0.0), vec2(len * 0.7 * s, len * 0.45 * s), [255, 120, 40], 0.40 * thrust);
        p.add(Shape::convex_polygon(vec![m(-38.0, -8.0), m(-38.0 - len * 0.55, -bulge), m(-38.0 - len, 0.0), m(-38.0 - len * 0.55, bulge), m(-38.0, 8.0)], Color32::from_rgba_unmultiplied(255, 112, 30, 220), Stroke::NONE));
        p.add(Shape::convex_polygon(vec![m(-38.0, -4.5), m(-38.0 - len * 0.34, -2.8), m(-38.0 - len * 0.62, 0.0), m(-38.0 - len * 0.34, 2.8), m(-38.0, 4.5)], Color32::from_rgba_unmultiplied(255, 232, 150, 245), Stroke::NONE));
    }
    p.add(Shape::convex_polygon(vec![m(-31.0, -7.0), m(-38.0, -10.0), m(-38.0, 10.0), m(-31.0, 7.0)], steel_dark, line));
    // the wrist cuff and the palm
    p.add(Shape::convex_polygon(vec![m(-31.0, -11.0), m(-22.0, -12.0), m(-22.0, 12.0), m(-31.0, 11.0)], steel_dark, line));
    p.add(Shape::convex_polygon(vec![m(-22.0, -14.0), m(8.0, -15.0), m(15.0, -10.0), m(15.0, 10.0), m(8.0, 15.0), m(-22.0, 14.0)], steel, line));
    p.add(Shape::line(vec![m(-6.0, -13.0), m(-6.0, 13.0)], Stroke::new((1.3 * s).max(0.6), steel_dark)));
    for (x, y) in [(-17.0, -9.0), (-17.0, 9.0), (3.0, -10.0), (3.0, 10.0)] {
        p.circle_filled(m(x, y), (1.5 * s).max(0.6), Color32::from_rgb(200, 204, 212));
    }
    // a lit eye in the palm
    soft_ellipse(p, m(-8.0, 0.0), vec2(9.0 * s, 9.0 * s), [255, 120, 40], 0.35);
    p.circle_filled(m(-8.0, 0.0), (3.0 * s).max(0.8), ember.gamma_multiply(0.75 + 0.25 * (t * 4.0).sin()));
    // three fingers, each of two joints, closing on a card; and a thumb
    let width = Stroke::new((6.4 * s).max(1.0), steel);
    for y in [-9.0f32, 0.0, 9.0] {
        let tip_y = y * (1.0 - 0.6 * grip);
        let a = m(15.0, y);
        let b = m(27.0, y * (1.0 - 0.3 * grip));
        let c = m(40.0 - 3.0 * grip, tip_y);
        p.add(Shape::line_segment([a, b], width));
        p.add(Shape::line_segment([b, c], Stroke::new((5.2 * s).max(0.9), steel)));
        for q in [a, b] {
            p.circle_filled(q, (3.2 * s).max(0.6), steel);
            p.circle_stroke(q, (3.2 * s).max(0.6), Stroke::new((0.7 * s).max(0.4), steel_dark));
        }
        p.circle_filled(c, (2.7 * s).max(0.6), Color32::from_rgb(214, 218, 226));
    }
    let tb = m(2.0, 15.0);
    let tm = m(15.0, 22.0 - 5.0 * grip);
    let tt = m(27.0, 19.0 - 8.0 * grip);
    p.add(Shape::line_segment([tb, tm], width));
    p.add(Shape::line_segment([tm, tt], Stroke::new((5.2 * s).max(0.9), steel)));
    p.circle_filled(tm, (3.2 * s).max(0.6), steel);
    p.circle_filled(tt, (2.7 * s).max(0.6), Color32::from_rgb(214, 218, 226));
}
