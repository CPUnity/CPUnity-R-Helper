//! The Ally of Darkness, painted: the Alley as an escape room. The rules are in `alley` (the chain of
//! events) and `ally` (the saved state); this draws the city canyon wall by wall, the things on the walls,
//! what you carry, the close-ups and the circuit puzzle, and turns clicks into acts.

use super::backdrop::soft_ellipse;
use crate::alley::{self, Goal, Item, Look, Reaction, SpotId, Switches};
use crate::ally::{Puzzle, State};
use crate::ally_room::{self, Era, TOPICS};
use eframe::egui::{self, pos2, vec2, Color32, Pos2, Rect, Shape, Stroke};

/// The cat's talk at the end of the street.
struct CatTalk {
    /// One bit for each question already asked.
    asked: u8,
    /// What he last said, and when he began.
    reply: Option<(&'static str, f64)>,
}

pub struct Scene {
    /// Which wall you are looking at: 0 to 3 (see `alley::view_name`).
    view: usize,
    /// Which time you look at.
    era: Era,
    /// The clock sign's circuit, when it is open.
    puzzle: Option<Puzzle>,
    talk: Option<CatTalk>,
    say: Option<&'static str>,
    /// The close-up that is open.
    look: Option<Look>,
    /// The wheels of the lock that is open.
    wheels: [u8; 4],
    /// The panel of switches, when it is open.
    panel: Option<Switches>,
    /// The tries at the shield's code so far: (the four numbers, gold marks, pale marks).
    guesses: Vec<([u8; 4], u8, u8)>,
    /// When the shield was broken in this visit (it blinks and goes out).
    shield_down_at: Option<f64>,
    /// The thing selected in the bar.
    selected: Option<Item>,
    opened: f64,
    /// Things just picked up and on their way to the bar: (what, when, where from).
    pickups: Vec<(Item, f64, Pos2)>,
    /// Where the last click in the room was (a pick-up starts there).
    last_click: Option<Pos2>,
    /// When "Start over" was pressed once (it asks to be pressed again).
    confirm_reset: Option<f64>,
    /// How many hints of the current step were shown, and for which step.
    hint_goal: Option<Goal>,
    hint_shown: usize,
    /// The part of a second not yet added to the timer, and when the timer was last advanced.
    tick: f64,
    last_tick: f64,
}

/// How long a pick-up takes to arrive in the bar, in seconds (and a little more for the flash).
const PICKUP_SECS: f32 = 2.0;

impl Scene {
    pub fn new(now: f64) -> Scene {
        Scene {
            view: 0,
            era: Era::Now,
            puzzle: None,
            talk: None,
            say: None,
            look: None,
            wheels: [0; 4],
            panel: None,
            guesses: Vec::new(),
            shield_down_at: None,
            selected: None,
            opened: now,
            pickups: Vec::new(),
            last_click: None,
            confirm_reset: None,
            hint_goal: None,
            hint_shown: 0,
            tick: 0.0,
            last_tick: now,
        }
    }
}

/// What the scene wants the app to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    None,
    /// Walked away (Esc): back to the market, the way you came.
    Leave,
    /// Through the hatch: the way out, at the foot of the clock tower.
    Out,
}

pub struct Frame {
    pub exit: Exit,
    /// The quest state changed (the caller saves it).
    pub changed: bool,
}

const GOLD: Color32 = Color32::from_rgb(244, 200, 80);
const INK: Color32 = Color32::from_rgb(250, 240, 214);

fn alpha(c: [u8; 3], a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// A vertical gradient.
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

/// The colours of a time: (wall top, wall bottom, floor, the light).
fn palette(era: Era) -> (Color32, Color32, Color32, [u8; 3]) {
    match era {
        Era::Now => (
            Color32::from_rgb(8, 22, 28),
            Color32::from_rgb(14, 40, 46),
            Color32::from_rgb(5, 14, 18),
            [90, 220, 210],
        ),
        Era::Past => (
            Color32::from_rgb(30, 20, 12),
            Color32::from_rgb(58, 40, 24),
            Color32::from_rgb(22, 15, 10),
            [255, 190, 110],
        ),
        Era::Future => (
            Color32::from_rgb(6, 12, 34),
            Color32::from_rgb(14, 30, 62),
            Color32::from_rgb(4, 8, 22),
            [130, 170, 255],
        ),
    }
}

/// Draws the scene and handles the clicks and keys.

/// Draws the scene and handles the clicks and keys.
pub fn show(ui: &mut egui::Ui, scene: &mut Scene, state: &mut State, animated: bool) -> Frame {
    let mut out = Frame { exit: Exit::None, changed: false };
    let rect = ui.max_rect();
    if rect.width() < 240.0 || rect.height() < 200.0 {
        return out;
    }
    let now = ui.input(|i| i.time);
    // the first time in gives this run its codes
    state.seed_with((now * 1000.0) as u32);
    let before = *state;
    let t = if animated { (now - scene.opened) as f32 } else { 0.0 };
    let painter = ui.painter_at(rect);
    let bar_h = (rect.height() * 0.09).clamp(34.0, 56.0);
    let room = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - bar_h));
    let bar = Rect::from_min_max(pos2(rect.left(), room.bottom()), rect.max);
    gradient_bar(&painter, bar);
    let shield_age = scene.shield_down_at.map(|at| (now - at) as f32);
    paint_view(&painter, room, scene.view, state, scene.era, t, shield_age);
    let modal = scene.puzzle.is_some()
        || scene.talk.is_some()
        || scene.say.is_some()
        || scene.look.is_some();
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
    let carried = Item::carried(state);

    // the room: a click is an act
    let response = ui.interact(room, ui.id().with("ally_room"), egui::Sense::click());
    if !modal {
        if let Some(p) = response.hover_pos() {
            let (x, y) = ((p.x - room.left()) / room.width(), (p.y - room.top()) / room.height());
            if let Some(id) = alley::spot_at(scene.view, x, y) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                let o = object_rect(room, id);
                let line = Stroke::new(1.5, alpha([244, 200, 80], 0.4));
                match id {
                    SpotId::ClockSign => {
                        painter.circle_stroke(o.center(), o.height().min(o.width()) * 0.56, line);
                    }
                    SpotId::Hatch => {
                        painter.add(Shape::line(
                            ellipse_points(o.center(), o.width() * 0.52, o.height() * 0.52, 56),
                            line,
                        ));
                    }
                    _ => {
                        painter.rect_stroke(o.expand(3.0), 8.0, line, egui::StrokeKind::Outside);
                    }
                }
            }
        }
        if response.clicked() {
            if let Some(p) = response.interact_pointer_pos() {
                let (x, y) =
                    ((p.x - room.left()) / room.width(), (p.y - room.top()) / room.height());
                if let Some(id) = alley::spot_at(scene.view, x, y) {
                    scene.last_click = Some(p);
                    let reaction = alley::click(state, id, scene.selected, scene.era);
                    apply_reaction(scene, state, reaction, &mut out);
                    if scene.selected.is_some_and(|s| !s.held(state)) {
                        scene.selected = None;
                    }
                }
            }
        }
    }

    // turning to another wall
    let mut turn: i32 = 0;
    if turn_arrow(ui, &painter, room, true) && !modal {
        turn = -1;
    }
    if turn_arrow(ui, &painter, room, false) && !modal {
        turn = 1;
    }
    if !modal {
        if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            turn = -1;
        }
        if ui.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            turn = 1;
        }
    }
    if turn != 0 {
        scene.view = (scene.view as i32 + turn).rem_euclid(alley::VIEWS as i32) as usize;
    }

    // the wall's name and the time spent
    let secs = state.spent();
    painter.text(
        pos2(room.center().x, room.top() + 18.0),
        egui::Align2::CENTER_CENTER,
        format!("{}   {:02}:{:02}", alley::view_name(scene.view), secs / 60, secs % 60),
        egui::FontId::proportional((room.height() * 0.028).clamp(13.0, 20.0)),
        alpha([220, 226, 240], 0.75),
    );

    // what you carry
    let mut tip: Option<(Rect, Item)> = None;
    for (i, item) in carried.iter().enumerate() {
        let sr = slot_rect(room, i);
        let r = ui.interact(sr, ui.id().with(("ally_slot", i)), egui::Sense::click());
        let chosen = scene.selected == Some(*item);
        let flying = scene
            .pickups
            .iter()
            .any(|(it, born, _)| it == item && ((now - *born) as f32) < PICKUP_SECS);
        painter.rect_filled(sr, 8.0, Color32::from_rgba_unmultiplied(6, 8, 14, 205));
        painter.rect_stroke(
            sr,
            8.0,
            Stroke::new(
                if chosen { 3.0 } else { 1.5 },
                if chosen { GOLD } else { alpha([244, 200, 80], 0.35) },
            ),
            egui::StrokeKind::Inside,
        );
        if !flying {
            paint_item(&painter, *item, sr.center(), sr.width() * 0.34, 0.0, 0.0, 1.0);
        }
        if r.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            tip = Some((sr, *item));
        }
        if r.clicked() && !modal {
            match scene.selected {
                Some(sel) if sel == *item => scene.selected = None,
                Some(sel) => match alley::combine(state, sel, *item) {
                    Some(text) => {
                        scene.say = Some(text);
                        scene.selected = None;
                    }
                    None => {
                        scene.say = Some("Those two do not go together.");
                        scene.selected = Some(*item);
                    }
                },
                None => scene.selected = Some(*item),
            }
        }
    }
    if let Some((sr, item)) = tip {
        if !modal {
            let fs = (room.height() * 0.026).clamp(12.0, 19.0);
            let galley = painter.layout(
                format!("{}\n{}", item.name(), item.blurb()),
                egui::FontId::proportional(fs),
                INK,
                (room.width() * 0.34).max(220.0),
            );
            let size = galley.size() + vec2(fs, fs);
            let r = Rect::from_min_size(pos2(sr.left(), sr.bottom() + 6.0), size);
            painter.rect_filled(r, 8.0, Color32::from_rgb(10, 12, 22));
            painter.rect_stroke(r, 8.0, Stroke::new(1.5, GOLD), egui::StrokeKind::Inside);
            painter.galley(r.min + vec2(fs * 0.5, fs * 0.5), galley, INK);
        }
    }
    if let (Some(sel), Some(p)) = (scene.selected, ui.input(|i| i.pointer.hover_pos())) {
        if room.contains(p) && !modal {
            painter.text(
                p + vec2(16.0, 18.0),
                egui::Align2::LEFT_TOP,
                format!("using: {}", sel.name()),
                egui::FontId::proportional(15.0),
                alpha([255, 226, 140], 0.95),
            );
        }
    }

    // the times you can look at (once you hold a hand), and the buttons on the right
    let mut x = bar.left() + 12.0;
    if state.hands_held() > 0 {
        for era in Era::ALL {
            if !era.available(state) {
                continue;
            }
            let b = Rect::from_min_size(pos2(x, bar.top() + bar_h * 0.15), vec2(96.0, bar_h * 0.7));
            x += 104.0;
            let r = ui.interact(b, ui.id().with(("ally_era", era.name())), egui::Sense::click());
            let on = scene.era == era;
            painter.rect_filled(
                b,
                6.0,
                if on { alpha(palette(era).3, 0.35) } else { Color32::from_rgb(16, 18, 26) },
            );
            painter.rect_stroke(
                b,
                6.0,
                Stroke::new(1.5, if on { GOLD } else { alpha([160, 170, 190], 0.5) }),
                egui::StrokeKind::Inside,
            );
            painter.text(
                b.center(),
                egui::Align2::CENTER_CENTER,
                era.name(),
                egui::FontId::proportional((bar_h * 0.38).clamp(13.0, 20.0)),
                if on { INK } else { Color32::from_rgb(170, 176, 196) },
            );
            if r.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if r.clicked() && !modal && era.available(state) {
                scene.era = era;
            }
        }
    }
    let button_h = bar_h * 0.7;
    let top = bar.top() + bar_h * 0.15;
    let leave = Rect::from_min_size(pos2(bar.right() - 232.0, top), vec2(220.0, button_h));
    let again = Rect::from_min_size(pos2(leave.left() - 182.0, top), vec2(170.0, button_h));
    let hint = Rect::from_min_size(pos2(again.left() - 162.0, top), vec2(150.0, button_h));
    let leave_r = ui.interact(leave, ui.id().with("ally_leave"), egui::Sense::click());
    let again_r = ui.interact(again, ui.id().with("ally_again"), egui::Sense::click());
    let hint_r = ui.interact(hint, ui.id().with("ally_hint"), egui::Sense::click());
    let font = egui::FontId::proportional((bar_h * 0.34).clamp(12.0, 18.0));
    let asking = scene.confirm_reset.is_some_and(|at| now - at < 4.0);
    if !asking {
        scene.confirm_reset = None;
    }
    let button = |r: Rect, hovered: bool, fill: Color32, line: Color32, text: &str| {
        painter.rect_filled(r, 6.0, if hovered { lighten(fill) } else { fill });
        painter.rect_stroke(r, 6.0, Stroke::new(1.5, line), egui::StrokeKind::Inside);
        painter.text(r.center(), egui::Align2::CENTER_CENTER, text, font.clone(), INK);
    };
    button(leave, leave_r.hovered(), Color32::from_rgb(20, 16, 38), GOLD, "Save & exit  (Esc)");
    button(
        again,
        again_r.hovered(),
        if asking { Color32::from_rgb(90, 24, 30) } else { Color32::from_rgb(20, 16, 38) },
        alpha([200, 90, 90], 0.8),
        if asking { "Sure? Press again" } else { "Start over" },
    );
    button(
        hint,
        hint_r.hovered(),
        Color32::from_rgb(20, 16, 38),
        alpha([120, 200, 230], 0.8),
        "Hint  (+30 s)",
    );
    if leave_r.hovered() || again_r.hovered() || hint_r.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if hint_r.clicked() && !modal {
        let goal = alley::goal(state);
        if scene.hint_goal != Some(goal) {
            scene.hint_goal = Some(goal);
            scene.hint_shown = 0;
        }
        let level = scene.hint_shown.min(2);
        if scene.hint_shown < 3 {
            state.add_spent(alley::HINT_COST);
            scene.hint_shown += 1;
        }
        scene.say = Some(alley::hint(state, level));
    }
    if again_r.clicked() && !modal {
        if asking {
            // everything forgotten: the street again, as on the first visit
            state.reset();
            state.enter();
            state.seed_with((now * 1000.0) as u32);
            *scene = Scene::new(now);
            out.changed = true;
            return out;
        }
        scene.confirm_reset = Some(now);
    }

    // what is open over the room (only handled once it has been open for a frame, so the click that
    // opened it is not also a click on it)
    if modal {
        if scene.puzzle.is_some() {
            match puzzle_modal(
                ui,
                &painter,
                rect,
                scene.puzzle.as_mut(),
                "The clock sign's circuit",
                t,
                escape,
            ) {
                PuzzleResult::Open => {}
                PuzzleResult::Closed => scene.puzzle = None,
                PuzzleResult::Solved => {
                    scene.puzzle = None;
                    let (_, text) = alley::circuit_solved(state);
                    scene.say = Some(text);
                }
            }
        } else if scene.talk.is_some() {
            if talk_modal(ui, &painter, rect, scene.talk.as_mut(), state, now, t, animated, escape)
            {
                scene.talk = None;
            }
        } else if let Some(look) = scene.look {
            match look_modal(
                ui,
                &painter,
                rect,
                look,
                &mut scene.wheels,
                &mut scene.guesses,
                scene.panel.as_mut(),
                state,
                t,
                escape,
            ) {
                LookResult::Open => {}
                LookResult::Closed => {
                    scene.look = None;
                    scene.panel = None;
                }
                LookResult::Done(reaction) => {
                    scene.look = None;
                    scene.panel = None;
                    if let Reaction::Got(_, text) = reaction {
                        scene.say = Some(text);
                    }
                }
                LookResult::Shield => {
                    scene.look = None;
                    scene.shield_down_at = Some(now);
                    scene.say = Some(alley::shield_dropped(state));
                }
                LookResult::Powered => {
                    scene.look = None;
                    scene.panel = None;
                    scene.say = Some(alley::panel_solved(state));
                }
            }
        } else if let Some(text) = scene.say {
            if say_bar(ui, &painter, room, text) || escape {
                scene.say = None;
            }
        }
    } else if escape || leave_r.clicked() {
        if escape && scene.selected.is_some() {
            scene.selected = None;
        } else {
            out.exit = Exit::Leave;
        }
    }

    // what was just picked up flies to its place in the bar
    let after = Item::carried(state);
    let from = scene
        .last_click
        .take()
        .unwrap_or_else(|| pos2(room.center().x, room.top() + room.height() * 0.6));
    for item in &after {
        if !item.held(&before) {
            scene.pickups.push((*item, now, from));
        }
    }
    scene.pickups.retain(|(_, born, _)| ((now - *born) as f32) < PICKUP_SECS + 0.6);
    for (item, born, origin) in &scene.pickups {
        if let Some(i) = after.iter().position(|a| a == item) {
            paint_pickup(&painter, room, *item, slot_rect(room, i), (now - *born) as f32, *origin);
        }
    }
    if !scene.pickups.is_empty() {
        ui.ctx().request_repaint();
    }
    out.changed = *state != before;

    // the timer runs until the hour hand is yours (the first area's end)
    let dt = (now - scene.last_tick).clamp(0.0, 0.5);
    scene.last_tick = now;
    if !state.is_lit(0) {
        scene.tick += dt;
        while scene.tick >= 1.0 {
            scene.tick -= 1.0;
            state.add_spent(1);
        }
    }
    if animated {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }
    out
}

/// What a click did (opening the close-ups and so on).
fn apply_reaction(scene: &mut Scene, state: &State, reaction: Reaction, out: &mut Frame) {
    match reaction {
        Reaction::Nothing => {}
        Reaction::Say(text) | Reaction::Got(_, text) => scene.say = Some(text),
        Reaction::Look(look) => {
            scene.look = Some(look);
            scene.wheels = if look == Look::Shield { [1; 4] } else { [0; 4] };
            scene.last_click = None;
            if look == Look::Shield {
                scene.guesses.clear();
            }
            if look == Look::Panel {
                scene.panel = Some(Switches::make(state.seed()));
            }
        }
        Reaction::Talk => scene.talk = Some(CatTalk { asked: 0, reply: None }),
        Reaction::Circuit => {
            scene.puzzle = Some(alley::sign_puzzle(state.seed()));
            scene.last_click = None;
        }
        Reaction::Out => out.exit = Exit::Out,
        Reaction::Passage => {
            scene.say = Some(
                "The bricks are gone in the past: a way through to a back lot, lit by a street that no longer exists. The way on is not built yet.",
            )
        }
    }
}

fn lighten(c: Color32) -> Color32 {
    let l = |v: u8| v.saturating_add(24);
    Color32::from_rgb(l(c.r()), l(c.g()), l(c.b()))
}

/// The arrow at the edge of the room that turns you to the next wall. True when clicked.
fn turn_arrow(ui: &mut egui::Ui, painter: &egui::Painter, room: Rect, left: bool) -> bool {
    let zone_w = (room.width() * 0.06).clamp(40.0, 90.0);
    let zone = if left {
        Rect::from_min_max(
            pos2(room.left(), room.top() + room.height() * 0.2),
            pos2(room.left() + zone_w, room.bottom() - room.height() * 0.2),
        )
    } else {
        Rect::from_min_max(
            pos2(room.right() - zone_w, room.top() + room.height() * 0.2),
            pos2(room.right(), room.bottom() - room.height() * 0.2),
        )
    };
    let id = if left { "ally_turn_left" } else { "ally_turn_right" };
    let response = ui.interact(zone, ui.id().with(id), egui::Sense::click());
    let strength = if response.hovered() { 0.95 } else { 0.4 };
    let c = zone.center();
    let s = zone_w * 0.22;
    let d = if left { -1.0 } else { 1.0 };
    painter.add(Shape::line(
        vec![
            pos2(c.x + d * s * 0.5, c.y - s * 1.4),
            pos2(c.x - d * s * 0.5, c.y),
            pos2(c.x + d * s * 0.5, c.y + s * 1.4),
        ],
        Stroke::new(3.0, alpha([230, 220, 190], strength)),
    ));
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
}

/// The slot in the bar at the top left that the `i`th thing you carry sits in.
fn slot_rect(room: Rect, i: usize) -> Rect {
    let s = (room.height() * 0.075).clamp(40.0, 60.0);
    Rect::from_min_size(
        pos2(room.left() + 16.0 + i as f32 * (s + 8.0), room.top() + 34.0),
        vec2(s, s),
    )
}

/// The rectangle of a thing on the wall.
fn object_rect(room: Rect, id: SpotId) -> Rect {
    let q = alley::rect_of(id);
    sub(room, q[0], q[1], q[2], q[3])
}

/// A part of a rectangle, from fractions of it.
fn sub(r: Rect, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::from_min_max(
        pos2(r.left() + x0 * r.width(), r.top() + y0 * r.height()),
        pos2(r.left() + x1 * r.width(), r.top() + y1 * r.height()),
    )
}

fn gradient_bar(painter: &egui::Painter, bar: Rect) {
    gradient(painter, bar, Color32::from_rgb(10, 12, 18), Color32::from_rgb(4, 5, 9));
}

// ----------------------------------------------------------------------------------------- a line
/// A line of text at the foot of the room; true when it is clicked away.
fn say_bar(ui: &mut egui::Ui, painter: &egui::Painter, room: Rect, text: &str) -> bool {
    let fs = (room.height() * 0.034).clamp(15.0, 26.0);
    let width = (room.width() * 0.8).min(900.0);
    let galley =
        painter.layout(text.to_string(), egui::FontId::proportional(fs), INK, width - fs * 2.0);
    let size = galley.size() + vec2(fs * 2.0, fs * 1.6);
    let r = Rect::from_min_size(
        pos2(room.center().x - size.x / 2.0, room.bottom() - size.y - fs),
        size,
    );
    let response = ui.interact(room, ui.id().with("ally_say"), egui::Sense::click());
    painter.rect_filled(r, 12.0, Color32::from_rgb(12, 10, 24));
    painter.rect_stroke(r, 12.0, Stroke::new(2.0, GOLD), egui::StrokeKind::Inside);
    painter.galley(r.min + vec2(fs, fs * 0.8), galley, INK);
    painter.text(
        pos2(r.right() - fs * 0.5, r.bottom() - fs * 0.3),
        egui::Align2::RIGHT_BOTTOM,
        "click",
        egui::FontId::proportional(fs * 0.55),
        alpha([200, 190, 230], 0.6),
    );
    response.clicked()
}

// ---------------------------------------------------------------------------------------- puzzles
enum PuzzleResult {
    Open,
    Closed,
    Solved,
}

/// The circuit puzzle: turn the pieces until the current runs from the battery to the lamp.
fn puzzle_modal(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    puzzle: Option<&mut Puzzle>,
    title: &str,
    t: f32,
    escape: bool,
) -> PuzzleResult {
    let Some(puzzle) = puzzle else {
        return PuzzleResult::Closed;
    };
    painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 190));
    let n = puzzle.size;
    let side = (rect.height() * 0.78).min(rect.width() * 0.86);
    let cell = side / (n as f32 + 2.0);
    let grid_w = cell * (n as f32 + 2.0);
    let origin =
        pos2(rect.center().x - grid_w / 2.0, rect.center().y - cell * n as f32 / 2.0 + cell * 0.2);
    let cell_rect = |row: usize, col: usize| {
        Rect::from_min_size(
            pos2(origin.x + cell * (col as f32 + 1.0), origin.y + cell * row as f32),
            vec2(cell, cell),
        )
    };
    // the board
    let board = Rect::from_min_size(origin, vec2(grid_w, cell * n as f32)).expand(cell * 0.2);
    painter.rect_filled(board, 14.0, Color32::from_rgb(8, 20, 20));
    painter.rect_stroke(board, 14.0, Stroke::new(2.0, GOLD), egui::StrokeKind::Inside);
    painter.text(
        pos2(rect.center().x, board.top() - cell * 0.35),
        egui::Align2::CENTER_CENTER,
        format!("{title}   (click a piece to turn it)"),
        egui::FontId::proportional((cell * 0.34).clamp(15.0, 26.0)),
        INK,
    );
    let live = puzzle.live();
    let lit = puzzle.powered();
    let wire = |live: bool| {
        if live {
            Color32::from_rgb(255, 196, 70)
        } else {
            Color32::from_rgb(70, 96, 100)
        }
    };
    let mut turned: Option<(usize, usize)> = None;
    for row in 0..n {
        for col in 0..n {
            let r = cell_rect(row, col);
            let tile = puzzle.tiles[row * n + col];
            let is_live = live[row * n + col];
            painter.rect_filled(r.shrink(2.0), 6.0, Color32::from_rgb(14, 30, 32));
            let c = r.center();
            let w = (cell * 0.16).max(3.0);
            let mut any = false;
            for side in 0..4 {
                if tile.open(side) {
                    any = true;
                    let edge = match side {
                        0 => pos2(c.x, r.top()),
                        1 => pos2(r.right(), c.y),
                        2 => pos2(c.x, r.bottom()),
                        _ => pos2(r.left(), c.y),
                    };
                    painter.line_segment([c, edge], Stroke::new(w, wire(is_live)));
                }
            }
            if any {
                painter.circle_filled(c, w * 0.9, wire(is_live));
            }
            let response =
                ui.interact(r, ui.id().with(("ally_cell", row, col)), egui::Sense::click());
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                painter.rect_stroke(
                    r.shrink(2.0),
                    6.0,
                    Stroke::new(1.5, alpha([255, 230, 160], 0.7)),
                    egui::StrokeKind::Inside,
                );
            }
            if response.clicked() && tile.kind != crate::ally::Kind::Empty {
                turned = Some((row, col));
            }
        }
    }
    // the battery on the left of its row, and the lamp on the right of its row
    let b = cell_rect(puzzle.source, 0);
    let battery = Rect::from_center_size(
        pos2(b.left() - cell * 0.5, b.center().y),
        vec2(cell * 0.55, cell * 0.7),
    );
    painter.rect_filled(battery, 4.0, Color32::from_rgb(70, 200, 120));
    painter.rect_filled(
        Rect::from_center_size(
            pos2(battery.center().x, battery.top() - cell * 0.06),
            vec2(cell * 0.2, cell * 0.12),
        ),
        2.0,
        Color32::from_rgb(200, 240, 210),
    );
    painter.line_segment(
        [pos2(battery.right(), b.center().y), pos2(b.left(), b.center().y)],
        Stroke::new((cell * 0.16).max(3.0), wire(live[puzzle.source * n])),
    );
    let l = cell_rect(puzzle.lamp, n - 1);
    let lamp = pos2(l.right() + cell * 0.5, l.center().y);
    painter.line_segment(
        [pos2(l.right(), l.center().y), lamp],
        Stroke::new((cell * 0.16).max(3.0), wire(lit)),
    );
    if lit {
        soft_ellipse(painter, lamp, vec2(cell * 1.1, cell * 1.1), [255, 210, 110], 0.5);
    }
    painter.circle_filled(
        lamp,
        cell * 0.26,
        if lit { Color32::from_rgb(255, 226, 140) } else { Color32::from_rgb(40, 44, 50) },
    );
    painter.circle_stroke(lamp, cell * 0.26, Stroke::new(2.0, GOLD));
    let _ = t;
    // a way back
    let back =
        Rect::from_min_size(pos2(rect.right() - 170.0, rect.top() + 10.0), vec2(160.0, 28.0));
    let back_r = ui.interact(back, ui.id().with("ally_puzzle_back"), egui::Sense::click());
    painter.text(
        back.right_center(),
        egui::Align2::RIGHT_CENTER,
        "Esc: put it down",
        egui::FontId::proportional(14.0),
        alpha([200, 190, 230], if back_r.hovered() { 0.95 } else { 0.5 }),
    );
    if let Some((row, col)) = turned {
        puzzle.turn(row, col);
        if puzzle.powered() {
            return PuzzleResult::Solved;
        }
    }
    if escape || back_r.clicked() {
        return PuzzleResult::Closed;
    }
    PuzzleResult::Open
}

// ---------------------------------------------------------------------------------- the cat talks
/// The talk in the archway (placeholder lines in `ally_room`): true when it is over.
#[allow(clippy::too_many_arguments)]
fn talk_modal(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    talk: Option<&mut CatTalk>,
    state: &State,
    now: f64,
    t: f32,
    animated: bool,
    escape: bool,
) -> bool {
    let Some(talk) = talk else {
        return true;
    };
    let (w, h) = (rect.width(), rect.height());
    painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(4, 2, 12, 225));
    // the cat, close: he stands in the arch
    let k = (h / 820.0).min(w * 0.26 / 430.0).min(h / 1500.0);
    let hx = rect.left() + w * 0.25;
    let hy = rect.top() + h * 0.08 + 170.0 * k;
    let (text, since): (&str, f32) = match talk.reply {
        Some((text, at)) => (text, (now - at) as f32),
        None => ("There you are. Careful. It is worse in here than it looks.", 99.0),
    };
    let typing = crate::cat_talk::revealed(text, since) < text.chars().count();
    soft_ellipse(
        painter,
        pos2(hx, hy + 300.0 * k),
        vec2(520.0 * k, 700.0 * k),
        [60, 90, 120],
        0.18,
    );
    super::cat_talk::paint_cat_portrait(painter, hx, hy, k, t, animated, typing);
    // what he says
    let fs = (h * 0.03).clamp(15.0, 28.0);
    let pad = fs * 0.8;
    let col_left = rect.left() + w * 0.50;
    let col_width = (w * 0.45).max(220.0);
    let font = egui::FontId::proportional(fs);
    let shown = crate::cat_talk::revealed(text, since);
    let r = super::cat_talk::speech(
        painter,
        text,
        shown,
        &font,
        INK,
        pos2(col_left, rect.top() + h * 0.09),
        col_width,
        pad,
        Color32::from_rgb(18, 12, 34),
        GOLD,
        Some(pos2(hx + 330.0 * k * 0.5, hy + 110.0 * k * 0.5)),
    );
    // what you can ask
    let mut y = r.bottom() + pad;
    let mut picked: Option<usize> = None;
    for (i, label) in TOPICS.iter().enumerate() {
        if !ally_room::topic_open(i, talk.asked) {
            continue;
        }
        let bar = Rect::from_min_size(pos2(col_left, y), vec2(col_width, fs * 2.0));
        y += fs * 2.3;
        let response = ui.interact(bar, ui.id().with(("ally_ask", i)), egui::Sense::click());
        let hot = response.hovered();
        let asked = talk.asked & (1 << i) != 0;
        painter.rect_filled(
            bar,
            8.0,
            if hot { Color32::from_rgb(34, 24, 62) } else { Color32::from_rgb(20, 14, 40) },
        );
        painter.rect_stroke(
            bar,
            8.0,
            Stroke::new(1.5, if hot { GOLD } else { alpha([200, 180, 120], 0.5) }),
            egui::StrokeKind::Inside,
        );
        painter.text(
            pos2(bar.left() + pad, bar.center().y),
            egui::Align2::LEFT_CENTER,
            *label,
            font.clone(),
            if asked { Color32::from_rgb(170, 160, 190) } else { INK },
        );
        if hot {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            picked = Some(i);
        }
    }
    if let Some(i) = picked {
        talk.asked |= 1 << i;
        talk.reply = Some((ally_room::cat_reply(i, state), now));
    }
    let back =
        Rect::from_min_size(pos2(rect.right() - 190.0, rect.top() + 10.0), vec2(180.0, 28.0));
    let back_r = ui.interact(back, ui.id().with("ally_talk_back"), egui::Sense::click());
    painter.text(
        back.right_center(),
        egui::Align2::RIGHT_CENTER,
        "Esc: step back",
        egui::FontId::proportional(14.0),
        alpha([200, 190, 230], if back_r.hovered() { 0.95 } else { 0.5 }),
    );
    escape || back_r.clicked()
}

// ----------------------------------------------------------------------------- what you carry
/// A thing drawn at `c` (`size` is about half its height): a gum wrapper, a fuse, a screwdriver, a token, a hand
/// of the clock or a key. `spin` turns it; `glow` lights it from behind; `tint` fades it.
fn paint_item(
    painter: &egui::Painter,
    item: Item,
    c: Pos2,
    size: f32,
    spin: f32,
    glow: f32,
    tint: f32,
) {
    let a = tint.clamp(0.0, 1.0);
    let col = |rgb: [u8; 3]| alpha(rgb, a);
    let (sin, cos) = spin.sin_cos();
    let at = |x: f32, y: f32| {
        let (x, y) = (x * size, y * size);
        c + vec2(x * cos - y * sin, x * sin + y * cos)
    };
    let line = |p: Pos2, q: Pos2, w: f32, rgb: [u8; 3]| {
        painter.line_segment([p, q], Stroke::new((w * size).max(1.2), col(rgb)));
    };
    let quad = |x0: f32, y0: f32, x1: f32, y1: f32, rgb: [u8; 3], edge: [u8; 3]| {
        painter.add(Shape::convex_polygon(
            vec![at(x0, y0), at(x1, y0), at(x1, y1), at(x0, y1)],
            col(rgb),
            Stroke::new(1.0, col(edge)),
        ));
    };
    if glow > 0.0 {
        soft_ellipse(painter, c, vec2(size * 1.6, size * 1.6), [255, 210, 110], 0.35 * glow);
    }
    match item {
        Item::Foil => {
            quad(-0.75, -0.5, 0.75, 0.5, [208, 214, 224], [120, 126, 140]);
            line(at(-0.75, 0.0), at(0.75, 0.0), 0.05, [150, 156, 170]);
            line(at(0.0, -0.5), at(0.0, 0.5), 0.05, [150, 156, 170]);
            quad(-0.75, -0.5, -0.45, 0.5, [150, 60, 80], [100, 40, 60]);
        }
        Item::BurntFuse => {
            quad(-0.75, -0.28, 0.75, 0.28, [150, 160, 170], [100, 110, 120]);
            quad(-1.0, -0.32, -0.75, 0.32, [190, 120, 60], [120, 70, 30]);
            quad(0.75, -0.32, 1.0, 0.32, [190, 120, 60], [120, 70, 30]);
            line(at(-0.75, 0.0), at(-0.3, 0.1), 0.06, [30, 30, 34]);
            line(at(0.3, -0.1), at(0.75, 0.0), 0.06, [30, 30, 34]);
            painter.circle_filled(at(0.0, 0.0), size * 0.18, col([20, 20, 24]));
        }
        Item::Fuse => {
            quad(-0.75, -0.28, 0.75, 0.28, [170, 182, 194], [100, 110, 120]);
            quad(-1.0, -0.32, -0.75, 0.32, [190, 120, 60], [120, 70, 30]);
            quad(0.75, -0.32, 1.0, 0.32, [190, 120, 60], [120, 70, 30]);
            line(at(-0.75, 0.0), at(0.75, 0.0), 0.06, [230, 150, 70]);
            quad(-0.25, -0.34, 0.3, 0.34, [214, 220, 230], [140, 146, 160]);
        }
        Item::Screwdriver => {
            quad(-1.0, -0.24, -0.25, 0.24, [226, 120, 40], [120, 60, 20]);
            line(at(-0.25, 0.0), at(0.95, 0.0), 0.12, [170, 176, 186]);
            quad(0.8, -0.16, 1.0, 0.16, [190, 196, 206], [110, 116, 126]);
        }
        Item::Token => {
            painter.circle_filled(at(0.0, 0.0), size * 0.72, col([190, 146, 52]));
            painter.circle_stroke(at(0.0, 0.0), size * 0.72, Stroke::new(1.5, col([110, 80, 20])));
            painter.circle_stroke(
                at(0.0, 0.0),
                size * 0.46,
                Stroke::new((size * 0.06).max(1.2), col([240, 200, 100])),
            );
            line(at(0.0, -0.3), at(0.0, 0.3), 0.1, [240, 200, 100]);
        }
        Item::Hand(i) => {
            let len = [0.9, 1.15, 1.4][i.min(2)];
            let width = [0.2, 0.14, 0.07][i.min(2)];
            line(at(0.0, len * 0.28), at(0.0, -len), width, [244, 200, 80]);
            painter.circle_filled(at(0.0, 0.0), (width * size * 1.3).max(2.0), col([244, 200, 80]));
            painter.circle_filled(
                at(0.0, -len),
                (width * size * 0.9).max(2.0),
                col([244, 200, 80]),
            );
        }
        Item::WarehouseKey => {
            painter.circle_stroke(
                at(0.0, -0.5),
                size * 0.3,
                Stroke::new((size * 0.13).max(1.5), col([244, 200, 80])),
            );
            line(at(0.0, -0.2), at(0.0, 0.8), 0.12, [244, 200, 80]);
            for dy in [0.5f32, 0.7] {
                line(at(0.0, dy), at(0.22, dy), 0.1, [244, 200, 80]);
            }
        }
    }
}

/// A thing you have just got: it comes out of where you clicked, turns and shines in the middle of the room
/// with its name, then flies to its place in the bar and the slot flashes.
fn paint_pickup(painter: &egui::Painter, room: Rect, item: Item, slot: Rect, age: f32, from: Pos2) {
    let ease = |u: f32| u * u * (3.0 - 2.0 * u);
    let centre = pos2(room.center().x, room.top() + room.height() * 0.42);
    let big = room.height() * 0.1;
    let small = slot.width() * 0.34;
    if age >= PICKUP_SECS {
        // it lands: a ring leaves the slot
        let u = ((age - PICKUP_SECS) / 0.6).clamp(0.0, 1.0);
        painter.circle_stroke(
            slot.center(),
            slot.width() * (0.5 + 1.2 * u),
            Stroke::new(2.5, alpha([255, 226, 140], 1.0 - u)),
        );
        return;
    }
    let (pos, size) = if age < 0.6 {
        let u = ease(age / 0.6);
        (from.lerp(centre, u), mix(small, big, u))
    } else if age < 1.3 {
        (centre, big * (1.0 + 0.05 * (age * 10.0).sin()))
    } else {
        let u = ease((age - 1.3) / 0.7);
        (centre.lerp(slot.center(), u), mix(big, small, u))
    };
    if age < 1.3 {
        // rings of light leaving it, and sparks
        let ring = (age * 1.4).fract();
        painter.circle_stroke(
            pos,
            big * (0.8 + 1.6 * ring),
            Stroke::new(2.0, alpha([255, 220, 130], 0.5 * (1.0 - ring))),
        );
        for k in 0..12u32 {
            let a = k as f32 * std::f32::consts::TAU / 12.0 + unit(k) * 0.4;
            let d = (age - 0.25).max(0.0) * big * (0.8 + unit(k + 30));
            painter.circle_filled(
                pos + vec2(a.cos(), a.sin()) * d,
                1.5 + 2.0 * unit(k + 60),
                alpha([255, 226, 140], (1.0 - age / 1.3).clamp(0.0, 1.0)),
            );
        }
    }
    paint_item(painter, item, pos, size, if age < 1.3 { age * 1.6 } else { 0.0 }, 1.0, 1.0);
    if (0.4..1.5).contains(&age) {
        let a = ((age - 0.4) / 0.3).min(1.0) * (1.0 - ((age - 1.2) / 0.3).clamp(0.0, 1.0));
        painter.text(
            centre + vec2(0.0, big * 1.7),
            egui::Align2::CENTER_CENTER,
            item.name(),
            egui::FontId::proportional((room.height() * 0.035).clamp(16.0, 30.0)),
            alpha([250, 240, 214], a),
        );
    }
}

// ------------------------------------------------------------------------------------- the city
fn base_rgb(era: Era) -> [u8; 3] {
    match era {
        Era::Now => [10, 16, 26],
        Era::Past => [28, 17, 11],
        Era::Future => [8, 10, 32],
    }
}

fn window_rgb(era: Era) -> [u8; 3] {
    match era {
        Era::Now => [255, 206, 120],
        Era::Past => [255, 176, 86],
        Era::Future => [140, 200, 255],
    }
}

/// The wall you face, the things on it, and the weather over it all.
fn paint_view(
    painter: &egui::Painter,
    room: Rect,
    view: usize,
    state: &State,
    era: Era,
    t: f32,
    shield_age: Option<f32>,
) {
    let painter = &painter.with_clip_rect(room);
    let light = palette(era).3;
    let powered = state.flag(alley::F_POWER);
    if view % alley::VIEWS == 0 {
        paint_street(painter, room, state, era, t, light, powered);
    } else {
        paint_facade(painter, room, view, era, t, light, powered);
    }
    paint_objects(painter, room, view % alley::VIEWS, state, era, t, light, shield_age);
    paint_weather(painter, room, t, era, light);
}

/// Sky: a dark gradient, clouds drifting across it and, now and then, lightning behind them. Clouds sit in the
/// top `band` of the rectangle.
fn paint_sky(painter: &egui::Painter, r: Rect, t: f32, era: Era, band: f32) {
    let (w, h) = (r.width(), r.height());
    let (top, bottom) = match era {
        Era::Now => (Color32::from_rgb(3, 4, 10), Color32::from_rgb(20, 15, 36)),
        Era::Past => (Color32::from_rgb(18, 10, 8), Color32::from_rgb(58, 32, 24)),
        Era::Future => (Color32::from_rgb(2, 4, 16), Color32::from_rgb(14, 22, 56)),
    };
    gradient(painter, r, top, bottom);
    for i in 0..18u32 {
        let speed = 0.004 + 0.007 * unit(i);
        let x = (unit(i * 5 + 1) + t * speed).rem_euclid(1.0);
        let y = unit(i * 5 + 2) * band;
        let radii =
            vec2(w * (0.12 + 0.18 * unit(i * 5 + 3)), h * band * (0.12 + 0.2 * unit(i * 5 + 4)));
        let rgb = if i % 3 == 0 { [64, 52, 90] } else { [26, 22, 44] };
        let a = if i % 3 == 0 { 0.4 } else { 0.6 };
        soft_ellipse(painter, pos2(r.left() + x * w, r.top() + y * h), radii, rgb, a);
        if x * w + radii.x > w {
            soft_ellipse(painter, pos2(r.left() + (x - 1.0) * w, r.top() + y * h), radii, rgb, a);
        }
    }
    // lightning
    let phase = t.rem_euclid(13.0);
    let flash = if phase < 0.4 {
        1.0 - phase / 0.4
    } else if (0.55..0.7).contains(&phase) {
        0.5
    } else {
        0.0
    };
    if flash > 0.0 {
        painter.rect_filled(r, 0.0, alpha([180, 200, 255], 0.14 * flash));
        soft_ellipse(
            painter,
            pos2(r.left() + w * (0.3 + 0.4 * unit((t / 13.0) as u32)), r.top() + h * band * 0.5),
            vec2(w * 0.4, h * band),
            [210, 220, 255],
            0.3 * flash,
        );
    }
}

/// The point at a fraction of the way along a line of points.
fn point_along(pts: &[Pos2], f: f32) -> Pos2 {
    let total: f32 = pts.windows(2).map(|p| (p[1] - p[0]).length()).sum();
    let mut left = total * f.clamp(0.0, 1.0);
    for p in pts.windows(2) {
        let len = (p[1] - p[0]).length();
        if left <= len || len <= 0.0 {
            let u = if len > 0.0 { left / len } else { 0.0 };
            return p[0].lerp(p[1], u);
        }
        left -= len;
    }
    pts.last().copied().unwrap_or(Pos2::ZERO)
}

/// A line of circuit board; once the walls have power, sparks of light run up it.
fn paint_trace(
    painter: &egui::Painter,
    pts: &[Pos2],
    rgb: [u8; 3],
    powered: bool,
    t: f32,
    seed: u32,
    width: f32,
) {
    if pts.len() < 2 {
        return;
    }
    let a = if powered { 0.6 } else { 0.17 };
    painter.add(Shape::line(pts.to_vec(), Stroke::new(width, alpha(rgb, a))));
    if powered {
        for k in 0..2u32 {
            let f = (t * 0.15 + unit(seed.wrapping_mul(7).wrapping_add(k)) + k as f32 * 0.5)
                .rem_euclid(1.0);
            let c = point_along(pts, f);
            soft_ellipse(painter, c, vec2(width * 7.0, width * 7.0), rgb, 0.4);
            painter.circle_filled(c, width * 1.5, alpha([255, 255, 255], 0.9));
        }
    }
}

/// A trace on a flat wall: up from `y0` to `y1` at `x`, jogging sideways on 45-degree bends.
fn trace_path(r: Rect, x: f32, y0: f32, y1: f32, seed: u32) -> Vec<Pos2> {
    let mut pts = vec![pos2(x, y0)];
    let (mut cx, mut cy) = (x, y0);
    let mut k = 0u32;
    while cy > y1 && k < 40 {
        cy -= r.height() * (0.05 + 0.12 * unit(seed.wrapping_mul(31).wrapping_add(k)));
        pts.push(pos2(cx, cy.max(y1)));
        if unit(seed.wrapping_mul(17).wrapping_add(k + 99)) > 0.45 && cy > y1 {
            let sign =
                if unit(seed.wrapping_mul(19).wrapping_add(k + 7)) > 0.5 { 1.0 } else { -1.0 };
            let d =
                r.width() * (0.012 + 0.02 * unit(seed.wrapping_mul(13).wrapping_add(k + 5))) * sign;
            cx = (cx + d).clamp(r.left() + 6.0, r.right() - 6.0);
            cy -= d.abs();
            pts.push(pos2(cx, cy.max(y1)));
        }
        k += 1;
    }
    pts
}

/// A chip on the wall: a dark package with pins and a glint.
fn paint_chip(painter: &egui::Painter, c: Rect, rgb: [u8; 3], seed: u32) {
    painter.rect_filled(c, 2.0, Color32::from_rgb(14, 18, 26));
    painter.rect_stroke(c, 2.0, Stroke::new(1.0, alpha(rgb, 0.4)), egui::StrokeKind::Inside);
    let pins = 5 + (seed % 4) as usize;
    for i in 0..pins {
        let f = (i as f32 + 0.5) / pins as f32;
        let x = c.left() + c.width() * f;
        painter.line_segment(
            [pos2(x, c.top()), pos2(x, c.top() - c.height() * 0.25)],
            Stroke::new(1.5, alpha(rgb, 0.35)),
        );
        painter.line_segment(
            [pos2(x, c.bottom()), pos2(x, c.bottom() + c.height() * 0.25)],
            Stroke::new(1.5, alpha(rgb, 0.35)),
        );
    }
    painter.circle_filled(
        c.left_top() + vec2(c.width() * 0.15, c.height() * 0.2),
        1.8,
        alpha(rgb, 0.7),
    );
}

/// A point on a face of the canyon wall: `u` runs from near (0) to far (1), `v` from the foot (0) to the top (1).
fn face(c: &[Pos2; 4], u: f32, v: f32) -> Pos2 {
    let s = (1.0 - 1.0 / (1.0 + 4.0 * u)) / 0.8;
    let top = c[0].lerp(c[1], s);
    let bottom = c[3].lerp(c[2], s);
    bottom.lerp(top, v)
}

/// One side of the canyon: the dark face of a tower, with its windows and the circuit board running up it.
fn paint_face(
    painter: &egui::Painter,
    c: &[Pos2; 4],
    era: Era,
    powered: bool,
    t: f32,
    seed: u32,
    light: [u8; 3],
) {
    let base = base_rgb(era);
    painter.add(Shape::convex_polygon(
        c.to_vec(),
        Color32::from_rgb(base[0], base[1], base[2]),
        Stroke::NONE,
    ));
    // a lighter foot where the street's glow reaches the wall
    painter.add(Shape::convex_polygon(
        vec![face(c, 0.0, 0.0), face(c, 1.0, 0.0), face(c, 1.0, 0.18), face(c, 0.0, 0.18)],
        alpha(light, 0.05),
        Stroke::NONE,
    ));
    // windows
    let win = window_rgb(era);
    for ui_ in 0..16u32 {
        let (u0, u1) = (ui_ as f32 / 16.0, (ui_ + 1) as f32 / 16.0);
        let du = u1 - u0;
        for vi in 0..30u32 {
            let (v0, v1) = (vi as f32 / 30.0, (vi + 1) as f32 / 30.0);
            let dv = v1 - v0;
            let hash = seed.wrapping_mul(1000).wrapping_add(ui_ * 40 + vi);
            if unit(hash) < 0.15 {
                let (a, b) = (u0 + du * 0.15, u1 - du * 0.15);
                let (lo, hi) = (v0 + dv * 0.2, v1 - dv * 0.2);
                let flicker = 0.7 + 0.3 * (t * (0.4 + unit(hash + 5)) + unit(hash + 9) * 6.0).sin();
                painter.add(Shape::convex_polygon(
                    vec![face(c, a, lo), face(c, b, lo), face(c, b, hi), face(c, a, hi)],
                    alpha(win, (0.22 + 0.25 * unit(hash + 3)) * flicker),
                    Stroke::NONE,
                ));
            }
        }
    }
    // ledges and the circuit board
    for k in 1..6u32 {
        let v = k as f32 / 6.0;
        painter
            .line_segment([face(c, 0.0, v), face(c, 1.0, v)], Stroke::new(1.0, alpha(light, 0.1)));
    }
    for i in 0..10u32 {
        let mut u = (i as f32 + 0.5) / 10.0;
        let mut v = 0.0;
        let mut pts = vec![face(c, u, 0.0)];
        let mut k = 0u32;
        while v < 1.0 && k < 30 {
            v = (v + 0.07 + 0.12 * unit(seed.wrapping_mul(97).wrapping_add(i * 31 + k))).min(1.0);
            pts.push(face(c, u, v));
            if unit(seed.wrapping_mul(53).wrapping_add(i * 17 + k)) > 0.5 && v < 1.0 {
                let sign = if unit(seed.wrapping_add(i * 11 + k * 3)) > 0.5 { 1.0 } else { -1.0 };
                u = (u + sign * 0.035).clamp(0.03, 0.97);
                v = (v + 0.03).min(1.0);
                pts.push(face(c, u, v));
            }
            k += 1;
        }
        paint_trace(painter, &pts, light, powered, t, seed * 100 + i, 1.8);
    }
    for k in 0..5u32 {
        let u = 0.08 + 0.8 * unit(seed.wrapping_add(k * 7 + 1));
        let v = 0.12 + 0.7 * unit(seed.wrapping_add(k * 7 + 2));
        let (a, b) = (u, (u + 0.05).min(1.0));
        let q = [face(c, a, v), face(c, b, v), face(c, b, v + 0.05), face(c, a, v + 0.05)];
        painter.add(Shape::convex_polygon(
            q.to_vec(),
            Color32::from_rgb(14, 18, 26),
            Stroke::new(1.0, alpha(light, 0.4)),
        ));
    }
}

/// Along the street: two towers close in on the way ahead, the sky between them, the city's glow at the
/// far end, and the cat.
fn paint_street(
    painter: &egui::Painter,
    r: Rect,
    state: &State,
    era: Era,
    t: f32,
    light: [u8; 3],
    powered: bool,
) {
    let (w, h) = (r.width(), r.height());
    let p = |x: f32, y: f32| pos2(r.left() + x * w, r.top() + y * h);
    paint_sky(painter, r, t, era, 0.45);
    // far off: the glow of the city and its towers
    soft_ellipse(painter, p(0.5, 0.56), vec2(w * 0.3, h * 0.2), light, 0.22);
    soft_ellipse(painter, p(0.5, 0.6), vec2(w * 0.16, h * 0.08), [255, 190, 110], 0.18);
    for i in 0..14u32 {
        let x = 0.40 + 0.2 * (i as f32 / 14.0);
        let top = 0.30 + 0.28 * unit(i * 3 + 1);
        painter.rect_filled(
            Rect::from_min_max(p(x, top), p(x + 0.016, 0.66)),
            0.0,
            Color32::from_rgb(4, 6, 12),
        );
        if unit(i * 3 + 2) > 0.5 {
            painter.circle_filled(p(x + 0.008, top + 0.02), 1.6, alpha(window_rgb(era), 0.7));
        }
    }
    // the ground
    painter.add(Shape::convex_polygon(
        vec![p(0.0, 0.94), p(0.40, 0.64), p(0.60, 0.64), p(1.0, 0.94), p(1.0, 1.0), p(0.0, 1.0)],
        Color32::from_rgb(8, 10, 16),
        Stroke::NONE,
    ));
    for k in 0..6u32 {
        let f = k as f32 / 6.0;
        let x = 0.5 + (f - 0.5) * 0.5;
        painter.line_segment(
            [p(x, 1.0), p(0.5 + (f - 0.5) * 0.06, 0.645)],
            Stroke::new(1.0, alpha(light, 0.07)),
        );
    }
    for k in 0..7u32 {
        let f = (k as f32 + (t * 0.05).fract()) / 7.0;
        let y = mix(0.645, 1.0, f * f);
        painter.line_segment(
            [p(0.5 - 0.004 - 0.03 * f, y), p(0.5 + 0.004 + 0.03 * f, y + 0.02 * f + 0.002)],
            Stroke::new(1.0 + 3.0 * f, alpha([200, 190, 150], 0.1 + 0.15 * f)),
        );
    }
    // the two faces of the canyon
    let left = [p(0.0, -0.30), p(0.40, 0.16), p(0.40, 0.64), p(0.0, 0.94)];
    let right = [p(1.0, -0.30), p(0.60, 0.16), p(0.60, 0.64), p(1.0, 0.94)];
    paint_face(painter, &left, era, powered, t, 1, light);
    paint_face(painter, &right, era, powered, t, 2, light);
    // wet street: the light of the walls on the ground, and puddles
    soft_ellipse(painter, p(0.5, 0.84), vec2(w * 0.22, h * 0.07), light, 0.07);
    for k in 0..4u32 {
        painter.add(Shape::convex_polygon(
            vec![
                p(0.18 + 0.15 * k as f32, 0.93),
                p(0.25 + 0.15 * k as f32, 0.90),
                p(0.30 + 0.15 * k as f32, 0.93),
                p(0.22 + 0.15 * k as f32, 0.96),
            ],
            alpha(light, 0.05),
            Stroke::NONE,
        ));
    }
    let _ = state;
}

/// A wall seen straight on: the dark face of a tower with the circuit board up it, and a strip of sky
/// and the tops of the towers opposite above.
fn paint_facade(
    painter: &egui::Painter,
    r: Rect,
    view: usize,
    era: Era,
    t: f32,
    light: [u8; 3],
    powered: bool,
) {
    let (w, h) = (r.width(), r.height());
    let p = |x: f32, y: f32| pos2(r.left() + x * w, r.top() + y * h);
    let seed = view as u32 * 50;
    paint_sky(painter, sub(r, 0.0, 0.0, 1.0, 0.18), t, era, 0.9);
    // the tops of the towers across the way
    let mut x = 0.0;
    let mut i = 0u32;
    while x < 1.0 {
        let wd = 0.05 + 0.08 * unit(i * 3 + seed);
        let top = 0.03 + 0.1 * unit(i * 3 + 1 + seed);
        painter.rect_filled(
            Rect::from_min_max(p(x, top), p(x + wd, 0.2)),
            0.0,
            Color32::from_rgb(4, 6, 12),
        );
        if unit(i * 3 + 2 + seed) > 0.55 {
            let ax = x + wd * 0.5;
            painter.line_segment(
                [p(ax, top), p(ax, top - 0.04)],
                Stroke::new(1.5, Color32::from_rgb(4, 6, 12)),
            );
            if (t * 0.8 + i as f32).sin() > 0.2 {
                painter.circle_filled(p(ax, top - 0.04), 2.4, alpha([255, 60, 60], 0.9));
            }
        }
        x += wd;
        i += 1;
    }
    // the face of the tower
    let wall = sub(r, 0.0, 0.17, 1.0, 0.86);
    let base = base_rgb(era);
    gradient(
        painter,
        wall,
        Color32::from_rgb(base[0], base[1], base[2]),
        Color32::from_rgb(
            base[0].saturating_add(8),
            base[1].saturating_add(10),
            base[2].saturating_add(14),
        ),
    );
    let win = window_rgb(era);
    for row in 0..16u32 {
        for col in 0..38u32 {
            let hash = seed.wrapping_mul(1000).wrapping_add(row * 64 + col);
            if unit(hash) < 0.12 {
                let c = Rect::from_min_size(
                    pos2(
                        wall.left() + wall.width() * (col as f32 / 38.0 + 0.004),
                        wall.top() + wall.height() * (row as f32 / 16.0 + 0.012),
                    ),
                    vec2(wall.width() / 38.0 * 0.6, wall.height() / 16.0 * 0.5),
                );
                let flicker = 0.7 + 0.3 * (t * (0.4 + unit(hash + 5)) + unit(hash + 9) * 6.0).sin();
                painter.rect_filled(c, 1.0, alpha(win, (0.2 + 0.25 * unit(hash + 3)) * flicker));
            }
        }
    }
    // the circuit board up the wall
    for k in 1..5u32 {
        let y = wall.top() + wall.height() * k as f32 / 5.0;
        painter.line_segment(
            [pos2(wall.left(), y), pos2(wall.right(), y)],
            Stroke::new(1.0, alpha(light, 0.08)),
        );
    }
    for i in 0..20u32 {
        let x = wall.left() + wall.width() * (i as f32 + 0.3 + 0.4 * unit(i + seed)) / 20.0;
        let pts = trace_path(wall, x, wall.bottom(), wall.top(), seed + i);
        paint_trace(painter, &pts, light, powered, t, seed + i + 1000, 2.0);
    }
    for k in 0..8u32 {
        let cx = wall.left() + wall.width() * (0.05 + 0.9 * unit(seed + k * 5 + 1));
        let cy = wall.top() + wall.height() * (0.1 + 0.8 * unit(seed + k * 5 + 2));
        paint_chip(
            painter,
            Rect::from_center_size(pos2(cx, cy), vec2(wall.width() * 0.035, wall.height() * 0.045)),
            light,
            seed + k,
        );
    }
    painter.line_segment(
        [pos2(wall.left(), wall.top()), pos2(wall.right(), wall.top())],
        Stroke::new(3.0, alpha(light, 0.25)),
    );
    // the pavement
    let ground = sub(r, 0.0, 0.86, 1.0, 1.0);
    gradient(painter, ground, Color32::from_rgb(10, 12, 18), Color32::from_rgb(4, 5, 9));
    painter.line_segment(
        [pos2(ground.left(), ground.top() + 2.0), pos2(ground.right(), ground.top() + 2.0)],
        Stroke::new(3.0, alpha(light, 0.18)),
    );
    for k in 0..5u32 {
        soft_ellipse(
            painter,
            pos2(ground.left() + ground.width() * (0.12 + 0.2 * k as f32), ground.center().y),
            vec2(ground.width() * 0.08, ground.height() * 0.25),
            light,
            0.06,
        );
    }
}

/// Rain, the dark closing in at the edges.
fn paint_weather(painter: &egui::Painter, r: Rect, t: f32, era: Era, light: [u8; 3]) {
    let (w, h) = (r.width(), r.height());
    let strength = if era == Era::Past { 0.06 } else { 0.14 };
    for i in 0..70u32 {
        let x = unit(i * 3);
        let y = (unit(i * 3 + 1) + t * (0.9 + 0.5 * unit(i * 3 + 2))).rem_euclid(1.0);
        let p = pos2(r.left() + x * w, r.top() + y * h);
        painter.line_segment(
            [p, p + vec2(-3.0, 12.0)],
            Stroke::new(1.0, alpha([190, 210, 255], strength)),
        );
    }
    painter.rect_filled(r, 0.0, alpha(light, 0.03));
    for i in 0..10 {
        let inset = w * 0.012 * i as f32;
        painter.rect_stroke(
            r.shrink(inset),
            0.0,
            Stroke::new(w * 0.014, Color32::from_rgba_unmultiplied(0, 0, 0, (60 - i * 5) as u8)),
            egui::StrokeKind::Inside,
        );
    }
}

/// A repeatable random number from 0 to 1.
fn unit(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x % 10_000) as f32 / 10_000.0
}

fn mix(a: f32, b: f32, f: f32) -> f32 {
    a + (b - a) * f
}

// ------------------------------------------------------------------------- the things on the walls
fn paint_objects(
    painter: &egui::Painter,
    room: Rect,
    view: usize,
    state: &State,
    era: Era,
    t: f32,
    light: [u8; 3],
    shield_age: Option<f32>,
) {
    let o = |id: SpotId| object_rect(room, id);
    match view {
        0 => {
            paint_neon(painter, o(SpotId::Neon), t);
            paint_vending(painter, o(SpotId::Vending), state, t);
            paint_cat(painter, o(SpotId::Cat), t);
        }
        1 => {
            paint_dumpster(painter, o(SpotId::Dumpster), state);
            paint_notice(painter, o(SpotId::Notice), state);
        }
        2 => {
            paint_passage(painter, o(SpotId::Passage), era, state, t, shield_age);
            paint_clock_sign(painter, o(SpotId::ClockSign), state, t, light);
            paint_hatch(painter, o(SpotId::Hatch), state, t);
        }
        _ => {
            paint_locker(painter, o(SpotId::Locker), state);
            paint_cabinet(painter, o(SpotId::Cabinet), state, t);
        }
    }
}

/// A neon sign, flickering.
fn paint_neon(painter: &egui::Painter, o: Rect, t: f32) {
    painter.rect_filled(o, 6.0, Color32::from_rgb(10, 10, 16));
    painter.rect_stroke(
        o,
        6.0,
        Stroke::new(3.0, Color32::from_rgb(44, 48, 62)),
        egui::StrokeKind::Inside,
    );
    let on = if (t * 0.9).fract() > 0.93 { 0.25 } else { 1.0 };
    let a = on * (0.8 + 0.2 * (t * 31.0).sin().abs());
    let c = o.center() - vec2(0.0, o.height() * 0.08);
    let font = egui::FontId::proportional(o.height() * 0.4);
    for (dx, dy) in [(-2.0, 0.0), (2.0, 0.0), (0.0, -2.0), (0.0, 2.0)] {
        painter.text(
            c + vec2(dx, dy),
            egui::Align2::CENTER_CENTER,
            "VOLT-COLA",
            font.clone(),
            alpha([255, 60, 120], 0.2 * a),
        );
    }
    painter.text(c, egui::Align2::CENTER_CENTER, "VOLT-COLA", font, alpha([255, 170, 200], a));
    soft_ellipse(
        painter,
        o.center(),
        vec2(o.width() * 0.7, o.height() * 0.9),
        [255, 40, 110],
        0.16 * a,
    );
    let y = o.bottom() - o.height() * 0.18;
    painter.line_segment(
        [pos2(o.left() + o.width() * 0.1, y), pos2(o.right() - o.width() * 0.1, y)],
        Stroke::new(3.0, alpha([90, 200, 255], 0.85 * a)),
    );
}

fn paint_vending(painter: &egui::Painter, o: Rect, state: &State, t: f32) {
    painter.rect_filled(o, 6.0, Color32::from_rgb(24, 52, 64));
    painter.rect_stroke(
        o,
        6.0,
        Stroke::new(3.0, Color32::from_rgb(10, 24, 30)),
        egui::StrokeKind::Inside,
    );
    painter.rect_filled(sub(o, 0.0, 0.0, 1.0, 0.07), 4.0, Color32::from_rgb(120, 30, 50));
    let glass = sub(o, 0.1, 0.1, 0.9, 0.58);
    painter.rect_filled(glass, 3.0, Color32::from_rgb(8, 16, 22));
    for row in 0..3u32 {
        for col in 0..4u32 {
            let c = Rect::from_min_size(
                pos2(
                    glass.left() + glass.width() * (0.06 + 0.23 * col as f32),
                    glass.top() + glass.height() * (0.08 + 0.3 * row as f32),
                ),
                vec2(glass.width() * 0.17, glass.height() * 0.22),
            );
            let hue = [[200, 50, 70], [70, 150, 200], [220, 170, 60], [90, 180, 110]]
                [((row + col) % 4) as usize];
            painter.rect_filled(c, 3.0, alpha(hue, 0.35));
        }
    }
    painter.rect_filled(sub(o, 0.72, 0.64, 0.9, 0.7), 2.0, Color32::from_rgb(6, 8, 12));
    painter.text(
        sub(o, 0.7, 0.72, 0.92, 0.8).center(),
        egui::Align2::CENTER_CENTER,
        "TOKEN",
        egui::FontId::proportional(o.width() * 0.07),
        alpha([170, 180, 190], 0.6),
    );
    // the service door at the bottom
    let door = sub(o, 0.1, 0.66, 0.66, 0.95);
    if state.flag(alley::F_VENDING) {
        painter.rect_filled(door, 3.0, Color32::from_rgb(3, 4, 8));
        painter.add(Shape::convex_polygon(
            vec![
                door.left_top(),
                door.left_top() + vec2(-door.width() * 0.25, door.height() * 0.08),
                door.left_bottom() + vec2(-door.width() * 0.25, -door.height() * 0.02),
                door.left_bottom(),
            ],
            Color32::from_rgb(44, 50, 60),
            Stroke::new(1.0, Color32::from_rgb(90, 100, 112)),
        ));
    } else {
        painter.rect_filled(door, 3.0, Color32::from_rgb(40, 46, 56));
        painter.rect_stroke(
            door,
            3.0,
            Stroke::new(2.0, Color32::from_rgb(90, 100, 112)),
            egui::StrokeKind::Inside,
        );
        for k in 0..4u32 {
            painter.circle_filled(
                pos2(door.left() + door.width() * (0.2 + 0.2 * k as f32), door.center().y),
                o.width() * 0.03,
                Color32::from_rgb(20, 24, 30),
            );
        }
        painter.text(
            sub(door, 0.0, 0.0, 1.0, 0.3).center(),
            egui::Align2::CENTER_CENTER,
            "SERVICE",
            egui::FontId::proportional(o.width() * 0.06),
            alpha([170, 180, 190], 0.7),
        );
    }
    let _ = t;
}

fn paint_cat(painter: &egui::Painter, o: Rect, t: f32) {
    let ink = Color32::from_rgb(1, 1, 3);
    let c = o.center_bottom();
    let (ow, oh) = (o.width(), o.height());
    painter.add(Shape::convex_polygon(
        vec![
            c + vec2(-ow * 0.42, 0.0),
            c + vec2(-ow * 0.17, -oh * 0.55),
            c + vec2(ow * 0.17, -oh * 0.55),
            c + vec2(ow * 0.42, 0.0),
        ],
        ink,
        Stroke::NONE,
    ));
    let head = c + vec2(0.0, -oh * 0.7);
    painter.circle_filled(head, ow * 0.26, ink);
    for sx in [-1.0f32, 1.0] {
        painter.add(Shape::convex_polygon(
            vec![
                head + vec2(sx * ow * 0.05, -ow * 0.2),
                head + vec2(sx * ow * 0.25, -ow * 0.12),
                head + vec2(sx * ow * 0.2, -ow * 0.42),
            ],
            ink,
            Stroke::NONE,
        ));
    }
    // a cane at his side
    painter.line_segment(
        [c + vec2(ow * 0.5, 0.0), c + vec2(ow * 0.5, -oh * 0.5)],
        Stroke::new(1.5, ink),
    );
    if (t * 0.2).fract() > 0.03 {
        for sx in [-1.0f32, 1.0] {
            let e = head + vec2(sx * ow * 0.1, 0.0);
            soft_ellipse(painter, e, vec2(ow * 0.22, ow * 0.22), [255, 190, 70], 0.35);
            painter.circle_filled(e, (ow * 0.04).max(1.5), Color32::from_rgb(255, 200, 80));
        }
    }
}

fn paint_dumpster(painter: &egui::Painter, o: Rect, state: &State) {
    let (w, h) = (o.width(), o.height());
    // the things hanging out of it
    painter.line_segment(
        [o.left_top() + vec2(w * 0.7, h * 0.28), o.left_top() + vec2(w * 0.9, -h * 0.1)],
        Stroke::new(3.0, Color32::from_rgb(40, 40, 50)),
    );
    painter.line_segment(
        [o.left_top() + vec2(w * 0.9, -h * 0.1), o.left_top() + vec2(w * 0.74, -h * 0.02)],
        Stroke::new(2.0, Color32::from_rgb(70, 40, 40)),
    );
    painter.add(Shape::convex_polygon(
        vec![
            o.left_top() + vec2(w * 0.15, h * 0.3),
            o.left_top() + vec2(w * 0.45, h * 0.12),
            o.left_top() + vec2(w * 0.55, h * 0.3),
        ],
        Color32::from_rgb(120, 96, 70),
        Stroke::NONE,
    ));
    // the body
    painter.add(Shape::convex_polygon(
        vec![
            o.left_top() + vec2(w * 0.03, h * 0.28),
            o.right_top() + vec2(-w * 0.03, h * 0.28),
            o.right_bottom(),
            o.left_bottom(),
        ],
        Color32::from_rgb(26, 50, 40),
        Stroke::new(2.0, Color32::from_rgb(10, 22, 16)),
    ));
    for k in 1..6u32 {
        let x = o.left() + w * k as f32 / 6.0;
        painter.line_segment(
            [pos2(x, o.top() + h * 0.32), pos2(x, o.bottom())],
            Stroke::new(2.0, Color32::from_rgb(18, 36, 28)),
        );
    }
    painter.rect_filled(sub(o, 0.0, 0.22, 1.0, 0.3), 3.0, Color32::from_rgb(36, 70, 56));
    painter.text(
        sub(o, 0.0, 0.5, 1.0, 0.75).center(),
        egui::Align2::CENTER_CENTER,
        "NO DUMPING",
        egui::FontId::proportional(h * 0.12),
        alpha([200, 210, 190], 0.5),
    );
    // wheels
    for x in [0.16, 0.84] {
        painter.circle_filled(
            pos2(o.left() + w * x, o.bottom()),
            h * 0.05,
            Color32::from_rgb(8, 8, 10),
        );
    }
    let _ = state;
}

fn paint_notice(painter: &egui::Painter, o: Rect, state: &State) {
    painter.rect_filled(o, 4.0, Color32::from_rgb(70, 52, 34));
    painter.rect_stroke(
        o,
        4.0,
        Stroke::new(4.0, Color32::from_rgb(40, 30, 22)),
        egui::StrokeKind::Inside,
    );
    let paper = sub(o, 0.07, 0.1, 0.5, 0.9);
    painter.rect_filled(paper, 1.0, Color32::from_rgb(226, 218, 196));
    for k in 0..9u32 {
        let y = paper.top() + paper.height() * (0.14 + 0.09 * k as f32);
        painter.line_segment(
            [
                pos2(paper.left() + paper.width() * 0.1, y),
                pos2(paper.right() - paper.width() * (0.1 + 0.3 * unit(k)), y),
            ],
            Stroke::new(1.5, Color32::from_rgb(110, 104, 92)),
        );
    }
    let cal = sub(o, 0.56, 0.14, 0.94, 0.86);
    painter.rect_filled(cal, 1.0, Color32::from_rgb(234, 232, 224));
    painter.rect_filled(sub(cal, 0.0, 0.0, 1.0, 0.16), 1.0, Color32::from_rgb(170, 50, 50));
    let code = alley::locker_code(state.seed());
    for d in 0..30u32 {
        let idx = d + 3;
        let (row, col) = (idx / 7, idx % 7);
        let c = pos2(
            cal.left() + cal.width() * (0.07 + 0.14 * col as f32),
            cal.top() + cal.height() * (0.3 + 0.13 * row as f32),
        );
        painter.circle_filled(c, 1.6, Color32::from_rgb(90, 90, 100));
        if code.contains(&((d + 1) as u8)) {
            painter.circle_stroke(
                c,
                cal.width() * 0.065,
                Stroke::new(1.8, Color32::from_rgb(200, 40, 40)),
            );
        }
    }
    for (x, y) in [(0.29, 0.07), (0.75, 0.09)] {
        painter.circle_filled(
            pos2(o.left() + o.width() * x, o.top() + o.height() * y),
            3.5,
            Color32::from_rgb(190, 50, 50),
        );
    }
}

/// Points round an ellipse.
fn ellipse_points(c: Pos2, rx: f32, ry: f32, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / n as f32;
            pos2(c.x + rx * a.cos(), c.y + ry * a.sin())
        })
        .collect()
}

/// The clock of the tower, set into the wall: a round dark face in a heavy ring, with the wall's circuit
/// board running into the ring from every side. Dead until the walls have power; then it flickers; once
/// its circuit is solved it burns.
fn paint_clock_sign(painter: &egui::Painter, o: Rect, state: &State, t: f32, light: [u8; 3]) {
    let powered = state.flag(alley::F_POWER);
    let lit = state.is_lit(0);
    let c = o.center();
    let r = o.height().min(o.width()) * 0.5;
    let glow = if lit {
        1.0
    } else if powered {
        0.22 + 0.2 * (t * 9.0).sin().abs() * (t * 2.3).sin().abs()
    } else {
        0.0
    };
    let rgb = if lit { [255, 206, 120] } else { light };
    // the wall dips into a round recess, and the feeds run into the ring
    painter.circle_filled(c, r * 1.12, Color32::from_rgb(5, 7, 12));
    painter.circle_stroke(c, r * 1.12, Stroke::new(2.0, alpha(light, 0.2)));
    for k in 0..12u32 {
        let a = k as f32 * std::f32::consts::TAU / 12.0 + 0.13;
        let (s_, c_) = a.sin_cos();
        let dir = vec2(c_, s_);
        let far = c + dir * r * (1.9 + 0.5 * unit(k + 40));
        let mid = c + dir * r * 1.45;
        let bend = mid + vec2(-dir.y, dir.x) * r * 0.12 * (if k % 2 == 0 { 1.0 } else { -1.0 });
        let rim = c + dir * r * 1.12;
        paint_trace(painter, &[far, bend, mid, rim], light, powered, t, 700 + k, 2.2);
    }
    // the ring and the face
    painter.circle_filled(c, r, Color32::from_rgb(30, 34, 46));
    painter.circle_stroke(c, r, Stroke::new(3.0, Color32::from_rgb(70, 78, 98)));
    painter.circle_filled(c, r * 0.86, Color32::from_rgb(8, 10, 16));
    painter.circle_stroke(c, r * 0.86, Stroke::new(2.0, alpha(rgb, 0.25 + 0.7 * glow)));
    if glow > 0.0 {
        soft_ellipse(painter, c, vec2(r * 1.3, r * 1.3), rgb, 0.3 * glow);
    }
    for k in 0..12u32 {
        let a = k as f32 * std::f32::consts::TAU / 12.0;
        let (s_, c_) = a.sin_cos();
        let dir = vec2(s_, -c_);
        painter.line_segment(
            [c + dir * r * 0.68, c + dir * r * 0.8],
            Stroke::new(if k % 3 == 0 { 4.0 } else { 2.0 }, alpha(rgb, 0.3 + 0.7 * glow)),
        );
    }
    // four bolts on the ring
    for k in 0..4u32 {
        let a = k as f32 * std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4;
        painter.circle_filled(
            c + vec2(a.cos(), a.sin()) * r * 0.93,
            r * 0.03,
            Color32::from_rgb(110, 118, 136),
        );
    }
    painter.circle_filled(c, r * 0.06, alpha(rgb, 0.4 + 0.6 * glow));
}

/// A hatch in the road, flat on the ground (so an ellipse): a heavy iron cover worked like a piece of old
/// craftsmanship, with a gilded rim of beads, a ring of vine and leaf, and three ornate keyholes. Each is cut for
/// the hand of a clock, and glows with the hand in it once you hold it.
fn paint_hatch(painter: &egui::Painter, o: Rect, state: &State, t: f32) {
    let c = o.center();
    let (rx, ry) = (o.width() * 0.5, o.height() * 0.5);
    let k = ry / rx;
    let usable = state.has_key() || (alley::FIRST_AREA_ONLY && state.has_hand(0));
    let gold = |a: f32| alpha([206, 160, 70], a);
    let ell = |f: f32| ellipse_points(c, rx * f, ry * f, 64);
    // the shadow, the iron and the bronze bands
    painter.add(Shape::convex_polygon(
        ellipse_points(c + vec2(0.0, ry * 0.14), rx * 1.05, ry * 1.05, 64),
        Color32::from_rgb(3, 3, 5),
        Stroke::NONE,
    ));
    painter.add(Shape::convex_polygon(
        ell(1.0),
        Color32::from_rgb(34, 28, 24),
        Stroke::new(3.0, Color32::from_rgb(120, 92, 44)),
    ));
    painter.add(Shape::convex_polygon(
        ell(0.93),
        Color32::from_rgb(46, 38, 30),
        Stroke::new(1.5, gold(0.7)),
    ));
    if usable {
        soft_ellipse(
            painter,
            c,
            vec2(rx * 1.1, ry * 1.6),
            [255, 200, 100],
            0.2 + 0.05 * (t * 3.0).sin(),
        );
    }
    // a rim of beads
    for i in 0..56u32 {
        let a = std::f32::consts::TAU * i as f32 / 56.0;
        painter.circle_filled(
            pos2(c.x + a.cos() * rx * 0.965, c.y + a.sin() * ry * 0.965),
            (ry * 0.045).max(1.5),
            gold(0.8),
        );
    }
    // the ring of vine and leaf
    let vine: Vec<Pos2> = (0..=192u32)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / 192.0;
            let f = 0.82 + 0.045 * (a * 12.0).sin();
            pos2(c.x + rx * f * a.cos(), c.y + ry * f * a.sin())
        })
        .collect();
    painter.add(Shape::line(vine, Stroke::new(2.2, gold(0.9))));
    for i in 0..12u32 {
        let a = std::f32::consts::TAU * (i as f32 + 0.25) / 12.0;
        let f = 0.82 + 0.045;
        let leaf = pos2(c.x + rx * f * a.cos(), c.y + ry * f * a.sin());
        painter.add(Shape::convex_polygon(
            ellipse_points(leaf, rx * 0.032, ry * 0.05, 10),
            gold(0.85),
            Stroke::NONE,
        ));
        let petal = pos2(c.x + rx * 0.78 * a.cos(), c.y + ry * 0.78 * a.sin());
        painter.circle_filled(petal, (ry * 0.03).max(1.2), gold(0.6));
    }
    painter.add(Shape::line(ell(0.72), Stroke::new(1.5, gold(0.7))));
    painter.add(Shape::line(ell(0.70), Stroke::new(1.0, gold(0.4))));
    // the field: a dark plate with a fine engraved star
    painter.add(Shape::convex_polygon(ell(0.69), Color32::from_rgb(26, 21, 18), Stroke::NONE));
    for i in 0..32u32 {
        let a = std::f32::consts::TAU * i as f32 / 32.0;
        let (s_, c_) = a.sin_cos();
        let long = if i % 2 == 0 { 0.68 } else { 0.52 };
        painter.line_segment(
            [
                pos2(c.x + c_ * rx * 0.34, c.y + s_ * ry * 0.34),
                pos2(c.x + c_ * rx * long, c.y + s_ * ry * long),
            ],
            Stroke::new(1.0, gold(0.12)),
        );
    }
    // the three escutcheons, each with its keyhole
    for i in 0..3usize {
        let have = state.has_hand(i);
        let cx = c.x + (i as f32 - 1.0) * rx * 0.43;
        let cy = c.y + ry * 0.02;
        let a = rx * 0.155;
        let at = |dx: f32, dy: f32| pos2(cx + dx * a, cy + dy * a * k);
        // the scalloped plate
        for j in 0..14u32 {
            let ang = std::f32::consts::TAU * j as f32 / 14.0;
            painter.circle_filled(
                at(ang.cos() * 1.0, ang.sin() * 1.0),
                (a * 0.17).max(1.5),
                gold(0.9),
            );
        }
        painter.add(Shape::convex_polygon(
            ellipse_points(pos2(cx, cy), a * 1.0, a * k * 1.0, 40),
            Color32::from_rgb(96, 72, 34),
            Stroke::new(1.5, gold(0.95)),
        ));
        painter.add(Shape::convex_polygon(
            ellipse_points(pos2(cx, cy), a * 0.76, a * k * 0.76, 40),
            Color32::from_rgb(22, 17, 14),
            Stroke::new(1.0, gold(0.7)),
        ));
        // curls either side
        for side in [-1.0f32, 1.0] {
            let curl: Vec<Pos2> = (0..=16u32)
                .map(|n| {
                    let u = n as f32 / 16.0;
                    let ang = u * std::f32::consts::PI * 1.6;
                    let rad = 0.2 * (1.0 - u * 0.7);
                    at(side * (0.45 + rad * ang.cos() * 0.8), -0.1 + rad * ang.sin() * 1.1)
                })
                .collect();
            painter.add(Shape::line(curl, Stroke::new(1.3, gold(0.8))));
        }
        // the keyhole: a round eye and a tapering slot
        let eye = at(0.0, -0.18);
        if have {
            soft_ellipse(painter, pos2(cx, cy), vec2(a * 1.5, a * k * 1.8), [255, 206, 90], 0.45);
        }
        let hole = if have { Color32::from_rgb(255, 214, 110) } else { Color32::from_rgb(2, 2, 4) };
        painter.add(Shape::convex_polygon(
            ellipse_points(eye, a * 0.2, a * k * 0.2, 20),
            hole,
            Stroke::new(1.5, gold(0.9)),
        ));
        painter.add(Shape::convex_polygon(
            vec![at(-0.07, -0.12), at(0.07, -0.12), at(0.16, 0.5), at(-0.16, 0.5)],
            hole,
            Stroke::new(1.0, gold(0.9)),
        ));
        // a ruby at the top
        let gem = at(0.0, -0.58);
        painter.circle_filled(gem, (a * 0.1).max(2.0), Color32::from_rgb(160, 28, 44));
        painter.circle_filled(
            gem + vec2(-1.0, -1.0),
            (a * 0.03).max(1.0),
            alpha([255, 200, 210], 0.8),
        );
        // the hand in the lock
        if have {
            let len = a * [0.8, 1.0, 1.25][i];
            painter.line_segment(
                [pos2(cx, cy), pos2(cx + len * 0.6, cy - len * 0.7 * k * 1.6)],
                Stroke::new([5.0, 3.6, 2.2][i], Color32::from_rgb(255, 222, 120)),
            );
            painter.circle_filled(
                pos2(cx + len * 0.6, cy - len * 0.7 * k * 1.6),
                [5.0, 4.0, 3.0][i],
                Color32::from_rgb(255, 222, 120),
            );
        }
    }
}

/// Numbers falling like rain inside the arch, as in the old films: the head of each column bright, its trail
/// fading, and the digits changing as they fall.
fn paint_rain(
    painter: &egui::Painter,
    o: Rect,
    inside: &dyn Fn(Pos2) -> bool,
    t: f32,
    strength: f32,
) {
    let cols = 9u32;
    let colw = o.width() / cols as f32;
    let fs = (colw * 0.9).max(9.0);
    let cell = fs * 1.15;
    let rows = (o.height() / cell).ceil() as i32;
    let trail = 11i32;
    let font = egui::FontId::monospace(fs);
    for col in 0..cols {
        let speed = 3.0 + 5.0 * unit(col * 7 + 1);
        let offset = unit(col * 13 + 2) * (rows + trail) as f32;
        let head = ((t * speed + offset) % (rows + trail) as f32).floor() as i32;
        for k in 0..trail {
            let row = head - k;
            if row < 0 || row >= rows {
                continue;
            }
            let p = pos2(o.left() + (col as f32 + 0.5) * colw, o.top() + (row as f32 + 0.5) * cell);
            if !inside(p) {
                continue;
            }
            let digit =
                (unit(col * 101 + row as u32 * 17 + (t * 3.0) as u32 * 5) * 10.0) as u32 % 10;
            let (rgb, a) = if k == 0 {
                ([210, 255, 240], 0.95)
            } else {
                ([70, 255, 190], 0.8 * (1.0 - k as f32 / trail as f32).powf(1.4))
            };
            painter.text(
                p,
                egui::Align2::CENTER_CENTER,
                digit.to_string(),
                font.clone(),
                alpha(rgb, a * strength),
            );
        }
    }
}

/// An old arched doorway in the tower wall. Dead and dark until the walls have power; then a shield of falling
/// numbers fills it and numbers run round its frame; when the code is broken the shield blinks like a failing
/// field and goes out, leaving the arch open on the back lot. In the past it was never shielded: you can see through.
fn paint_passage(
    painter: &egui::Painter,
    o: Rect,
    era: Era,
    state: &State,
    t: f32,
    shield_age: Option<f32>,
) {
    let painter = &painter.with_clip_rect(o.expand(60.0));
    let w = o.width();
    let cx = o.center().x;
    let spring = o.top() + w * 0.5;
    let arch = |grow: f32| -> Vec<Pos2> {
        let mut pts = vec![pos2(o.left() - grow, o.bottom())];
        pts.push(pos2(o.left() - grow, spring));
        for i in 1..24 {
            let a = std::f32::consts::PI + std::f32::consts::PI * i as f32 / 24.0;
            pts.push(pos2(cx + (w * 0.5 + grow) * a.cos(), spring + (w * 0.5 + grow) * a.sin()));
        }
        pts.push(pos2(o.right() + grow, spring));
        pts.push(pos2(o.right() + grow, o.bottom()));
        pts
    };
    let inside = |p: Pos2| -> bool {
        if p.y >= spring {
            p.x >= o.left() && p.x <= o.right() && p.y <= o.bottom()
        } else {
            (p.x - cx).powi(2) + (p.y - spring).powi(2) <= (w * 0.5).powi(2)
        }
    };
    let powered = state.flag(alley::F_POWER);
    let down = state.flag(alley::F_SHIELD);
    let age = if down { shield_age.unwrap_or(99.0) } else { -1.0 };
    let teal = [70, 255, 190];
    // the stone frame
    painter.add(Shape::convex_polygon(
        arch(w * 0.07),
        Color32::from_rgb(26, 32, 44),
        Stroke::new(2.0, Color32::from_rgb(54, 62, 80)),
    ));
    let open = era == Era::Past || (down && age >= 2.0);
    painter.add(Shape::convex_polygon(arch(0.0), Color32::from_rgb(4, 8, 12), Stroke::NONE));
    if open {
        let (glow, tint): ([u8; 3], f32) =
            if era == Era::Past { ([255, 170, 90], 0.4) } else { ([120, 210, 255], 0.3) };
        soft_ellipse(
            painter,
            pos2(cx, o.center().y + o.height() * 0.1),
            vec2(w * 0.7, o.height() * 0.36),
            glow,
            tint,
        );
        // a lamp post and the lot beyond
        painter.line_segment(
            [pos2(cx + w * 0.1, o.bottom()), pos2(cx + w * 0.1, o.top() + o.height() * 0.42)],
            Stroke::new(3.0, Color32::from_rgb(8, 6, 6)),
        );
        painter.circle_filled(
            pos2(cx + w * 0.1, o.top() + o.height() * 0.4),
            w * 0.05,
            Color32::from_rgb(255, 214, 140),
        );
        painter.rect_filled(
            Rect::from_min_max(
                pos2(o.left(), o.bottom() - o.height() * 0.07),
                pos2(o.right(), o.bottom()),
            ),
            0.0,
            Color32::from_rgb(40, 26, 18),
        );
        painter.add(Shape::line(
            {
                let mut pts = arch(0.0);
                pts.push(pts[0]);
                pts
            },
            Stroke::new(2.0, alpha(teal, if era == Era::Past { 0.0 } else { 0.25 })),
        ));
        return;
    }
    // the shield: blinking and fading while it fails
    let strength = if down {
        let fade = 1.0 - age / 2.0;
        let blink = if ((age * 11.0) as i32) % 2 == 0 { 1.0 } else { 0.2 };
        fade * blink
    } else if powered {
        1.0
    } else {
        0.0
    };
    if powered {
        soft_ellipse(painter, o.center(), vec2(w * 0.8, o.height() * 0.5), teal, 0.1 * strength);
        paint_rain(painter, o, &inside, t, strength);
    } else {
        // dead static: a few dim digits flicker
        for k in 0..6u32 {
            let p = pos2(
                o.left() + w * (0.15 + 0.7 * unit(k + 3)),
                o.top() + o.height() * (0.35 + 0.55 * unit(k + 9)),
            );
            if inside(p) && (t * 2.0 + k as f32).sin() > 0.4 {
                painter.text(
                    p,
                    egui::Align2::CENTER_CENTER,
                    ((unit(k + (t * 2.0) as u32) * 10.0) as u32 % 10).to_string(),
                    egui::FontId::monospace(w * 0.08),
                    alpha(teal, 0.15),
                );
            }
        }
    }
    // the frame is made of numbers too
    let pts = arch(w * 0.035);
    let glyph = egui::FontId::monospace((w * 0.075).max(9.0));
    let n = 46u32;
    for i in 0..n {
        let p = point_along(&pts, (i as f32 + 0.5) / n as f32);
        let digit = (unit(i * 31 + (t * 2.0) as u32 * 7) * 10.0) as u32 % 10;
        let a =
            if powered { (0.55 + 0.4 * (t * 3.0 + i as f32 * 0.7).sin()) * strength } else { 0.12 };
        painter.text(
            p,
            egui::Align2::CENTER_CENTER,
            digit.to_string(),
            glyph.clone(),
            alpha(teal, a.clamp(0.0, 1.0)),
        );
    }
}

fn paint_locker(painter: &egui::Painter, o: Rect, state: &State) {
    let (w, h) = (o.width(), o.height());
    painter.rect_filled(o, 4.0, Color32::from_rgb(8, 10, 12));
    let open = state.flag(alley::F_LOCKER);
    if open {
        painter.add(Shape::convex_polygon(
            vec![
                o.left_top(),
                o.left_top() + vec2(-w * 0.35, h * 0.05),
                o.left_bottom() + vec2(-w * 0.35, -h * 0.03),
                o.left_bottom(),
            ],
            Color32::from_rgb(62, 80, 74),
            Stroke::new(2.0, Color32::from_rgb(30, 40, 38)),
        ));
        painter.rect_filled(sub(o, 0.15, 0.7, 0.85, 0.95), 3.0, Color32::from_rgb(36, 28, 26));
    } else {
        painter.rect_filled(o, 4.0, Color32::from_rgb(62, 80, 74));
        painter.rect_stroke(
            o,
            4.0,
            Stroke::new(3.0, Color32::from_rgb(30, 40, 38)),
            egui::StrokeKind::Inside,
        );
        for k in 0..5u32 {
            let y = o.top() + h * (0.07 + 0.035 * k as f32);
            painter.line_segment(
                [pos2(o.left() + w * 0.2, y), pos2(o.right() - w * 0.2, y)],
                Stroke::new(3.0, Color32::from_rgb(20, 28, 26)),
            );
        }
        painter.text(
            sub(o, 0.0, 0.3, 1.0, 0.4).center(),
            egui::Align2::CENTER_CENTER,
            "MAINT.",
            egui::FontId::proportional(h * 0.05),
            alpha([210, 220, 210], 0.7),
        );
        painter.rect_filled(sub(o, 0.76, 0.5, 0.86, 0.62), 2.0, Color32::from_rgb(30, 36, 34));
        // the padlock
        let lock = sub(o, 0.62, 0.62, 0.92, 0.74);
        painter.circle_stroke(
            lock.center_top(),
            lock.width() * 0.3,
            Stroke::new(3.0, Color32::from_rgb(160, 160, 170)),
        );
        painter.rect_filled(lock, 3.0, Color32::from_rgb(176, 140, 56));
        painter.rect_stroke(
            lock,
            3.0,
            Stroke::new(1.5, Color32::from_rgb(100, 76, 20)),
            egui::StrokeKind::Inside,
        );
    }
}

fn paint_cabinet(painter: &egui::Painter, o: Rect, state: &State, t: f32) {
    let (w, h) = (o.width(), o.height());
    painter.rect_filled(o, 4.0, Color32::from_rgb(58, 62, 70));
    painter.rect_stroke(
        o,
        4.0,
        Stroke::new(3.0, Color32::from_rgb(28, 30, 36)),
        egui::StrokeKind::Inside,
    );
    if !state.flag(alley::F_CABINET) {
        for (x, y) in [(0.07, 0.06), (0.93, 0.06), (0.07, 0.94), (0.93, 0.94)] {
            let c = pos2(o.left() + w * x, o.top() + h * y);
            painter.circle_filled(c, w * 0.03, Color32::from_rgb(130, 134, 142));
            painter.line_segment(
                [c + vec2(-w * 0.02, 0.0), c + vec2(w * 0.02, 0.0)],
                Stroke::new(2.0, Color32::from_rgb(30, 32, 38)),
            );
        }
        let sign = sub(o, 0.36, 0.3, 0.64, 0.6);
        painter.add(Shape::convex_polygon(
            vec![sign.center_top(), sign.right_bottom(), sign.left_bottom()],
            Color32::from_rgb(220, 180, 40),
            Stroke::new(2.0, Color32::from_rgb(40, 30, 10)),
        ));
        painter.text(
            sign.center() + vec2(0.0, sign.height() * 0.2),
            egui::Align2::CENTER_CENTER,
            "!",
            egui::FontId::proportional(h * 0.1),
            Color32::from_rgb(30, 24, 8),
        );
        return;
    }
    // the cover is off: a fuse holder and a panel of lights
    painter.rect_filled(sub(o, 0.05, 0.05, 0.95, 0.95), 3.0, Color32::from_rgb(8, 10, 14));
    let holder = sub(o, 0.12, 0.1, 0.88, 0.26);
    painter.rect_filled(holder, 3.0, Color32::from_rgb(30, 34, 42));
    painter.rect_filled(sub(holder, 0.04, 0.2, 0.2, 0.8), 2.0, Color32::from_rgb(180, 120, 60));
    painter.rect_filled(sub(holder, 0.8, 0.2, 0.96, 0.8), 2.0, Color32::from_rgb(180, 120, 60));
    let fused = state.flag(alley::F_FUSE_IN);
    if fused {
        painter.rect_filled(
            sub(holder, 0.18, 0.25, 0.82, 0.75),
            3.0,
            Color32::from_rgb(170, 182, 194),
        );
        painter.rect_filled(
            sub(holder, 0.42, 0.2, 0.58, 0.8),
            1.0,
            Color32::from_rgb(214, 220, 230),
        );
    }
    let powered = state.flag(alley::F_POWER);
    for row in 0..4u32 {
        for col in 0..4u32 {
            let c = pos2(
                o.left() + w * (0.24 + 0.17 * col as f32),
                o.top() + h * (0.4 + 0.14 * row as f32),
            );
            let on = powered || (fused && (row + col) % 2 == 0);
            painter.circle_filled(
                c,
                w * 0.05,
                if on { Color32::from_rgb(120, 230, 170) } else { Color32::from_rgb(20, 26, 30) },
            );
            if on {
                soft_ellipse(
                    painter,
                    c,
                    vec2(w * 0.1, w * 0.1),
                    [120, 230, 170],
                    0.25 + 0.05 * (t * 4.0 + col as f32).sin(),
                );
            }
        }
    }
}

// ------------------------------------------------------------------------------------ close-ups
enum LookResult {
    Open,
    Closed,
    /// A lock opened (with what it gave).
    Done(Reaction),
    /// The panel of switches is solved.
    Powered,
    /// The shield's code is broken.
    Shield,
}

/// A row of `n` number wheels, each with an arrow above and below it. True when one was turned.
fn wheel_row(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    area: Rect,
    wheels: &mut [u8; 4],
    n: usize,
    lo: u8,
    hi: u8,
) -> bool {
    let size = (area.height() * 0.22).min(area.width() / (n as f32 + 1.0)).clamp(40.0, 110.0);
    let gap = size * 1.25;
    let cy = area.bottom() - size * 1.5;
    let mut changed = false;
    for i in 0..n {
        let cx = area.center().x + (i as f32 - (n as f32 - 1.0) / 2.0) * gap;
        let boxr = Rect::from_center_size(pos2(cx, cy), vec2(size * 0.8, size));
        let up = Rect::from_center_size(pos2(cx, cy - size * 0.82), vec2(size * 0.8, size * 0.55));
        let down =
            Rect::from_center_size(pos2(cx, cy + size * 0.82), vec2(size * 0.8, size * 0.55));
        painter.rect_filled(boxr, 8.0, Color32::from_rgb(8, 10, 16));
        painter.rect_stroke(boxr, 8.0, Stroke::new(2.0, GOLD), egui::StrokeKind::Inside);
        painter.text(
            boxr.center(),
            egui::Align2::CENTER_CENTER,
            wheels[i].to_string(),
            egui::FontId::proportional(size * 0.7),
            INK,
        );
        let ur = ui.interact(up, ui.id().with(("ally_wheel_up", i)), egui::Sense::click());
        let dr = ui.interact(down, ui.id().with(("ally_wheel_down", i)), egui::Sense::click());
        for (r, sign, hovered) in [(up, -1.0f32, ur.hovered()), (down, 1.0, dr.hovered())] {
            let c = r.center();
            let s = size * 0.2;
            painter.add(Shape::convex_polygon(
                vec![
                    c + vec2(-s, sign * -s * 0.6),
                    c + vec2(s, sign * -s * 0.6),
                    c + vec2(0.0, sign * s * 0.7),
                ],
                if hovered { GOLD } else { alpha([244, 200, 80], 0.6) },
                Stroke::NONE,
            ));
        }
        if ur.hovered() || dr.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if ur.clicked() {
            wheels[i] = if wheels[i] >= hi { lo } else { wheels[i] + 1 };
            changed = true;
        }
        if dr.clicked() {
            wheels[i] = if wheels[i] <= lo { hi } else { wheels[i] - 1 };
            changed = true;
        }
    }
    changed
}

/// The close-ups: the notice board, the two locks, the neon sign and the panel of switches.
#[allow(clippy::too_many_arguments)]
fn look_modal(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    look: Look,
    wheels: &mut [u8; 4],
    guesses: &mut Vec<([u8; 4], u8, u8)>,
    panel: Option<&mut Switches>,
    state: &mut State,
    _t: f32,
    escape: bool,
) -> LookResult {
    let time = ui.input(|i| i.time) as f32;
    painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 205));
    let frame =
        Rect::from_center_size(rect.center(), vec2(rect.width() * 0.74, rect.height() * 0.8));
    painter.rect_filled(frame, 16.0, Color32::from_rgb(12, 14, 22));
    painter.rect_stroke(frame, 16.0, Stroke::new(2.0, GOLD), egui::StrokeKind::Inside);
    let title = match look {
        Look::Notice => "The notice board",
        Look::Locker => "The locker's padlock",
        Look::Neon => "The neon sign",
        Look::Vending => "The vending machine's service door",
        Look::Panel => "The cabinet's panel",
        Look::Shield => "The shield over the arch",
    };
    let fs = (frame.height() * 0.05).clamp(16.0, 28.0);
    painter.text(
        pos2(frame.center().x, frame.top() + 34.0),
        egui::Align2::CENTER_CENTER,
        title,
        egui::FontId::proportional(fs),
        INK,
    );
    let back =
        Rect::from_min_size(pos2(frame.right() - 130.0, frame.top() + 14.0), vec2(116.0, 34.0));
    let back_r = ui.interact(back, ui.id().with("ally_look_back"), egui::Sense::click());
    painter.rect_filled(
        back,
        6.0,
        if back_r.hovered() {
            Color32::from_rgb(40, 30, 70)
        } else {
            Color32::from_rgb(20, 16, 38)
        },
    );
    painter.rect_stroke(back, 6.0, Stroke::new(1.5, GOLD), egui::StrokeKind::Inside);
    painter.text(
        back.center(),
        egui::Align2::CENTER_CENTER,
        "Back  (Esc)",
        egui::FontId::proportional(15.0),
        INK,
    );
    if back_r.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let body = Rect::from_min_max(
        pos2(frame.left() + 24.0, frame.top() + 70.0),
        pos2(frame.right() - 24.0, frame.bottom() - 24.0),
    );
    let mut result = LookResult::Open;
    let note = |text: &str| {
        painter.text(
            pos2(body.center().x, body.top() + 6.0),
            egui::Align2::CENTER_TOP,
            text,
            egui::FontId::proportional((frame.height() * 0.032).clamp(13.0, 20.0)),
            alpha([200, 196, 230], 0.8),
        );
    };
    match look {
        Look::Notice => {
            let paper = sub(body, 0.02, 0.02, 0.47, 0.98);
            painter.rect_filled(paper, 3.0, Color32::from_rgb(232, 224, 200));
            let ink = Color32::from_rgb(42, 38, 32);
            let lines = [
                "WORK ORDER 4471",
                "City Works, Block 9 alley",
                "",
                "Re: maintenance locker, west wall.",
                "Padlock has been reset. The new",
                "combination is the three circled",
                "days on the calendar, read in",
                "date order.",
                "",
                "Do not write it on the locker.",
                "                      the Foreman",
            ];
            let lh = paper.height() / (lines.len() as f32 + 1.5);
            for (k, line) in lines.iter().enumerate() {
                painter.text(
                    pos2(paper.left() + paper.width() * 0.07, paper.top() + lh * (k as f32 + 0.9)),
                    egui::Align2::LEFT_CENTER,
                    *line,
                    egui::FontId::proportional((lh * 0.62).clamp(11.0, 22.0)),
                    ink,
                );
            }
            // the calendar
            let cal = sub(body, 0.53, 0.02, 0.98, 0.98);
            painter.rect_filled(cal, 3.0, Color32::from_rgb(238, 236, 228));
            painter.rect_filled(sub(cal, 0.0, 0.0, 1.0, 0.13), 3.0, Color32::from_rgb(170, 50, 50));
            painter.text(
                sub(cal, 0.0, 0.0, 1.0, 0.13).center(),
                egui::Align2::CENTER_CENTER,
                "OCTOBER",
                egui::FontId::proportional(cal.height() * 0.07),
                Color32::from_rgb(250, 240, 230),
            );
            let cw = cal.width() / 7.0;
            let ch = cal.height() * 0.8 / 6.0;
            for (k, name) in ["S", "M", "T", "W", "T", "F", "S"].iter().enumerate() {
                painter.text(
                    pos2(cal.left() + cw * (k as f32 + 0.5), cal.top() + cal.height() * 0.19),
                    egui::Align2::CENTER_CENTER,
                    *name,
                    egui::FontId::proportional(ch * 0.4),
                    Color32::from_rgb(120, 110, 100),
                );
            }
            let code = alley::locker_code(state.seed());
            for day in 1..=30u32 {
                let idx = day + 2;
                let (row, col) = (idx / 7, idx % 7);
                let c = pos2(
                    cal.left() + cw * (col as f32 + 0.5),
                    cal.top() + cal.height() * 0.2 + ch * (row as f32 + 0.8),
                );
                painter.text(
                    c,
                    egui::Align2::CENTER_CENTER,
                    day.to_string(),
                    egui::FontId::proportional(ch * 0.5),
                    ink,
                );
                if code.contains(&(day as u8)) {
                    painter.circle_stroke(
                        c,
                        ch * 0.5,
                        Stroke::new(2.5, Color32::from_rgb(200, 40, 40)),
                    );
                }
            }
        }
        Look::Locker => {
            // the padlock, big
            let lock = Rect::from_center_size(
                pos2(body.center().x, body.top() + body.height() * 0.27),
                vec2(body.height() * 0.34, body.height() * 0.26),
            );
            painter.circle_stroke(
                lock.center_top(),
                lock.width() * 0.32,
                Stroke::new(lock.width() * 0.09, Color32::from_rgb(170, 172, 182)),
            );
            painter.rect_filled(lock, 8.0, Color32::from_rgb(176, 140, 56));
            painter.rect_stroke(
                lock,
                8.0,
                Stroke::new(2.0, Color32::from_rgb(100, 76, 20)),
                egui::StrokeKind::Inside,
            );
            painter.circle_filled(
                lock.center(),
                lock.width() * 0.07,
                Color32::from_rgb(40, 30, 10),
            );
            note("Three wheels.  Turn them to the code.");
            if wheel_row(ui, painter, body, wheels, 3, 0, 9) {
                if let Some(reaction) = alley::try_lock(state, Look::Locker, &wheels[..3]) {
                    result = LookResult::Done(reaction);
                }
            }
        }
        Look::Vending => {
            let door = Rect::from_center_size(
                pos2(body.center().x, body.top() + body.height() * 0.27),
                vec2(body.height() * 0.5, body.height() * 0.28),
            );
            painter.rect_filled(door, 6.0, Color32::from_rgb(40, 46, 56));
            painter.rect_stroke(
                door,
                6.0,
                Stroke::new(3.0, Color32::from_rgb(90, 100, 112)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                door.center(),
                egui::Align2::CENTER_CENTER,
                "SERVICE ONLY",
                egui::FontId::proportional(door.height() * 0.22),
                alpha([170, 180, 190], 0.8),
            );
            note("A four-digit lock.");
            if wheel_row(ui, painter, body, wheels, 4, 0, 9) {
                if let Some(reaction) = alley::try_lock(state, Look::Vending, &wheels[..4]) {
                    result = LookResult::Done(reaction);
                }
            }
        }
        Look::Neon => {
            let sign = sub(body, 0.1, 0.06, 0.9, 0.9);
            painter.rect_filled(sign, 12.0, Color32::from_rgb(8, 8, 14));
            painter.rect_stroke(
                sign,
                12.0,
                Stroke::new(4.0, Color32::from_rgb(44, 48, 62)),
                egui::StrokeKind::Inside,
            );
            let flick = 0.8 + 0.2 * (time * 31.0).sin().abs();
            let head = sub(sign, 0.0, 0.0, 1.0, 0.4).center();
            let font = egui::FontId::proportional(sign.height() * 0.2);
            for (dx, dy) in [(-2.0, 0.0), (2.0, 0.0), (0.0, -2.0), (0.0, 2.0)] {
                painter.text(
                    head + vec2(dx, dy),
                    egui::Align2::CENTER_CENTER,
                    "VOLT-COLA",
                    font.clone(),
                    alpha([255, 60, 120], 0.2 * flick),
                );
            }
            painter.text(
                head,
                egui::Align2::CENTER_CENTER,
                "VOLT-COLA",
                font,
                alpha([255, 170, 200], flick),
            );
            soft_ellipse(
                painter,
                sign.center(),
                vec2(sign.width() * 0.5, sign.height() * 0.4),
                [255, 40, 110],
                0.1,
            );
            // four tubes, one after another
            let now_shown = alley::neon_at(state.seed(), time);
            for i in 0..4usize {
                let c = pos2(
                    sign.left() + sign.width() * (0.2 + 0.2 * i as f32),
                    sign.top() + sign.height() * 0.7,
                );
                let size = sign.height() * 0.22;
                let r = Rect::from_center_size(c, vec2(size * 0.8, size));
                painter.rect_stroke(
                    r,
                    6.0,
                    Stroke::new(2.0, alpha([90, 200, 255], 0.25)),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    c,
                    egui::Align2::CENTER_CENTER,
                    "8",
                    egui::FontId::proportional(size * 0.9),
                    alpha([90, 200, 255], 0.07),
                );
                if let Some((slot, digit)) = now_shown {
                    if slot == i {
                        soft_ellipse(
                            painter,
                            c,
                            vec2(size * 1.1, size * 1.1),
                            [90, 200, 255],
                            0.35,
                        );
                        painter.text(
                            c,
                            egui::Align2::CENTER_CENTER,
                            digit.to_string(),
                            egui::FontId::proportional(size * 0.9),
                            alpha([200, 240, 255], flick),
                        );
                    }
                }
            }
            note("It flickers a digit at a time, then goes dark and starts again.");
            ui.ctx().request_repaint();
        }
        Look::Shield => {
            let seed = state.seed();
            let code = alley::shield_code(seed);
            let left = sub(body, 0.0, 0.0, 0.46, 1.0);
            let right = sub(body, 0.52, 0.0, 1.0, 1.0);
            // the tries so far
            let row_h = (left.height() / (alley::SHIELD_TRIES as f32 + 1.0)).min(46.0);
            painter.text(
                pos2(left.left(), left.top() + row_h * 0.4),
                egui::Align2::LEFT_CENTER,
                format!("Tries  {} / {}", guesses.len(), alley::SHIELD_TRIES),
                egui::FontId::proportional(row_h * 0.5),
                alpha([200, 196, 230], 0.8),
            );
            for (k, (guess, exact, pale)) in guesses.iter().enumerate() {
                let y = left.top() + row_h * (k as f32 + 1.4);
                for (d, digit) in guess.iter().enumerate() {
                    let b = Rect::from_center_size(
                        pos2(left.left() + row_h * (0.5 + 0.95 * d as f32), y),
                        vec2(row_h * 0.8, row_h * 0.8),
                    );
                    painter.rect_filled(b, 5.0, Color32::from_rgb(8, 14, 16));
                    painter.rect_stroke(
                        b,
                        5.0,
                        Stroke::new(1.0, alpha([90, 255, 210], 0.4)),
                        egui::StrokeKind::Inside,
                    );
                    painter.text(
                        b.center(),
                        egui::Align2::CENTER_CENTER,
                        digit.to_string(),
                        egui::FontId::monospace(row_h * 0.55),
                        alpha([120, 255, 220], 0.95),
                    );
                }
                for m in 0..4u8 {
                    let c = pos2(left.left() + row_h * 4.6 + row_h * 0.45 * m as f32, y);
                    if m < *exact {
                        painter.circle_filled(c, row_h * 0.16, GOLD);
                    } else if m < exact + pale {
                        painter.circle_filled(c, row_h * 0.16, alpha([200, 210, 230], 0.55));
                    } else {
                        painter.circle_stroke(
                            c,
                            row_h * 0.16,
                            Stroke::new(1.0, alpha([120, 130, 150], 0.4)),
                        );
                    }
                }
            }
            // the rules
            let text = "Break the code: four numbers, each from 1 to 6, and a number can come up twice. A gold mark means a right number in the right place. A pale mark means a right number in the wrong place. After ten tries the shield wipes the board and you start over.";
            let fs = (right.height() * 0.04).clamp(13.0, 20.0);
            let galley = painter.layout(
                text.to_string(),
                egui::FontId::proportional(fs),
                alpha([200, 196, 230], 0.85),
                right.width(),
            );
            painter.galley(right.min, galley, INK);
            // the wheels, and the button
            let wheel_area = sub(right, 0.0, 0.0, 1.0, 0.8);
            wheel_row(ui, painter, wheel_area, wheels, 4, 1, 6);
            let try_r = Rect::from_center_size(
                pos2(right.center().x, right.bottom() - right.height() * 0.07),
                vec2(right.width() * 0.6, right.height() * 0.1),
            );
            let try_resp =
                ui.interact(try_r, ui.id().with("ally_shield_try"), egui::Sense::click());
            painter.rect_filled(
                try_r,
                8.0,
                if try_resp.hovered() {
                    Color32::from_rgb(30, 70, 66)
                } else {
                    Color32::from_rgb(16, 40, 40)
                },
            );
            painter.rect_stroke(
                try_r,
                8.0,
                Stroke::new(2.0, alpha([90, 255, 210], 0.8)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                try_r.center(),
                egui::Align2::CENTER_CENTER,
                "Try this code",
                egui::FontId::proportional(try_r.height() * 0.42),
                INK,
            );
            if try_resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if try_resp.clicked() {
                let guess = [wheels[0], wheels[1], wheels[2], wheels[3]];
                let (exact, pale) = alley::shield_score(code, guess);
                if exact == 4 {
                    result = LookResult::Shield;
                } else {
                    if guesses.len() >= alley::SHIELD_TRIES {
                        guesses.clear();
                    }
                    guesses.push((guess, exact, pale));
                }
            }
        }
        Look::Panel => {
            let Some(panel) = panel else {
                return LookResult::Closed;
            };
            note("Make every light shine.  A press turns the light and the ones next to it.");
            let n = panel.size;
            let side = (body.height() * 0.82).min(body.width() * 0.6);
            let cell = side / n as f32;
            let origin = pos2(body.center().x - side / 2.0, body.bottom() - side - 4.0);
            for row in 0..n {
                for col in 0..n {
                    let i = row * n + col;
                    let r = Rect::from_min_size(
                        pos2(origin.x + cell * col as f32, origin.y + cell * row as f32),
                        vec2(cell, cell),
                    )
                    .shrink(cell * 0.08);
                    let on = panel.on[i];
                    painter.rect_filled(
                        r,
                        10.0,
                        if on {
                            Color32::from_rgb(244, 200, 80)
                        } else {
                            Color32::from_rgb(18, 22, 30)
                        },
                    );
                    painter.rect_stroke(
                        r,
                        10.0,
                        Stroke::new(2.0, alpha([244, 200, 80], 0.5)),
                        egui::StrokeKind::Inside,
                    );
                    if on {
                        soft_ellipse(
                            painter,
                            r.center(),
                            vec2(r.width() * 0.8, r.height() * 0.8),
                            [255, 220, 120],
                            0.25,
                        );
                    }
                    let response =
                        ui.interact(r, ui.id().with(("ally_switch", i)), egui::Sense::click());
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if response.clicked() {
                        panel.press(i);
                    }
                }
            }
            if panel.solved() {
                result = LookResult::Powered;
            }
        }
    }
    if matches!(result, LookResult::Open) && (back_r.clicked() || escape) {
        result = LookResult::Closed;
    }
    result
}
