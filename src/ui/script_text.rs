// Text in The Void's script (see void_script.rs): words made of stroke-drawn signs, which decode into
// plain letters while the pointer is on them.
use crate::theme::{VOID_CYAN, VOID_MAGENTA};
use crate::void_script::{self, Letter};
use eframe::egui::{self, pos2, vec2, Color32, Rect, Stroke};

const VIOLET: Color32 = Color32::from_rgb(155, 107, 255);
const BRIGHT: Color32 = Color32::from_rgb(205, 178, 255);

/// Draws `text` (with `**bold**` parts) in the script at `size` points, wrapping like normal text.
/// `strong` is for titles: brighter signs.
pub fn show(ui: &mut egui::Ui, text: &str, size: f32, strong: bool) {
    let adv = size * 0.62;
    let height = size * 1.28;
    let spans = crate::doc::spans(text);
    let key = ui.id().with(text);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(adv * 0.7, 3.0);
        let mut index = 0usize;
        for (piece, bold) in &spans {
            for word in piece.split_whitespace() {
                word_widget(ui, word, key.with(index), size, adv, height, strong || *bold);
                index += 1;
            }
        }
    });
}

fn word_widget(ui: &mut egui::Ui, word: &str, id: egui::Id, size: f32, adv: f32, height: f32, strong: bool) {
    let chars: Vec<char> = word.chars().collect();
    let (rect, response) = ui.allocate_exact_size(vec2(adv * chars.len() as f32, height), egui::Sense::hover());
    // how far along it is is kept between frames
    let before: f32 = ui.data(|d| d.get_temp(id)).unwrap_or(0.0);
    let dt = ui.input(|i| i.stable_dt);
    let amount = void_script::step_amount(before, response.hovered(), dt);
    if amount != before {
        ui.data_mut(|d| d.insert_temp(id, amount));
        ui.ctx().request_repaint();
    }
    if !ui.is_rect_visible(rect) {
        return;
    }
    let time = ui.input(|i| i.time) as f32;
    let painter = ui.painter();
    if amount > 0.0 {
        painter.rect(
            rect.expand2(vec2(adv * 0.25, 1.0)),
            egui::CornerRadius::same(3),
            VIOLET.gamma_multiply(0.18 * amount),
            Stroke::new(1.0, VIOLET.gamma_multiply(0.55 * amount)),
            egui::StrokeKind::Inside,
        );
    }
    let font = egui::FontId::monospace(size);
    let sign_colour = if strong { BRIGHT } else { VIOLET };
    for (i, c) in chars.iter().enumerate() {
        let cell = Rect::from_min_size(rect.min + vec2(adv * i as f32, 0.0), vec2(adv, height));
        let state = if void_script::is_sign(*c) { void_script::letter(i, chars.len(), amount, time) } else { Letter::Plain };
        match state {
            Letter::Sign => paint_sign(painter, cell, *c, sign_colour, 1.0),
            Letter::Flicker(other) => paint_sign(painter, cell, other, VOID_CYAN, 1.0),
            Letter::Plain => {
                let decoded = void_script::is_sign(*c);
                let colour = if decoded { Color32::WHITE } else { crate::theme::text() };
                if decoded {
                    // the split of cyan and magenta that marks a decoded letter
                    painter.text(cell.center() + vec2(-1.2, 0.0), egui::Align2::CENTER_CENTER, c.to_string(), font.clone(), VOID_CYAN.gamma_multiply(0.85));
                    painter.text(cell.center() + vec2(1.2, 0.0), egui::Align2::CENTER_CENTER, c.to_string(), font.clone(), VOID_MAGENTA.gamma_multiply(0.85));
                }
                painter.text(cell.center(), egui::Align2::CENTER_CENTER, c.to_string(), font.clone(), colour);
            }
        }
    }
}

/// One sign in `cell`: violet strokes with a cyan and a magenta copy a pixel to either side.
pub(crate) fn paint_sign(painter: &egui::Painter, cell: Rect, c: char, colour: Color32, width_scale: f32) {
    let (strokes, dot) = void_script::sign(c);
    let inner = Rect::from_min_max(
        pos2(cell.left() + cell.width() * 0.12, cell.top() + cell.height() * 0.14),
        pos2(cell.right() - cell.width() * 0.12, cell.bottom() - cell.height() * 0.14),
    );
    let at = |p: (f32, f32)| pos2(inner.left() + p.0 * inner.width(), inner.top() + p.1 * inner.height());
    let w = (cell.height() / 17.0).clamp(1.1, 2.2) * width_scale;
    for (shift, tone, alpha) in [(-0.9, VOID_CYAN, 0.5), (0.9, VOID_MAGENTA, 0.5), (0.0, colour, 1.0)] {
        let stroke = Stroke::new(w, tone.gamma_multiply(alpha));
        for (a, b) in &strokes {
            painter.line_segment([at(*a) + vec2(shift, 0.0), at(*b) + vec2(shift, 0.0)], stroke);
        }
        if let Some(d) = dot {
            painter.circle_filled(at(d) + vec2(shift, 0.0), w * 0.8, tone.gamma_multiply(alpha));
        }
    }
}

/// Lines of the script drawn without a widget (for a speech bubble): `lines` laid out from `top_left`
/// at `size` points a letter, only the first `shown` letters of them (counted over all the lines,
/// spaces included). Every word decodes on its own, as the words of the Info text do: the
/// pointer on a word turns that word (and only that one) into letters, and it goes back to signs
/// when the pointer leaves. `id` tells this block from other ones; where each word is in decoding is
/// kept between frames. Only the first `shown` letters are drawn, all at `alpha`.
pub fn paint_lines_by_word(
    ctx: &egui::Context,
    painter: &egui::Painter,
    id: egui::Id,
    top_left: egui::Pos2,
    lines: &[String],
    size: f32,
    shown: usize,
    time: f32,
    alpha: f32,
) -> egui::Vec2 {
    let adv = size * 0.62;
    let height = size * 1.28;
    let font = egui::FontId::monospace(size);
    let pointer = ctx.input(|i| i.pointer.hover_pos());
    let dt = ctx.input(|i| i.stable_dt);
    let mut count = 0usize;
    let mut widest = 0usize;
    let mut word_no = 0usize;
    for (row, line) in lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        widest = widest.max(chars.len());
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == ' ' {
                count += 1;
                i += 1;
                continue;
            }
            let start = i;
            while i < chars.len() && chars[i] != ' ' {
                i += 1;
            }
            let word = &chars[start..i];
            let first = count;
            count += word.len();
            word_no += 1;
            let visible = shown.saturating_sub(first).min(word.len());
            if visible == 0 {
                continue;
            }
            let rect = Rect::from_min_size(top_left + vec2(adv * start as f32, height * row as f32), vec2(adv * word.len() as f32, height));
            let hovered = alpha > 0.3 && pointer.is_some_and(|p| rect.expand2(vec2(adv * 0.25, 1.0)).contains(p));
            let word_id = id.with(word_no);
            let before: f32 = ctx.data(|d| d.get_temp(word_id)).unwrap_or(0.0);
            let amount = void_script::step_amount(before, hovered, dt);
            if amount != before {
                ctx.data_mut(|d| d.insert_temp(word_id, amount));
                ctx.request_repaint();
            }
            if amount > 0.0 {
                painter.rect(
                    rect.expand2(vec2(adv * 0.25, 1.0)),
                    egui::CornerRadius::same(3),
                    VIOLET.gamma_multiply(0.18 * amount * alpha),
                    Stroke::new(1.0, VIOLET.gamma_multiply(0.55 * amount * alpha)),
                    egui::StrokeKind::Inside,
                );
            }
            for (j, c) in word.iter().enumerate().take(visible) {
                let cell = Rect::from_min_size(rect.min + vec2(adv * j as f32, 0.0), vec2(adv, height));
                let state = if void_script::is_sign(*c) { void_script::letter(j, word.len(), amount, time) } else { Letter::Plain };
                match state {
                    Letter::Sign => paint_sign(painter, cell, *c, BRIGHT.gamma_multiply(alpha), 1.0),
                    Letter::Flicker(other) => paint_sign(painter, cell, other, VOID_CYAN.gamma_multiply(alpha), 1.0),
                    Letter::Plain => {
                        if void_script::is_sign(*c) {
                            painter.text(cell.center() + vec2(-1.2, 0.0), egui::Align2::CENTER_CENTER, c.to_string(), font.clone(), VOID_CYAN.gamma_multiply(0.85 * alpha));
                            painter.text(cell.center() + vec2(1.2, 0.0), egui::Align2::CENTER_CENTER, c.to_string(), font.clone(), VOID_MAGENTA.gamma_multiply(0.85 * alpha));
                        }
                        painter.text(cell.center(), egui::Align2::CENTER_CENTER, c.to_string(), font.clone(), Color32::WHITE.gamma_multiply(alpha));
                    }
                }
            }
        }
        count += 1; // the space the line break stands for
    }
    vec2(adv * widest as f32, height * lines.len() as f32)
}
