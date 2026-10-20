// The cat king of the Gold Edition's Info tab: the colosseum picture with the live king drawn on
// top (see king.rs for what he does). Clicking him makes him laugh.
use crate::{gold, info, king};
use eframe::egui::{self, pos2, vec2, Color32, Shape, Stroke};

fn gold_colour(value: f32, alpha: f32) -> Color32 {
    let [r, g, b] = gold::ramp(value);
    Color32::from_rgba_unmultiplied(r, g, b, (alpha.clamp(0.0, 1.0) * 255.0) as u8)
}

/// Draws the colosseum and the king. `laughing_since` is when the current laugh began (egui time)
/// and is cleared when it is over; a click on him sets it. `animate` is false while the window is
/// hidden; `motion` is the Options switch (off: he holds still, but a click still makes him laugh).
pub fn paint_king(
    ui: &mut egui::Ui,
    arena: &egui::TextureHandle,
    laughing_since: &mut Option<f64>,
    animate: bool,
    motion: bool,
) {
    let (w, h) = info::fit_logo(ui.available_width(), king::PICTURE, ui.ctx().pixels_per_point());
    if w <= 0.0 {
        return;
    }
    let (rect, response) = ui.allocate_exact_size(vec2(w, h), egui::Sense::click());
    let scale = w / king::PICTURE.0;
    let at = |p: (f32, f32)| pos2(rect.min.x + p.0 * scale, rect.min.y + p.1 * scale);

    let now = ui.input(|i| i.time);
    if let Some(c) = *laughing_since {
        if now - c >= f64::from(king::REACTION) {
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

    let painter = ui.painter().with_clip_rect(rect);
    painter.image(
        arena.id(),
        rect,
        egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    let pose = king::pose((now % 3600.0) as f32, since, motion);
    for poly in king::scene(&pose) {
        let points: Vec<egui::Pos2> = poly.points.iter().map(|&p| at(p)).collect();
        if points.len() < 2 {
            continue;
        }
        if poly.fill != king::Tone::None && points.len() >= 3 {
            // filled as a fan from the middle (the shapes are all round the middle)
            let middle = points.iter().fold(egui::Vec2::ZERO, |sum, p| sum + p.to_vec2()) / points.len() as f32;
            let fill = gold_colour(poly.fill.value(), 1.0);
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(middle.to_pos2(), fill);
            for p in &points {
                mesh.colored_vertex(*p, fill);
            }
            let n = points.len();
            for i in 0..n {
                mesh.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
            }
            painter.add(Shape::mesh(mesh));
        }
        if poly.glow > 0.0 {
            let halo = Stroke::new((7.0 * scale).max(2.0), gold_colour(0.9, 0.16 * poly.glow.min(1.0)));
            painter.add(if poly.closed { Shape::closed_line(points.clone(), halo) } else { Shape::line(points.clone(), halo) });
        }
        if poly.line.1 > 0.0 {
            let stroke = Stroke::new((poly.line.1 * scale).max(1.0), gold_colour(poly.line.0, 1.0));
            painter.add(if poly.closed { Shape::closed_line(points, stroke) } else { Shape::line(points, stroke) });
        }
    }
    if let Some(s) = since {
        for (text, pos, strength, size) in king::has(s) {
            let font = egui::FontId::proportional(size * scale);
            painter.text(at((pos.0 + 2.0, pos.1 + 2.0)), egui::Align2::CENTER_CENTER, text, font.clone(), Color32::from_black_alpha((200.0 * strength) as u8));
            painter.text(at(pos), egui::Align2::CENTER_CENTER, text, font, gold_colour(0.95, strength));
        }
        if let Some((text, fade)) = king::speech(s) {
            let font = egui::FontId::proportional(40.0 * scale);
            let centre = at(king::SPEECH_AT);
            painter.text(centre + vec2(2.0, 2.0), egui::Align2::CENTER_CENTER, &text, font.clone(), Color32::from_black_alpha((220.0 * fade) as u8));
            painter.text(centre, egui::Align2::CENTER_CENTER, &text, font, gold_colour(0.97, fade));
        }
    }
    if animate && (motion || since.is_some()) {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }
}
