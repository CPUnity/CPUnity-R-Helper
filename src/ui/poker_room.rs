// The room the Devil's Table stands in, painted behind the five people: a hall of fine joinery (framed
// panels of dark walnut with gold thread, a carved cornice, a chair rail and a skirting board) between
// pillars of raw black rock that have cracks of ember in them and an iron sconce each, under a ceiling of
// rough stone, on a floor of polished black marble that shows the light of the sconces and of the lava.
//
// Everything is laid out in the board's own pixels (1100 x 700, see poker.rs) and goes through the
// camera, so that it turns and leans in with the table. It repeats sideways without end, so any width
// of window, and any turn of the camera, finds more room.
use super::backdrop::soft_ellipse;
use eframe::egui::{pos2, vec2, Color32, Mesh, Painter, Pos2, Rect, Shape, Stroke, StrokeKind};

/// Where the wall meets the floor.
const WALL_Y: f32 = 262.0;
/// One repeat of the wall: a wooden bay and then a pillar of rock. The bays are placed so that each of
/// the five people (at 60, 255, 470, 715 and 975) sits in front of wood, with a pillar between them.
const BAY: f32 = 240.0;
const WOOD_W: f32 = 176.0;
const ROCK_W: f32 = BAY - WOOD_W;
/// Where bay number 0 begins.
const OFFSET: f32 = -78.0;
/// The underside of the stone ceiling, where the cornice runs.
const CEILING_Y: f32 = 8.0;
/// How far down the floor goes before the table hides it.
const FLOOR_END: f32 = 352.0;

/// The board's pixels on the screen: where the board's origin is and how big a board pixel is.
pub struct View {
    pub rect: Rect,
    pub ox: f32,
    pub oy: f32,
    pub k: f32,
}

impl View {
    fn p(&self, x: f32, y: f32) -> Pos2 {
        pos2(self.ox + x * self.k, self.oy + y * self.k)
    }
    /// A width in points (never thinner than `min`).
    fn w(&self, v: f32, min: f32) -> f32 {
        (v * self.k).max(min)
    }
}

fn hash(i: i32, s: u32) -> f32 {
    let mut x = (i as u32).wrapping_mul(0x9E37_79B1) ^ s.wrapping_mul(0x85EB_CA6B).wrapping_add(0x1234_5678);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0xFFFF) as f32 / 65535.0
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// A four-cornered shape with a colour at each corner (top left, top right, bottom right, bottom left).
fn quad(p: &Painter, c: [Pos2; 4], col: [Color32; 4]) {
    let mut m = Mesh::default();
    for i in 0..4 {
        m.colored_vertex(c[i], col[i]);
    }
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(Shape::mesh(m));
}

/// A rectangle fading from `top` to `bottom`.
fn vgrad(p: &Painter, r: Rect, top: Color32, bottom: Color32) {
    quad(p, [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()], [top, top, bottom, bottom]);
}

pub fn paint(p: &Painter, v: &View, t: f32) {
    let r = v.rect;
    p.rect_filled(r, 0.0, rgb(7, 5, 6));
    let xl = (r.left() - v.ox) / v.k;
    let xr = (r.right() - v.ox) / v.k;
    let top = ((r.top() - v.oy) / v.k).min(0.0) - 60.0;
    let first = ((xl - OFFSET) / BAY).floor() as i32 - 1;
    let last = ((xr - OFFSET) / BAY).ceil() as i32 + 1;

    ceiling(p, v, xl, xr, top);
    for i in first..=last {
        wood_bay(p, v, i, OFFSET + i as f32 * BAY);
    }
    for i in first..=last {
        rock_pillar(p, v, i, OFFSET + i as f32 * BAY + WOOD_W, top, t);
    }
    cornice(p, v, xl, xr);
    floor(p, v, xl, xr, first, last, t);
}

// ---- the ceiling -----------------------------------------------------------------------------------

/// Rough stone above the cornice, in courses of uneven blocks.
fn ceiling(p: &Painter, v: &View, xl: f32, xr: f32, top: f32) {
    let r = Rect::from_min_max(v.p(xl - 40.0, top.min(-700.0)), v.p(xr + 40.0, CEILING_Y + 2.0));
    vgrad(p, r, rgb(14, 12, 15), rgb(34, 29, 31));
    let mut y = CEILING_Y - 30.0;
    let mut row = 0;
    while y > top {
        let mut pts = Vec::new();
        let mut x = (xl / 40.0).floor() * 40.0 - 40.0;
        while x < xr + 40.0 {
            pts.push(v.p(x, y + (hash((x / 40.0) as i32, 3 + row as u32) - 0.5) * 7.0));
            x += 40.0;
        }
        p.add(Shape::line(pts, Stroke::new(v.w(1.4, 0.7), rgba(0, 0, 0, 0.55))));
        // the joints between the blocks of the course
        let mut jx = (xl / 120.0).floor() * 120.0 - (row % 2) as f32 * 60.0;
        while jx < xr + 120.0 {
            let jj = jx + (hash((jx / 60.0) as i32, 90 + row as u32) - 0.5) * 36.0;
            p.add(Shape::line_segment([v.p(jj, y), v.p(jj + (hash(row, jx as u32) - 0.5) * 8.0, y - 30.0)], Stroke::new(v.w(1.2, 0.6), rgba(0, 0, 0, 0.5))));
            jx += 120.0;
        }
        y -= 30.0;
        row += 1;
    }
    // the shadow it throws down the wall
    vgrad(p, Rect::from_min_max(v.p(xl - 40.0, CEILING_Y), v.p(xr + 40.0, CEILING_Y + 70.0)), rgba(0, 0, 0, 0.7), rgba(0, 0, 0, 0.0));
}

/// The cornice that runs all along under the ceiling: a carved beam of walnut with a row of dentils and a
/// thread of gold.
fn cornice(p: &Painter, v: &View, xl: f32, xr: f32) {
    let (x0, x1) = (xl - 20.0, xr + 20.0);
    p.rect_filled(Rect::from_min_max(v.p(x0, CEILING_Y - 4.0), v.p(x1, CEILING_Y + 14.0)), 0.0, rgb(70, 34, 20));
    vgrad(p, Rect::from_min_max(v.p(x0, CEILING_Y - 4.0), v.p(x1, CEILING_Y + 3.0)), rgb(128, 74, 42), rgb(76, 38, 22));
    p.add(Shape::line_segment([v.p(x0, CEILING_Y - 4.0), v.p(x1, CEILING_Y - 4.0)], Stroke::new(v.w(1.2, 0.7), rgb(176, 118, 68))));
    p.add(Shape::line_segment([v.p(x0, CEILING_Y + 14.0), v.p(x1, CEILING_Y + 14.0)], Stroke::new(v.w(1.4, 0.8), rgb(176, 130, 62))));
    let mut x = (x0 / 12.0).floor() * 12.0;
    while x < x1 {
        p.rect_filled(Rect::from_min_max(v.p(x + 2.0, CEILING_Y + 14.0), v.p(x + 9.0, CEILING_Y + 23.0)), 0.0, rgb(58, 28, 16));
        p.add(Shape::line_segment([v.p(x + 2.0, CEILING_Y + 14.0), v.p(x + 9.0, CEILING_Y + 14.0)], Stroke::new(v.w(0.8, 0.5), rgb(140, 84, 48))));
        x += 12.0;
    }
    p.rect_filled(Rect::from_min_max(v.p(x0, CEILING_Y + 23.0), v.p(x1, CEILING_Y + 26.0)), 0.0, rgba(0, 0, 0, 0.55));
}

// ---- the wooden bay ---------------------------------------------------------------------------------

/// A framed panel of wood: a frame with lit and shaded edges, a recessed field with the grain running up
/// it, and a thread of gold set inside. `frame` is how thick the frame is.
fn panel(p: &Painter, v: &View, x0: f32, y0: f32, x1: f32, y1: f32, frame: f32, seed: u32) {
    let outer = Rect::from_min_max(v.p(x0, y0), v.p(x1, y1));
    p.rect_filled(outer, 0.0, rgb(88, 44, 26));
    // the frame is lit from above and the left
    p.add(Shape::line(vec![v.p(x0, y1), v.p(x0, y0), v.p(x1, y0)], Stroke::new(v.w(1.6, 0.8), rgb(158, 98, 56))));
    p.add(Shape::line(vec![v.p(x1, y0), v.p(x1, y1), v.p(x0, y1)], Stroke::new(v.w(1.6, 0.8), rgb(30, 13, 9))));
    let (fx0, fy0, fx1, fy1) = (x0 + frame, y0 + frame, x1 - frame, y1 - frame);
    let field = Rect::from_min_max(v.p(fx0, fy0), v.p(fx1, fy1));
    vgrad(p, field, rgb(60, 28, 17), rgb(44, 20, 12));
    // the grain: long wavering lines, light and dark
    let n = ((fx1 - fx0) / 9.0).max(3.0) as i32;
    for g in 0..n {
        let gx = fx0 + (g as f32 + 0.5 + (hash(g, seed) - 0.5) * 0.8) * (fx1 - fx0) / n as f32;
        let phase = hash(g, seed + 7) * 6.0;
        let amp = 1.2 + hash(g, seed + 11) * 2.4;
        let pts: Vec<Pos2> = (0..=8).map(|j| {
            let f = j as f32 / 8.0;
            v.p(gx + (f * 5.0 + phase).sin() * amp, fy0 + (fy1 - fy0) * f)
        }).collect();
        let dark = g % 2 == 0;
        p.add(Shape::line(pts, Stroke::new(v.w(if dark { 1.1 } else { 0.8 }, 0.5), if dark { rgba(16, 6, 4, 0.22) } else { rgba(150, 84, 46, 0.13) })));
    }
    // the field is shaded where it is let into the frame
    p.rect_stroke(field, 0.0, Stroke::new(v.w(2.0, 1.0), rgba(0, 0, 0, 0.35)), StrokeKind::Inside);
    p.add(Shape::line(vec![v.p(fx0, fy1), v.p(fx0, fy0), v.p(fx1, fy0)], Stroke::new(v.w(1.0, 0.6), rgba(0, 0, 0, 0.6))));
    // a thread of gold inlaid, with a bead at each corner
    let inset = frame * 0.5 + 3.0;
    let gold = Rect::from_min_max(v.p(fx0 + inset, fy0 + inset), v.p(fx1 - inset, fy1 - inset));
    p.rect_stroke(gold, 0.0, Stroke::new(v.w(1.0, 0.6), rgba(188, 140, 66, 0.6)), StrokeKind::Inside);
    for (cx, cy) in [(gold.left(), gold.top()), (gold.right(), gold.top()), (gold.right(), gold.bottom()), (gold.left(), gold.bottom())] {
        p.circle_filled(pos2(cx, cy), v.w(1.8, 1.0), rgb(212, 168, 84));
    }
}

fn wood_bay(p: &Painter, v: &View, i: i32, x0: f32) {
    let x1 = x0 + WOOD_W;
    // the wall behind the panelling
    vgrad(p, Rect::from_min_max(v.p(x0, CEILING_Y), v.p(x1, WALL_Y)), rgb(22, 10, 8), rgb(40, 18, 12));
    // the tall panel
    let pad = 16.0;
    panel(p, v, x0 + pad, 44.0, x1 - pad, 168.0, 12.0, 100 + i.rem_euclid(5) as u32 * 13);
    // a diamond of gold in the middle of it, with a bead
    let (cx, cy) = ((x0 + x1) / 2.0, 106.0);
    let dia = vec![v.p(cx, cy - 34.0), v.p(cx + 20.0, cy), v.p(cx, cy + 34.0), v.p(cx - 20.0, cy)];
    p.add(Shape::closed_line(dia, Stroke::new(v.w(1.3, 0.7), rgba(204, 156, 76, 0.7))));
    p.circle_filled(v.p(cx, cy), v.w(2.4, 1.2), rgb(222, 178, 92));
    // the chair rail
    p.rect_filled(Rect::from_min_max(v.p(x0, 172.0), v.p(x1, 184.0)), 0.0, rgb(96, 50, 29));
    p.add(Shape::line_segment([v.p(x0, 172.0), v.p(x1, 172.0)], Stroke::new(v.w(1.4, 0.7), rgb(178, 116, 66))));
    p.add(Shape::line_segment([v.p(x0, 184.0), v.p(x1, 184.0)], Stroke::new(v.w(1.4, 0.7), rgba(0, 0, 0, 0.6))));
    p.add(Shape::line_segment([v.p(x0, 178.0), v.p(x1, 178.0)], Stroke::new(v.w(0.9, 0.5), rgba(200, 154, 74, 0.55))));
    // the wainscot: two lower panels
    let half = (WOOD_W - 3.0 * 12.0) / 2.0;
    for j in 0..2 {
        let a = x0 + 12.0 + j as f32 * (half + 12.0);
        panel(p, v, a, 192.0, a + half, 246.0, 7.0, 300 + (i.rem_euclid(5) * 2 + j) as u32 * 17);
    }
    // the skirting board
    p.rect_filled(Rect::from_min_max(v.p(x0, 250.0), v.p(x1, WALL_Y)), 0.0, rgb(66, 33, 19));
    p.add(Shape::line_segment([v.p(x0, 250.0), v.p(x1, 250.0)], Stroke::new(v.w(1.4, 0.7), rgb(160, 102, 58))));
    p.add(Shape::line_segment([v.p(x0, 254.0), v.p(x1, 254.0)], Stroke::new(v.w(0.8, 0.5), rgba(200, 154, 74, 0.5))));
    // the room is dark: the wood is lit from the sconces at its sides, and the dark gathers overhead
    vgrad(p, Rect::from_min_max(v.p(x0, CEILING_Y), v.p(x1, WALL_Y)), rgba(0, 0, 0, 0.42), rgba(0, 0, 0, 0.12));
    // the light of the sconce next to it falls on the wood
    soft_ellipse(p, v.p(x1, 150.0), vec2(v.w(150.0, 8.0), v.w(150.0, 8.0)), [255, 150, 70], 0.09);
    soft_ellipse(p, v.p(x0, 150.0), vec2(v.w(150.0, 8.0), v.w(150.0, 8.0)), [255, 150, 70], 0.09);
}

// ---- the pillar of rock -------------------------------------------------------------------------------

fn rock_pillar(p: &Painter, v: &View, i: i32, x0: f32, top: f32, t: f32) {
    let seed = (i.rem_euclid(97)) as u32 + 500;
    // the rows of points down the pillar are on a grid of the wall's own (so the rock keeps its shape as the
    // camera moves); only the rows that can be seen are made
    const Y0: f32 = -600.0;
    const STEP: f32 = 26.0;
    let n0 = (((top - Y0) / STEP).floor().max(0.0)) as i32;
    let mut rows: Vec<(i32, f32)> = Vec::new();
    let mut n = n0;
    loop {
        let y = Y0 + n as f32 * STEP + (hash(n, seed) - 0.5) * 8.0;
        if y >= WALL_Y - 10.0 {
            rows.push((n, WALL_Y));
            break;
        }
        rows.push((n, y));
        n += 1;
    }
    // three columns of points down the pillar: a ragged left edge, a lit middle, a ragged right edge
    let lamp_y = 150.0;
    let mut m = Mesh::default();
    for &(ri, y) in rows.iter() {
        let jl = (hash(ri, seed + 1) - 0.5) * 18.0;
        let jr = (hash(ri, seed + 2) - 0.5) * 18.0;
        // a little wider at the foot, where the rock meets the floor
        let foot = ((y - (WALL_Y - 40.0)) / 40.0).clamp(0.0, 1.0) * 8.0;
        let lit = (1.0 - ((y - lamp_y).abs() / 170.0)).clamp(0.0, 1.0);
        let warm = |base: [u8; 3], boost: f32| {
            Color32::from_rgb((base[0] as f32 + boost * 40.0 * lit) as u8, (base[1] as f32 + boost * 20.0 * lit) as u8, (base[2] as f32 + boost * 8.0 * lit) as u8)
        };
        m.colored_vertex(v.p(x0 - 2.0 + jl - foot, y), warm([20, 17, 20], 0.7));
        m.colored_vertex(v.p(x0 + ROCK_W * 0.5 + (hash(ri, seed + 3) - 0.5) * 10.0, y), warm([54, 47, 51], 1.0));
        m.colored_vertex(v.p(x0 + ROCK_W + 2.0 + jr + foot, y), warm([24, 20, 24], 0.7));
    }
    for ri in 0..rows.len() as u32 - 1 {
        let (a, b) = (ri * 3, (ri + 1) * 3);
        m.add_triangle(a, a + 1, b);
        m.add_triangle(a + 1, b + 1, b);
        m.add_triangle(a + 1, a + 2, b + 1);
        m.add_triangle(a + 2, b + 2, b + 1);
    }
    p.add(Shape::mesh(m));
    // chisel marks and edges of the facets
    for w in rows.windows(2) {
        let ((ri, y0), (_, y1)) = (w[0], w[1]);
        let xa = x0 + 4.0 + hash(ri, seed + 4) * 20.0;
        let xb = x0 + ROCK_W - 4.0 - hash(ri, seed + 5) * 20.0;
        p.add(Shape::line(vec![v.p(xa, y0), v.p((xa + xb) / 2.0, y0 + (hash(ri, seed + 6) - 0.4) * 6.0), v.p(xb, y0 + 2.0)], Stroke::new(v.w(1.0, 0.6), rgba(0, 0, 0, 0.5))));
        if hash(ri, seed + 8) > 0.5 {
            let fx = x0 + 10.0 + hash(ri, seed + 9) * (ROCK_W - 20.0);
            p.add(Shape::line_segment([v.p(fx, y0), v.p(fx + (hash(ri, seed + 10) - 0.5) * 16.0, y1)], Stroke::new(v.w(0.9, 0.5), rgba(130, 118, 124, 0.22))));
        }
    }
    // a crack with an ember in it, breathing
    let breathe = 0.75 + 0.25 * (t * 1.3 + i as f32 * 1.7).sin();
    let mut crack = Vec::new();
    let mut cx = x0 + ROCK_W * (0.3 + hash(i, seed + 20) * 0.4);
    let mut y = 40.0 + hash(i, seed + 21) * 40.0;
    let end = WALL_Y - 12.0 - hash(i, seed + 22) * 30.0;
    let mut n = 0;
    while y < end {
        crack.push(v.p(cx, y));
        cx += (hash(i * 31 + n, seed + 23) - 0.5) * 16.0;
        cx = cx.clamp(x0 + 14.0, x0 + ROCK_W - 14.0);
        y += 12.0 + hash(i * 17 + n, seed + 24) * 12.0;
        n += 1;
    }
    if crack.len() >= 2 {
        p.add(Shape::line(crack.clone(), Stroke::new(v.w(7.0, 3.0), rgba(255, 90, 30, 0.10 * breathe))));
        p.add(Shape::line(crack.clone(), Stroke::new(v.w(2.6, 1.2), rgba(255, 120, 40, 0.45 * breathe))));
        p.add(Shape::line(crack, Stroke::new(v.w(1.0, 0.6), rgba(255, 206, 130, 0.9 * breathe))));
    }
    sconce(p, v, x0 + ROCK_W * 0.5, lamp_y, t, i);
}

/// An iron bracket on the rock with a flame in it that never keeps still.
fn sconce(p: &Painter, v: &View, x: f32, y: f32, t: f32, i: i32) {
    let flick = 0.8 + 0.12 * (t * 9.0 + i as f32).sin() + 0.08 * (t * 23.0 + i as f32 * 2.0).sin();
    soft_ellipse(p, v.p(x, y - 6.0), vec2(v.w(130.0 * flick, 10.0), v.w(130.0 * flick, 10.0)), [255, 150, 60], 0.26);
    soft_ellipse(p, v.p(x, y - 6.0), vec2(v.w(46.0 * flick, 5.0), v.w(56.0 * flick, 5.0)), [255, 190, 100], 0.34);
    // the bracket and the cup
    p.add(Shape::line(vec![v.p(x - 16.0, y - 14.0), v.p(x - 16.0, y + 4.0), v.p(x, y + 12.0), v.p(x + 16.0, y + 4.0), v.p(x + 16.0, y - 14.0)], Stroke::new(v.w(2.6, 1.2), rgb(16, 14, 18))));
    p.add(Shape::convex_polygon(vec![v.p(x - 12.0, y - 14.0), v.p(x + 12.0, y - 14.0), v.p(x + 8.0, y - 2.0), v.p(x - 8.0, y - 2.0)], rgb(30, 26, 30), Stroke::new(v.w(1.0, 0.6), rgb(150, 100, 60))));
    // the flame
    let h = 26.0 * flick;
    let lean = (t * 7.0 + i as f32 * 3.0).sin() * 2.5;
    p.add(Shape::convex_polygon(vec![v.p(x - 8.0, y - 14.0), v.p(x - 5.0 + lean * 0.3, y - 14.0 - h * 0.55), v.p(x + lean, y - 14.0 - h), v.p(x + 5.0 + lean * 0.3, y - 14.0 - h * 0.55), v.p(x + 8.0, y - 14.0)], rgba(255, 120, 34, 0.92), Stroke::NONE));
    p.add(Shape::convex_polygon(vec![v.p(x - 4.0, y - 14.0), v.p(x + lean * 0.4, y - 14.0 - h * 0.62), v.p(x + 4.0, y - 14.0)], rgba(255, 226, 150, 0.95), Stroke::NONE));
}

// ---- the floor -----------------------------------------------------------------------------------------

/// Black marble, polished: slabs laid in perspective, veined in grey and gold, with the sconces
/// standing in it as long soft reflections.
fn floor(p: &Painter, v: &View, xl: f32, xr: f32, first: i32, last: i32, t: f32) {
    let area = Rect::from_min_max(v.p(xl - 40.0, WALL_Y), v.p(xr + 40.0, FLOOR_END));
    let q = p.with_clip_rect(area.intersect(p.clip_rect()));
    vgrad(&q, area, rgb(7, 6, 9), rgb(28, 26, 33));
    // the slabs: lines running back to a point on the wall, and rows that crowd up towards it
    let vp = (550.0, 40.0);
    let line = Stroke::new(v.w(1.0, 0.6), rgba(170, 170, 190, 0.11));
    let mut xf = (xl / 120.0).floor() * 120.0 - 1200.0;
    while xf < xr + 1200.0 {
        let at = |y: f32| v.p(vp.0 + (xf - vp.0) * (y - vp.1) / (FLOOR_END - vp.1), y);
        q.add(Shape::line_segment([at(WALL_Y), at(FLOOR_END)], line));
        xf += 120.0;
    }
    for n in 1..6 {
        let y = WALL_Y + (FLOOR_END - WALL_Y) * (n as f32 / 6.0).powf(1.6);
        q.add(Shape::line_segment([v.p(xl - 40.0, y), v.p(xr + 40.0, y)], line));
    }
    // the veins in the stone
    let mut idx = (xl / 150.0).floor() as i32 - 1;
    while (idx as f32) * 150.0 < xr + 150.0 {
        let mut x = idx as f32 * 150.0 + hash(idx, 61) * 150.0;
        let mut y = WALL_Y + 4.0 + hash(idx, 62) * 70.0;
        let mut pts = vec![v.p(x, y)];
        for s in 0..7 {
            x += 22.0 + hash(idx * 9 + s, 63) * 34.0;
            y = (y + (hash(idx * 9 + s, 64) - 0.5) * 30.0).clamp(WALL_Y + 2.0, FLOOR_END);
            pts.push(v.p(x, y));
        }
        let gold = idx.rem_euclid(3) == 0;
        let (col, wd) = if gold { (rgba(196, 154, 72, 0.16), 1.0) } else { (rgba(200, 200, 216, 0.10 + 0.10 * hash(idx, 65)), 0.8 + hash(idx, 66)) };
        q.add(Shape::line(pts, Stroke::new(v.w(wd, 0.5), col)));
        idx += 1;
    }
    // the polish: a broad sheen, and the sconces in it
    soft_ellipse(&q, v.p(550.0, 300.0), vec2(v.w(560.0, 20.0), v.w(34.0, 3.0)), [210, 205, 225], 0.06);
    for i in first..=last {
        let x = OFFSET + i as f32 * BAY + WOOD_W + ROCK_W * 0.5;
        let flick = 0.8 + 0.12 * (t * 9.0 + i as f32).sin();
        soft_ellipse(&q, v.p(x, WALL_Y + 52.0), vec2(v.w(14.0, 2.0), v.w(64.0 * flick, 4.0)), [255, 160, 70], 0.16);
        soft_ellipse(&q, v.p(x, WALL_Y + 38.0), vec2(v.w(6.0, 1.0), v.w(34.0 * flick, 3.0)), [255, 214, 140], 0.20);
    }
    // where the floor meets the wall it is in shadow
    vgrad(&q, Rect::from_min_max(v.p(xl - 40.0, WALL_Y), v.p(xr + 40.0, WALL_Y + 22.0)), rgba(0, 0, 0, 0.6), rgba(0, 0, 0, 0.0));
}
