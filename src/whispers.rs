//! Soul Whispers, the Underworld's own backdrop: souls rise out of the lava between the slabs and
//! drift up through the window. They notice the cursor: slow it down and they come curious, sweep it
//! fast and they scatter. Hold it still near one and the soul leans in and whispers a line, then
//! lets go and fades away.
//!
//! Nothing here draws anything or reads the window: the caller hands in the time passed, the size of
//! the view, where the lava shows (`spawn_y`) and where the cursor is, all in points from the view's
//! top left, and gets back what to paint (`looks`). The same calls always give the same souls, so it
//! is all tested. The painting is in `ui/underworld.rs`.

pub type Pt = (f32, f32);

/// Most souls in the air at once.
pub const MAX_SOULS: usize = 34;
/// How far the cursor draws a soul in, in points.
pub const CURIOUS_RADIUS: f32 = 150.0;
/// How far a soul will come from to listen, in points.
pub const LISTEN_RADIUS: f32 = 110.0;
/// How far a fast cursor throws souls away.
pub const SCATTER_RADIUS: f32 = 130.0;
/// A cursor slower than this (points a second) is "still" and one faster than `FAST` scatters.
pub const STILL_SPEED: f32 = 28.0;
pub const FAST_SPEED: f32 = 900.0;
/// How long the cursor has to stay still before a soul comes to whisper, in seconds.
pub const STILL_FOR: f32 = 0.7;
/// Seconds a soul takes to lean in, and how long its words stay.
pub const LEAN_TIME: f32 = 0.6;
pub const WHISPER_TIME: f32 = 5.0;
/// Seconds before another soul will whisper once one has.
pub const REST: f32 = 3.0;
/// Once this many lines have been heard the souls change: they go red before they fade (the first
/// sign that something is wrong, see the story plan). Kept in step with what is heard, so it stays.
pub const RED_AFTER: u32 = 6;
/// Positions kept behind a soul for its tail.
pub const TRAIL: usize = 9;

/// Whatever the luck, by this many whispers every line has been heard: past the point where the
/// whispers left are no more than the lines not yet heard, only those are said.
pub const GUARANTEE: u32 = 30;

/// What the souls say. Any of them can come up at any time, at random (never the same one twice in
/// a row), within the `GUARANTEE`. `clue` marks the ones that point at where the story is going.
pub struct Line {
    pub text: &'static str,
    pub clue: bool,
}

pub const LINES: [Line; 12] = [
    Line { text: "It is warm here. We stopped minding.", clue: false },
    Line { text: "Do you hear the skeletons counting?", clue: false },
    Line { text: "Someone keeps the lava turning.", clue: false },
    Line { text: "We rise. We always rise. Then we stop.", clue: false },
    Line { text: "The old king used to hum down here.", clue: false },
    Line { text: "Father Time lost count of us.", clue: false },
    Line { text: "Do not step on the cracks. They remember.", clue: false },
    Line { text: "Something big breathes under the lava.", clue: true },
    Line { text: "Something is asking about the light above us.", clue: true },
    Line { text: "We used to reach the light. Lately we only reach the dark.", clue: true },
    Line { text: "Whoever you are, you are louder than the last one.", clue: false },
    Line { text: "Hush. Up there, someone is listening back.", clue: true },
];

/// A small deterministic random stream.
#[derive(Debug, Clone)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// 0..1.
    fn unit(&mut self) -> f32 {
        ((self.next() >> 40) & 0xFF_FFFF) as f32 / 0x100_0000 as f32
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
}

/// What a soul is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    /// Drifting up.
    Rising,
    /// Coming to the cursor to listen (seconds so far).
    Leaning(f32),
    /// Whispering line `line`, `t` seconds into it, hovering where it began.
    Whispering { line: usize, t: f32 },
}

#[derive(Debug, Clone)]
pub struct Soul {
    pub pos: Pt,
    pub vel: Pt,
    pub age: f32,
    pub life: f32,
    pub size: f32,
    phase: f32,
    pub mode: Mode,
    /// Where it hovers while it whispers.
    anchor: Pt,
    pub trail: [Pt; TRAIL],
}

/// What to paint for one soul.
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub pos: Pt,
    /// 1 is full size.
    pub scale: f32,
    /// 0..1 how visible.
    pub alpha: f32,
    /// 0..1 how far it has opened its eyes to listen.
    pub eyes: f32,
    /// 0..1 how red it has gone (once enough has been heard, a soul reddens as its time runs out).
    pub red: f32,
    /// The tail, newest first.
    pub trail: [Pt; TRAIL],
    /// Its words and how visible they are (0..1), while it whispers.
    pub words: Option<(&'static str, f32)>,
}

#[derive(Debug, Clone)]
pub struct Field {
    rng: Rng,
    pub souls: Vec<Soul>,
    spawn_in: f32,
    still_for: f32,
    last_pointer: Option<Pt>,
    pointer_speed: f32,
    rest: f32,
    last_line: Option<usize>,
    /// Bit `n` is set once line `n` has been heard (kept for the trophies to come).
    pub heard: u32,
    /// How many whispers there have been in all (kept with the save, for the `GUARANTEE`).
    pub spoken: u32,
}

fn dist(a: Pt, b: Pt) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Field {
    pub fn new(seed: u64) -> Field {
        Field {
            rng: Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1),
            souls: Vec::new(),
            spawn_in: 0.0,
            still_for: 0.0,
            last_pointer: None,
            pointer_speed: 0.0,
            rest: 0.0,
            last_line: None,
            heard: 0,
            spoken: 0,
        }
    }

    /// How many lines have been heard.
    pub fn heard_count(&self) -> u32 {
        (self.heard & ((1u32 << LINES.len()) - 1)).count_ones()
    }

    /// Take back what a save remembers: the lines heard and how many whispers there have been (an
    /// older save that only knows the lines still counts one whisper for each).
    pub fn restore(&mut self, heard: u32, spoken: u32) {
        self.heard |= heard;
        self.spoken = self.spoken.max(spoken).max(self.heard_count());
    }

    /// A random line, not the one just said. But once the whispers that are left (within the
    /// `GUARANTEE`) are no more than the lines not yet heard, it is one of those.
    fn pick_line(&mut self) -> usize {
        let n = LINES.len();
        let unheard: Vec<usize> = (0..n).filter(|i| self.heard & (1 << i) == 0).collect();
        let left = GUARANTEE.saturating_sub(self.spoken) as usize;
        let pool: Vec<usize> = if !unheard.is_empty() && left <= unheard.len() {
            unheard
        } else {
            (0..n).filter(|i| Some(*i) != self.last_line).collect()
        };
        pool[((self.rng.next() >> 20) % pool.len() as u64) as usize]
    }

    /// One soul speaks: choose its line and count it.
    fn speak(&mut self) -> usize {
        let line = self.pick_line();
        self.heard |= 1 << line;
        self.spoken = self.spoken.saturating_add(1);
        self.last_line = Some(line);
        line
    }

    fn spawn(&mut self, view: Pt, spawn_y: &[f32]) {
        let y = if spawn_y.is_empty() { view.1 - 20.0 } else { spawn_y[(self.rng.next() % spawn_y.len() as u64) as usize] };
        let x = self.rng.range(view.0 * 0.04, view.0 * 0.96);
        let pos = (x, y);
        self.souls.push(Soul {
            pos,
            vel: (0.0, -self.rng.range(8.0, 20.0)),
            age: 0.0,
            life: self.rng.range(7.5, 11.5),
            size: self.rng.range(0.75, 1.25),
            phase: self.rng.range(0.0, std::f32::consts::TAU),
            mode: Mode::Rising,
            anchor: pos,
            trail: [pos; TRAIL],
        });
    }

    /// Moves everything on by `dt` seconds. `view` is the size of the window area, `spawn_y` the
    /// heights (from its top) where the lava shows and souls may come up, and `pointer` the cursor.
    pub fn step(&mut self, dt: f32, view: Pt, spawn_y: &[f32], pointer: Option<Pt>) {
        let dt = if dt.is_finite() { dt.clamp(0.0, 0.1) } else { 0.0 };
        if dt <= 0.0 || !(view.0 >= 8.0 && view.1 >= 8.0) {
            return;
        }
        // the cursor: how fast it moves, and for how long it has been still
        self.pointer_speed = match (pointer, self.last_pointer) {
            (Some(p), Some(q)) => dist(p, q) / dt,
            _ => 0.0,
        };
        self.last_pointer = pointer;
        if pointer.is_some() && self.pointer_speed < STILL_SPEED {
            self.still_for += dt;
        } else {
            self.still_for = 0.0;
        }
        self.rest = (self.rest - dt).max(0.0);

        // new souls come up now and then
        self.spawn_in -= dt;
        if self.spawn_in <= 0.0 {
            if self.souls.len() < MAX_SOULS {
                self.spawn(view, spawn_y);
            }
            self.spawn_in = self.rng.range(0.25, 0.6);
        }

        // a soul comes to listen when the cursor has been still near one for a while
        if let (Some(p), true) = (pointer, self.still_for >= STILL_FOR && self.rest <= 0.0) {
            let nearest = self
                .souls
                .iter()
                .enumerate()
                .filter(|(_, s)| s.mode == Mode::Rising && s.age > 1.5 && alpha_of(s) >= 0.5)
                .map(|(i, s)| (i, dist(s.pos, p)))
                .filter(|(_, d)| *d <= LISTEN_RADIUS)
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((i, _)) = nearest {
                self.souls[i].mode = Mode::Leaning(0.0);
                self.rest = REST;
            }
        }

        let speed = self.pointer_speed;
        let mut begin: Vec<(usize, Pt)> = Vec::new();
        for (i, s) in self.souls.iter_mut().enumerate() {
            let before = s.pos;
            match s.mode {
                Mode::Rising => {
                    s.age += dt;
                    // drifts up and sways on its own
                    let want = ((s.age * 0.9 + s.phase).sin() * 9.0, -16.0 - 6.0 * (s.age * 0.4 + s.phase).sin());
                    let mut ax = (want.0 - s.vel.0) * 1.4;
                    let mut ay = (want.1 - s.vel.1) * 1.4;
                    if let Some(p) = pointer {
                        let d = dist(s.pos, p);
                        if speed >= FAST_SPEED && d < SCATTER_RADIUS && d > 0.01 {
                            // thrown away from a fast cursor
                            let push = 1.0 - d / SCATTER_RADIUS;
                            ax += (s.pos.0 - p.0) / d * 1400.0 * push;
                            ay += (s.pos.1 - p.1) / d * 1400.0 * push;
                        } else if speed < 450.0 && d < CURIOUS_RADIUS && d > 18.0 {
                            // curious: drawn gently towards a slow cursor
                            let pull = 1.0 - d / CURIOUS_RADIUS;
                            ax += (p.0 - s.pos.0) / d * 46.0 * pull;
                            ay += (p.1 - s.pos.1) / d * 46.0 * pull;
                        }
                    }
                    s.vel.0 += ax * dt;
                    s.vel.1 += ay * dt;
                    s.pos.0 += s.vel.0 * dt;
                    s.pos.1 += s.vel.1 * dt;
                }
                Mode::Leaning(t) => {
                    let t = t + dt;
                    if let Some(p) = pointer {
                        // to a spot beside the cursor
                        let target = (p.0 + 34.0, p.1 - 26.0);
                        let k = (dt * 3.5).min(1.0);
                        s.pos.0 += (target.0 - s.pos.0) * k;
                        s.pos.1 += (target.1 - s.pos.1) * k;
                    }
                    s.vel = (0.0, 0.0);
                    if pointer.is_none() || speed > 250.0 {
                        // the cursor moved off: it loses interest
                        s.mode = Mode::Rising;
                        s.vel = (0.0, -20.0);
                    } else if t >= LEAN_TIME {
                        begin.push((i, s.pos));
                        s.mode = Mode::Whispering { line: 0, t: 0.0 };
                    } else {
                        s.mode = Mode::Leaning(t);
                    }
                }
                Mode::Whispering { line, t } => {
                    let t = t + dt;
                    // hovers where it began, bobbing a little
                    s.pos.0 = s.anchor.0 + (t * 1.3).sin() * 1.5;
                    s.pos.1 = s.anchor.1 + (t * 1.7).sin() * 2.0;
                    if t >= WHISPER_TIME {
                        // done: it lets go and slips upward, and is gone soon after
                        s.mode = Mode::Rising;
                        s.vel = (0.0, -30.0);
                        s.life = s.age + 2.2;
                    } else {
                        s.mode = Mode::Whispering { line, t };
                    }
                }
            }
            // the tail follows
            if dist(before, s.pos) > 0.0 {
                s.trail.rotate_right(1);
                s.trail[0] = s.pos;
            } else {
                s.trail[0] = s.pos;
            }
        }
        // the ones that began to whisper this step choose their lines and stay where they are
        for (i, at) in begin {
            let line = self.speak();
            self.souls[i].mode = Mode::Whispering { line, t: 0.0 };
            self.souls[i].anchor = at;
        }
        // souls whose time is up, or that have left the top of the view, are gone
        self.souls.retain(|s| match s.mode {
            Mode::Rising => s.age < s.life && s.pos.1 > -40.0,
            _ => true,
        });
    }

    /// Whether the souls have started to go red (enough lines have been heard).
    pub fn reddening(&self) -> bool {
        self.heard_count() >= RED_AFTER
    }

    /// What to paint, far to near (smallest first).
    pub fn looks(&self) -> Vec<Look> {
        let reddening = self.reddening();
        let mut out: Vec<Look> = self
            .souls
            .iter()
            .map(|s| {
                let (eyes, words) = match s.mode {
                    Mode::Rising => (0.0, None),
                    Mode::Leaning(t) => (smooth(t / LEAN_TIME), None),
                    Mode::Whispering { line, t } => {
                        // the words fade in, stay, and fade out
                        let a = smooth(t / 0.7).min(smooth((WHISPER_TIME - t) / 0.9));
                        (1.0, Some((LINES[line % LINES.len()].text, a)))
                    }
                };
                let red = if reddening && s.mode == Mode::Rising { smooth((s.age - 0.5 * s.life) / (0.4 * s.life)) } else { 0.0 };
                Look { pos: s.pos, scale: s.size, alpha: alpha_of(s), eyes, red, trail: s.trail, words }
            })
            .collect();
        out.sort_by(|a, b| a.scale.total_cmp(&b.scale));
        out
    }
}

/// How visible a soul is: it fades in as it comes up and out as its time runs down, and stays
/// clear while it listens.
fn alpha_of(s: &Soul) -> f32 {
    match s.mode {
        Mode::Rising => smooth(s.age / 1.2).min(smooth((s.life - s.age) / 1.8)),
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: Pt = (500.0, 700.0);
    const LAVA: [f32; 2] = [300.0, 520.0];

    fn run(f: &mut Field, seconds: f32, pointer: Option<Pt>) {
        for _ in 0..(seconds / 0.033) as usize {
            f.step(0.033, VIEW, &LAVA, pointer);
        }
    }

    #[test]
    fn souls_come_up_from_the_lava_and_rise_and_the_crowd_is_capped() {
        let mut f = Field::new(1);
        run(&mut f, 3.0, None);
        assert!(!f.souls.is_empty());
        for s in &f.souls {
            assert!(s.pos.0 >= 0.0 && s.pos.0 <= VIEW.0);
        }
        let start: Vec<f32> = f.souls.iter().map(|s| s.pos.1).collect();
        let before = f.souls.len();
        run(&mut f, 1.0, None);
        let risen = f.souls.iter().take(before.min(start.len())).zip(&start).filter(|(s, y)| s.pos.1 < **y).count();
        assert!(risen * 2 >= before.min(start.len()), "most of them went up");
        run(&mut f, 60.0, None);
        assert!(f.souls.len() <= MAX_SOULS);
        // they do not pile up for ever: the old ones are gone
        assert!(f.souls.iter().all(|s| s.age <= 11.6 + 0.1 || s.mode != Mode::Rising));
    }

    #[test]
    fn the_same_seed_gives_the_same_souls_and_another_seed_gives_others() {
        let (mut a, mut b, mut c) = (Field::new(5), Field::new(5), Field::new(6));
        for f in [&mut a, &mut b, &mut c] {
            run(f, 6.0, Some((200.0, 300.0)));
        }
        assert_eq!(a.looks(), b.looks());
        assert_ne!(a.looks(), c.looks());
    }

    #[test]
    fn nothing_happens_with_no_time_or_no_view_and_odd_numbers_are_tamed() {
        let mut f = Field::new(2);
        f.step(0.0, VIEW, &LAVA, None);
        f.step(0.03, (0.0, 0.0), &LAVA, None);
        f.step(f32::NAN, VIEW, &LAVA, None);
        assert!(f.souls.is_empty());
        f.step(30.0, VIEW, &LAVA, Some((f32::NAN, 5.0)));
        assert!(f.souls.len() <= 1, "a huge step counts as a short one");
        // with nowhere listed to come up from, they come up at the bottom
        let mut g = Field::new(3);
        for _ in 0..40 {
            g.step(0.05, VIEW, &[], None);
        }
        assert!(!g.souls.is_empty() && g.souls.iter().all(|s| s.pos.1 > 100.0));
    }

    #[test]
    fn a_still_cursor_brings_a_soul_to_whisper_and_it_lets_go_afterwards() {
        let mut f = Field::new(9);
        // let some come up, with the cursor far away
        run(&mut f, 6.0, Some((20.0, 20.0)));
        // find a soul that can listen and hold the cursor next to it
        let target = f.souls.iter().find(|s| s.age > 2.0 && alpha_of(s) >= 0.5).map(|s| s.pos).expect("a soul");
        let cursor = (target.0 - 20.0, target.1 + 10.0);
        let mut whispered = None;
        for _ in 0..200 {
            f.step(0.033, VIEW, &LAVA, Some(cursor));
            if let Some(w) = f.looks().into_iter().find_map(|l| l.words.filter(|w| w.1 > 0.8)) {
                whispered = Some(w.0);
                break;
            }
        }
        let words = whispered.expect("it whispered");
        assert!(LINES.iter().any(|l| l.text == words));
        assert_eq!(f.heard_count(), 1);
        // it stays while it speaks, even if the cursor goes off
        run(&mut f, 1.0, Some((400.0, 650.0)));
        assert!(f.souls.iter().any(|s| matches!(s.mode, Mode::Whispering { .. })));
        // and lets go once the words are done, and is gone soon after
        run(&mut f, WHISPER_TIME + 1.0, Some((400.0, 650.0)));
        assert!(!f.souls.iter().any(|s| matches!(s.mode, Mode::Whispering { .. })));
        run(&mut f, 4.0, Some((400.0, 650.0)));
        assert!(f.looks().iter().all(|l| l.words.is_none()));
    }

    #[test]
    fn a_moving_cursor_gets_no_whisper() {
        let mut f = Field::new(4);
        run(&mut f, 6.0, None);
        for i in 0..300 {
            // always moving, always near the souls
            let p = (100.0 + (i % 50) as f32 * 6.0, 200.0 + (i % 37) as f32 * 8.0);
            f.step(0.033, VIEW, &LAVA, Some(p));
        }
        assert_eq!(f.heard_count(), 0);
        assert!(f.souls.iter().all(|s| !matches!(s.mode, Mode::Whispering { .. })));
    }

    #[test]
    fn souls_rest_between_whispers_and_all_the_lines_come_round_in_the_end() {
        let mut f = Field::new(11);
        // as if all but a dozen of the guaranteed whispers had been said: the rest cover every line left
        f.spoken = GUARANTEE - LINES.len() as u32;
        run(&mut f, 5.0, Some((10.0, 10.0)));
        let mut spoken = std::collections::BTreeSet::new();
        // keep the cursor still in the middle of where they rise
        let mut longest_quiet = f32::MAX;
        let mut last_start = None::<f32>;
        let mut t = 0.0;
        let mut was = false;
        for _ in 0..(LINES.len() as f32 * (WHISPER_TIME + REST + 4.0) / 0.033) as usize * 2 {
            f.step(0.033, VIEW, &LAVA, Some((250.0, 420.0)));
            t += 0.033;
            let now = f.souls.iter().any(|s| matches!(s.mode, Mode::Whispering { .. }));
            for s in &f.souls {
                if let Mode::Whispering { line, .. } = s.mode {
                    spoken.insert(line);
                }
            }
            if now && !was {
                if let Some(l) = last_start {
                    longest_quiet = longest_quiet.min(t - l);
                }
                last_start = Some(t);
            }
            was = now;
        }
        assert_eq!(spoken.len(), LINES.len(), "every line was heard: {spoken:?}");
        assert_eq!(f.heard_count() as usize, LINES.len());
        assert!(longest_quiet >= WHISPER_TIME, "one at a time: {longest_quiet}");
    }

    #[test]
    fn a_fast_cursor_scatters_the_souls_around_it_and_a_slow_one_draws_them() {
        let mut f = Field::new(7);
        run(&mut f, 6.0, None);
        let centre = f.souls[0].pos;
        // a slow cursor: souls come closer to it
        let mut slow = f.clone();
        let mut p = (centre.0 + 90.0, centre.1);
        for _ in 0..40 {
            p.0 += 0.2;
            slow.step(0.033, VIEW, &LAVA, Some(p));
        }
        let drawn = slow.souls.iter().filter(|s| s.age > 0.0).map(|s| dist(s.pos, p)).fold(f32::MAX, f32::min);
        assert!(drawn < 90.0);
        // a fast one throws a soul next to it away: one soul, the cursor sweeping past on its left
        let mut one = Field::new(8);
        one.spawn_in = 1e9;
        let soul = Soul { pos: (250.0, 400.0), vel: (0.0, 0.0), age: 3.0, life: 20.0, size: 1.0, phase: 0.0, mode: Mode::Rising, anchor: (250.0, 400.0), trail: [(250.0, 400.0); TRAIL] };
        one.souls.push(soul);
        one.step(0.033, VIEW, &LAVA, Some((100.0, 400.0)));
        one.step(0.033, VIEW, &LAVA, Some((200.0, 400.0)));
        assert!(one.pointer_speed >= FAST_SPEED, "{}", one.pointer_speed);
        assert!(one.souls[0].vel.0 > 20.0, "pushed to the right, away from the cursor: {:?}", one.souls[0].vel);
        // the same sweep at a crawl does nothing of the kind: it is drawn in instead
        let mut crawl = Field::new(8);
        crawl.spawn_in = 1e9;
        crawl.souls.push(one.souls[0].clone());
        crawl.souls[0].pos = (250.0, 400.0);
        crawl.souls[0].vel = (0.0, 0.0);
        crawl.step(0.033, VIEW, &LAVA, Some((200.0, 400.0)));
        crawl.step(0.033, VIEW, &LAVA, Some((200.5, 400.0)));
        assert!(crawl.souls[0].vel.0 < 5.0, "{:?}", crawl.souls[0].vel);
    }

    #[test]
    fn what_is_painted_is_visible_only_when_it_should_be() {
        let mut f = Field::new(3);
        run(&mut f, 10.0, None);
        let looks = f.looks();
        assert_eq!(looks.len(), f.souls.len());
        for l in &looks {
            assert!((0.0..=1.0).contains(&l.alpha) && l.scale > 0.0 && l.eyes == 0.0 && l.words.is_none());
        }
        assert!(looks.windows(2).all(|w| w[0].scale <= w[1].scale), "far first");
        // a soul that has just come up is faint, and one at the end of its time too
        let fresh = Soul { pos: (1.0, 1.0), vel: (0.0, 0.0), age: 0.0, life: 9.0, size: 1.0, phase: 0.0, mode: Mode::Rising, anchor: (1.0, 1.0), trail: [(1.0, 1.0); TRAIL] };
        assert_eq!(alpha_of(&fresh), 0.0);
        assert_eq!(alpha_of(&Soul { age: 9.0, ..fresh.clone() }), 0.0);
        assert_eq!(alpha_of(&Soul { age: 4.0, ..fresh }), 1.0);
    }

    #[test]
    fn the_lines_are_short_enough_to_read_and_some_point_at_the_story() {
        assert!(LINES.len() <= 32, "one bit each");
        for l in &LINES {
            assert!(l.text.len() > 10 && l.text.len() <= 64, "{}", l.text);
            assert!(l.text.is_ascii() || l.text.chars().all(|c| !c.is_control()));
        }
        assert!(LINES.iter().filter(|l| l.clue).count() >= 3);
        let mut seen = std::collections::HashSet::new();
        assert!(LINES.iter().all(|l| seen.insert(l.text)), "no line twice");
    }

    #[test]
    fn once_enough_is_heard_the_souls_go_red_as_their_time_runs_out() {
        let mut f = Field::new(12);
        run(&mut f, 12.0, None);
        assert!(!f.reddening());
        assert!(f.looks().iter().all(|l| l.red == 0.0), "nothing red before anything is heard");
        // heard enough (as if loaded from a save)
        f.heard = (1 << RED_AFTER) - 1;
        assert!(f.reddening());
        run(&mut f, 12.0, None);
        let looks = f.looks();
        assert!(looks.iter().any(|l| l.red > 0.5), "the old ones are red");
        assert!(looks.iter().any(|l| l.red == 0.0), "the young ones are not");
        assert!(looks.iter().all(|l| (0.0..=1.0).contains(&l.red)));
        // one less is not enough
        let mut g = Field::new(12);
        g.heard = (1 << (RED_AFTER - 1)) - 1;
        assert!(!g.reddening());
        // a soul that is whispering stays its own colour
        let soul = Soul { pos: (5.0, 5.0), vel: (0.0, 0.0), age: 8.0, life: 9.0, size: 1.0, phase: 0.0, mode: Mode::Whispering { line: 0, t: 1.0 }, anchor: (5.0, 5.0), trail: [(5.0, 5.0); TRAIL] };
        f.souls.push(soul);
        assert_eq!(f.looks().iter().find(|l| l.words.is_some()).unwrap().red, 0.0);
    }

    #[test]
    fn the_heard_lines_can_be_given_back_from_a_save() {
        let mut f = Field::new(1);
        f.restore(0b101, 0);
        assert_eq!(f.heard_count(), 2);
        // an older save that only knew the lines still counts a whisper for each
        assert_eq!(f.spoken, 2);
        f.restore(0, 7);
        assert_eq!(f.spoken, 7);
        // a smaller count never takes it back
        f.restore(0, 3);
        assert_eq!(f.spoken, 7);
        // bits beyond the lines do not count
        f.restore(u32::MAX, 0);
        assert_eq!(f.heard_count() as usize, LINES.len());
    }

    #[test]
    fn the_lines_come_at_random_and_never_the_same_one_twice_running() {
        let mut firsts = std::collections::BTreeSet::new();
        let mut in_order = 0;
        let mut repeats_before_all = 0;
        for seed in 0..300u64 {
            let mut f = Field::new(seed);
            let said: Vec<usize> = (0..GUARANTEE).map(|_| f.speak()).collect();
            firsts.insert(said[0]);
            assert!(said.windows(2).all(|w| w[0] != w[1]), "seed {seed}: {said:?}");
            if said.iter().take(LINES.len()).copied().eq(0..LINES.len()) {
                in_order += 1;
            }
            let mut seen = std::collections::HashSet::new();
            if !said.iter().take(LINES.len()).all(|l| seen.insert(*l)) {
                repeats_before_all += 1;
            }
        }
        assert!(firsts.len() >= 8, "the first one varies: {firsts:?}");
        assert!(in_order <= 3, "not just the list in order: {in_order}");
        assert!(repeats_before_all > 100, "it is not a shuffle that deals each one once: {repeats_before_all}");
    }

    #[test]
    fn whatever_the_luck_every_line_is_heard_by_the_guaranteed_whisper() {
        for seed in 0..2000u64 {
            let mut f = Field::new(seed);
            for n in 1..=GUARANTEE {
                f.speak();
                assert_eq!(f.spoken, n);
            }
            assert_eq!(f.heard_count() as usize, LINES.len(), "seed {seed}: all heard in {GUARANTEE}");
        }
    }

    #[test]
    fn the_guarantee_holds_across_sessions_and_for_a_save_that_runs_late() {
        // quit and come back all the way along: the count is kept, so the last few are still forced
        let mut f = Field::new(5);
        for _ in 0..20 {
            f.speak();
        }
        let (heard, spoken) = (f.heard, f.spoken);
        let mut g = Field::new(99);
        g.restore(heard, spoken);
        for _ in 0..(GUARANTEE - spoken) {
            g.speak();
        }
        assert_eq!(g.heard_count() as usize, LINES.len());
        // the last whisper left with one line missing: that is the one
        let mut h = Field::new(3);
        h.heard = ((1u32 << LINES.len()) - 1) & !(1 << 8);
        h.spoken = GUARANTEE - 1;
        assert_eq!(h.speak(), 8);
        // past the guarantee with one still missing (a save from before): it comes next, then it is free
        let mut late = Field::new(4);
        late.heard = ((1u32 << LINES.len()) - 1) & !(1 << 2);
        late.spoken = GUARANTEE + 40;
        assert_eq!(late.speak(), 2);
        assert_eq!(late.heard_count() as usize, LINES.len());
        let after: Vec<usize> = (0..50).map(|_| late.speak()).collect();
        assert!(after.windows(2).all(|w| w[0] != w[1]) && after.iter().all(|l| *l < LINES.len()));
        // and with everything heard the count just keeps going
        assert_eq!(late.spoken, GUARANTEE + 40 + 51);
    }
}
