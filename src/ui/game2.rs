// Game 2 on screen: the staff of time in its golden fist against the angel cats and the cat king (rules in game2.rs).
use super::game::Outcome;
use crate::game2::{self, Game, Phase, ShipTone, ShotKind, ARENA};
use crate::{angels, gold, king};
use eframe::egui::{self, pos2, vec2, Color32, Shape, Stroke};

fn gold_at(value: f32, alpha: f32) -> Color32 {
    let [r, g, b] = gold::ramp(value);
    Color32::from_rgba_unmultiplied(r, g, b, (alpha.clamp(0.0, 1.0) * 255.0) as u8)
}

const CREAM: Color32 = Color32::from_rgb(255, 240, 190);

/// Space, to fire; the arrows or A / D to turn, and up or W to thrust.
pub fn read_input(ctx: &egui::Context) -> game2::Input {
    use egui::Key::*;
    let down = |keys: &[egui::Key]| ctx.input(|i| keys.iter().any(|k| i.key_down(*k)));
    game2::Input {
        turn: (down(&[ArrowRight, D]) as i32 - down(&[ArrowLeft, A]) as i32) as f32,
        thrust: down(&[ArrowUp, W]),
        fire: down(&[Space]),
    }
}

fn hash01(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    ((x.wrapping_mul(0x297A_2D39) >> 8) & 0xFFFF) as f32 / 65535.0
}

/// Fills a polygon (a fan from its middle, as the shapes are all round it) and outlines it.
fn fan(painter: &egui::Painter, points: Vec<egui::Pos2>, fill: Color32) {
    if points.len() < 3 {
        return;
    }
    let middle = points.iter().fold(egui::Vec2::ZERO, |sum, p| sum + p.to_vec2()) / points.len() as f32;
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

/// The king's polygons, moved and scaled by `map` (`px` is how many screen points one picture
/// pixel has become, for the line widths).
fn paint_king(painter: &egui::Painter, pose: &king::Pose, map: &dyn Fn((f32, f32)) -> egui::Pos2, px: f32) {
    for poly in king::scene(pose) {
        let points: Vec<egui::Pos2> = poly.points.iter().map(|&p| map(p)).collect();
        if points.len() < 2 {
            continue;
        }
        if poly.fill != king::Tone::None && points.len() >= 3 {
            fan(painter, points.clone(), gold_at(poly.fill.value(), 1.0));
        }
        if poly.glow > 0.0 {
            let halo = Stroke::new((7.0 * px).max(2.0), gold_at(0.9, 0.16 * poly.glow.min(1.0)));
            painter.add(if poly.closed { Shape::closed_line(points.clone(), halo) } else { Shape::line(points.clone(), halo) });
        }
        if poly.line.1 > 0.0 {
            let stroke = Stroke::new((poly.line.1 * px).max(1.0), gold_at(poly.line.0, 1.0));
            painter.add(if poly.closed { Shape::closed_line(points, stroke) } else { Shape::line(points, stroke) });
        }
    }
}

fn rgba(c: (u8, u8, u8), alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.0, c.1, c.2, (alpha.clamp(0.0, 1.0) * 255.0) as u8)
}

/// The Void: a black hole ringed in purple, its spiral arms and its edges split into cyan and
/// magenta that jitter now and then, as if the picture were tearing. `r` is in screen points.
fn paint_void(painter: &egui::Painter, c: egui::Pos2, r: f32, clock: f32) {
    if r < 1.0 {
        return;
    }
    const CYAN: (u8, u8, u8) = (70, 240, 255);
    const MAGENTA: (u8, u8, u8) = (255, 60, 200);
    const PURPLE: (u8, u8, u8) = (130, 50, 220);
    let tear = hash01((clock * 14.0) as u32, 7);
    let off = r * 0.015 + 1.5 + if tear > 0.8 { r * 0.05 * tear } else { 0.0 };
    painter.circle_filled(c, r * 1.35, rgba(PURPLE, 0.08));
    painter.circle_filled(c, r * 1.15, rgba(PURPLE, 0.12));
    painter.circle_filled(c, r, Color32::from_rgb(4, 1, 10));
    painter.circle_stroke(c + vec2(-off, 0.0), r, Stroke::new(2.5, rgba(CYAN, 0.8)));
    painter.circle_stroke(c + vec2(off, 0.0), r, Stroke::new(2.5, rgba(MAGENTA, 0.8)));
    painter.circle_stroke(c, r, Stroke::new(1.5, rgba(PURPLE, 0.9)));
    for arm in 0..5u32 {
        let base = arm as f32 * std::f32::consts::TAU / 5.0 + clock * 1.6;
        let points = |shift: egui::Vec2| -> Vec<egui::Pos2> {
            (1..=36)
                .map(|i| {
                    let f = i as f32 / 36.0;
                    let a = base - f * 5.0;
                    c + shift + vec2(a.cos(), a.sin()) * r * f * 0.96
                })
                .collect()
        };
        painter.add(Shape::line(points(vec2(-off, 0.0)), Stroke::new(1.6, rgba(CYAN, 0.45))));
        painter.add(Shape::line(points(vec2(off, 0.0)), Stroke::new(1.6, rgba(MAGENTA, 0.45))));
        painter.add(Shape::line(points(vec2(0.0, 0.0)), Stroke::new(2.0, rgba(PURPLE, 0.6))));
    }
    painter.circle_filled(c, r * 0.3, Color32::from_rgb(2, 0, 5));
}

/// Draws the whole game and plays it a frame. `angels_tex` is the angel cats picture (three cats
/// side by side). Beating the king plays his death: the Void opens and takes everything in.
pub fn show(ui: &mut egui::Ui, angels_tex: Option<&egui::TextureHandle>, game: &mut Game) -> Outcome {
    let ctx = ui.ctx().clone();
    let avail = ui.available_size();
    let scale = (avail.x / ARENA.0).min(avail.y / ARENA.1).max(0.2);
    let (area, response) = ui.allocate_exact_size(avail, egui::Sense::click());
    ui.painter().rect_filled(area, 0.0, Color32::from_rgb(6, 5, 3));
    let arena = egui::Rect::from_center_size(area.center(), vec2(ARENA.0 * scale, ARENA.1 * scale));
    let at = |p: (f32, f32)| pos2(arena.min.x + p.0 * scale, arena.min.y + p.1 * scale);
    let k = (scale / 0.7).clamp(1.0, 2.4);
    let mut outcome = Outcome::Playing;

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        // the king is dead by then, so leaving cannot lose the prize
        outcome = if game.phase == Phase::Won { Outcome::Finished } else { Outcome::Quit };
    }
    let dt = ctx.input(|i| i.stable_dt);
    game.step(dt, read_input(&ctx));
    let confirm = ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space)) || response.clicked();
    match game.phase {
        Phase::Lost if game.since_end > 1.0 && confirm => outcome = Outcome::Retry,
        Phase::Won if game.finished() => outcome = Outcome::Finished,
        _ => {}
    }

    let painter = ui.painter().with_clip_rect(arena);
    let t = game.t;
    let clock = ctx.input(|i| i.time) as f32;
    // once the king is dead the Void pulls everything in: where a thing is, and how big it still is
    let fin = game.finale();
    let pull = |p: (f32, f32), delay: f32| match &fin {
        Some(f) if f.pull > 0.0 => game2::pulled(p, f.centre, f.pull, delay),
        _ => (p, 1.0),
    };
    let lag = |p: (f32, f32)| match &fin {
        Some(f) => (game2::dist(p, f.centre) / 1000.0).min(1.0) * game2::PULL_STAGGER * 0.75,
        None => 0.0,
    };
    // the sky: gold at the top, black below, with beams of light coming down
    let (top, bottom) = (Color32::from_rgb(64, 41, 10), Color32::from_rgb(6, 5, 3));
    let mut sky = egui::Mesh::default();
    sky.colored_vertex(arena.left_top(), top);
    sky.colored_vertex(arena.right_top(), top);
    sky.colored_vertex(arena.right_bottom(), bottom);
    sky.colored_vertex(arena.left_bottom(), bottom);
    sky.add_triangle(0, 1, 2);
    sky.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(sky));
    let apex = pos2(arena.center().x, arena.top() - arena.height() * 0.15);
    let mut beams = egui::Mesh::default();
    for n in 0..9 {
        let angle = (n as f32 - 4.0) * 0.21;
        let shimmer = 0.6 + 0.4 * (t * 0.6 + n as f32 * 1.7).sin();
        let end = |a: f32| apex + vec2(a.sin(), a.cos()) * arena.height() * 1.5;
        let near = gold_at(0.7, 0.10 * shimmer);
        let first = beams.vertices.len() as u32;
        beams.colored_vertex(apex, near);
        beams.colored_vertex(end(angle - 0.04), Color32::TRANSPARENT);
        beams.colored_vertex(end(angle + 0.04), Color32::TRANSPARENT);
        beams.add_triangle(first, first + 1, first + 2);
    }
    painter.add(Shape::mesh(beams));
    // stars drifting down, which is what makes it feel like flying
    for n in 0..80u32 {
        let depth = hash01(n, 3);
        let y = (hash01(n, 1) * ARENA.1 + t * (14.0 + 60.0 * depth)) % ARENA.1;
        let x = hash01(n, 2) * ARENA.0;
        let (pos, sz) = pull((x, y), hash01(n, 5) * game2::PULL_STAGGER);
        painter.circle_filled(at(pos), (0.7 + 1.4 * depth) * scale.max(0.6) * sz, gold_at(0.85, 0.25 + 0.5 * depth));
    }

    // the heavenly coliseum behind the king: three storeys of arches over the sand
    let rise = game.coliseum_amount() * fin.map_or(1.0, |f| 1.0 - f.swallow);
    if rise > 0.0 {
        for wall in game2::coliseum() {
            let points: Vec<egui::Pos2> = wall.points.iter().map(|&q| at(q)).collect();
            if wall.filled {
                fan(&painter, points.clone(), gold_at(0.45, 0.15 * rise));
            }
            let stroke = Stroke::new((1.3 * scale).max(1.0), gold_at(wall.tone, 0.34 * rise));
            painter.add(if wall.closed { Shape::closed_line(points, stroke) } else { Shape::line(points, stroke) });
        }
    }

    // the angel cats, each with a streak of light above it
    if let Some(tex) = angels_tex {
        let h = 120.0;
        let w = h * angels::CELL.0 / angels::CELL.1;
        for a in &game.angels {
            let (pos, sz) = pull(a.pos, lag(a.pos));
            let c = at(pos);
            let scale = scale * sz;
            painter.line_segment([c - vec2(0.0, 40.0 * scale), c - vec2(0.0, 150.0 * scale)], Stroke::new(2.5 * scale, gold_at(0.8, 0.12)));
            painter.circle_filled(c, 58.0 * scale, gold_at(0.6, 0.09));
            let [u0, v0, u1, v1] = angels::uv(a.kind as usize);
            painter.image(
                tex.id(),
                egui::Rect::from_center_size(c, vec2(w, h) * scale),
                egui::Rect::from_min_max(pos2(u0, v0), pos2(u1, v1)),
                Color32::WHITE,
            );
            if a.flash > 0.0 {
                painter.circle_filled(c, 46.0 * scale, Color32::from_white_alpha((a.flash * 1500.0).min(150.0) as u8));
            }
        }
    }

    // the king
    if let Some(kg) = &game.king {
        let mut pose = king::pose(t, None, true);
        pose.tilt += 2.0 * (kg.age * 0.5).sin();
        if kg.flash > 0.0 {
            pose.brow = 1.0;
            pose.squint = 0.5;
        }
        if game.phase == Phase::Won {
            pose.jaw = 1.0;
            pose.squint = 0.8;
        }
        let gone = fin.map_or(0.0, |f| f.king_gone);
        let throes = if game.phase == Phase::Won { (game.since_end / game2::FINALE_DEATH).clamp(0.0, 1.0) } else { 0.0 };
        let shake = if throes > 0.0 && gone < 1.0 {
            vec2((clock * 71.0).sin(), (clock * 53.0).cos()) * (3.0 + 9.0 * throes) * scale
        } else {
            vec2(0.0, 0.0)
        };
        let s = 0.37 * (1.0 - 0.9 * gone * gone);
        let c = at(kg.pos) + shake;
        painter.circle_filled(c, 170.0 * scale, gold_at(0.7, 0.07));
        painter.circle_filled(c, 120.0 * scale, gold_at(0.8, 0.07));
        let map = |p: (f32, f32)| pos2(c.x + (p.0 - 700.0) * s * scale, c.y + (p.1 - 515.0) * s * scale);
        if gone < 1.0 {
            // rays of light bursting out of him as he dies
            for i in 0..14u32 {
                let a = i as f32 * 0.449 + hash01(i, 6) + clock * 0.3;
                let len = (60.0 + 260.0 * hash01(i, 8)) * throes * (1.0 - gone);
                painter.line_segment([c + vec2(a.cos(), a.sin()) * 40.0 * scale, c + vec2(a.cos(), a.sin()) * (40.0 + len) * scale], Stroke::new(3.0 * scale, gold_at(0.95, 0.5 * throes)));
            }
            paint_king(&painter, &pose, &map, s * scale);
        }
        if throes > 0.0 {
            painter.circle_filled(c, 130.0 * scale * (1.0 - gone), Color32::from_white_alpha((90.0 * throes * (1.0 - gone)) as u8));
        }
        if kg.flash > 0.0 {
            painter.circle_filled(c, 100.0 * scale, Color32::from_white_alpha((kg.flash * 900.0).min(90.0) as u8));
        }
    }

    // what they shoot
    for s in &game.shots {
        let c = at(s.pos);
        let len = (s.vel.0 * s.vel.0 + s.vel.1 * s.vel.1).sqrt().max(1.0);
        let dir = vec2(s.vel.0 / len, s.vel.1 / len);
        match s.kind {
            ShotKind::Arrow => {
                painter.line_segment([c - dir * 15.0 * scale, c + dir * 13.0 * scale], Stroke::new(3.0 * scale, CREAM));
                let side = vec2(-dir.y, dir.x) * 5.0 * scale;
                let tip = c + dir * 20.0 * scale;
                painter.add(Shape::convex_polygon(vec![tip, c + dir * 8.0 * scale + side, c + dir * 8.0 * scale - side], CREAM, Stroke::NONE));
            }
            ShotKind::Note => {
                painter.circle_filled(c, 15.0 * scale, gold_at(0.7, 0.16));
                painter.circle_filled(c, 7.0 * scale, gold_at(0.9, 1.0));
                painter.line_segment([c + vec2(6.0, 0.0) * scale, c + vec2(6.0, -24.0) * scale], Stroke::new(2.5 * scale, gold_at(0.9, 1.0)));
                painter.line_segment([c + vec2(6.0, -24.0) * scale, c + vec2(15.0, -17.0) * scale], Stroke::new(2.5 * scale, gold_at(0.9, 1.0)));
            }
            ShotKind::Blast => {
                painter.circle_filled(c, 20.0 * scale, gold_at(0.7, 0.14));
                painter.rect_filled(egui::Rect::from_center_size(c, vec2(12.0, 36.0) * scale), 6.0 * scale, CREAM);
            }
            ShotKind::Orb => {
                painter.circle_filled(c, 20.0 * scale, Color32::from_rgb(255, 150, 50).gamma_multiply(0.28));
                painter.circle_filled(c, 11.0 * scale, Color32::from_rgb(255, 176, 64));
                painter.circle_filled(c, 5.0 * scale, CREAM);
            }
        }
    }
    // the staff's bolts
    for b in &game.bolts {
        let (pos, sz) = pull(b.pos, lag(b.pos));
        let c = at(pos);
        let scale = scale * sz;
        let len = (b.vel.0 * b.vel.0 + b.vel.1 * b.vel.1).sqrt().max(1.0);
        let dir = vec2(b.vel.0 / len, b.vel.1 / len);
        painter.line_segment([c - dir * 26.0 * scale, c], Stroke::new(8.0 * scale, gold_at(0.8, 0.22)));
        painter.line_segment([c - dir * 24.0 * scale, c], Stroke::new(3.5 * scale, CREAM));
    }

    // the staff of time in its golden fist (and, once the king is dead, the last thing the Void
    // takes: it spins as it falls in)
    let p = game.player;
    let flicker = p.invuln > 0.0 && (t * 16.0) as u32 % 2 == 0 && game.phase == Phase::Playing;
    let (ppos, psz) = pull(p.pos, game2::PULL_STAGGER);
    let sc = scale * psz;
    let ang = p.angle + fin.map_or(0.0, |f| ((f.pull - game2::PULL_STAGGER) / game2::PULL_TRAVEL).clamp(0.0, 1.0).powi(2) * std::f32::consts::TAU * 2.0);
    if game.phase != Phase::Lost && !flicker && psz > 0.02 {
        let place = |l: (f32, f32)| {
            let r = game2::turned(l, ang);
            at((ppos.0 + r.0 * psz, ppos.1 + r.1 * psz))
        };
        // thrust: a jet of golden sand out of the butt of the staff
        if p.thrusting && game.phase == Phase::Playing {
            let butt = game2::STAFF_BUTT;
            let flame = 28.0 + 16.0 * (t * 60.0).sin().abs();
            let pts = vec![place((butt.0 - 7.0, butt.1)), place((butt.0, butt.1 + flame)), place((butt.0 + 7.0, butt.1))];
            painter.add(Shape::convex_polygon(pts, Color32::from_rgb(255, 170, 60), Stroke::NONE));
            let pts = vec![place((butt.0 - 3.5, butt.1)), place((butt.0, butt.1 + flame * 0.6)), place((butt.0 + 3.5, butt.1))];
            painter.add(Shape::convex_polygon(pts, CREAM, Stroke::NONE));
            for i in 0..5u32 {
                let drift = (t * 9.0 + i as f32 * 0.37).fract();
                let spread = (hash01(i, (t * 9.0) as u32) - 0.5) * 22.0 * drift;
                painter.circle_filled(place((butt.0 + spread, butt.1 + 10.0 + (flame + 26.0) * drift)), 2.0 * sc, gold_at(0.9, 1.0 - drift));
            }
        }
        painter.circle_filled(at(ppos), 44.0 * sc, gold_at(0.7, 0.07));
        for piece in game2::ship(ang) {
            let pts: Vec<egui::Pos2> = piece.points.iter().map(|q| at((ppos.0 + q.0 * psz, ppos.1 + q.1 * psz))).collect();
            let fill = match piece.tone {
                ShipTone::Shaft => gold_at(0.8, 1.0),
                ShipTone::Glove => gold_at(0.68, 1.0),
                ShipTone::Knuckle => gold_at(0.84, 1.0),
                ShipTone::Cuff => gold_at(0.46, 1.0),
                ShipTone::Gem => gold_at(0.95, 1.0),
            };
            painter.add(Shape::convex_polygon(pts, fill, Stroke::new((1.2 * sc).max(1.0), gold_at(0.22, 1.0))));
        }
        // the head of the staff: an hourglass in a ring of light, which flares as it fires
        let head = place(game2::STAFF_HEAD);
        let firing = game.bolts.last().is_some_and(|b| b.age < 0.08);
        painter.circle_filled(head, (if firing { 26.0 } else { 16.0 }) * sc, gold_at(0.9, if firing { 0.45 } else { 0.18 }));
        painter.circle_stroke(head, 12.0 * sc, Stroke::new((1.6 * sc).max(1.0), CREAM));
        for (a, b, c) in [((-6.0, -6.0), (6.0, -6.0), (0.0, 0.0)), ((0.0, 0.0), (6.0, 6.0), (-6.0, 6.0))] {
            let q = |v: (f32, f32)| place((game2::STAFF_HEAD.0 + v.0, game2::STAFF_HEAD.1 + v.1));
            painter.add(Shape::convex_polygon(vec![q(a), q(b), q(c)], CREAM, Stroke::NONE));
        }
    }

    // explosions
    for b in &game.bursts {
        let pr = b.progress();
        let r = (if b.big { 170.0 } else { 56.0 }) * pr.sqrt();
        let c = at(b.pos);
        painter.circle_filled(c, r * scale, gold_at(0.9, 0.18 * (1.0 - pr)));
        painter.circle_stroke(c, r * scale, Stroke::new(3.0 * scale, gold_at(0.95, 1.0 - pr)));
        for i in 0..10u32 {
            let a = i as f32 * 0.628 + hash01(i, 9);
            let (inner, outer) = (r * 0.5, r * (0.9 + 0.5 * hash01(i, 4)));
            painter.line_segment(
                [c + vec2(a.cos(), a.sin()) * inner * scale, c + vec2(a.cos(), a.sin()) * outer * scale],
                Stroke::new(2.0 * scale, CREAM.gamma_multiply(1.0 - pr)),
            );
        }
    }

    // the Void: it opens where the king fell, and swallows the screen
    if let Some(f) = &fin {
        if f.swallow > 0.0 {
            painter.rect_filled(arena, 0.0, Color32::from_rgba_unmultiplied(8, 3, 18, (f.swallow * 255.0) as u8));
        }
        paint_void(&painter, at(f.centre), f.radius * scale, clock);
        if f.swallow > 0.0 {
            // glitch slices of cyan and magenta over it
            let frame = (clock * 10.0) as u32;
            for n in 0..9u32 {
                if hash01(frame, n) > 0.45 {
                    let y = arena.top() + hash01(frame, n + 20) * arena.height();
                    let h = (3.0 + 16.0 * hash01(frame, n + 40)) * k;
                    let x0 = arena.left() + hash01(frame, n + 60) * arena.width() * 0.6;
                    let w = arena.width() * (0.2 + 0.4 * hash01(frame, n + 80));
                    let tone = if n % 2 == 0 { (70, 240, 255) } else { (255, 60, 200) };
                    painter.rect_filled(
                        egui::Rect::from_min_size(pos2(x0, y), vec2(w, h)),
                        0.0,
                        Color32::from_rgba_unmultiplied(tone.0, tone.1, tone.2, (50.0 * f.swallow) as u8),
                    );
                }
            }
        }
    }

    // what is being said
    if let Some((text, fade)) = game.line() {
        let font = egui::FontId::monospace(30.0 * scale.max(0.7));
        let centre = at((ARENA.0 / 2.0, 120.0));
        painter.text(centre + vec2(1.5, 1.5), egui::Align2::CENTER_CENTER, text, font.clone(), Color32::from_black_alpha((200.0 * fade) as u8));
        painter.text(centre, egui::Align2::CENTER_CENTER, text, font, gold_at(0.95, fade));
    }

    // the HUD: lives, score, and how far you are (the angels, then the king)
    let hud = arena.min + vec2(12.0 * k, 10.0 * k);
    for i in 0..game.rules().lives {
        let c = hud + vec2((8.0 + i as f32 * 22.0) * k, 10.0 * k);
        if i < game.lives {
            painter.circle_filled(c, 7.0 * k, gold_at(0.85, 1.0));
        } else {
            painter.circle_stroke(c, 7.0 * k, Stroke::new(1.2, gold_at(0.6, 0.4)));
        }
    }
    if game.infinite {
        painter.text(
            hud + vec2(0.0, 30.0 * k),
            egui::Align2::LEFT_TOP,
            "INVINCIBLE",
            egui::FontId::monospace(12.0 * k),
            crate::theme::VOID_CYAN,
        );
    }
    painter.text(
        arena.right_top() + vec2(-12.0 * k, 10.0 * k),
        egui::Align2::RIGHT_TOP,
        format!("score {:06}", game.score),
        egui::FontId::monospace(15.0 * k),
        gold_at(0.85, 1.0),
    );
    let label = if game.king_out() {
        "THE CAT KING".to_string()
    } else {
        format!("angels {}/{}", game.kills.min(game.rules().kills_for_king), game.rules().kills_for_king)
    };
    let bar = egui::Rect::from_min_size(pos2(arena.min.x + 14.0 * k, arena.max.y - 18.0 * k), vec2(arena.width() - 28.0 * k, 6.0 * k));
    painter.rect_filled(bar, 3.0, gold_at(0.6, 0.2));
    painter.rect_filled(egui::Rect::from_min_size(bar.min, vec2(bar.width() * game.progress(), bar.height())), 3.0, gold_at(0.85, 1.0));
    painter.text(bar.left_top() - vec2(0.0, 4.0 * k), egui::Align2::LEFT_BOTTOM, label, egui::FontId::monospace(12.0 * k), gold_at(0.8, 0.9));
    if game.phase == Phase::Playing && game.t < 7.0 {
        painter.text(
            arena.center_top() + vec2(0.0, 44.0 * k),
            egui::Align2::CENTER_TOP,
            "left/right or A/D turn   up or W thrust   Space fire   Esc leave",
            egui::FontId::monospace(13.0 * k),
            gold_at(0.8, (1.0 - (game.t - 5.0).max(0.0) / 2.0).clamp(0.0, 1.0)),
        );
    }

    // the end
    match game.phase {
        Phase::Lost if game.since_end > 0.6 => {
            painter.rect_filled(arena, 0.0, Color32::from_black_alpha(150));
            painter.text(arena.center(), egui::Align2::CENTER_CENTER, "HEAVEN ALWAYS WINS", egui::FontId::monospace(30.0 * k), gold_at(0.85, 1.0));
            painter.text(
                arena.center() + vec2(0.0, 36.0 * k),
                egui::Align2::CENTER_CENTER,
                "Enter: try again     Esc: leave",
                egui::FontId::monospace(15.0 * k),
                gold_at(0.8, 0.85),
            );
        }
        Phase::Won => {
            // the king's last words come first; the Void takes the rest
            let x = game.since_end;
            let a = (((x - 0.5) / 0.6).min((2.6 - x) / 0.6)).clamp(0.0, 1.0);
            if a > 0.0 {
                painter.text(at((ARENA.0 / 2.0, 430.0)), egui::Align2::CENTER_CENTER, "THE CAT KING IS DOWN", egui::FontId::monospace(30.0 * k), gold_at(0.9, a));
            }
        }
        _ => {}
    }

    ctx.request_repaint();
    outcome
}
