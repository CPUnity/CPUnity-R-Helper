// What the broken staff lets out: the glitch dying away while gold seams stitch the tears shut, a
// ring of sand turning the clock back, a hole opening in the middle of space, a voice in the dark,
// talons coming out of it, violet fire in one hand, eyes, and then the shadow monarch himself.
// The timeline and the shapes are in monarch.rs; this only draws them, over the dark page.
use super::backdrop::soft_ellipse;
use super::monarch_art::{self, Art};
use crate::monarch::{self, HandPose, Kind, Piece, Pt};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke};
use std::sync::OnceLock;

const VIOLET: [u8; 3] = [138, 92, 245];
const LILAC: [u8; 3] = [196, 168, 255];
const BLACK: [u8; 3] = [10, 4, 22];
const GOLD: [u8; 3] = [232, 199, 106];
const SAND: [[u8; 3]; 3] = [[255, 241, 184], [241, 210, 122], [201, 162, 74]];

fn rgba(c: [u8; 3], a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// The figure and its triangles, worked out once.
fn figure() -> &'static Vec<(Piece, Vec<[usize; 3]>)> {
    static FIGURE: OnceLock<Vec<(Piece, Vec<[usize; 3]>)>> = OnceLock::new();
    FIGURE.get_or_init(|| {
        monarch::figure()
            .into_iter()
            .map(|p| {
                let tris = if p.filled() { monarch::triangulate(&p.pts) } else { Vec::new() };
                (p, tris)
            })
            .collect()
    })
}

/// Draws the whole scene `t` seconds after the moon hit the staff, inside `rect` (the backdrop's
/// area, `area` its size in points). `clock` is only used to make things move.
pub fn paint(painter: &Painter, rect: Rect, area: Pt, t: f32, clock: f32) {
    paint_with(painter, rect, area, t, clock, true, true);
}

/// The same, with or without his typed voice (Game 3's ending writes its own words) and with or
/// without the arms, talons and violet fire that come out of the hole (the ending brings its own).
pub fn paint_with(painter: &Painter, rect: Rect, area: Pt, t: f32, clock: f32, voice: bool, arms: bool) {
    let b = monarch::beat(t);
    let at = |p: Pt| pos2(rect.min.x + p.0, rect.min.y + p.1);
    let rift = monarch::rift(area);
    let centre = at(rift.centre);
    let k = rift.k;

    // the page goes dark
    if b.dim > 0.0 {
        painter.rect_filled(rect, 0.0, rgba([6, 3, 14], b.dim));
    }
    paint_seams(painter, rect, b.seams, b.rift);
    if b.ring > 0.0 {
        paint_ring(painter, centre, 128.0 * k, b.ring, b.ring_turn);
    }
    // the painted pictures, if they could be read
    let art = monarch_art::art(painter.ctx());
    if b.rift > 0.0 {
        match &art {
            Some(a) => paint_rift_art(painter, a, centre, rift.rx, rift.ry, b.rift, clock),
            None => paint_rift(painter, centre, rift.rx, rift.ry, b.rift, clock),
        }
    }

    // him: small and deep in the hole to begin with, snapping towards you in three lurches and then
    // lunging at the screen
    let (origin, scale) = monarch::body_at_beat(area, &b);
    let body_at = |p: Pt| at((origin.0 + p.0 * scale, origin.1 + p.1 * scale));
    if b.body > 0.0 {
        match &art {
            Some(a) => paint_body_art(painter, a, &body_at, scale, b.body, t),
            None => paint_body(painter, &body_at, scale, b.body),
        }
    }

    // his arms come out of the dark of the hole, joined to him, and the talons on them; as he pulls
    // himself through they swing down to hang from his shoulders
    if arms && b.talons > 0.0 {
        let a = b.talons.min(1.0);
        let hands = monarch::hand_poses(area, t);
        for i in 0..2 {
            let (pts, thin, thick) = monarch::arm_points(area, t, i);
            let screen = [at(pts[0]), at(pts[1]), at(pts[2])];
            paint_arm(painter, &screen, thin, thick, a);
            if b.hang > 0.3 {
                // the spike on his elbow
                let spike: Vec<Pos2> = monarch::arms()[i].spike.iter().map(|p| body_at(*p)).collect();
                painter.add(Shape::convex_polygon(spike, rgba(BLACK, a * b.hang), Stroke::new(1.2, rgba(VIOLET, 0.8 * a * b.hang))));
            }
            let pose = hands[i];
            match &art {
                Some(art) => paint_hand_art(painter, art, &at, pose, b.uncurl, a),
                None => {
                    let tf = |p: Pt| at(monarch::place(p, pose.origin, pose.rot, pose.scale, pose.mirror));
                    for piece in monarch::hand(b.uncurl) {
                        draw_piece(painter, &piece, &tf, pose.scale, a);
                    }
                }
            }
        }
        if b.flame > 0.0 {
            let (fire, size) = monarch::fire_now(area, t);
            paint_flame(painter, at(fire), size, b.flame, clock);
        }
    }

    // his eyes: lit in the dark before he shows (the painted faces have their own once he does)
    let eyes = if art.is_some() { b.eyes * (1.0 - b.body) } else { b.eyes };
    if eyes > 0.0 {
        paint_eyes(painter, &body_at, scale, eyes, clock);
    }

    // his voice
    if voice {
        paint_voice(painter, rect, centre, k, &b, clock);
    }
}

// ---- the gold seams and the sand ring ----------------------------------------------------------------

/// Gold sand runs along the lines where the page was torn and stitches them shut.
fn paint_seams(painter: &Painter, rect: Rect, seams: f32, rift: f32) {
    if seams <= 0.0 {
        return;
    }
    let fade = 1.0 - 0.75 * rift;
    for i in 0..monarch::SEAMS {
        let share = monarch::seam_share(i, seams);
        if share <= 0.0 {
            continue;
        }
        let line = monarch::seam(i);
        let n = ((line.len() - 1) as f32 * share).floor() as usize;
        let mut pts: Vec<Pos2> = line[..=n].iter().map(|p| pos2(rect.min.x + p.0 * rect.width(), rect.min.y + p.1 * rect.height())).collect();
        if n + 1 < line.len() {
            // the end of the line is still being drawn: a part of its last step
            let (a, c) = (line[n], line[n + 1]);
            let f = (line.len() - 1) as f32 * share - n as f32;
            pts.push(pos2(rect.min.x + (a.0 + (c.0 - a.0) * f) * rect.width(), rect.min.y + (a.1 + (c.1 - a.1) * f) * rect.height()));
        }
        if pts.len() < 2 {
            continue;
        }
        painter.add(Shape::line(pts.clone(), Stroke::new(7.0, rgba(GOLD, 0.10 * fade))));
        painter.add(Shape::line(pts.clone(), Stroke::new(2.0, rgba(GOLD, 0.85 * fade))));
        // the stitches across it
        for w in pts.windows(2) {
            let mid = w[0].lerp(w[1], 0.5);
            let d = (w[1] - w[0]).normalized();
            let n = vec2(-d.y, d.x) * 4.0;
            painter.line_segment([mid - n, mid + n], Stroke::new(1.2, rgba(SAND[0], 0.7 * fade)));
        }
    }
}

/// The ring of sand that turns the clock back: twelve ticks, two hands going the wrong way, and
/// grains spiralling in.
fn paint_ring(painter: &Painter, c: Pos2, r: f32, a: f32, turn: f32) {
    soft_ellipse(painter, c, vec2(r * 1.25, r * 1.25), GOLD, 0.16 * a);
    let n = 72;
    let ring: Vec<Pos2> = (0..n).map(|i| c + vec2((i as f32 / n as f32 * std::f32::consts::TAU).cos(), (i as f32 / n as f32 * std::f32::consts::TAU).sin()) * r).collect();
    painter.add(Shape::closed_line(ring, Stroke::new(1.4, rgba(GOLD, 0.55 * a))));
    for i in 0..12 {
        let ang = i as f32 / 12.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let d = vec2(ang.cos(), ang.sin());
        let long = if i % 3 == 0 { 12.0 } else { 6.0 };
        painter.line_segment([c + d * (r - long), c + d * r], Stroke::new(1.6, rgba(SAND[1], 0.7 * a)));
    }
    // the hands run backwards
    for (len, speed, w) in [(0.62, 1.0, 2.2), (0.84, 12.0, 1.4)] {
        let ang = turn * speed - std::f32::consts::FRAC_PI_2;
        painter.line_segment([c, c + vec2(ang.cos(), ang.sin()) * r * len], Stroke::new(w, rgba(SAND[0], 0.8 * a)));
    }
    for i in 0..monarch::RING_GRAINS {
        let (p, size, alpha, tone) = monarch::ring_grain(i, turn * 1.6);
        painter.rect_filled(Rect::from_center_size(c + vec2(p.0, p.1) * r, vec2(size, size)), 0.0, rgba(SAND[tone as usize % 3], alpha * a));
    }
}

// ---- the hole ------------------------------------------------------------------------------------

fn wobbly(c: Pos2, rx: f32, ry: f32, wob: f32, clock: f32, shift: f32) -> Vec<Pos2> {
    let n = 56;
    (0..n)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / n as f32;
            let w = 1.0 + wob * ((a * 5.0 + clock * 1.7 + shift).sin() * 0.6 + (a * 9.0 - clock * 2.3).sin() * 0.4);
            c + vec2(a.cos() * rx * w + shift * 3.0, a.sin() * ry * w)
        })
        .collect()
}

/// Fills a shape that can be seen from its middle `c` all the way round (the hole is: it wobbles
/// but is never folded), as a fan of triangles, since egui can only fill convex shapes itself.
fn fan(painter: &Painter, c: Pos2, pts: &[Pos2], colour: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(c, colour);
    for p in pts {
        mesh.colored_vertex(*p, colour);
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(mesh));
}

/// The opening in the middle of space: black, with a violet light round its edge that splits into
/// cyan and magenta.
fn paint_rift(painter: &Painter, c: Pos2, rx: f32, ry: f32, open: f32, clock: f32) {
    let (rx, ry) = (rx * open.max(0.02), ry * open.max(0.02));
    soft_ellipse(painter, c, vec2(rx * 1.5 + 6.0, ry * 1.4 + 6.0), VIOLET, 0.5 * open.sqrt());
    let wob = 0.035 * open;
    fan(painter, c, &wobbly(c, rx, ry, wob, clock, 0.0), rgba([2, 1, 6], 0.97));
    painter.add(Shape::closed_line(wobbly(c, rx, ry, wob, clock, -1.0), Stroke::new(1.6, theme::VOID_CYAN.gamma_multiply(0.7 * open))));
    painter.add(Shape::closed_line(wobbly(c, rx, ry, wob, clock, 1.0), Stroke::new(1.6, theme::VOID_MAGENTA.gamma_multiply(0.7 * open))));
    painter.add(Shape::closed_line(wobbly(c, rx, ry, wob, clock, 0.0), Stroke::new(2.0, rgba(LILAC, 0.8 * open))));
}

// ---- the painted pictures --------------------------------------------------------------------------

const CYAN: [u8; 3] = [63, 240, 255];
const MAGENTA: [u8; 3] = [255, 63, 200];

/// The tear in space: the painted hole, breathing a little, with a cyan and a magenta ghost of it
/// a hair to each side.
fn paint_rift_art(painter: &Painter, art: &Art, c: Pos2, rx: f32, ry: f32, open: f32, clock: f32) {
    let open = open.max(0.02) * (1.0 + 0.012 * (clock * 1.7).sin());
    // the picture is drawn with the hole 122 units wide and 176 high
    let (sx, sy) = (rx / 122.0 * open, ry / 176.0 * open);
    let a = (open * 2.5).min(1.0);
    for (dx, rgb) in [(-2.0, CYAN), (2.0, MAGENTA)] {
        art.rift.draw(painter, &move |p: Pt| pos2(c.x + dx + p.0 * sx, c.y + p.1 * sy), monarch_art::ghost(rgb, 0.3 * a));
    }
    art.rift.draw(painter, &move |p: Pt| pos2(c.x + p.0 * sx, c.y + p.1 * sy), monarch_art::fade(a));
}

/// Him, painted: the glow behind him, his body and cloak, and his head, which is one face until the
/// lunge and tears into another.
fn paint_body_art(painter: &Painter, art: &Art, body_at: &dyn Fn(Pt) -> Pos2, scale: f32, a: f32, t: f32) {
    soft_ellipse(painter, body_at((0.0, 200.0)), vec2(300.0, 380.0) * scale, VIOLET, 0.22 * a);
    art.body.draw(painter, body_at, monarch_art::fade(a));
    paint_head(painter, art, body_at, a, t);
}

/// His head at `t`: the first face, the second, or, while it changes, strips of one and the other
/// knocked sideways with a cyan and a magenta ghost of the new face pulled apart behind them.
fn paint_head(painter: &Painter, art: &Art, body_at: &dyn Fn(Pt) -> Pos2, a: f32, t: f32) {
    let bands = monarch::face_bands(t);
    let chroma = monarch::face_chroma(t);
    if chroma > 0.0 {
        let d = chroma * 16.0;
        for (sign, rgb) in [(-1.0_f32, CYAN), (1.0, MAGENTA)] {
            art.head_b.draw(painter, &move |p: Pt| body_at((p.0 + sign * d, p.1)), monarch_art::ghost(rgb, 0.45 * chroma * a));
        }
    }
    let tint = monarch_art::fade(a);
    if bands.iter().all(|b| *b == bands[0]) && bands[0].dx == 0.0 {
        let face = if bands[0].new { &art.head_b } else { &art.head_a };
        face.draw(painter, body_at, tint);
        return;
    }
    let n = monarch::BANDS as f32;
    for (i, band) in bands.iter().enumerate() {
        let face = if band.new { &art.head_b } else { &art.head_a };
        face.draw_strip(painter, body_at, i as f32 / n, (i as f32 + 1.0) / n, band.dx, tint);
    }
}

/// A painted hand at `pose`: the fingers drawn in tight while it is still coming out of the dark
/// and open once they have uncurled (the two pictures fade into one another).
fn paint_hand_art(painter: &Painter, art: &Art, at: &dyn Fn(Pt) -> Pos2, pose: HandPose, uncurl: f32, a: f32) {
    let u = uncurl.clamp(0.0, 1.0);
    for (sprite, share) in [(&art.hand_curled, 1.0 - u), (&art.hand_open, u)] {
        if share <= 0.01 {
            continue;
        }
        let (x0, y0, x1, y1) = sprite.units();
        let corner = |p: Pt| at(monarch::place(p, pose.origin, pose.rot, pose.scale, pose.mirror));
        sprite.draw_quad(painter, [corner((x0, y0)), corner((x1, y0)), corner((x1, y1)), corner((x0, y1))], monarch_art::fade(a * share));
    }
}

// ---- his voice -----------------------------------------------------------------------------------

/// Types out `text` (`count` letters of it) centred on `x`, so it grows from where it starts rather
/// than sliding as it is typed. A cyan and a magenta ghost sit a point to each side.
fn typed(painter: &Painter, x: f32, y: f32, text: &str, count: usize, alpha: f32, size: f32) {
    if count == 0 || alpha <= 0.0 {
        return;
    }
    let font = FontId::proportional(size);
    let whole = painter.layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE);
    let left = x - whole.size().x / 2.0;
    let shown: String = text.chars().take(count).collect();
    // the rift and the lightning round it are bright, so the line sits on a cloud of black that is
    // as wide as the whole line from the first letter (it does not grow as the line is typed)
    let wide = whole.size().x;
    let high = whole.size().y;
    soft_ellipse(painter, pos2(x, y), vec2(wide * 0.62 + 46.0, high * 2.1 + 16.0), [4, 2, 10], 0.95 * alpha);
    soft_ellipse(painter, pos2(x, y), vec2(wide * 0.5 + 22.0, high * 1.2 + 8.0), [0, 0, 0], 0.9 * alpha);
    for (dx, colour) in [(-1.2, theme::VOID_CYAN.gamma_multiply(0.35 * alpha)), (1.2, theme::VOID_MAGENTA.gamma_multiply(0.35 * alpha))] {
        painter.text(pos2(left + dx, y), Align2::LEFT_CENTER, &shown, font.clone(), colour);
    }
    // a black edge all round the letters, then the letters, a hair apart so they read heavier
    for (dx, dy) in [(-1.5, 0.0), (1.5, 0.0), (0.0, -1.5), (0.0, 1.5), (-1.1, -1.1), (1.1, 1.1), (-1.1, 1.1), (1.1, -1.1)] {
        painter.text(pos2(left + dx, y + dy), Align2::LEFT_CENTER, &shown, font.clone(), rgba([0, 0, 0], 0.85 * alpha));
    }
    for dx in [0.0, 0.6] {
        painter.text(pos2(left + dx, y), Align2::LEFT_CENTER, &shown, font.clone(), rgba([255, 252, 255], alpha));
    }
}

fn paint_voice(painter: &Painter, rect: Rect, centre: Pos2, k: f32, b: &monarch::Beat, _clock: f32) {
    let size = (17.0 * k).clamp(14.0, 22.0);
    let (n1, a1) = b.line1;
    typed(painter, centre.x, centre.y - 118.0 * k, monarch::LINE1, n1, a1, size);
    let (n2, a2) = b.line2;
    typed(painter, centre.x, rect.max.y - 26.0 * k.max(0.7), monarch::LINE2, n2, a2, size);
}

// ---- him -----------------------------------------------------------------------------------------

fn paint_eyes(painter: &Painter, body_at: &dyn Fn(Pt) -> Pos2, scale: f32, a: f32, clock: f32) {
    let flicker = 0.9 + 0.1 * (clock * 9.0).sin();
    for eye in monarch::eyes() {
        let c = body_at(eye.centre);
        let rgb = if eye.cyan { [63, 240, 255] } else { [255, 63, 200] };
        soft_ellipse(painter, c, vec2(30.0, 22.0) * (scale * 2.0).max(0.5), rgb, 0.55 * a * flicker);
        let tri: Vec<Pos2> = eye.slit.iter().map(|p| body_at(*p)).collect();
        painter.add(Shape::convex_polygon(tri, rgba([250, 246, 255], a), Stroke::new(1.0, rgba(rgb, a))));
    }
}

/// Draws him, without his arms and his eyes, at `a`.
fn paint_body(painter: &Painter, body_at: &dyn Fn(Pt) -> Pos2, scale: f32, a: f32) {
    let heart = body_at((0.0, 200.0));
    soft_ellipse(painter, heart, vec2(300.0, 380.0) * scale, VIOLET, 0.22 * a);
    for (piece, tris) in figure() {
        draw_piece_with(painter, piece, tris, body_at, scale, a);
    }
}

/// An arm through three points, `w0` wide at the shoulder and `w1` at the wrist.
pub fn paint_arm(painter: &Painter, pts: &[Pos2; 3], w0: f32, w1: f32, a: f32) {
    if a <= 0.0 {
        return;
    }
    let widths = [w0, (w0 + w1) / 2.0, w1];
    for (colour, grow) in [(rgba(VIOLET, 0.75 * a), 3.2), (rgba(BLACK, a), 0.0)] {
        for i in 0..2 {
            let (p, q) = (pts[i], pts[i + 1]);
            let along = q - p;
            if along.length() < 0.5 {
                continue;
            }
            let side = vec2(-along.y, along.x).normalized();
            let (hp, hq) = ((widths[i] + grow) / 2.0, (widths[i + 1] + grow) / 2.0);
            painter.add(Shape::convex_polygon(vec![p + side * hp, q + side * hq, q - side * hq, p - side * hp], colour, Stroke::NONE));
        }
        for (p, w) in pts.iter().zip(widths) {
            painter.circle_filled(*p, (w + grow) / 2.0, colour);
        }
    }
}

// ---- the violet fire -----------------------------------------------------------------------------

/// A flame of violet magic hanging above a palm at `at`, `size` points across, burning at `a`.
fn paint_flame(painter: &Painter, at: Pos2, size: f32, a: f32, clock: f32) {
    if a <= 0.0 {
        return;
    }
    let flick = |i: u32, speed: f32| ((clock * speed + i as f32 * 1.7).sin() + (clock * speed * 1.9 + i as f32 * 0.6).sin()) * 0.5;
    soft_ellipse(painter, at - vec2(0.0, size * 14.0), vec2(size * 60.0, size * 60.0), VIOLET, 0.55 * a);
    soft_ellipse(painter, at - vec2(0.0, size * 10.0), vec2(size * 34.0, size * 40.0), [190, 140, 255], 0.5 * a);
    // tongues, outer to inner
    for (layer, (rgb, width, height, alpha)) in [([120, 60, 230], 1.0, 1.0, 0.8), ([176, 124, 255], 0.7, 0.8, 0.85), ([244, 232, 255], 0.38, 0.55, 0.9)].into_iter().enumerate() {
        for i in 0..5u32 {
            let off = (i as f32 - 2.0) * 7.0 * size * width;
            let sway = flick(i + layer as u32 * 7, 5.0) * 5.0 * size;
            let tall = (22.0 + 9.0 * (2.0 - (i as f32 - 2.0).abs())) * size * height * (1.0 + 0.18 * flick(i + 3, 7.0));
            let w = 6.5 * size * width;
            let tri = vec![at + vec2(off - w, 0.0), at + vec2(off + w, 0.0), at + vec2(off + sway, -tall)];
            painter.add(Shape::convex_polygon(tri, rgba(rgb, alpha * a), Stroke::NONE));
        }
    }
    // sparks rising from it
    for i in 0..9u32 {
        let life = ((clock * 0.8 + i as f32 * 0.37).fract()).clamp(0.0, 1.0);
        let x = ((i as f32 * 2.399).sin()) * 14.0 * size + flick(i, 3.0) * 4.0 * size;
        let p = at + vec2(x, -life * 62.0 * size - 6.0 * size);
        painter.rect_filled(Rect::from_center_size(p, vec2(2.2, 2.2) * size.max(0.6)), 0.0, rgba(LILAC, (1.0 - life) * 0.9 * a));
    }
}

// ---- drawing the shapes --------------------------------------------------------------------------

/// A thick dark limb along `pts`, rimmed in violet, rounded at the joints.
fn paint_limb(painter: &Painter, pts: &[Pos2], width: f32, a: f32) {
    if a <= 0.0 || pts.len() < 2 {
        return;
    }
    let rim = 1.6;
    painter.add(Shape::line(pts.to_vec(), Stroke::new(width + rim * 2.0, rgba(VIOLET, 0.75 * a))));
    for p in pts {
        painter.circle_filled(*p, width / 2.0 + rim, rgba(VIOLET, 0.75 * a));
    }
    painter.add(Shape::line(pts.to_vec(), Stroke::new(width, rgba(BLACK, a))));
    for p in pts {
        painter.circle_filled(*p, width / 2.0, rgba(BLACK, a));
    }
}

/// A piece with its triangles worked out here (for hands, which change).
fn draw_piece(painter: &Painter, piece: &Piece, tf: &dyn Fn(Pt) -> Pos2, sc: f32, a: f32) {
    let tris = if piece.filled() { monarch::triangulate(&piece.pts) } else { Vec::new() };
    draw_piece_with(painter, piece, &tris, tf, sc, a);
}

/// Draws one piece of him. `tf` puts a point of the figure on the screen, `sc` is how many points a
/// unit of the figure is (for line widths), `a` is how visible.
fn draw_piece_with(painter: &Painter, piece: &Piece, tris: &[[usize; 3]], tf: &dyn Fn(Pt) -> Pos2, sc: f32, a: f32) {
    if a <= 0.0 {
        return;
    }
    let pts: Vec<Pos2> = piece.pts.iter().map(|p| tf(*p)).collect();
    let fill = |painter: &Painter, colour: &dyn Fn(usize) -> Color32| {
        if tris.is_empty() {
            return;
        }
        let mut mesh = Mesh::default();
        for (i, p) in pts.iter().enumerate() {
            mesh.colored_vertex(*p, colour(i));
        }
        for t in tris {
            mesh.add_triangle(t[0] as u32, t[1] as u32, t[2] as u32);
        }
        painter.add(Shape::mesh(mesh));
    };
    match piece.kind {
        Kind::Solid => {
            fill(painter, &|_| rgba(BLACK, a));
            painter.add(Shape::closed_line(pts, Stroke::new((1.6 * sc * 2.0).clamp(0.8, 2.0), rgba(VIOLET, 0.65 * a))));
        }
        Kind::Cloak => {
            // dark at the shoulders, thinning to nothing towards the hem
            let (top, bottom) = (piece.pts.iter().map(|p| p.1).fold(f32::MAX, f32::min), piece.pts.iter().map(|p| p.1).fold(f32::MIN, f32::max));
            fill(painter, &|i| {
                let f = ((piece.pts[i].1 - top) / (bottom - top).max(1.0)).clamp(0.0, 1.0);
                rgba([14, 6, 30], a * (0.97 - 0.85 * f))
            });
        }
        Kind::Crown => {
            fill(painter, &|_| rgba(BLACK, a));
            painter.add(Shape::closed_line(pts, Stroke::new(1.4, rgba([232, 224, 255], 0.85 * a))));
        }
        Kind::Claw => fill(painter, &|_| rgba([236, 226, 255], a)),
        Kind::Limb(w) => paint_limb(painter, &pts, w * sc, a),
        Kind::Thin(w) => {
            painter.add(Shape::line(pts, Stroke::new((w * sc * 2.0).clamp(0.7, 4.0), rgba([160, 120, 255], 0.6 * a))));
        }
        Kind::Teeth => {
            painter.add(Shape::line(pts, Stroke::new(1.0, rgba([240, 235, 255], 0.85 * a))));
        }
    }
}

/// A tiny hint, once the scene has been going a while, that it can be skipped.
pub fn paint_hint(painter: &Painter, rect: Rect, t: f32) {
    let a = ((t - 2.5) / 1.0).clamp(0.0, 1.0) * 0.35;
    if a <= 0.0 || t > monarch::END - 3.0 {
        return;
    }
    painter.text(rect.right_bottom() - vec2(10.0, 8.0), Align2::RIGHT_BOTTOM, "click or press a key to skip", FontId::proportional(11.0), rgba(LILAC, a));
}
