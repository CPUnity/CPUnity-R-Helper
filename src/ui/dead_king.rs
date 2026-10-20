// The dead monarch king, The Void's backdrop: the cat king (see king.rs) with his gold gone black
// and his eyes dead, standing at the top of the page over the glitching stars. Clicking him makes
// him laugh, slowly, while his head turns all the way round on his neck and everything glitches.
// His eyes are pitch black with a white X in each, and round him there is a glow of life and death
// (see the end of king.rs for what moves).
use super::backdrop::soft_ellipse;
use crate::{gold, info, king, theme};
use eframe::egui::{self, pos2, vec2, Color32, Pos2, Shape, Stroke};

pub(super) fn dead_colour(value: f32, alpha: f32) -> Color32 {
    let [r, g, b] = king::dead_ramp(value);
    Color32::from_rgba_unmultiplied(r, g, b, (alpha.clamp(0.0, 1.0) * 255.0) as u8)
}

/// The tallest the king is drawn, in points.
const MAX_HEIGHT: f32 = 340.0;

/// Draws the dead king at the top of the page. `laughing_since` is when the current laugh began
/// (egui time) and is cleared when it is over; a click on him sets it. Returns how hard everything
/// is glitching (0 to 1), which the caller turns into a glitch over the whole window.
pub fn paint_dead_king(ui: &mut egui::Ui, laughing_since: &mut Option<f64>, animate: bool, motion: bool) -> f32 {
    let available = ui.available_width();
    let (full, _) = info::fit_logo(available, king::PICTURE, ui.ctx().pixels_per_point());
    if full <= 0.0 {
        return 0.0;
    }
    let w = full.min(MAX_HEIGHT * king::PICTURE.0 / king::PICTURE.1);
    let h = w * king::PICTURE.1 / king::PICTURE.0;
    let (stage, _) = ui.allocate_exact_size(vec2(available, h), egui::Sense::hover());
    let rect = egui::Rect::from_center_size(stage.center(), vec2(w, h));
    let response = ui.interact(rect, ui.id().with("dead_king"), egui::Sense::click());
    let scale = w / king::PICTURE.0;
    let at = |p: (f32, f32)| pos2(rect.min.x + p.0 * scale, rect.min.y + p.1 * scale);

    let now = ui.input(|i| i.time);
    if let Some(c) = *laughing_since {
        if now - c >= f64::from(king::DEAD_REACTION) {
            *laughing_since = None;
        }
    }
    let since = laughing_since.map(|c| (now - c) as f32);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.clicked() && since.is_none() {
        *laughing_since = Some(now);
    }
    let since = laughing_since.map(|c| (now - c) as f32);
    let level = king::dead_glitch(since);
    let t = (now % 3600.0) as f32;

    // how torn the picture is right now
    let frame = if level > 0.03 {
        gold::glitch_frame_with(t, level)
    } else {
        gold::GlitchFrame { block: None, bands: Vec::new(), flash: 0.0 }
    };
    let pose = king::dead_pose(t, since, motion);
    let polys: Vec<(Vec<Pos2>, king::Poly)> = king::scene(&pose)
        .into_iter()
        .map(|poly| {
            let points = poly.points.iter().map(|&p| at(king::glitch_point(p, &frame))).collect();
            (points, poly)
        })
        .collect();

    let painter = ui.painter();
    // the glow stands still if the person asked for no motion (and he is not laughing)
    let glow_t = if motion || since.is_some() { t } else { 0.3 };
    paint_glow(painter, &polys, glow_t, scale, level, &at);
    // a cyan and a magenta ghost of him, further apart the worse it gets
    if level > 0.12 {
        let spread = (4.0 + 26.0 * level) * scale * 2.0;
        let jump = if (t * 14.0) as u32 % 3 == 0 { 1.6 } else { 1.0 };
        for (dx, tone) in [(-spread * jump, theme::VOID_CYAN), (spread * jump, theme::VOID_MAGENTA)] {
            for (points, poly) in &polys {
                if poly.fill != king::Tone::None && points.len() >= 3 {
                    fill_fan(painter, points, tone.gamma_multiply(0.22 * level), vec2(dx, 0.0));
                }
            }
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
    if let Some(s) = since {
        for (text, pos, strength, size) in king::dead_has(s) {
            let font = egui::FontId::proportional(size * scale);
            let spot = at(king::glitch_point(pos, &frame));
            painter.text(spot + vec2(-2.0, 0.0), egui::Align2::CENTER_CENTER, text, font.clone(), theme::VOID_CYAN.gamma_multiply(0.5 * strength));
            painter.text(spot + vec2(2.0, 0.0), egui::Align2::CENTER_CENTER, text, font.clone(), theme::VOID_MAGENTA.gamma_multiply(0.5 * strength));
            painter.text(spot, egui::Align2::CENTER_CENTER, text, font, Color32::from_rgba_unmultiplied(170, 140, 235, (strength * 255.0) as u8));
        }
    }
    if frame.flash > 0.0 {
        painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(120, 70, 230, (frame.flash * 255.0) as u8));
    }
    if animate && (motion || since.is_some()) {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }
    level
}

/// A dead eye: pitch black inside a thin dull rim, with a white X burnt into it.
pub(super) fn paint_dead_eye(painter: &egui::Painter, points: &[Pos2], rim: f32, scale: f32) {
    fill_fan(painter, points, Color32::BLACK, vec2(0.0, 0.0));
    painter.add(Shape::closed_line(points.to_vec(), Stroke::new(rim, Color32::from_rgb(70, 60, 64))));
    let outline: Vec<(f32, f32)> = points.iter().map(|p| (p.x, p.y)).collect();
    let white = Color32::from_rgb(244, 240, 255);
    let width = (3.4 * scale).max(2.0);
    let middle = centre(points);
    soft_ellipse(painter, middle, vec2(30.0, 22.0) * scale.max(0.5), [210, 200, 255], 0.22);
    for (from, to) in king::eye_cross(&outline) {
        let (from, to) = (pos2(from.0, from.1), pos2(to.0, to.1));
        painter.add(Shape::line_segment([from, to], Stroke::new(width, white)));
        // round the ends so it looks burnt in rather than cut out
        painter.circle_filled(from, width * 0.5, white);
        painter.circle_filled(to, width * 0.5, white);
    }
}

/// The glow of life and death round him: a cold dark violet behind everything, the same violet and
/// a sick mint hugging his outline (swelling with every beat of the heart he does not have), rings
/// going out from his chest, lights lifting off him and ash falling, and a pale copy of him rising
/// and dissolving. `polys` are his shapes on the screen; `level` is how hard he is glitching, which
/// makes all of it stronger.
fn paint_glow(painter: &egui::Painter, polys: &[(Vec<Pos2>, king::Poly)], t: f32, scale: f32, level: f32, at: &dyn Fn((f32, f32)) -> Pos2) {
    let beat = king::dead_beat(t);
    let boost = 1.0 + 0.5 * level;
    let alpha = |c: [u8; 3], a: f32| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0) as u8);
    let heart = at(king::HEART);

    // death: a huge cold bloom, darker than the sky
    soft_ellipse(painter, heart + vec2(0.0, -20.0 * scale), vec2(600.0, 620.0) * scale, king::DEATH_DEEP, (0.5 + 0.2 * beat) * boost);

    // the pale copy of him rising off him
    let (up, bright) = king::dead_soul(t);
    if bright > 0.004 {
        for (points, poly) in polys {
            if poly.fill != king::Tone::None && poly.fill != king::Tone::Eye && poly.fill.value() > 0.15 && points.len() >= 3 {
                fill_fan(painter, points, alpha(king::LIFE, bright * boost), vec2(0.0, -up * scale));
            }
        }
    }

    // the glow hugging his outline: violet further out, mint close in, a few steps each so it fades
    let passes: [([u8; 3], f32, f32); 7] = [
        (king::DEATH, 70.0, 0.035),
        (king::DEATH, 46.0, 0.045),
        (king::DEATH, 26.0, 0.06),
        (king::LIFE, 24.0, 0.035),
        (king::LIFE, 14.0, 0.06),
        (king::LIFE, 8.0, 0.09),
        (king::LIFE, 3.5, 0.16),
    ];
    for (points, poly) in polys {
        if poly.fill == king::Tone::None || poly.fill == king::Tone::Eye || points.len() < 3 {
            continue;
        }
        for (colour, width, strength) in passes {
            let swell = if colour == king::LIFE { 0.55 + 0.9 * beat } else { 1.0 };
            let stroke = Stroke::new((width * scale).max(1.0), alpha(colour, strength * swell * boost));
            painter.add(Shape::closed_line(points.clone(), stroke));
        }
    }

    // rings out from his chest, mint turning to violet as they go
    for (p, a) in king::dead_rings(t) {
        if a < 0.01 {
            continue;
        }
        let r = (230.0 + p * 340.0) * scale;
        let colour = [0, 1, 2].map(|i| (king::LIFE[i] as f32 * (1.0 - p) + king::DEATH[i] as f32 * p) as u8);
        let ring: Vec<Pos2> = (0..72).map(|k| {
            let a = std::f32::consts::TAU * k as f32 / 72.0;
            heart + vec2(a.cos() * r, a.sin() * r * 1.08)
        }).collect();
        painter.add(Shape::closed_line(ring.clone(), Stroke::new((9.0 * scale).max(3.0), alpha(colour, 0.12 * a * boost))));
        painter.add(Shape::closed_line(ring, Stroke::new((2.5 * scale).max(1.0), alpha(colour, 0.6 * a * boost))));
    }

    // lights lifting off him, and ash falling
    for mote in king::dead_motes(t) {
        if mote.alpha < 0.02 {
            continue;
        }
        let colour = if mote.rising { king::LIFE } else { king::DEATH };
        let size = (mote.size * scale * 2.0).max(1.5);
        let spot = at(mote.pos);
        if mote.rising {
            painter.line_segment([spot + vec2(size / 2.0, size), spot + vec2(size / 2.0, size + 30.0 * scale)], Stroke::new(1.0, alpha(colour, mote.alpha * 0.3)));
        }
        painter.rect_filled(egui::Rect::from_min_size(spot, vec2(size, size)), 0.0, alpha(colour, mote.alpha * 0.9 * boost));
    }
}

pub(super) fn centre(points: &[Pos2]) -> Pos2 {
    (points.iter().fold(egui::Vec2::ZERO, |sum, p| sum + p.to_vec2()) / points.len() as f32).to_pos2()
}

/// Fills a shape that is all round its middle (the king's are) as a fan of triangles, moved by
/// `offset`.
pub(super) fn fill_fan(painter: &egui::Painter, points: &[Pos2], colour: Color32, offset: egui::Vec2) {
    let middle = centre(points);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(middle + offset, colour);
    for p in points {
        mesh.colored_vertex(*p + offset, colour);
    }
    let n = points.len();
    for i in 0..n {
        mesh.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
    }
    painter.add(Shape::mesh(mesh));
}
