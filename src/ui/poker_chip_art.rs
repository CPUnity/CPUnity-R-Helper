// The chips of the Devil's Table, drawn: round black CR-Tokens, each with its number on the top and a
// coloured rim and edge that tell the kinds apart at a glance (1 bone, 5 blood, 25 emerald, 50 soul
// blue, 100 gold, 500 violet, 1000 ember). What an amount is made of is in poker_chips.rs.
use crate::poker_chips as chips;
use crate::poker_table::Chips;
use eframe::egui::epaint::Vertex;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, TextureHandle, TextureId};

/// The CR logo, cut round, for the face of a chip.
const FACE_PNG: &[u8] = include_bytes!("../../assets/chip-face.png");

/// Loads the logo for the chips' faces.
pub fn load_face(ctx: &egui::Context) -> Option<TextureHandle> {
    let img = image::load_from_memory(FACE_PNG).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(ctx.load_texture("poker_chip_face", egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()), egui::TextureOptions::LINEAR))
}

/// The picture laid on an ellipse (the middle of the picture at `c`).
fn picture(p: &Painter, tex: TextureId, c: Pos2, rx: f32, ry: f32) {
    const N: usize = 32;
    let mut m = Mesh::with_texture(tex);
    m.vertices.push(Vertex { pos: c, uv: pos2(0.5, 0.5), color: Color32::WHITE });
    for i in 0..N {
        let a = std::f32::consts::TAU * i as f32 / N as f32;
        m.vertices.push(Vertex { pos: c + vec2(a.cos() * rx, a.sin() * ry), uv: pos2(0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin()), color: Color32::WHITE });
    }
    for i in 0..N as u32 {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % N as u32);
    }
    p.add(Shape::mesh(m));
}

/// Most chips drawn in one tower.
const MAX_TOWER: Chips = 14;

fn tone(rgb: [u8; 3], a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// One chip, its top face centred on `c`. `k` is points to a board pixel (a chip is 26 across and
/// 3.4 thick); `label` writes its number on it (the top chip of a tower).
pub fn paint_chip(p: &Painter, c: Pos2, k: f32, denom: Chips, label: bool, face: Option<TextureId>) {
    let accent = chips::colour(denom);
    let (rx, ry, th) = (13.0 * k, 5.6 * k, 3.4 * k);
    let dark = Color32::from_rgb(20, 14, 18);
    // the edge: a black band with a stripe of the kind's colour every so often
    p.add(Shape::ellipse_filled(c + vec2(0.0, th), vec2(rx, ry), dark));
    p.add(Shape::rect_filled(Rect::from_min_max(pos2(c.x - rx, c.y), pos2(c.x + rx, c.y + th)), 0.0, dark));
    for a in [-1.15f32, -0.58, 0.0, 0.58, 1.15] {
        let x = c.x + rx * a.sin();
        let w = (rx * 0.2 * a.cos()).max(rx * 0.06);
        let y = c.y + ry * a.cos();
        p.add(Shape::rect_filled(Rect::from_min_max(pos2(x - w / 2.0, y), pos2(x + w / 2.0, y + th)), 0.0, tone(accent, 0.95)));
    }
    p.add(Shape::ellipse_stroke(c + vec2(0.0, th), vec2(rx, ry), Stroke::new((0.8 * k).max(0.5), Color32::from_black_alpha(160))));
    // the top: a rim in the kind's colour with its bone-white inlays, and (on the top chip) the CR logo
    // in the middle with the number over the wordmark
    p.add(Shape::ellipse_filled(c, vec2(rx, ry), tone(accent, 0.95)));
    for i in 0..8 {
        let a = std::f32::consts::TAU * (i as f32 + 0.5) / 8.0;
        let at = c + vec2(a.cos() * rx * 0.88, a.sin() * ry * 0.88);
        p.add(Shape::ellipse_filled(at, vec2((rx * 0.13).max(0.7), (ry * 0.2).max(0.45)), Color32::from_rgb(232, 226, 210)));
    }
    p.add(Shape::ellipse_filled(c, vec2(rx * 0.76, ry * 0.76), Color32::from_rgb(14, 8, 10)));
    p.add(Shape::ellipse_stroke(c, vec2(rx, ry), Stroke::new((1.0 * k).max(0.6), Color32::from_black_alpha(170))));
    match (label, face) {
        (true, Some(tex)) => {
            picture(p, tex, c, rx * 0.74, ry * 0.74);
            let size = (rx * 0.6).max(6.5);
            let at = c + vec2(0.0, ry * 0.36);
            for d in [vec2(-0.8, 0.0), vec2(0.8, 0.0), vec2(0.0, -0.8), vec2(0.0, 0.8)] {
                p.text(at + d * k, Align2::CENTER_CENTER, chips::label(denom), FontId::proportional(size), Color32::from_black_alpha(230));
            }
            p.text(at, Align2::CENTER_CENTER, chips::label(denom), FontId::proportional(size), Color32::from_rgb(244, 238, 224));
        }
        (true, None) => {
            p.add(Shape::ellipse_stroke(c, vec2(rx * 0.74, ry * 0.74), Stroke::new((0.9 * k).max(0.5), tone(accent, 0.7))));
            p.text(c + vec2(0.0, -0.2 * k), Align2::CENTER_CENTER, chips::label(denom), FontId::proportional((rx * 0.78).max(7.0)), tone(accent, 1.0));
        }
        _ => {}
    }
}

/// `amount` as towers of chips standing side by side, the biggest kinds on the left, with the middle of
/// their base line at `at`. `reveal` (0 to 1) puts the chips down one after another. Returns how wide
/// the row is.
pub fn paint_towers(p: &Painter, at: Pos2, k: f32, amount: Chips, reveal: f32, face: Option<TextureId>) -> f32 {
    let stack = chips::breakdown(amount);
    if stack.is_empty() {
        return 0.0;
    }
    let gap = 28.0 * k;
    let width = gap * (stack.len() - 1) as f32;
    let mut budget = (chips::count(&stack) as f32 * reveal.clamp(0.0, 1.0)).ceil() as Chips;
    for (i, &(denom, n)) in stack.iter().enumerate() {
        let draw = n.min(MAX_TOWER).min(budget);
        if draw == 0 {
            break;
        }
        budget -= draw;
        // each tower a little nearer or further, so they do not stand in a ruler's line
        let base = pos2(at.x - width / 2.0 + i as f32 * gap, at.y + [0.0, -3.5, 2.0][i % 3] * k);
        p.add(Shape::ellipse_filled(base + vec2(0.0, 5.0 * k), vec2(15.0 * k, 6.0 * k), Color32::from_black_alpha(110)));
        for j in 0..draw {
            paint_chip(p, base + vec2(0.0, -(j as f32) * 3.4 * k), k, denom, j + 1 == draw, face);
        }
    }
    width + 26.0 * k
}
