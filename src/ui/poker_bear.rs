// The Demon Bear at the Hold'em table, drawn by hand from the design boards: a towering, scarred
// grizzly in the Underworld's charred browns and lava orange, hunched over the table, breathing
// slowly. One eye is a prism that bleeds (see poker_react.rs for when). When he wins big he rises
// with his arms thrown wide, flexing, and roars with a grin and a wink; when he is beaten he
// comes down on his elbows with his claws dug into the table and roars to kill.
//
// Everything is in the design board's own units (an 800 x 680 picture with the table's edge at the
// bottom): `Bear::at` puts the bottom middle of that picture where you want it, and `scale` says
// how many points one unit is. Only polygons, lines and circles are used (no pictures), and the
// fur is made ragged by moving the outline's points a little.
use super::backdrop::soft_ellipse;
use eframe::egui::{pos2, vec2, Color32, Mesh, Painter, Pos2, Shape, Stroke};
use std::f32::consts::TAU;

/// What he is doing with his body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    /// Sat at the table, hunched, breathing.
    Seated,
    /// "I win!": risen, arms wide and flexed, a grin and a wink.
    Joy,
    /// "I'll kill you all!": elbows out, claws in the table.
    Rage,
}

pub struct Bear {
    /// Where the bottom middle of the picture (the table's edge) goes.
    pub at: Pos2,
    /// Points per unit of the picture.
    pub scale: f32,
    /// Seconds since the program started: drives the breathing and the flicker.
    pub t: f32,
    pub mood: Mood,
    /// Seconds since `mood` began (for the wink and the roar).
    pub since: f32,
    /// The prism's eye is bleeding.
    pub bleeding: bool,
    /// He is making up his mind (0 to 1): the brows come down and the eye looks about.
    pub think: f32,
}

const FUR_LIGHT: [u8; 3] = [52, 32, 26];
const FUR_MID: [u8; 3] = [26, 15, 11];
const FUR_DARK: [u8; 3] = [8, 5, 4];
const EMBER: [u8; 3] = [255, 106, 26];
const BONE: [u8; 3] = [239, 227, 208];
const ROSE: [u8; 3] = [255, 138, 106];
const SCAR: [u8; 3] = [216, 201, 174];

pub(super) fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

pub(super) fn rgba(c: [u8; 3], a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

fn hash(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    ((x.wrapping_mul(0x297A_2D39) >> 8) & 0xFFFF) as f32 / 65535.0
}

pub(super) fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

pub(super) type Pt = (f32, f32);

/// Splits every edge of a closed outline into pieces and shakes the new points about, so it looks
/// like fur rather than a cut shape.
pub(super) fn ragged(points: &[Pt], pieces: usize, amp: f32, seed: u32) -> Vec<Pt> {
    let mut out = Vec::new();
    let n = points.len();
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        for k in 0..pieces {
            let f = k as f32 / pieces as f32;
            let (x, y) = (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
            let id = (i * pieces + k) as u32;
            let (dx, dy) = ((hash(id, seed) - 0.5) * 2.0 * amp, (hash(id, seed + 99) - 0.5) * 2.0 * amp);
            out.push((x + dx, y + dy));
        }
    }
    out
}

/// A quad for a limb from `p0` (width `w0`) to `p1` (width `w1`), with round ends added by the caller.
fn limb_quad(p0: Pt, p1: Pt, w0: f32, w1: f32) -> [Pt; 4] {
    let (dx, dy) = (p1.0 - p0.0, p1.1 - p0.1);
    let n = (dx * dx + dy * dy).sqrt().max(0.001);
    let (nx, ny) = (-dy / n, dx / n);
    [
        (p0.0 + nx * w0 / 2.0, p0.1 + ny * w0 / 2.0),
        (p1.0 + nx * w1 / 2.0, p1.1 + ny * w1 / 2.0),
        (p1.0 - nx * w1 / 2.0, p1.1 - ny * w1 / 2.0),
        (p0.0 - nx * w0 / 2.0, p0.1 - ny * w0 / 2.0),
    ]
}

/// An ellipse as points, turned by `deg` degrees.
pub(super) fn ellipse(c: Pt, rx: f32, ry: f32, deg: f32, n: usize) -> Vec<Pt> {
    let (s, co) = deg.to_radians().sin_cos();
    (0..n)
        .map(|i| {
            let a = TAU * i as f32 / n as f32;
            let (x, y) = (a.cos() * rx, a.sin() * ry);
            (c.0 + x * co - y * s, c.1 + x * s + y * co)
        })
        .collect()
}

pub(super) fn lerp(a: Pt, b: Pt, t: f32) -> Pt {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

pub(super) fn norm(a: Pt, b: Pt) -> Pt {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let n = (dx * dx + dy * dy).sqrt().max(0.001);
    (-dy / n, dx / n)
}

/// A point `t` of the way from `a` to `b` and `off` to the side (units).
pub(super) fn along(a: Pt, b: Pt, t: f32, off: f32) -> Pt {
    let p = lerp(a, b, t);
    let n = norm(a, b);
    (p.0 + n.0 * off, p.1 + n.1 * off)
}

pub(super) struct Draw<'a> {
    pub(super) painter: &'a Painter,
    pub(super) at: Pos2,
    pub(super) s: f32,
    /// The picture's own origin is (400, 640): the table's edge, in the middle.
    pub(super) ox: f32,
}

impl Draw<'_> {
    pub(super) fn m(&self, p: Pt) -> Pos2 {
        pos2(self.at.x + (p.0 - self.ox) * self.s, self.at.y + (p.1 - 640.0) * self.s)
    }

    pub(super) fn pts(&self, v: &[Pt]) -> Vec<Pos2> {
        v.iter().map(|&p| self.m(p)).collect()
    }

    /// A shape that is all round its middle, filled flat.
    pub(super) fn fan(&self, v: &[Pt], col: Color32) {
        let p = self.pts(v);
        fan(self.painter, &p, col);
    }

    /// The same with a lighter middle: `mid` at `centre`, `edge` at the edge.
    pub(super) fn glow_fan(&self, v: &[Pt], centre: Pt, mid: Color32, edge: Color32) {
        let c = self.m(centre);
        let p = self.pts(v);
        let mut mesh = Mesh::default();
        mesh.colored_vertex(c, mid);
        for q in &p {
            mesh.colored_vertex(*q, edge);
        }
        let n = p.len() as u32;
        for i in 0..n {
            mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
        }
        self.painter.add(Shape::mesh(mesh));
    }

    pub(super) fn outline(&self, v: &[Pt], col: Color32, w: f32) {
        self.painter.add(Shape::closed_line(self.pts(v), Stroke::new((w * self.s).max(0.8), col)));
    }

    pub(super) fn line(&self, v: &[Pt], col: Color32, w: f32) {
        self.painter.add(Shape::line(self.pts(v), Stroke::new((w * self.s).max(0.7), col)));
    }

    pub(super) fn quad(&self, q: [Pt; 4], col: Color32) {
        self.painter.add(Shape::convex_polygon(self.pts(&q), col, Stroke::NONE));
    }

    pub(super) fn dot(&self, c: Pt, r: f32, col: Color32) {
        self.painter.circle_filled(self.m(c), (r * self.s).max(0.6), col);
    }

    pub(super) fn soft(&self, c: Pt, rx: f32, ry: f32, col: [u8; 3], a: f32) {
        soft_ellipse(self.painter, self.m(c), vec2(rx * self.s, ry * self.s), col, a);
    }

    /// A limb: a quad with round ends.
    pub(super) fn limb(&self, p0: Pt, p1: Pt, w0: f32, w1: f32, col: Color32, rim: Color32) {
        let q = limb_quad(p0, p1, w0, w1);
        self.quad(q, col);
        self.dot(p0, w0 / 2.0, col);
        self.dot(p1, w1 / 2.0, col);
        self.painter.add(Shape::line_segment([self.m(q[0]), self.m(q[1])], Stroke::new((2.2 * self.s).max(0.8), rim)));
        self.painter.add(Shape::line_segment([self.m(q[3]), self.m(q[2])], Stroke::new((2.2 * self.s).max(0.8), rim)));
    }
}

pub(super) fn fan(painter: &Painter, p: &[Pos2], col: Color32) {
    if p.len() < 3 {
        return;
    }
    let mut centre = vec2(0.0, 0.0);
    for q in p {
        centre += q.to_vec2();
    }
    let centre = (centre / p.len() as f32).to_pos2();
    let mut mesh = Mesh::default();
    mesh.colored_vertex(centre, col);
    for q in p {
        mesh.colored_vertex(*q, col);
    }
    let n = p.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(mesh));
}

/// Draws the Bear.
pub fn paint(painter: &Painter, b: &Bear) {
    let t = b.t;
    // the breathing: slow, even and relaxed
    let slow = 0.5 - 0.5 * (TAU * t / 5.6).cos();
    let heave = match b.mood {
        Mood::Seated => 0.014 * slow,
        _ => 0.02 * (0.5 + 0.5 * (TAU * 1.4 * t).sin()),
    };
    // everything above the table breathes: it grows from the table's edge
    let grow = 1.0 + heave;
    let d = Draw { painter, at: pos2(b.at.x, b.at.y), s: b.scale * grow, ox: 400.0 };

    paint_back(&d, b.mood);
    match b.mood {
        Mood::Seated => {
            paint_head(&d, b, 52.0 + 5.0 * slow, 0.0);
            paint_paws(&d);
        }
        Mood::Joy => {
            paint_torso(&d);
            paint_arms_joy(&d, t);
            paint_head(&d, b, -30.0, 1.0);
        }
        Mood::Rage => {
            paint_torso(&d);
            paint_arms_rage(&d, t);
            paint_head(&d, b, -6.0, 0.7);
        }
    }
}

// ---- the body -----------------------------------------------------------------------------------

fn paint_back(d: &Draw, mood: Mood) {
    let outline: Vec<Pt> = match mood {
        // hunched: a dome of a back, over the head
        Mood::Seated => vec![
            (-20.0, 680.0),
            (-4.0, 540.0),
            (24.0, 420.0),
            (90.0, 330.0),
            (140.0, 262.0),
            (210.0, 214.0),
            (290.0, 188.0),
            (330.0, 160.0),
            (370.0, 148.0),
            (400.0, 146.0),
            (430.0, 148.0),
            (470.0, 160.0),
            (510.0, 188.0),
            (590.0, 214.0),
            (660.0, 262.0),
            (710.0, 330.0),
            (776.0, 420.0),
            (804.0, 540.0),
            (820.0, 680.0),
        ],
        // risen: shoulders and lats flaring from a narrow waist
        _ => vec![
            (170.0, 690.0),
            (128.0, 590.0),
            (66.0, 500.0),
            (58.0, 410.0),
            (56.0, 346.0),
            (108.0, 298.0),
            (170.0, 270.0),
            (230.0, 238.0),
            (290.0, 206.0),
            (340.0, 190.0),
            (370.0, 150.0),
            (430.0, 150.0),
            (460.0, 190.0),
            (510.0, 206.0),
            (570.0, 238.0),
            (630.0, 270.0),
            (692.0, 298.0),
            (744.0, 346.0),
            (742.0, 410.0),
            (734.0, 500.0),
            (672.0, 590.0),
            (630.0, 690.0),
        ],
    };
    let rough = ragged(&outline, 4, 5.0, 11);
    d.glow_fan(&rough, (400.0, 260.0), rgba(FUR_LIGHT, 1.0), rgba(FUR_DARK, 1.0));
    d.outline(&rough, rgba(EMBER, 0.8), 3.2);
    // the trapezius sloping up to the neck, the shoulder blades
    let trap = [(232.0, 340.0), (256.0, 262.0), (326.0, 196.0), (400.0, 190.0), (474.0, 196.0), (544.0, 262.0), (568.0, 340.0), (520.0, 306.0), (460.0, 296.0), (400.0, 296.0), (340.0, 296.0), (280.0, 306.0)];
    d.fan(&trap, rgba([58, 34, 26], 0.95));
    d.outline(&trap, rgba([255, 138, 58], 0.45), 1.6);
    if mood == Mood::Seated {
        for side in [-1.0_f32, 1.0] {
            let blade: Vec<Pt> = [(64.0, 480.0), (40.0, 410.0), (66.0, 336.0), (138.0, 298.0), (188.0, 276.0), (232.0, 300.0), (238.0, 352.0), (242.0, 414.0), (202.0, 474.0), (142.0, 504.0), (102.0, 518.0)]
                .iter()
                .map(|&(px, py)| (if side < 0.0 { px } else { 800.0 - px }, py))
                .collect();
            d.fan(&blade, rgba([46, 27, 20], 0.95));
            d.outline(&blade, rgba([255, 138, 58], 0.45), 1.6);
        }
    }
    // the spine and the old claw rakes
    d.line(&[(400.0, 190.0), (400.0, 296.0)], rgba([10, 5, 4], 0.75), 2.6);
    d.line(&[(372.0, 196.0), (360.0, 230.0), (350.0, 262.0), (340.0, 298.0)], rgba([10, 5, 4], 0.7), 2.4);
    d.line(&[(428.0, 196.0), (440.0, 230.0), (450.0, 262.0), (460.0, 298.0)], rgba([10, 5, 4], 0.7), 2.4);
    for (a, b) in [
        ([(96.0, 380.0), (130.0, 360.0), (120.0, 412.0), (160.0, 440.0)], 0),
        ([(700.0, 340.0), (668.0, 364.0), (678.0, 410.0), (644.0, 446.0)], 1),
    ] {
        let _ = b;
        d.line(&a, rgba(SCAR, 0.7), 2.2);
    }
}

fn paint_torso(d: &Draw) {
    let skin = [(120.0, 310.0), (180.0, 285.0), (260.0, 275.0), (400.0, 272.0), (540.0, 275.0), (620.0, 285.0), (680.0, 310.0), (650.0, 420.0), (585.0, 644.0), (215.0, 644.0), (150.0, 420.0)];
    d.glow_fan(&ragged(&skin, 3, 2.5, 5), (400.0, 360.0), rgba([60, 34, 26], 1.0), rgba([18, 10, 7], 1.0));
    d.outline(&skin, rgba(EMBER, 0.75), 2.4);
    // pecs
    let pec_l = [(190.0, 360.0), (250.0, 335.0), (340.0, 345.0), (394.0, 372.0), (394.0, 450.0), (330.0, 482.0), (240.0, 472.0), (196.0, 420.0)];
    let pec_r: Vec<Pt> = pec_l.iter().map(|&(x, y)| (800.0 - x, y)).collect();
    for p in [&pec_l[..], &pec_r[..]] {
        d.fan(p, rgba([58, 33, 24], 1.0));
        d.outline(p, rgba([255, 138, 58], 0.7), 1.6);
    }
    d.line(&[(204.0, 428.0), (252.0, 466.0), (332.0, 470.0), (390.0, 446.0)], rgba([10, 5, 4], 0.75), 3.5);
    d.line(&[(596.0, 428.0), (548.0, 466.0), (468.0, 470.0), (410.0, 446.0)], rgba([10, 5, 4], 0.75), 3.5);
    // sternum and abs
    d.line(&[(400.0, 372.0), (400.0, 644.0)], rgba([10, 5, 4], 0.8), 3.5);
    for (i, y) in [500.0_f32, 548.0, 596.0].iter().enumerate() {
        let w = 46.0 - i as f32 * 2.0;
        for sx in [348.0_f32, 406.0] {
            let r = [(sx, *y), (sx + w, *y), (sx + w, *y + 40.0), (sx, *y + 40.0)];
            d.fan(&r, rgba([43, 25, 18], 1.0));
            d.outline(&r, rgba([255, 122, 42], 0.55), 1.4);
        }
    }
    // the old stitched wound down the belly and the rakes across the chest
    d.line(&[(476.0, 512.0), (492.0, 560.0), (480.0, 604.0), (498.0, 636.0)], rgba(SCAR, 0.75), 2.2);
    d.line(&[(214.0, 340.0), (262.0, 392.0), (252.0, 428.0), (318.0, 486.0)], rgba(SCAR, 0.8), 2.4);
    d.line(&[(240.0, 330.0), (286.0, 380.0), (276.0, 416.0), (340.0, 470.0)], rgba(SCAR, 0.8), 2.4);
}

// ---- arms ---------------------------------------------------------------------------------------

fn claw(d: &Draw, base: Pt, tip: Pt, w: f32, glow: f32) {
    let n = norm(base, tip);
    let tri = [(base.0 + n.0 * w / 2.0, base.1 + n.1 * w / 2.0), tip, (base.0 - n.0 * w / 2.0, base.1 - n.1 * w / 2.0)];
    // bone at the root, rose towards the tip
    let mut mesh = Mesh::default();
    mesh.colored_vertex(d.m(tri[0]), rgb(BONE));
    mesh.colored_vertex(d.m(tri[1]), rgb(ROSE));
    mesh.colored_vertex(d.m(tri[2]), rgb(BONE));
    mesh.add_triangle(0, 1, 2);
    d.painter.add(Shape::mesh(mesh));
    d.painter.add(Shape::closed_line(d.pts(&tri), Stroke::new((1.0 * d.s).max(0.7), rgba([255, 106, 74], 0.7 * glow))));
}

fn arm_side(p: Pt, mirror: bool) -> Pt {
    if mirror {
        (800.0 - p.0, p.1)
    } else {
        p
    }
}

/// The big flexed arm of the "I win!" pose; `mirror` is the right one.
fn arm_joy(d: &Draw, mirror: bool, t: f32) {
    let f = |p: Pt| arm_side(p, mirror);
    let (sh, el, wr) = (f((180.0, 322.0)), f((40.0, 250.0)), f((-40.0, 174.0)));
    let fur = rgba(FUR_MID, 1.0);
    let rim = rgba(EMBER, 0.85);
    d.limb(sh, el, 104.0, 70.0, fur, rim);
    d.limb(el, wr, 70.0, 48.0, fur, rim);
    d.dot(sh, 70.0, fur);
    let muscle = rgba([138, 78, 52], 1.0);
    // the deltoid, then the biceps standing up off the arm
    d.fan(&ellipse(f((172.0, 306.0)), 54.0, 46.0, 0.0, 20), muscle);
    let ang = (el.1 - sh.1).atan2(el.0 - sh.0).to_degrees();
    let bi = along(sh, el, 0.5, if mirror { -24.0 } else { 24.0 });
    d.fan(&ellipse(bi, 68.0, 30.0, ang, 22), muscle);
    let tri = along(sh, el, 0.5, if mirror { 26.0 } else { -26.0 });
    d.fan(&ellipse(tri, 64.0, 22.0, ang, 20), rgba([42, 23, 15], 1.0));
    d.line(&[along(sh, el, 0.15, 6.0), along(sh, el, 0.5, 12.0), along(sh, el, 0.85, 6.0)], rgba([10, 5, 4], 0.8), 3.4);
    d.line(&[along(sh, el, 0.2, 44.0), along(sh, el, 0.5, 52.0), along(sh, el, 0.8, 44.0)], rgba([255, 217, 160], 0.45), 2.4);
    // the forearm, the cords, the veins that throb
    let ang2 = (wr.1 - el.1).atan2(wr.0 - el.0).to_degrees();
    d.fan(&ellipse(along(el, wr, 0.3, 8.0), 52.0, 24.0, ang2, 20), muscle);
    d.line(&[along(el, wr, 0.15, -10.0), along(el, wr, 0.55, -6.0), along(el, wr, 0.95, -6.0)], rgba([10, 5, 4], 0.7), 2.2);
    d.line(&[along(el, wr, 0.2, 22.0), along(el, wr, 0.55, 18.0), along(el, wr, 0.9, 16.0)], rgba([10, 5, 4], 0.65), 2.0);
    let throb = 0.6 + 0.4 * (TAU * 1.2 * t).sin();
    d.line(&[along(sh, el, 0.1, 34.0), along(sh, el, 0.5, 34.0), along(sh, el, 0.95, 30.0)], rgba([196, 32, 46], throb), 2.0);
    // the paw, claws up and spread
    let c = f((-52.0, 152.0));
    let paw = ellipse((c.0, c.1 + 10.0), 38.0, 27.0, 0.0, 20);
    d.fan(&paw, rgba([28, 16, 12], 1.0));
    d.outline(&paw, rgba([168, 97, 47], 1.0), 1.6);
    let toes: [(f32, f32, f32); 4] = [(-27.0, -14.0, 12.0), (-9.0, -24.0, 12.0), (11.0, -24.0, 12.0), (29.0, -12.0, 11.0)];
    for (i, (dx, dy, r)) in toes.iter().enumerate() {
        let dx = if mirror { -dx } else { *dx };
        d.fan(&ellipse((c.0 + dx, c.1 + dy), *r, r + 4.0, 0.0, 12), rgba([28, 16, 12], 1.0));
        let lean: f32 = if mirror { 5.0 } else { -5.0 };
        let tip = (c.0 + dx * 1.5 + lean * (i as f32 - 1.5) * 0.4, c.1 + dy - 78.0 - (i % 2) as f32 * 14.0);
        claw(d, (c.0 + dx, c.1 + dy - 8.0), tip, 18.0, 1.0);
    }
}

/// The "I'll kill you all!" arm: elbow out, forearm down, claws in the table.
fn arm_rage(d: &Draw, mirror: bool, t: f32) {
    let f = |p: Pt| arm_side(p, mirror);
    // the raking: a small slow drag back and forth
    let drag = (TAU * 0.9 * t).sin();
    let (sh, el, wr) = (f((176.0, 336.0)), f((70.0, 462.0)), f((186.0 + 3.0 * drag, 614.0 + 4.0 * drag.abs())));
    let fur = rgba(FUR_MID, 1.0);
    let rim = rgba(EMBER, 0.85);
    d.limb(sh, el, 104.0, 72.0, fur, rim);
    d.limb(el, wr, 78.0, 52.0, fur, rim);
    d.dot(sh, 68.0, fur);
    let muscle = rgba([138, 78, 52], 1.0);
    d.fan(&ellipse(f((170.0, 322.0)), 52.0, 44.0, 0.0, 20), muscle);
    let a1 = (el.1 - sh.1).atan2(el.0 - sh.0).to_degrees();
    d.fan(&ellipse(along(sh, el, 0.55, if mirror { 20.0 } else { -20.0 }), 66.0, 24.0, a1, 20), rgba([42, 23, 15], 1.0));
    d.fan(&ellipse(along(sh, el, 0.5, if mirror { -18.0 } else { 18.0 }), 70.0, 30.0, a1, 22), muscle);
    d.line(&[along(sh, el, 0.2, 0.0), along(sh, el, 0.5, 6.0), along(sh, el, 0.85, 0.0)], rgba([10, 5, 4], 0.8), 3.2);
    let a2 = (wr.1 - el.1).atan2(wr.0 - el.0).to_degrees();
    d.fan(&ellipse(along(el, wr, 0.35, 0.0), 70.0, 30.0, a2, 22), muscle);
    d.line(&[along(el, wr, 0.15, -14.0), along(el, wr, 0.55, -10.0), along(el, wr, 0.95, -8.0)], rgba([10, 5, 4], 0.7), 2.2);
    d.line(&[along(el, wr, 0.2, 24.0), along(el, wr, 0.55, 20.0), along(el, wr, 0.9, 18.0)], rgba([10, 5, 4], 0.65), 2.0);
    let throb = 0.6 + 0.4 * (TAU * 2.4 * t).sin();
    d.line(&[along(el, wr, 0.1, 18.0), along(el, wr, 0.5, 14.0), along(el, wr, 0.9, 12.0)], rgba([196, 32, 46], throb), 1.8);
    // a little fur on the elbow, bristling back
    let tuft: Vec<Pt> = [(62.0, 426.0), (26.0, 414.0), (46.0, 440.0), (14.0, 448.0), (44.0, 460.0), (20.0, 482.0), (54.0, 478.0), (42.0, 506.0), (72.0, 490.0), (84.0, 516.0), (92.0, 486.0)]
        .iter()
        .map(|&p| f(p))
        .collect();
    d.fan(&tuft, rgba([44, 26, 18], 1.0));
    d.outline(&tuft, rgba([168, 97, 47], 0.9), 1.3);
    // the paw planted on the table and the claws dug in, with gouges trailing off it
    let px = if mirror { 800.0 - wr.0 } else { wr.0 };
    let (cx, cy) = (wr.0, wr.1 + 8.0);
    let _ = px;
    let paw = ellipse((cx, cy - 4.0), 46.0, 24.0, 0.0, 20);
    d.fan(&paw, rgba([28, 16, 12], 1.0));
    d.outline(&paw, rgba([168, 97, 47], 1.0), 1.6);
    let phase = (t * 0.9).rem_euclid(1.0);
    for (i, dx) in [-30.0_f32, -10.0, 10.0, 30.0].iter().enumerate() {
        let x = cx + if mirror { -dx } else { *dx };
        d.fan(&ellipse((x, cy + 14.0), 12.0, 12.0, 0.0, 12), rgba([28, 16, 12], 1.0));
        claw(d, (x, cy + 10.0), (x, cy + 56.0), 16.0, 1.0);
        // the gouge: grows down the table and fades
        let g = (phase + i as f32 * 0.08).min(1.0);
        d.line(&[(x, cy + 40.0), (x - 2.0, cy + 40.0 + 26.0 * g), (x + 2.0, cy + 40.0 + 50.0 * g)], rgba([194, 138, 80], 0.9 - 0.5 * g), 3.0);
    }
}

fn paint_arms_joy(d: &Draw, t: f32) {
    arm_joy(d, false, t);
    arm_joy(d, true, t);
}

fn paint_arms_rage(d: &Draw, t: f32) {
    arm_rage(d, false, t);
    arm_rage(d, true, t);
    // splinters flying off the table
    for i in 0..8 {
        let ph = (t * 0.9 + i as f32 * 0.17).rem_euclid(1.0);
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let x = if i < 4 { 190.0 } else { 610.0 } + (i % 4) as f32 * 16.0 - 24.0;
        let p = (x + side * 36.0 * ph, 640.0 - 46.0 * ph + 60.0 * ph * ph);
        d.dot(p, 4.0 * (1.0 - 0.6 * ph), rgba([224, 189, 136], 1.0 - ph));
    }
}

// ---- the paws on the table (seated) ------------------------------------------------------------

fn paint_paws(d: &Draw) {
    let left = [(108.0, 640.0), (94.0, 590.0), (130.0, 548.0), (205.0, 542.0), (270.0, 538.0), (322.0, 562.0), (330.0, 640.0)];
    let right: Vec<Pt> = left.iter().map(|&(x, y)| (800.0 - x, y)).collect();
    for p in [&left[..], &right[..]] {
        d.glow_fan(&ragged(p, 4, 2.4, 21), (220.0, 590.0), rgba(FUR_LIGHT, 1.0), rgba(FUR_DARK, 1.0));
        d.outline(p, rgba(EMBER, 0.85), 2.4);
    }
    for (xs, ys) in [([150.0_f32, 196.0, 246.0, 294.0], [554.0_f32, 541.0, 539.0, 551.0])] {
        for side in [false, true] {
            for i in 0..4 {
                let x = if side { 800.0 - xs[i] } else { xs[i] };
                d.fan(&ellipse((x, ys[i]), 19.0, 18.0, 0.0, 14), rgba([28, 16, 12], 1.0));
                d.outline(&ellipse((x, ys[i]), 19.0, 18.0, 0.0, 14), rgba([168, 97, 47], 1.0), 1.5);
                claw(d, (x, ys[i] + 12.0), (x + if side { 4.0 } else { -4.0 }, 640.0), 15.0, 1.0);
            }
        }
    }
}

// ---- the head ---------------------------------------------------------------------------------

const HEAD: [Pt; 12] = [
    (400.0, 205.0),
    (335.0, 218.0),
    (290.0, 250.0),
    (268.0, 310.0),
    (262.0, 370.0),
    (296.0, 420.0),
    (400.0, 446.0),
    (504.0, 420.0),
    (538.0, 370.0),
    (532.0, 310.0),
    (510.0, 250.0),
    (465.0, 218.0),
];

fn shifted(v: &[Pt], dy: f32) -> Vec<Pt> {
    v.iter().map(|&(x, y)| (x, y + dy)).collect()
}

/// `open` is how far the roar has the jaw open (0 shut, 1 wide).
fn paint_head(d: &Draw, b: &Bear, dy: f32, open: f32) {
    let t = b.t;
    let joy = b.mood == Mood::Joy;
    let rage = b.mood == Mood::Rage;
    // the roar: the jaw drops with each bellow
    let bellow = if rage { 0.5 + 0.5 * (TAU * 2.2 * t).sin() } else if joy { 0.5 + 0.5 * (TAU * 0.65 * t).sin() } else { 0.0 };
    let drop = open * (62.0 + 12.0 * bellow);
    let shake = if rage { vec2((TAU * 7.0 * t).sin() * 1.1, (TAU * 9.0 * t).cos() * 0.8) } else { vec2(0.0, 0.0) };
    let d = &Draw { painter: d.painter, at: d.at + shake * d.s, s: d.s, ox: d.ox };
    let h = |v: &[Pt]| shifted(v, dy);
    let p = |x: f32, y: f32| (x, y + dy);

    // ears
    let ear_l: Vec<Pt> = vec![(254.0, 232.0), (250.0, 192.0), (278.0, 166.0), (312.0, 184.0), (336.0, 198.0), (336.0, 222.0), (326.0, 236.0)];
    let ear_r: Vec<Pt> = ear_l.iter().map(|&(x, y)| (800.0 - x, y)).collect();
    for ear in [ear_l, ear_r] {
        let e = h(&ear);
        d.fan(&e, rgba(FUR_MID, 1.0));
        d.outline(&e, rgba(EMBER, 0.8), 2.0);
    }

    // the lower jaw hangs from behind the cheeks when he roars; shut, it is part of the face
    if open > 0.0 {
        paint_jaw(d, dy, drop, joy, t);
    }

    // the face
    let outline = ragged(&h(&HEAD), 3, 2.4, 3);
    d.glow_fan(&outline, p(400.0, 300.0), rgba(FUR_LIGHT, 1.0), rgba(FUR_DARK, 1.0));
    d.outline(&outline, rgba(EMBER, 0.9), 3.0);
    if open == 0.0 {
        // the chin and the closed mouth
        let chin = h(&[(296.0, 416.0), (300.0, 436.0), (400.0, 472.0), (500.0, 436.0), (504.0, 416.0)]);
        d.fan(&chin, rgba(FUR_MID, 1.0));
        d.outline(&chin, rgba(EMBER, 0.7), 2.4);
    }
    // facets
    let fac = |pts: &[Pt], c: [u8; 3], a: f32| d.fan(&h(pts), rgba(c, a));
    fac(&[(335.0, 218.0), (400.0, 255.0), (350.0, 262.0)], [122, 42, 16], 0.4);
    fac(&[(465.0, 218.0), (400.0, 255.0), (450.0, 262.0)], [168, 80, 26], 0.4);
    fac(&[(268.0, 310.0), (335.0, 290.0), (340.0, 380.0), (262.0, 370.0)], [90, 29, 14], 0.4);
    fac(&[(532.0, 310.0), (465.0, 290.0), (460.0, 380.0), (538.0, 370.0)], [122, 42, 16], 0.4);
    fac(&[(296.0, 420.0), (345.0, 425.0), (340.0, 380.0), (262.0, 370.0)], [138, 58, 20], 0.28);
    fac(&[(504.0, 420.0), (455.0, 425.0), (460.0, 380.0), (538.0, 370.0)], [90, 29, 14], 0.28);
    // the muzzle
    let muzzle = h(&[(365.0, 330.0), (435.0, 330.0), (460.0, 380.0), (455.0, 425.0), (400.0, 446.0), (345.0, 425.0), (340.0, 380.0)]);
    d.fan(&muzzle, rgba([44, 26, 20], 0.95));
    d.outline(&muzzle, rgba([168, 97, 47], 0.9), 1.2);
    // the wireframe cracks of lava over it all
    for seg in [
        [(400.0, 205.0), (400.0, 255.0)],
        [(335.0, 218.0), (350.0, 262.0)],
        [(350.0, 262.0), (400.0, 255.0)],
        [(290.0, 250.0), (335.0, 290.0)],
        [(335.0, 290.0), (365.0, 330.0)],
        [(335.0, 290.0), (340.0, 380.0)],
        [(465.0, 218.0), (450.0, 262.0)],
        [(450.0, 262.0), (400.0, 255.0)],
        [(510.0, 250.0), (465.0, 290.0)],
        [(465.0, 290.0), (435.0, 330.0)],
        [(465.0, 290.0), (460.0, 380.0)],
        [(365.0, 330.0), (435.0, 330.0)],
    ] {
        d.line(&h(&seg), rgba([255, 154, 74], 0.3), 0.9);
    }
    // the brows
    let (brow_l, brow_r): (Vec<Pt>, Vec<Pt>) = if joy {
        (vec![(288.0, 254.0), (306.0, 236.0), (342.0, 236.0), (360.0, 252.0), (354.0, 268.0), (338.0, 258.0), (314.0, 258.0), (298.0, 268.0)], vec![(512.0, 254.0), (494.0, 236.0), (458.0, 236.0), (440.0, 252.0), (446.0, 268.0), (462.0, 258.0), (486.0, 258.0), (502.0, 268.0)])
    } else {
        (vec![(292.0, 262.0), (350.0, 284.0), (358.0, 262.0), (296.0, 238.0)], vec![(508.0, 262.0), (450.0, 284.0), (442.0, 262.0), (504.0, 238.0)])
    };
    let lower = 8.0 * b.think;
    let (brow_l, brow_r): (Vec<Pt>, Vec<Pt>) = (brow_l.iter().map(|&(x, y)| (x, y + lower)).collect(), brow_r.iter().map(|&(x, y)| (x, y + lower)).collect());
    for br in [&brow_l, &brow_r] {
        d.fan(&h(br), rgba([5, 2, 2], 1.0));
        d.outline(&h(br), rgba([168, 97, 47], 0.9), 1.4);
    }
    d.line(&h(&[(382.0, 232.0), (396.0, 258.0)]), rgba([10, 5, 4], 0.85), 2.4);
    d.line(&h(&[(418.0, 232.0), (404.0, 258.0)]), rgba([10, 5, 4], 0.85), 2.4);

    // the plain eye (his right, on your left): a slit that glows; it winks when he is pleased
    let eye = p(335.0, 292.0);
    d.fan(&ellipse(eye, 26.0, 14.0, 0.0, 16), rgba([4, 2, 2], 1.0));
    d.soft(eye, 44.0, 26.0, [255, 90, 20], 0.5 + 0.2 * (TAU * 1.2 * t).sin());
    d.fan(&ellipse(eye, 12.0, 9.0, 0.0, 14), rgba([255, 190, 100], 1.0));
    d.fan(&[(eye.0, eye.1 - 8.0), (eye.0 - 4.0, eye.1), (eye.0, eye.1 + 8.0), (eye.0 + 4.0, eye.1)], rgba([4, 2, 2], 1.0));
    if joy {
        // one wink every few seconds
        let c = b.since.rem_euclid(3.4);
        let shut = smooth((c - 0.8) / 0.12) * (1.0 - smooth((c - 1.7) / 0.14));
        if shut > 0.02 {
            let lid = ellipse(eye, 30.0, 19.0 * shut.max(0.15), 0.0, 18);
            d.fan(&lid, rgba([51, 32, 26], shut));
            d.line(&[(eye.0 - 26.0, eye.1 - 2.0), (eye.0 - 12.0, eye.1 + 10.0 * shut), (eye.0 + 12.0, eye.1 + 10.0 * shut), (eye.0 + 26.0, eye.1 - 2.0)], rgba([5, 2, 2], shut), 5.0);
        }
    }

    // the eye: a shard of crystal, a broken trapezoid (wide and leaning at the top, narrowing to a
    // chipped foot) with splinters off its corners, cut in the cold ice of the Platinum glass
    let pc = p(465.0, 292.0);
    let pulse = 0.78 + 0.22 * (TAU * if rage || joy { 1.2 } else { 0.4 } * t).sin();
    let at = |q: Pt| (pc.0 + q.0, pc.1 + q.1);
    let ice = [214, 228, 246];
    d.soft(pc, 64.0, 56.0, [176, 206, 255], 0.26 * pulse);
    d.soft(pc, 34.0, 30.0, [255, 96, 30], 0.16 * pulse);
    let rim: Vec<Pt> = [(-36.0, -30.0), (-8.0, -38.0), (4.0, -31.0), (41.0, -43.0), (46.0, -10.0), (34.0, 4.0), (27.0, 34.0), (-13.0, 30.0), (-31.0, 10.0)].iter().map(|&q| at(q)).collect();
    // splinters
    for sp in [[(34.0, -36.0), (62.0, -64.0), (46.0, -24.0)], [(-20.0, 27.0), (-42.0, 52.0), (-6.0, 31.0)], [(-33.0, -2.0), (-56.0, -9.0), (-32.0, 14.0)], [(40.0, 0.0), (58.0, 14.0), (30.0, 18.0)]] {
        let tri: Vec<Pt> = sp.iter().map(|&q| at(q)).collect();
        d.fan(&tri, rgba([30, 40, 62], 0.95));
        d.fan(&tri, rgba(ice, 0.16 + 0.1 * pulse));
        d.outline(&tri, rgba(ice, 0.8), 1.2);
    }
    d.fan(&rim, rgba([12, 16, 28], 0.98));
    // the facets, brighter as a slow band of light crosses them
    let heart = at((2.0, -4.0));
    let tones = [0.45, 0.8, 0.5, 0.95, 0.4, 0.65, 0.3, 0.6, 0.5];
    let sweep = (t * 0.24).rem_euclid(1.9) - 0.45;
    let n = rim.len();
    for i in 0..n {
        let (a, c2) = (rim[i], rim[(i + 1) % n]);
        let mid = ((a.0 + c2.0 + heart.0) / 3.0 - pc.0, (a.1 + c2.1 + heart.1) / 3.0 - pc.1);
        let diag = (mid.0 + mid.1) / 110.0 + 0.5;
        let light = (1.0 - (diag - sweep).abs() / 0.3).max(0.0);
        let alpha = 0.08 + 0.34 * tones[i] * (0.6 + 0.4 * pulse) + 0.45 * light;
        d.fan(&[a, c2, heart], rgba(ice, alpha.clamp(0.0, 0.9)));
        d.line(&[a, heart], rgba(ice, 0.16 + 0.3 * light), 0.9);
    }
    d.outline(&rim, rgba([232, 242, 255], 0.95), 1.8);
    // a smaller cut stone in the middle of it, and the slit that makes it an eye
    let core: Vec<Pt> = [(-14.0, -14.0), (12.0, -18.0), (18.0, 6.0), (6.0, 18.0), (-12.0, 10.0)].iter().map(|&q| at(q)).collect();
    d.glow_fan(&core, at((0.0, -2.0)), rgba([240, 247, 255], 0.55), rgba([120, 160, 220], 0.35));
    d.outline(&core, rgba([236, 245, 255], 0.8), 1.0);
    let gz = b.think * 6.0 * (TAU * 0.45 * t).sin();
    d.fan(&[at((1.0 + gz, -17.0)), at((5.5 + gz, -1.0)), at((0.0 + gz, 18.0)), at((-4.5 + gz, -1.0))], rgba([4, 3, 8], 0.9));
    d.dot(at((1.0 + gz, -1.0)), 2.6, rgba([255, 120, 50], 0.55 + 0.4 * pulse));
    // a glint that comes and goes at the high corner
    let tw = ((TAU * 0.33 * t).sin() * 0.5 + 0.5).powf(3.0);
    let g = at((38.0, -38.0));
    d.line(&[(g.0 - 9.0, g.1), (g.0 + 9.0, g.1)], rgba([255, 255, 255], 0.8 * tw), 1.1);
    d.line(&[(g.0, g.1 - 9.0), (g.0, g.1 + 9.0)], rgba([255, 255, 255], 0.8 * tw), 1.1);
    if b.bleeding {
        // a thick run of blood from under the prism, down the cheek, and drops that fall
        let run = [(pc.0 - 8.0, pc.1 + 40.0), (pc.0 - 16.0, pc.1 + 62.0), (pc.0 - 6.0, pc.1 + 92.0), (pc.0 - 12.0, pc.1 + 128.0), (pc.0 - 4.0, pc.1 + 156.0)];
        d.line(&run, rgba([224, 24, 42], 0.95), 11.0);
        d.line(&run, rgba([255, 138, 138], 0.5), 2.0);
        d.line(&[(pc.0 + 34.0, pc.1 + 30.0), (pc.0 + 36.0, pc.1 + 58.0), (pc.0 + 32.0, pc.1 + 76.0)], rgba([224, 24, 42], 0.85), 6.0);
        for k in 0..2 {
            let ph = (t * 0.55 + k as f32 * 0.5).rem_euclid(1.0);
            d.dot((pc.0 - 4.0 + k as f32 * 40.0, pc.1 + 150.0 + 100.0 * ph), 5.0 - 2.0 * ph, rgba([224, 24, 42], 1.0 - ph));
        }
    }

    // the nose
    let nose = h(&[(362.0, 366.0), (438.0, 366.0), (428.0, 398.0), (400.0, 410.0), (372.0, 398.0)]);
    d.fan(&nose, rgba([4, 2, 2], 1.0));
    d.outline(&nose, rgba([168, 97, 47], 0.9), 1.6);
    for x in [383.0_f32, 417.0] {
        d.fan(&ellipse(p(x, 384.0), 7.0, 5.0, 0.0, 10), rgba([42, 10, 10], 1.0));
    }
    // the scars
    d.line(&h(&[(300.0, 300.0), (330.0, 340.0), (322.0, 372.0), (352.0, 418.0)]), rgba(SCAR, 0.8), 2.2);
    d.line(&h(&[(314.0, 296.0), (342.0, 334.0), (336.0, 366.0), (364.0, 410.0)]), rgba(SCAR, 0.8), 2.0);
    if joy {
        // cheeks shoved up in a grin
        for s in [-1.0_f32, 1.0] {
            d.fan(&ellipse(p(400.0 + s * 82.0, 352.0), 38.0, 18.0, s * 18.0, 16), rgba([58, 33, 24], 0.7));
            d.line(&h(&[(400.0 + s * 92.0, 424.0), (400.0 + s * 112.0, 410.0), (400.0 + s * 120.0, 390.0), (400.0 + s * 116.0, 368.0)]), rgba([10, 5, 4], 0.8), 4.0);
        }
    }

    // the teeth of the upper jaw, and the smoke that leaks from the corners of a roar
    if open > 0.0 {
        for fang in [[(312.0, 424.0), (324.0, 482.0), (344.0, 432.0)], [(456.0, 432.0), (476.0, 482.0), (488.0, 424.0)]] {
            d.fan(&h(&fang), rgba([239, 229, 204], 1.0));
            d.outline(&h(&fang), rgba([110, 95, 76], 0.9), 0.9);
        }
        for tooth in [[(350.0, 436.0), (356.0, 458.0), (366.0, 440.0)], [(374.0, 438.0), (381.0, 462.0), (390.0, 441.0)], [(410.0, 441.0), (419.0, 462.0), (426.0, 438.0)], [(434.0, 440.0), (444.0, 458.0), (450.0, 436.0)]] {
            d.fan(&h(&tooth), rgba([232, 220, 192], 1.0));
        }
        d.line(&h(&[(296.0, 420.0), (336.0, 436.0), (370.0, 446.0), (400.0, 446.0), (430.0, 446.0), (464.0, 436.0), (504.0, 420.0)]), rgba([168, 97, 47], 0.9), 2.0);
        let rate = if rage { 1.25 } else { 0.9 };
        for side in [-1.0_f32, 1.0] {
            for k in 0..3 {
                let ph = (t * rate + k as f32 * 0.33 + if side > 0.0 { 0.15 } else { 0.0 }).rem_euclid(1.0);
                let c = p(400.0 + side * 88.0 + side * 130.0 * ph, 418.0 - 80.0 * ph);
                d.soft(c, 13.0 + 34.0 * ph, 11.0 + 30.0 * ph, [230, 177, 132], (1.0 - ph) * 0.55);
            }
        }
    } else {
        // shut: a row of teeth showing at the lip, and breath through the nose
        for fang in [[(352.0, 426.0), (362.0, 450.0), (372.0, 428.0)], [(428.0, 428.0), (438.0, 450.0), (448.0, 426.0)]] {
            d.fan(&h(&fang), rgba([239, 229, 204], 1.0));
        }
        for tooth in [[(376.0, 430.0), (381.0, 444.0), (387.0, 431.0)], [(394.0, 432.0), (400.0, 446.0), (406.0, 432.0)], [(413.0, 431.0), (419.0, 444.0), (424.0, 430.0)]] {
            d.fan(&h(&tooth), rgba([232, 220, 192], 1.0));
        }
        d.line(&h(&[(346.0, 426.0), (400.0, 434.0), (454.0, 426.0)]), rgba([5, 2, 2], 0.9), 2.4);
        // a faint breath from the nose as he breathes out (the same slow rhythm as his chest)
        let cycle = (t / 5.6).rem_euclid(1.0);
        let out = ((cycle - 0.5) / 0.5).clamp(0.0, 1.0);
        if out > 0.0 {
            let fade = (out * std::f32::consts::PI).sin();
            for x in [383.0_f32, 417.0] {
                d.soft(p(x, 442.0 - 22.0 * out), 10.0 + 12.0 * out, 7.0 + 9.0 * out, [201, 138, 90], 0.2 * fade);
            }
        }
    }
}

/// The lean jaw of a roar: a narrow, tapering bear's jaw that swings down and out from behind the
/// cheeks (not a wide round one), the lava-lit inside of the mouth, the tongue and the lower teeth.
fn paint_jaw(d: &Draw, dy: f32, drop: f32, joy: bool, t: f32) {
    let dy = dy + drop * 0.0;
    let down = |v: &[Pt], k: f32| -> Vec<Pt> { v.iter().map(|&(x, y)| (x, y + dy + if y > 440.0 { drop * k } else { 0.0 })).collect() };
    let jaw = [(304.0, 408.0), (298.0, 470.0), (320.0, 532.0), (360.0, 582.0), (376.0, 604.0), (390.0, 616.0), (400.0, 618.0), (410.0, 616.0), (424.0, 604.0), (440.0, 582.0), (480.0, 532.0), (502.0, 470.0), (496.0, 408.0), (470.0, 440.0), (436.0, 456.0), (400.0, 456.0), (364.0, 456.0), (330.0, 440.0)];
    let outer = down(&jaw, 1.0);
    let cavity = down(&[(312.0, 424.0), (400.0, 448.0), (488.0, 424.0), (494.0, 466.0), (478.0, 508.0), (452.0, 540.0), (432.0, 564.0), (368.0, 564.0), (348.0, 540.0), (322.0, 508.0), (306.0, 466.0)], 1.0);
    // behind the face: fur of the jaw, then the dark and the lava of the throat
    d.fan(&ragged(&outer, 3, 2.0, 41), rgba(FUR_MID, 1.0));
    d.outline(&outer, rgba(EMBER, 0.9), 3.0);
    d.glow_fan(&cavity, (400.0, 450.0 + dy), rgba([255, 90, 20], 1.0), rgba([20, 4, 10], 1.0));
    // the tongue, lolling as he roars
    let sway = (TAU * 0.9 * t).sin() * 4.0;
    let tongue = down(&[(358.0, 546.0), (356.0, 516.0), (376.0, 502.0), (400.0, 504.0), (424.0, 502.0), (444.0, 516.0), (442.0, 546.0), (426.0, 564.0), (374.0, 564.0)], 1.0);
    let tongue: Vec<Pt> = tongue.iter().map(|&(x, y)| (x + sway, y)).collect();
    d.fan(&tongue, rgba([138, 20, 36], 1.0));
    d.line(&down(&[(400.0, 506.0), (400.0, 558.0)], 1.0).iter().map(|&(x, y)| (x + sway, y)).collect::<Vec<_>>(), rgba([74, 6, 16], 1.0), 3.0);
    // the lower tusks and teeth
    for tusk in [[(338.0, 540.0), (336.0, 494.0), (354.0, 534.0)], [(446.0, 534.0), (464.0, 494.0), (462.0, 540.0)]] {
        d.fan(&down(&tusk, 1.0), rgba([232, 220, 192], 1.0));
    }
    for tooth in [[(366.0, 558.0), (372.0, 538.0), (381.0, 560.0)], [(386.0, 563.0), (392.0, 542.0), (399.0, 565.0)], [(401.0, 565.0), (408.0, 542.0), (414.0, 563.0)], [(419.0, 560.0), (428.0, 538.0), (434.0, 558.0)]] {
        d.fan(&down(&tooth, 1.0), rgba([216, 201, 174], 1.0));
    }
    let _ = joy;
}
