// The picture behind every tab, drawn in the accent colour.
use crate::angels;
use crate::dragons;
use crate::father;
use crate::info;
use crate::platinum;
use crate::smoker;
use crate::theme;
use crate::void;
use eframe::egui;
use egui::{pos2, vec2, Color32, Shape, Stroke};

/// The strength-adjusted accent colour the picture is drawn in.
pub fn tint(strength: f32) -> Color32 {
    let accent = theme::accent();
    let [r, g, b] = info::backdrop_tint([accent.r(), accent.g(), accent.b()]);
    Color32::from_rgb(r, g, b).gamma_multiply(strength.clamp(0.0, 1.0))
}

/// Paints a one-picture style over the whole visible panel. It is drawn first, before the header,
/// tabs and cards, so they all sit on top of it, and it stays put while the content scrolls.
pub fn paint(ui: &egui::Ui, textures: &[egui::TextureHandle], strength: f32) {
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 || textures.len() != 1 {
        return;
    }
    let px = textures[0].size_vec2();
    let [u0, v0, u1, v1] = info::cover_uv((px.x, px.y), (rect.width(), rect.height()));
    ui.painter().image(
        textures[0].id(),
        rect,
        egui::Rect::from_min_max(pos2(u0, v0), pos2(u1, v1)),
        tint(strength),
    );
}

/// Father Time as a still picture for a preview: the clock face, the silhouette in the background
/// colour, then the dragon, all three covering the visible rectangle. No hands, no reactions.
pub fn paint_father_still(ui: &egui::Ui, textures: &[egui::TextureHandle], strength: f32) {
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 || textures.len() != 3 {
        return;
    }
    let px = textures[0].size_vec2();
    let [u0, v0, u1, v1] = info::cover_uv((px.x, px.y), (rect.width(), rect.height()));
    let uv = egui::Rect::from_min_max(pos2(u0, v0), pos2(u1, v1));
    let painter = ui.painter();
    painter.image(textures[0].id(), rect, uv, tint(strength));
    painter.image(textures[1].id(), rect, uv, theme::bg());
    painter.image(textures[2].id(), rect, uv, tint(strength));
}

/// Father Time. Unlike the other styles he is not behind everything: he stands at the top of the
/// page, in the place of the logo, and everything else starts below the clock. Call this first
/// inside the scroll area; it takes the room it needs. `clicked_at` is when his beard was last
/// clicked (egui time), `animate` is false while the window is hidden so nothing keeps redrawing.
/// With `smoker` (the Gold Edition) he sits smoking instead of reacting to his beard; see `smoker.rs`.
/// `staff_textures` are the front and mask layers without his staff, and the staff (`gold.rs`): once
/// the cigarette has been thrown he holds the staff out, drawn by hand, and the game starts when it
/// is taken.
pub fn paint_father(
    ui: &mut egui::Ui,
    textures: &[egui::TextureHandle],
    strength: f32,
    clicked_at: &mut Option<f64>,
    smoker: Option<&mut smoker::Smoker>,
    animate: bool,
    motion: bool,
    staff_textures: &[egui::TextureHandle],
    void_father: Option<&VoidTextures>,
) -> FatherEvents {
    let width = ui.available_width();
    let scale = father::scale_for(width);
    let (stage, _) = ui.allocate_exact_size(vec2(width, father::stage_height(scale)), egui::Sense::hover());
    if textures.len() != 3 {
        return FatherEvents::default();
    }
    let mut smoker = smoker;
    // In The Void he has no staff (so the layers without it are needed) and falls apart
    let void_father = void_father.filter(|_| staff_textures.len() == 3);
    let is_void = void_father.is_some();
    let now = ui.input(|i| i.time);
    // Standing still: one frozen frame of the rain, the orbs and the sand; no blinking. The clock's
    // hands still keep time, and a click on the beard still plays out.
    let t = if motion { (now % 3600.0) as f32 } else { STILL };
    let (ox, oy) = father::origin((stage.min.x, stage.min.y), width, scale);
    let at = |p: (f32, f32)| pos2(ox + p.0 * scale, oy + p.1 * scale);
    let colour = tint(strength);
    let background = theme::bg();

    // what the beard is doing: untouched, or reacting to a click
    if let Some(c) = *clicked_at {
        if now - c >= f64::from(father::REACTION) {
            *clicked_at = None;
        }
    }
    let since = clicked_at.map_or(-1.0, |c| (now - c) as f32);
    let reaction = father::reaction(since);

    // the beard can be clicked (the widgets that come later in the page are above this one)
    if smoker.is_none() && !is_void {
        let beard = egui::Rect::from_min_max(at(father::BEARD_BOX.0), at(father::BEARD_BOX.1));
        let response = ui.interact(beard, ui.id().with("father_beard"), egui::Sense::click());
        if reaction.is_none() {
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                *clicked_at = Some(now);
            }
        }
    }

    // the secret: five quick clicks on the middle of the clock start the game (see game1.rs)
    let mut start_game = false;
    let hub = egui::Rect::from_center_size(at(info::CLOCK_CENTER), vec2(110.0 * scale, 110.0 * scale));
    let hub_id = ui.id().with("father_hub");
    if ui.interact(hub, hub_id, egui::Sense::click()).clicked() {
        let (mut count, last) = ui.data(|d| d.get_temp::<(u8, f64)>(hub_id)).unwrap_or((0, 0.0));
        if now - last > 2.0 {
            count = 0;
        }
        count += 1;
        if count >= 5 {
            count = 0;
            start_game = true;
        }
        ui.data_mut(|d| d.insert_temp(hub_id, (count, now)));
    }

    // The Void: a click on his face and he speaks, in signs (one of three things, one after the other in order). Where he is in it is kept between frames.
    let say_id = ui.id().with("void_father_say");
    let last_id = ui.id().with("void_father_last");
    let mut say: Option<(f64, usize)> = None;
    if is_void {
        say = ui.data(|d| d.get_temp::<(f64, usize)>(say_id));
        if let Some((start, line)) = say {
            if (now - start) as f32 >= father::void_speech_len(father::VOID_LINES[line].chars().count()) {
                say = None;
                ui.data_mut(|d| d.remove::<(f64, usize)>(say_id));
            }
        }
        let face_rect = egui::Rect::from_min_max(at(smoker::FACE_BOX.0), at(smoker::FACE_BOX.1));
        let face = ui.interact(face_rect, ui.id().with("void_father_face"), egui::Sense::click());
        if say.is_none() {
            if face.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if face.clicked() {
                let last = ui.data(|d| d.get_temp::<usize>(last_id));
                let line = father::void_next(last);
                say = Some((now, line));
                ui.data_mut(|d| {
                    d.insert_temp(say_id, (now, line));
                    d.insert_temp(last_id, line);
                });
            }
        }
    }

    // Gold Edition: he sits smoking. A click on his face makes him talk. His cigarette can be
    // grabbed and thrown, but nothing shows that it can: the pointer never changes over it.
    let mut staff_taken = false;
    let mut staff_show = smoker::StaffShow::InHand;
    let mut smoker_busy = false;
    if let Some(sm) = smoker.as_deref_mut() {
        sm.tick(now);
        let to_picture = |p: egui::Pos2| ((p.x - ox) / scale, (p.y - oy) / scale);
        let pointer = ui.input(|i| i.pointer.latest_pos());
        let over_cigarette = pointer.is_some_and(|p| smoker::on_cigarette(to_picture(p)));
        let face_rect = egui::Rect::from_min_max(at(smoker::FACE_BOX.0), at(smoker::FACE_BOX.1));
        let face = ui.interact(face_rect, ui.id().with("gold_face"), egui::Sense::click());
        if face.hovered() && sm.can_click() && !over_cigarette {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if face.clicked() && sm.can_click() {
            sm.click(now);
        }
        if sm.can_grab() || sm.is_held() {
            let grab_rect = egui::Rect::from_min_max(at(smoker::GRAB_BOX.0), at(smoker::GRAB_BOX.1));
            let grab = ui.interact(grab_rect, ui.id().with("gold_cigarette"), egui::Sense::click_and_drag());
            if grab.drag_started() {
                if let Some(p) = pointer {
                    sm.grab(now, to_picture(p));
                }
            }
            if grab.dragged() {
                if let Some(p) = pointer {
                    sm.drag(now, to_picture(p));
                }
            }
            if grab.drag_stopped() {
                sm.release(now, pointer.map_or(smoker::rest_centre(), to_picture));
            }
            if grab.clicked() {
                sm.click(now);
            }
        }
        // the staff, once he holds it out: grab its head
        if staff_textures.len() == 3 {
            if let smoker::StaffShow::Out { pose, .. } = sm.view(now).staff {
                if sm.staff_offered(now) {
                    let (c, r) = (smoker::head_centre(&pose), smoker::head_radius(&pose));
                    let rect = egui::Rect::from_center_size(at(c), vec2(2.0 * r * scale, 2.0 * r * scale));
                    let response = ui.interact(rect, ui.id().with("gold_staff"), egui::Sense::click_and_drag());
                    let over = pointer.map(to_picture).filter(|&p| sm.staff_can_be_taken_at(now, p));
                    if over.is_some() && response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if let (Some(p), true) = (over, response.drag_started() || response.clicked()) {
                        staff_taken = sm.take_staff(now, p) == Some(smoker::Event::StaffTaken);
                    }
                }
            }
        }
        let view = sm.view(now);
        smoker_busy = view.speech.is_some()
            || view.mood != smoker::Mood::Droop
            || sm.is_held()
            || !matches!(view.staff, smoker::StaffShow::InHand);
        if staff_textures.len() == 3 {
            staff_show = view.staff;
        }
    }

    let painter = ui.painter();
    let whole = egui::Rect::from_min_size(at((0.0, 0.0)), vec2(father::PICTURE.0 * scale, father::PICTURE.1 * scale));
    let all = egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    // the clock face, the hands, then the dragon's silhouette in the background colour so the hands
    // pass behind him, then the dragon
    if let Some(vt) = void_father {
        paint_void_layers(painter, vt, textures, staff_textures, &at, scale, colour, background, whole, if motion { t } else { STILL }, motion);
    } else {
        painter.image(textures[0].id(), whole, all, colour);
        paint_hands(painter, &at, scale, colour);
        // with his staff out of his hand the layers without it are used, and the staff is drawn by hand
        let (mask, front) = if staff_show == smoker::StaffShow::InHand {
            (&textures[1], &textures[2])
        } else {
            (&staff_textures[1], &staff_textures[0])
        };
        painter.image(mask.id(), whole, all, background);
        painter.image(front.id(), whole, all, colour);
        if let smoker::StaffShow::Out { pose, glow } = &staff_show {
            paint_staff(painter, &at, scale, colour, &staff_textures[2], pose, *glow);
        }
    }

    // everything that moves
    let font = egui::FontId::monospace(21.0 * scale);
    for d in father::rain(t) {
        painter.text(at(d.pos), egui::Align2::CENTER_CENTER, d.ch, font.clone(), colour.gamma_multiply(d.strength * 0.85));
    }
    let presence = reaction.as_ref().map_or(1.0, |r| 1.0 - r.beard_gone);
    for dots in [father::orbs(t), father::sand(t), father::motes(t), father::grains(t, presence)] {
        for d in dots {
            if d.strength > 0.01 {
                painter.circle_filled(at(d.pos), (d.radius * scale).max(0.7), colour.gamma_multiply(d.strength));
            }
        }
    }
    let blink = if motion && smoker.is_none() && !is_void { father::blink(t) } else { 0.0 };
    if blink > 0.02 {
        let lid = Color32::from_rgba_unmultiplied(background.r(), background.g(), background.b(), (blink * 255.0) as u8);
        for mirror in [false, true] {
            let eye: Vec<egui::Pos2> =
                father::EYE.iter().map(|&(x, y)| at(if mirror { (1200.0 - x, y) } else { (x, y) })).collect();
            painter.add(Shape::convex_polygon(eye.clone(), lid, Stroke::NONE));
            if blink > 0.5 {
                painter.line_segment([eye[0], eye[3]], Stroke::new(1.5, colour));
            }
        }
    }
    if let Some(sm) = smoker.as_deref() {
        paint_smoker(painter, &at, scale, colour, background, &sm.view(now), if motion { t } else { STILL });
    }
    if let Some(r) = &reaction {
        // the beard turns to pixels and is gone; the jaw drops
        if r.beard_gone > 0.0 {
            let cover = Color32::from_rgba_unmultiplied(
                background.r(),
                background.g(),
                background.b(),
                (r.beard_gone * 255.0) as u8,
            );
            let points: Vec<egui::Pos2> = father::BEARD.iter().map(|&p| at(p)).collect();
            painter.add(Shape::convex_polygon(points, cover, Stroke::NONE));
        }
        for poly in father::face(r.jaw) {
            let points: Vec<egui::Pos2> = poly.points.iter().map(|&p| at(p)).collect();
            let fill = match poly.fill {
                father::Fill::None => None,
                father::Fill::Background => Some(background),
                father::Fill::Dim => Some(colour.gamma_multiply(0.16)),
                father::Fill::Bright => Some(colour.gamma_multiply(0.55)),
            };
            if let Some(fill) = fill {
                if poly.closed {
                    painter.add(Shape::convex_polygon(points.clone(), fill, Stroke::NONE));
                }
            }
            if poly.line > 0.0 {
                let stroke = Stroke::new(1.4, colour.gamma_multiply(poly.line));
                painter.add(if poly.closed { Shape::closed_line(points, stroke) } else { Shape::line(points, stroke) });
            }
        }
        for bit in &r.bits {
            let half = (bit.size * scale / 2.0).max(0.6);
            let centre = at(bit.pos);
            painter.rect_filled(
                egui::Rect::from_center_size(centre, vec2(half * 2.0, half * 2.0)),
                0.0,
                colour.gamma_multiply(bit.strength),
            );
        }
        if r.speech > 0.01 {
            painter.text(
                at(father::SPEECH_AT),
                egui::Align2::CENTER_CENTER,
                "How could you?",
                egui::FontId::monospace(34.0 * scale),
                colour.gamma_multiply(r.speech),
            );
        }
    }
    if let Some((start, line)) = say {
        void_speech_bubble(ui, &at, scale, colour, now, start, line);
    }
    if animate {
        // moving or reacting: smooth; standing still: just often enough for the second hand
        let wait = if motion || reaction.is_some() || smoker_busy || say.is_some() { 50 } else { 1000 };
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(wait));
    }
    FatherEvents { game1: start_game, staff_taken }
}

/// The pictures of Father Time in blocks, for the Void's glitching of him: for each block size, his
/// back, mask and front layers (the last two without the staff).
pub struct VoidTextures {
    pixel: Vec<(u32, Vec<egui::TextureHandle>)>,
}

/// Makes the pixelated pictures for The Void's Father Time from the layers' PNG files.
pub fn load_void_father(ctx: &egui::Context, layers: [&[u8]; 3]) -> VoidTextures {
    let size = (father::PICTURE.0 as u32, father::PICTURE.1 as u32);
    // (all three at the same size, so their blocks line up)
    let images: Vec<Option<image::RgbaImage>> = layers
        .iter()
        .map(|png| {
            image::load_from_memory(png)
                .ok()
                .map(|img| img.resize_exact(size.0, size.1, image::imageops::FilterType::Triangle).to_rgba8())
        })
        .collect();
    let pixel = crate::gold::BLOCKS
        .iter()
        .map(|&block| {
            let textures = images
                .iter()
                .enumerate()
                .filter_map(|(i, img)| {
                    let img = img.as_ref()?;
                    let (w, h, bytes) = father::pixelate_alpha(img.width() as usize, img.height() as usize, img.as_raw(), block as usize);
                    Some(ctx.load_texture(
                        format!("cpunity_void_father_{block}_{i}"),
                        egui::ColorImage::from_rgba_unmultiplied([w, h], &bytes),
                        egui::TextureOptions::NEAREST,
                    ))
                })
                .collect();
            (block, textures)
        })
        .collect();
    VoidTextures { pixel }
}

/// His picture in The Void: no staff, the clock spinning on its own, pixelated pictures and torn
/// strips, and a cyan and a magenta ghost of him jumping about. `t` is the time the glitch is at.
#[allow(clippy::too_many_arguments)]
fn paint_void_layers(
    painter: &egui::Painter,
    vt: &VoidTextures,
    textures: &[egui::TextureHandle],
    staff_textures: &[egui::TextureHandle],
    at: &dyn Fn((f32, f32)) -> egui::Pos2,
    scale: f32,
    colour: Color32,
    background: Color32,
    whole: egui::Rect,
    t: f32,
    glitching: bool,
) {
    let all = egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    let frame = if glitching {
        father::void_glitch(t)
    } else {
        crate::gold::GlitchFrame { block: None, bands: Vec::new(), flash: 0.0 }
    };
    // the layers: his back (the clock face), the mask, his front; the whole thing, or in blocks
    let (face, mask, front) = (&textures[0], &staff_textures[1], &staff_textures[0]);
    let blocks = frame.block.and_then(|b| vt.pixel.iter().find(|(size, _)| *size == b)).map(|(_, tex)| tex).filter(|tex| tex.len() == 3);
    let (face, mask, front) = match blocks {
        Some(tex) => (&tex[0], &tex[1], &tex[2]),
        None => (face, mask, front),
    };
    let (hour, minute, second) = father::void_hands(t);
    let (cyan_dx, magenta_dx) = if glitching { father::void_ghosts(t) } else { (0.0, 0.0) };
    painter.image(face.id(), whole, all, colour);
    paint_hands_at(painter, at, scale, colour, hour, minute, Some((second, 1.0)));
    painter.image(mask.id(), whole, all, background);
    if glitching {
        painter.image(front.id(), whole.translate(vec2(cyan_dx * scale, 0.0)), all, theme::VOID_CYAN.gamma_multiply(0.32));
        painter.image(front.id(), whole.translate(vec2(magenta_dx * scale, 0.0)), all, theme::VOID_MAGENTA.gamma_multiply(0.32));
    }
    painter.image(front.id(), whole, all, colour);
    // strips of him torn sideways (the stage, not the whole picture, is what the bands are fractions of)
    let rows = father::STAGE_BOTTOM - father::STAGE_TOP;
    for band in &frame.bands {
        let (y0, y1) = (father::STAGE_TOP + band.top * rows, father::STAGE_TOP + band.bottom * rows);
        let cover = egui::Rect::from_min_max(pos2(whole.left(), at((0.0, y0)).y), pos2(whole.right(), at((0.0, y1)).y));
        let uv = egui::Rect::from_min_max(pos2(0.0, y0 / father::PICTURE.1), pos2(1.0, y1 / father::PICTURE.1));
        let dst = cover.translate(vec2(band.shift * father::PICTURE.0 * 1.5 * scale, 0.0));
        painter.rect_filled(cover, 0.0, background);
        painter.image(face.id(), dst, uv, colour);
        painter.image(mask.id(), dst, uv, background);
        painter.image(front.id(), dst, uv, colour);
        // the torn edge of it in cyan
        painter.line_segment([dst.left_top(), dst.right_top()], Stroke::new(1.0, theme::VOID_CYAN.gamma_multiply(0.5)));
    }
    if frame.flash > 0.0 {
        let stage = egui::Rect::from_min_max(at((0.0, father::STAGE_TOP)), at((father::PICTURE.0, father::STAGE_BOTTOM)));
        painter.rect_filled(stage, 0.0, Color32::from_rgba_unmultiplied(120, 70, 230, (frame.flash * 255.0) as u8));
    }
}

/// What he says, in the Void's signs, in a bubble under his chin whose tail points at his mouth. The
/// signs come in letter by letter; with the pointer on a word it turns into letters, that word only.
fn void_speech_bubble(
    ui: &mut egui::Ui,
    at: &dyn Fn((f32, f32)) -> egui::Pos2,
    scale: f32,
    colour: Color32,
    now: f64,
    start: f64,
    line: usize,
) {
    let text = father::VOID_LINES[line];
    let Some((shown, alpha)) = father::void_speech((now - start) as f32, text.chars().count()) else { return };
    let size = (34.0 * scale).clamp(11.0, 22.0);
    let lines = crate::void_script::wrap(text, 30);
    let (adv, height) = (size * 0.62, size * 1.28);
    let widest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as f32;
    let (pad_x, pad_y) = (14.0, 10.0);
    let inner = vec2(adv * widest, height * lines.len() as f32);
    let bottom = at((600.0, 1100.0));
    let rect = egui::Rect::from_min_size(pos2(bottom.x - inner.x / 2.0 - pad_x, bottom.y - inner.y - 2.0 * pad_y), inner + vec2(2.0 * pad_x, 2.0 * pad_y));

    // each word decodes on its own when the pointer is on it, like the words of the Info text
    let ctx = ui.ctx().clone();
    let id = ui.id().with("void_father_decode").with(line);

    let time = (now % 3600.0) as f32;
    let painter = ui.painter();
    let violet = Color32::from_rgb(155, 107, 255);
    let fill = Color32::from_rgb(10, 4, 22).gamma_multiply(0.94 * alpha);
    let tip = at(smoker::SPEECH_TAIL);
    let tx = tip.x.clamp(rect.left() + 24.0, rect.right() - 24.0);
    let (left, right) = (pos2(tx - 14.0, rect.top() + 1.0), pos2(tx + 4.0, rect.top() + 1.0));
    painter.add(Shape::convex_polygon(vec![left, tip, right], fill, Stroke::NONE));
    painter.rect(rect, egui::CornerRadius::same(10), fill, Stroke::NONE, egui::StrokeKind::Inside);
    for (dx, tone, a) in [(-1.0, theme::VOID_CYAN, 0.55), (1.0, theme::VOID_MAGENTA, 0.55), (0.0, violet, 1.0)] {
        let stroke = Stroke::new(1.4, tone.gamma_multiply(a * alpha));
        painter.rect_stroke(rect.translate(vec2(dx, 0.0)), egui::CornerRadius::same(10), stroke, egui::StrokeKind::Inside);
        painter.add(Shape::line(vec![left + vec2(dx, 0.0), tip + vec2(dx, 0.0), right + vec2(dx, 0.0)], stroke));
    }
    painter.line_segment([pos2(left.x + 1.5, rect.top()), pos2(right.x - 1.5, rect.top())], Stroke::new(2.6, fill));
    super::script_text::paint_lines_by_word(&ctx, painter, id, rect.min + vec2(pad_x, pad_y), &lines, size, shown, time, alpha);
    let _ = colour;
}

/// What Father Time tells the page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FatherEvents {
    /// Five quick clicks on the clock's hub: the first game.
    pub game1: bool,
    /// He held out his staff and it was taken: the next game.
    pub staff_taken: bool,
}

/// Gold Father Time: new eyes over the ones in the picture, the cigarette, its smoke, and what he
/// says. Everything is worked out in `smoker.rs`.
fn paint_smoker(
    painter: &egui::Painter,
    at: &dyn Fn((f32, f32)) -> egui::Pos2,
    scale: f32,
    colour: Color32,
    background: Color32,
    view: &smoker::View,
    t: f32,
) {
    let points = |pts: &[(f32, f32)]| -> Vec<egui::Pos2> { pts.iter().map(|&p| at(p)).collect() };
    // the eyes in the picture are covered, then his own are drawn
    for cover in smoker::eye_covers() {
        painter.add(Shape::convex_polygon(points(&cover), background, Stroke::NONE));
    }
    let bright = theme::mix(colour, Color32::WHITE, 0.45);
    let face = smoker::face(view.mood);
    for eye in &face.eyes {
        painter.add(Shape::convex_polygon(
            points(&eye.points),
            bright.gamma_multiply(eye.fill * 0.9),
            Stroke::new(1.4, colour.gamma_multiply(eye.edge)),
        ));
    }
    for &(pos, radius) in &face.pupils {
        painter.circle_filled(at(pos), (radius * scale).max(1.5), background);
    }
    for line in &face.strokes {
        painter.add(Shape::line(points(&line.points), Stroke::new((line.width * scale).max(1.2), colour.gamma_multiply(line.alpha))));
    }

    // the smoke, behind the cigarette
    if let Some(from) = view.smoke_from {
        let grey = Color32::from_rgb(235, 215, 170);
        for puff in smoker::smoke(from, t, view.smoke) {
            if puff.alpha > 0.004 {
                painter.circle_filled(at(puff.pos), (puff.radius * scale).max(0.8), grey.gamma_multiply(puff.alpha));
            }
        }
    }
    // the cigarette: a filter, the paper, and the ember
    if let Some((base, tip)) = view.cigarette {
        let (b, e) = (at(base), at(tip));
        let along = e - b;
        if along.length() > 1.0 {
            let dir = along.normalized();
            let side = vec2(-dir.y, dir.x) * (smoker::HALF_WIDTH * scale).max(1.2);
            let edge = Stroke::new(1.0, Color32::from_rgb(150, 104, 26));
            painter.add(Shape::convex_polygon(vec![b + side, e + side, e - side, b - side], Color32::from_rgb(255, 244, 214), edge));
            let filter_end = b + dir * (26.0 * scale);
            painter.add(Shape::convex_polygon(vec![b + side, filter_end + side, filter_end - side, b - side], Color32::from_rgb(214, 150, 60), edge));
        }
        let ember = view.ember.clamp(0.0, 1.0);
        painter.circle_filled(e, (15.0 * scale).max(3.0), Color32::from_rgb(255, 190, 90).gamma_multiply(0.22 * ember));
        painter.circle_filled(e, (6.5 * scale).max(2.0), Color32::from_rgb(255, 168, 60).gamma_multiply(ember));
        painter.circle_filled(e, (3.2 * scale).max(1.0), Color32::from_rgb(255, 236, 170).gamma_multiply(ember));
    }
    for puff in &view.trail {
        painter.circle_filled(at(puff.pos), (puff.radius * scale).max(0.8), Color32::from_rgb(255, 190, 90).gamma_multiply(puff.alpha));
    }

    // what he says: words in a bubble whose tail points at his mouth
    if let Some((text, alpha)) = view.speech {
        if alpha > 0.01 {
            let ink = Color32::from_rgb(255, 226, 140);
            let galley = painter.layout_no_wrap(text.to_string(), egui::FontId::monospace(38.0 * scale), ink.gamma_multiply(alpha));
            let size = galley.size();
            let rect = egui::Rect::from_center_size(at(view.speech_at), size + vec2(56.0 * scale, 44.0 * scale));
            let fill = Color32::from_rgb(14, 11, 6).gamma_multiply(alpha * 0.96);
            let line = Stroke::new(1.8, colour.gamma_multiply(alpha));
            let tip = at(smoker::SPEECH_TAIL);
            let tx = tip.x.clamp(rect.left() + 40.0 * scale, rect.right() - 40.0 * scale);
            let (left, right) = (pos2(tx - 34.0 * scale, rect.top() + 1.0), pos2(tx + 6.0 * scale, rect.top() + 1.0));
            painter.add(Shape::convex_polygon(vec![left, tip, right], fill, Stroke::NONE));
            painter.add(Shape::line(vec![left, tip, right], line));
            painter.rect(rect, egui::CornerRadius::same((24.0 * scale) as u8), fill, line, egui::StrokeKind::Inside);
            // the bubble's outline is drawn over the tail's top, so cut the join
            painter.line_segment([pos2(left.x + 1.0, rect.top()), pos2(right.x - 1.0, rect.top())], Stroke::new(2.6, fill));
            painter.galley(rect.center() - size / 2.0, galley, ink);
        }
    }
}

/// His staff, held out: a soft glow round its hourglass, then the picture of it drawn as a strip of
/// quads that follow the pose (see `smoker::staff_rows`), so it can swing out and grow towards you.
fn paint_staff(
    painter: &egui::Painter,
    at: &dyn Fn((f32, f32)) -> egui::Pos2,
    scale: f32,
    colour: Color32,
    texture: &egui::TextureHandle,
    pose: &smoker::StaffPose,
    glow: f32,
) {
    let glow = glow.clamp(0.0, 1.0);
    let (head, radius) = (at(smoker::head_centre(pose)), smoker::head_radius(pose) * scale);
    let gold = Color32::from_rgb(255, 214, 120);
    for (size, alpha) in [(1.15, 0.04), (0.85, 0.07), (0.55, 0.11)] {
        painter.circle_filled(head, radius * size, gold.gamma_multiply(alpha * glow));
    }
    let tint = theme::mix(colour, Color32::WHITE, 0.35 * glow);
    let rows = smoker::staff_rows(pose);
    let mut mesh = egui::Mesh::with_texture(texture.id());
    for row in &rows {
        mesh.vertices.push(egui::epaint::Vertex { pos: at(row.left), uv: pos2(0.0, row.v), color: tint });
        mesh.vertices.push(egui::epaint::Vertex { pos: at(row.right), uv: pos2(1.0, row.v), color: tint });
    }
    for i in 0..rows.len() - 1 {
        let a = 2 * i as u32;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    painter.add(Shape::mesh(mesh));
}

/// The time the frozen picture shows when the animation is switched off.
const STILL: f32 = 14.0;

/// The hour, minute and second hand at the computer's local time.
fn paint_hands(
    painter: &egui::Painter,
    at: &dyn Fn((f32, f32)) -> egui::Pos2,
    scale: f32,
    colour: Color32,
) {
    let now = crate::sensor_log::now();
    let (hour, minute, second) = info::hand_angles(now.hour, now.minute, now.second);
    paint_hands_at(painter, at, scale, colour, hour, minute, Some((second, 1.0)));
}

/// The hands at the given angles (degrees clockwise from 12). `second` is the angle and how
/// strongly it is drawn, or None to leave the second hand out.
pub fn paint_hands_at(
    painter: &egui::Painter,
    at: &dyn Fn((f32, f32)) -> egui::Pos2,
    scale: f32,
    colour: Color32,
    hour: f32,
    minute: f32,
    second: Option<(f32, f32)>,
) {
    let to_screen = |points: &[(f32, f32)], degrees: f32| -> Vec<egui::Pos2> {
        info::turn_about_clock(points, degrees).into_iter().map(at).collect()
    };
    let fill = colour.gamma_multiply(0.3);
    let outline = Stroke::new(1.5, colour);
    for (outline_pts, shaft, head, degrees) in [
        (&info::HOUR_OUTLINE[..], &info::HOUR_SHAFT[..], &info::HOUR_HEAD[..], hour),
        (&info::MINUTE_OUTLINE[..], &info::MINUTE_SHAFT[..], &info::MINUTE_HEAD[..], minute),
    ] {
        // the shape is not convex, so it is filled in two convex pieces and outlined whole
        painter.add(Shape::convex_polygon(to_screen(shaft, degrees), fill, Stroke::NONE));
        painter.add(Shape::convex_polygon(to_screen(head, degrees), fill, Stroke::NONE));
        painter.add(Shape::closed_line(to_screen(outline_pts, degrees), outline));
    }
    if let Some((degrees, strength)) = second {
        let second_pts = to_screen(&info::SECOND_HAND, degrees);
        painter.line_segment([second_pts[0], second_pts[1]], Stroke::new(1.4, colour.gamma_multiply(strength)));
    }
    painter.circle_stroke(at(info::CLOCK_CENTER), info::HUB_RADIUS * scale, Stroke::new(1.5, colour));
}

/// Fire dragons: the dim picture behind everything, and the fire that runs round the circuits
/// that join the dragons, lighting them up. Drawn in the same place as the other pictures.
/// `animate` is false while the window is hidden so nothing keeps redrawing; `motion` is the
/// Options switch: off, only the dim picture is drawn.
pub fn paint_dragons(
    ui: &egui::Ui,
    textures: &[egui::TextureHandle],
    strength: f32,
    animate: bool,
    motion: bool,
) {
    paint(ui, textures, strength);
    if !motion {
        return;
    }
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 || textures.len() != 1 {
        return;
    }
    let (offset, scale) = dragons::cover((rect.width(), rect.height()));
    let at = |p: (f32, f32)| pos2(rect.min.x + offset.0 + p.0 * scale, rect.min.y + offset.1 + p.1 * scale);
    let t = (ui.input(|i| i.time) % 3600.0) as f32;
    let colour = tint(strength);
    let painter = ui.painter();
    for ring in dragons::rings() {
        for n in 0..ring.heads {
            let head = ring.head(n, t);
            // the lit trail behind the head
            for (points, glow) in ring.trail(head.arc) {
                let points: Vec<egui::Pos2> = points.into_iter().map(at).collect();
                painter.add(Shape::line(points, Stroke::new(2.6, colour.gamma_multiply(glow))));
            }
            let p = at(ring.point(head.arc));
            painter.circle_filled(p, 9.0 * scale.max(0.4), colour.gamma_multiply(0.22));
            painter.circle_filled(p, 3.6, colour);
        }
    }
    // the puffs: flame tongues, filled from the middle and outlined
    for puff in dragons::puffs(t) {
        for (shape, glow) in dragons::tongues(&puff) {
            let points: Vec<egui::Pos2> = shape.into_iter().map(at).collect();
            let n = points.len();
            let middle = points.iter().fold(egui::Vec2::ZERO, |sum, p| sum + p.to_vec2()) / n as f32;
            let fill = colour.gamma_multiply(0.32 * glow);
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(middle.to_pos2(), fill);
            for p in &points {
                mesh.colored_vertex(*p, fill);
            }
            for i in 0..n {
                mesh.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
            }
            painter.add(Shape::mesh(mesh));
            painter.add(Shape::closed_line(points, Stroke::new(1.4, colour.gamma_multiply(glow))));
        }
    }
    if animate {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }
}

/// Angel cats: a scatter of small winged cats drifting down from above, each with a faint streak of
/// light behind it. Unlike the other pictures they are in their own colours (gold on dark), not the
/// accent colour; `strength` only dims them. In The Void (`void`) the picture they are in is a dark
/// bronze one with dead eyes (see `BackdropStyle::pngs_in`) and their light is dim. With `motion` off they stand still. `animate` is false
/// while the window is hidden so nothing keeps redrawing.
pub fn paint_angels(
    ui: &egui::Ui,
    textures: &[egui::TextureHandle],
    strength: f32,
    animate: bool,
    motion: bool,
    void: bool,
    silver: bool,
) {
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 || textures.len() != 1 {
        return;
    }
    let t = if motion { (ui.input(|i| i.time) % 86_400.0) as f32 } else { 0.0 };
    let strength = strength.clamp(0.0, 1.0);
    let painter = ui.painter();
    for angel in angels::placements((rect.width(), rect.height()), t) {
        let a = strength * angel.alpha;
        if a <= 0.004 {
            continue;
        }
        let centre = pos2(rect.min.x + angel.centre.0, rect.min.y + angel.centre.1);
        let size = vec2(angel.size.0, angel.size.1);
        let image = egui::Rect::from_center_size(centre, size);
        if !image.intersects(rect) {
            continue;
        }
        // the streak of light: three thin beams above the cat that fade out upwards
        // (in The Void the cats have lost their gold, and so has the light behind them)
        // (the Soul Catsters of the Platinum look leave a silver one)
        let light = if void {
            Color32::from_rgb(150, 98, 48)
        } else if silver {
            Color32::from_rgb(214, 228, 246)
        } else {
            Color32::from_rgb(240, 192, 84)
        };
        let length = size.y * 1.6 * angel.trail;
        for (dx, width, weight) in [(-0.16, 2.0, 0.5), (0.0, 3.0, 0.8), (0.16, 2.0, 0.5)] {
            let x = centre.x + dx * size.x;
            let bottom = centre.y - size.y * 0.3;
            let mut mesh = egui::Mesh::default();
            let base = light.gamma_multiply(a * weight * 0.55);
            mesh.colored_vertex(pos2(x - width * 0.5, bottom), base);
            mesh.colored_vertex(pos2(x + width * 0.5, bottom), base);
            mesh.colored_vertex(pos2(x + width * 0.5, bottom - length), Color32::TRANSPARENT);
            mesh.colored_vertex(pos2(x - width * 0.5, bottom - length), Color32::TRANSPARENT);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            painter.add(Shape::mesh(mesh));
        }
        if !void {
            let [u0, v0, u1, v1] = angels::uv(angel.pose);
            painter.image(
                textures[0].id(),
                image,
                egui::Rect::from_min_max(pos2(u0, v0), pos2(u1, v1)),
                Color32::WHITE.gamma_multiply(a),
            );
            continue;
        }
        // in The Void: now and then the picture tears into strips that slip sideways, and the white
        // crosses in the eyes glitch out and leave them black for a moment
        let torn = angels::glitch(angel.id, t);
        let a = a * torn.as_ref().map_or(1.0, |g| g.flicker);
        let tears: &[angels::Tear] = torn.as_ref().map_or(&[][..], |g| g.tears.as_slice());
        paint_torn(painter, textures[0].id(), image, angels::uv_void(angel.pose, false), tears, a);
        let cross = angels::crosses(angel.id, t);
        if cross > 0.0 {
            paint_torn(painter, textures[0].id(), image, angels::uv_void(angel.pose, true), tears, a * cross);
        }
    }
    if animate && motion {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
    }
}

/// Draws the part `uv` of a picture on `image` with the strips in `tears` slipped sideways (see
/// `angels::glitch`): the rest of it where it belongs, and a thin cyan line above and a magenta one
/// below every strip that has moved.
fn paint_torn(painter: &egui::Painter, texture: egui::TextureId, image: egui::Rect, uv: [f32; 4], tears: &[angels::Tear], alpha: f32) {
    let [u0, v0, u1, v1] = uv;
    let tint = Color32::WHITE.gamma_multiply(alpha);
    let piece = |from: f32, to: f32, shift: f32| {
        let (top, bottom) = (image.top() + image.height() * from, image.top() + image.height() * to);
        let dx = image.width() * shift;
        painter.image(
            texture,
            egui::Rect::from_min_max(pos2(image.left() + dx, top), pos2(image.right() + dx, bottom)),
            egui::Rect::from_min_max(pos2(u0, v0 + (v1 - v0) * from), pos2(u1, v0 + (v1 - v0) * to)),
            tint,
        );
    };
    let mut cursor = 0.0;
    for tear in tears {
        if tear.top > cursor {
            piece(cursor, tear.top, 0.0);
        }
        piece(tear.top, tear.bottom, tear.shift);
        let dx = image.width() * tear.shift;
        for (y, tone) in [(image.top() + image.height() * tear.top, theme::VOID_CYAN), (image.top() + image.height() * tear.bottom - 1.5, theme::VOID_MAGENTA)] {
            painter.rect_filled(
                egui::Rect::from_min_size(pos2(image.left() + dx, y), vec2(image.width(), 1.5)),
                0.0,
                tone.gamma_multiply(0.55 * alpha),
            );
        }
        cursor = tear.bottom;
    }
    if cursor < 1.0 {
        piece(cursor, 1.0, 0.0);
    }
}

/// The Gold Edition's sun rays: a fan of soft gold beams from above the top of the window,
/// fading out as they spread. Painted before everything else.
pub fn paint_gold_rays(ui: &egui::Ui, t: f32) {
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let apex = pos2(rect.center().x, rect.top() - rect.height() * 0.18);
    let length = rect.height() * 1.35 + rect.width() * 0.25;
    let gold = Color32::from_rgb(234, 185, 74);
    let mut mesh = egui::Mesh::default();
    for n in 0..crate::gold::RAYS {
        let angle = crate::gold::ray_angle(n);
        let half = 0.045;
        let bright = crate::gold::ray_shimmer(n, t);
        let near = gold.gamma_multiply(0.085 * bright);
        let far = gold.gamma_multiply(0.0);
        let at = |a: f32| apex + vec2(a.sin(), a.cos()) * length;
        let first = mesh.vertices.len() as u32;
        mesh.colored_vertex(apex, near);
        mesh.colored_vertex(at(angle - half), far);
        mesh.colored_vertex(at(angle + half), far);
        mesh.add_triangle(first, first + 1, first + 2);
    }
    ui.painter().with_clip_rect(rect).add(Shape::mesh(mesh));
}

/// The Platinum skin's art: a lattice of crystal facets that glimmer on their own and light up as a
/// band of light sweeps across the window, slow rays from above, a halo, and a rosette turning very
/// slowly behind it all (see `platinum.rs`). Painted before everything else. `t` is the time in
/// seconds (0 holds it still).
pub fn paint_platinum(ui: &egui::Ui, t: f32, strength: f32) {
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let k = strength.clamp(0.0, 1.0);
    let (w, h) = (rect.width(), rect.height());
    let at = |p: (f32, f32)| pos2(rect.left() + p.0 * w, rect.top() + p.1 * h);
    let ice = Color32::from_rgb(214, 228, 246);
    let sweep = platinum::sweep_position(t);

    // the halo above the window
    soft_ellipse(&painter, pos2(rect.center().x, rect.top()), vec2(w * 0.65, h * 0.36), [196, 216, 244], 0.13 * k);
    soft_ellipse(&painter, pos2(rect.center().x, rect.top() + h * 0.02), vec2(w * 0.26, h * 0.13), [236, 244, 255], (0.11 + 0.03 * (t * 0.5).sin()) * k);

    // the rosette: rings and two stars turning against each other
    let centre = pos2(rect.center().x, rect.top() + h * 0.36);
    let radius = (w * 0.46).min(h * 0.45);
    let turn = platinum::rosette_turn(t);
    for (f, a) in [(1.0, 0.055), (0.82, 0.045), (0.36, 0.05)] {
        painter.circle_stroke(centre, radius * f, Stroke::new(1.0, ice.gamma_multiply(a * k)));
    }
    for (points, inner, direction, a) in [(16usize, 0.62f32, 1.0f32, 0.07f32), (8, 0.42, -1.0, 0.06)] {
        let corners: Vec<egui::Pos2> = platinum::star(points, inner, turn * direction).into_iter().map(|(x, y)| centre + vec2(x, y) * radius).collect();
        painter.add(Shape::closed_line(corners, Stroke::new(1.0, ice.gamma_multiply(a * k))));
    }

    // the facets, then their edges
    let lattice = platinum::lattice();
    let mut mesh = egui::Mesh::default();
    for (i, tri) in lattice.tris.iter().enumerate() {
        let c = lattice.centre(i);
        let light = platinum::sweep_light(platinum::diagonal(c.0, c.1), sweep);
        let alpha = ((0.010 + 0.036 * lattice.tones[i] * platinum::glimmer(i, t) + 0.16 * light) * k).clamp(0.0, 1.0);
        let colour = Color32::from_rgba_unmultiplied(ice.r(), ice.g(), ice.b(), (alpha * 255.0) as u8);
        let first = mesh.vertices.len() as u32;
        for &p in tri {
            mesh.colored_vertex(at(lattice.points[p]), colour);
        }
        mesh.add_triangle(first, first + 1, first + 2);
    }
    painter.add(Shape::mesh(mesh));
    for (a, b) in lattice.edges() {
        let (pa, pb) = (lattice.points[a], lattice.points[b]);
        let mid = ((pa.0 + pb.0) / 2.0, (pa.1 + pb.1) / 2.0);
        let light = platinum::sweep_light(platinum::diagonal(mid.0, mid.1), sweep);
        painter.line_segment([at(pa), at(pb)], Stroke::new(1.0, ice.gamma_multiply(((0.04 + 0.34 * light) * k).clamp(0.0, 1.0))));
    }

    // the rays, from above the top of the window
    let apex = pos2(rect.center().x, rect.top() - h * 0.18);
    let length = h * 1.35 + w * 0.25;
    let mut rays = egui::Mesh::default();
    for n in 0..platinum::RAYS {
        let r = platinum::ray(n, t);
        let strength = if n % 2 == 1 { 0.11 } else { 0.07 };
        let near = Color32::from_rgb(222, 236, 252).gamma_multiply(strength * r.bright * k);
        let at = |a: f32| apex + vec2(a.sin(), a.cos()) * length;
        let first = rays.vertices.len() as u32;
        rays.colored_vertex(apex, near);
        rays.colored_vertex(at(r.angle - r.half), Color32::TRANSPARENT);
        rays.colored_vertex(at(r.angle + r.half), Color32::TRANSPARENT);
        rays.add_triangle(first, first + 1, first + 2);
    }
    painter.add(Shape::mesh(rays));

    // a little shade at the bottom, so the light seems to come from above
    let shade_top = rect.bottom() - h * 0.3;
    let mut shade = egui::Mesh::default();
    let dark = Color32::from_black_alpha((90.0 * k) as u8);
    shade.colored_vertex(pos2(rect.left(), shade_top), Color32::TRANSPARENT);
    shade.colored_vertex(pos2(rect.right(), shade_top), Color32::TRANSPARENT);
    shade.colored_vertex(pos2(rect.right(), rect.bottom()), dark);
    shade.colored_vertex(pos2(rect.left(), rect.bottom()), dark);
    shade.add_triangle(0, 1, 2);
    shade.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(shade));
}

/// The Platinum skin's glints: flares that drift up in white, ice, lilac and mint, drawn above the
/// cards so they can be seen, and that flare when the sweep of light passes them. The brightest ones
/// split into red and cyan for an instant, like light through a cut stone. They never take clicks.
pub fn paint_platinum_sparkles(ui: &egui::Ui, t: f32) {
    let rect = ui.clip_rect();
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("platinum_sparkles"));
    let painter = ui.ctx().layer_painter(layer).with_clip_rect(rect);
    for g in platinum::glints(t) {
        if g.glow < 0.03 || !(0.0..=1.0).contains(&g.y) {
            continue;
        }
        let c = pos2(rect.left() + g.x * rect.width(), rect.top() + g.y * rect.height());
        let [r, gr, b] = platinum::PALETTE[g.tint];
        let colour = Color32::from_rgb(r, gr, b);
        let arm = g.size * (0.6 + 0.8 * g.glow);
        soft_ellipse(&painter, c, vec2(arm * 1.4, arm * 1.4), [r, gr, b], 0.16 * g.glow);
        // the long cross, and a short one turned a quarter
        let long = Stroke::new(1.0, colour.gamma_multiply(0.55 * g.glow));
        painter.line_segment([c - vec2(arm * 2.6, 0.0), c + vec2(arm * 2.6, 0.0)], long);
        painter.line_segment([c - vec2(0.0, arm * 2.6), c + vec2(0.0, arm * 2.6)], long);
        let short = Stroke::new(0.8, colour.gamma_multiply(0.35 * g.glow));
        let d = arm * 0.95;
        painter.line_segment([c - vec2(d, d), c + vec2(d, d)], short);
        painter.line_segment([c - vec2(d, -d), c + vec2(d, -d)], short);
        painter.circle_filled(c, 1.5, Color32::WHITE.gamma_multiply(0.9 * g.glow));
        if g.glow > 0.85 {
            painter.circle_filled(c + vec2(-1.6, 0.0), 1.3, Color32::from_rgb(255, 120, 150).gamma_multiply(0.55));
            painter.circle_filled(c + vec2(1.6, 0.0), 1.3, Color32::from_rgb(120, 230, 255).gamma_multiply(0.55));
        }
    }
}

/// The Gold Edition's sparkles: small four-pointed glints drifting up, drawn above the cards so
/// they can be seen. They never take clicks.
pub fn paint_gold_sparkles(ui: &egui::Ui, t: f32) {
    let rect = ui.clip_rect();
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("gold_sparkles"));
    let painter = ui.ctx().layer_painter(layer).with_clip_rect(rect);
    let pale = Color32::from_rgb(255, 238, 170);
    for s in crate::gold::sparkles(t) {
        if s.glow < 0.03 || !(0.0..=1.0).contains(&s.y) {
            continue;
        }
        let c = pos2(rect.left() + s.x * rect.width(), rect.top() + s.y * rect.height());
        let colour = pale.gamma_multiply(0.45 * s.glow);
        let arm = s.size * (0.6 + 0.4 * s.glow);
        let stroke = Stroke::new(1.2, colour);
        painter.line_segment([c - vec2(arm, 0.0), c + vec2(arm, 0.0)], stroke);
        painter.line_segment([c - vec2(0.0, arm), c + vec2(0.0, arm)], stroke);
        painter.circle_filled(c, 1.3, colour);
    }
}

/// A soft round (or oval) glow: thickest at `alpha` in the middle, gone at the edge.
pub fn soft_ellipse(painter: &egui::Painter, centre: egui::Pos2, radii: egui::Vec2, rgb: [u8; 3], alpha: f32) {
    const SEGMENTS: usize = 28;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(centre, Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], (alpha.clamp(0.0, 1.0) * 255.0) as u8));
    for i in 0..SEGMENTS {
        let a = std::f32::consts::TAU * i as f32 / SEGMENTS as f32;
        mesh.colored_vertex(centre + vec2(a.cos() * radii.x, a.sin() * radii.y), Color32::TRANSPARENT);
    }
    for i in 0..SEGMENTS as u32 {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % SEGMENTS as u32);
    }
    painter.add(Shape::mesh(mesh));
}

/// The Void's backdrop: black and dark-purple clouds drifting through each other, with small stars
/// that flicker, and now and then one splits into cyan and magenta, jumps, or leaves a streak, while
/// bars of colour tear across (see `void.rs`). `t` is the time in seconds (0 holds it still). It is
/// drawn first, like the other backdrops.
pub fn paint_void_stars(ui: &egui::Ui, t: f32) {
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let at = |x: f32, y: f32| pos2(rect.left() + x * rect.width(), rect.top() + y * rect.height());
    // the purple clouds first, then the black ones over them, so the black cuts through the purple
    for dark in [false, true] {
        for i in 0..void::CLOUDS {
            let c = void::cloud(i, t);
            if c.dark != dark {
                continue;
            }
            let rgb = if dark { [0, 0, 0] } else if i % 3 == 0 { [74, 39, 184] } else { [62, 22, 140] };
            soft_ellipse(&painter, at(c.x, c.y), vec2(c.rx * rect.width(), c.ry * rect.height()), rgb, c.alpha);
        }
    }
    let cyan = theme::VOID_CYAN;
    let magenta = theme::VOID_MAGENTA;
    for i in 0..void::STARS {
        let s = void::star(i);
        let f = void::flicker(&s, i, t);
        let p = at(s.x, s.y) + vec2(f.jump.0, f.jump.1);
        let square = |centre: egui::Pos2| egui::Rect::from_center_size(centre, vec2(s.size, s.size));
        if f.streak > 0.0 {
            painter.line_segment([p - vec2(f.streak, 0.0), p], Stroke::new(1.0, cyan.gamma_multiply(0.45)));
            painter.line_segment([p - vec2(f.streak * 0.6, -1.5), p + vec2(0.0, 1.5)], Stroke::new(1.0, magenta.gamma_multiply(0.35)));
        }
        if s.dim {
            painter.rect_filled(square(p), 0.0, Color32::from_rgb(155, 107, 255).gamma_multiply(0.7 * f.alpha));
            continue;
        }
        if f.split > 0.0 {
            painter.rect_filled(square(p - vec2(f.split, 0.0)), 0.0, cyan.gamma_multiply(0.9 * f.alpha));
            painter.rect_filled(square(p + vec2(f.split, 0.0)), 0.0, magenta.gamma_multiply(0.9 * f.alpha));
        }
        painter.rect_filled(square(p), 0.0, Color32::from_rgb(244, 239, 255).gamma_multiply(f.alpha));
    }
    for k in 0..void::TEARS {
        if let Some(x) = void::tear(k, t) {
            let bar = egui::Rect::from_min_size(at(x.x, x.y), vec2(x.w * rect.width(), x.h));
            painter.rect_filled(bar, 0.0, (if x.cyan { cyan } else { magenta }).gamma_multiply(x.alpha));
        }
    }
}

/// The shop's Embers backdrop: sparks drifting up from the bottom of the window and cooling from
/// white-orange to dull red as they rise (see `ambient.rs`). `t` is the time in seconds (0 holds it
/// still); `strength` fades the whole thing.
pub fn paint_embers(ui: &egui::Ui, t: f32, strength: f32) {
    use crate::ambient;
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let k = strength.clamp(0.0, 1.0);
    for i in 0..ambient::EMBERS {
        let e = ambient::ember(i, t);
        let p = pos2(rect.left() + e.x * rect.width(), rect.top() + e.y * rect.height());
        let [r, g, b] = ambient::ember_colour(e.heat);
        if e.heat > 0.45 {
            soft_ellipse(&painter, p, vec2(e.size * 5.0, e.size * 5.0), [r, g, b], 0.22 * e.heat * k);
        }
        painter.circle_filled(p, e.size * 0.5, Color32::from_rgb(r, g, b).gamma_multiply((0.35 + 0.65 * e.heat) * k));
    }
}

/// The shop's Data rain backdrop: columns of characters falling in the accent colour, bright at the
/// head and fading up the trail. `t` is the time in seconds (0 holds it still).
pub fn paint_data_rain(ui: &egui::Ui, t: f32, strength: f32) {
    use crate::ambient;
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    const CELL: f32 = 16.0;
    let painter = ui.painter().with_clip_rect(rect);
    let accent = theme::accent();
    let head_colour = theme::mix(accent, Color32::WHITE, 0.65);
    let font = egui::FontId::monospace(13.0);
    let k = strength.clamp(0.0, 1.0);
    for i in 0..ambient::COLUMNS {
        let c = ambient::column(i, t);
        let x = rect.left() + c.x * rect.width();
        for row in 0..ambient::TRAIL {
            let y = rect.top() + c.head * rect.height() - row as f32 * CELL;
            if y < rect.top() - CELL || y > rect.bottom() + CELL {
                continue;
            }
            let base = if row == 0 { head_colour } else { accent };
            let colour = base.gamma_multiply(ambient::trail_alpha(row) * 0.55 * k);
            painter.text(pos2(x, y), egui::Align2::CENTER_CENTER, ambient::glyph(i, row, t).to_string(), font.clone(), colour);
        }
    }
}

/// The shop's Northern lights backdrop: slow curtains of green, teal and violet that sway and swell.
pub fn paint_northern(ui: &egui::Ui, t: f32, strength: f32) {
    use crate::ambient;
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let k = strength.clamp(0.0, 1.0);
    for i in 0..ambient::CURTAINS {
        let slices = ambient::curtain(i, t);
        let [r, g, b] = ambient::curtain_colour(i);
        let mut mesh = egui::Mesh::default();
        for s in &slices {
            let x = rect.left() + s.x * rect.width();
            let y = rect.top() + s.y * rect.height();
            let h = s.height * rect.height();
            let mid = Color32::from_rgba_unmultiplied(r, g, b, (s.alpha * k * 255.0).clamp(0.0, 255.0) as u8);
            mesh.colored_vertex(pos2(x, y - h * 0.6), Color32::TRANSPARENT);
            mesh.colored_vertex(pos2(x, y), mid);
            mesh.colored_vertex(pos2(x, y + h * 0.4), Color32::TRANSPARENT);
        }
        for s in 0..slices.len() as u32 - 1 {
            let (a, b2) = (3 * s, 3 * (s + 1));
            for j in 0..2 {
                mesh.add_triangle(a + j, a + j + 1, b2 + j);
                mesh.add_triangle(a + j + 1, b2 + j + 1, b2 + j);
            }
        }
        painter.add(Shape::mesh(mesh));
    }
}

/// The shop's Sakura petals backdrop: cherry-blossom petals falling and turning over (see
/// `ambient.rs`). `t` is the time in seconds (0 holds it still).
pub fn paint_petals(ui: &egui::Ui, t: f32, strength: f32) {
    use crate::ambient;
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let outline = ambient::petal_outline();
    let k = strength.clamp(0.0, 1.0);
    for i in 0..ambient::PETALS {
        let p = ambient::petal(i, t);
        let [r, g, b] = ambient::petal_colour(p.tone);
        let centre = pos2(rect.left() + p.x * rect.width(), rect.top() + p.y * rect.height());
        let (sin, cos) = p.angle.sin_cos();
        let alpha = (p.alpha * k * 255.0).clamp(0.0, 255.0) as u8;
        let mut mesh = egui::Mesh::default();
        // the middle is a little deeper than the edge
        mesh.colored_vertex(centre, Color32::from_rgba_unmultiplied(r, g.saturating_sub(18), b.saturating_sub(14), alpha));
        for (ox, oy) in outline {
            // flipped over: narrower across, then turned
            let (x, y) = (ox * p.flip * p.size, oy * p.size);
            mesh.colored_vertex(centre + vec2(x * cos - y * sin, x * sin + y * cos), Color32::from_rgba_unmultiplied(r, g, b, alpha));
        }
        for j in 0..ambient::PETAL_POINTS as u32 {
            mesh.add_triangle(0, 1 + j, 1 + (j + 1) % ambient::PETAL_POINTS as u32);
        }
        painter.add(Shape::mesh(mesh));
    }
}

/// The shop's Synthwave grid backdrop: a striped sun sitting on the horizon over a neon grid that
/// rolls towards you (see `ambient.rs`). `t` is the time in seconds (0 holds it still).
pub fn paint_synth_grid(ui: &egui::Ui, t: f32, strength: f32) {
    use crate::ambient;
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let k = strength.clamp(0.0, 1.0);
    let magenta = theme::SYNTH_MAGENTA;
    let cyan = theme::SYNTH_CYAN;
    let horizon = rect.top() + ambient::HORIZON * rect.height();
    let radius = ambient::SUN_RADIUS * rect.height();
    let centre = pos2(rect.center().x, horizon);
    // the sun: a half disc, yellow at the top and magenta at the horizon
    let sun = |up: f32, a: f32| -> Color32 {
        let (top, bottom) = ([255.0, 214.0, 72.0], [255.0, 46.0, 150.0]);
        let c = [0, 1, 2].map(|i| (bottom[i] + (top[i] - bottom[i]) * up) as u8);
        Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (a * k * 255.0) as u8)
    };
    let steps = 40;
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(centre, sun(0.0, 0.85));
    for i in 0..=steps {
        let a = std::f32::consts::PI * i as f32 / steps as f32;
        mesh.colored_vertex(centre + vec2(a.cos() * radius, -a.sin() * radius), sun(a.sin(), 0.85));
    }
    for i in 0..steps as u32 {
        mesh.add_triangle(0, 1 + i, 2 + i);
    }
    painter.add(Shape::mesh(mesh));
    // the stripes cut out of it
    for (up, thick) in ambient::sun_stripes() {
        let half = (1.0 - up * up).max(0.0).sqrt() * radius;
        let y = horizon - up * radius;
        painter.rect_filled(egui::Rect::from_center_size(pos2(centre.x, y), vec2(half * 2.0, thick * radius)), 0.0, theme::bg());
    }
    // the grid
    for y in ambient::grid_rows(t) {
        let depth = (y - ambient::HORIZON) / (1.0 - ambient::HORIZON);
        let py = rect.top() + y * rect.height();
        painter.line_segment([pos2(rect.left(), py), pos2(rect.right(), py)], Stroke::new(1.0, magenta.gamma_multiply((0.2 + 0.6 * depth) * k)));
    }
    for x in ambient::grid_columns() {
        painter.line_segment(
            [pos2(rect.center().x, horizon), pos2(rect.left() + x * rect.width(), rect.bottom())],
            Stroke::new(1.0, magenta.gamma_multiply(0.45 * k)),
        );
    }
    // the horizon glows
    painter.line_segment([pos2(rect.left(), horizon), pos2(rect.right(), horizon)], Stroke::new(4.0, cyan.gamma_multiply(0.18 * k)));
    painter.line_segment([pos2(rect.left(), horizon), pos2(rect.right(), horizon)], Stroke::new(1.5, cyan.gamma_multiply(0.8 * k)));
}

/// Draws a branch of blossoms (see `sakura.rs`) grown from `root`, `scale` pixels to the unit. `t`
/// and `phase` make it sway in the wind (`t` 0 holds it still); `k` fades it (0 to 1).
pub fn paint_blossom_art(painter: &egui::Painter, art: &crate::sakura::Art, root: egui::Pos2, scale: f32, t: f32, phase: f32, k: f32) {
    use crate::{ambient, sakura};
    let at = |p: (f32, f32)| {
        let (x, y) = sakura::swayed(p, t, phase);
        root + vec2(x, y) * scale
    };
    let wood = Color32::from_rgb(118, 72, 86).gamma_multiply(k);
    for s in &art.segs {
        painter.line_segment([at(s.a), at(s.b)], Stroke::new((s.width * scale * 0.035).max(1.2), wood));
    }
    for b in &art.blooms {
        let [r, g, bl] = ambient::petal_colour(b.tone);
        let colour = Color32::from_rgb(r, g, bl).gamma_multiply(k);
        let centre = at(b.at);
        let radius = b.size * scale * 0.6;
        if radius < 2.5 {
            painter.circle_filled(centre, radius * 1.7, colour);
            continue;
        }
        for i in 0..5 {
            let a = std::f32::consts::TAU * i as f32 / 5.0 + b.turn;
            painter.circle_filled(centre + vec2(a.cos(), a.sin()) * radius, radius, colour);
        }
        painter.circle_filled(centre, radius * 0.4, Color32::from_rgb(250, 226, 150).gamma_multiply(k));
    }
}

/// The Sakura skin's corner art: a big blossoming branch reaching in from the top right and a
/// smaller one from the top left, swaying a little. Drawn first, behind the cards.
pub fn paint_blossom_branches(ui: &egui::Ui, t: f32, strength: f32) {
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let k = (strength * 1.1).clamp(0.0, 1.0);
    let big = crate::sakura::branch(3, 2.5, 4);
    paint_blossom_art(&painter, &big, rect.right_top() + vec2(8.0, -8.0), 0.24 * rect.height(), t, 0.0, k);
    let small = crate::sakura::branch(11, 0.75, 4);
    paint_blossom_art(&painter, &small, rect.left_top() + vec2(-8.0, -8.0), 0.15 * rect.height(), t, 2.0, k);
}

/// A pine (see `pine.rs`) standing with its foot at `base`, `height` pixels tall: a trunk, then each
/// tier of boughs from the bottom up with its snow on top. `k` fades it (0 to 1).
pub fn paint_pine(painter: &egui::Painter, tree: &crate::pine::Tree, base: egui::Pos2, height: f32, dark: Color32, snow: Color32, k: f32) {
    let at = |p: (f32, f32)| base + vec2(p.0, p.1) * height;
    let (dark, snow) = (dark.gamma_multiply(k), snow.gamma_multiply(k));
    let (l, t, r, b) = tree.trunk;
    painter.rect_filled(egui::Rect::from_min_max(at((l, t)), at((r, b))), 0.0, dark);
    for tier in &tree.tiers {
        painter.add(Shape::convex_polygon(tier.body.iter().map(|p| at(*p)).collect(), dark, Stroke::NONE));
        for bough in &tier.boughs {
            painter.add(Shape::convex_polygon(bough.iter().map(|p| at(*p)).collect(), dark, Stroke::NONE));
        }
        painter.add(Shape::convex_polygon(tier.snow.iter().map(|p| at(*p)).collect(), snow, Stroke::NONE));
    }
}

/// The Frost skin's winter forest: stars and a moon over the window, a far row of pines along the
/// bottom, a drift of snow, a nearer row standing in it and a big pine in each corner (see `pine.rs`). Drawn first,
/// behind the cards. `t` is the time in seconds (0 holds the stars still).
pub fn paint_pine_forest(ui: &egui::Ui, t: f32, strength: f32) {
    use crate::pine;
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let (w, h) = (rect.width(), rect.height());
    let k = (strength * 1.1).clamp(0.0, 1.0);
    for s in pine::stars(40) {
        let p = pos2(rect.left() + s.x * w, rect.top() + s.y * h);
        painter.circle_filled(p, s.size, Color32::from_rgb(214, 232, 255).gamma_multiply(0.7 * pine::twinkle(&s, t) * k));
    }
    let moon = pos2(rect.left() + 0.13 * w, rect.top() + 0.10 * h);
    soft_ellipse(&painter, moon, vec2(70.0, 70.0), [170, 205, 255], 0.18 * k);
    painter.circle_filled(moon, 15.0, Color32::from_rgb(226, 238, 252).gamma_multiply(0.9 * k));
    let foot = |x: f32, lift: f32| pos2(rect.left() + x * w, rect.bottom() - pine::drift(x) * h * lift);
    for p in pine::row(13, 0, 0.09, 0.16) {
        paint_pine(&painter, &pine::tree(p.seed, p.tiers), foot(p.x, 0.7), p.height * h, Color32::from_rgb(34, 60, 82), Color32::from_rgb(190, 210, 230), 0.8 * k);
    }
    // the snow the nearer trees stand in, lying over the feet of the far ones
    let mut mesh = egui::Mesh::default();
    let snow = Color32::from_rgba_unmultiplied(200, 218, 236, (0.4 * k * 255.0) as u8);
    let steps = 48u32;
    for i in 0..=steps {
        let x = i as f32 / steps as f32;
        mesh.colored_vertex(pos2(rect.left() + x * w, rect.bottom() - pine::drift(x) * h), snow);
        mesh.colored_vertex(pos2(rect.left() + x * w, rect.bottom()), snow);
    }
    for i in 0..steps {
        let a = 2 * i;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    painter.add(Shape::mesh(mesh));
    for p in pine::row(7, 1, 0.16, 0.26) {
        paint_pine(&painter, &pine::tree(p.seed, p.tiers), foot(p.x, 0.5), p.height * h, Color32::from_rgb(20, 42, 56), Color32::from_rgb(220, 236, 250), 0.9 * k);
    }
    let big = Color32::from_rgb(12, 30, 40);
    let white = Color32::from_rgb(236, 246, 255);
    paint_pine(&painter, &pine::tree(7, 6), foot(0.03, 0.4), 0.42 * h, big, white, k);
    paint_pine(&painter, &pine::tree(9, 5), foot(0.975, 0.4), 0.34 * h, big, white, k);
}

/// The Descent skin's art: a deep red glow rising from the bottom of the window and a single thin
/// thread running down into the dark, with a bead of light slowly sliding down it. `t` is the time in
/// seconds (0 holds it still).
pub fn paint_descent(ui: &egui::Ui, t: f32, strength: f32) {
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let k = strength.clamp(0.0, 1.0);
    let red = theme::accent();
    soft_ellipse(&painter, pos2(rect.center().x, rect.bottom()), vec2(rect.width() * 0.7, rect.height() * 0.4), [255, 42, 60], 0.2 * k);
    // the thread: from about 58% of the way down to 88%, swaying a little
    let at = |f: f32| {
        let y = rect.top() + (0.58 + 0.30 * f) * rect.height();
        pos2(rect.center().x + (f * 5.0 + t * 0.25).sin() * 6.0, y)
    };
    let points: Vec<egui::Pos2> = (0..=24).map(|i| at(i as f32 / 24.0)).collect();
    painter.add(Shape::line(points, Stroke::new(1.0, red.gamma_multiply(0.35 * k))));
    let bead = at((t / 12.0).rem_euclid(1.0));
    soft_ellipse(&painter, bead, vec2(9.0, 9.0), [255, 42, 60], 0.5 * k);
    painter.circle_filled(bead, 1.8, red.gamma_multiply(0.85 * k));
}

/// The shop's Bubbles backdrop: beams of light coming down from the surface and bubbles rising
/// through them (see `ambient.rs`). `t` is the time in seconds (0 holds it still).
pub fn paint_bubbles(ui: &egui::Ui, t: f32, strength: f32) {
    use crate::ambient;
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let (w, h) = (rect.width(), rect.height());
    let k = strength.clamp(0.0, 1.0);
    for i in 0..ambient::SHAFTS {
        let s = ambient::shaft(i, t);
        let (x, half) = (rect.left() + s.x * w, s.width * w / 2.0);
        let bottom = x + s.lean * w;
        let lit = Color32::from_rgba_unmultiplied(150, 230, 240, (s.alpha * 1.6 * k * 255.0).clamp(0.0, 255.0) as u8);
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(pos2(x - half, rect.top()), lit);
        mesh.colored_vertex(pos2(x + half, rect.top()), lit);
        mesh.colored_vertex(pos2(bottom + half * 2.0, rect.bottom()), Color32::TRANSPARENT);
        mesh.colored_vertex(pos2(bottom - half * 2.0, rect.bottom()), Color32::TRANSPARENT);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(Shape::mesh(mesh));
    }
    for i in 0..ambient::BUBBLES {
        let b = ambient::bubble(i, t);
        let c = pos2(rect.left() + b.x * w, rect.top() + b.y * h);
        let a = (b.alpha * k * 255.0).clamp(0.0, 255.0) as u8;
        painter.circle_filled(c, b.radius, Color32::from_rgba_unmultiplied(170, 235, 245, a / 8));
        painter.circle_stroke(c, b.radius, Stroke::new(1.0, Color32::from_rgba_unmultiplied(170, 235, 245, a)));
        painter.circle_filled(c + vec2(-b.radius * 0.35, -b.radius * 0.35), (b.radius * 0.2).max(0.7), Color32::from_rgba_unmultiplied(255, 255, 255, a));
    }
}

/// The shop's Scanlines backdrop: fine lines across the window in the accent colour, a brighter band
/// rolling slowly down, and a prompt with a blinking cursor in the bottom left corner (see `ambient.rs`).
/// `t` is the time in seconds (0 holds it still).
pub fn paint_scanlines(ui: &egui::Ui, t: f32, strength: f32) {
    use crate::ambient;
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let k = strength.clamp(0.0, 1.0);
    let tint = theme::accent();
    let line = Stroke::new(1.0, tint.gamma_multiply(0.07 * ambient::scan_flicker(t) * k));
    let mut y = rect.top();
    while y < rect.bottom() {
        painter.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], line);
        y += ambient::SCAN_SPACING;
    }
    let band = rect.top() + ambient::scan_band(t) * rect.height();
    let (r, g, b) = (tint.r(), tint.g(), tint.b());
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(pos2(rect.left(), band - 0.08 * rect.height()), Color32::TRANSPARENT);
    mesh.colored_vertex(pos2(rect.right(), band - 0.08 * rect.height()), Color32::TRANSPARENT);
    mesh.colored_vertex(pos2(rect.left(), band), Color32::from_rgba_unmultiplied(r, g, b, (0.10 * k * 255.0) as u8));
    mesh.colored_vertex(pos2(rect.right(), band), Color32::from_rgba_unmultiplied(r, g, b, (0.10 * k * 255.0) as u8));
    mesh.colored_vertex(pos2(rect.left(), band + 0.01 * rect.height()), Color32::TRANSPARENT);
    mesh.colored_vertex(pos2(rect.right(), band + 0.01 * rect.height()), Color32::TRANSPARENT);
    for a in [0u32, 2] {
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    painter.add(Shape::mesh(mesh));
    let prompt = pos2(rect.left() + 14.0, rect.bottom() - 12.0);
    painter.text(prompt, egui::Align2::LEFT_BOTTOM, ">", egui::FontId::monospace(16.0), tint.gamma_multiply(0.6 * k));
    if ambient::cursor_on(t) {
        painter.rect_filled(egui::Rect::from_min_size(prompt + vec2(14.0, -15.0), vec2(9.0, 15.0)), 0.0, tint.gamma_multiply(0.6 * k));
    }
}

/// The shop's Fireflies backdrop: soft yellow-green lights wandering about and glowing now and then
/// (see `ambient.rs`). `t` is the time in seconds (0 holds it still).
pub fn paint_fireflies(ui: &egui::Ui, t: f32, strength: f32) {
    use crate::ambient;
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let k = strength.clamp(0.0, 1.0);
    for i in 0..ambient::FIREFLIES {
        let f = ambient::firefly(i, t);
        let c = pos2(rect.left() + f.x * rect.width(), rect.top() + f.y * rect.height());
        let rgb = ambient::firefly_colour(f.glow);
        if f.glow > 0.05 {
            soft_ellipse(&painter, c, vec2(f.size * 9.0, f.size * 9.0), rgb, 0.4 * f.glow * k);
        }
        painter.circle_filled(c, f.size * 0.6, Color32::from_rgb(rgb[0], rgb[1], rgb[2]).gamma_multiply((0.1 + 0.9 * f.glow) * k));
    }
}

/// The shop's Snowfall backdrop: snow drifting down, the nearer flakes bigger and brighter (see
/// `ambient.rs`). `t` is the time in seconds (0 holds it still).
pub fn paint_snowfall(ui: &egui::Ui, t: f32, strength: f32) {
    use crate::ambient;
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let k = strength.clamp(0.0, 1.0);
    for i in 0..ambient::FLAKES {
        let f = ambient::flake(i, t);
        let c = pos2(rect.left() + f.x * rect.width(), rect.top() + f.y * rect.height());
        painter.circle_filled(c, f.size, Color32::from_rgb(226, 238, 252).gamma_multiply(f.alpha * k));
    }
}

/// The shop's Storm backdrop: rain slanting down and, now and then, a flash of lightning far off with
/// a thin bolt (see `ambient.rs`). `t` is the time in seconds (0 holds it still).
pub fn paint_storm(ui: &egui::Ui, t: f32, strength: f32) {
    use crate::ambient;
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let (w, h) = (rect.width(), rect.height());
    let k = strength.clamp(0.0, 1.0);
    let flash = ambient::flash(t);
    if flash > 0.01 {
        painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(190, 210, 255, (flash * 0.16 * k * 255.0) as u8));
    }
    let slant = vec2(ambient::RAIN_SLANT, 1.0).normalized();
    for i in 0..ambient::DROPS {
        let d = ambient::drop(i, t);
        let head = pos2(rect.left() + d.x * w, rect.top() + d.y * h);
        painter.line_segment([head - slant * d.len, head], Stroke::new(1.0, Color32::from_rgb(170, 195, 230).gamma_multiply(d.alpha * k)));
    }
    if flash > 0.25 {
        for line in ambient::bolt(t) {
            let points: Vec<egui::Pos2> = line.iter().map(|p| pos2(rect.left() + p.0 * w, rect.top() + p.1 * h)).collect();
            painter.add(Shape::line(points.clone(), Stroke::new(4.0, Color32::from_rgb(190, 210, 255).gamma_multiply(0.14 * flash * k))));
            painter.add(Shape::line(points, Stroke::new(1.2, Color32::from_rgb(235, 240, 255).gamma_multiply(0.9 * flash * k))));
        }
    }
}

/// The Constellations backdrop (it comes with the program, made for Midnight): stars that twinkle slowly
/// and drift a hair, with faint lines between the ones that lie close (see `ambient.rs`). `t` is the time in
/// seconds (0 holds it still).
pub fn paint_constellations(ui: &egui::Ui, t: f32, strength: f32) {
    use crate::ambient;
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let (w, h) = (rect.width(), rect.height());
    let k = strength.clamp(0.0, 1.0);
    let aspect = w / h;
    let stars: Vec<ambient::Star> = (0..ambient::STARS).map(|i| ambient::star(i, t)).collect();
    let at = |s: &ambient::Star| pos2(rect.left() + s.x * w, rect.top() + s.y * h);
    for (i, a) in stars.iter().enumerate() {
        for b in &stars[i + 1..] {
            let near = ambient::link(*a, *b, aspect);
            if near > 0.0 {
                let alpha = 0.32 * near * a.glow.min(b.glow) * k;
                painter.line_segment([at(a), at(b)], Stroke::new(1.0, Color32::from_rgb(140, 160, 255).gamma_multiply(alpha)));
            }
        }
    }
    for s in &stars {
        let c = at(s);
        soft_ellipse(&painter, c, vec2(s.size * 5.0, s.size * 5.0), [150, 170, 255], 0.35 * s.glow * k);
        painter.circle_filled(c, s.size * 0.7, Color32::from_rgb(214, 224, 255).gamma_multiply((0.3 + 0.7 * s.glow) * k));
    }
}

/// The Sunbeams backdrop (it comes with the program, made for Copper): soft slanting beams of warm light that
/// swell and fade, with dust drifting up through them (see `ambient.rs`). `t` is the time in seconds (0 holds
/// it still).
pub fn paint_sunbeams(ui: &egui::Ui, t: f32, strength: f32) {
    use crate::ambient;
    let rect = ui.clip_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    let (w, h) = (rect.width(), rect.height());
    let k = strength.clamp(0.0, 1.0);
    let lean = ambient::BEAM_LEAN * h;
    for i in 0..ambient::BEAMS {
        let b = ambient::beam(i, t);
        let left = rect.left() + b.x * w;
        let right = left + b.width * w;
        let warm = Color32::from_rgba_unmultiplied(236, 160, 96, (b.alpha * k * 255.0).clamp(0.0, 255.0) as u8);
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(pos2(left, rect.top()), warm);
        mesh.colored_vertex(pos2(right, rect.top()), warm);
        mesh.colored_vertex(pos2(left + lean, rect.bottom()), Color32::TRANSPARENT);
        mesh.colored_vertex(pos2(right + lean, rect.bottom()), Color32::TRANSPARENT);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(1, 3, 2);
        painter.add(Shape::mesh(mesh));
    }
    for i in 0..ambient::MOTES {
        let m = ambient::mote(i, t);
        let c = pos2(rect.left() + m.x * w, rect.top() + m.y * h);
        painter.circle_filled(c, m.size * 0.7, Color32::from_rgb(255, 214, 160).gamma_multiply(m.alpha * 0.8 * k));
    }
}
