// The Devil's Table: the Texas Hold'em game in the Underworld, seen from your chair. Everything is
// painted: the cavern, the five people (poker_bear.rs and poker_art.rs), the felt, the cards and
// the chips. The game itself is a `poker_run::Session`; this file only draws it and passes on your
// clicks. The layout is the design board's: a picture 1100 by 700 (with room to spare at both sides, see
// `VIEW_W`) that is scaled to fit the window.
use super::monarch_art;
use super::backdrop::soft_ellipse;
use super::poker_art::{self, FatherState};
use super::poker_bear::{self, Bear, Mood};
use super::poker_card_art;
use super::poker_chip_art;
use super::poker_room;
use crate::audio::Audio;
use crate::poker_cam::{Director, Frame, Who, VIEW_W};
use crate::poker_deal::{self as deal, Beat, Dealing, Pose, Put, Spot};
use crate::poker_cards::{self as cards, Card};
use crate::poker_fair::HandProof;
use crate::poker_game::{BEAR, CAT, FATHER, MONARCH, PAYOUTS, PLAYERS, YOU};
use crate::poker_react::Look;
use crate::poker_run::{group, short_name, Finish, Phase, Session};
use crate::poker_save::TableOptions;
use crate::poker_story::{Cue, Speaker};
use crate::poker_table::{Action, Chips};
use crate::king;
use crate::sfx::{self, Sfx};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Ui};

const CREAM: Color32 = Color32::from_rgb(244, 233, 216);
const SAND: Color32 = Color32::from_rgb(232, 220, 203);
const AMBER: Color32 = Color32::from_rgb(241, 178, 122);
const GOLD: Color32 = Color32::from_rgb(216, 192, 122);
const DIM: Color32 = Color32::from_rgb(184, 165, 144);
const MINT: Color32 = Color32::from_rgb(159, 195, 182);
const INK: Color32 = Color32::from_rgb(10, 5, 5);

/// Where each person sits along the table edge, in the board's own pixels (1100 x 700).
const SEAT_X: [f32; PLAYERS] = [550.0, 60.0, 470.0, 975.0, 715.0];
const DEALER_X: f32 = 255.0;
/// Where each nameplate (and the chips in front of it) is. The King sits half out of the picture, so
/// his plate is further in than he is.
const PLATE_X: [f32; PLAYERS] = [550.0, 115.0, 470.0, 975.0, 715.0];
const DEALER_PLATE_X: f32 = 277.0;
/// The table's far edge, and the row the nameplates sit on.
const EDGE_Y: f32 = 334.0;
const PLATE_Y: f32 = 316.0;

/// What the screen tells the program.
#[derive(Debug, Clone, Copy)]
pub enum Event {
    None,
    /// Leave the table (the game is over, or has been left).
    Quit,
    /// The game has been decided: pay out and, if the Monarch won, take the soul. Given once.
    Finished(Finish),
}

pub struct Run {
    pub s: Session,
    born: Option<f64>,
    raise: f64,
    was_my_turn: bool,
    seed_edit: String,
    dir: Director,
    frame: Frame,
    audio: Option<Audio>,
    /// Mammon's robotic hand, which puts the cards down.
    dealing: Dealing,
    /// Which take of the card sound is next.
    card_n: usize,
    /// When your chips were put in front of you (once the camera was back and the first hand could be dealt).
    stack_born: Option<f64>,
    /// How much of them is down by now, 0 to 1.
    stack_reveal: f32,
    chips_paid: u64,
    cued: Option<&'static str>,
    king_laugh: Option<f64>,
    father_off: Option<f64>,
    father_back: Option<f64>,
    father_was_off: bool,
    monarch_in: Option<f64>,
    bow_at: f64,
    last_hand_no: u64,
    looks: [Look; PLAYERS],
    mood: (Mood, f64),
    confirm_leave: bool,
    fair_open: bool,
    /// The hell gear's settings (kept in the config by the caller: see `opts_changed`).
    pub opts: TableOptions,
    /// Set when the gear's settings were changed; the caller saves them and clears it.
    pub opts_changed: bool,
    /// Set when the two lovers were dealt to you together (the developer switch's hand too); the caller takes it.
    pub together: bool,
    /// Both lovers are in your hand and their scenes are still to play out; the trophy comes once they are set back down.
    pending_together: bool,
    /// The gear's panel is open.
    gear_open: bool,
    /// How far the gear has turned.
    gear_a: f32,
    /// Seconds the table has been waiting to deal by itself.
    auto_wait: f32,
    auto_phase: Option<Phase>,
    /// The hand and board the skipped dealing last made a sound for.
    skip_seen: (u64, usize),
    /// The CR logo that the chips carry on their faces (loaded on the first frame).
    chip_face: Option<egui::TextureHandle>,
    chip_face_tried: bool,
    /// Who the game is waiting on to act (once for a moment), and since when.
    wait_on: Option<(usize, f64)>,
    think_seat: Option<usize>,
    /// How far their thinking has come in, 0 to 1.
    think: f32,
    /// The line on the screen has Father Time blowing smoke in your face when it is clicked past.
    smoke_owed: bool,
    /// The first Queen or King dealt to you begs for help: the card, since when, and which line it is on.
    plea: Option<(Card, f64, usize)>,
    /// Whether you have been begged by the King of Hearts and the Queen of Diamonds yet (this game), what is left of
    /// the scenes of this hand, when the line on the screen came up, and which hand it was.
    seen_king: bool,
    seen_queen: bool,
    plea_queue: Vec<Card>,
    plea_line_at: f64,
    plea_hand: u64,
    /// What you are told when you stare at a chained monarch in your hand (instead of what they beg), and how
    /// many times you have.
    gaze: Option<&'static str>,
    gaze_n: usize,
    /// Souls let out of a card that was clicked (which card, and when: see `card_key`), and a card's remark (which
    /// card, what it said and when).
    bursts: Vec<(u8, f32)>,
    quip: Option<(u8, &'static str, &'static str, f32)>,
    /// The developer rows of the gear: the animation list is open; the Shadow is shown (since when) without
    /// having sat down; a look or a think forced on someone for a few seconds; the arrival is to be cued.
    dev_open: bool,
    dev_shadow: Option<f64>,
    dev_look: Option<(usize, Look, f64)>,
    dev_think: Option<(usize, f64)>,
    dev_arrival: bool,
    /// When the camera last let go of someone it had been waiting on.
    think_end: f64,
    /// How the game came out, once it is decided.
    pub result: Option<Finish>,
}

/// The board's pixels on the screen.
struct Lay {
    rect: Rect,
    k: f32,
    ox: f32,
    oy: f32,
}

impl Lay {
    fn new(rect: Rect) -> Lay {
        let k = (rect.width() / VIEW_W).min(rect.height() / 700.0).max(0.2);
        Lay { rect, k, ox: rect.center().x - 550.0 * k, oy: rect.center().y - 350.0 * k }
    }

    /// The same picture seen by the camera: magnified by `f.zoom` about the board point (f.x, f.y), which
    /// slides towards the upper middle of the window as the zoom grows, and shaken by `f.shake`.
    fn with_cam(rect: Rect, f: &Frame) -> Lay {
        let k0 = (rect.width() / VIEW_W).min(rect.height() / 700.0).max(0.2);
        // a window wider than the picture shows more of the sides: close shots are made closer to match
        let widen = 1.0 + (rect.width() / k0 / VIEW_W - 1.0).max(0.0) * (f.zoom - 1.0).clamp(0.0, 1.0);
        let zoom = f.zoom.max(1.0) * widen;
        let k = k0 * zoom;
        let s0 = pos2(rect.center().x - 550.0 * k0 + f.x * k0, rect.center().y - 350.0 * k0 + f.y * k0);
        let want = pos2(rect.center().x, rect.top() + rect.height() * 0.42);
        let u = ((zoom - 1.0) / 0.6).clamp(0.0, 1.0);
        let u = u * u * (3.0 - 2.0 * u);
        let s = s0 + (want - s0) * u;
        Lay { rect, k, ox: s.x - f.x * k + f.shake.0 * k, oy: s.y - f.y * k + f.shake.1 * k }
    }

    fn p(&self, x: f32, y: f32) -> Pos2 {
        pos2(self.ox + x * self.k, self.oy + y * self.k)
    }

    /// A size in board pixels, never smaller than `min` points.
    fn s(&self, v: f32, min: f32) -> f32 {
        (v * self.k).max(min)
    }

    /// The part of the picture between two board x positions, from the top of the window to the table edge.
    fn column(&self, x0: f32, x1: f32) -> Rect {
        Rect::from_min_max(pos2(self.ox + x0 * self.k, self.rect.top()), pos2(self.ox + x1 * self.k, self.oy + (EDGE_Y + 4.0) * self.k))
    }
}

fn faded(c: Color32, a: f32) -> Color32 {
    c.gamma_multiply(a.clamp(0.0, 1.0))
}

fn text(p: &Painter, pos: Pos2, anchor: Align2, s: impl ToString, size: f32, col: Color32) -> Rect {
    p.text(pos, anchor, s, FontId::proportional(size), col)
}

fn seat_colour(seat: usize) -> Color32 {
    match seat {
        YOU => SAND,
        CAT => Color32::from_rgb(182, 164, 238),
        BEAR => AMBER,
        FATHER => Color32::from_rgb(207, 196, 166),
        _ => Color32::from_rgb(168, 140, 240),
    }
}

fn speaker_colour(who: Speaker) -> Color32 {
    match who {
        Speaker::Dealer => GOLD,
        Speaker::Bear => AMBER,
        Speaker::Cat => seat_colour(CAT),
        Speaker::Father => seat_colour(FATHER),
        Speaker::Monarch => seat_colour(MONARCH),
        Speaker::Stage => DIM,
    }
}

// ---- cards and chips -------------------------------------------------------------------------------

fn suit_icon(p: &Painter, c: Pos2, r: f32, suit: usize, col: Color32) {
    let tri = |a: Pos2, b: Pos2, d: Pos2| {
        p.add(Shape::convex_polygon(vec![a, b, d], col, Stroke::NONE));
    };
    match suit {
        // clubs
        0 => {
            for (dx, dy) in [(0.0, -0.55), (-0.58, 0.2), (0.58, 0.2)] {
                p.circle_filled(c + vec2(dx * r, dy * r), r * 0.5, col);
            }
            tri(c + vec2(0.0, 0.0), c + vec2(-0.35 * r, 1.1 * r), c + vec2(0.35 * r, 1.1 * r));
        }
        // diamonds
        1 => {
            p.add(Shape::convex_polygon(vec![c + vec2(0.0, -1.1 * r), c + vec2(0.8 * r, 0.0), c + vec2(0.0, 1.1 * r), c + vec2(-0.8 * r, 0.0)], col, Stroke::NONE));
        }
        // hearts
        2 => {
            for dx in [-0.5, 0.5] {
                p.circle_filled(c + vec2(dx * r, -0.3 * r), r * 0.58, col);
            }
            tri(c + vec2(-1.04 * r, -0.12 * r), c + vec2(1.04 * r, -0.12 * r), c + vec2(0.0, 1.1 * r));
        }
        // spades
        _ => {
            for dx in [-0.5, 0.5] {
                p.circle_filled(c + vec2(dx * r, 0.28 * r), r * 0.58, col);
            }
            tri(c + vec2(-1.04 * r, 0.1 * r), c + vec2(1.04 * r, 0.1 * r), c + vec2(0.0, -1.1 * r));
            tri(c + vec2(0.0, 0.2 * r), c + vec2(-0.38 * r, 1.1 * r), c + vec2(0.38 * r, 1.1 * r));
        }
    }
}

fn rank_label(c: Card) -> String {
    match cards::RANKS[cards::rank(c)] {
        'T' => "10".to_string(),
        other => other.to_string(),
    }
}

/// The plain card (for the little ones at showdown and when the engraved picture cannot be read): satin
/// silver with dark ink, like the engraved ones.
const CARD_FIELD: Color32 = Color32::from_rgb(170, 174, 182);

fn etch(a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(15, 17, 27, (a.clamp(0.0, 1.0) * 235.0) as u8)
}
/// The light blue of the souls (the whispers of the Underworld), a little brighter for the red suits.
const SOUL_DEEP: [u8; 3] = [118, 176, 240];
const SOUL_BRIGHT: [u8; 3] = [186, 226, 255];
/// Polished silver: the card's frame, its numbers and its big pip. And the shade of it in the shadows.
const SILVER: [u8; 3] = [224, 230, 240];
const SILVER_SHADE: [u8; 3] = [92, 100, 116];

fn soul(c: [u8; 3], a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// The text, drawn twice half a point apart so that it reads as bold.
fn bold(p: &Painter, pos: Pos2, s: &str, size: f32, col: Color32) {
    text(p, pos + vec2(-0.3, 0.0), Align2::CENTER_CENTER, s, size, col);
    text(p, pos + vec2(0.3, 0.0), Align2::CENTER_CENTER, s, size, col);
}

/// A card face: black, framed and numbered in polished silver, with the big pip in silver too (shaded
/// below and catching the light above). The small pips in the corners are the soul blue that the souls
/// come out of (the hearts and diamonds the brighter blue, so that the suits can be told apart at a
/// glance). The little cards at showdown are the rank and the pip.
fn card_face(p: &Painter, r: Rect, c: Card, shadow: bool) {
    // the engraved picture of the card, when it is big enough to be worth it
    if r.width() >= 24.0 && poker_card_art::paint(p, r, c as usize, shadow) {
        // a gleam rolls over the big silver symbol every few seconds
        poker_card_art::gleam(p, r, c as usize, p.ctx().input(|i| i.time) as f32);
        return;
    }
    let (w, h) = (r.width(), r.height());
    let round = (w * 0.075).max(2.0);
    if shadow {
        p.rect_filled(r.translate(vec2(0.0, h * 0.05)).expand(w * 0.012), round, Color32::from_black_alpha(150));
    }
    p.rect_filled(r, round, CARD_FIELD);
    p.rect_stroke(r, round, Stroke::new(1.3, etch(0.9)), StrokeKind::Inside);
    let suit = cards::suit(c);
    let blue = soul(if suit == 1 || suit == 2 { SOUL_BRIGHT } else { SOUL_DEEP }, 1.0);
    let ink = etch(1.0);
    if w < 40.0 {
        bold(p, pos2(r.center().x, r.top() + h * 0.30), &rank_label(c), (h * 0.40).max(9.0), ink);
        suit_icon(p, pos2(r.center().x, r.top() + h * 0.72), w * 0.2, suit, blue);
        return;
    }
    let inner = r.shrink(w * 0.07);
    p.rect_stroke(inner, round * 0.6, Stroke::new(0.8, etch(0.3)), StrokeKind::Inside);
    let pip = pos2(r.center().x + w * 0.05, r.center().y + h * 0.09);
    soft_ellipse(p, pip, vec2(w * 0.34, w * 0.34), SILVER, 0.10);
    let ix = r.left() + w * 0.22;
    bold(p, pos2(ix, r.top() + h * 0.17), &rank_label(c), h * 0.21, ink);
    suit_icon(p, pos2(ix, r.top() + h * 0.335), w * 0.085, suit, blue);
    silver_pip(p, pip, w * 0.27, suit);
    suit_icon(p, pos2(r.right() - w * 0.2, r.bottom() - h * 0.12), w * 0.07, suit, blue);
}

/// The big pip in polished silver: a darker edge below it, the silver itself and a bright spot where
/// the light catches the upper left.
fn silver_pip(p: &Painter, c: Pos2, r: f32, suit: usize) {
    suit_icon(p, c + vec2(0.0, 0.05 * r), r, suit, soul(SILVER_SHADE, 1.0));
    suit_icon(p, c, r * 0.97, suit, soul(SILVER, 1.0));
    suit_icon(p, c + vec2(-0.14 * r, -0.16 * r), r * 0.5, suit, soul([252, 253, 255], 0.55));
}

/// Every so often a soul comes up out of the big pip of a face-up card: it swells out of the icon,
/// opens its eyes, drifts up past the top of the card and fades. Each card has its own slow rhythm
/// (from its own number), so they never come together.
fn card_soul(p: &Painter, r: Rect, c: Card, t: f32) {
    let w = r.width();
    if w < 40.0 || !poker_card_art::has_wells(c as usize) {
        return;
    }
    let seed = c as f32;
    let period = 22.0 + (seed * 1.618).fract() * 26.0;
    let lead = (seed * 3.17).fract() * period;
    let u = (t + lead) % period;
    const LIFE: f32 = 4.4;
    if u >= LIFE {
        return;
    }
    let a = u / LIFE;
    // (a soul comes out of one of the two wells, the other one the next time)
    let cycle = ((t + lead) / period) as u64;
    let well = if cycle % 2 == 0 { poker_card_art::WELL_TOP } else { poker_card_art::WELL_BOTTOM };
    let origin = pos2(r.left() + w * 0.5, r.top() + r.height() * well);
    soul_wisp(p, r, origin, seed, a);
}

/// One soul, `a` (0 to 1) of the way through its rise from `origin` (a well of the card at `r`): it swells out
/// of the well, opens its eyes, drifts up past the top of the card and fades.
fn soul_wisp(p: &Painter, r: Rect, origin: Pos2, seed: f32, a: f32) {
    use crate::poker_cam::bump;
    let w = r.width();
    let at = |a: f32| -> Pos2 {
        let a = a.clamp(0.0, 1.0);
        origin + vec2((a * 6.0 + seed).sin() * w * 0.12 * a, -a * r.height() * 0.95 - a * a * r.height() * 0.3)
    };
    let fade = bump(a, 0.0, 0.18, 0.7, 1.0, 1.0);
    let unit = w * 0.0175 * (0.45 + 0.55 * (a * 2.0).min(1.0));
    let blue = SOUL_BRIGHT;
    for k in 1..9 {
        let f = 1.0 - k as f32 / 9.0;
        p.circle_filled(at(a - k as f32 * 0.03), 5.2 * unit * (0.3 + 0.7 * f), soul(blue, 0.34 * fade * f));
    }
    let head = at(a);
    p.circle_filled(head, 20.0 * unit, soul(SOUL_DEEP, 0.12 * fade));
    p.circle_filled(head, 11.0 * unit, soul(blue, 0.28 * fade));
    p.circle_filled(head, 5.6 * unit, soul(blue, 0.95 * fade));
    p.circle_filled(head + vec2(0.0, -0.8 * unit), 3.2 * unit, soul([255, 255, 255], fade));
    let eyes = bump(a, 0.25, 0.4, 0.65, 0.8, 1.0);
    if eyes > 0.01 {
        for dx in [-1.9_f32, 1.9] {
            p.circle_filled(head + vec2(dx * unit, -0.2 * unit), 0.95 * unit * eyes, soul([24, 40, 84], 0.9 * fade));
        }
    }
}

/// Souls let out of both wells of a card (`age` seconds after it was clicked): two from each, one after another.
fn card_burst(p: &Painter, r: Rect, c: Card, age: f32) {
    if r.width() < 40.0 || !(0.0..8.0).contains(&age) {
        return;
    }
    const LIFE: f32 = 3.6;
    for k in 0..4 {
        let a = (age - k as f32 * 0.28) / LIFE;
        if !(0.0..=1.0).contains(&a) {
            continue;
        }
        let well = if k % 2 == 0 { poker_card_art::WELL_TOP } else { poker_card_art::WELL_BOTTOM };
        let seed = c as f32 + k as f32 * 1.7;
        let side = if k < 2 { -0.05 } else { 0.05 };
        soul_wisp(p, r, pos2(r.left() + r.width() * (0.5 + side), r.top() + r.height() * well), seed, a);
    }
}

/// A card back: near black with a lattice of silver and one blue ember in the middle.
fn card_back(p: &Painter, r: Rect, shadow: bool) {
    if r.width() >= 24.0 && poker_card_art::paint(p, r, poker_card_art::BACK, shadow) {
        return;
    }
    let (w, h) = (r.width(), r.height());
    let round = (w * 0.075).max(2.0);
    if shadow {
        p.rect_filled(r.translate(vec2(0.0, h * 0.05)).expand(w * 0.012), round, Color32::from_black_alpha(150));
    }
    p.rect_filled(r, round, CARD_FIELD);
    p.rect_stroke(r, round, Stroke::new(1.3, etch(0.9)), StrokeKind::Inside);
    let inner = r.shrink(w * 0.1);
    p.rect_stroke(inner, round * 0.5, Stroke::new(0.8, etch(0.42)), StrokeKind::Inside);
    if w >= 24.0 {
        let q = p.with_clip_rect(inner);
        let step = (w * 0.2).max(4.0);
        let line = Stroke::new(0.7, etch(0.2));
        let n = ((inner.width() + inner.height()) / step) as i32 + 1;
        for i in 0..=n {
            let o = i as f32 * step;
            q.line_segment([pos2(inner.left() + o, inner.top()), pos2(inner.left() + o - inner.height(), inner.bottom())], line);
            q.line_segment([pos2(inner.left() + o - inner.height(), inner.top()), pos2(inner.left() + o, inner.bottom())], line);
        }
    }
    let c = r.center();
    let (hw, hh) = (w * 0.2, h * 0.14);
    p.add(Shape::convex_polygon(vec![c + vec2(0.0, -hh), c + vec2(hw, 0.0), c + vec2(0.0, hh), c + vec2(-hw, 0.0)], CARD_FIELD, Stroke::new(1.0, etch(0.9))));
    p.circle_filled(c, (w * 0.05).max(1.0), soul(SOUL_BRIGHT, 1.0));
}

/// A card that has just been put down: it lands a little above the felt, face down, and turns over.
fn flip_card(p: &Painter, r: Rect, c: Card, age: f32) {
    let f = (age / 0.30).clamp(0.0, 1.0);
    if f >= 1.0 {
        card_face(p, r, c, true);
        return;
    }
    let r = r.translate(vec2(0.0, -(1.0 - (age / 0.12).clamp(0.0, 1.0)).powi(2) * r.height() * 0.10));
    // seen edge-on half way through the turn
    let sx = (std::f32::consts::PI * f).cos().abs();
    let narrow = Rect::from_center_size(r.center(), vec2((r.width() * sx).max(2.0), r.height()));
    if sx < 0.2 {
        // edge-on: just the thickness of a card
        p.rect_filled(narrow, 1.0, CARD_FIELD);
        p.rect_stroke(narrow, 1.0, Stroke::new(1.0, etch(0.9)), StrokeKind::Inside);
    } else if f < 0.5 {
        card_back(p, narrow, true);
    } else {
        card_face(p, narrow, c, true);
    }
}

/// A chain along `pts`: satin sterling silver (the silver of the cards: dull, with a soft gleam that
/// travels slowly along it) in links turned alternately flat and edge on, with a clamp plate and a rivet
/// every few links.
fn chain(p: &Painter, pts: &[Pos2], k: f32, clip: Rect, t: f32) {
    let step = 15.0 * k;
    let mut links: Vec<(Pos2, f32, usize)> = Vec::new();
    let mut count = 0usize;
    let mut since = step;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let seg = b - a;
        let len = seg.length();
        if len < 0.01 {
            continue;
        }
        let dir = seg / len;
        let ang = dir.y.atan2(dir.x);
        let mut s = (step - since).max(0.0);
        while s <= len {
            let at = a + dir * s;
            if clip.contains(at) {
                links.push((at, ang, count));
            }
            count += 1;
            s += step;
        }
        since = len - (s - step);
    }
    let oval = |c: Pos2, ang: f32, rx: f32, ry: f32| -> Vec<Pos2> {
        let (ca, sa) = (ang.cos(), ang.sin());
        (0..14)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / 14.0;
                let (x, y) = (a.cos() * rx, a.sin() * ry);
                c + vec2(x * ca - y * sa, x * sa + y * ca)
            })
            .collect()
    };
    for (c, ang, i) in links.iter() {
        let (rx, ry) = if i % 2 == 0 { (step * 0.66, step * 0.36) } else { (step * 0.34, step * 0.34) };
        // the shadow under the link, then the silver in three passes: a darker body, the polished metal
        // and the bright line where the light runs along its top
        p.add(Shape::closed_line(oval(*c + vec2(0.0, 2.2 * k), *ang, rx, ry), Stroke::new((3.8 * k).max(1.6), Color32::from_black_alpha(150))));
        // the gleam: a slow wave along the chain, sharpened so that it is a few links at a time
        let gleam = (0.5 + 0.5 * (t * 0.9 - *i as f32 * 0.16).sin()).powf(7.0);
        let satin = crate::theme::mix(Color32::from_rgb(138, 144, 156), Color32::from_rgb(190, 196, 206), gleam);
        p.add(Shape::closed_line(oval(*c, *ang, rx, ry), Stroke::new((3.2 * k).max(1.4), Color32::from_rgb(70, 76, 88))));
        p.add(Shape::closed_line(oval(*c + vec2(0.0, -0.3 * k), *ang, rx, ry), Stroke::new((2.1 * k).max(1.0), satin)));
        p.add(Shape::closed_line(oval(*c + vec2(0.0, -0.9 * k), *ang, rx, ry), Stroke::new((0.8 * k).max(0.6), crate::theme::mix(Color32::from_rgb(166, 172, 184), Color32::from_rgb(228, 232, 240), gleam))));
    }
    for (c, ang, i) in links.iter() {
        if i % 8 != 4 {
            continue;
        }
        let (ca, sa) = (ang.cos(), ang.sin());
        let corner = |x: f32, y: f32| *c + vec2(x * ca - y * sa, x * sa + y * ca) * k;
        let plate = vec![corner(-10.0, -7.0), corner(10.0, -7.0), corner(10.0, 7.0), corner(-10.0, 7.0)];
        p.add(Shape::convex_polygon(plate, Color32::from_rgb(112, 118, 130), Stroke::new((1.2 * k).max(1.0), Color32::from_rgb(178, 184, 195))));
        p.circle_filled(*c, (2.3 * k).max(1.2), Color32::from_rgb(196, 201, 211));
        p.circle_stroke(*c, (2.3 * k).max(1.2), Stroke::new((0.7 * k).max(0.5), Color32::from_rgb(70, 76, 88)));
    }
}

/// Your eyes screwed shut: the lids come down from the top and up from the bottom with soft edges.
fn paint_lids(p: &Painter, r: Rect, squint: f32) {
    if squint < 0.02 {
        return;
    }
    let lid = Color32::from_rgb(6, 3, 4);
    let h = r.height() * 0.5 * squint.clamp(0.0, 1.0);
    let soft = (r.height() * 0.08).max(12.0);
    p.rect_filled(Rect::from_min_max(r.left_top(), pos2(r.right(), r.top() + h)), 0.0, lid);
    gradient_band(p, Rect::from_min_max(pos2(r.left(), r.top() + h), pos2(r.right(), r.top() + h + soft)), lid, false);
    p.rect_filled(Rect::from_min_max(pos2(r.left(), r.bottom() - h), r.right_bottom()), 0.0, lid);
    gradient_band(p, Rect::from_min_max(pos2(r.left(), r.bottom() - h - soft), pos2(r.right(), r.bottom() - h)), lid, true);
}

/// A band that fades from clear to `col` going down (or up, with `up`).
fn gradient_band(p: &Painter, r: Rect, col: Color32, up: bool) {
    let clear = Color32::TRANSPARENT;
    let (top, bottom) = if up { (clear, col) } else { (col, clear) };
    let mut m = Mesh::default();
    m.colored_vertex(r.left_top(), top);
    m.colored_vertex(r.right_top(), top);
    m.colored_vertex(r.right_bottom(), bottom);
    m.colored_vertex(r.left_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(Shape::mesh(m));
}

fn pill(p: &Painter, centre: Pos2, label: &str, size: f32, ink: Color32, fill: Color32, line: Color32) -> Rect {
    let g = p.layout_no_wrap(label.to_string(), FontId::proportional(size), ink);
    let r = Rect::from_center_size(centre, g.size() + vec2(size * 1.1, size * 0.6));
    p.rect_filled(r, size * 0.5, fill);
    p.rect_stroke(r, size * 0.5, Stroke::new(1.0, line), StrokeKind::Inside);
    p.galley(r.center() - g.size() / 2.0, g, ink);
    r
}

// ---- the screen ------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Plain,
    Primary,
    Danger,
}

fn press(ui: &mut Ui, p: &Painter, r: Rect, label: &str, style: Style, id: &str) -> bool {
    let resp = ui.interact(r, ui.id().with(id), egui::Sense::click());
    let hot = resp.hovered();
    let (fill, line, ink) = match style {
        Style::Plain => (Color32::from_rgb(42, 21, 18), Color32::from_rgb(110, 58, 40), SAND),
        Style::Primary => (AMBER, AMBER, Color32::from_rgb(27, 11, 10)),
        Style::Danger => (Color32::from_rgb(90, 15, 12), Color32::from_rgb(196, 26, 14), Color32::from_rgb(255, 217, 207)),
    };
    let fill = if hot { crate::theme::mix(fill, Color32::WHITE, 0.12) } else { fill };
    p.rect_filled(r, 8.0, fill);
    p.rect_stroke(r, 8.0, Stroke::new(if hot { 2.0 } else { 1.0 }, line), StrokeKind::Inside);
    text(p, r.center(), Align2::CENTER_CENTER, label.to_uppercase(), (r.height() * 0.42).clamp(12.0, 20.0), ink);
    if hot {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

/// The hell gear: a toothed iron wheel with a molten core, turning slowly (faster when it is touched).
fn paint_gear(p: &Painter, c: Pos2, r: f32, angle: f32, lit: bool, t: f32) {
    use std::f32::consts::TAU;
    let pulse = 0.5 + 0.5 * (t * 2.3).sin();
    let heat = if lit { 0.85 } else { 0.5 } + 0.15 * pulse;
    // the heat around it
    p.circle_filled(c, r * 1.3, Color32::from_rgba_unmultiplied(255, 80, 20, (heat * 42.0) as u8));
    p.circle_filled(c, r * 1.12, Color32::from_rgba_unmultiplied(255, 110, 30, (heat * 46.0) as u8));
    let teeth = 10usize;
    let (outer, inner) = (r * 0.98, r * 0.78);
    let step = TAU / teeth as f32;
    let mut ring: Vec<Pos2> = Vec::with_capacity(teeth * 4);
    for i in 0..teeth {
        let base = angle + i as f32 * step;
        for (f, rad) in [(0.0, inner), (0.14, outer), (0.4, outer), (0.54, inner)] {
            let a = base + step * f;
            ring.push(c + vec2(a.cos(), a.sin()) * rad);
        }
    }
    // the body: fanned from the middle (the shape is star-like, so every triangle is inside it)
    let mut m = Mesh::default();
    m.colored_vertex(c, Color32::from_rgb(40, 22, 20));
    for q in &ring {
        m.colored_vertex(*q, Color32::from_rgb(104, 54, 38));
    }
    let n = ring.len() as u32;
    for i in 0..n {
        m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    p.add(Shape::mesh(m));
    let rim = if lit { Color32::from_rgb(255, 150, 70) } else { Color32::from_rgb(214, 96, 44) };
    p.add(Shape::closed_line(ring, Stroke::new((r * 0.07).max(1.2), rim)));
    // a groove, five lightening holes, and the core
    p.circle_stroke(c, inner * 0.74, Stroke::new((r * 0.05).max(1.0), Color32::from_rgb(150, 70, 40)));
    for i in 0..5 {
        let a = angle + i as f32 * TAU / 5.0 + 0.3;
        let hole = c + vec2(a.cos(), a.sin()) * inner * 0.52;
        p.circle_filled(hole, inner * 0.15, Color32::from_rgb(12, 5, 5));
        p.circle_stroke(hole, inner * 0.15, Stroke::new(1.0, Color32::from_rgb(120, 52, 30)));
    }
    p.circle_filled(c, inner * 0.3, Color32::from_rgb(20, 8, 7));
    p.circle_filled(c, inner * 0.24, Color32::from_rgb(255, (70.0 + 60.0 * heat) as u8, 18));
    p.circle_filled(c, inner * 0.11, Color32::from_rgb(255, 224, 150));
}

/// One row of the gear's panel: a name, a line under it and a switch; true when it is clicked.
fn toggle_row(ui: &mut Ui, p: &Painter, r: Rect, name: &str, sub: &str, on: bool, lay: &Lay, id: &str) -> bool {
    let resp = ui.interact(r, ui.id().with(id), egui::Sense::click());
    let hot = resp.hovered();
    if hot {
        p.rect_filled(r, 8.0, Color32::from_rgba_unmultiplied(255, 120, 60, 22));
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    text(p, pos2(r.left() + 12.0, r.center().y - r.height() * 0.17), Align2::LEFT_CENTER, name, lay.s(17.0, 13.0), SAND);
    text(p, pos2(r.left() + 12.0, r.center().y + r.height() * 0.23), Align2::LEFT_CENTER, sub, lay.s(12.0, 10.0), DIM);
    // the switch
    let (sw, sh) = (r.height() * 0.95, r.height() * 0.5);
    let track = Rect::from_center_size(pos2(r.right() - 12.0 - sw / 2.0, r.center().y), vec2(sw, sh));
    let (fill, line) = if on { (Color32::from_rgb(196, 60, 18), Color32::from_rgb(255, 150, 70)) } else { (Color32::from_rgb(34, 17, 15), Color32::from_rgb(100, 56, 42)) };
    p.rect_filled(track, sh / 2.0, fill);
    p.rect_stroke(track, sh / 2.0, Stroke::new(1.0, line), StrokeKind::Inside);
    let kx = if on { track.right() - sh / 2.0 } else { track.left() + sh / 2.0 };
    p.circle_filled(pos2(kx, track.center().y), sh * 0.36, if on { Color32::from_rgb(255, 226, 170) } else { Color32::from_rgb(150, 124, 108) });
    resp.clicked()
}

/// A soft pool of light on `at` with the room dimmed round it (`a` is how much of it has come in, 0 to 1).
fn spotlight(p: &Painter, lay: &Lay, at: Pos2, a: f32, t: f32) {
    let k = lay.k;
    let breathe = 1.0 + 0.03 * (t * 0.9).sin();
    let (r0, r1) = (190.0 * k * breathe, 540.0 * k * breathe);
    let far = (lay.rect.width() + lay.rect.height()) * 2.0;
    const N: usize = 48;
    let (dark, clear) = (Color32::from_rgba_unmultiplied(5, 2, 8, (128.0 * a) as u8), Color32::from_rgba_unmultiplied(5, 2, 8, 0));
    let mut m = Mesh::default();
    for (rad, col) in [(r0, clear), (r1, dark), (far, dark)] {
        for i in 0..N {
            let ang = i as f32 / N as f32 * std::f32::consts::TAU;
            m.colored_vertex(at + vec2(ang.cos(), ang.sin()) * rad, col);
        }
    }
    for ring in 0..2 {
        for i in 0..N {
            let a0 = (ring * N + i) as u32;
            let a1 = (ring * N + (i + 1) % N) as u32;
            let (b0, b1) = (a0 + N as u32, a1 + N as u32);
            m.add_triangle(a0, a1, b0);
            m.add_triangle(a1, b1, b0);
        }
    }
    p.add(Shape::mesh(m));
    // a faint warm light on them
    soft_ellipse(p, at, vec2(150.0 * k, 190.0 * k), [255, 214, 160], 0.07 * a);
}

/// What a Queen or a King says to you when you are first dealt one (three lines, each of two at most).
fn plea_lines(c: Card) -> (&'static str, [&'static str; 3]) {
    if cards::rank(c) == 11 {
        (
            "THE KING",
            [
                "...You can hear me.\nI am the King. Time keeps my soul in this card.",
                "Every hand you play, he feeds on us.\nPlay me well. Do not throw me away.",
                "Please. Help me.",
            ],
        )
    } else {
        (
            "THE QUEEN",
            [
                "...Stranger. Please, do not look away.\nI am the Queen. He sealed me in this silver.",
                "I feel the chain on my neck every time you fold.\nHold me. Play me. Break it.",
                "Please. Help me.",
            ],
        )
    }
}

const KING_OF_HEARTS: Card = 37;
const QUEEN_OF_DIAMONDS: Card = 23;
/// (not a card: the scene of the two of them together)
const DUET: Card = 255;

/// A small speech bubble over `card` (its tail points down at it), with a name in blue over what is said.
fn bubble(p: &Painter, card: Rect, name: &str, lines: &[&str], k: f32, a: f32, screen: Rect) {
    if a <= 0.01 {
        return;
    }
    let size = (16.0 * k).clamp(11.0, 19.0);
    let galleys: Vec<_> = lines.iter().map(|l| p.layout_no_wrap((*l).to_string(), FontId::proportional(size), faded(CREAM, a))).collect();
    let w = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max).max(size * 6.0);
    let line_h = size * 1.3;
    let (pad, name_h) = (size * 0.8, size * 1.2);
    let (bw, bh) = (w + pad * 2.0, pad * 2.0 + name_h + line_h * lines.len() as f32);
    let mut cx = card.center().x;
    cx = cx.clamp(screen.left() + bw / 2.0 + 8.0, screen.right() - bw / 2.0 - 8.0);
    let bottom = card.top() - size * 1.2;
    let r = Rect::from_min_size(pos2(cx - bw / 2.0, bottom - bh), vec2(bw, bh));
    let fill = Color32::from_rgba_unmultiplied(8, 12, 30, (232.0 * a) as u8);
    let line = Stroke::new(1.3, Color32::from_rgba_unmultiplied(150, 200, 255, (190.0 * a) as u8));
    // the tail
    let tx = card.center().x.clamp(r.left() + size * 1.4, r.right() - size * 1.4);
    p.add(Shape::convex_polygon(vec![pos2(tx - size * 0.5, r.bottom() - 1.0), pos2(tx + size * 0.5, r.bottom() - 1.0), pos2(tx, r.bottom() + size * 1.1)], fill, line));
    p.rect_filled(r, size * 0.7, fill);
    p.rect_stroke(r, size * 0.7, line, StrokeKind::Inside);
    p.rect_filled(Rect::from_center_size(pos2(tx, r.bottom() - 1.0), vec2(size, 3.0)), 0.0, fill);
    text(p, pos2(r.left() + pad, r.top() + pad + name_h * 0.4), Align2::LEFT_CENTER, name, size * 0.72, faded(Color32::from_rgb(160, 208, 255), a));
    for (i, g) in galleys.into_iter().enumerate() {
        p.galley(pos2(r.left() + pad, r.top() + pad + name_h + line_h * i as f32), g, faded(CREAM, a));
    }
}

/// The Queen of Diamonds and the King of Hearts are the two monarchs who want out (and may be freed, one day).
fn begs(c: Card) -> bool {
    c == 23 || c == 37
}

/// What a card says when you click it and it is not one that begs (a Queen or a King who likes it in there,
/// or a Jack): the name to show, and the line (`n` takes a different one each time).
fn remark(c: Card, n: usize) -> (&'static str, &'static str) {
    let (suit, rank) = (cards::suit(c), cards::rank(c));
    let (name, lines): (&str, &[&'static str]) = match (rank, suit) {
        (10, 3) => ("Queen of Spades", &["Free me!? I love being in chains honey...", "Careful with that lock, sugar. I picked it out.", "Darling, you could not afford me out of here."]),
        (10, 0) => ("Queen of Clubs", &["Unchain me? I just got the cell the way I like it.", "Oh, a visitor. Mind the shackles, they are a conversation piece.", "I would curtsy, but, well. Chains."]),
        (10, _) => ("Queen of Hearts", &["Free me, and go where? The whole kingdom is a card game.", "Do not pull on that, dear, it is my favourite chain.", "I am wonderful company. Ask anyone. They are all in cards too."]),
        (11, 3) => ("King of Spades", &["Free me? And give up my corner seat? Never.", "I have been bound for centuries. I find it rather relaxing.", "Leave the chains. They go with the crown."]),
        (11, 0) => ("King of Clubs", &["Release me? I have not finished sulking.", "I outranked gods once. Now I outrank a poker table. Lovely.", "Chains? What chains. It is a bold fashion statement."]),
        (11, _) => ("King of Diamonds", &["Free me, and I would have to do something about my life.", "Go on, press the lock. I will wait. I have nothing but time.", "I could escape, but I would miss the company. You are good company."]),
        _ => ("The Jack", &["I am not chained. I am just a man with a sword and no plans.", "The Jack nods at you. He is the only one here with a day job.", "Could you stop touching me? I am not for sale."]),
    };
    (name, lines[n % lines.len()])
}

/// What you are told when you stare at the monarch chained in your hand and nothing happens.
const GAZES: [&str; 16] = [
    "Your intense gaze falls flat of freedom.",
    "You glare at the chain. The chain does not care.",
    "You squint harder. Somewhere, a link yawns.",
    "You will the silver to open. It stays silver.",
    "You poke the card. It is a card. They are still in it.",
    "Your stare could not free a fly from a web.",
    "You whisper 'hang in there'. They do, because they have to.",
    "A heroic pose, a firm jaw, and absolutely no effect.",
    "You offer a pep talk. The chains offer nothing back.",
    "You blow on the card. Nothing melts. Not even the dust.",
    "You consider tearing the card. You consider that this is a card game. You stop.",
    "You are one mortal holding one prisoner. Both of you are helpless.",
    "You try the look that works on your dog. It does not work on iron.",
    "They look back at you with an expression you can only call 'a long wait'.",
    "You press a thumb to the lock. The lock is not impressed.",
    "Not you, not yet. Whatever frees them is not in this hand.",
];

/// What the developer rows of the gear can make someone do.
#[derive(Clone, Copy)]
enum Dev {
    Bow,
    Look(usize, Look),
    Think(usize),
    Puff,
    BeardOff,
    BeardOn,
    Appear,
    Plea(Card),
}

/// The animations list of the developer rows: a name for each person (blank for a second row of theirs), and a
/// button for each thing they do.
const DEV_ROWS: &[(&str, &[(&str, Dev)])] = &[
    ("Mammon", &[("Bow", Dev::Bow)]),
    ("King Cat", &[("Cackle", Dev::Look(CAT, Look::Triumphant)), ("Furious", Dev::Look(CAT, Look::Furious)), ("Think", Dev::Think(CAT))]),
    ("Demon Bear", &[("Roar", Dev::Look(BEAR, Look::Furious)), ("Crow", Dev::Look(BEAR, Look::Triumphant)), ("Think", Dev::Think(BEAR))]),
    ("Father Time", &[("Smoke", Dev::Puff), ("Beard off", Dev::BeardOff), ("Beard on", Dev::BeardOn)]),
    ("", &[("Quiver", Dev::Look(FATHER, Look::Rattled)), ("Think", Dev::Think(FATHER))]),
    ("Shadow", &[("Appear", Dev::Appear), ("2nd face", Dev::Look(MONARCH, Look::Triumphant)), ("Think", Dev::Think(MONARCH))]),
    ("Cards", &[("King plea", Dev::Plea(37)), ("Queen plea", Dev::Plea(23))]),
];

/// One row of the gear's panel with a button instead of a switch; true when it is pressed.
fn action_row(ui: &mut Ui, p: &Painter, r: Rect, name: &str, sub: &str, label: &str, enabled: bool, lay: &Lay, id: &str) -> bool {
    let resp = ui.interact(r, ui.id().with(id), egui::Sense::click());
    let hot = resp.hovered() && enabled;
    if hot {
        p.rect_filled(r, 8.0, Color32::from_rgba_unmultiplied(255, 120, 60, 22));
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let ink = if enabled { SAND } else { DIM.gamma_multiply(0.6) };
    text(p, pos2(r.left() + 12.0, r.center().y - r.height() * 0.17), Align2::LEFT_CENTER, name, lay.s(17.0, 13.0), ink);
    text(p, pos2(r.left() + 12.0, r.center().y + r.height() * 0.23), Align2::LEFT_CENTER, sub, lay.s(12.0, 10.0), DIM);
    let b = Rect::from_center_size(pos2(r.right() - 12.0 - lay.s(34.0, 26.0), r.center().y), vec2(lay.s(68.0, 52.0), r.height() * 0.5));
    let (fill, line) = if enabled { (Color32::from_rgb(196, 60, 18), Color32::from_rgb(255, 150, 70)) } else { (Color32::from_rgb(34, 17, 15), Color32::from_rgb(100, 56, 42)) };
    p.rect_filled(b, b.height() / 2.0, fill);
    p.rect_stroke(b, b.height() / 2.0, Stroke::new(1.0, line), StrokeKind::Inside);
    text(p, b.center(), Align2::CENTER_CENTER, label, lay.s(12.0, 10.0), if enabled { Color32::from_rgb(255, 226, 170) } else { Color32::from_rgb(150, 124, 108) });
    resp.clicked() && enabled
}

impl Run {
    pub fn new(audio: Option<Audio>) -> Run {
        let s = Session::new();
        let seed_edit = s.client_seed.clone();
        Run {
            s,
            born: None,
            raise: 0.0,
            was_my_turn: false,
            seed_edit,
            dir: Director::new(),
            frame: Frame { zoom: 1.0, x: 550.0, y: 350.0, shake: (0.0, 0.0), squint: 0.0, blow: None },
            audio,
            dealing: Dealing::new(),
            card_n: 0,
            stack_born: None,
            stack_reveal: 0.0,
            chips_paid: 0,
            cued: None,
            king_laugh: None,
            father_off: None,
            father_back: None,
            father_was_off: false,
            monarch_in: None,
            bow_at: -10.0,
            last_hand_no: 0,
            looks: [Look::Calm; PLAYERS],
            mood: (Mood::Seated, -100.0),
            confirm_leave: false,
            fair_open: false,
            opts: TableOptions::default(),
            opts_changed: false,
            together: false,
            pending_together: false,
            gear_open: false,
            gear_a: 0.0,
            auto_wait: 0.0,
            auto_phase: None,
            skip_seen: (0, usize::MAX),
            smoke_owed: false,
            wait_on: None,
            think_seat: None,
            think: 0.0,
            chip_face: None,
            chip_face_tried: false,
            plea: None,
            seen_king: false,
            seen_queen: false,
            plea_queue: Vec::new(),
            plea_line_at: 0.0,
            plea_hand: 0,
            gaze: None,
            gaze_n: 0,
            bursts: Vec::new(),
            quip: None,
            dev_open: false,
            dev_shadow: None,
            dev_look: None,
            dev_think: None,
            dev_arrival: false,
            think_end: -10.0,
            result: None,
        }
    }

    /// Plays the game a frame and draws it.
    pub fn show(&mut self, ui: &mut Ui) -> Event {
        let ctx = ui.ctx().clone();
        let avail = ui.available_size();
        let (rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
        let now = ctx.input(|i| i.time);
        let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.1);
        let t = (now % 3600.0) as f32;
        self.born.get_or_insert(now);
        let mut event = Event::None;

        let (enter, esc) = ctx.input(|i| {
            let mut enter = false;
            let mut esc = false;
            for e in &i.events {
                if let egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } = e {
                    if modifiers.ctrl || modifiers.alt || modifiers.command {
                        continue;
                    }
                    match *key {
                        egui::Key::Space | egui::Key::Enter => enter = true,
                        egui::Key::Escape => esc = true,
                        _ => {}
                    }
                }
            }
            (enter, esc)
        });

        if !self.chip_face_tried {
            self.chip_face_tried = true;
            self.chip_face = poker_chip_art::load_face(&ctx);
        }
        poker_card_art::use_context(&ctx);
        self.s.tick(dt, 0.01);
        if let Some(f) = self.s.take_finish() {
            self.result = Some(f);
            event = Event::Finished(f);
        }
        self.deal_cards(dt);
        self.watch_plea(now);
        if std::mem::take(&mut self.dev_arrival) && self.s.dev_arrival() {
            self.show_shadow(now);
        }
        self.auto_deal(dt);
        self.watch_faces(now);
        self.steer_camera(now, dt);
        self.chips_arrive(now);
        self.chips_sound();

        // `base` is the window as laid out; `lay` is the same through the camera (turned, leaned in, shaking)
        let base = Lay::new(rect);
        let lay = Lay::with_cam(rect, &self.frame);
        let painter = ui.painter().with_clip_rect(rect);
        self.paint_room(&painter, &lay, t);
        self.paint_people(&painter, &ctx, &lay, now, t);
        self.paint_thought(&painter, &lay, t);
        self.paint_table(&painter, &lay, &base, t);
        self.paint_plates(&painter, &lay, t);
        self.paint_dealer(&painter, &lay, t);
        // Father Time's smoke comes at you, and your eyes shut
        if let Some(age) = self.frame.blow {
            let mouth = poker_art::father_mouth(lay.p(SEAT_X[FATHER], 232.0), 0.30 * lay.k);
            poker_art::paint_blow(&painter, mouth, rect, age);
        }
        paint_lids(&painter, rect, self.frame.squint);
        self.touch_cards(ui, &base, &lay, now);
        let plea_open = self.plea.is_some();
        self.paint_plea(ui, &painter, rect, &base, now, t, enter);
        self.paint_hud(&painter, &base);
        self.paint_words(&painter, &base);

        // a click anywhere reads the next line of the story
        if self.s.phase == Phase::Story {
            let resp = ui.interact(rect, ui.id().with("poker_story"), egui::Sense::click());
            if resp.clicked() || enter {
                self.s.next_line();
            }
        }

        let quit = self.controls(ui, &painter, &base, enter && !plea_open);
        if esc {
            if self.gear_open {
                self.gear_open = false;
            } else if self.confirm_leave {
                self.confirm_leave = false;
            } else if self.s.phase == Phase::Over {
                return Event::Quit;
            } else {
                self.confirm_leave = true;
            }
        }
        self.windows(&ctx);
        ctx.request_repaint();
        if quit {
            return Event::Quit;
        }
        event
    }

    /// The first Queen or King dealt to you (once a game) makes the table stop: see `paint_plea`.
    fn watch_plea(&mut self, now: f64) {
        // (with the developer switch on, they beg again every hand)
        if self.s.dev_monarchs && self.s.t.table.hand_no != self.plea_hand {
            self.seen_king = false;
            self.seen_queen = false;
        }
        // the trophy comes once the King and Queen have been heard out and are set back down on the table
        if self.plea.is_none() && std::mem::take(&mut self.pending_together) {
            self.together = true;
        }
        if self.plea.is_none() {
            if self.s.phase != Phase::Playing || self.dealing.busy() || self.s.is_watching() || self.s.fast || self.confirm_leave {
                return;
            }
            if self.s.t.table.seats.get(YOU).is_some_and(|st| st.folded) {
                return;
            }
            let Some(hole) = self.s.your_hole() else { return };
            let (has_king, has_queen) = (hole.contains(&KING_OF_HEARTS), hole.contains(&QUEEN_OF_DIAMONDS));
            let (king, queen) = (has_king && !self.seen_king, has_queen && !self.seen_queen);
            if !has_king && !has_queen {
                return;
            }
            // (once the cards are turned up in front of you)
            if !hole.iter().enumerate().all(|(i, _)| self.age_of(Put::Hole(YOU, i as u8)).is_some_and(|a| a > 0.9)) {
                return;
            }
            // both in your hand is the trophy, whether or not they have begged before, once their scenes are over
            // (the developer switch's rigged hand counts too, so the trophy and its toast can be tested)
            if has_king && has_queen {
                self.pending_together = true;
            }
            if !king && !queen {
                return;
            }
            // each one who has not begged yet begs, one after the other; two together for the first time then speak to each other
            let mut scenes: Vec<Card> = Vec::new();
            if king {
                scenes.push(KING_OF_HEARTS);
            }
            if queen {
                scenes.push(QUEEN_OF_DIAMONDS);
            }
            if king && queen {
                scenes.push(DUET);
            }
            self.seen_king |= king;
            self.seen_queen |= queen;
            self.plea_hand = self.s.t.table.hand_no;
            scenes.reverse();
            let first = scenes.pop();
            self.plea_queue = scenes;
            self.plea = first.map(|c| (c, now, 0));
            self.plea_line_at = now;
        }
        // the game waits for you to hear it out
        self.s.hold = true;
    }

    /// The monarch in the card, chained, begging: the card comes up big in the middle of a darkened room,
    /// shaking in its chains, with what they say under it. A click (or space) goes on to the next line.
    fn paint_plea(&mut self, ui: &mut Ui, p: &Painter, rect: Rect, base: &Lay, now: f64, t: f32, enter: bool) {
        let Some((c, since, line)) = self.plea else { return };
        if c == DUET {
            self.paint_duet(ui, p, rect, base, now, t, enter, since, line);
            return;
        }
        let age = (now - since) as f32;
        let a = (age / 0.6).clamp(0.0, 1.0);
        let e = a * a * (3.0 - 2.0 * a);
        let resp = ui.interact(rect, ui.id().with("poker_plea"), egui::Sense::click());
        p.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(3, 5, 14, (205.0 * e) as u8));
        let centre = pos2(rect.center().x, rect.top() + rect.height() * 0.40);
        let ch = rect.height() * 0.58 * (0.5 + 0.5 * e);
        let cw = ch * 384.0 / 537.0;
        let shake = (1.2 + 2.4 * (t * 1.9).sin().abs()) * e * base.k;
        let tremble = vec2((t * 43.0).sin(), (t * 37.0).cos()) * shake;
        // the soul's light round the card, breathing
        let pulse = 0.85 + 0.15 * (t * 2.1).sin();
        soft_ellipse(p, centre, vec2(cw * 1.5, ch * 0.95) * pulse, SOUL_DEEP, 0.42 * e);
        soft_ellipse(p, centre, vec2(cw * 0.9, ch * 0.6) * pulse, SOUL_BRIGHT, 0.20 * e);
        let r = Rect::from_center_size(centre + tremble, vec2(cw, ch));
        card_face(p, r, c, true);
        // the chains pulled tight from the corners of the room to the corners of the card
        let clip = rect;
        let ck = base.k * 1.5;
        for (from, to) in [(rect.left_top(), r.left_top()), (rect.right_top(), r.right_top()), (rect.left_bottom(), r.left_bottom()), (rect.right_bottom(), r.right_bottom())] {
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let u = i as f32 / 24.0;
                    let sag = (u * std::f32::consts::PI).sin() * 6.0 * base.k * (t * 3.0 + u * 5.0).sin();
                    from + (to - from) * (u * e) + vec2(0.0, sag)
                })
                .collect();
            chain(p, &pts, ck, clip, t);
        }
        // what they say
        let (who, lines): (&str, Vec<&'static str>) = match self.gaze {
            Some(l) => ("YOU STARE", vec![l]),
            None => {
                let (w, l) = plea_lines(c);
                (w, l.to_vec())
            }
        };
        let last = lines.len() - 1;
        let line = line.min(last);
        let shown = ((age - 0.7) / 0.5).clamp(0.0, 1.0);
        if shown > 0.0 {
            let w = (rect.width() * 0.8).min(base.s(980.0, 700.0));
            let box_r = Rect::from_center_size(pos2(rect.center().x, rect.bottom() - rect.height() * 0.13), vec2(w, base.s(120.0, 90.0)));
            p.rect_filled(box_r, 12.0, Color32::from_rgba_unmultiplied(6, 8, 20, (225.0 * shown) as u8));
            p.rect_stroke(box_r, 12.0, Stroke::new(1.4, Color32::from_rgba_unmultiplied(150, 200, 255, (150.0 * shown) as u8)), StrokeKind::Inside);
            text(p, pos2(box_r.left() + 20.0, box_r.top() + 18.0), Align2::LEFT_CENTER, who, base.s(13.0, 10.0), faded(Color32::from_rgb(160, 208, 255), shown));
            let mut y = box_r.top() + base.s(48.0, 38.0);
            for l in lines[line].split('\n') {
                text(p, pos2(box_r.left() + 20.0, y), Align2::LEFT_CENTER, l, base.s(21.0, 16.0), faded(CREAM, shown));
                y += base.s(28.0, 22.0);
            }
            text(p, pos2(box_r.right() - 16.0, box_r.bottom() - 14.0), Align2::RIGHT_CENTER, if self.gaze.is_some() { "click to look away" } else if line >= last { "click to let them go on" } else { "click to go on" }, base.s(12.0, 9.0), faded(DIM, shown * (0.55 + 0.35 * (t * 4.0).sin().abs())));
        }
        if age > 0.9 && (resp.clicked() || enter) {
            if line >= last {
                self.gaze = None;
                self.plea = self.plea_queue.pop().map(|c| (c, now, 0));
                self.plea_line_at = now;
            } else {
                self.plea = Some((c, since, line + 1));
                self.plea_line_at = now;
            }
        }
    }

    /// The King of Hearts and the Queen of Diamonds, dealt to you together for the first time, side by side in the dark
    /// with a thread of light between them, and a word for each other (line 0 is his, line 1 is hers).
    fn paint_duet(&mut self, ui: &mut Ui, p: &Painter, rect: Rect, base: &Lay, now: f64, t: f32, enter: bool, since: f64, line: usize) {
        let age = (now - since) as f32;
        let a = (age / 0.6).clamp(0.0, 1.0);
        let e = a * a * (3.0 - 2.0 * a);
        let resp = ui.interact(rect, ui.id().with("poker_plea"), egui::Sense::click());
        p.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(3, 5, 14, (205.0 * e) as u8));
        let ch = rect.height() * 0.46 * (0.5 + 0.5 * e);
        let cw = ch * 384.0 / 537.0;
        let y = rect.top() + rect.height() * 0.46;
        let gap = cw * 0.55 * e + cw * 0.1;
        let shake = (1.0 + 2.0 * (t * 1.7).sin().abs()) * e * base.k;
        let trem = |ph: f32| vec2((t * 43.0 + ph).sin(), (t * 37.0 + ph).cos()) * shake;
        let kc = pos2(rect.center().x - cw / 2.0 - gap, y) + trem(0.0);
        let qc = pos2(rect.center().x + cw / 2.0 + gap, y) + trem(2.0);
        let (kr, qr) = (Rect::from_center_size(kc, vec2(cw, ch)), Rect::from_center_size(qc, vec2(cw, ch)));
        // their light, and a thread of it from one to the other
        let pulse = 0.85 + 0.15 * (t * 2.1).sin();
        for c in [kc, qc] {
            soft_ellipse(p, c, vec2(cw * 1.4, ch * 0.9) * pulse, SOUL_DEEP, 0.38 * e);
        }
        let (from, to) = (pos2(kr.right(), y), pos2(qr.left(), y));
        soft_ellipse(p, pos2((from.x + to.x) / 2.0, y), vec2((to.x - from.x).abs() * 0.9 + cw * 0.3, ch * 0.25) * pulse, SOUL_BRIGHT, 0.20 * e);
        for i in 0..14 {
            let u = i as f32 / 13.0;
            let lit = ((t * 1.6 - u * 3.0).sin() * 0.5 + 0.5).powi(2);
            let at = from + (to - from) * u + vec2(0.0, (u * 9.0 + t * 2.0).sin() * 5.0 * base.k);
            p.circle_filled(at, (2.0 + 2.5 * lit) * base.k, soul(SOUL_BRIGHT, (0.25 + 0.7 * lit) * e));
        }
        card_face(p, kr, KING_OF_HEARTS, true);
        card_face(p, qr, QUEEN_OF_DIAMONDS, true);
        // each in its own chains, to the near corners of the room
        let ck = base.k * 1.5;
        for (from, to) in [(rect.left_top(), kr.left_top()), (rect.left_bottom(), kr.left_bottom()), (rect.right_top(), qr.right_top()), (rect.right_bottom(), qr.right_bottom())] {
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let u = i as f32 / 24.0;
                    let sag = (u * std::f32::consts::PI).sin() * 6.0 * base.k * (t * 3.0 + u * 5.0).sin();
                    from + (to - from) * (u * e) + vec2(0.0, sag)
                })
                .collect();
            chain(p, &pts, ck, rect, t);
        }
        // the King speaks first and the Queen answers
        let said_king = ((age - 0.7) / 0.4).clamp(0.0, 1.0);
        let said_queen = if line >= 1 { ((now - self.plea_line_at) as f32 / 0.4).clamp(0.0, 1.0) } else { 0.0 };
        let dim_king = if line >= 1 { 0.7 } else { 1.0 };
        bubble(p, kr, "THE KING", &["I miss you my love..", "In time, we will be bound", "in each others light once again"], base.k, said_king * dim_king, rect);
        bubble(p, qr, "THE QUEEN", &["Maybe they can save us all?..."], base.k, said_queen, rect);
        let hint = base.s(12.0, 9.0);
        if said_king > 0.5 {
            text(p, pos2(rect.center().x, rect.bottom() - rect.height() * 0.08), Align2::CENTER_CENTER, if line >= 1 { "click to let them go on" } else { "click to go on" }, hint, faded(DIM, 0.55 + 0.35 * (t * 4.0).sin().abs()));
        }
        if age > 0.9 && (resp.clicked() || enter) {
            if line >= 1 {
                self.plea = self.plea_queue.pop().map(|c| (c, now, 0));
            } else {
                self.plea = Some((DUET, since, 1));
            }
            self.plea_line_at = now;
        }
    }

    /// The souls and the remark of a card that has been clicked (`key`: 0 and 1 your cards, 10 to 14 the board).
    fn paint_card_clicks(&self, p: &Painter, key: u8, r: Rect, c: Card, t: f32, k: f32) {
        if let Some((_, since)) = self.bursts.iter().find(|(kk, _)| *kk == key) {
            let age = (t - since + 3600.0) % 3600.0;
            card_burst(p, r, c, age);
        }
        if let Some((kk, name, line, since)) = self.quip {
            let age = (t - since + 3600.0) % 3600.0;
            if kk == key && age < 4.2 {
                let a = 1.0 - ((age - 3.4) / 0.8).clamp(0.0, 1.0);
                let a = a * (age / 0.15).clamp(0.0, 1.0);
                pill(p, pos2(r.center().x, r.top() - 26.0 * k), &format!("{name}: \"{line}\""), (15.0 * k).clamp(11.0, 17.0), faded(CREAM, a), faded(INK, a * 0.9), faded(Color32::from_rgb(160, 208, 255), a * 0.7));
            }
        }
    }

    /// A face-up card has been clicked. A numbered card lets its souls out; a Queen or a King that wants out
    /// (see `begs`) has you stare at it, with the camera coming in on the card; one that does not says
    /// something unhelpful; so does a Jack.
    fn click_card(&mut self, key: u8, c: Card, now: f64) {
        let t = (now % 3600.0) as f32;
        self.gaze_n = (self.gaze_n + 1 + (now * 7.0) as usize % 3) % 48;
        if poker_card_art::has_wells(c as usize) {
            self.bursts.retain(|(k, _)| *k != key);
            self.bursts.push((key, t));
        } else if begs(c) {
            self.gaze = Some(GAZES[self.gaze_n % GAZES.len()]);
            self.plea = Some((c, now, 0));
        } else {
            let (name, line) = remark(c, self.gaze_n);
            self.quip = Some((key, name, line, t));
        }
    }

    /// Where your two hole cards are on the screen (your cards are held close, so they do not move with the camera).
    fn hole_rects(base: &Lay) -> [Rect; 2] {
        let bk = base.k;
        let (w, h) = (96.0 * bk, 136.0 * bk);
        let y = base.p(0.0, 548.0).y;
        std::array::from_fn(|i| {
            let x = base.p(550.0, 0.0).x + if i == 0 { -w - 7.0 * bk } else { 7.0 * bk };
            Rect::from_min_size(pos2(x, y + if i == 0 { 0.0 } else { 4.0 * bk }), vec2(w, h))
        })
    }

    /// Your cards and the board's can be clicked (see `click_card`).
    fn touch_cards(&mut self, ui: &mut Ui, base: &Lay, lay: &Lay, now: f64) {
        if self.plea.is_some() || !matches!(self.s.phase, Phase::Playing | Phase::Showdown) || self.dealing.busy() || self.confirm_leave {
            return;
        }
        let mut cards_on_screen: Vec<(u8, Rect, Card)> = Vec::new();
        if let Some(hole) = self.s.your_hole() {
            let folded = self.s.t.table.seats.get(YOU).is_some_and(|st| st.folded);
            let rects = Self::hole_rects(base);
            for (i, c) in hole.iter().enumerate() {
                if !folded && self.age_of(Put::Hole(YOU, i as u8)).is_some_and(|a| a > 0.9) {
                    cards_on_screen.push((i as u8, rects[i], *c));
                }
            }
        }
        let k = lay.k;
        let (cw, ch, gap) = (70.0 * k, 98.0 * k, 12.0 * k);
        let x0 = lay.p(550.0, 0.0).x - (5.0 * cw + 4.0 * gap) / 2.0;
        let y0 = lay.p(0.0, 398.0).y;
        for (i, &c) in self.s.t.table.board.iter().enumerate().take(5) {
            if self.age_of(Put::Board(i)).is_some_and(|a| a > 0.9) {
                cards_on_screen.push((10 + i as u8, Rect::from_min_size(pos2(x0 + i as f32 * (cw + gap), y0), vec2(cw, ch)), c));
            }
        }
        for (key, r, c) in cards_on_screen {
            let resp = ui.interact(r, ui.id().with(("poker_card", key)), egui::Sense::click());
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                self.click_card(key, c, now);
                return;
            }
        }
    }

    /// The Shadow Monarch is on the screen without having sat down (for the developer rows).
    fn show_shadow(&mut self, now: f64) {
        self.dev_shadow = Some(now);
        self.dir.react(Who::Monarch, 6.5, now);
    }

    /// Does one of the things in the developer rows' list.
    fn play_dev(&mut self, d: Dev, now: f64) {
        match d {
            Dev::Bow => self.bow_at = now,
            Dev::Look(seat, look) => {
                if seat == MONARCH {
                    self.show_shadow(now);
                }
                self.dev_look = Some((seat, look, now + 3.5));
            }
            Dev::Think(seat) => {
                if seat == MONARCH {
                    self.show_shadow(now);
                }
                self.dev_think = Some((seat, now + 5.0));
            }
            Dev::Puff => self.dir.puff(now),
            Dev::BeardOff => {
                self.father_off = Some(now);
                self.father_back = None;
                self.dir.react(Who::Father, 3.8, now);
            }
            Dev::BeardOn => {
                self.father_back = Some(now);
                self.father_off = None;
                self.dir.react(Who::Father, 3.0, now);
            }
            Dev::Appear => {
                self.dev_shadow = Some(now);
                self.dir.react(Who::Monarch, 6.5, now);
            }
            Dev::Plea(c) => self.plea = Some((c, now, 0)),
        }
    }

    /// Notices what the faces are doing (the King's cackle, Father Time's beard, the Bear's roar).
    fn watch_faces(&mut self, now: f64) {
        let mut looks = [Look::Calm; PLAYERS];
        for (i, l) in looks.iter_mut().enumerate() {
            *l = self.s.reactor.look(i);
        }
        // (the developer rows can make someone look a way for a few seconds)
        if let Some((seat, look, until)) = self.dev_look {
            if now < until {
                looks[seat] = look;
            } else {
                self.dev_look = None;
            }
        }
        if looks[CAT] == Look::Triumphant && self.looks[CAT] != Look::Triumphant {
            self.king_laugh = Some(now);
            self.dir.react(Who::Cat, 3.4, now);
        }
        if let Some(c) = self.king_laugh {
            if now - c >= f64::from(king::DEAD_REACTION) {
                self.king_laugh = None;
            }
        }
        let off = self.s.reactor.beard_off();
        if off && !self.father_was_off {
            self.father_off = Some(now);
            self.father_back = None;
            self.dir.react(Who::Father, 3.8, now);
        }
        if looks[FATHER] == Look::Rattled && self.looks[FATHER] != Look::Rattled {
            self.dir.react(Who::Father, 3.0, now);
        }
        if !off && self.father_was_off {
            self.father_back = Some(now);
            self.father_off = None;
        }
        self.father_was_off = off;
        if self.father_back.is_some_and(|b| now - b > 2.6) {
            self.father_back = None;
        }
        if looks[BEAR] != self.looks[BEAR] {
            let mood = match looks[BEAR] {
                Look::Triumphant => Mood::Joy,
                Look::Furious => Mood::Rage,
                _ => Mood::Seated,
            };
            self.mood = (mood, now);
            match mood {
                Mood::Rage => self.dir.roar(true, now),
                Mood::Joy => self.dir.roar(false, now),
                Mood::Seated => {}
            }
        }
        if self.s.t.monarch_in && self.monarch_in.is_none() {
            self.monarch_in = Some(now);
            self.dev_shadow = None;
            self.dir.react(Who::Monarch, 6.5, now);
        }
        let hn = self.s.t.table.hand_no;
        if hn != self.last_hand_no {
            self.last_hand_no = hn;
            self.bow_at = now;
        }
        self.looks = looks;
    }

    /// The camera turns to whoever is speaking, snaps onto the Bear when he roars, and lets go when it
    /// is your move (so that you can see the table).
    fn steer_camera(&mut self, now: f64, dt: f32) {
        let who = self.s.speaker_now().and_then(|w| match w {
            Speaker::Dealer => Some(Who::Dealer),
            Speaker::Bear => Some(Who::Bear),
            Speaker::Cat => Some(Who::Cat),
            Speaker::Father => Some(Who::Father),
            Speaker::Monarch => Some(Who::Monarch),
            Speaker::Stage => None,
        });
        self.dir.set_auto_blow(self.s.phase != Phase::Story);
        // when the game is waiting for one of the others to make up their mind, the camera is on them (once
        // it has been a moment, so that a quick fold does not swing it about), a little less close than when
        // someone is talking, and their face works at it. A relaxed table (the default) takes its time over all
        // of it: the others think longer, the camera glides slowly, comes only when someone really is slow, and
        // a soft spotlight finds them. A quick table (for players who know the game) leaves the camera alone.
        let quick = self.opts.quick_table;
        self.s.pace = if quick { 1.0 } else { 1.5 };
        self.dir.set_ease(if quick { 2.4 } else { 1.25 });
        let dev = self.dev_think.filter(|&(_, until)| now < until).map(|(s, _)| s);
        let real = if self.s.phase == Phase::Playing && !self.s.hold && !self.s.is_watching() && !self.s.fast { self.s.t.table.to_act.filter(|&s| s != YOU) } else { None };
        let waiting = dev.or(real);
        match (waiting, self.wait_on) {
            (Some(s), Some((w, _))) if s == w => {}
            (Some(s), _) => self.wait_on = Some((s, now)),
            (None, _) => self.wait_on = None,
        }
        let recent = now - self.think_end < 1.2;
        let delay = if dev.is_some() {
            0.2
        } else if quick {
            0.3
        } else if recent {
            1.7
        } else {
            0.8
        };
        let was = self.think_seat;
        self.think_seat = self.wait_on.filter(|&(_, since)| now - since > delay).map(|(s, _)| s);
        if was.is_some() && self.think_seat.is_none() {
            self.think_end = now;
        }
        let rate = if self.think_seat.is_some() { 2.2 } else { -2.2 };
        self.think = (self.think + rate * dt).clamp(0.0, 1.0);
        let moves = !quick || dev.is_some();
        let who = match self.think_seat.and_then(Who::of_seat).filter(|_| moves) {
            Some(w) => {
                self.dir.set_zoom_k(if quick { 0.82 } else { 0.78 });
                Some(w)
            }
            None => {
                self.dir.set_zoom_k(1.0);
                who
            }
        };
        // a line with a cue directs the camera itself
        if !self.s.line_now().is_some_and(|l| l.cue.moves_camera()) {
            self.dir.speak(who, now);
        }
        // what the story makes the camera do: flinch from the Bear, take a breath of Father Time's smoke
        match self.s.line_now() {
            Some(line) if self.cued != Some(line.text) => {
                // the smoke comes once the line that owes it has been read and clicked past
                if std::mem::take(&mut self.smoke_owed) {
                    self.dir.puff(now);
                }
                self.cued = Some(line.text);
                match line.cue {
                    // (the story goes on to Father Time and his jokes)
                    Cue::Flinch => self.dir.startle(now, Some(Who::Father)),
                    Cue::Blow => self.smoke_owed = true,
                    Cue::None => {}
                }
            }
            Some(_) => {}
            None => {
                // (skipping the story skips the smoke)
                self.smoke_owed = false;
                self.cued = None;
            }
        }
        let calm = self.s.your_turn().is_some();
        self.frame = self.dir.frame(dt, now, calm);
    }

    // ---- the dealer's hand -------------------------------------------------------------------------

    /// True when the cards are simply there (no hand puts them down): between hands, and when the game
    /// is being played through without you.
    fn instant(&self) -> bool {
        self.s.phase != Phase::Playing || self.s.is_watching() || self.s.fast || self.opts.skip_dealing
    }

    /// With "Deal automatically" on: deals the first hand once the chips are down, and the next one a
    /// moment after a hand is over (long enough to see how it came out).
    fn auto_deal(&mut self, dt: f32) {
        let phase = self.s.phase;
        if self.auto_phase != Some(phase) {
            self.auto_phase = Some(phase);
            self.auto_wait = 0.0;
        }
        let ready = match phase {
            Phase::Ready => self.stack_reveal >= 1.0 && self.frame.zoom < 1.03,
            Phase::Showdown => true,
            _ => false,
        };
        if !self.opts.auto_deal || !ready || self.confirm_leave || self.fair_open || self.s.is_watching() || self.dealing.busy() {
            self.auto_wait = 0.0;
            return;
        }
        self.auto_wait += dt;
        let wait = if phase == Phase::Ready { 0.7 } else { 3.2 };
        if self.auto_wait >= wait {
            self.auto_wait = 0.0;
            if phase == Phase::Ready {
                self.s.deal();
            } else {
                self.s.deal_next();
            }
        }
    }

    /// How long ago a card was put down (None if it has not been yet).
    fn age_of(&self, c: Put) -> Option<f32> {
        if self.instant() {
            Some(99.0)
        } else {
            self.dealing.age(c)
        }
    }

    /// Starts the dealing when a hand or a street begins, plays its sounds, and holds the game while a
    /// card is still on its way.
    fn deal_cards(&mut self, dt: f32) {
        let tb = &self.s.t.table;
        let dealt: [bool; deal::SEATS] = std::array::from_fn(|i| tb.seats.get(i).is_some_and(|st| st.in_hand && st.hole.is_some()));
        let (hand_no, board) = (tb.hand_no, tb.board.len());
        if self.instant() {
            self.dealing.settle(hand_no, dealt, board);
        } else {
            if self.dealing.hand_no() != hand_no {
                self.dealing.begin_hand(hand_no, dealt);
            }
            self.dealing.street(board);
        }
        // skipping the dealing: the cards are simply there, with the sound of the last one put down
        if self.opts.skip_dealing && self.s.phase == Phase::Playing && !self.s.is_watching() && !self.s.fast && self.skip_seen != (hand_no, board) {
            self.skip_seen = (hand_no, board);
            if let Some(a) = &self.audio {
                self.card_n += 1;
                sfx::play(a, Sfx::Card(self.card_n));
            }
        }
        let beats = self.dealing.tick(dt);
        if let Some(a) = &self.audio {
            for b in beats {
                match b {
                    Beat::Shuffle => sfx::play(a, Sfx::Shuffle),
                    Beat::Land(_) => {
                        self.card_n += 1;
                        sfx::play(a, Sfx::Card(self.card_n));
                    }
                }
            }
        }
        self.s.hold = !self.instant() && self.dealing.busy();
    }

    /// Where Mammon's wrist is, and so the dock for the hand (its palm sits just beyond).
    fn dock(&self, lay: &Lay) -> Pos2 {
        poker_art::mammon_dock(lay.p(DEALER_X, EDGE_Y), 0.34 * lay.k)
    }

    /// The middle of the hand for a pose: on the way from one place to the next, rising over the felt.
    fn hand_pos(&self, lay: &Lay, pose: &Pose) -> Pos2 {
        let dock = self.dock(lay) + vec2(0.0, 26.0 * 0.38 * lay.k);
        let at = |s: Spot| match s {
            Spot::Dock => dock,
            other => {
                let (x, y) = deal::spot_xy(other);
                lay.p(x, y)
            }
        };
        let (a, b) = (at(pose.from), at(pose.to));
        let arc = if pose.from == pose.to { 0.0 } else { (std::f32::consts::PI * pose.f).sin() * 34.0 * lay.k };
        a + (b - a) * pose.f + vec2(0.0, -arc)
    }

    /// How far the hand is from Mammon's wrist, 0 (on it) to 1 (gone).
    fn hand_away(&self, lay: &Lay) -> f32 {
        let dock = self.dock(lay) + vec2(0.0, 26.0 * 0.38 * lay.k);
        ((self.hand_pos(lay, &self.dealing.hand()) - dock).length() / (24.0 * lay.k)).clamp(0.0, 1.0)
    }

    /// The robotic hand, the cards sliding over to the people they were dealt to, and the smoke of its
    /// rocket.
    fn paint_dealer(&self, p: &Painter, lay: &Lay, t: f32) {
        let k = lay.k;
        // the cards just dealt slide over to their people
        for seat in 0..deal::SEATS {
            for n in 0..2u8 {
                let Some(age) = self.dealing.age(Put::Hole(seat, n)).filter(|_| !self.instant()) else { continue };
                if age > 0.45 {
                    continue;
                }
                let f = (age / 0.45).clamp(0.0, 1.0);
                let f = f * f * (3.0 - 2.0 * f);
                let (sx, sy) = deal::spot_xy(Spot::Seat(seat));
                let dest = if seat == YOU { (sx, sy + 90.0) } else { (PLATE_X[seat], PLATE_Y + 6.0) };
                let c = lay.p(sx + (dest.0 - sx) * f + n as f32 * 9.0, sy + (dest.1 - sy) * f);
                let size = vec2(34.0 * k, 48.0 * k) * (1.0 - 0.55 * f * if seat == YOU { -1.0 } else { 1.0 });
                card_back(p, Rect::from_center_size(c, size), true);
            }
        }
        let pose = self.dealing.hand();
        let centre = self.hand_pos(lay, &pose);
        let dock_centre = self.dock(lay) + vec2(0.0, 26.0 * 0.38 * k);
        let away = ((centre - dock_centre).length() / (24.0 * k)).clamp(0.0, 1.0);
        // the rocket's trail: puffs of hot smoke along where the hand has just been
        for i in 1..=10 {
            let back = self.dealing.hand_back(0.035 * i as f32);
            if back.thrust < 0.05 {
                continue;
            }
            let q = self.hand_pos(lay, &back);
            let fade = 1.0 - i as f32 / 11.0;
            let r = (7.0 + 3.0 * i as f32) * k;
            soft_ellipse(p, q, vec2(r, r), [255, 150, 70], 0.30 * back.thrust * fade);
            soft_ellipse(p, q, vec2(r * 1.6, r * 1.6), [70, 56, 56], 0.20 * back.thrust * fade);
        }
        // which way the fingers point: along its flight, and down at the table once it is still
        let before = self.hand_pos(lay, &self.dealing.hand_back(0.05));
        let v = centre - before;
        let w = (v.length() / (12.0 * k)).clamp(0.0, 1.0);
        let down = vec2(0.0, 1.0);
        let dir = if v.length() > 0.01 { down * (1.0 - w) + (v / v.length()) * w } else { down };
        let dir = if dir.length() > 0.01 { dir / dir.length() } else { down };
        let s = k * (0.38 + 0.52 * away);
        // the card in its fingers
        if pose.carrying {
            let c = centre + dir * (30.0 * s);
            card_back(p, Rect::from_center_size(c, vec2(40.0 * k, 56.0 * k) * (0.7 + 0.3 * away)), true);
        }
        poker_art::paint_robot_hand(p, centre, dir, s, pose.thrust, t, if pose.carrying { 1.0 } else { 0.0 });
    }

    /// Your chips are put down in front of you, with their sound, once the camera is all the way back and
    /// the first hand can be dealt.
    fn chips_arrive(&mut self, now: f64) {
        if self.stack_born.is_none() && matches!(self.s.phase, Phase::Ready | Phase::Playing | Phase::Showdown) && self.frame.zoom < 1.03 {
            self.stack_born = Some(now);
            if let Some(a) = &self.audio {
                sfx::play(a, Sfx::Chips);
            }
        }
        self.stack_reveal = self.stack_born.map_or(0.0, |b| ((now - b) / 0.85).clamp(0.0, 1.0) as f32);
    }

    /// The sound of chips when a pot is pushed to you (once for each hand).
    fn chips_sound(&mut self) {
        if !matches!(self.s.phase, Phase::Showdown | Phase::Story | Phase::Over) {
            return;
        }
        let hn = self.s.t.table.hand_no;
        if hn == 0 || self.chips_paid == hn {
            return;
        }
        let won = self.s.summary.as_ref().and_then(|sum| sum.won.get(YOU).copied()).unwrap_or(0);
        if won > 0 {
            self.chips_paid = hn;
            if let Some(a) = &self.audio {
                sfx::play(a, Sfx::Chips);
            }
        }
    }

    // ---- the room and the people -------------------------------------------------------------------

    fn paint_room(&self, p: &Painter, lay: &Lay, t: f32) {
        let r = lay.rect;
        // the hall: fine joinery between pillars of rock, a marble floor
        poker_room::paint(p, &poker_room::View { rect: r, ox: lay.ox, oy: lay.oy, k: lay.k }, t);
        // lava along the far edge of the table
        let band = Rect::from_min_max(pos2(r.left(), lay.p(0.0, EDGE_Y - 90.0).y), pos2(r.right(), lay.p(0.0, EDGE_Y).y));
        gradient_band(p, band, Color32::from_rgba_unmultiplied(255, 110, 40, 90), true);
        // the light over the table
        soft_ellipse(p, lay.p(550.0, 400.0), vec2(lay.s(430.0, 150.0), lay.s(150.0, 50.0)), [255, 170, 90], 0.10);
        // embers rising
        for i in 0..28u32 {
            let seed = i as f32 * 12.9898;
            let fx = (seed.sin() * 43758.547).fract().abs();
            let speed = 0.015 + 0.03 * (seed.cos() * 1234.5).fract().abs();
            let life = (t * speed + fx * 7.0).fract();
            let x = lay.p(1100.0 * fx + (t * 0.7 + seed).sin() * 12.0, 0.0).x;
            let y = lay.p(0.0, 330.0 - 330.0 * life).y;
            p.circle_filled(pos2(x, y), lay.s(1.8, 1.0), Color32::from_rgb(255, 140, 50).gamma_multiply((1.0 - life) * 0.8));
        }
    }

    fn is_out(&self, seat: usize) -> bool {
        let tb = &self.s.t.table;
        match tb.seats.get(seat) {
            Some(st) => st.stack == 0 && !(tb.running && st.in_hand),
            None => seat == MONARCH && !self.s.t.monarch_in && self.dev_shadow.is_none(),
        }
    }

    fn paint_people(&self, p: &Painter, ctx: &egui::Context, lay: &Lay, now: f64, t: f32) {
        let k = lay.k;
        let dim = |p: &Painter, r: Rect| p.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(8, 4, 5, 150));
        // the room goes dark while the void is open
        if let Some(at) = self.dev_shadow.or(self.monarch_in) {
            let d = poker_art::void_dim((now - at) as f32);
            if d > 0.0 {
                p.rect_filled(lay.rect, 0.0, Color32::from_rgba_unmultiplied(3, 0, 10, (120.0 * d) as u8));
            }
        }

        // the dead king, half out of the frame on the left
        let col = lay.column(-3000.0, 170.0);
        let pc = p.with_clip_rect(col.intersect(p.clip_rect()));
        let laugh = self.king_laugh.map(|c| (now - c) as f32);
        poker_art::paint_king(&pc, lay.p(SEAT_X[CAT], 262.0), 0.32 * k, t, laugh);
        if self.is_out(CAT) {
            dim(&pc, col);
        }

        // Mammon, who deals
        let col = lay.column(197.0, 333.0);
        let pc = p.with_clip_rect(col.intersect(p.clip_rect()));
        let since = (now - self.bow_at) as f32;
        let bow = if since < 0.6 { (since / 0.6 * std::f32::consts::PI).sin().max(0.0) } else { 0.0 };
        poker_art::paint_mammon(&pc, lay.p(DEALER_X, EDGE_Y), 0.34 * k, t, bow, self.hand_away(lay));

        // the Demon Bear, across the table
        let col = lay.column(320.0, 625.0);
        let pc = p.with_clip_rect(col.intersect(p.clip_rect()));
        let (mood, since) = (self.mood.0, (now - self.mood.1) as f32);
        let roar = since < 7.0;
        let bear = Bear {
            at: lay.p(SEAT_X[BEAR], EDGE_Y),
            scale: 0.36 * k,
            t,
            mood: if roar { mood } else { Mood::Seated },
            since,
            bleeding: self.s.reactor.bleeding().is_some(),
            think: if self.think_seat == Some(BEAR) { self.think } else { 0.0 },
        };
        poker_bear::paint(&pc, &bear);
        if self.is_out(BEAR) {
            dim(&pc, col);
        }

        // the Shadow Monarch, once he has come through the void
        if let (Some(art), Some(at)) = (monarch_art::art(ctx), self.dev_shadow.or(self.monarch_in)) {
            let col = lay.column(540.0, 900.0);
            let pc = p.with_clip_rect(col.intersect(p.clip_rect()));
            let age = (now - at) as f32;
            let second = matches!(self.looks[MONARCH], Look::Triumphant | Look::Furious);
            poker_art::paint_monarch(&pc, &art, lay.p(SEAT_X[MONARCH], 222.0), 118.0 * k, t, age, second);
            if self.is_out(MONARCH) {
                dim(&pc, col);
            }
        }

        // Father Time, half out of the frame on the right, smoking
        if let Some(tex) = poker_art::father_tex(ctx) {
            let col = lay.column(850.0, 3000.0);
            let pc = p.with_clip_rect(col.intersect(p.clip_rect()));
            let st = FatherState {
                beard_off: self.father_off.map(|c| (now - c) as f32),
                beard_back: self.father_back.map(|c| (now - c) as f32),
                rattled: self.looks[FATHER] == Look::Rattled,
                blow: self.frame.blow,
            };
            poker_art::paint_father(&pc, &tex, lay.p(SEAT_X[FATHER], 232.0), 0.30 * k, t, &st);
            if self.is_out(FATHER) {
                dim(&pc, col);
            }
        }
    }

    // ---- the table ---------------------------------------------------------------------------------

    fn paint_table(&self, p: &Painter, lay: &Lay, base: &Lay, t: f32) {
        let r = lay.rect;
        let k = lay.k;
        // everything on the table is laid out in board pixels and put through the camera, so it stays
        // where it is on the table when the camera moves (the chains and the cloth included)
        let (xl, xr) = ((r.left() - lay.ox) / k, (r.right() - lay.ox) / k);
        let curve = |xb: f32| EDGE_Y + 14.0 + ((xb - 550.0).abs() / 700.0).min(1.0).powi(5) * 90.0;
        let top = lay.p(0.0, EDGE_Y + 14.0).y;
        let centre = lay.p(550.0, 470.0);
        // the felt: blood red, lit in the middle and nearly black at the edges, its far corners rounded off
        let mut outline: Vec<Pos2> = Vec::new();
        let steps = 40;
        for i in 0..=steps {
            let xb = xl + (xr - xl) * i as f32 / steps as f32;
            outline.push(lay.p(xb, curve(xb)));
        }
        outline.push(r.right_bottom());
        outline.push(r.left_bottom());
        let mut m = Mesh::default();
        m.colored_vertex(centre, Color32::from_rgb(138, 28, 36));
        for q in &outline {
            let d = ((q.x - centre.x).abs() / (r.width() * 0.5)).min(1.0);
            let c = crate::theme::mix(Color32::from_rgb(98, 18, 26), Color32::from_rgb(40, 6, 10), d.max(0.2));
            m.colored_vertex(*q, c);
        }
        let n = outline.len() as u32;
        for i in 0..n {
            m.add_triangle(0, 1 + i, 1 + (i + 1) % n);
        }
        p.add(Shape::mesh(m));
        // the cloth: a quilt of dark diamonds, as faint as the weave of felt
        {
            let y0 = EDGE_Y + 14.0 + 32.0;
            let clip = Rect::from_min_max(pos2(r.left(), lay.p(0.0, y0).y), r.right_bottom());
            let q = p.with_clip_rect(clip.intersect(p.clip_rect()));
            let line = Stroke::new(1.0, Color32::from_rgba_unmultiplied(20, 0, 4, 46));
            let (first, last) = (((xl - 700.0) / 46.0).floor() as i32, (xr / 46.0).ceil() as i32);
            for i in first..=last {
                let xb = i as f32 * 46.0;
                q.line_segment([lay.p(xb, y0), lay.p(xb + 700.0, y0 + 700.0)], line);
                q.line_segment([lay.p(xb + 700.0, y0), lay.p(xb, y0 + 700.0)], line);
            }
        }
        // the betting line round the board, in ember
        let ring = lay.p(550.0, 452.0);
        p.add(Shape::ellipse_stroke(ring, vec2(292.0 * k, 92.0 * k), Stroke::new((1.4 * k).max(1.0), Color32::from_rgba_unmultiplied(232, 130, 70, 70))));
        p.add(Shape::ellipse_stroke(ring, vec2(300.0 * k, 98.0 * k), Stroke::new((0.8 * k).max(0.6), Color32::from_rgba_unmultiplied(232, 130, 70, 38))));
        // the wooden edge, with a thread of gold
        let rail: Vec<Pos2> = outline.iter().take(steps + 1).map(|q| *q + vec2(0.0, -7.0 * k)).collect();
        p.add(Shape::line(rail.clone(), Stroke::new(14.0 * k, Color32::from_rgb(34, 18, 13))));
        p.add(Shape::line(rail.iter().map(|q| *q + vec2(0.0, -5.0 * k)).collect(), Stroke::new((2.0 * k).max(1.0), Color32::from_rgb(92, 52, 34))));
        p.add(Shape::line(rail.iter().map(|q| *q + vec2(0.0, 1.0 * k)).collect(), Stroke::new((1.2 * k).max(0.8), Color32::from_rgb(168, 122, 58).gamma_multiply(0.7))));
        // shadow under the edge, onto the felt
        gradient_band(p, Rect::from_min_max(pos2(r.left(), top), pos2(r.right(), top + 40.0 * k)), Color32::from_black_alpha(140), false);
        // the chain that clamps the felt down along the inner edge of the rail: its links are counted from a
        // fixed point far off the left of the table, so each one stays where it is
        let inner: Vec<Pos2> = (0..=290)
            .map(|i| {
                let xb = -900.0 + 2900.0 * i as f32 / 290.0;
                lay.p(xb, curve(xb) + 9.0)
            })
            .collect();
        chain(p, &inner, k, r.expand(40.0), t);
        // a pool of light on the middle of the felt
        soft_ellipse(p, centre, vec2(lay.s(420.0, 140.0), lay.s(110.0, 40.0)), [255, 120, 90], 0.12 + 0.012 * (t * 0.7).sin());

        // the board
        let board = &self.s.t.table.board;
        let (cw, ch, gap) = (70.0 * k, 98.0 * k, 12.0 * k);
        let x0 = lay.p(550.0, 0.0).x - (5.0 * cw + 4.0 * gap) / 2.0;
        let y0 = lay.p(0.0, 398.0).y;
        for i in 0..5 {
            let cr = Rect::from_min_size(pos2(x0 + i as f32 * (cw + gap), y0), vec2(cw, ch));
            match (board.get(i), self.age_of(Put::Board(i))) {
                (Some(&c), Some(age)) => flip_card(p, cr, c, age),
                _ => {
                    p.rect_filled(cr, 7.0 * k, Color32::from_black_alpha(46));
                    p.rect_stroke(cr, 7.0 * k, Stroke::new(1.5, Color32::from_rgb(214, 120, 60).gamma_multiply(0.45)), StrokeKind::Inside);
                }
            }
        }
        // a soul now and then comes up out of a card
        for (i, &c) in board.iter().enumerate().take(5) {
            if self.age_of(Put::Board(i)).is_some_and(|a| a > 0.5) {
                let cr = Rect::from_min_size(pos2(x0 + i as f32 * (cw + gap), y0), vec2(cw, ch));
                card_soul(p, cr, c, t);
                self.paint_card_clicks(p, 10 + i as u8, cr, c, t, k);
            }
        }
        // the burnt cards, face down in a little pile beside the board
        for j in 0..3u8 {
            if let Some(age) = self.age_of(Put::Burn(j)) {
                let c = lay.p(deal::spot_xy(Spot::Burn).0, deal::spot_xy(Spot::Burn).1);
                let lift = (1.0 - (age / 0.18).clamp(0.0, 1.0)).powi(2) * 14.0 * k;
                let r = Rect::from_center_size(c + vec2(j as f32 * 2.2 * k, -(j as f32) * 1.6 * k - lift), vec2(52.0 * k, 73.0 * k));
                card_back(p, r, true);
            }
        }
        if self.s.t.table.running {
            text(p, lay.p(550.0, 387.0), Align2::CENTER_CENTER, self.s.t.table.street.label().to_uppercase(), lay.s(12.0, 10.0), SAND.gamma_multiply(0.75));
        }
        // the pot (or, once the hand is over, how it went)
        match (&self.s.summary, self.s.phase) {
            (Some(sum), Phase::Showdown | Phase::Story | Phase::Over) if !sum.lines.is_empty() => {
                let shown: Vec<&String> = sum.lines.iter().take(2).collect();
                let h = 20.0 * k * shown.len() as f32 + 10.0 * k;
                let w = 560.0 * k;
                let pr = Rect::from_center_size(pos2(lay.p(550.0, 0.0).x, lay.p(0.0, 504.0).y + h / 2.0), vec2(w, h));
                p.rect_filled(pr, 8.0, Color32::from_black_alpha(176));
                p.rect_stroke(pr, 8.0, Stroke::new(1.0, Color32::from_rgb(150, 70, 44)), StrokeKind::Inside);
                for (i, line) in shown.iter().enumerate() {
                    text(p, pos2(pr.center().x, pr.top() + 5.0 * k + 10.0 * k + i as f32 * 20.0 * k), Align2::CENTER_CENTER, line.as_str(), lay.s(16.0, 12.0), CREAM);
                }
            }
            _ => {
                let pot = self.s.t.table.pot();
                if pot > 0 || self.s.t.table.running {
                    text(p, lay.p(550.0, 508.0), Align2::CENTER_CENTER, format!("POT  {}", group(pot)), lay.s(20.0, 13.0), SAND);
                    // what is in the middle: everything put in before this round (this round's chips are still
                    // in front of whoever bet them)
                    let this_round: Chips = self.s.t.table.seats.iter().map(|s| s.bet).sum();
                    poker_chip_art::paint_towers(p, lay.p(550.0, 546.0), k * 0.9, pot.saturating_sub(this_round), 1.0, self.chip_face.as_ref().map(|t| t.id()));
                }
            }
        }
        // each of the others has his stack on the felt beside his nameplate, in towers of numbered chips
        for i in [CAT, BEAR, MONARCH, FATHER] {
            if i == MONARCH && !self.s.t.monarch_in {
                continue;
            }
            let Some(st) = self.s.t.table.seats.get(i) else { continue };
            if st.stack > 0 {
                poker_chip_art::paint_towers(p, lay.p(PLATE_X[i] - 78.0, 388.0), k * 0.72, st.stack, self.stack_reveal, self.chip_face.as_ref().map(|t| t.id()));
            }
        }
        // chips on the felt
        for i in 0..PLAYERS {
            let Some(st) = self.s.t.table.seats.get(i) else { continue };
            if st.bet > 0 {
                let at = if i == YOU { lay.p(436.0, 532.0) } else { lay.p(PLATE_X[i] + if i == BEAR { 60.0 } else { 0.0 }, 366.0) };
                let w = poker_chip_art::paint_towers(p, at, k * 0.8, st.bet, 1.0, self.chip_face.as_ref().map(|t| t.id()));
                text(p, at + vec2(w * 0.5 + 6.0 * k, -3.0 * k), Align2::LEFT_CENTER, group(st.bet), (14.0 * k).max(11.0), CREAM);
            }
        }
        // your cards, held close: they stay where they are when the camera turns away
        let (bk, bp) = (base.k, |x: f32, y: f32| base.p(x, y));
        if let Some(hole) = self.s.your_hole() {
            let folded = self.s.t.table.seats.get(YOU).is_some_and(|s| s.folded);
            let (w, h) = (96.0 * bk, 136.0 * bk);
            let y = bp(0.0, 548.0).y;
            for (i, c) in hole.iter().enumerate() {
                // your cards turn up a moment after the hand has put them down in front of you
                let Some(age) = self.age_of(Put::Hole(YOU, i as u8)).map(|a| a - 0.25).filter(|a| *a >= 0.0) else { continue };
                let x = bp(550.0, 0.0).x + if i == 0 { -w - 7.0 * bk } else { 7.0 * bk };
                let r = Rect::from_min_size(pos2(x, y + if i == 0 { 0.0 } else { 4.0 * bk }), vec2(w, h));
                flip_card(p, r, *c, age);
                if folded {
                    p.rect_filled(r, 9.0 * bk, Color32::from_black_alpha(150));
                } else if age > 0.5 {
                    card_soul(p, r, *c, t);
                }
                self.paint_card_clicks(p, i as u8, r, *c, t, bk);
            }
        }
        // your chips
        let you = self.s.t.table.seats.get(YOU).map_or(0, |s| s.stack);
        let at = bp(150.0, 556.0);
        text(p, at, Align2::LEFT_TOP, "YOUR CHIPS", base.s(15.0, 11.0), SAND.gamma_multiply(0.85));
        text(p, at + vec2(0.0, 18.0 * bk), Align2::LEFT_TOP, group(you), base.s(26.0, 16.0), AMBER);
        // and the chips themselves, on the felt beside you
        poker_chip_art::paint_towers(p, bp(372.0, 640.0), bk * 1.2, you, self.stack_reveal, self.chip_face.as_ref().map(|t| t.id()));
        if self.s.t.table.to_act == Some(YOU) && self.s.phase == Phase::Playing && !self.s.is_watching() && !self.s.hold {
            text(p, at + vec2(0.0, 50.0 * bk), Align2::LEFT_TOP, "YOUR MOVE", base.s(14.0, 11.0), Color32::from_rgb(255, 190, 120).gamma_multiply(0.6 + 0.4 * (t * 5.0).sin().abs()));
        }
    }

    /// Three dots over the head of whoever the game is waiting on, lighting one after another.
    fn paint_thought(&self, p: &Painter, lay: &Lay, t: f32) {
        let (Some(seat), true) = (self.think_seat, self.think > 0.02) else {
            return;
        };
        let Some(who) = Who::of_seat(seat) else {
            return;
        };
        let (fx, fy) = who.face();
        let k = lay.k;
        let a = self.think;
        let relaxed = !self.opts.quick_table || self.dev_think.is_some();
        if relaxed {
            // a soft light finds whoever is thinking and the room goes a little darker round them
            spotlight(p, lay, lay.p(fx, fy + 20.0), a, t);
        }
        let at = lay.p(fx + 72.0, fy - 108.0);
        p.add(Shape::ellipse_filled(at + vec2(0.0, 1.0 * k), vec2(34.0 * k, 17.0 * k), Color32::from_rgba_unmultiplied(8, 5, 8, (150.0 * a) as u8)));
        p.add(Shape::ellipse_stroke(at + vec2(0.0, 1.0 * k), vec2(34.0 * k, 17.0 * k), Stroke::new((1.2 * k).max(0.8), Color32::from_rgba_unmultiplied(190, 205, 230, (110.0 * a) as u8))));
        for i in 0..3 {
            let lit = ((t * 2.6 - i as f32 * 0.9).sin() * 0.5 + 0.5).powi(2);
            let c = at + vec2((i as f32 - 1.0) * 16.0 * k, 0.0);
            p.circle_filled(c, 4.6 * k * (0.8 + 0.3 * lit), Color32::from_rgba_unmultiplied(214, 226, 246, ((70.0 + 170.0 * lit) * a) as u8));
        }
        if relaxed {
            text(p, at + vec2(0.0, 27.0 * k), Align2::CENTER_CENTER, format!("{} is thinking", short_name(seat)), lay.s(13.0, 10.0), faded(Color32::from_rgb(190, 205, 230), a * 0.8));
        }
    }

    /// The nameplates along the table's edge, the cards the others hold, and what they just said.
    fn paint_plates(&self, p: &Painter, lay: &Lay, t: f32) {
        let k = lay.k;
        let tb = &self.s.t.table;
        let size = lay.s(16.0, 12.0);
        for seat in [CAT, BEAR, MONARCH, FATHER] {
            if seat == MONARCH && !self.s.t.monarch_in {
                continue;
            }
            let Some(st) = tb.seats.get(seat) else { continue };
            let out = self.is_out(seat);
            let name = short_name(seat);
            let col = seat_colour(seat);
            let active = tb.to_act == Some(seat) && self.s.phase == Phase::Playing;
            let won = self.s.summary.as_ref().filter(|_| self.s.phase == Phase::Showdown).is_some_and(|s| s.won.get(seat).is_some_and(|w| *w > 0));
            let dealer = tb.button == seat && (tb.running || self.s.phase == Phase::Showdown);
            let backs = tb.running && st.in_hand && !st.folded;

            let name_g = p.layout_no_wrap(name.to_string(), FontId::proportional(size), if out { DIM.gamma_multiply(0.6) } else { col });
            let tail = if out {
                "OUT".to_string()
            } else if st.folded && tb.running {
                "folded".to_string()
            } else if st.all_in {
                format!("{}  ALL IN", group(st.stack))
            } else {
                group(st.stack)
            };
            let tail_g = p.layout_no_wrap(tail, FontId::proportional(size), if out || (st.folded && tb.running) { DIM.gamma_multiply(0.7) } else { CREAM });
            let backs_w = if backs { 30.0 * k } else { 0.0 };
            let w = backs_w + name_g.size().x + 12.0 * k + tail_g.size().x + 20.0 * k;
            let h = name_g.size().y + 8.0 * k;
            let x = lay.p(PLATE_X[seat], 0.0).x;
            let r = Rect::from_center_size(pos2(x, lay.p(0.0, PLATE_Y).y), vec2(w, h));
            p.rect_filled(r, h * 0.5, Color32::from_rgba_unmultiplied(10, 5, 5, 205));
            let line = if won {
                Color32::from_rgb(255, 214, 120)
            } else if active {
                Color32::from_rgb(255, 150, 70).gamma_multiply(0.6 + 0.4 * (t * 6.0).sin().abs())
            } else {
                col.gamma_multiply(0.45)
            };
            p.rect_stroke(r, h * 0.5, Stroke::new(if won || active { 2.0 } else { 1.0 }, line), StrokeKind::Inside);
            let mut cursor = r.left() + 10.0 * k;
            if backs {
                for i in 0..2 {
                    // each shows once the hand's card has slid over to them
                    if !self.age_of(Put::Hole(seat, i as u8)).is_some_and(|a| a > 0.3) {
                        continue;
                    }
                    let cr = Rect::from_min_size(pos2(cursor + i as f32 * 8.0 * k, r.center().y - 9.0 * k), vec2(12.0 * k, 17.0 * k));
                    card_back(p, cr, false);
                }
                cursor += backs_w - 4.0 * k;
            }
            let ny = r.center().y - name_g.size().y / 2.0;
            let nw = name_g.size().x;
            p.galley(pos2(cursor, ny), name_g, col);
            cursor += nw + 12.0 * k;
            p.galley(pos2(cursor, ny), tail_g, CREAM);
            if dealer {
                let c = pos2(r.right() + 14.0 * k, r.center().y);
                p.circle_filled(c, 9.0 * k, CREAM);
                text(p, c, Align2::CENTER_CENTER, "D", lay.s(12.0, 9.0), Color32::from_rgb(27, 11, 10));
            }
            // the cards turned over at showdown
            if self.s.phase == Phase::Showdown {
                if let Some(Some(hole)) = self.s.summary.as_ref().map(|s| s.shown[seat]) {
                    for (i, c) in hole.iter().enumerate() {
                        let cr = Rect::from_min_size(pos2(x + (i as f32 - 1.0) * 28.0 * k + 2.0 * k, r.bottom() + 6.0 * k), vec2(26.0 * k, 37.0 * k));
                        card_face(p, cr, *c, true);
                    }
                }
            }
            // what they just did
            if let Some(said) = &self.s.said[seat] {
                if said.age < 3.4 {
                    let a = 1.0 - ((said.age - 2.6) / 0.8).clamp(0.0, 1.0);
                    let y = r.bottom() + if self.s.phase == Phase::Showdown { 50.0 * k } else { 14.0 * k };
                    pill(p, pos2(x, y), &said.text, lay.s(13.0, 10.0), faded(CREAM, a), faded(INK, a * 0.85), faded(col, a * 0.6));
                }
            }
            // what they won
            if won {
                if let Some(sum) = &self.s.summary {
                    let w = sum.won.get(seat).copied().unwrap_or(0);
                    text(p, pos2(x, r.top() - 12.0 * k), Align2::CENTER_CENTER, format!("+{}", group(w)), lay.s(20.0, 13.0), Color32::from_rgb(255, 214, 120));
                }
            }
        }
        // the dealer's own small plate
        let dr = pill(p, lay.p(DEALER_PLATE_X, PLATE_Y), "Mammon  ·  dealer", lay.s(14.0, 11.0), Color32::from_rgb(216, 192, 122), Color32::from_rgba_unmultiplied(10, 5, 5, 205), Color32::from_rgb(120, 100, 60));
        let _ = dr;
        // your own result at showdown
        if self.s.phase == Phase::Showdown {
            if let Some(sum) = &self.s.summary {
                let w = sum.won.get(YOU).copied().unwrap_or(0);
                if w > 0 {
                    text(p, lay.p(550.0, 536.0), Align2::CENTER_CENTER, format!("You win {}", group(w)), lay.s(22.0, 14.0), Color32::from_rgb(255, 214, 120));
                }
            }
        }
    }

    // ---- the strip at the top, the words ----------------------------------------------------------

    fn paint_hud(&self, p: &Painter, lay: &Lay) {
        let r = lay.rect;
        let h = lay.s(40.0, 30.0);
        let strip = Rect::from_min_max(r.left_top(), pos2(r.right(), r.top() + h));
        p.rect_filled(strip, 0.0, Color32::from_rgba_unmultiplied(10, 5, 5, 178));
        text(p, pos2(r.left() + 20.0, strip.center().y), Align2::LEFT_CENTER, "The Devil's Table", lay.s(19.0, 14.0), AMBER);
        let (sb, bb) = self.s.blinds();
        let level_left = self.s.t.hands_left_in_level();
        let hand = if self.s.t.table.running { self.s.t.table.hand_no } else { self.s.t.hands + 1 };
        let mut parts: Vec<(String, String)> = vec![
            ("Hand".to_string(), hand.to_string()),
            ("Blinds".to_string(), format!("{} / {}", group(sb), group(bb))),
            ("Next level".to_string(), format!("{} hand{}", level_left, if level_left == 1 { "" } else { "s" })),
        ];
        if r.width() >= 1250.0 {
            parts.push(("Prizes".to_string(), format!("1st {} · 2nd {}", PAYOUTS[0], PAYOUTS[1])));
        }
        // (right to left, leaving room for the two buttons)
        let mut x = r.right() - lay.s(300.0, 220.0);
        let size = lay.s(16.0, 12.0);
        for (label, value) in parts.iter().rev() {
            let g_val = p.layout_no_wrap(value.clone(), FontId::proportional(size), SAND);
            let g_lab = p.layout_no_wrap(label.to_uppercase(), FontId::proportional(size * 0.85), DIM);
            let w = g_lab.size().x + 6.0 + g_val.size().x;
            x -= w + lay.s(22.0, 12.0);
            let y = strip.center().y;
            p.galley(pos2(x, y - g_lab.size().y / 2.0), g_lab.clone(), DIM);
            p.galley(pos2(x + g_lab.size().x + 6.0, y - g_val.size().y / 2.0), g_val, SAND);
        }
        let _ = x;
    }

    /// The subtitle under the strip: who is speaking and what they say (not a speech bubble).
    fn paint_words(&self, p: &Painter, lay: &Lay) {
        let r = lay.rect;
        let top = r.top() + lay.s(40.0, 30.0) + 6.0;
        let line = self.s.story_line().or_else(|| self.s.chat.as_ref().map(|(l, _)| *l));
        let width = (r.width() * 0.62).clamp(280.0, 900.0);
        if let Some(l) = line {
            let who = l.who.name();
            let ink = if l.who == Speaker::Stage { DIM } else { CREAM };
            let g = p.layout(l.text.to_string(), FontId::proportional(lay.s(21.0, 15.0)), ink, width);
            let h = g.size().y + if who.is_empty() { 14.0 } else { lay.s(22.0, 16.0) + 10.0 };
            let box_r = Rect::from_center_size(pos2(r.center().x, top + h / 2.0), vec2(g.size().x.max(width * 0.5) + 40.0, h));
            p.rect_filled(box_r, 10.0, Color32::from_black_alpha(140));
            let mut y = box_r.top() + 7.0;
            if !who.is_empty() {
                text(p, pos2(r.center().x, y + lay.s(8.0, 6.0)), Align2::CENTER_CENTER, who.to_uppercase(), lay.s(14.0, 11.0), speaker_colour(l.who));
                y += lay.s(22.0, 16.0);
            }
            p.galley(pos2(r.center().x - g.size().x / 2.0, y), g, ink);
        } else if let Some((note, _)) = &self.s.notice {
            let g = p.layout(note.clone(), FontId::proportional(lay.s(17.0, 13.0)), faded(GOLD, 0.95), width);
            let box_r = Rect::from_center_size(pos2(r.center().x, top + g.size().y / 2.0 + 6.0), vec2(g.size().x + 36.0, g.size().y + 12.0));
            p.rect_filled(box_r, 10.0, Color32::from_black_alpha(120));
            p.galley(pos2(r.center().x - g.size().x / 2.0, box_r.top() + 6.0), g, GOLD);
        }
    }

    // ---- the buttons -------------------------------------------------------------------------------

    /// Draws the bar at the bottom and the Leave and fairness buttons; returns true to leave the table.
    fn controls(&mut self, ui: &mut Ui, p: &Painter, lay: &Lay, enter: bool) -> bool {
        let r = lay.rect;
        let k = lay.k;
        let bar_h = (54.0 * k).max(46.0);
        let bar = Rect::from_min_max(pos2(r.left(), r.bottom() - bar_h), r.right_bottom());
        p.rect_filled(bar, 0.0, Color32::from_rgba_unmultiplied(10, 5, 5, 215));
        let bh = bar_h * 0.7;
        let by = bar.center().y - bh / 2.0;
        let mut quit = false;

        // top right: how the shuffle is checked, and standing up
        let strip_h = lay.s(40.0, 30.0);
        let leave_r = Rect::from_min_size(pos2(r.right() - lay.s(98.0, 76.0), r.top() + strip_h * 0.15), vec2(lay.s(86.0, 66.0), strip_h * 0.7));
        let leaving = self.s.phase == Phase::Over;
        if press(ui, p, leave_r, if leaving { "Leave" } else { "Stand up" }, Style::Plain, "poker_leave") {
            if leaving {
                quit = true;
            } else {
                self.confirm_leave = true;
            }
        }
        let fair_label = match self.s.proof_ok {
            _ if self.s.rigged => "Test hand: rigged",
            Some(true) => "Fair shuffle: checked",
            Some(false) => "Shuffle check FAILED",
            None => "Fair shuffle: sealed",
        };
        let fw = lay.s(172.0, 130.0);
        let fair_r = Rect::from_min_size(pos2(leave_r.left() - fw - 8.0, r.top() + strip_h * 0.15), vec2(fw, strip_h * 0.7));
        let style = if self.s.proof_ok == Some(false) { Style::Danger } else { Style::Plain };
        if press(ui, p, fair_r, fair_label, style, "poker_fair") {
            self.fair_open = !self.fair_open;
        }

        // the hell gear, beside them
        let gs = strip_h * 1.05;
        let gear_r = Rect::from_center_size(pos2(fair_r.left() - 10.0 - gs / 2.0, fair_r.center().y), vec2(gs, gs));
        let resp = ui.interact(gear_r, ui.id().with("poker_gear"), egui::Sense::click());
        let hot = resp.hovered();
        if hot {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            self.gear_open = !self.gear_open;
        }
        let (gdt, gnow) = ui.ctx().input(|i| (i.stable_dt.min(0.1), i.time));
        self.gear_a = (self.gear_a + gdt * if hot || self.gear_open { 1.9 } else { 0.55 }) % std::f32::consts::TAU;
        paint_gear(p, gear_r.center(), gs * 0.5, self.gear_a, hot || self.gear_open, (gnow % 3600.0) as f32);
        if self.gear_open {
            self.gear_panel(ui, p, lay, gear_r, leave_r.right());
        }

        if self.confirm_leave {
            return quit;
        }
        match self.s.phase {
            Phase::Story => {
                text(p, bar.center(), Align2::CENTER_CENTER, "Click anywhere to go on", lay.s(17.0, 13.0), DIM);
                let sr = Rect::from_min_size(pos2(bar.right() - lay.s(120.0, 90.0), by), vec2(lay.s(100.0, 76.0), bh));
                if press(ui, p, sr, "Skip", Style::Plain, "poker_skip") {
                    self.s.skip_story();
                }
                let _ = enter;
            }
            Phase::Ready => {
                if self.s.is_watching() {
                    self.watching_bar(ui, p, lay, bar, by, bh);
                } else {
                    // the seal, your seed, and the deal
                    let seal = &self.s.next_commit;
                    let short = if seal.len() > 16 { format!("{}…{}", &seal[..8], &seal[seal.len() - 6..]) } else { seal.clone() };
                    text(p, pos2(bar.left() + 20.0, bar.center().y - bh * 0.22), Align2::LEFT_CENTER, "SEALED BEFORE THE DEAL", lay.s(11.0, 9.0), DIM);
                    text(p, pos2(bar.left() + 20.0, bar.center().y + bh * 0.2), Align2::LEFT_CENTER, short, lay.s(15.0, 12.0), MINT);
                    let width = lay.s(250.0, 170.0);
                    let area = Rect::from_min_size(pos2(r.center().x - width - lay.s(60.0, 30.0), by), vec2(width + lay.s(90.0, 60.0), bh));
                    ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                        ui.horizontal_centered(|ui| {
                            ui.label(egui::RichText::new("Your seed").color(DIM));
                            let edit = ui.add(egui::TextEdit::singleline(&mut self.seed_edit).desired_width(width).char_limit(40));
                            if edit.changed() && self.s.set_client_seed(&self.seed_edit) {
                                self.seed_edit = self.s.client_seed.clone();
                            }
                        });
                    });
                    let dr = Rect::from_min_size(pos2(r.center().x + lay.s(70.0, 40.0), by), vec2(lay.s(170.0, 120.0), bh));
                    if press(ui, p, dr, "Deal", Style::Primary, "poker_deal") {
                        self.s.deal();
                    }
                }
            }
            Phase::Playing => {
                if self.s.is_watching() {
                    self.watching_bar(ui, p, lay, bar, by, bh);
                } else if let Some(legal) = self.s.your_turn() {
                    if !self.was_my_turn {
                        self.raise = legal.min_raise_to as f64;
                    }
                    self.raise = self.raise.clamp(legal.min_raise_to as f64, legal.max_raise_to.max(legal.min_raise_to) as f64);
                    let mut act: Option<Action> = None;
                    let (bw, gap) = (lay.s(118.0, 80.0), lay.s(10.0, 6.0));
                    let call_label = if legal.can_check { "Check".to_string() } else { format!("Call {}", group(legal.call)) };
                    let call_w = lay.s(168.0, 110.0);
                    let slider_w = if legal.can_raise { lay.s(230.0, 150.0) } else { 0.0 };
                    let raise_w = if legal.can_raise { lay.s(190.0, 130.0) } else { 0.0 };
                    let all_w = if legal.can_raise { bw } else { 0.0 };
                    let n = if legal.can_raise { 5.0 } else { 2.0 };
                    let total = bw + call_w + slider_w + raise_w + all_w + gap * (n - 1.0);
                    let mut x = (r.center().x - total / 2.0).max(r.left() + 8.0);
                    let rect_for = |x: f32, w: f32| Rect::from_min_size(pos2(x, by), vec2(w, bh));
                    if press(ui, p, rect_for(x, bw), "Fold", Style::Plain, "poker_fold") {
                        act = Some(Action::Fold);
                    }
                    x += bw + gap;
                    if press(ui, p, rect_for(x, call_w), &call_label, Style::Plain, "poker_call") {
                        act = Some(if legal.can_check { Action::Check } else { Action::Call });
                    }
                    x += call_w + gap;
                    if legal.can_raise {
                        let lo = legal.min_raise_to as f64;
                        let hi = legal.max_raise_to.max(legal.min_raise_to) as f64;
                        if hi > lo {
                            let area = rect_for(x, slider_w);
                            ui.scope_builder(egui::UiBuilder::new().max_rect(area), |ui| {
                                ui.horizontal_centered(|ui| {
                                    ui.spacing_mut().slider_width = slider_w - 20.0;
                                    ui.add(egui::Slider::new(&mut self.raise, lo..=hi).show_value(false).step_by(1.0));
                                });
                            });
                        }
                        x += slider_w + gap;
                        let amount = self.raise.round() as Chips;
                        let label = if amount >= legal.max_raise_to { format!("All in {}", group(amount)) } else { format!("Raise to {}", group(amount)) };
                        if press(ui, p, rect_for(x, raise_w), &label, Style::Primary, "poker_raise") {
                            act = Some(Action::Raise(amount.clamp(legal.min_raise_to, legal.max_raise_to)));
                        }
                        x += raise_w + gap;
                        if press(ui, p, rect_for(x, all_w), "All in", Style::Danger, "poker_allin") {
                            act = Some(Action::Raise(legal.max_raise_to));
                        }
                        // quick sizes: half the pot, the pot. They sit in a row just above the bar, over
                        // the raise buttons, so that they can never cover Fold or Call.
                        let pot = self.s.t.table.pot();
                        let quick = [("Min", legal.min_raise_to), ("½ pot", (pot / 2).max(legal.min_raise_to)), ("Pot", pot.max(legal.min_raise_to))];
                        let (qw, qh) = (lay.s(70.0, 52.0), (bh * 0.6).max(22.0));
                        let right = x + all_w; // x is now at the All in button
                        for (i, (label, to)) in quick.iter().enumerate() {
                            let qx = right - (3 - i) as f32 * (qw + 6.0) + 6.0;
                            let qr = Rect::from_min_size(pos2(qx, bar.top() - qh - 6.0), vec2(qw, qh));
                            if press(ui, p, qr, label, Style::Plain, &format!("poker_q{i}")) {
                                self.raise = (*to).clamp(legal.min_raise_to, legal.max_raise_to) as f64;
                            }
                        }
                    }
                    if let Some(a) = act {
                        let _ = self.s.act(a);
                    }
                    self.was_my_turn = true;
                    return quit;
                } else {
                    // someone else is thinking
                    let who = self.s.t.table.to_act.map_or("the table", short_name);
                    text(p, bar.center(), Align2::CENTER_CENTER, format!("Waiting for {who}…"), lay.s(17.0, 13.0), DIM);
                }
            }
            Phase::Showdown => {
                if self.s.is_watching() {
                    self.watching_bar(ui, p, lay, bar, by, bh);
                } else {
                    let dr = Rect::from_center_size(bar.center(), vec2(lay.s(240.0, 160.0), bh));
                    if press(ui, p, dr, "Deal next hand", Style::Primary, "poker_next") || enter {
                        self.s.deal_next();
                    }
                }
            }
            Phase::Over => {
                let msg = match self.result {
                    Some(f) => {
                        let place = ["1st", "2nd", "3rd", "4th", "5th"][f.place.clamp(1, 5) - 1];
                        let mut s = format!("You finished {place}. {} CR-Tokens paid.", f.payout);
                        if f.ending.soul_taken() {
                            s.push_str(" The Shadow Monarch holds your soul.");
                        }
                        s
                    }
                    None => "The game is over.".to_string(),
                };
                text(p, pos2(bar.left() + 24.0, bar.center().y), Align2::LEFT_CENTER, msg, lay.s(18.0, 13.0), SAND);
                let lr = Rect::from_min_size(pos2(bar.right() - lay.s(220.0, 150.0), by), vec2(lay.s(200.0, 130.0), bh));
                if press(ui, p, lr, "Leave the table", Style::Primary, "poker_over") || enter {
                    quit = true;
                }
            }
        }
        self.was_my_turn = false;
        quit
    }

    /// The gear's panel, hung under it: the table's options, and under them the developer rows.
    fn gear_panel(&mut self, ui: &mut Ui, p: &Painter, lay: &Lay, gear_r: Rect, right: f32) {
        let now = ui.ctx().input(|i| i.time);
        let (pw, rh) = (lay.s(340.0, 262.0), lay.s(52.0, 40.0));
        let head = lay.s(30.0, 24.0);
        let panel = Rect::from_min_size(pos2(right - pw, gear_r.bottom() + 8.0), vec2(pw, head * 2.0 + rh * 6.0 + 12.0));
        // (the panel takes the clicks that land on it, so they do not reach what is behind it)
        let _ = ui.interact(panel, ui.id().with("poker_gear_panel"), egui::Sense::click());
        p.rect_filled(panel.translate(vec2(0.0, 3.0)), 10.0, Color32::from_rgba_unmultiplied(0, 0, 0, 120));
        p.rect_filled(panel, 10.0, Color32::from_rgb(22, 9, 8));
        p.rect_stroke(panel, 10.0, Stroke::new(1.5, Color32::from_rgb(170, 60, 24)), StrokeKind::Inside);
        text(p, pos2(panel.left() + 16.0, panel.top() + head * 0.55), Align2::LEFT_CENTER, "THE GEAR", lay.s(12.0, 10.0), Color32::from_rgb(222, 104, 52));
        let rows = [
            ("Deal automatically", "The next hand is dealt for you", self.opts.auto_deal),
            ("Skip the dealing", "Cards go straight down", self.opts.skip_dealing),
            ("Quick table", "No slow camera, faster players", self.opts.quick_table),
        ];
        for (i, (name, sub, on)) in rows.iter().enumerate() {
            let row = Rect::from_min_size(pos2(panel.left() + 6.0, panel.top() + head + rh * i as f32), vec2(pw - 12.0, rh));
            if toggle_row(ui, p, row, name, sub, *on, lay, &format!("poker_gear_opt{i}")) {
                match i {
                    0 => self.opts.auto_deal = !self.opts.auto_deal,
                    1 => self.opts.skip_dealing = !self.opts.skip_dealing,
                    _ => self.opts.quick_table = !self.opts.quick_table,
                }
                self.opts_changed = true;
            }
        }
        // the developer rows
        let dy = panel.top() + head + rh * 3.0;
        p.line_segment([pos2(panel.left() + 12.0, dy + 2.0), pos2(panel.right() - 12.0, dy + 2.0)], Stroke::new(1.0, Color32::from_rgba_unmultiplied(170, 60, 24, 120)));
        text(p, pos2(panel.left() + 16.0, dy + head * 0.6), Align2::LEFT_CENTER, "DEVELOPER", lay.s(12.0, 10.0), Color32::from_rgb(222, 104, 52));
        let ready = self.s.phase == Phase::Ready;
        let sub = if ready { "The story plays; he comes out of the dark" } else { "Only between hands, before the deal" };
        let row = Rect::from_min_size(pos2(panel.left() + 6.0, dy + head), vec2(pw - 12.0, rh));
        if action_row(ui, p, row, "Cue the Shadow's arrival", sub, "PLAY", ready, lay, "poker_dev_arrival") {
            self.dev_arrival = true;
            self.gear_open = false;
        }
        let row = Rect::from_min_size(pos2(panel.left() + 6.0, dy + head + rh), vec2(pw - 12.0, rh));
        if toggle_row(ui, p, row, "Character animations", "Play what each of them does", self.dev_open, lay, "poker_dev_anims") {
            self.dev_open = !self.dev_open;
        }
        let row = Rect::from_min_size(pos2(panel.left() + 6.0, dy + head + rh * 2.0), vec2(pw - 12.0, rh));
        if toggle_row(ui, p, row, "The pleading monarchs", "Every hand: K of hearts, Q of diamonds. Not fair", self.s.dev_monarchs, lay, "poker_dev_monarchs") {
            self.s.dev_monarchs = !self.s.dev_monarchs;
        }
        if self.dev_open {
            self.dev_panel(ui, p, lay, Rect::from_min_max(pos2(panel.left() - lay.s(392.0, 300.0) - 8.0, panel.top()), pos2(panel.left() - 8.0, panel.top())), now);
        }
    }

    /// Every character's animations, one button each (hung to the left of the gear's panel).
    fn dev_panel(&mut self, ui: &mut Ui, p: &Painter, lay: &Lay, at: Rect, now: f64) {
        let (rh, head) = (lay.s(38.0, 30.0), lay.s(30.0, 24.0));
        let panel = Rect::from_min_size(at.min, vec2(at.width(), head + rh * DEV_ROWS.len() as f32 + 8.0));
        let _ = ui.interact(panel, ui.id().with("poker_dev_panel"), egui::Sense::click());
        p.rect_filled(panel.translate(vec2(0.0, 3.0)), 10.0, Color32::from_rgba_unmultiplied(0, 0, 0, 120));
        p.rect_filled(panel, 10.0, Color32::from_rgb(22, 9, 8));
        p.rect_stroke(panel, 10.0, Stroke::new(1.5, Color32::from_rgb(170, 60, 24)), StrokeKind::Inside);
        text(p, pos2(panel.left() + 16.0, panel.top() + head * 0.55), Align2::LEFT_CENTER, "ANIMATIONS", lay.s(12.0, 10.0), Color32::from_rgb(222, 104, 52));
        let name_w = lay.s(104.0, 80.0);
        let bw = (panel.width() - name_w - 22.0) / 3.0 - 4.0;
        for (i, (who, buttons)) in DEV_ROWS.iter().enumerate() {
            let y = panel.top() + head + rh * i as f32;
            text(p, pos2(panel.left() + 16.0, y + rh / 2.0), Align2::LEFT_CENTER, *who, lay.s(14.0, 11.0), SAND);
            for (j, (label, d)) in buttons.iter().enumerate() {
                let b = Rect::from_min_size(pos2(panel.left() + name_w + 10.0 + j as f32 * (bw + 4.0), y + rh * 0.12), vec2(bw, rh * 0.76));
                let resp = ui.interact(b, ui.id().with(("poker_dev_anim", i, j)), egui::Sense::click());
                let hot = resp.hovered();
                if hot {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                p.rect_filled(b, 6.0, if hot { Color32::from_rgb(120, 40, 16) } else { Color32::from_rgb(52, 20, 14) });
                p.rect_stroke(b, 6.0, Stroke::new(1.0, Color32::from_rgb(150, 70, 36)), StrokeKind::Inside);
                text(p, b.center(), Align2::CENTER_CENTER, *label, lay.s(12.0, 9.5), Color32::from_rgb(255, 226, 170));
                if resp.clicked() {
                    self.play_dev(*d, now);
                }
            }
        }
    }

    fn watching_bar(&mut self, ui: &mut Ui, p: &Painter, lay: &Lay, bar: Rect, by: f32, bh: f32) {
        let msg = if self.s.left { "You stood up. The game plays on without you." } else { "You are out. The game plays on without you." };
        text(p, pos2(bar.left() + 24.0, bar.center().y), Align2::LEFT_CENTER, msg, lay.s(17.0, 13.0), DIM);
        if !self.s.fast {
            let sr = Rect::from_min_size(pos2(bar.right() - lay.s(190.0, 130.0), by), vec2(lay.s(170.0, 110.0), bh));
            if press(ui, p, sr, "Skip ahead", Style::Plain, "poker_watch_skip") {
                self.s.skip();
            }
        }
    }

    // ---- windows -----------------------------------------------------------------------------------

    fn windows(&mut self, ctx: &egui::Context) {
        if self.confirm_leave {
            let mut stay = false;
            let mut go = false;
            egui::Window::new("Stand up?").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.set_max_width(380.0);
                ui.label("Your seat folds from now on, and the game plays out without you. Standing up does not buy you out of what is at the end of it, and your CR-Tokens stay in the pot.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Stay and play").clicked() {
                        stay = true;
                    }
                    if ui.button("Stand up").clicked() {
                        go = true;
                    }
                });
            });
            if stay {
                self.confirm_leave = false;
            }
            if go {
                self.confirm_leave = false;
                self.s.leave();
            }
        }
        if self.fair_open {
            let mut open = true;
            let s = &mut self.s;
            let seed_edit = &mut self.seed_edit;
            egui::Window::new("How the shuffle is kept honest").open(&mut open).collapsible(false).default_width(520.0).show(ctx, |ui| {
                fair_panel(ui, s, seed_edit);
            });
            self.fair_open = open;
        }
    }
}

fn fair_panel(ui: &mut Ui, s: &mut Session, seed_edit: &mut String) {
    ui.label(
        "Before every hand the table picks a secret seed and shows you its fingerprint (SHA-256). After the hand it shows the seed. \
         The 52 cards are shuffled from that seed, your own seed and the hand number, so the deck was fixed before anyone acted, and you can rebuild it and check.",
    );
    ui.add_space(6.0);
    ui.strong("Next hand");
    ui.horizontal_wrapped(|ui| {
        ui.label("Sealed fingerprint:");
        ui.monospace(&s.next_commit);
    });
    ui.horizontal(|ui| {
        ui.label("Your seed:");
        let edit = ui.add(egui::TextEdit::singleline(seed_edit).desired_width(260.0).char_limit(40));
        if edit.changed() && s.set_client_seed(seed_edit) {
            *seed_edit = s.client_seed.clone();
        }
    });
    ui.label(egui::RichText::new("You can change your seed between hands; the table cannot know it before it seals its own.").weak());
    ui.add_space(8.0);
    ui.strong("Last hand");
    match s.proof.clone() {
        Some(proof) => proof_lines(ui, &proof, s.proof_ok),
        None => {
            ui.label("None yet.");
        }
    }
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(
            "To check it yourself: SHA-256 of \"CPUNITY-POKER-COMMIT|v1|\" followed by the 32 seed bytes must equal the fingerprint shown before the hand. \
             Then the deck is the shuffle of cards 0 to 51 driven by SHA-256(seed + \"|\" + your seed + \"|\" + hand number + \"|\" + block number). The computer players are only ever told their own two cards.",
        )
        .weak(),
    );
}

fn proof_lines(ui: &mut Ui, proof: &HandProof, ok: Option<bool>) {
    ui.label(format!("Hand {}", proof.hand_no));
    ui.horizontal_wrapped(|ui| {
        ui.label("Fingerprint shown before the deal:");
        ui.monospace(proof.commitment_hex());
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Seed revealed:");
        ui.monospace(proof.server_seed_hex());
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Your seed:");
        ui.monospace(&proof.client_seed);
    });
    ui.label("Deck, top card first:");
    ui.add(egui::Label::new(egui::RichText::new(proof.deck_text()).monospace()).wrap());
    match (ok, proof.verify()) {
        (_, Ok(())) => {
            ui.colored_label(Color32::from_rgb(140, 210, 170), "Checked here: the seed matches its fingerprint and rebuilds exactly this deck.");
        }
        (_, Err(e)) => {
            ui.colored_label(Color32::from_rgb(235, 110, 90), format!("Check failed: {e:?}"));
        }
    }
}

/// The scene when the soul lock runs out: lines read one click at a time. `step` is where it is
/// (None: not on the screen).
pub fn free_scene(ctx: &egui::Context, step: &mut Option<usize>) {
    let Some(i) = *step else { return };
    let lines = crate::poker_story::broke_free();
    let Some(line) = lines.get(i).copied() else {
        *step = None;
        return;
    };
    let mut next = false;
    egui::Window::new("poker_free_scene").title_bar(false).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        ui.set_max_width(420.0);
        let name = line.who.name();
        if !name.is_empty() {
            ui.label(egui::RichText::new(name.to_uppercase()).small().color(speaker_colour(line.who)));
        }
        let ink = if line.who == Speaker::Stage { DIM } else { CREAM };
        ui.label(egui::RichText::new(line.text).size(19.0).color(ink));
        ui.add_space(8.0);
        if ui.button(if i + 1 < lines.len() { "Go on" } else { "Done" }).clicked() {
            next = true;
        }
    });
    if next {
        *step = if i + 1 < lines.len() { Some(i + 1) } else { None };
    }
}
