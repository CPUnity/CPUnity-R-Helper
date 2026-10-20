//! Game 2, the Gold Edition's game: you are the staff of time, held in a golden fist, flying like a
//! ship in Asteroids (turn, thrust, drift) and shooting bolts from its head. Angel cats come down from
//! heaven (archers aim arrows at you, harpists fan out notes, trumpeters blast straight down). Fell
//! enough of them and the golden cat king himself descends, in a heavenly coliseum; beat him and
//! he dies there, opening a Void that pulls everyone in, you too. That is what unlocks the Void.
//!
//! Everything here is pure (no drawing, no clock): `Game::step` advances it, so it is tested.
//! Coordinates are in the arena: `ARENA` units, x to the right, y down, the angels come in from
//! the top.

use crate::games::{Cheat, Cheats, Difficulty};
use std::f32::consts::{PI, TAU};

pub type Pt = (f32, f32);

/// The arena, in its own units. The painter scales it to fit the window.
pub const ARENA: Pt = (1200.0, 800.0);

/// What a difficulty changes: how many lives you have, how many angels fall before the king comes,
/// how tough he is, how fast the angels are, how much they shoot, and how quickly you can fire.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rules {
    pub lives: u8,
    pub kills_for_king: u32,
    pub king_hp: u32,
    /// 1 is full speed; lower is slower.
    pub enemy_speed: f32,
    /// How long they wait between shots, compared with Hard (1): more is calmer.
    pub enemy_fire_gap: f32,
    /// Seconds between your bolts.
    pub fire_cooldown: f32,
}

impl Rules {
    pub fn of(d: Difficulty) -> Rules {
        match d {
            Difficulty::Easy => Rules { lives: 5, kills_for_king: 12, king_hp: 24, enemy_speed: 0.7, enemy_fire_gap: 1.8, fire_cooldown: 0.16 },
            Difficulty::Normal => Rules { lives: 4, kills_for_king: 18, king_hp: 36, enemy_speed: 0.85, enemy_fire_gap: 1.3, fire_cooldown: 0.2 },
            Difficulty::Hard => Rules { lives: 3, kills_for_king: 25, king_hp: 50, enemy_speed: 1.0, enemy_fire_gap: 1.0, fire_cooldown: 0.24 },
        }
    }
}

pub const PLAYER_RADIUS: f32 = 17.0;
pub const ANGEL_RADIUS: f32 = 36.0;
pub const KING_RADIUS: f32 = 95.0;
const BOLT_RADIUS: f32 = 6.0;
const TURN_SPEED: f32 = 3.6;
const THRUST: f32 = 640.0;
/// How much of the speed is lost every second.
const DRAG: f32 = 0.9;
const MAX_SPEED: f32 = 480.0;
const BOLT_SPEED: f32 = 980.0;
const BOLT_LIFE: f32 = 1.0;
/// How high up the staff can fly, and how low.
pub const PLAYER_TOP: f32 = 240.0;
pub const PLAYER_BOTTOM: f32 = ARENA.1 - 36.0;
/// Untouchable for this long after a hit.
pub const INVULN: f32 = 2.2;
const MAX_ANGELS: usize = 8;
const LINE_LIFE: f32 = 3.2;
/// How long the sky is silent before an angel has something to say.
const IDLE_GAP: f32 = 9.0;
/// How long the king takes to come down, and where he hovers.
pub const KING_ENTER: f32 = 3.0;
const KING_Y: f32 = 175.0;
const BURST_LIFE: f32 = 0.6;
const BIG_BURST_LIFE: f32 = 1.6;

/// What heaven says. Some of it is for the mortal at the keyboard, some for the old man whose staff
/// it is (Father Time); the king shouts. Every line fits the screen in one go.
pub const START: &str = "A mortal! And he brought the old man.";
pub const HIT: [&str; 12] = [
    "Ouch. Did the mortal flinch, or the staff?",
    "That one was aimed, you know.",
    "Careful, old man. Your hip.",
    "Skill issue. Heaven-certified.",
    "Press a different key. Any key.",
    "Time flies. You, less so.",
    "We have seen toddlers aim better.",
    "Is that a keyboard or a cutting board?",
    "Your reflexes called. They quit.",
    "The arrow keys are the ones with arrows.",
    "You dodge like a grandfather clock.",
    "Out of time, old man.",
];
pub const LAST_LIFE: [&str; 2] = ["One life left. Make it dramatic.", "Last life, mortal. Say something profound."];
pub const STREAK: [&str; 6] = [
    "Rude. That one had a family.",
    "Stop shooting the choir!",
    "That was Gabriel's cousin.",
    "Do you know the paperwork that causes?",
    "Heaven has more where that came from.",
    "Centuries of practice. Gone in a blink.",
];
pub const CHATTER: [&str; 12] = [
    "Are you even trying, mortal?",
    "Father Time. More like Father Slow.",
    "Your chair is embarrassed for you.",
    "Heaven has your screen time. It is a lot.",
    "Even the harpists feel sorry for you.",
    "Time heals all wounds. Not yours.",
    "A staff of time, and still running late.",
    "Blink less. You are missing everything.",
    "Nice beard, old man. Is it load-bearing?",
    "Try using both hands.",
    "Retire, old man. Everyone else has.",
    "Mortal, the keys are not decorative.",
];
pub const KING_HALF: [&str; 3] = ["NOW YOU HAVE MY ATTENTION.", "YOU HIT LIKE A DEAD CLOCK.", "KNEEL, MORTAL. KNEEL, OLD MAN."];
pub const LOSS: [&str; 7] = [
    "Time is up. For both of you.",
    "The old man is dead. The mortal was slow.",
    "Tick. Tock. Tick. That was you.",
    "Try again. Try harder. Try anything.",
    "Even the trumpeter is embarrassed.",
    "Skill issue. Documented.",
    "Father Time, defeated by his own staff.",
];

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Input {
    /// -1 turns left, 1 right
    pub turn: f32,
    pub thrust: bool,
    pub fire: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Playing,
    Won,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Player {
    pub pos: Pt,
    pub vel: Pt,
    /// Radians; 0 points right and -PI/2 straight up (y is down).
    pub angle: f32,
    pub thrusting: bool,
    /// Seconds left of being untouchable.
    pub invuln: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bolt {
    pub pos: Pt,
    pub vel: Pt,
    pub age: f32,
}

/// What an angel cat is: the cell of the picture (`angels::uv`) it is drawn from.
pub const ARCHER: u8 = 0;
pub const HARPIST: u8 = 1;
pub const TRUMPETER: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angel {
    pub kind: u8,
    pub pos: Pt,
    pub age: f32,
    pub hp: u8,
    /// Seconds left of the flash after a hit.
    pub flash: f32,
    base_x: f32,
    sway: f32,
    rate: f32,
    phase: f32,
    speed: f32,
    shot_in: f32,
    gap: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotKind {
    Arrow,
    Note,
    Blast,
    Orb,
}

impl ShotKind {
    pub fn radius(self) -> f32 {
        match self {
            ShotKind::Arrow => 7.0,
            ShotKind::Note => 9.0,
            ShotKind::Blast => 10.0,
            ShotKind::Orb => 12.0,
        }
    }
}

/// Something the angels or the king shot at you.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shot {
    pub pos: Pt,
    pub vel: Pt,
    pub kind: ShotKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct King {
    pub pos: Pt,
    pub hp: u32,
    pub max_hp: u32,
    pub age: f32,
    pub flash: f32,
    shot_in: f32,
    pattern: u32,
}

impl King {
    /// He can be hurt once he has come all the way down.
    pub fn arrived(&self) -> bool {
        self.age >= KING_ENTER
    }
}

/// A puff where something blew up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Burst {
    pub pos: Pt,
    pub age: f32,
    pub big: bool,
}

impl Burst {
    /// 0..1 how far through it is.
    pub fn progress(&self) -> f32 {
        (self.age / if self.big { BIG_BURST_LIFE } else { BURST_LIFE }).clamp(0.0, 1.0)
    }
}

/// With Rapid fire the staff waits this fraction of the usual time between shots.
pub const RAPID_FIRE_FACTOR: f32 = 0.3;

pub struct Game {
    pub t: f32,
    pub difficulty: Difficulty,
    pub player: Player,
    pub lives: u8,
    pub score: u32,
    pub kills: u32,
    pub phase: Phase,
    pub angels: Vec<Angel>,
    pub bolts: Vec<Bolt>,
    pub shots: Vec<Shot>,
    pub king: Option<King>,
    pub bursts: Vec<Burst>,
    /// How long ago the game ended (win or loss).
    pub since_end: f32,
    rules: Rules,
    fire_cool: f32,
    next_angel: f32,
    rng: u32,
    line: Option<(&'static str, f32)>,
    /// A cheat for testing: hits cost nothing (toggled with I in the game).
    pub infinite: bool,
    /// The Rapid fire cheat.
    rapid_fire: bool,
    said_start: bool,
    said_half: bool,
    quiet: f32,
    last_said: &'static str,
    death_reported: bool,
}

impl Game {
    /// A game on Hard, which is how it is before there is a choice.
    pub fn new(seed: u32) -> Game {
        Game::with_difficulty(seed, Difficulty::Hard)
    }

    pub fn with_difficulty(seed: u32, difficulty: Difficulty) -> Game {
        Game::with_cheats(seed, difficulty, Cheats::NONE)
    }

    /// A game with the cheats the player has switched on (Options > Games, once they are earned).
    pub fn with_cheats(seed: u32, difficulty: Difficulty, cheats: Cheats) -> Game {
        let rules = Rules::of(difficulty);
        Game {
            t: 0.0,
            difficulty,
            player: Player { pos: (ARENA.0 / 2.0, ARENA.1 - 110.0), vel: (0.0, 0.0), angle: -PI / 2.0, thrusting: false, invuln: 0.0 },
            lives: rules.lives,
            score: 0,
            kills: 0,
            phase: Phase::Playing,
            angels: Vec::new(),
            bolts: Vec::new(),
            shots: Vec::new(),
            king: None,
            bursts: Vec::new(),
            since_end: 0.0,
            rules,
            fire_cool: 0.0,
            next_angel: 1.2,
            rng: seed.wrapping_mul(2_654_435_761) | 1,
            line: None,
            infinite: cheats.has(Cheat::Invincible),
            rapid_fire: cheats.has(Cheat::RapidFire),
            said_start: false,
            said_half: false,
            quiet: 0.0,
            last_said: "",
            death_reported: false,
        }
    }

    pub fn rules(&self) -> Rules {
        self.rules
    }

    /// How long the staff takes to fire again (a fraction of that with Rapid fire).
    pub fn fire_gap(&self) -> f32 {
        self.rules.fire_cooldown * if self.rapid_fire { RAPID_FIRE_FACTOR } else { 1.0 }
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

    /// Whether a cheat is on (Rapid fire, Invincible, or the test switch): a run with one pays no tokens.
    pub fn cheated(&self) -> bool {
        self.rapid_fire || self.infinite
    }

    /// Whether the king has come (the angels are done).
    pub fn king_out(&self) -> bool {
        self.king.is_some()
    }

    /// 0..1: how many of the angels you needed are down, or, once the king is out, how much of him
    /// is gone.
    pub fn progress(&self) -> f32 {
        match &self.king {
            Some(k) => 1.0 - k.hp as f32 / k.max_hp.max(1) as f32,
            None => (self.kills as f32 / self.rules.kills_for_king.max(1) as f32).clamp(0.0, 1.0),
        }
    }

    /// What is being said and how clearly (0..1), if anything.
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
        self.last_said = text;
        self.quiet = 0.0;
    }

    /// Says one of `pool` (never the same one twice in a row).
    fn say_from(&mut self, pool: &'static [&'static str]) {
        let mut i = (self.random() * pool.len() as f32) as usize % pool.len();
        if pool.len() > 1 && pool[i] == self.last_said {
            i = (i + 1) % pool.len();
        }
        self.say(pool[i]);
    }

    /// Where the bolts come out: the head of the staff.
    pub fn staff_tip(&self) -> Pt {
        staff_tip(self.player.pos, self.player.angle)
    }

    fn burst(&mut self, pos: Pt, big: bool) {
        self.bursts.push(Burst { pos, age: 0.0, big });
    }

    /// The staff was hit (nothing happens while it is untouchable).
    fn hurt_player(&mut self) {
        if self.player.invuln > 0.0 || self.phase != Phase::Playing {
            return;
        }
        if self.infinite {
            // the cheat: it still hurts to look at, but it costs nothing
            self.player.invuln = 0.8;
            self.player.vel = (0.0, 0.0);
            let pos = self.player.pos;
            self.burst(pos, false);
            return;
        }
        self.lives = self.lives.saturating_sub(1);
        self.player.invuln = INVULN;
        self.player.vel = (0.0, 0.0);
        let pos = self.player.pos;
        self.burst(pos, false);
        if self.lives == 0 {
            self.phase = Phase::Lost;
            self.since_end = 0.0;
            self.say_from(&LOSS);
            self.burst(pos, true);
        } else if self.lives == 1 {
            self.say_from(&LAST_LIFE);
        } else {
            self.say_from(&HIT);
        }
    }

    fn spawn_angel(&mut self) {
        let kind = (self.random() * 3.0) as u8 % 3;
        let base_x = 120.0 + self.random() * (ARENA.0 - 240.0);
        let (hp, speed, sway, rate, gap) = match kind {
            ARCHER => (1, 72.0, 70.0, 1.1, 2.4),
            HARPIST => (1, 58.0, 120.0, 0.8, 3.4),
            _ => (2, 46.0, 36.0, 1.3, 3.0),
        };
        let (a, b, c) = (self.random(), self.random(), self.random());
        self.angels.push(Angel {
            kind,
            pos: (base_x, -50.0),
            age: 0.0,
            hp,
            flash: 0.0,
            base_x,
            sway,
            rate: rate * (0.85 + 0.3 * a),
            phase: b * TAU,
            speed: speed * self.rules.enemy_speed * (0.9 + 0.2 * c),
            shot_in: gap * self.rules.enemy_fire_gap * (0.5 + 0.5 * a),
            gap,
        });
    }

    /// A shot's speed, a little slower on the easier settings.
    fn sp(&self, v: f32) -> f32 {
        v * (0.75 + 0.25 * self.rules.enemy_speed)
    }

    fn aim_from(&self, from: Pt) -> Pt {
        let (dx, dy) = (self.player.pos.0 - from.0, self.player.pos.1 - from.1);
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        (dx / len, dy / len)
    }

    fn angel_fires(&mut self, kind: u8, from: Pt) {
        match kind {
            ARCHER => {
                let d = self.aim_from(from);
                let v = self.sp(340.0);
                self.shots.push(Shot { pos: from, vel: (d.0 * v, d.1 * v), kind: ShotKind::Arrow });
            }
            HARPIST => {
                let d = self.aim_from(from);
                let base = d.1.atan2(d.0);
                let v = self.sp(230.0);
                for off in [-0.38_f32, 0.0, 0.38] {
                    let a = base + off;
                    self.shots.push(Shot { pos: from, vel: (a.cos() * v, a.sin() * v), kind: ShotKind::Note });
                }
            }
            _ => {
                let v = self.sp(470.0);
                self.shots.push(Shot { pos: from, vel: (0.0, v), kind: ShotKind::Blast });
            }
        }
    }

    fn king_fires(&mut self) {
        let Some(k) = self.king else { return };
        let from = k.pos;
        match k.pattern % 3 {
            0 => {
                // a ring of orbs
                let v = self.sp(200.0);
                for i in 0..14 {
                    let a = TAU * i as f32 / 14.0 + k.age;
                    self.shots.push(Shot { pos: from, vel: (a.cos() * v, a.sin() * v), kind: ShotKind::Orb });
                }
            }
            1 => {
                // three arrows at you
                let d = self.aim_from(from);
                let base = d.1.atan2(d.0);
                let v = self.sp(330.0);
                for off in [-0.28_f32, 0.0, 0.28] {
                    let a = base + off;
                    self.shots.push(Shot { pos: from, vel: (a.cos() * v, a.sin() * v), kind: ShotKind::Arrow });
                }
            }
            _ => {
                // a curtain of blasts straight down
                let v = self.sp(380.0);
                for i in 0..6 {
                    self.shots.push(Shot { pos: (from.0 + (i as f32 - 2.5) * 110.0, from.1 + 60.0), vel: (0.0, v), kind: ShotKind::Blast });
                }
            }
        }
    }

    /// Advances the game by `dt` seconds (long frames are cut to a twentieth of a second).
    pub fn step(&mut self, dt: f32, input: Input) {
        let dt = dt.clamp(0.0, 0.05);
        if let Some((_, age)) = &mut self.line {
            *age += dt;
        }
        for b in &mut self.bursts {
            b.age += dt;
        }
        self.bursts.retain(|b| b.progress() < 1.0);
        if self.phase != Phase::Playing {
            self.since_end += dt;
            return;
        }
        self.t += dt;
        if !self.said_start && self.t >= 0.8 {
            self.said_start = true;
            self.say(START);
        }

        // when nobody has spoken for a while, somebody does
        let speaking = self.line.is_some_and(|(_, age)| age < LINE_LIFE);
        self.quiet = if speaking { 0.0 } else { self.quiet + dt };
        if self.quiet >= IDLE_GAP {
            self.say_from(&CHATTER);
        }

        // the staff: turn, thrust, drift
        let p = &mut self.player;
        p.angle += input.turn.clamp(-1.0, 1.0) * TURN_SPEED * dt;
        p.thrusting = input.thrust;
        if input.thrust {
            p.vel.0 += p.angle.cos() * THRUST * dt;
            p.vel.1 += p.angle.sin() * THRUST * dt;
        }
        let keep = (-DRAG * dt).exp();
        p.vel = (p.vel.0 * keep, p.vel.1 * keep);
        let speed = (p.vel.0 * p.vel.0 + p.vel.1 * p.vel.1).sqrt();
        if speed > MAX_SPEED {
            p.vel = (p.vel.0 * MAX_SPEED / speed, p.vel.1 * MAX_SPEED / speed);
        }
        p.pos = (p.pos.0 + p.vel.0 * dt, p.pos.1 + p.vel.1 * dt);
        // out of one side and in at the other
        if p.pos.0 < -20.0 {
            p.pos.0 = ARENA.0 + 20.0;
        } else if p.pos.0 > ARENA.0 + 20.0 {
            p.pos.0 = -20.0;
        }
        if p.pos.1 < PLAYER_TOP {
            p.pos.1 = PLAYER_TOP;
            p.vel.1 = p.vel.1.max(0.0);
        } else if p.pos.1 > PLAYER_BOTTOM {
            p.pos.1 = PLAYER_BOTTOM;
            p.vel.1 = p.vel.1.min(0.0);
        }
        p.invuln = (p.invuln - dt).max(0.0);

        // the staff fires
        self.fire_cool = (self.fire_cool - dt).max(0.0);
        if input.fire && self.fire_cool <= 0.0 {
            self.fire_cool = self.fire_gap();
            let a = self.player.angle;
            self.bolts.push(Bolt { pos: self.staff_tip(), vel: (a.cos() * BOLT_SPEED, a.sin() * BOLT_SPEED), age: 0.0 });
        }

        // the angels come down (fewer, and only a few, once the king is out)
        self.next_angel -= dt;
        let (cap, gap) = if self.king.is_some() {
            (3, 7.0)
        } else {
            (MAX_ANGELS, 1.4 - 0.5 * self.progress())
        };
        if self.next_angel <= 0.0 && self.angels.len() < cap {
            self.spawn_angel();
            self.next_angel = gap * (0.8 + 0.4 * self.random());
        } else if self.next_angel <= 0.0 {
            self.next_angel = 0.5;
        }
        let mut fire: Vec<(u8, Pt)> = Vec::new();
        for a in &mut self.angels {
            a.age += dt;
            a.flash = (a.flash - dt).max(0.0);
            a.pos.1 += a.speed * dt;
            a.pos.0 = (a.base_x + (a.age * a.rate + a.phase).sin() * a.sway).clamp(40.0, ARENA.0 - 40.0);
            a.shot_in -= dt;
            if a.shot_in <= 0.0 {
                a.shot_in = a.gap * self.rules.enemy_fire_gap * (0.8 + 0.5 * ((a.age * 7.31).sin() * 0.5 + 0.5));
                if (40.0..520.0).contains(&a.pos.1) {
                    fire.push((a.kind, a.pos));
                }
            }
        }
        for (kind, pos) in fire {
            self.angel_fires(kind, pos);
        }
        self.angels.retain(|a| a.pos.1 < ARENA.1 + 60.0);

        // the king
        if self.king.is_none() && self.kills >= self.rules.kills_for_king {
            self.king = Some(King { pos: (ARENA.0 / 2.0, -150.0), hp: self.rules.king_hp, max_hp: self.rules.king_hp, age: 0.0, flash: 0.0, shot_in: 1.5, pattern: 0 });
            self.say("KNEEL, OLD MAN.");
        }
        let mut king_fires = false;
        if let Some(k) = &mut self.king {
            k.age += dt;
            k.flash = (k.flash - dt).max(0.0);
            if k.age < KING_ENTER {
                let x = (k.age / KING_ENTER).clamp(0.0, 1.0);
                let e = x * x * (3.0 - 2.0 * x);
                k.pos = (ARENA.0 / 2.0, -150.0 + (KING_Y + 150.0) * e);
            } else {
                k.pos = (ARENA.0 / 2.0 + 360.0 * ((k.age - KING_ENTER) * 0.5).sin(), KING_Y);
                k.shot_in -= dt;
                if k.shot_in <= 0.0 {
                    king_fires = true;
                }
            }
        }
        if king_fires {
            self.king_fires();
            let rage = self.king.is_some_and(|k| k.hp * 2 < k.max_hp);
            let gap = 2.0 * self.rules.enemy_fire_gap * if rage { 0.65 } else { 1.0 };
            if let Some(k) = &mut self.king {
                k.pattern += 1;
                k.shot_in = gap;
            }
        }

        // the bolts
        for b in &mut self.bolts {
            b.pos = (b.pos.0 + b.vel.0 * dt, b.pos.1 + b.vel.1 * dt);
            b.age += dt;
        }
        let kills_before = self.kills;
        let mut bolts = std::mem::take(&mut self.bolts);
        bolts.retain(|b| {
            if b.age >= BOLT_LIFE || b.pos.0 < -20.0 || b.pos.0 > ARENA.0 + 20.0 || b.pos.1 < -20.0 || b.pos.1 > ARENA.1 + 20.0 {
                return false;
            }
            if let Some(i) = self.angels.iter().position(|a| dist(a.pos, b.pos) <= ANGEL_RADIUS + BOLT_RADIUS) {
                let a = &mut self.angels[i];
                a.hp = a.hp.saturating_sub(1);
                a.flash = 0.12;
                if a.hp == 0 {
                    let (pos, kind) = (a.pos, a.kind);
                    self.angels.remove(i);
                    self.kills += 1;
                    self.score += if kind == TRUMPETER { 150 } else { 100 };
                    self.bursts.push(Burst { pos, age: 0.0, big: false });
                }
                return false;
            }
            if let Some(k) = &mut self.king {
                if k.arrived() && dist(k.pos, b.pos) <= KING_RADIUS + BOLT_RADIUS {
                    k.hp = k.hp.saturating_sub(1);
                    k.flash = 0.1;
                    self.score += 10;
                    return false;
                }
            }
            true
        });
        self.bolts = bolts;
        if self.kills > kills_before && self.kills % 5 == 0 && self.king.is_none() {
            self.say_from(&STREAK);
        }
        if !self.said_half && self.king.is_some_and(|k| k.hp > 0 && k.hp * 2 < k.max_hp) {
            self.said_half = true;
            self.say_from(&KING_HALF);
        }
        if self.king.is_some_and(|k| k.hp == 0) {
            let pos = self.king.map_or((0.0, 0.0), |k| k.pos);
            self.phase = Phase::Won;
            self.since_end = 0.0;
            self.score += 1000;
            self.shots.clear();
            self.burst(pos, true);
            self.say("...Keep your hourglass.");
            return;
        }

        // what the angels shot at you
        for s in &mut self.shots {
            s.pos = (s.pos.0 + s.vel.0 * dt, s.pos.1 + s.vel.1 * dt);
        }
        self.shots.retain(|s| s.pos.0 > -40.0 && s.pos.0 < ARENA.0 + 40.0 && s.pos.1 > -80.0 && s.pos.1 < ARENA.1 + 40.0);
        let me = self.player.pos;
        if self.player.invuln <= 0.0 {
            if let Some(i) = self.shots.iter().position(|s| dist(s.pos, me) <= PLAYER_RADIUS + s.kind.radius()) {
                self.shots.remove(i);
                self.hurt_player();
            }
        }
        // ramming: the angel goes, you are hurt
        if self.phase == Phase::Playing && self.player.invuln <= 0.0 {
            if let Some(i) = self.angels.iter().position(|a| dist(a.pos, me) <= ANGEL_RADIUS + PLAYER_RADIUS) {
                let pos = self.angels[i].pos;
                self.angels.remove(i);
                self.burst(pos, false);
                self.hurt_player();
            } else if self.king.is_some_and(|k| dist(k.pos, me) <= KING_RADIUS + PLAYER_RADIUS) {
                self.hurt_player();
            }
        }
    }
}

/// When the king dies the Void opens where he fell (seconds after his death): he has this long to
/// come apart, then it opens, then everything is pulled in, then the screen is swallowed.
pub const FINALE_DEATH: f32 = 1.8;
/// Everything starts falling in at this moment.
pub const FINALE_PULL: f32 = 2.6;
/// How long one thing takes to fall in; the last to go (the staff) starts `PULL_STAGGER` later.
pub const PULL_TRAVEL: f32 = 2.6;
pub const PULL_STAGGER: f32 = 0.8;
/// The game is over, and the popup comes.
pub const FINALE_END: f32 = 7.6;

/// Where the king's death has got to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Finale {
    /// Where the Void is: where the king fell.
    pub centre: Pt,
    /// How wide it has opened, in arena units (0 before it opens).
    pub radius: f32,
    /// Seconds since things began to fall in (negative before).
    pub pull: f32,
    /// 0..1: how much of the screen it has swallowed.
    pub swallow: f32,
    /// 0..1: how much of the king has gone, into the opening.
    pub king_gone: f32,
}

impl Game {
    /// The king's death and what it opens (None unless he has just been beaten).
    pub fn finale(&self) -> Option<Finale> {
        if self.phase != Phase::Won {
            return None;
        }
        let s = self.since_end;
        let centre = self.king.map_or((ARENA.0 / 2.0, KING_Y), |k| k.pos);
        let g = ((s - FINALE_DEATH) / (FINALE_END - FINALE_DEATH - 1.2)).clamp(0.0, 1.0);
        let radius = if s < FINALE_DEATH { 0.0 } else { 40.0 + 480.0 * g * g };
        let swallow_from = FINALE_PULL + PULL_TRAVEL + PULL_STAGGER - 1.6;
        Some(Finale {
            centre,
            radius,
            pull: s - FINALE_PULL,
            swallow: ((s - swallow_from) / 1.8).clamp(0.0, 1.0),
            king_gone: ((s - (FINALE_DEATH - 0.6)) / 0.6).clamp(0.0, 1.0),
        })
    }

    /// The end of the game has played out.
    pub fn finished(&self) -> bool {
        self.phase == Phase::Won && self.since_end >= FINALE_END
    }

    /// 0..1: how much of the heavenly coliseum is in view: it rises behind the king as he comes down.
    pub fn coliseum_amount(&self) -> f32 {
        match &self.king {
            Some(k) => {
                let x = (k.age / KING_ENTER).clamp(0.0, 1.0);
                x * x * (3.0 - 2.0 * x)
            }
            None => 0.0,
        }
    }
}

/// Where something that was at `p` is once the Void has pulled at it for `pull` seconds (it began
/// `delay` seconds late), and how big it still is (1 at rest, 0 when it is gone). It spirals in.
pub fn pulled(p: Pt, centre: Pt, pull: f32, delay: f32) -> (Pt, f32) {
    let u = ((pull - delay) / PULL_TRAVEL).clamp(0.0, 1.0);
    let u2 = u * u;
    let (rx, ry) = (p.0 - centre.0, p.1 - centre.1);
    let (s, c) = (u2 * TAU * 1.5).sin_cos();
    let keep = 1.0 - u2;
    ((centre.0 + (rx * c - ry * s) * keep, centre.1 + (rx * s + ry * c) * keep), keep)
}

/// A piece of the heavenly coliseum behind the king.
#[derive(Debug, Clone, PartialEq)]
pub struct Wall {
    pub points: Vec<Pt>,
    pub closed: bool,
    /// An opening (arch) rather than a line: filled with light.
    pub filled: bool,
    /// 0..1 on the gold ramp.
    pub tone: f32,
}

/// Three storeys of arches along the back and the sand of the floor, in arena units.
pub fn coliseum() -> Vec<Wall> {
    let mut walls = Vec::new();
    let storey = 112.0;
    let top = 64.0;
    for s in 0..3 {
        let y0 = top + s as f32 * storey;
        let y1 = y0 + storey;
        walls.push(Wall { points: vec![(0.0, y1), (ARENA.0, y1)], closed: false, filled: false, tone: 0.8 });
        for i in 0..12 {
            let cx = 50.0 + 100.0 * i as f32;
            let spring = y0 + 44.0;
            let mut pts = vec![(cx - 30.0, y1 - 8.0)];
            for k in 0..=10 {
                let a = PI + PI * k as f32 / 10.0;
                pts.push((cx + 30.0 * a.cos(), spring + 30.0 * a.sin()));
            }
            pts.push((cx + 30.0, y1 - 8.0));
            walls.push(Wall { points: pts, closed: true, filled: true, tone: 0.6 });
            walls.push(Wall { points: vec![(cx + 50.0, y0 + 14.0), (cx + 50.0, y1)], closed: false, filled: false, tone: 0.5 });
        }
    }
    walls.push(Wall { points: vec![(0.0, top), (ARENA.0, top)], closed: false, filled: false, tone: 0.9 });
    // the sand: two rings
    for (rx, ry, tone) in [(760.0, 110.0, 0.5), (560.0, 70.0, 0.4)] {
        let pts = (0..48).map(|i| {
            let a = TAU * i as f32 / 48.0;
            (ARENA.0 / 2.0 + rx * a.cos(), ARENA.1 - 10.0 + ry * a.sin())
        });
        walls.push(Wall { points: pts.collect(), closed: true, filled: false, tone });
    }
    walls
}

pub fn dist(a: Pt, b: Pt) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// A point on the ship (in its own picture, head up, the fist at 0,0) turned to face `angle`.
pub fn turned(local: Pt, angle: f32) -> Pt {
    let r = angle + PI / 2.0;
    let (s, c) = r.sin_cos();
    (local.0 * c - local.1 * s, local.0 * s + local.1 * c)
}

/// The head of the staff of time (the hourglass), in the ship's own picture (head up).
pub const STAFF_HEAD: Pt = (0.0, -70.0);
/// The butt of the staff, where the thrust comes out.
pub const STAFF_BUTT: Pt = (0.0, 66.0);

/// Where the head of the staff is for a ship at `pos` facing `angle`.
pub fn staff_tip(pos: Pt, angle: f32) -> Pt {
    let t = turned(STAFF_HEAD, angle);
    (pos.0 + t.0, pos.1 + t.1)
}

/// How the ship's parts are shaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShipTone {
    Shaft,
    Glove,
    Knuckle,
    Cuff,
    Gem,
}

/// One convex piece of the ship.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub points: Vec<Pt>,
    pub tone: ShipTone,
}

/// The ship: the staff of time held upright in a golden gloved fist (four fingers across the shaft,
/// the thumb over the top, a cuff below), as convex pieces round the fist at 0,0, turned to face
/// `angle`. It reads like a cursor: the hand is what you steer, the staff is what points.
pub fn ship(angle: f32) -> Vec<Piece> {
    // the fist is drawn from the side, its back to the right: the fingers wrap round the shaft on
    // the left, the thumb folds over them, the cuff is below. `dx` centres it on the shaft.
    let dx = -5.5;
    let at = |pts: &[Pt]| -> Vec<Pt> { pts.iter().map(|p| (p.0 + dx, p.1)).collect() };
    let mut local: Vec<(Vec<Pt>, ShipTone)> = vec![
        (vec![(-2.5, -64.0), (2.5, -64.0), (2.5, 58.0), (-2.5, 58.0)], ShipTone::Shaft),
        (vec![(0.0, 66.0), (6.0, 58.0), (0.0, 50.0), (-6.0, 58.0)], ShipTone::Gem),
        // the cuff, and the back of the hand
        (at(&[(3.0, 19.0), (23.0, 17.0), (27.0, 31.0), (1.0, 32.0)]), ShipTone::Cuff),
        (at(&[(2.0, 17.0), (24.0, 15.0), (24.0, 20.0), (2.0, 22.0)]), ShipTone::Gem),
        (at(&[(-1.0, -19.0), (13.0, -21.0), (23.0, -12.0), (25.0, 4.0), (21.0, 17.0), (6.0, 20.0), (-1.0, 17.0)]), ShipTone::Glove),
    ];
    // four fingers across the shaft, the little one shorter and tucked in
    for i in 0..4 {
        let y = -14.5 + 9.0 * i as f32;
        let left = if i == 3 { -13.0 } else { -15.0 + 1.0 * (i % 2) as f32 };
        local.push((at(&[(left, y), (left + 2.5, y - 4.2), (9.0, y - 4.2), (11.5, y), (9.0, y + 4.2), (left + 2.5, y + 4.2)]), ShipTone::Knuckle));
    }
    // the thumb, folded over the top finger
    local.push((at(&[(-12.0, -19.0), (-3.0, -25.0), (8.0, -22.0), (9.0, -15.0), (-1.0, -12.5), (-10.0, -13.0)]), ShipTone::Gem));
    local.into_iter().map(|(pts, tone)| Piece { points: pts.into_iter().map(|p| turned(p, angle)).collect(), tone }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDLE: Input = Input { turn: 0.0, thrust: false, fire: false };

    fn run(g: &mut Game, secs: f32, input: Input) {
        for _ in 0..(secs * 60.0) as usize {
            g.step(1.0 / 60.0, input);
        }
    }

    /// A game with nothing in the sky, and the angels held off.
    fn quiet(seed: u32) -> Game {
        let mut g = Game::new(seed);
        g.next_angel = 1e9;
        g
    }

    fn angel_at(g: &mut Game, kind: u8, pos: Pt) {
        g.spawn_angel();
        let a = g.angels.last_mut().unwrap();
        a.kind = kind;
        a.hp = if kind == TRUMPETER { 2 } else { 1 };
        a.pos = pos;
        a.base_x = pos.0;
        a.sway = 0.0;
        a.speed = 0.0;
        a.shot_in = 1e9;
    }

    #[test]
    fn the_difficulty_sets_the_lives_the_angels_and_the_king() {
        let (e, n, h) = (Rules::of(Difficulty::Easy), Rules::of(Difficulty::Normal), Rules::of(Difficulty::Hard));
        assert!(e.lives > n.lives && n.lives > h.lives);
        assert!(e.kills_for_king < n.kills_for_king && n.kills_for_king < h.kills_for_king);
        assert!(e.king_hp < n.king_hp && n.king_hp < h.king_hp);
        assert!(e.enemy_speed < n.enemy_speed && n.enemy_speed < h.enemy_speed && h.enemy_speed == 1.0);
        assert!(e.enemy_fire_gap > n.enemy_fire_gap && n.enemy_fire_gap > h.enemy_fire_gap && h.enemy_fire_gap == 1.0);
        assert!(e.fire_cooldown < n.fire_cooldown && n.fire_cooldown < h.fire_cooldown);
        for d in Difficulty::ALL {
            let g = Game::with_difficulty(1, d);
            assert_eq!((g.difficulty, g.lives, g.rules()), (d, Rules::of(d).lives, Rules::of(d)));
        }
        assert_eq!(Game::new(1).difficulty, Difficulty::Hard);
    }

    #[test]
    fn the_staff_starts_low_in_the_middle_pointing_up() {
        let g = Game::new(1);
        assert!((g.player.pos.0 - ARENA.0 / 2.0).abs() < 1.0 && g.player.pos.1 > ARENA.1 * 0.7);
        assert!((g.player.angle + PI / 2.0).abs() < 1e-6);
        assert_eq!(g.phase, Phase::Playing);
        let tip = g.staff_tip();
        assert!(tip.1 < g.player.pos.1 - 40.0, "the staff points up the screen");
    }

    #[test]
    fn it_turns_thrusts_and_drifts() {
        let mut g = quiet(1);
        run(&mut g, 0.5, Input { turn: 1.0, ..IDLE });
        assert!((g.player.angle - (-PI / 2.0 + TURN_SPEED * 0.5)).abs() < 0.02, "turned right");
        let mut g = quiet(1);
        run(&mut g, 0.5, Input { turn: -1.0, ..IDLE });
        assert!(g.player.angle < -PI / 2.0 - 1.0, "turned left");
        // thrust goes the way the nose points
        let mut g = quiet(1);
        let y0 = g.player.pos.1;
        run(&mut g, 0.4, Input { thrust: true, ..IDLE });
        assert!(g.player.thrusting && g.player.vel.1 < -100.0 && g.player.vel.0.abs() < 1.0 && g.player.pos.1 < y0 - 20.0);
        // letting go it keeps drifting, slowing down
        let v = g.player.vel.1;
        run(&mut g, 0.5, IDLE);
        assert!(!g.player.thrusting && g.player.vel.1 < 0.0 && g.player.vel.1 > v, "still drifting, slower");
        run(&mut g, 8.0, IDLE);
        assert!(g.player.vel.1.abs() < 12.0, "and it all but stops");
    }

    #[test]
    fn the_speed_has_a_limit() {
        let mut g = quiet(1);
        g.player.pos = (ARENA.0 / 2.0, PLAYER_TOP + 200.0);
        run(&mut g, 0.2, Input { thrust: true, ..IDLE });
        run(&mut g, 3.0, Input { thrust: true, turn: 1.0, ..IDLE });
        let v = g.player.vel;
        assert!((v.0 * v.0 + v.1 * v.1).sqrt() <= MAX_SPEED + 0.5);
    }

    #[test]
    fn out_of_one_side_and_in_at_the_other_but_never_off_the_top_or_bottom() {
        let mut g = quiet(1);
        g.player.angle = 0.0; // facing right
        g.player.pos = (ARENA.0 - 10.0, 600.0);
        run(&mut g, 1.5, Input { thrust: true, ..IDLE });
        assert!(g.player.pos.0 < ARENA.0 / 2.0, "came back in on the left: {:?}", g.player.pos);
        let mut g = quiet(1);
        run(&mut g, 6.0, Input { thrust: true, ..IDLE }); // straight up
        assert!(g.player.pos.1 >= PLAYER_TOP - 0.01 && g.player.vel.1 >= 0.0 - 0.01);
        g.player.angle = PI / 2.0; // facing down
        run(&mut g, 6.0, Input { thrust: true, ..IDLE });
        assert!(g.player.pos.1 <= PLAYER_BOTTOM + 0.01);
    }

    #[test]
    fn the_staff_fires_bolts_out_of_its_head_at_the_rate_of_the_difficulty() {
        for d in Difficulty::ALL {
            let mut g = Game::with_difficulty(2, d);
            g.next_angel = 1e9;
            g.step(1.0 / 60.0, Input { fire: true, ..IDLE });
            assert_eq!(g.bolts.len(), 1);
            let b = g.bolts[0];
            let tip = g.staff_tip();
            assert!((b.pos.0 - tip.0).abs() < 25.0 && (b.pos.1 - tip.1).abs() < 25.0, "from the staff, {b:?} {tip:?}");
            assert!(b.vel.1 < -900.0 && b.vel.0.abs() < 1.0, "up the screen");
            run(&mut g, 1.0, Input { fire: true, ..IDLE });
            let shots = 1 + (1.0 / Rules::of(d).fire_cooldown) as usize;
            assert!(g.bolts.len() <= shots + 1, "{d:?}: no faster than the cooldown");
        }
        // an easier game fires more often
        let count = |d| {
            let mut g = Game::with_difficulty(2, d);
            g.next_angel = 1e9;
            let mut n = 0;
            for _ in 0..120 {
                let before = g.bolts.len();
                g.step(1.0 / 60.0, Input { fire: true, ..IDLE });
                n += (g.bolts.len() > before) as u32;
                g.bolts.clear();
            }
            n
        };
        assert!(count(Difficulty::Easy) > count(Difficulty::Hard));
        // bolts fly where the nose points, and are gone after a while
        let mut g = quiet(2);
        g.player.angle = 0.0;
        g.step(1.0 / 60.0, Input { fire: true, ..IDLE });
        assert!(g.bolts[0].vel.0 > 900.0 && g.bolts[0].vel.1.abs() < 1.0);
        run(&mut g, BOLT_LIFE + 0.2, IDLE);
        assert!(g.bolts.is_empty());
    }

    #[test]
    fn a_bolt_fells_an_angel_and_scores() {
        let mut g = quiet(3);
        let tip = g.staff_tip();
        angel_at(&mut g, ARCHER, (tip.0, tip.1 - 200.0));
        run(&mut g, 0.0, IDLE);
        g.step(1.0 / 60.0, Input { fire: true, ..IDLE });
        run(&mut g, 0.5, IDLE);
        assert!(g.angels.is_empty() && g.kills == 1 && g.score == 100);
        assert!(!g.bursts.is_empty());
        // the trumpeter takes two
        let mut g = quiet(3);
        let tip = g.staff_tip();
        angel_at(&mut g, TRUMPETER, (tip.0, tip.1 - 200.0));
        g.step(1.0 / 60.0, Input { fire: true, ..IDLE });
        run(&mut g, 0.4, IDLE);
        assert_eq!((g.angels.len(), g.kills), (1, 0));
        assert!(g.angels[0].hp == 1 && g.angels[0].flash >= 0.0);
        run(&mut g, 0.3, Input { fire: true, ..IDLE });
        run(&mut g, 0.6, IDLE);
        assert_eq!((g.angels.len(), g.kills, g.score), (0, 1, 150));
    }

    #[test]
    fn angels_come_down_from_the_top_and_leave_at_the_bottom_when_nobody_stops_them() {
        let mut g = Game::new(5);
        g.player.pos = (50.0, PLAYER_BOTTOM); // out of the way
        let mut max_alive = 0;
        let mut seen_top = false;
        for _ in 0..(40 * 60) {
            g.step(1.0 / 60.0, IDLE);
            g.player.invuln = 5.0; // nothing hurts: this is about the angels
            max_alive = max_alive.max(g.angels.len());
            seen_top |= g.angels.iter().any(|a| a.pos.1 < 0.0);
            assert!(g.angels.iter().all(|a| a.pos.0 >= 40.0 && a.pos.0 <= ARENA.0 - 40.0 && a.pos.1 < ARENA.1 + 60.0));
        }
        assert!(seen_top && max_alive >= 3 && max_alive <= MAX_ANGELS, "{max_alive}");
        assert_eq!(g.kills, 0, "escaping angels do not count, and cost nothing");
        assert_eq!(g.phase, Phase::Playing);
    }

    #[test]
    fn each_kind_of_angel_shoots_its_own_way() {
        let mut g = quiet(7);
        g.player.pos = (600.0, 700.0);
        g.fire_cool = 1e9;
        for (kind, n, shot) in [(ARCHER, 1, ShotKind::Arrow), (HARPIST, 3, ShotKind::Note), (TRUMPETER, 1, ShotKind::Blast)] {
            g.shots.clear();
            g.angel_fires(kind, (300.0, 200.0));
            assert_eq!(g.shots.len(), n, "{kind}");
            assert!(g.shots.iter().all(|s| s.kind == shot));
            let s = g.shots[0];
            match kind {
                ARCHER => assert!(s.vel.0 > 0.0 && s.vel.1 > 0.0, "aimed at the staff: {s:?}"),
                TRUMPETER => assert!(s.vel.0 == 0.0 && s.vel.1 > 300.0, "straight down"),
                _ => assert!(g.shots.iter().any(|s| s.vel.0 > 0.0) && g.shots.iter().all(|s| s.vel.1 > 0.0)),
            }
        }
        // an angel with its shot due fires once in range
        let mut g = quiet(7);
        g.player.invuln = 99.0;
        angel_at(&mut g, TRUMPETER, (900.0, 300.0));
        g.angels[0].shot_in = 0.01;
        g.step(1.0 / 60.0, IDLE);
        assert_eq!(g.shots.len(), 1);
        // but not while it is still above the arena
        let mut g = quiet(7);
        angel_at(&mut g, TRUMPETER, (900.0, -30.0));
        g.angels[0].shot_in = 0.01;
        g.step(1.0 / 60.0, IDLE);
        assert!(g.shots.is_empty());
    }

    #[test]
    fn a_hit_costs_a_life_then_you_are_untouchable_for_a_while() {
        let mut g = quiet(9);
        let me = g.player.pos;
        g.shots.push(Shot { pos: me, vel: (0.0, 0.0), kind: ShotKind::Arrow });
        g.step(1.0 / 60.0, IDLE);
        assert_eq!(g.lives, Rules::of(Difficulty::Hard).lives - 1);
        assert!(g.player.invuln > 2.0 && g.shots.is_empty());
        // a second one at once does nothing
        g.shots.push(Shot { pos: g.player.pos, vel: (0.0, 0.0), kind: ShotKind::Arrow });
        g.step(1.0 / 60.0, IDLE);
        assert_eq!(g.lives, Rules::of(Difficulty::Hard).lives - 1);
        // after the time it counts again
        run(&mut g, INVULN + 0.1, IDLE);
        g.shots.push(Shot { pos: g.player.pos, vel: (0.0, 0.0), kind: ShotKind::Orb });
        g.step(1.0 / 60.0, IDLE);
        assert_eq!(g.lives, Rules::of(Difficulty::Hard).lives - 2);
    }

    #[test]
    fn ramming_an_angel_hurts_and_the_angel_goes() {
        let mut g = quiet(9);
        let me = g.player.pos;
        angel_at(&mut g, HARPIST, (me.0 + 10.0, me.1));
        g.step(1.0 / 60.0, IDLE);
        assert!(g.angels.is_empty() && g.lives == Rules::of(Difficulty::Hard).lives - 1 && g.kills == 0);
    }

    #[test]
    fn losing_all_the_lives_ends_it_and_a_death_is_reported_once() {
        let mut g = quiet(9);
        assert!(!g.take_death());
        for _ in 0..Rules::of(Difficulty::Hard).lives {
            g.player.invuln = 0.0;
            g.shots.push(Shot { pos: g.player.pos, vel: (0.0, 0.0), kind: ShotKind::Orb });
            g.step(1.0 / 60.0, IDLE);
        }
        assert_eq!((g.phase, g.lives), (Phase::Lost, 0));
        assert!(g.line().is_some_and(|l| LOSS.contains(&l.0)), "{:?}", g.line());
        assert!(g.take_death() && !g.take_death());
        // time stops, and the end screen's clock runs
        let t = g.t;
        run(&mut g, 1.0, Input { thrust: true, fire: true, ..IDLE });
        assert_eq!(g.t, t);
        assert!(g.since_end > 0.9 && g.bolts.is_empty());
    }

    #[test]
    fn enough_angels_down_brings_the_king_who_comes_in_from_above() {
        let mut g = quiet(11);
        g.kills = Rules::of(Difficulty::Hard).kills_for_king;
        g.step(1.0 / 60.0, IDLE);
        let k = g.king.unwrap();
        assert!(g.king_out() && k.pos.1 < 0.0 && !k.arrived() && k.hp == Rules::of(Difficulty::Hard).king_hp);
        assert_eq!(g.line().map(|l| l.0), Some("KNEEL, OLD MAN."));
        run(&mut g, KING_ENTER + 0.2, IDLE);
        let k = g.king.unwrap();
        assert!(k.arrived() && (k.pos.1 - KING_Y).abs() < 1.0);
        // he sways from side to side, inside the arena
        let mut xs = Vec::new();
        for _ in 0..(8 * 60) {
            g.player.invuln = 9.0;
            g.step(1.0 / 60.0, IDLE);
            xs.push(g.king.unwrap().pos.0);
        }
        let (lo, hi) = xs.iter().fold((f32::MAX, f32::MIN), |a, &x| (a.0.min(x), a.1.max(x)));
        assert!(hi - lo > 300.0 && lo > 100.0 && hi < ARENA.0 - 100.0, "{lo} {hi}");
        // and nothing hurts him on the way in
        let mut g = quiet(11);
        g.kills = 99;
        g.step(1.0 / 60.0, IDLE);
        g.king.as_mut().unwrap().pos = (600.0, 600.0);
        let hp = g.king.unwrap().hp;
        g.bolts.push(Bolt { pos: (600.0, 600.0), vel: (0.0, 0.0), age: 0.0 });
        g.step(1.0 / 60.0, IDLE);
        assert_eq!(g.king.unwrap().hp, hp, "he is not hurt before he has arrived");
    }

    #[test]
    fn the_king_shoots_three_ways_in_turn_and_faster_when_he_is_hurt() {
        let mut g = quiet(13);
        g.kills = 99;
        g.player.invuln = 1e6;
        g.step(1.0 / 60.0, IDLE);
        run(&mut g, KING_ENTER + 0.1, IDLE);
        let mut kinds = std::collections::BTreeSet::new();
        let mut volleys = 0;
        let mut last = 0;
        for _ in 0..(14 * 60) {
            g.step(1.0 / 60.0, IDLE);
            if g.shots.len() > last + 2 {
                volleys += 1;
            }
            last = g.shots.len();
            for s in &g.shots {
                kinds.insert(format!("{:?}", s.kind));
            }
        }
        assert!(volleys >= 3, "{volleys}");
        assert!(kinds.contains("Orb") && kinds.contains("Arrow") && kinds.contains("Blast"), "{kinds:?}");
        // hurt, his pauses are shorter
        let gap = |hurt: bool| {
            let mut g = quiet(13);
            g.kills = 99;
            g.step(1.0 / 60.0, IDLE);
            run(&mut g, KING_ENTER + 0.1, IDLE);
            let k = g.king.as_mut().unwrap();
            if hurt {
                k.hp = k.max_hp / 4;
            }
            k.shot_in = 0.0;
            g.player.invuln = 1e6;
            g.step(1.0 / 60.0, IDLE);
            g.king.unwrap().shot_in
        };
        assert!(gap(true) < gap(false));
    }

    #[test]
    fn felling_the_king_wins_and_stops_the_shooting() {
        let mut g = quiet(15);
        g.kills = 99;
        g.player.invuln = 1e6;
        g.step(1.0 / 60.0, IDLE);
        run(&mut g, KING_ENTER + 0.1, IDLE);
        g.king.as_mut().unwrap().hp = 1;
        let k = g.king.unwrap().pos;
        g.bolts.push(Bolt { pos: k, vel: (0.0, 0.0), age: 0.0 });
        g.shots.push(Shot { pos: (10.0, 10.0), vel: (0.0, 0.0), kind: ShotKind::Orb });
        let before = g.score;
        g.step(1.0 / 60.0, IDLE);
        assert_eq!(g.phase, Phase::Won);
        assert!(g.shots.is_empty() && g.score >= before + 1000 && g.bursts.iter().any(|b| b.big));
        assert_eq!(g.line().map(|l| l.0), Some("...Keep your hourglass."));
        assert!(!g.take_death(), "a win is no death");
        let t = g.t;
        run(&mut g, 1.0, IDLE);
        assert_eq!(g.t, t);
        assert!(g.since_end > 0.9);
    }

    fn won_game() -> Game {
        let mut g = quiet(15);
        g.kills = 99;
        g.player.invuln = 1e6;
        g.step(1.0 / 60.0, IDLE);
        run(&mut g, KING_ENTER + 0.1, IDLE);
        g.king.as_mut().unwrap().hp = 1;
        let k = g.king.unwrap().pos;
        g.bolts.push(Bolt { pos: k, vel: (0.0, 0.0), age: 0.0 });
        g.step(1.0 / 60.0, IDLE);
        assert_eq!(g.phase, Phase::Won);
        g
    }

    #[test]
    fn the_kings_death_opens_the_void_where_he_fell_and_swallows_the_screen() {
        let mut g = quiet(3);
        assert!(g.finale().is_none() && !g.finished(), "nothing before he is beaten");
        let mut g = won_game();
        let fell = g.king.unwrap().pos;
        let f = g.finale().unwrap();
        assert_eq!((f.centre, f.radius, f.swallow, f.king_gone), (fell, 0.0, 0.0, 0.0));
        // it opens only after the death throes
        run(&mut g, FINALE_DEATH - 0.2, IDLE);
        assert_eq!(g.finale().unwrap().radius, 0.0);
        run(&mut g, 0.4, IDLE);
        assert!(g.finale().unwrap().radius >= 40.0);
        assert_eq!(g.finale().unwrap().king_gone, 1.0, "the king is gone into it");
        // it grows, the pull begins, then it swallows the screen, and only then is it over
        let mut last = g.finale().unwrap().radius;
        let mut swallow = 0.0;
        while !g.finished() {
            run(&mut g, 0.25, IDLE);
            let f = g.finale().unwrap();
            assert!(f.radius >= last && f.swallow >= swallow);
            (last, swallow) = (f.radius, f.swallow);
            assert!(g.since_end < FINALE_END + 0.5);
        }
        assert_eq!(g.finale().unwrap().swallow, 1.0, "the screen is all Void before the popup");
        assert!(g.finale().unwrap().pull > PULL_TRAVEL + PULL_STAGGER, "everything has fallen in");
    }

    #[test]
    fn a_pulled_thing_spirals_in_and_shrinks_until_it_is_gone() {
        let centre = (600.0, 175.0);
        let p = (900.0, 600.0);
        assert_eq!(pulled(p, centre, -1.0, 0.0), (p, 1.0));
        assert_eq!(pulled(p, centre, 0.0, 0.0), (p, 1.0));
        let mut last = dist(p, centre);
        for n in 1..=10 {
            let (q, size) = pulled(p, centre, n as f32 * PULL_TRAVEL / 10.0, 0.0);
            assert!(dist(q, centre) < last + 1e-3 && size < 1.0);
            last = dist(q, centre);
        }
        let (q, size) = pulled(p, centre, PULL_TRAVEL, 0.0);
        assert!(dist(q, centre) < 1e-3 && size == 0.0);
        // late things are still where they were while the early ones fall
        assert_eq!(pulled(p, centre, 0.5, PULL_STAGGER), (p, 1.0));
        // it turns as it falls: halfway in it is not on the straight line to the middle
        let (q, _) = pulled(p, centre, PULL_TRAVEL * 0.6, 0.0);
        let straight = ((q.0 - centre.0) * (p.1 - centre.1) - (q.1 - centre.1) * (p.0 - centre.0)).abs();
        assert!(straight > 1.0);
    }

    #[test]
    fn the_coliseum_rises_as_the_king_comes_down_and_is_drawn_inside_the_arena() {
        let mut g = quiet(2);
        assert_eq!(g.coliseum_amount(), 0.0);
        g.kills = 99;
        g.step(1.0 / 60.0, IDLE);
        let early = g.coliseum_amount();
        run(&mut g, KING_ENTER, IDLE);
        assert!(early < 0.1 && g.coliseum_amount() == 1.0);
        let walls = coliseum();
        assert!(walls.len() > 70 && walls.iter().any(|w| w.filled));
        for w in &walls {
            assert!(w.points.len() >= 2 && (0.0..=1.0).contains(&w.tone));
            assert!(w.points.iter().all(|p| p.0.is_finite() && p.1.is_finite() && p.0 > -200.0 && p.0 < ARENA.0 + 200.0));
        }
    }

    #[test]
    fn every_line_fits_the_screen_and_speaks_of_a_mortal_or_the_old_man() {
        let pools: [&[&str]; 6] = [&HIT, &LAST_LIFE, &STREAK, &CHATTER, &KING_HALF, &LOSS];
        for pool in pools {
            for line in pool {
                assert!(line.len() <= 48 && !line.contains("dragon"), "{line}");
            }
        }
        assert!(START.len() <= 48);
        // the people at the keyboard are mortals; the staff is the old man's (Father Time)
        let all: Vec<&str> = HIT.iter().chain(CHATTER.iter()).chain(LOSS.iter()).copied().collect();
        assert!(all.iter().any(|l| l.contains("mortal") || l.contains("keyboard") || l.contains("keys")));
        assert!(all.iter().any(|l| l.contains("old man") || l.contains("Father Time")));
    }

    #[test]
    fn a_hit_gets_an_insult_never_the_same_twice_and_the_last_life_gets_its_own() {
        let mut g = quiet(5);
        g.lives = 6;
        let mut last = "";
        for _ in 0..40 {
            g.player.invuln = 0.0;
            g.lives = 6;
            g.shots.push(Shot { pos: g.player.pos, vel: (0.0, 0.0), kind: ShotKind::Orb });
            g.step(1.0 / 60.0, IDLE);
            let text = g.line().unwrap().0;
            assert!(HIT.contains(&text) && text != last, "{text}");
            last = text;
        }
        let mut g = quiet(5);
        g.lives = 2;
        g.shots.push(Shot { pos: g.player.pos, vel: (0.0, 0.0), kind: ShotKind::Orb });
        g.step(1.0 / 60.0, IDLE);
        assert!(g.line().is_some_and(|l| LAST_LIFE.contains(&l.0)));
    }

    #[test]
    fn the_angels_complain_every_fifth_one_and_chatter_when_it_goes_quiet() {
        let mut g = quiet(8);
        g.kills = 4;
        angel_at(&mut g, ARCHER, (300.0, 300.0));
        g.angels[0].hp = 1;
        g.bolts.push(Bolt { pos: (300.0, 300.0), vel: (0.0, 0.0), age: 0.0 });
        g.step(1.0 / 60.0, IDLE);
        assert_eq!(g.kills, 5);
        assert!(g.line().is_some_and(|l| STREAK.contains(&l.0)));
        // silence, then somebody speaks
        let mut g = quiet(8);
        run(&mut g, 0.8 + LINE_LIFE + 1.0, IDLE);
        assert!(g.line().is_none());
        run(&mut g, IDLE_GAP, IDLE);
        assert!(g.line().is_some_and(|l| CHATTER.contains(&l.0)), "{:?}", g.line());
    }

    #[test]
    fn the_king_shouts_once_when_he_is_down_to_half() {
        let mut g = quiet(12);
        g.kills = 99;
        g.player.invuln = 1e6;
        g.step(1.0 / 60.0, IDLE);
        run(&mut g, KING_ENTER + 0.2, IDLE);
        let k = g.king.as_mut().unwrap();
        k.hp = k.max_hp / 2;
        let at = k.pos;
        g.bolts.push(Bolt { pos: at, vel: (0.0, 0.0), age: 0.0 });
        g.step(1.0 / 60.0, IDLE);
        assert!(g.line().is_some_and(|l| KING_HALF.contains(&l.0)));
        let said = g.line().unwrap().0;
        g.say("x");
        g.bolts.push(Bolt { pos: at, vel: (0.0, 0.0), age: 0.0 });
        g.step(1.0 / 60.0, IDLE);
        assert_eq!(g.line().unwrap().0, "x", "he does not say it twice ({said})");
    }

    #[test]
    fn a_win_is_reported_once_and_cheats_are_seen() {
        let mut g = quiet(4);
        assert!(!g.take_win() && !g.cheated());
        g.phase = Phase::Won;
        assert!(g.take_win());
        assert!(!g.take_win() && !g.take_death());
        let mut h = quiet(5);
        h.infinite = true;
        assert!(h.cheated());
    }

    #[test]
    fn the_infinite_health_cheat_costs_no_lives_and_cannot_lose_the_game() {
        let mut g = quiet(4);
        g.infinite = true;
        for _ in 0..30 {
            g.player.invuln = 0.0;
            g.shots.push(Shot { pos: g.player.pos, vel: (0.0, 0.0), kind: ShotKind::Orb });
            g.step(1.0 / 60.0, IDLE);
        }
        assert_eq!(g.lives, Rules::of(Difficulty::Hard).lives);
        assert_eq!(g.phase, Phase::Playing);
        assert!(!g.take_death());
        assert!(g.player.invuln > 0.0 && !g.bursts.is_empty(), "it still flashes and pops");
        // and it can be won as usual (so the ending can be seen)
        let mut g = won_game();
        g.infinite = true;
        assert!(g.finale().is_some());
    }

    #[test]
    fn progress_follows_the_angels_then_the_king() {
        let mut g = quiet(1);
        assert_eq!(g.progress(), 0.0);
        g.kills = Rules::of(Difficulty::Hard).kills_for_king / 2;
        assert!((g.progress() - 0.5).abs() < 0.05);
        g.kills = 99;
        g.step(1.0 / 60.0, IDLE);
        assert_eq!(g.progress(), 0.0, "the king is untouched");
        g.king.as_mut().unwrap().hp = g.king.unwrap().max_hp / 2;
        assert!((g.progress() - 0.5).abs() < 0.01);
    }

    #[test]
    fn the_whole_game_is_winnable_on_every_difficulty_by_a_steady_player() {
        // a bot that sits still, aims at the nearest thing above it and holds fire, being immortal:
        // it shows that the angels and the king can all be brought down, and in a sensible time
        for d in Difficulty::ALL {
            let mut g = Game::with_difficulty(21, d);
            let mut ticks = 0;
            while g.phase == Phase::Playing && ticks < 60 * 400 {
                // turn towards the nearest target
                let me = g.player.pos;
                let target = g
                    .king
                    .map(|k| k.pos)
                    .or_else(|| g.angels.iter().map(|a| a.pos).min_by(|a, b| dist(*a, me).total_cmp(&dist(*b, me))));
                let mut input = Input { fire: true, ..IDLE };
                if let Some(t) = target {
                    let want = (t.1 - me.1).atan2(t.0 - me.0);
                    let mut diff = want - g.player.angle;
                    while diff > PI {
                        diff -= TAU;
                    }
                    while diff < -PI {
                        diff += TAU;
                    }
                    input.turn = diff.signum() * (diff.abs() > 0.03) as i32 as f32;
                }
                g.player.invuln = 5.0;
                g.step(1.0 / 60.0, input);
                ticks += 1;
            }
            assert_eq!(g.phase, Phase::Won, "{d:?} after {} s", ticks / 60);
            assert!(ticks < 60 * 300, "{d:?} took {} s", ticks / 60);
        }
    }

    #[test]
    fn the_same_seed_plays_out_the_same_way() {
        let play = |seed| {
            let mut g = Game::new(seed);
            for k in 0..900 {
                let input = Input { turn: ((k / 70) % 3) as f32 - 1.0, thrust: k % 90 < 30, fire: k % 7 < 3 };
                g.step(1.0 / 60.0, input);
            }
            (g.player.pos, g.lives, g.kills, g.score, g.angels.len(), g.shots.len())
        };
        assert_eq!(play(4), play(4));
        assert_ne!(play(4), play(5));
    }

    #[test]
    fn the_ship_is_a_fist_on_a_staff_drawn_round_the_fist_and_turns_with_the_staff() {
        for angle in [-PI / 2.0, 0.0, 1.0, PI] {
            let pieces = ship(angle);
            assert_eq!(pieces.len(), 10);
            for p in &pieces {
                assert!(p.points.len() >= 3);
                for q in &p.points {
                    assert!(q.0.is_finite() && q.1.is_finite() && q.0.hypot(q.1) < 90.0, "{q:?}");
                }
            }
        }
        // head up: the shaft runs above and below the fist, the cuff is under the fingers
        let up = ship(-PI / 2.0);
        let shaft = up.iter().find(|p| p.tone == ShipTone::Shaft).unwrap();
        let cuff = up.iter().find(|p| p.tone == ShipTone::Cuff).unwrap();
        assert!(shaft.points.iter().any(|p| p.1 < -60.0) && shaft.points.iter().any(|p| p.1 > 50.0));
        assert!(cuff.points.iter().all(|p| p.1 > 10.0));
        // turned right: the staff points right and the cuff is behind
        let right = ship(0.0);
        let shaft = right.iter().find(|p| p.tone == ShipTone::Shaft).unwrap();
        let cuff = right.iter().find(|p| p.tone == ShipTone::Cuff).unwrap();
        assert!(shaft.points.iter().any(|p| p.0 > 60.0));
        assert!(cuff.points.iter().all(|p| p.0 < -10.0));
        // the head of the staff is at the top of the shaft, and the bolts come out of it
        let tip = staff_tip((100.0, 100.0), -PI / 2.0);
        assert!((tip.0 - 100.0).abs() < 0.01 && (tip.1 - 30.0).abs() < 0.01);
        let tip = staff_tip((100.0, 100.0), 0.0);
        assert!((tip.0 - 170.0).abs() < 0.01 && (tip.1 - 100.0).abs() < 0.01);
    }

    /// Prints a mid-game and a king scene for tools/game2_check.py. Run with
    /// `cargo test dump_game2 -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn dump_game2() {
        let bot = |g: &mut Game, secs: f32| {
            for _ in 0..(secs * 60.0) as usize {
                let me = g.player.pos;
                let target = g.king.map(|k| k.pos).or_else(|| g.angels.iter().map(|a| a.pos).min_by(|a, b| dist(*a, me).total_cmp(&dist(*b, me))));
                let mut input = Input { fire: true, thrust: g.t % 4.0 < 0.3, ..IDLE };
                if let Some(t) = target {
                    let want = (t.1 - me.1).atan2(t.0 - me.0);
                    let mut diff = want - g.player.angle;
                    while diff > PI {
                        diff -= TAU;
                    }
                    while diff < -PI {
                        diff += TAU;
                    }
                    input.turn = diff.signum() * (diff.abs() > 0.03) as i32 as f32;
                }
                g.player.invuln = 5.0;
                g.step(1.0 / 60.0, input);
            }
        };
        let mut a = Game::new(31);
        bot(&mut a, 20.0);
        let mut b = Game::new(31);
        b.kills = 99;
        bot(&mut b, 9.0);
        for (name, g) in [("angels", &a), ("king", &b)] {
            println!("== {name}");
            let p = g.player;
            println!("PLAYER {:.1} {:.1} {:.3} {}", p.pos.0, p.pos.1, p.angle, p.thrusting as u8);
            for an in &g.angels {
                println!("ANGEL {} {:.1} {:.1}", an.kind, an.pos.0, an.pos.1);
            }
            for s in &g.shots {
                println!("SHOT {:?} {:.1} {:.1} {:.1} {:.1}", s.kind, s.pos.0, s.pos.1, s.vel.0, s.vel.1);
            }
            for bo in &g.bolts {
                println!("BOLT {:.1} {:.1} {:.1} {:.1}", bo.pos.0, bo.pos.1, bo.vel.0, bo.vel.1);
            }
            if let Some(k) = g.king {
                println!("KING {:.1} {:.1} {} {}", k.pos.0, k.pos.1, k.hp, k.max_hp);
                for poly in crate::king::scene(&crate::king::pose(g.t, None, true)) {
                    let pts: Vec<String> = poly.points.iter().map(|q| format!("{:.1},{:.1}", k.pos.0 + (q.0 - 700.0) * 0.37, k.pos.1 + (q.1 - 515.0) * 0.37)).collect();
                    println!("KPOLY {} {} {} {} {}", poly.fill.value(), poly.line.0, poly.line.1 * 0.37, poly.closed as u8, pts.join(" "));
                }
            }
            for piece in ship(p.angle) {
                let pts: Vec<String> = piece.points.iter().map(|q| format!("{:.1},{:.1}", p.pos.0 + q.0, p.pos.1 + q.1)).collect();
                println!("DRAGON {:?} {}", piece.tone, pts.join(" "));
            }
            let tip = g.staff_tip();
            println!("TIP {:.1} {:.1}", tip.0, tip.1);
            if g.coliseum_amount() > 0.0 {
                for w in coliseum() {
                    let pts: Vec<String> = w.points.iter().map(|q| format!("{:.1},{:.1}", q.0, q.1)).collect();
                    println!("WALL {} {} {} {}", w.tone, w.filled as u8, w.closed as u8, pts.join(" "));
                }
            }
        }
    }

    /// The king's death at a few moments, as the painter lays it out (for tools/game2_check.py).
    #[test]
    #[ignore]
    fn dump_finale() {
        let mut g = Game::new(31);
        g.kills = 99;
        g.player.invuln = 1e6;
        g.step(1.0 / 60.0, IDLE);
        run(&mut g, KING_ENTER + 0.5, IDLE);
        g.player.pos = (420.0, 640.0);
        for (i, (x, y)) in [(200.0, 380.0), (900.0, 330.0), (1010.0, 520.0), (330.0, 520.0), (700.0, 470.0)].into_iter().enumerate() {
            g.spawn_angel();
            let a = g.angels.last_mut().unwrap();
            a.pos = (x, y);
            a.kind = (i % 3) as u8;
        }
        g.king.as_mut().unwrap().hp = 1;
        let k = g.king.unwrap().pos;
        g.bolts.push(Bolt { pos: k, vel: (0.0, 0.0), age: 0.0 });
        g.step(1.0 / 60.0, IDLE);
        let mut at = 0.0;
        for s in [1.2f32, 2.3, 3.6, 4.8, 5.8, 7.0] {
            run(&mut g, s - at, IDLE);
            at = s;
            let f = g.finale().unwrap();
            println!("== finale {s}");
            println!("SWALLOW {:.3}", f.swallow);
            println!("VOID {:.1} {:.1} {:.1}", f.centre.0, f.centre.1, f.radius);
            println!("KINGGONE {:.3} {:.1} {:.1}", f.king_gone, f.centre.0, f.centre.1);
            for w in coliseum() {
                let pts: Vec<String> = w.points.iter().map(|q| format!("{:.1},{:.1}", q.0, q.1)).collect();
                println!("WALL {} {} {} {}", w.tone, w.filled as u8, w.closed as u8, pts.join(" "));
            }
            for an in &g.angels {
                let d = (dist(an.pos, f.centre) / 1000.0).min(1.0) * PULL_STAGGER * 0.75;
                let (q, sz) = pulled(an.pos, f.centre, f.pull, d);
                println!("ANGEL {} {:.1} {:.1} {:.3}", an.kind, q.0, q.1, sz);
            }
            let (q, sz) = pulled(g.player.pos, f.centre, f.pull, PULL_STAGGER);
            let ang = g.player.angle + ((f.pull - PULL_STAGGER) / PULL_TRAVEL).clamp(0.0, 1.0).powi(2) * TAU * 2.0;
            for piece in ship(ang) {
                let pts: Vec<String> = piece.points.iter().map(|p| format!("{:.1},{:.1}", q.0 + p.0 * sz, q.1 + p.1 * sz)).collect();
                println!("DRAGON {:?} {}", piece.tone, pts.join(" "));
            }
            if f.king_gone < 1.0 {
                let sc = 0.37 * (1.0 - 0.9 * f.king_gone * f.king_gone);
                for poly in crate::king::scene(&crate::king::pose(g.t, None, true)) {
                    let pts: Vec<String> = poly.points.iter().map(|q| format!("{:.1},{:.1}", f.centre.0 + (q.0 - 700.0) * sc, f.centre.1 + (q.1 - 515.0) * sc)).collect();
                    println!("KPOLY {} {} {} {} {}", poly.fill.value(), poly.line.0, poly.line.1 * sc, poly.closed as u8, pts.join(" "));
                }
            }
        }
    }

    #[test]
    fn the_words_come_in_and_fade_while_playing_and_stay_at_the_end() {
        let mut g = quiet(1);
        run(&mut g, 1.0, IDLE);
        assert_eq!(g.line().map(|l| l.0), Some(START));
        run(&mut g, LINE_LIFE, IDLE);
        assert!(g.line().is_none());
        for t in [0.0, 0.1, 1.0, 2.5] {
            g.line = Some(("x", t));
            assert!(g.line().map_or(true, |l| (0.0..=1.0).contains(&l.1)));
        }
    }

    #[test]
    fn rapid_fire_shoots_more_than_three_times_as_fast_and_invincible_costs_nothing() {
        let fire = Input { fire: true, ..IDLE };
        let count = |cheats: Cheats| {
            let mut g = Game::with_cheats(2, Difficulty::Hard, cheats);
            g.next_angel = 1e9;
            let mut shots = 0usize;
            let mut was = 0usize;
            for _ in 0..120 {
                g.step(1.0 / 60.0, fire);
                if g.bolts.len() > was { shots += g.bolts.len() - was }
                was = g.bolts.len();
            }
            shots
        };
        let plain = count(Cheats::NONE);
        let rapid = count(Cheats::NONE.with(Cheat::RapidFire, true));
        assert!(rapid as f32 >= plain as f32 * 3.0, "{rapid} shots against {plain}");
        assert!((Game::with_cheats(1, Difficulty::Hard, Cheats::NONE.with(Cheat::RapidFire, true)).fire_gap() - Rules::of(Difficulty::Hard).fire_cooldown * RAPID_FIRE_FACTOR).abs() < 1e-6);
        // invincible is the same as the old infinite health, and it can be won
        let mut g = Game::with_cheats(4, Difficulty::Hard, Cheats::NONE.with(Cheat::Invincible, true));
        g.next_angel = 1e9;
        assert!(g.infinite);
        for _ in 0..30 {
            g.player.invuln = 0.0;
            g.shots.push(Shot { pos: g.player.pos, vel: (0.0, 0.0), kind: ShotKind::Orb });
            g.step(1.0 / 60.0, IDLE);
        }
        assert_eq!(g.lives, Rules::of(Difficulty::Hard).lives);
        assert!(!g.take_death());
        // and without the cheats nothing is on
        let plain = Game::with_difficulty(4, Difficulty::Hard);
        assert!(!plain.infinite && (plain.fire_gap() - Rules::of(Difficulty::Hard).fire_cooldown).abs() < 1e-6);
    }
}
