//! Game 1, hidden behind Father Time: you are the cat, inside his clock. The clock's hands sweep
//! round and try to hit you, a thin second hand joins in after fifteen seconds, and sand falls out
//! of his hourglass (catch it for extra lives). Arrow keys or WASD move, Space leaps over a hand.
//! Surviving the time takes you to a checkpoint (a minute on Hard, less on the easier settings), and
//! then comes part two: the clock's real hour, faster, with the second hand out from the start and a
//! second hand that turns the other way. Surviving that wins, and winning unlocks the Gold Edition.
//! The checkpoint is kept (see `games.rs`): losing in part two starts part two again, not the game.
//!
//! Everything here is pure (no drawing, no clock): `Game::step` advances it, so it is tested.
//! Coordinates are Father Time's picture coordinates (see `father.rs` / `info.rs`).

use crate::games::{Cheat, Cheats, Difficulty};
use crate::{father, info};

/// What a difficulty changes: how long you must survive, and how long the leap takes to come back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rules {
    /// How long part one lasts.
    pub duration: f32,
    pub leap_cooldown: f32,
    /// How long part two lasts.
    pub part_two: f32,
}

impl Rules {
    pub fn of(d: Difficulty) -> Rules {
        match d {
            Difficulty::Easy => Rules { duration: 30.0, leap_cooldown: 0.4, part_two: 20.0 },
            Difficulty::Normal => Rules { duration: 45.0, leap_cooldown: 0.7, part_two: 30.0 },
            Difficulty::Hard => Rules { duration: 60.0, leap_cooldown: 1.0, part_two: 40.0 },
        }
    }
}

pub const CENTRE: (f32, f32) = info::CLOCK_CENTER;
/// The cat can walk on the clock face, a disc this big.
pub const FIELD_RADIUS: f32 = 400.0;
pub const CAT_RADIUS: f32 = 24.0;
pub const CAT_SPEED: f32 = 360.0;
/// How long a leap lasts (you cannot be hit). How long before the next one depends on the
/// difficulty (`Rules`).
pub const LEAP_TIME: f32 = 0.45;
/// How long you are untouchable after a hit.
pub const HURT_TIME: f32 = 1.4;
/// The Borrowed Time cheat runs the clock's hands at this fraction of their speed.
pub const SLOW_HANDS: f32 = 0.5;
pub const START_LIVES: u8 = 3;
pub const MAX_LIVES: u8 = 5;
/// A life for every this many grains of sand.
pub const SAND_PER_LIFE: u32 = 6;
pub const SAND_RADIUS: f32 = 30.0;
/// The thin second hand starts hunting at this time; it shows up (dim) a few seconds before.
pub const SECOND_FROM: f32 = 15.0;
pub const SECOND_WARNING: f32 = 3.0;
/// Part two: the hands start where part one's would be late on (so they are fast from the first second).
pub const PART_TWO_SKEW: f32 = 35.0;
/// Part two: the second hand is dangerous from this time, and the hand that turns the other way is
/// shown at `COUNTER_SHOWN` and dangerous from `COUNTER_LIVE`.
pub const PART_TWO_SECOND_LIVE: f32 = 2.0;
pub const COUNTER_SHOWN: f32 = 5.0;
pub const COUNTER_LIVE: f32 = 8.0;
/// Part two never stops speeding up: the hands' time runs at 1 + this x how far through you are, so
/// they end up this much faster than they began.
pub const PART_TWO_SPEEDUP: f32 = 1.4;
/// Part two is the reveal: the cat is rising (towards Heaven, towards a crown) and Father Time is
/// trying to stop him. What Father Time says, and when (a fraction of part two; the first is at 0.8 s).
const PART_TWO_LINES: [(f32, &str); 5] = [
    (0.0, "Stop climbing, cat."),
    (0.22, "That light is not yours."),
    (0.45, "You do not know what you are becoming."),
    (0.68, "Heaven does not want you. It wants a KING."),
    (0.86, "Do NOT wear that crown!"),
];
/// How long a grain takes to reach where it lands, how long it stays, and how long a line of
/// Father Time's speech stays up.
const GRAIN_TRAVEL: f32 = 1.6;
const GRAIN_LIFE: f32 = 6.5;
const LINE_LIFE: f32 = 2.4;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Input {
    /// -1..1, right is positive
    pub dx: f32,
    /// -1..1, down is positive
    pub dy: f32,
    pub leap: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Playing,
    /// Part one is survived: the checkpoint. Part two begins when `begin_part_two` is called.
    Checkpoint,
    Won,
    Lost,
}

/// Which half of the game this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    One,
    Two,
}

impl Part {
    /// Where a new go begins: part two once the checkpoint is kept, else part one.
    pub fn from_checkpoint(kept: bool) -> Part {
        if kept {
            Part::Two
        } else {
            Part::One
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Grain {
    start: (f32, f32),
    target: (f32, f32),
    pub age: f32,
}

impl Grain {
    /// Where it is now: it flies out of the hourglass and slows to a stop.
    pub fn pos(&self) -> (f32, f32) {
        let x = (self.age / GRAIN_TRAVEL).clamp(0.0, 1.0);
        let ease = 1.0 - (1.0 - x).powi(3);
        (self.start.0 + (self.target.0 - self.start.0) * ease, self.start.1 + (self.target.1 - self.start.1) * ease)
    }
    /// 0..1 how bright it is: fades in, and out at the end of its life.
    pub fn strength(&self) -> f32 {
        let fade_in = (self.age / 0.3).clamp(0.0, 1.0);
        let fade_out = ((GRAIN_LIFE - self.age) / 1.2).clamp(0.0, 1.0);
        fade_in.min(fade_out)
    }
}

/// The angles of the hands, degrees clockwise from 12 (the clock's own convention).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hands {
    pub hour: f32,
    pub minute: f32,
    pub second: f32,
    /// The second hand is out (dim, harmless) from `SECOND_FROM - SECOND_WARNING`.
    pub second_shown: bool,
    /// ... and dangerous from `SECOND_FROM`.
    pub second_live: bool,
    /// Part two's hand that turns the other way (degrees like the others), shown dim and then live.
    pub counter: f32,
    pub counter_shown: bool,
    pub counter_live: bool,
}

/// Where the hands are `t` seconds in. They speed up as the minute goes on.
pub fn hands(t: f32) -> Hands {
    Hands {
        minute: 20.0 + 55.0 * t + t * t / 3.0,
        hour: 200.0 - (35.0 * t + 25.0 * t * t / 120.0),
        second: 140.0 * (t - SECOND_FROM + SECOND_WARNING),
        second_shown: t >= SECOND_FROM - SECOND_WARNING,
        second_live: t >= SECOND_FROM,
        counter: 0.0,
        counter_shown: false,
        counter_live: false,
    }
}

/// Where the hands are `t` seconds into part two: already fast, the thin hand out and quick, and a
/// second thin hand that turns the other way.
pub fn hands_part_two(t: f32) -> Hands {
    Hands {
        second: 170.0 * (t + 1.0),
        second_shown: true,
        second_live: t >= PART_TWO_SECOND_LIVE,
        counter: 90.0 - 120.0 * (t - COUNTER_SHOWN),
        counter_shown: t >= COUNTER_SHOWN,
        counter_live: t >= COUNTER_LIVE,
        ..hands(t + PART_TWO_SKEW)
    }
}

pub struct Game {
    pub t: f32,
    /// The time the hands show: it runs with `t`, or slower with Borrowed Time.
    pub hand_t: f32,
    pub difficulty: Difficulty,
    pub cat: (f32, f32),
    /// -1 facing left, 1 facing right
    pub facing: f32,
    pub lives: u8,
    pub sand: u32,
    pub hits: u32,
    pub phase: Phase,
    pub part: Part,
    /// Test switch: nothing costs a life (you still flash when a hand touches you). Counts as a cheat.
    pub invincible: bool,
    pub grains: Vec<Grain>,
    /// How long ago the game ended (win or loss).
    pub since_end: f32,
    leap_left: f32,
    leap_cool: f32,
    hurt_left: f32,
    next_grain: f32,
    rng: u32,
    line: Option<(&'static str, f32)>,
    said_start: bool,
    said_half: bool,
    /// Part two: how many of Father Time's lines about the climb have been said.
    stage: usize,
    death_reported: bool,
    checkpoint_reported: bool,
    rules: Rules,
    cheats: Cheats,
}

const HIT_LINES: [&str; 4] = ["Tick.", "Too slow, little cat.", "Time stings.", "Tock."];

impl Game {
    /// A game on Hard, which is how it was before there was a choice.
    pub fn new(seed: u32) -> Game {
        Game::with_difficulty(seed, Difficulty::Hard)
    }

    pub fn with_difficulty(seed: u32, difficulty: Difficulty) -> Game {
        Game::with_cheats(seed, difficulty, Cheats::NONE)
    }

    pub fn with_cheats(seed: u32, difficulty: Difficulty, cheats: Cheats) -> Game {
        Game {
            t: 0.0,
            hand_t: 0.0,
            cheats,
            difficulty,
            rules: Rules::of(difficulty),
            death_reported: false,
            checkpoint_reported: false,
            cat: (CENTRE.0, CENTRE.1 + 250.0),
            facing: 1.0,
            lives: START_LIVES,
            sand: 0,
            hits: 0,
            phase: Phase::Playing,
            part: Part::One,
            invincible: false,
            grains: Vec::new(),
            since_end: 0.0,
            leap_left: 0.0,
            leap_cool: 0.0,
            hurt_left: 0.0,
            next_grain: 1.5,
            rng: seed.wrapping_mul(2_654_435_761) | 1,
            line: None,
            said_start: false,
            said_half: false,
            stage: 0,
        }
    }

    /// A game that begins at `part` (part two when the checkpoint was kept).
    pub fn start(seed: u32, difficulty: Difficulty, cheats: Cheats, part: Part) -> Game {
        let mut g = Game::with_cheats(seed, difficulty, cheats);
        if part == Part::Two {
            g.begin_part_two();
        }
        g
    }

    /// Goes on from the checkpoint into part two: the clock starts again, faster, and you have your
    /// three lives back (or the lives you had, if you had more).
    pub fn begin_part_two(&mut self) {
        self.part = Part::Two;
        self.phase = Phase::Playing;
        self.t = 0.0;
        self.hand_t = 0.0;
        self.since_end = 0.0;
        self.lives = self.lives.max(START_LIVES);
        self.cat = (CENTRE.0, CENTRE.1 + 250.0);
        self.grains.clear();
        self.leap_left = 0.0;
        self.leap_cool = 0.0;
        self.hurt_left = 0.0;
        self.next_grain = 1.5;
        self.said_start = false;
        self.said_half = false;
        self.stage = 0;
        self.death_reported = false;
        self.line = None;
    }

    /// True once, the first time it is asked after part one is survived: for keeping the checkpoint.
    pub fn take_checkpoint(&mut self) -> bool {
        if self.phase == Phase::Checkpoint && !self.checkpoint_reported {
            self.checkpoint_reported = true;
            return true;
        }
        false
    }

    /// The seconds to pay for: a win counts both parts, a loss only the part it was lost in (so a
    /// retry from the checkpoint never pays part one again).
    pub fn paid_secs(&self) -> f32 {
        match (self.phase, self.part) {
            (Phase::Won, Part::Two) => self.rules.duration + self.t,
            _ => self.t,
        }
    }

    fn random(&mut self) -> f32 {
        // xorshift32
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    /// 0..1 how far through the leap; None when on the ground.
    pub fn leaping(&self) -> Option<f32> {
        (self.leap_left > 0.0).then(|| 1.0 - self.leap_left / LEAP_TIME)
    }
    /// 0..1 how recovered the leap is (1 = ready).
    pub fn leap_ready(&self) -> f32 {
        1.0 - (self.leap_cool / self.rules.leap_cooldown).clamp(0.0, 1.0)
    }
    /// Flashing after a hit.
    pub fn hurt(&self) -> bool {
        self.hurt_left > 0.0
    }
    /// 0..1 how far the cat has risen: nothing in part one, then it grows through part two (the glow,
    /// the lift, the halo that becomes a crown), and stays at the top once it is won.
    pub fn ascension(&self) -> f32 {
        match (self.part, self.phase) {
            (Part::One, _) => 0.0,
            (Part::Two, Phase::Won) => 1.0,
            (Part::Two, _) => self.progress(),
        }
    }

    /// 0..1 how far through the time you have to survive.
    pub fn progress(&self) -> f32 {
        (self.t / self.duration()).clamp(0.0, 1.0)
    }

    /// How long you have to survive in this part, in seconds.
    pub fn duration(&self) -> f32 {
        match self.part {
            Part::One => self.rules.duration,
            Part::Two => self.rules.part_two,
        }
    }

    /// True once, the first time it is asked after you lost: for counting deaths.
    pub fn take_death(&mut self) -> bool {
        if self.phase == Phase::Lost && !self.death_reported {
            self.death_reported = true;
            return true;
        }
        false
    }

    /// True once, the first time it is asked after you won: for paying the tokens. (A game ends
    /// either won or lost, so this shares `take_death`'s flag.)
    pub fn take_win(&mut self) -> bool {
        if self.phase == Phase::Won && !self.death_reported {
            self.death_reported = true;
            return true;
        }
        false
    }

    /// Whether a cheat is on: a run with one pays no tokens.
    pub fn cheated(&self) -> bool {
        self.cheats != Cheats::NONE || self.invincible
    }

    /// What Father Time is saying and how clearly (0..1), if anything.
    pub fn line(&self) -> Option<(&'static str, f32)> {
        let (text, age) = self.line?;
        let life = if self.phase == Phase::Playing { LINE_LIFE } else { 99.0 };
        if age >= life {
            return None;
        }
        let fade_in = (age / 0.15).clamp(0.0, 1.0);
        let fade_out = if self.phase == Phase::Playing { ((life - age) / 0.5).clamp(0.0, 1.0) } else { 1.0 };
        Some((text, fade_in.min(fade_out)))
    }

    fn say(&mut self, text: &'static str) {
        self.line = Some((text, 0.0));
    }

    /// Where the hands are now (they run slow with Borrowed Time).
    pub fn hands_now(&self) -> Hands {
        match self.part {
            Part::One => hands(self.hand_t),
            Part::Two => hands_part_two(self.hand_t),
        }
    }

    /// Part two's backwards hand as a segment in picture coordinates, when it is out.
    pub fn counter_segment(&self) -> Option<([(f32, f32); 2], bool)> {
        let h = self.hands_now();
        h.counter_shown.then(|| {
            let p = info::turn_about_clock(&info::SECOND_HAND, h.counter);
            ([p[0], p[1]], h.counter_live)
        })
    }

    /// The hands as polygons in picture coordinates right now, for hit tests and drawing.
    pub fn hand_shapes(&self) -> (Vec<(f32, f32)>, Vec<(f32, f32)>, [(f32, f32); 2]) {
        let h = self.hands_now();
        (
            info::turn_about_clock(&info::HOUR_OUTLINE, h.hour),
            info::turn_about_clock(&info::MINUTE_OUTLINE, h.minute),
            {
                let p = info::turn_about_clock(&info::SECOND_HAND, h.second);
                [p[0], p[1]]
            },
        )
    }

    fn touching_a_hand(&self) -> bool {
        let h = self.hands_now();
        let r = CAT_RADIUS * 0.72;
        let (hour, minute, second) = self.hand_shapes();
        circle_hits_polygon(self.cat, r, &hour)
            || circle_hits_polygon(self.cat, r, &minute)
            || (h.second_live && distance_to_segment(self.cat, second[0], second[1]) <= r + 3.0)
            || self.counter_segment().is_some_and(|(c, live)| live && distance_to_segment(self.cat, c[0], c[1]) <= r + 3.0)
            // the hub in the middle
            || dist(self.cat, CENTRE) <= r + info::HUB_RADIUS
    }

    /// Advances the game by `dt` seconds (long frames are cut to a twentieth of a second).
    pub fn step(&mut self, dt: f32, input: Input) {
        let dt = dt.clamp(0.0, 0.05);
        if let Some((_, age)) = &mut self.line {
            *age += dt;
        }
        if self.phase != Phase::Playing {
            self.since_end += dt;
            return;
        }
        self.t += dt;
        let slow = if self.cheats.has(Cheat::BorrowedTime) { SLOW_HANDS } else { 1.0 };
        // (in part two the clock runs faster and faster the further the cat has climbed)
        let racing = if self.part == Part::Two { 1.0 + PART_TWO_SPEEDUP * self.progress() } else { 1.0 };
        self.hand_t += dt * slow * racing;
        if self.part == Part::One {
            if !self.said_start && self.t >= 0.8 {
                self.said_start = true;
                self.say("You dare enter my hour?");
            }
            if !self.said_half && self.t >= self.duration() / 2.0 {
                self.said_half = true;
                self.say("...Not bad, kitten.");
            }
        } else if let Some(&(at, text)) = PART_TWO_LINES.get(self.stage) {
            // part two: he is begging, then threatening, as the cat climbs
            if self.t >= (at * self.duration()).max(0.8) {
                self.stage += 1;
                self.say(text);
            }
        }

        // moving
        let (mut dx, mut dy) = (input.dx.clamp(-1.0, 1.0), input.dy.clamp(-1.0, 1.0));
        let len = (dx * dx + dy * dy).sqrt();
        if len > 1.0 {
            dx /= len;
            dy /= len;
        }
        if dx.abs() > 0.05 {
            self.facing = dx.signum();
        }
        self.cat.0 += dx * CAT_SPEED * dt;
        self.cat.1 += dy * CAT_SPEED * dt;
        let (ox, oy) = (self.cat.0 - CENTRE.0, self.cat.1 - CENTRE.1);
        let from_centre = (ox * ox + oy * oy).sqrt();
        let limit = FIELD_RADIUS - CAT_RADIUS;
        if from_centre > limit {
            self.cat = (CENTRE.0 + ox / from_centre * limit, CENTRE.1 + oy / from_centre * limit);
        }

        // leaping
        self.leap_left = (self.leap_left - dt).max(0.0);
        self.leap_cool = (self.leap_cool - dt).max(0.0);
        self.hurt_left = (self.hurt_left - dt).max(0.0);
        if input.leap && self.leap_left <= 0.0 && self.leap_cool <= 0.0 {
            self.leap_left = LEAP_TIME;
            self.leap_cool = self.rules.leap_cooldown;
        }

        // sand out of the hourglass, more often as time goes on
        self.next_grain -= dt;
        if self.next_grain <= 0.0 {
            self.next_grain = 1.1 - 0.45 * self.progress() + self.random() * 0.5;
            let angle = self.random() * std::f32::consts::TAU;
            let reach = FIELD_RADIUS * 0.8 * self.random().sqrt();
            let target = (CENTRE.0 + angle.cos() * reach, CENTRE.1 + angle.sin() * reach);
            self.grains.push(Grain { start: father::HOURGLASS, target, age: 0.0 });
        }
        for g in &mut self.grains {
            g.age += dt;
        }
        self.grains.retain(|g| g.age < GRAIN_LIFE);
        // catching it
        let cat = self.cat;
        let before = self.grains.len();
        self.grains.retain(|g| !(g.age > 0.35 && dist(g.pos(), cat) <= SAND_RADIUS + CAT_RADIUS * 0.5));
        let caught = (before - self.grains.len()) as u32;
        for _ in 0..caught {
            self.sand += 1;
            if self.sand % SAND_PER_LIFE == 0 && self.lives < MAX_LIVES {
                self.lives += 1;
            }
        }

        // the hands
        let touched = self.leap_left <= 0.0 && self.hurt_left <= 0.0 && self.touching_a_hand();
        if touched && self.invincible {
            // the test switch: it flashes, and costs nothing
            self.hurt_left = HURT_TIME;
            self.hits += 1;
        } else if touched {
            self.lives -= 1;
            self.hits += 1;
            self.hurt_left = HURT_TIME;
            if self.lives == 0 {
                self.phase = Phase::Lost;
                self.since_end = 0.0;
                self.say("Time always wins.");
                return;
            }
            // (in part two he is not taunting: his words are about the climb)
            if self.part == Part::One {
                let pick = (self.random() * HIT_LINES.len() as f32) as usize % HIT_LINES.len();
                self.say(HIT_LINES[pick]);
            }
        }
        if self.t >= self.duration() {
            self.t = self.duration();
            self.since_end = 0.0;
            if self.part == Part::One {
                // the checkpoint: part two is for after this
                self.phase = Phase::Checkpoint;
                self.say("...Fine. Then I stop holding back.");
            } else {
                self.phase = Phase::Won;
                self.say(if self.part == Part::One { "IMPOSSIBLE." } else { "...Then rise. And see what it costs." });
            }
        }
    }
}

pub fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Distance from `p` to the segment `a`-`b`.
pub fn distance_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let len2 = abx * abx + aby * aby;
    if len2 <= f32::EPSILON {
        return dist(p, a);
    }
    let t = (((p.0 - a.0) * abx + (p.1 - a.1) * aby) / len2).clamp(0.0, 1.0);
    dist(p, (a.0 + abx * t, a.1 + aby * t))
}

/// Whether a circle touches a polygon (inside it, or within `r` of its edge).
pub fn circle_hits_polygon(centre: (f32, f32), r: f32, polygon: &[(f32, f32)]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    if father::inside(centre, polygon) {
        return true;
    }
    (0..polygon.len()).any(|i| distance_to_segment(centre, polygon[i], polygon[(i + 1) % polygon.len()]) <= r)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STILL: Input = Input { dx: 0.0, dy: 0.0, leap: false };

    fn run(game: &mut Game, seconds: f32, input: Input) {
        let mut t = 0.0;
        while t < seconds {
            game.step(1.0 / 60.0, input);
            t += 1.0 / 60.0;
        }
    }

    #[test]
    fn the_hands_speed_up_and_turn_opposite_ways() {
        let a = hands(0.0);
        let b = hands(30.0);
        let c = hands(60.0);
        assert!(b.minute > a.minute && c.minute > b.minute);
        assert!(b.hour < a.hour && c.hour < b.hour);
        // the second half of the minute is faster than the first
        assert!(c.minute - b.minute > b.minute - a.minute);
    }

    #[test]
    fn the_second_hand_is_warned_then_live() {
        assert!(!hands(5.0).second_shown);
        assert!(hands(13.0).second_shown && !hands(13.0).second_live);
        assert!(hands(16.0).second_live);
    }

    #[test]
    fn a_circle_touches_a_polygon_inside_or_near_its_edge() {
        let square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(circle_hits_polygon((5.0, 5.0), 1.0, &square));
        assert!(circle_hits_polygon((12.0, 5.0), 3.0, &square));
        assert!(!circle_hits_polygon((20.0, 5.0), 3.0, &square));
        assert!(!circle_hits_polygon((5.0, 5.0), 1.0, &square[..2]));
    }

    #[test]
    fn distance_to_a_segment_covers_the_ends() {
        assert!((distance_to_segment((5.0, 3.0), (0.0, 0.0), (10.0, 0.0)) - 3.0).abs() < 1e-5);
        assert!((distance_to_segment((-4.0, 3.0), (0.0, 0.0), (10.0, 0.0)) - 5.0).abs() < 1e-5);
        assert!((distance_to_segment((1.0, 1.0), (2.0, 2.0), (2.0, 2.0)) - 2f32.sqrt()).abs() < 1e-5);
    }

    #[test]
    fn the_cat_walks_and_stays_on_the_clock() {
        let mut g = Game::new(1);
        let start = g.cat;
        g.step(0.05, Input { dx: 1.0, dy: 0.0, leap: false });
        assert!(g.cat.0 > start.0);
        assert_eq!(g.facing, 1.0);
        g.step(0.05, Input { dx: -1.0, dy: 0.0, leap: false });
        assert_eq!(g.facing, -1.0);
        for _ in 0..600 {
            g.step(0.05, Input { dx: 1.0, dy: -1.0, leap: false });
            assert!(dist(g.cat, CENTRE) <= FIELD_RADIUS + 0.01);
            if g.phase != Phase::Playing {
                break;
            }
        }
    }

    #[test]
    fn walking_diagonally_is_not_faster() {
        let mut a = Game::new(1);
        let mut b = Game::new(1);
        a.step(0.05, Input { dx: 1.0, dy: 0.0, leap: false });
        b.step(0.05, Input { dx: 1.0, dy: 1.0, leap: false });
        let (da, db) = (dist(a.cat, Game::new(1).cat), dist(b.cat, Game::new(1).cat));
        assert!((da - db).abs() < 0.01, "{} vs {}", da, db);
    }

    #[test]
    fn standing_on_the_hub_costs_a_life() {
        let mut g = Game::new(3);
        g.cat = CENTRE;
        g.step(0.016, STILL);
        assert_eq!(g.lives, START_LIVES - 1);
        assert!(g.hurt());
        assert_eq!(g.hits, 1);
        // untouchable for a moment after
        g.step(0.016, STILL);
        assert_eq!(g.lives, START_LIVES - 1);
    }

    #[test]
    fn a_leap_makes_you_untouchable() {
        let mut g = Game::new(3);
        g.cat = CENTRE;
        g.step(0.016, Input { dx: 0.0, dy: 0.0, leap: true });
        assert_eq!(g.lives, START_LIVES);
        assert!(g.leaping().is_some());
        // the leap ends and the hub gets you
        run(&mut g, LEAP_TIME + 0.1, STILL);
        assert_eq!(g.lives, START_LIVES - 1);
    }

    #[test]
    fn a_leap_needs_to_recover() {
        let mut g = Game::new(3);
        g.cat = (CENTRE.0 + 300.0, CENTRE.1 + 300.0 * 0.0 + 250.0);
        g.step(0.016, Input { dx: 0.0, dy: 0.0, leap: true });
        assert!(g.leaping().is_some());
        assert!(g.leap_ready() < 0.1);
        run(&mut g, LEAP_TIME + 0.05, STILL);
        assert!(g.leaping().is_none());
        // too soon for another
        g.step(0.016, Input { dx: 0.0, dy: 0.0, leap: true });
        assert!(g.leaping().is_none());
        run(&mut g, Rules::of(Difficulty::Hard).leap_cooldown, STILL);
        assert!(g.leap_ready() > 0.99);
    }

    #[test]
    fn a_far_corner_is_safe_early_on() {
        // right at the edge of the clock, below the middle, nothing reaches you for a moment
        let mut g = Game::new(5);
        g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
        g.step(0.05, STILL);
        assert_eq!(g.lives, START_LIVES);
    }

    #[test]
    fn sand_is_caught_and_every_few_grains_give_a_life() {
        let mut g = Game::new(7);
        g.lives = 2;
        for i in 0..SAND_PER_LIFE {
            g.grains.push(Grain { start: g.cat, target: g.cat, age: 1.0 + i as f32 * 0.0 });
        }
        // put the cat somewhere safe, then let it collect
        g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
        for gr in &mut g.grains {
            gr.start = g.cat;
            gr.target = g.cat;
        }
        g.step(0.016, STILL);
        assert_eq!(g.sand, SAND_PER_LIFE);
        assert_eq!(g.lives, 3);
    }

    #[test]
    fn lives_are_capped() {
        let mut g = Game::new(7);
        g.lives = MAX_LIVES;
        g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
        for _ in 0..SAND_PER_LIFE {
            g.grains.push(Grain { start: g.cat, target: g.cat, age: 1.0 });
        }
        g.step(0.016, STILL);
        assert_eq!(g.lives, MAX_LIVES);
    }

    #[test]
    fn grains_fly_out_of_the_hourglass_and_stop() {
        let g = Grain { start: father::HOURGLASS, target: (600.0, 600.0), age: 0.0 };
        assert_eq!(g.pos(), father::HOURGLASS);
        let landed = Grain { age: GRAIN_TRAVEL + 1.0, ..g };
        assert_eq!(landed.pos(), (600.0, 600.0));
        assert!(g.strength() < 0.01);
        assert!(Grain { age: 2.0, ..g }.strength() > 0.99);
        assert!(Grain { age: GRAIN_LIFE - 0.1, ..g }.strength() < 0.2);
    }

    #[test]
    fn sand_keeps_coming_and_nothing_piles_up() {
        let mut g = Game::new(9);
        g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
        let mut seen = 0;
        for _ in 0..(20 * 60) {
            g.step(1.0 / 60.0, STILL);
            seen = seen.max(g.grains.len());
            if g.phase != Phase::Playing {
                break;
            }
            // keep it alive for the test
            g.lives = START_LIVES;
            g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
        }
        assert!(seen >= 2 && seen <= 14, "saw {} at once", seen);
    }

    #[test]
    fn a_win_is_reported_once_and_a_cheat_is_seen() {
        let mut g = Game::new(3);
        assert!(!g.take_win() && !g.cheated());
        g.phase = Phase::Won;
        assert!(g.take_win());
        assert!(!g.take_win() && !g.take_death(), "once, and a game is not both won and lost");
        assert!(Game::with_cheats(1, Difficulty::Hard, Cheats::NONE.with(crate::games::Cheat::BorrowedTime, true)).cheated());
    }

    #[test]
    fn losing_all_lives_ends_the_game() {
        let mut g = Game::new(11);
        g.lives = 1;
        g.cat = CENTRE;
        g.step(0.016, STILL);
        assert_eq!(g.phase, Phase::Lost);
        assert_eq!(g.lives, 0);
        let t = g.t;
        g.step(0.016, STILL);
        assert_eq!(g.t, t, "time stops when it is over");
        assert!(g.since_end > 0.0);
        assert_eq!(g.line().unwrap().0, "Time always wins.");
    }

    #[test]
    fn surviving_the_minute_wins() {
        let mut g = Game::new(13);
        // stay at the rim and never be hurt
        for _ in 0..(61 * 60) {
            g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
            g.lives = START_LIVES;
            g.step(1.0 / 60.0, STILL);
            if g.phase != Phase::Playing {
                break;
            }
        }
        assert_eq!(g.phase, Phase::Checkpoint, "part one ends at the checkpoint, not in a win");
        assert_eq!(g.t, g.duration());
        assert!(!g.take_win() && !g.take_death());
        assert!((g.progress() - 1.0).abs() < 1e-6);
        // part two is the win
        assert!(g.take_checkpoint() && !g.take_checkpoint(), "the checkpoint is reported once");
        g.begin_part_two();
        assert_eq!((g.part, g.phase, g.t), (Part::Two, Phase::Playing, 0.0));
        for _ in 0..(45 * 60) {
            g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
            g.lives = START_LIVES;
            g.step(1.0 / 60.0, STILL);
            if g.phase != Phase::Playing {
                break;
            }
        }
        assert_eq!(g.phase, Phase::Won);
        assert_eq!(g.t, g.duration());
        assert_eq!(g.line().unwrap().0, "...Then rise. And see what it costs.");
        assert!((g.progress() - 1.0).abs() < 1e-6);
        assert_eq!(g.ascension(), 1.0);
    }

    #[test]
    fn part_two_keeps_speeding_up_and_part_one_does_not() {
        // the hands' own time runs faster the further through part two you are
        let rate = |progress_t: f32| {
            let mut g = Game::start(1, Difficulty::Hard, Cheats::NONE, Part::Two);
            g.t = progress_t;
            g.hurt_left = 99.0;
            g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
            let before = g.hand_t;
            g.step(0.05, STILL);
            (g.hand_t - before) / 0.05
        };
        let (early, late) = (rate(2.0), rate(38.0));
        assert!(early < 1.2 && late > 2.2, "{early} then {late}");
        assert!(rate(10.0) < rate(20.0) && rate(20.0) < rate(30.0), "and it never eases off");
        let mut one = Game::new(1);
        one.hurt_left = 99.0;
        one.t = 30.0;
        let before = one.hand_t;
        one.step(0.05, STILL);
        assert!(((one.hand_t - before) / 0.05 - 1.0).abs() < 1e-3, "part one keeps the clock's own pace");
        // Borrowed Time still slows it
        let mut slow = Game::start(1, Difficulty::Hard, Cheats::NONE.with(Cheat::BorrowedTime, true), Part::Two);
        slow.hurt_left = 99.0;
        slow.t = 38.0;
        let before = slow.hand_t;
        slow.step(0.05, STILL);
        assert!((slow.hand_t - before) / 0.05 < 1.5);
    }

    #[test]
    fn the_test_switch_costs_no_lives_flashes_and_counts_as_a_cheat() {
        let mut g = Game::new(9);
        assert!(!g.cheated());
        g.invincible = true;
        assert!(g.cheated(), "so it pays no tokens");
        g.cat = CENTRE;
        g.step(0.016, STILL);
        assert_eq!((g.lives, g.phase), (START_LIVES, Phase::Playing));
        assert!(g.hurt() && g.hits == 1, "it still flashes");
        for _ in 0..600 {
            g.cat = CENTRE;
            g.step(1.0 / 60.0, STILL);
        }
        assert_eq!(g.lives, START_LIVES, "never loses");
        assert_eq!(g.phase, Phase::Playing);
    }

    #[test]
    fn in_part_two_the_cat_rises_and_father_time_tries_to_stop_him() {
        let mut g = Game::new(3);
        assert_eq!(g.ascension(), 0.0, "nothing rises in part one");
        g.t = g.duration() - 0.01;
        g.step(0.02, STILL);
        g.begin_part_two();
        assert_eq!(g.ascension(), 0.0);
        let mut said: Vec<&str> = Vec::new();
        let mut last = 0.0;
        for _ in 0..(45 * 60) {
            g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
            g.lives = START_LIVES;
            g.step(1.0 / 60.0, STILL);
            assert!(g.ascension() >= last, "he only climbs");
            last = g.ascension();
            if let Some((text, _)) = g.line() {
                if said.last() != Some(&text) {
                    said.push(text);
                }
            }
            if g.phase != Phase::Playing {
                break;
            }
        }
        let want: Vec<&str> = PART_TWO_LINES.iter().map(|l| l.1).collect();
        assert_eq!(&said[..want.len()], &want[..], "his lines come in order as the cat climbs: {said:?}");
        assert_eq!(said.last(), Some(&"...Then rise. And see what it costs."));
        assert_eq!(g.ascension(), 1.0);
    }

    #[test]
    fn part_two_is_faster_and_harder_than_part_one() {
        for d in Difficulty::ALL {
            let r = Rules::of(d);
            assert!(r.part_two > 0.0 && r.part_two < r.duration, "{d:?}: shorter but fiercer");
        }
        // the hands are already fast at the first second, and the thin hand is out from the start
        let speed = |h: fn(f32) -> Hands| h(1.0).minute - h(0.0).minute;
        assert!(speed(hands_part_two) > speed(hands) * 1.4, "{} vs {}", speed(hands_part_two), speed(hands));
        assert!(hands_part_two(0.0).second_shown && !hands_part_two(0.0).second_live);
        assert!(hands_part_two(PART_TWO_SECOND_LIVE).second_live);
        // and a hand turns the other way, out later
        assert!(!hands_part_two(1.0).counter_shown && hands_part_two(COUNTER_SHOWN).counter_shown);
        assert!(!hands_part_two(COUNTER_SHOWN).counter_live && hands_part_two(COUNTER_LIVE).counter_live);
        assert!(hands_part_two(9.0).counter < hands_part_two(8.0).counter, "it turns backwards");
        assert!(hands_part_two(9.0).second > hands_part_two(8.0).second, "the thin one turns forwards");
        // part one has no such hand
        assert!(!hands(50.0).counter_shown && !hands(50.0).counter_live);
    }

    #[test]
    fn the_hand_that_turns_back_hurts_only_once_it_is_live() {
        let mut g = Game::start(5, Difficulty::Hard, Cheats::NONE, Part::Two);
        assert_eq!(g.part, Part::Two);
        g.hand_t = COUNTER_SHOWN + 0.5;
        let (seg, live) = g.counter_segment().unwrap();
        assert!(!live);
        // stand on the middle of the thin hand: dim, so harmless
        let mid = ((seg[0].0 + seg[1].0) / 2.0, (seg[0].1 + seg[1].1) / 2.0);
        assert!(distance_to_segment(mid, seg[0], seg[1]) < 1e-3);
        // (the other hands may be near, so only the counter segment's own test is checked here)
        g.hand_t = COUNTER_LIVE + 0.5;
        let (seg, live) = g.counter_segment().unwrap();
        assert!(live);
        let mid = ((seg[0].0 + seg[1].0) / 2.0, (seg[0].1 + seg[1].1) / 2.0);
        g.cat = mid;
        assert!(g.touching_a_hand(), "a live counter hand touches");
        g.part = Part::One;
        g.hand_t = 50.0;
        assert!(g.counter_segment().is_none(), "part one has none");
    }

    #[test]
    fn the_checkpoint_keeps_part_two_for_a_retry_and_pays_fairly() {
        let mut g = Game::new(21);
        g.t = g.duration() - 0.01;
        g.step(0.02, STILL);
        assert_eq!(g.phase, Phase::Checkpoint);
        assert_eq!(g.line().unwrap().0, "...Fine. Then I stop holding back.");
        // a game started from the checkpoint is part two with three lives
        let again = Game::start(22, Difficulty::Hard, Cheats::NONE, Part::Two);
        assert_eq!((again.part, again.phase, again.lives), (Part::Two, Phase::Playing, START_LIVES));
        assert_eq!(again.duration(), Rules::of(Difficulty::Hard).part_two);
        assert_eq!(Game::start(22, Difficulty::Hard, Cheats::NONE, Part::One).part, Part::One);
        // dying in part two pays only part two's time; winning pays both
        let mut lose = Game::start(23, Difficulty::Hard, Cheats::NONE, Part::Two);
        lose.lives = 1;
        lose.cat = CENTRE;
        lose.step(0.016, STILL);
        assert_eq!(lose.phase, Phase::Lost);
        assert!(lose.paid_secs() < 1.0);
        let mut win = Game::start(24, Difficulty::Hard, Cheats::NONE, Part::Two);
        win.t = win.duration() - 0.01;
        win.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
        win.hurt_left = 99.0;
        win.step(0.02, STILL);
        assert_eq!(win.phase, Phase::Won);
        let r = Rules::of(Difficulty::Hard);
        assert!((win.paid_secs() - (r.duration + r.part_two)).abs() < 0.01);
    }

    #[test]
    fn going_on_from_the_checkpoint_clears_the_clock_and_keeps_extra_lives() {
        let mut g = Game::new(31);
        g.t = g.duration() - 0.01;
        g.lives = 5;
        g.hurt_left = 99.0;
        g.step(0.02, STILL);
        assert_eq!(g.phase, Phase::Checkpoint);
        g.begin_part_two();
        assert_eq!((g.lives, g.t, g.hand_t, g.grains.len(), g.hurt()), (5, 0.0, 0.0, 0, false));
        assert_eq!(g.cat, (CENTRE.0, CENTRE.1 + 250.0));
        assert!(!g.said_start && !g.said_half);
    }

    /// Plays to the end at the rim, never hurt; the time it took.
    fn survive(d: Difficulty) -> (Game, f32) {
        let mut g = Game::with_difficulty(13, d);
        for _ in 0..(70 * 60) {
            g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
            g.lives = START_LIVES;
            g.step(1.0 / 60.0, STILL);
            if g.phase != Phase::Playing {
                break;
            }
        }
        let t = g.t;
        (g, t)
    }

    #[test]
    fn the_difficulty_sets_how_long_you_survive_and_how_fast_the_leap_comes_back() {
        let (easy, normal, hard) = (Rules::of(Difficulty::Easy), Rules::of(Difficulty::Normal), Rules::of(Difficulty::Hard));
        assert_eq!((easy.duration, normal.duration, hard.duration), (30.0, 45.0, 60.0));
        assert_eq!((easy.part_two, normal.part_two, hard.part_two), (20.0, 30.0, 40.0));
        // easier means a quicker leap, and Hard is a touch quicker than the old 1.1 seconds
        assert!(easy.leap_cooldown < normal.leap_cooldown && normal.leap_cooldown < hard.leap_cooldown);
        assert!(easy.leap_cooldown <= 0.5 && hard.leap_cooldown < 1.1 && hard.leap_cooldown > 0.9);
        for d in Difficulty::ALL {
            let (g, t) = survive(d);
            assert_eq!(g.phase, Phase::Checkpoint, "{d:?}");
            assert_eq!(g.difficulty, d);
            assert!((t - Rules::of(d).duration).abs() < 0.001 && (g.progress() - 1.0).abs() < 1e-6, "{d:?}: {t}");
        }
        assert_eq!(Game::new(1).difficulty, Difficulty::Hard, "the plain game is the hard one");
    }

    #[test]
    fn the_leap_bar_fills_at_the_speed_of_the_difficulty() {
        for d in Difficulty::ALL {
            let mut g = Game::with_difficulty(3, d);
            g.step(0.016, Input { dx: 0.0, dy: 0.0, leap: true });
            assert!(g.leap_ready() < 0.05);
            let cool = Rules::of(d).leap_cooldown;
            run(&mut g, cool * 0.5, STILL);
            assert!((g.leap_ready() - 0.5).abs() < 0.12, "{d:?}: halfway after half the time, {}", g.leap_ready());
            run(&mut g, cool * 0.6, STILL);
            assert!(g.leap_ready() > 0.99, "{d:?}");
        }
        // after the same time the easy leap is back and the hard one is not
        let ready = |d| {
            let mut g = Game::with_difficulty(3, d);
            g.step(0.016, Input { dx: 0.0, dy: 0.0, leap: true });
            run(&mut g, 0.6, STILL);
            g.leap_ready()
        };
        assert!(ready(Difficulty::Easy) > 0.99 && ready(Difficulty::Hard) < 0.7);
    }

    #[test]
    fn he_says_not_bad_halfway_whatever_the_length() {
        for d in Difficulty::ALL {
            let mut g = Game::with_difficulty(13, d);
            let half = Rules::of(d).duration / 2.0;
            for _ in 0..((half * 60.0) as usize - 30) {
                g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
                g.lives = START_LIVES;
                g.step(1.0 / 60.0, STILL);
            }
            assert!(!g.said_half, "{d:?} not yet");
            for _ in 0..60 {
                g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
                g.lives = START_LIVES;
                g.step(1.0 / 60.0, STILL);
            }
            assert!(g.said_half, "{d:?}");
        }
    }

    #[test]
    fn a_death_is_reported_once() {
        let mut g = Game::new(5);
        assert!(!g.take_death(), "still playing");
        g.lives = 1;
        g.cat = CENTRE;
        g.step(0.016, STILL);
        assert_eq!(g.phase, Phase::Lost);
        assert!(g.take_death());
        assert!(!g.take_death(), "only once");
        // and a win is no death
        let (mut won, _) = survive(Difficulty::Easy);
        assert!(!won.take_death());
    }

    #[test]
    fn the_same_seed_plays_out_the_same_way() {
        let play = |seed| {
            let mut g = Game::new(seed);
            for k in 0..600 {
                let input = Input { dx: ((k / 40) % 3) as f32 - 1.0, dy: ((k / 55) % 3) as f32 - 1.0, leap: k % 90 == 0 };
                g.step(1.0 / 60.0, input);
            }
            (g.cat, g.lives, g.sand, g.grains.len())
        };
        assert_eq!(play(4), play(4));
    }

    #[test]
    fn father_time_speaks_at_the_start_after_hits_and_at_the_half() {
        let mut g = Game::new(2);
        g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
        assert!(g.line().is_none());
        run(&mut g, 1.0, STILL);
        assert_eq!(g.line().unwrap().0, "You dare enter my hour?");
        // (the rim is within reach of the minute hand, so make the cat untouchable for this part)
        g.hurt_left = 99.0;
        run(&mut g, 3.0, STILL);
        assert!(g.line().is_none(), "lines fade away");
        g.hurt_left = 0.0;
        g.cat = CENTRE;
        g.step(0.016, STILL);
        assert!(HIT_LINES.contains(&g.line().unwrap().0));
    }

    #[test]
    fn a_long_frame_does_not_skip_through_the_game() {
        let mut g = Game::new(2);
        g.step(5.0, STILL);
        assert!(g.t <= 0.05 + 1e-6);
    }

    #[test]
    fn borrowed_time_runs_the_hands_at_half_speed_but_the_time_to_survive_is_the_same() {
        let mut slow = Game::with_cheats(3, Difficulty::Hard, Cheats::NONE.with(Cheat::BorrowedTime, true));
        let mut normal = Game::with_difficulty(3, Difficulty::Hard);
        for _ in 0..600 {
            slow.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
            normal.cat = slow.cat;
            slow.hurt_left = 99.0;
            normal.hurt_left = 99.0;
            slow.step(1.0 / 60.0, STILL);
            normal.step(1.0 / 60.0, STILL);
        }
        assert!((slow.t - normal.t).abs() < 1e-4, "the same clock for the game");
        assert!((slow.hand_t - normal.hand_t * SLOW_HANDS).abs() < 1e-3);
        assert_eq!(slow.hands_now(), hands(slow.hand_t));
        assert!(slow.hands_now().minute < normal.hands_now().minute);
        // and the win still comes at the same time
        let d = Rules::of(Difficulty::Hard).duration;
        let mut g = Game::with_cheats(3, Difficulty::Hard, Cheats::NONE.with(Cheat::BorrowedTime, true));
        for _ in 0..((d * 60.0) as usize + 5) {
            g.cat = (CENTRE.0, CENTRE.1 + FIELD_RADIUS - CAT_RADIUS);
            g.hurt_left = 99.0;
            g.step(1.0 / 60.0, STILL);
        }
        assert_eq!(g.phase, Phase::Checkpoint);
    }

    #[test]
    fn without_the_cheat_the_hands_keep_the_time_of_the_game() {
        let mut g = Game::new(5);
        g.hurt_left = 99.0;
        run(&mut g, 4.0, STILL);
        assert!((g.hand_t - g.t).abs() < 1e-4);
    }
}
