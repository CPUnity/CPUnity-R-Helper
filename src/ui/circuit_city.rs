// Circuit City: a night city whose buildings are parts of a circuit board.
//
// Chips with pins along their sides, capacitor towers with a striped band, heatsinks with fins, grids
// of solder balls, resistor stacks, and plain blocks with traces on their faces; the windows are vias
// (small rings that light up in amber, teal or cyan). Behind them run PCB traces with 45 degree bends
// and little pulses of light travelling along them, and motes drift up through the haze.
//
// `paint_city` draws the sky and the skyline into any rectangle (the strip at the top of the page and
// the cat's talk use it), `paint_window` draws it behind the whole window as a backdrop, and
// `paint_vias` and `paint_street_traces` are the pieces the street scene borrows.
use super::backdrop::soft_ellipse;
use eframe::egui::{self, pos2, vec2, Color32, Pos2, Rect, Shape, Stroke};

/// The city's main colour when the accent cannot be used (a nearly black or grey one).
const TEAL_FALLBACK: [u8; 3] = [48, 214, 190];
const CYAN: [u8; 3] = [96, 220, 255];
const AMBER: [u8; 3] = [255, 184, 84];
const MAGENTA: [u8; 3] = [255, 90, 170];

/// The main colour of the city (edges, traces, haze, the road), packed as 0xRRGGBB. It follows the
/// accent colour of the UI, set once a frame by `follow_accent`.
static PRIMARY: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x30D6BE);

/// Takes the UI's accent colour as the city's main colour, brightened so it glows on the dark board
/// (the default dark green would vanish) with its hue kept. A near-black or grey accent keeps teal.
pub fn follow_accent() {
    let a = crate::theme::accent();
    PRIMARY.store(glow_of([a.r(), a.g(), a.b()]), std::sync::atomic::Ordering::Relaxed);
}

/// `accent` scaled so its brightest channel is 225, or the fallback teal when there is no hue.
pub fn glow_colour(accent: [u8; 3]) -> [u8; 3] {
    let m = accent.iter().copied().max().unwrap_or(0) as f32;
    let lo = accent.iter().copied().min().unwrap_or(0) as f32;
    if m < 40.0 || (m - lo) < 24.0 {
        return TEAL_FALLBACK;
    }
    let k = 225.0 / m;
    [0, 1, 2].map(|i| (accent[i] as f32 * k).round().clamp(0.0, 255.0) as u8)
}

fn glow_of(accent: [u8; 3]) -> u32 {
    let c = glow_colour(accent);
    ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32
}

/// The city's main colour right now.
pub fn teal() -> [u8; 3] {
    let v = PRIMARY.load(std::sync::atomic::Ordering::Relaxed);
    [((v >> 16) & 0xff) as u8, ((v >> 8) & 0xff) as u8, (v & 0xff) as u8]
}

fn alpha(c: [u8; 3], a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// A tiny repeatable random number (so the city does not change from frame to frame).
fn hash(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x % 10_000) as f32 / 10_000.0
}

fn gradient(painter: &egui::Painter, r: Rect, top: Color32, bottom: Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(r.left_top(), top);
    mesh.colored_vertex(r.right_top(), top);
    mesh.colored_vertex(r.right_bottom(), bottom);
    mesh.colored_vertex(r.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

fn accent(seed: u32) -> [u8; 3] {
    let v = hash(seed);
    if v < 0.45 {
        teal()
    } else if v < 0.75 {
        CYAN
    } else if v < 0.95 {
        AMBER
    } else {
        MAGENTA
    }
}

// ------------------------------------------------------------------------------------ traces

/// A trace across the board: runs along, bends at 45 degrees, runs on.
fn route(seed: u32, r: Rect, top: f32, bottom: f32) -> Vec<Pos2> {
    let mut p = pos2(r.left() + r.width() * hash(seed * 7 + 1), top + (bottom - top) * hash(seed * 7 + 2));
    let mut pts = vec![p];
    let dir_x = if hash(seed * 7 + 3) < 0.5 { 1.0f32 } else { -1.0 };
    let dir_y = if hash(seed * 7 + 4) < 0.5 { 1.0f32 } else { -1.0 };
    for i in 0..6u32 {
        let len = r.width() * (0.05 + 0.12 * hash(seed * 13 + i));
        let step = match i % 3 {
            0 => vec2(dir_x * len, 0.0),
            1 => vec2(dir_x * len * 0.5, dir_y * len * 0.5),
            _ => vec2(0.0, dir_y * len * 0.6),
        };
        p += step;
        p.x = p.x.clamp(r.left(), r.right());
        p.y = p.y.clamp(top, bottom);
        pts.push(p);
    }
    pts
}

fn length_of(pts: &[Pos2]) -> f32 {
    pts.windows(2).map(|w| (w[1] - w[0]).length()).sum()
}

/// The point `dist` along a trace.
fn point_along(pts: &[Pos2], dist: f32) -> Pos2 {
    let mut left = dist;
    for w in pts.windows(2) {
        let seg = (w[1] - w[0]).length();
        if left <= seg || seg <= 0.0 {
            let u = if seg > 0.0 { left / seg } else { 0.0 };
            return w[0] + (w[1] - w[0]) * u;
        }
        left -= seg;
    }
    pts[pts.len() - 1]
}

/// A trace with its vias at the bends, and a pulse of light running along it.
fn paint_trace(painter: &egui::Painter, pts: &[Pos2], col: [u8; 3], seed: u32, t: f32, animated: bool, strength: f32) {
    painter.add(Shape::line(pts.to_vec(), Stroke::new(1.0, alpha(col, 0.16 * strength))));
    for p in pts.iter().skip(1) {
        painter.circle_stroke(*p, 2.4, Stroke::new(1.0, alpha(col, 0.28 * strength)));
    }
    let total = length_of(pts);
    if total <= 1.0 {
        return;
    }
    let speed = 0.03 + 0.05 * hash(seed + 5);
    let u = if animated { (hash(seed + 6) + t * speed).fract() } else { hash(seed + 6) };
    let head = point_along(pts, total * u);
    soft_ellipse(painter, head, vec2(7.0, 7.0), col, 0.55 * strength);
    painter.circle_filled(head, 1.6, alpha(col, 0.95 * strength));
}

/// The traces across the street, with a pulse running along each.
pub fn paint_street_traces(painter: &egui::Painter, street: Rect, t: f32, animated: bool) {
    for row in 0..4u32 {
        let y = street.top() + (row as f32 + 1.0) * street.height() / 5.0;
        let col = accent(row + 40);
        painter.line_segment([pos2(street.left(), y), pos2(street.right(), y)], Stroke::new(1.0, alpha(col, 0.14)));
        let speed = 0.05 + 0.03 * hash(row + 50);
        let u = if animated { (hash(row + 51) + t * speed * if row % 2 == 0 { 1.0 } else { -1.0 }).rem_euclid(1.0) } else { hash(row + 51) };
        let head = pos2(street.left() + street.width() * u, y);
        soft_ellipse(painter, head, vec2(8.0, 8.0), col, 0.5);
        painter.circle_filled(head, 1.5, alpha(col, 0.9));
    }
}

// ----------------------------------------------------------------------------------- buildings

/// Windows made of vias: small rings, some of them lit.
pub fn paint_vias(painter: &egui::Painter, b: Rect, seed: u32, step: f32, t: f32, animated: bool, bright: f32) {
    let mut row = 0u32;
    let mut y = b.top() + step * 0.8;
    while y < b.bottom() - step * 0.5 {
        let mut col = 0u32;
        let mut x = b.left() + step * 0.6;
        while x < b.right() - step * 0.3 {
            let id = seed * 131 + row * 17 + col;
            if hash(id) < 0.38 {
                let c = accent(id + 9);
                let flick = if animated && hash(id + 77) < 0.12 { 0.55 + 0.45 * (t * 3.0 + id as f32).sin() } else { 1.0 };
                painter.circle_stroke(pos2(x, y), step * 0.2, Stroke::new(1.0, alpha(c, 0.7 * bright * flick)));
                painter.circle_filled(pos2(x, y), step * 0.08, alpha(c, 0.9 * bright * flick));
            }
            x += step;
            col += 1;
        }
        y += step * 1.15;
        row += 1;
    }
}

/// One building: a chip, a capacitor, a heatsink, a ball grid, a resistor stack, or a block of traces.
fn building(painter: &egui::Painter, b: Rect, id: u32, bright: f32, t: f32, animated: bool) {
    let kind = (hash(id * 5 + 1) * 6.0) as u32;
    painter.rect_filled(b, 0.0, Color32::from_rgb((8.0 + 4.0 * bright) as u8, (26.0 + 10.0 * bright) as u8, (30.0 + 10.0 * bright) as u8));
    let edge_col = if hash(id * 5 + 2) < 0.7 { teal() } else { CYAN };
    painter.add(Shape::closed_line(vec![b.left_top(), b.right_top(), b.right_bottom(), b.left_bottom()], Stroke::new(1.0, alpha(edge_col, 0.35 * bright))));
    let step = (b.width() / 4.0).clamp(9.0, 16.0);
    match kind {
        0 => {
            // a chip: pins down both sides and a notch in the corner
            let mut y = b.top() + 5.0;
            while y < b.bottom() - 4.0 {
                painter.line_segment([pos2(b.left() - 3.0, y), pos2(b.left(), y)], Stroke::new(1.5, alpha(teal(), 0.4 * bright)));
                painter.line_segment([pos2(b.right(), y), pos2(b.right() + 3.0, y)], Stroke::new(1.5, alpha(teal(), 0.4 * bright)));
                y += 7.0;
            }
            painter.circle_stroke(b.left_top() + vec2(6.0, 6.0), 2.5, Stroke::new(1.0, alpha(edge_col, 0.5 * bright)));
            paint_vias(painter, b, id, step, t, animated, bright);
        }
        1 => {
            // a capacitor: a rounded top and a striped band
            painter.add(Shape::ellipse_filled(pos2(b.center().x, b.top()), vec2(b.width() * 0.5, b.width() * 0.1), alpha(teal(), 0.18 * bright)));
            let band_top = b.top() + b.height() * 0.1;
            painter.rect_filled(Rect::from_min_max(pos2(b.left(), band_top), pos2(b.right(), band_top + b.width() * 0.2)), 0.0, alpha(CYAN, 0.22 * bright));
            painter.line_segment([pos2(b.center().x - b.width() * 0.15, band_top + b.width() * 0.1), pos2(b.center().x + b.width() * 0.15, band_top + b.width() * 0.1)], Stroke::new(1.5, alpha(CYAN, 0.6 * bright)));
            let lower = Rect::from_min_max(pos2(b.left(), band_top + b.width() * 0.3), b.right_bottom());
            paint_vias(painter, lower, id, step, t, animated, bright);
        }
        2 => {
            // a heatsink: fins
            let mut x = b.left() + 3.0;
            while x < b.right() - 2.0 {
                painter.line_segment([pos2(x, b.top() + 2.0), pos2(x, b.bottom())], Stroke::new(1.0, alpha(teal(), 0.22 * bright)));
                x += 4.0;
            }
            painter.line_segment([b.left_top(), b.right_top()], Stroke::new(2.0, alpha(CYAN, 0.45 * bright)));
        }
        3 => {
            // a ball grid
            let mut y = b.top() + 6.0;
            let mut rows = 0;
            while y < b.bottom() - 4.0 && rows < 14 {
                let mut x = b.left() + 6.0;
                while x < b.right() - 3.0 {
                    painter.circle_filled(pos2(x, y), 1.1, alpha(CYAN, 0.28 * bright));
                    x += 7.0;
                }
                y += 7.0;
                rows += 1;
            }
            let lower = Rect::from_min_max(pos2(b.left(), (b.top() + 6.0 + 7.0 * 14.0).min(b.bottom())), b.right_bottom());
            if lower.height() > step {
                paint_vias(painter, lower, id, step, t, animated, bright);
            }
        }
        4 => {
            // a resistor stack: coloured bands
            for (j, c) in [AMBER, MAGENTA, CYAN, teal()].into_iter().enumerate() {
                let y = b.top() + b.height() * (0.12 + 0.2 * j as f32);
                painter.rect_filled(Rect::from_min_max(pos2(b.left(), y), pos2(b.right(), y + b.height() * 0.05)), 0.0, alpha(c, 0.28 * bright));
            }
            paint_vias(painter, Rect::from_min_max(pos2(b.left(), b.top() + b.height() * 0.8), b.right_bottom()), id, step, t, animated, bright);
        }
        _ => {
            // a block with traces on its face, jogging at 45 degrees, and vias
            for j in 0..3u32 {
                let x = b.left() + b.width() * (0.2 + 0.3 * j as f32);
                let y1 = b.bottom() - b.height() * (0.2 + 0.4 * hash(id * 9 + j));
                let d = b.width() * 0.08;
                painter.add(Shape::line(vec![pos2(x, b.bottom()), pos2(x, y1), pos2(x + d, y1 - d), pos2(x + d, b.top() + 4.0)], Stroke::new(1.0, alpha(edge_col, 0.25 * bright))));
            }
            paint_vias(painter, b, id, step, t, animated, bright);
        }
    }
    // an aerial with a blinking light
    if hash(id * 5 + 7) < 0.25 {
        let top = pos2(b.center().x, b.top() - 10.0 - 16.0 * hash(id * 5 + 8));
        painter.line_segment([pos2(b.center().x, b.top()), top], Stroke::new(1.0, alpha(teal(), 0.4 * bright)));
        let blink = if animated { 0.5 + 0.5 * (t * 2.0 + id as f32).sin() } else { 1.0 };
        painter.circle_filled(top, 1.8, alpha(MAGENTA, 0.9 * blink * bright));
    }
}

/// A row of buildings along `base`: the far row (dim and tall) or the near one (bright and lower).
fn skyline(painter: &egui::Painter, r: Rect, base: f32, layer: u32, t: f32, animated: bool) {
    let (w, h) = (r.width(), base - r.top());
    let (hmin, hmax, wmin, wmax, bright) = if layer == 0 { (0.25f32, 0.60f32, 0.05f32, 0.10f32, 0.55f32) } else { (0.14, 0.42, 0.07, 0.13, 1.0) };
    let mut x = r.left() - w * 0.02;
    let mut i = layer * 100;
    while x < r.right() {
        let bw = w * (wmin + (wmax - wmin) * hash(i * 3 + 11));
        let bh = h * (hmin + (hmax - hmin) * hash(i * 3 + 12));
        building(painter, Rect::from_min_max(pos2(x, base - bh), pos2(x + bw, base)), i, bright, t, animated);
        x += bw * if layer == 0 { 0.92 } else { 1.04 };
        i += 1;
    }
}

// ------------------------------------------------------------------------------------ the city

/// The sky and the skyline in `r`, with the buildings standing on `base` (a y coordinate): the board
/// dark behind everything, haze, traces with pulses running along them, the far buildings, the near
/// ones, and motes drifting up. `animated` false holds it still.
pub fn paint_city(painter: &egui::Painter, r: Rect, base: f32, t: f32, animated: bool) {
    follow_accent();
    let (w, h) = (r.width(), base - r.top());
    // the night sky takes a little of the accent colour too
    let p = teal();
    let horizon = Color32::from_rgb((6.0 + p[0] as f32 * 0.06) as u8, (14.0 + p[1] as f32 * 0.075) as u8, (18.0 + p[2] as f32 * 0.075) as u8);
    gradient(painter, r, Color32::from_rgb(3, 12, 18), horizon);
    soft_ellipse(painter, pos2(r.left() + w * 0.5, base - h * 0.35), vec2(w * 0.6, h * 0.6), teal(), 0.10);
    soft_ellipse(painter, pos2(r.left() + w * 0.8, base - h * 0.2), vec2(w * 0.35, h * 0.4), CYAN, 0.07);
    soft_ellipse(painter, pos2(r.left() + w * 0.15, base - h * 0.1), vec2(w * 0.3, h * 0.3), AMBER, 0.06);
    // traces across the sky
    for i in 0..10u32 {
        let pts = route(i + 1, r, r.top(), base);
        paint_trace(painter, &pts, accent(i + 20), i * 3, t, animated, 1.0);
    }
    skyline(painter, r, base, 0, t, animated);
    skyline(painter, r, base, 1, t, animated);
    // the road along the foot of the buildings
    painter.line_segment([pos2(r.left(), base), pos2(r.right(), base)], Stroke::new(1.5, alpha(teal(), 0.5)));
    // motes in the haze
    for i in 0..28u32 {
        let x = r.left() + w * hash(i * 3 + 900);
        let speed = 0.01 + 0.02 * hash(i * 3 + 901);
        let rise = if animated { (hash(i * 3 + 902) - t * speed).rem_euclid(1.0) } else { hash(i * 3 + 902) };
        let sway = if animated { (t * 0.5 + i as f32).sin() * 4.0 } else { 0.0 };
        painter.circle_filled(pos2(x + sway, base - h * rise), 1.2, alpha(accent(i + 60), 0.35 * (1.0 - rise * 0.6)));
    }
}

/// Circuit City behind the whole window, for the backdrop style of that name. `t` is the time in
/// seconds (0 holds it still); `strength` is how strong the backdrop is (the text sits right on it, so
/// it is dimmed).
pub fn paint_window(ui: &egui::Ui, t: f32, strength: f32) {
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    paint_city(&painter, rect, rect.bottom() - rect.height() * 0.04, t, t > 0.0);
    painter.rect_filled(rect, 0.0, alpha([4, 14, 18], (0.30 + (1.0 - strength.clamp(0.0, 1.0)) * 0.6).min(0.8)));
}
