// Game 1 on screen: the cat inside Father Time's clock (the rules are in game1.rs).
use crate::game1::{self, Game, Phase};
use crate::{father, king, theme};
use eframe::egui::{self, pos2, vec2, Color32, Shape, Stroke};

const GOLD: Color32 = Color32::from_rgb(255, 224, 130);

/// A colour that adds light (alpha 0, premultiplied): `a` is how much of `c` is added.
fn glow(c: Color32, a: f32) -> Color32 {
    let m = |v: u8| (v as f32 * a.clamp(0.0, 1.0)) as u8;
    Color32::from_rgba_premultiplied(m(c.r()), m(c.g()), m(c.b()), 0)
}

/// A picture drawn as a grid of quads whose corners are moved by `disp`, so it can wave.
fn warped_image(painter: &egui::Painter, id: egui::TextureId, rect: egui::Rect, tint: Color32, disp: &dyn Fn(egui::Pos2) -> egui::Pos2) {
    const N: usize = 16;
    let mut mesh = egui::Mesh::with_texture(id);
    for j in 0..=N {
        for i in 0..=N {
            let (u, v) = (i as f32 / N as f32, j as f32 / N as f32);
            let pos = disp(pos2(rect.min.x + rect.width() * u, rect.min.y + rect.height() * v));
            mesh.vertices.push(egui::epaint::Vertex { pos, uv: pos2(u, v), color: tint });
        }
    }
    let row = (N + 1) as u32;
    for j in 0..N as u32 {
        for i in 0..N as u32 {
            let a = j * row + i;
            mesh.add_triangle(a, a + 1, a + row);
            mesh.add_triangle(a + 1, a + row + 1, a + row);
        }
    }
    painter.add(Shape::mesh(mesh));
}

/// A rectangle of light that fades from `from` along one side to nothing at the opposite side.
fn fade_quad(painter: &egui::Painter, corners: [egui::Pos2; 4], colours: [Color32; 4]) {
    let mut mesh = egui::Mesh::default();
    for (p, c) in corners.iter().zip(colours) {
        mesh.colored_vertex(*p, c);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

/// What the player did this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Still going.
    Playing,
    /// Escape: back to the program.
    Quit,
    /// The game was lost and the player wants another go.
    Retry,
    /// The game was won and the celebration is over.
    Finished,
    /// The checkpoint was read and the player goes on to part two.
    Part2,
}

fn key(ctx: &egui::Context, keys: &[egui::Key]) -> bool {
    ctx.input(|i| keys.iter().any(|k| i.key_down(*k)))
}

/// Arrow keys or WASD to move, Space to leap.
pub fn read_input(ctx: &egui::Context) -> game1::Input {
    use egui::Key::*;
    let axis = |neg: &[egui::Key], pos: &[egui::Key]| {
        (if key(ctx, pos) { 1.0 } else { 0.0 }) - (if key(ctx, neg) { 1.0 } else { 0.0 })
    };
    game1::Input {
        dx: axis(&[ArrowLeft, A], &[ArrowRight, D]),
        dy: axis(&[ArrowUp, W], &[ArrowDown, S]),
        leap: key(ctx, &[Space]),
    }
}

fn shade(v: f32) -> Color32 {
    theme::mix(theme::bg(), theme::accent_stroke(), v.clamp(0.0, 1.0))
}

/// Draws the whole game and plays it a frame. `unlocking` says winning also unlocks the Gold
/// Edition (for the message on the win screen).
pub fn show(
    ui: &mut egui::Ui,
    textures: &[egui::TextureHandle],
    strength: f32,
    game: &mut Game,
    unlocking: bool,
) -> Outcome {
    let ctx = ui.ctx().clone();
    // The game has a window of its own, so it fills it: the picture's stage is scaled to fit the
    // window (as big as it will go) and centred, with the rest of the window in the page colour.
    let avail = ui.available_size();
    let scale = father::fit_scale(avail.x, avail.y);
    let (area, response) = ui.allocate_exact_size(avail, egui::Sense::click());
    ui.painter().rect_filled(area, 0.0, theme::bg());
    let stage = egui::Rect::from_center_size(
        area.center(),
        vec2(father::PICTURE.0 * scale, father::stage_height(scale)),
    );
    let width = stage.width();
    // the HUD and the messages grow with the window (but not shrink below their normal size)
    let k = (scale / 0.55).clamp(1.0, 2.4);
    let mut outcome = Outcome::Playing;

    // input and time
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        outcome = Outcome::Quit;
    }
    let dt = ctx.input(|i| i.stable_dt);
    let input = read_input(&ctx);
    game.step(dt, input);
    let confirm = ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)) || response.clicked();
    match game.phase {
        Phase::Lost if game.since_end > 0.8 && confirm => outcome = Outcome::Retry,
        Phase::Checkpoint if game.since_end > 1.2 && confirm => outcome = Outcome::Part2,
        Phase::Won if (game.since_end > 2.5 && confirm) || game.since_end > 9.0 => outcome = Outcome::Finished,
        _ => {}
    }

    let (ox, oy) = father::origin((stage.min.x, stage.min.y), width, scale);
    let base = |p: (f32, f32)| pos2(ox + p.0 * scale, oy + p.1 * scale);
    // part two: the whole scene (clock, hands, sand, cat) rises and goes wavy, as if it were leaving time
    let rise = game.ascension();
    let wob = rise * rise;
    let clock = ctx.input(|i| i.time) as f32;
    let ks = scale / 0.55;
    let disp = move |p: egui::Pos2| -> egui::Pos2 {
        if wob <= 0.0 {
            return p;
        }
        let dx = (p.y * 0.021 + clock * 1.7).sin() * 9.0 * ks * wob + (p.y * 0.0087 - clock * 0.9).sin() * 5.0 * ks * wob;
        let dy = (p.x * 0.017 + clock * 1.3).sin() * 6.0 * ks * wob - 38.0 * ks * wob;
        p + vec2(dx, dy)
    };
    let at = |p: (f32, f32)| disp(base(p));
    let colour = theme::mix(super::backdrop::tint(strength), GOLD, rise * 0.5);
    let accent = theme::accent_stroke();
    let painter = ui.painter().with_clip_rect(stage);
    if textures.len() == 3 {
        let whole = egui::Rect::from_min_size(base((0.0, 0.0)), vec2(father::PICTURE.0 * scale, father::PICTURE.1 * scale));
        let all = egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        let draw = |i: usize, tint: Color32| {
            if wob > 0.0 {
                warped_image(&painter, textures[i].id(), whole, tint, &disp);
            } else {
                painter.image(textures[i].id(), whole, all, tint);
            }
        };
        draw(0, colour);
        let h = game.hands_now();
        let second = if h.second_shown {
            Some((h.second, if h.second_live { 1.0 } else { 0.3 }))
        } else {
            None
        };
        super::backdrop::paint_hands_at(&painter, &at, scale, colour.gamma_multiply(1.6), h.hour, h.minute, second);
        // part two's hand that turns the other way
        if let Some((seg, live)) = game.counter_segment() {
            let strength = if live { 1.0 } else { 0.3 };
            painter.line_segment([at(seg[0]), at(seg[1])], Stroke::new(1.4, colour.gamma_multiply(1.6 * strength)));
            painter.circle_filled(at(seg[1]), (4.0 * scale).max(2.0), colour.gamma_multiply(1.6 * strength));
        }
        draw(1, theme::bg());
        draw(2, colour);
    }

    // the clock face you can walk on
    let gold = GOLD;
    painter.circle_stroke(at(game1::CENTRE), game1::FIELD_RADIUS * scale, Stroke::new(1.5, theme::mix(accent.gamma_multiply(0.35), gold.gamma_multiply(0.7), rise)));

    // sand
    for g in &game.grains {
        let p = at(g.pos());
        let s = g.strength();
        painter.circle_filled(p, 13.0 * scale, accent.gamma_multiply(0.18 * s));
        painter.circle_filled(p, (6.5 * scale).max(2.5), Color32::from_rgb(255, 236, 170).gamma_multiply(s));
    }

    // the cat
    let flicker = game.hurt() && (game.t * 14.0) as u32 % 2 == 0;
    if !flicker || game.phase != Phase::Playing {
        let lift = game.leaping().map_or(0.0, |p| (p * std::f32::consts::PI).sin() * 70.0);
        if lift > 0.0 {
            // shadow on the clock
            painter.circle_filled(at((game.cat.0, game.cat.1 + 18.0)), 22.0 * scale * (1.0 - lift / 200.0), Color32::from_black_alpha(110));
        }
        // part two: the cat is rising. Light gathers round him, he floats, a halo forms and becomes a crown.
        let float = rise * 26.0 + rise * 5.0 * (game.t * 2.2).sin();
        if rise > 0.0 {
            let c = at((game.cat.0, game.cat.1 - float));
            for (r, a) in [(120.0, 0.05), (84.0, 0.08), (52.0, 0.12)] {
                painter.circle_filled(c, r * scale * (0.5 + 0.7 * rise), gold.gamma_multiply(a * rise));
            }
            // light falling upward off him
            for i in 0..6 {
                let phase = (game.t * 0.7 + i as f32 / 6.0).fract();
                let x = ((i as f32 * 2.399) .sin()) * 36.0;
                painter.circle_filled(
                    at((game.cat.0 + x, game.cat.1 - float - phase * 170.0)),
                    (3.5 * scale).max(1.5),
                    gold.gamma_multiply((1.0 - phase) * rise),
                );
            }
            let top = at((game.cat.0, game.cat.1 - float - 74.0));
            painter.circle_stroke(top, 22.0 * scale * (0.6 + 0.4 * rise), Stroke::new(1.8, gold.gamma_multiply(0.5 + 0.5 * rise)));
            if rise > 0.6 {
                // the halo grows points: a crown
                let grow = ((rise - 0.6) / 0.4).clamp(0.0, 1.0);
                for i in 0..5 {
                    let a = -std::f32::consts::FRAC_PI_2 + (i as f32 - 2.0) * 0.5;
                    let from = top + vec2(a.cos(), a.sin()) * 22.0 * scale;
                    let to = top + vec2(a.cos(), a.sin()) * (22.0 + 20.0 * grow) * scale;
                    painter.line_segment([from, to], Stroke::new(2.2, gold.gamma_multiply(grow)));
                    painter.circle_filled(to, (3.2 * scale).max(1.5), gold.gamma_multiply(grow));
                }
            }
        }
        let mut pose = king::pose(0.0, None, false);
        pose.tilt = game.facing * 8.0;
        pose.bob = -lift - float;
        if game.leaping().is_some() {
            pose.jaw = 0.7;
            pose.squint = 0.7;
        }
        if game.hurt() {
            pose.squint = 0.9;
            pose.brow = 1.0;
        }
        if game.phase == Phase::Lost {
            pose.squint = 1.0;
        }
        for poly in king::cat_scene(&pose, game.cat, 0.062) {
            let points: Vec<egui::Pos2> = poly.points.iter().map(|&p| at(p)).collect();
            if points.len() < 2 {
                continue;
            }
            if poly.fill != king::Tone::None && points.len() >= 3 {
                let middle = points.iter().fold(egui::Vec2::ZERO, |sum, p| sum + p.to_vec2()) / points.len() as f32;
                let fill = if poly.fill == king::Tone::Eye { Color32::WHITE } else { shade(poly.fill.value() * 0.85) };
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
            if poly.line.1 > 0.0 {
                let stroke = Stroke::new((poly.line.1 * 0.062 * 2.0).max(1.0), accent);
                painter.add(if poly.closed { Shape::closed_line(points, stroke) } else { Shape::line(points, stroke) });
            }
        }
    }

    // part two: the gold light closes in. A wash, glowing edges, and slow beams falling from above.
    if rise > 0.0 {
        let k2 = rise * rise;
        painter.rect_filled(stage, 0.0, glow(GOLD, 0.10 * k2));
        let (edge, inner) = (glow(GOLD, 0.55 * k2), glow(GOLD, 0.0));
        let depth = stage.height().min(stage.width()) * 0.3;
        let (l, r, t, b) = (stage.left(), stage.right(), stage.top(), stage.bottom());
        fade_quad(&painter, [pos2(l, t), pos2(l + depth, t), pos2(l + depth, b), pos2(l, b)], [edge, inner, inner, edge]);
        fade_quad(&painter, [pos2(r - depth, t), pos2(r, t), pos2(r, b), pos2(r - depth, b)], [inner, edge, edge, inner]);
        fade_quad(&painter, [pos2(l, t), pos2(r, t), pos2(r, t + depth), pos2(l, t + depth)], [edge, edge, inner, inner]);
        fade_quad(&painter, [pos2(l, b - depth), pos2(r, b - depth), pos2(r, b), pos2(l, b)], [inner, inner, edge, edge]);
        for i in 0..6 {
            let sway = (clock * 0.35 + i as f32 * 1.7).sin() * 30.0 * ks;
            let x = l + stage.width() * (i as f32 + 0.5) / 6.0 + sway;
            let w = (22.0 + 14.0 * (i as f32 * 2.1).sin().abs()) * ks;
            let top = glow(GOLD, 0.16 * k2);
            fade_quad(&painter, [pos2(x - w, t), pos2(x + w, t), pos2(x + w * 2.5, b), pos2(x - w * 2.5, b)], [top, top, inner, inner]);
        }
    }

    // Father Time's words
    if let Some((text, fade)) = game.line() {
        let font = egui::FontId::monospace(34.0 * scale);
        let centre = at(father::SPEECH_AT);
        painter.text(centre + vec2(1.5, 1.5), egui::Align2::CENTER_CENTER, text, font.clone(), Color32::from_black_alpha((200.0 * fade) as u8));
        painter.text(centre, egui::Align2::CENTER_CENTER, text, font, accent.gamma_multiply(fade));
    }

    // the HUD: lives, sand, the leap, the minute
    let hud = egui::Rect::from_min_size(stage.min + vec2(10.0 * k, 8.0 * k), vec2(stage.width() - 20.0 * k, 28.0 * k));
    for i in 0..game1::MAX_LIVES {
        let c = hud.min + vec2((10.0 + i as f32 * 22.0) * k, 10.0 * k);
        if i < game.lives {
            painter.circle_filled(c, 7.0 * k, accent);
        } else {
            painter.circle_stroke(c, 7.0 * k, Stroke::new(1.2, accent.gamma_multiply(0.4)));
        }
    }
    painter.text(
        hud.right_top() + vec2(0.0, 2.0 * k),
        egui::Align2::RIGHT_TOP,
        format!("sand {}", game.sand),
        egui::FontId::monospace(15.0 * k),
        accent,
    );
    let bar = egui::Rect::from_min_size(pos2(stage.min.x + 14.0 * k, stage.max.y - 18.0 * k), vec2(stage.width() - 28.0 * k, 6.0 * k));
    painter.rect_filled(bar, 3.0, accent.gamma_multiply(0.18));
    painter.rect_filled(egui::Rect::from_min_size(bar.min, vec2(bar.width() * game.progress(), bar.height())), 3.0, accent);
    let leap = egui::Rect::from_min_size(pos2(stage.min.x + 14.0 * k, stage.max.y - 30.0 * k), vec2(80.0 * k, 4.0 * k));
    painter.rect_filled(leap, 2.0, accent.gamma_multiply(0.18));
    painter.rect_filled(egui::Rect::from_min_size(leap.min, vec2(leap.width() * game.leap_ready(), leap.height())), 2.0, Color32::from_rgb(255, 236, 170));
    painter.text(
        pos2(leap.right() + 8.0 * k, leap.center().y),
        egui::Align2::LEFT_CENTER,
        "leap (Space)",
        egui::FontId::monospace(11.0 * k),
        accent.gamma_multiply(0.8),
    );
    if game.phase == Phase::Playing && game.t < 6.0 {
        painter.text(
            stage.center_top() + vec2(0.0, 46.0 * k),
            egui::Align2::CENTER_TOP,
            "arrows / WASD to move   Space to leap   Esc to leave",
            egui::FontId::monospace(13.0 * k),
            accent.gamma_multiply((1.0 - (game.t - 4.0).max(0.0) / 2.0).clamp(0.0, 1.0)),
        );
    }

    // the end
    match game.phase {
        Phase::Lost if game.since_end > 0.5 => {
            painter.rect_filled(stage, 0.0, Color32::from_black_alpha(150));
            painter.text(stage.center(), egui::Align2::CENTER_CENTER, "TIME ALWAYS WINS", egui::FontId::monospace(30.0 * k), accent);
            painter.text(
                stage.center() + vec2(0.0, 36.0 * k),
                egui::Align2::CENTER_CENTER,
                "Enter: try again     Esc: leave",
                egui::FontId::monospace(15.0 * k),
                accent.gamma_multiply(0.8),
            );
        }
        Phase::Checkpoint => {
            let a = (game.since_end / 0.8).clamp(0.0, 1.0);
            painter.rect_filled(stage, 0.0, Color32::from_black_alpha((150.0 * a) as u8));
            painter.text(stage.center(), egui::Align2::CENTER_CENTER, "CHECKPOINT", egui::FontId::monospace(30.0 * k), accent.gamma_multiply(a));
            painter.text(
                stage.center() + vec2(0.0, 36.0 * k),
                egui::Align2::CENTER_CENTER,
                "His hour is only beginning.",
                egui::FontId::monospace(16.0 * k),
                gold.gamma_multiply(a),
            );
            if game.since_end > 1.2 {
                painter.text(
                    stage.center() + vec2(0.0, 66.0 * k),
                    egui::Align2::CENTER_CENTER,
                    "Enter: go on     Esc: leave (it is kept)",
                    egui::FontId::monospace(14.0 * k),
                    accent.gamma_multiply(0.8),
                );
            }
        }
        Phase::Won => {
            let a = (game.since_end / 1.0).clamp(0.0, 1.0);
            painter.rect_filled(stage, 0.0, Color32::from_black_alpha((140.0 * a) as u8));
            painter.text(stage.center(), egui::Align2::CENTER_CENTER, "YOU OUTLASTED TIME", egui::FontId::monospace(30.0 * k), accent.gamma_multiply(a));
            if unlocking {
                painter.text(
                    stage.center() + vec2(0.0, 38.0 * k),
                    egui::Align2::CENTER_CENTER,
                    "GOLD EDITION UNLOCKED",
                    egui::FontId::monospace(20.0 * k),
                    Color32::from_rgb(255, 224, 130).gamma_multiply(a),
                );
            }
        }
        _ => {}
    }

    if game.phase == Phase::Playing {
        ctx.request_repaint();
    } else {
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
    outcome
}
