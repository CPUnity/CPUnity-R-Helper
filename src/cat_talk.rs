//! The talk with the cat before he has a crown: the way into Game 1.
//!
//! He sits in a night market, half aged by what it cost him to climb out of Time, ill, composed, and he
//! needs a favour. You can ask him things, and then say yes or no. Yes starts Game 1 (the story is in
//! `claude/cpunity-opening-cat-plan.md`); no lets him go on waiting, and you can come back.
//!
//! Only the two opening lines are Trey's (kept word for word). Every other line here is a PLACEHOLDER
//! in his voice for him to rewrite.
//!
//! Also here: his cough. Now and then it starts with a twitch of the head, the very same twitch the
//! dead Monarch King has later (`king::dead_pose`), so that it is familiar when it comes back.
//!
//! Logic only: numbers in, numbers out, with time always passed in, so all of it can be tested. The
//! scene (`ui/cat_talk.rs`) turns it into the picture.

/// Who is speaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    Cat,
    You,
}

/// Trey's two opening lines, word for word.
pub const OPENING: [&str; 2] = [
    "Don't mind me, just trying to figure out how to buy time.",
    "You know actually.. I can offer a fancy new UI if you help me?",
];

/// What the player can say. (The bars on screen.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// "Who are you?"
    Who,
    /// "What fancy UI?"
    Ui,
    /// "Who is in your way?"
    Time,
    /// "What is wrong with your eye?"
    Eye,
    /// "I'll help." (agree)
    Help,
    /// "Not my problem." (disagree)
    Decline,
}

impl Ask {
    /// The questions, in the order they are offered.
    pub const QUESTIONS: [Ask; 4] = [Ask::Who, Ask::Ui, Ask::Time, Ask::Eye];

    /// What the bar says. (Placeholders.)
    pub fn label(self) -> &'static str {
        match self {
            Ask::Who => "Who are you?",
            Ask::Ui => "What fancy UI?",
            Ask::Time => "Who is in your way?",
            Ask::Eye => "What happened to your eye?",
            Ask::Help => "I'll help you.",
            Ask::Decline => "Not my problem.",
        }
    }

    fn index(self) -> usize {
        match self {
            Ask::Who => 0,
            Ask::Ui => 1,
            Ask::Time => 2,
            Ask::Eye => 3,
            Ask::Help => 4,
            Ask::Decline => 5,
        }
    }

    /// What he says back. (Placeholders.) `declines` is how many times you had said no before this
    /// talk; only his answer to "Not my problem." depends on it.
    fn answer(self, declines: u8) -> &'static [&'static str] {
        match self {
            Ask::Who => &[
                "Nobody you'd bother with. Not yet.",
                "Let's say I'm a cat with an appointment he can't afford to keep.",
            ],
            Ask::Ui => &[
                "The (???) Edition. It\u{2019}s a good deal nicer than this one.",
                "I've seen the plans. I wouldn't lie about the plans.",
            ],
            Ask::Time => &["Him. In the clock.", "Don't stare, he takes it personally."],
            Ask::Eye => &["Hm? An old injury. It's nothing.", "...It's nothing."],
            Ask::Help => &[
                "Good. Good. Thank you.",
                "I should tell you something first.",
                "I'm not just buying time. I'm leaving it.",
                "And he wants me back.",
            ],
            Ask::Decline => decline_reply(declines),
        }
    }
}

/// From this many earlier refusals on, he has said everything there is to say and answers only with
/// dots (see `decline_reply`).
pub const SILENT_AFTER: u8 = 6;

/// His answer to "Not my problem.", given how many times you had already said it. He remembers: he
/// is polite the first time, short the second, then more and more upset about the time being wasted,
/// until he tells you the truth (half of him is dying and he needs you), and after that, dots.
/// (Placeholders, except the second: "Yes yes.. You said that the first time.")
pub fn decline_reply(before: u8) -> &'static [&'static str] {
    match before {
        0 => &["Yeah. Fair.", "If you change your mind, I'll be right here. I'm not going far."],
        1 => &["Yes yes.. You said that the first time.", "The offer hasn't changed. Neither has the clock."],
        2 => &["Again? Really?", "Every time you come back just to say no, those are seconds I don't have!"],
        3 => &["...Hm. And there goes the anger. All that's left is annoyance.", "Farewell."],
        4 => &["Stop. Please. Stop spending what's left of it.", "I did not climb out of that clock to be turned down over and over."],
        5 => &[
            "...Fine. You want the truth?",
            "Half of me is dying. Look at the eye. That wasn't an injury.",
            "It spreads a little every day: the cheek, the fang. The clock is taking back what it lent me.",
            "I can't do this alone. I need your help. Please.",
        ],
        _ => &["..."],
    }
}

/// How he looks while he answers "Not my problem." the `before`th time: a little angry the third time.
fn decline_mood(before: u8) -> Mood {
    if before == 2 {
        Mood::Annoyed
    } else {
        Mood::Calm
    }
}

/// What he says when you come back, given how many times you have said no. The first time he is
/// polite and surprised; each visit after that is different, and from `SILENT_AFTER` it is only dots.
/// (Placeholders, except the first.)
pub fn greeting(declines: u8) -> &'static [&'static str] {
    match declines {
        0 => &OPENING,
        1 => &["Oh. You came back.", "The offer still stands."],
        2 => &["Back so soon.", "Tell me you've reconsidered."],
        3 => &["Hm. You.", "Let me guess."],
        4 => &["You came back.", "Please. Tell me you came to say yes."],
        5 => &["...Oh. It's you.", "I'm out of ways to ask nicely."],
        _ => &["..."],
    }
}

/// How the talk ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// You agreed: Game 1 starts.
    Help,
    /// You said no: he waits, and you can come back.
    Decline,
}

/// The look on his face while a line is said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mood {
    #[default]
    Calm,
    /// Slightly angry: the brows come down and the eyes narrow a little.
    Annoyed,
}

/// One line on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
    pub who: Speaker,
    pub text: &'static str,
    pub mood: Mood,
}

impl Line {
    fn new(who: Speaker, text: &'static str, mood: Mood) -> Line {
        Line { who, text, mood }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum State {
    /// Lines to read, one click each; `then` says what comes after the last one.
    Reading {
        lines: Vec<Line>,
        at: usize,
        then: Then,
    },
    /// The bars are up.
    Choosing,
    Done(Outcome),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Then {
    Choose,
    Finish(Outcome),
}

/// The talk.
#[derive(Debug, Clone, PartialEq)]
pub struct Talk {
    state: State,
    asked: [bool; 4],
    /// How many times you have said no before this talk.
    declines: u8,
}

impl Talk {
    /// A new talk. `declines` is how many times you have said no to him before.
    pub fn new(declines: u8) -> Talk {
        let lines: Vec<Line> = greeting(declines)
            .iter()
            .map(|&text| Line::new(Speaker::Cat, text, Mood::Calm))
            .collect();
        Talk {
            state: State::Reading { lines, at: 0, then: Then::Choose },
            asked: [false; 4],
            declines,
        }
    }

    /// The line on screen, if he or you are speaking.
    pub fn line(&self) -> Option<Line> {
        match &self.state {
            State::Reading { lines, at, .. } => lines.get(*at).copied(),
            _ => None,
        }
    }

    /// The player clicked through the line on screen.
    pub fn advance(&mut self) {
        let State::Reading { lines, at, then } = &mut self.state else { return };
        if *at + 1 < lines.len() {
            *at += 1;
            return;
        }
        self.state = match *then {
            Then::Choose => State::Choosing,
            Then::Finish(o) => State::Done(o),
        };
    }

    /// Whether the bars are up.
    pub fn choosing(&self) -> bool {
        self.state == State::Choosing
    }

    /// The bars to show now: the questions you have not asked yet, then yes and no.
    pub fn options(&self) -> Vec<Ask> {
        if !self.choosing() {
            return Vec::new();
        }
        let mut out: Vec<Ask> =
            Ask::QUESTIONS.iter().copied().filter(|q| !self.asked[q.index()]).collect();
        out.push(Ask::Help);
        out.push(Ask::Decline);
        out
    }

    /// The player picked a bar. Their words come first, then his.
    pub fn choose(&mut self, ask: Ask) {
        if !self.options().contains(&ask) {
            return;
        }
        if ask.index() < 4 {
            self.asked[ask.index()] = true;
        }
        let mood = if ask == Ask::Decline { decline_mood(self.declines) } else { Mood::Calm };
        let mut lines = vec![Line::new(Speaker::You, ask.label(), Mood::Calm)];
        lines.extend(
            ask.answer(self.declines).iter().map(|&text| Line::new(Speaker::Cat, text, mood)),
        );
        let then = match ask {
            Ask::Help => Then::Finish(Outcome::Help),
            Ask::Decline => Then::Finish(Outcome::Decline),
            _ => Then::Choose,
        };
        self.state = State::Reading { lines, at: 0, then };
    }

    /// How it ended, once it has.
    pub fn outcome(&self) -> Option<Outcome> {
        match self.state {
            State::Done(o) => Some(o),
            _ => None,
        }
    }

    /// How many times you had said no before this talk.
    pub fn declines(&self) -> u8 {
        self.declines
    }

    /// How many questions have been asked.
    pub fn asked(&self) -> usize {
        self.asked.iter().filter(|&&a| a).count()
    }
}

/// How many characters of a line are shown `secs` after it appeared (it is typed out).
pub fn revealed(text: &str, secs: f32) -> usize {
    const CHARS_PER_SEC: f32 = 42.0;
    ((secs.max(0.0) * CHARS_PER_SEC) as usize).min(text.chars().count())
}

// ----------------------------------------------------------------------------------- the street

/// The clock tower's light swells every this many seconds.
pub const CLOCK_PULSE_PERIOD: f32 = 6.5;
/// The first swell comes this long after the page shows the street.
pub const CLOCK_PULSE_FIRST: f32 = 2.0;
/// One swell lasts this long.
pub const CLOCK_PULSE_SECS: f32 = 2.4;

/// How the clock tower's light looks at one moment.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ClockLight {
    /// 0 the clock's own dim glow .. 1 the light at its brightest
    pub glow: f32,
    /// while a swell lasts: how far a ring of light has spread from the clock, 0..1
    pub ring: Option<f32>,
}

/// The clock tower's light `t` seconds after the street showed: every so often it swells, a ring of
/// light leaves the clock, and it sinks again. (This replaces the red line that ran from the clock
/// to the cat.)
pub fn clock_light(t: f32) -> ClockLight {
    if t < CLOCK_PULSE_FIRST {
        return ClockLight::default();
    }
    let p = (t - CLOCK_PULSE_FIRST).rem_euclid(CLOCK_PULSE_PERIOD);
    if p >= CLOCK_PULSE_SECS {
        return ClockLight::default();
    }
    let u = p / CLOCK_PULSE_SECS;
    let s = (std::f32::consts::PI * u).sin();
    ClockLight { glow: s * s, ring: Some(u) }
}

/// A lane of the street: which way the cats in it walk and how fast (street widths per second).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lane {
    pub dir: f32,
    pub speed: f32,
}

/// The far lane (small, dark, walking to the right) and the near lane (walking to the left).
pub const LANES: [Lane; 2] = [Lane { dir: 1.0, speed: 0.020 }, Lane { dir: -1.0, speed: 0.030 }];

/// A cat out for a walk, seen from the side, in ordinary clothes. The numbers pick from the
/// painter's palettes (they wrap round).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Walker {
    pub lane: usize,
    /// where it is at t = 0, in street widths from the left edge
    pub x0: f32,
    pub coat: u8,
    pub fur: u8,
    /// 0 bare-headed, 1 flat cap, 2 bowler
    pub hat: u8,
    pub scarf: bool,
    pub bag: bool,
}

const fn walker(
    lane: usize,
    x0: f32,
    coat: u8,
    fur: u8,
    hat: u8,
    scarf: bool,
    bag: bool,
) -> Walker {
    Walker { lane, x0, coat, fur, hat, scarf, bag }
}

/// Everyone walking by: single files in each lane.
pub const WALKERS: [Walker; 9] = [
    walker(0, 0.02, 0, 1, 1, false, false),
    walker(0, 0.10, 3, 0, 0, true, false),
    walker(0, 0.19, 5, 2, 2, false, true),
    walker(0, 0.60, 1, 3, 0, true, false),
    walker(1, 0.95, 2, 4, 2, false, false),
    walker(1, 0.86, 6, 1, 0, true, true),
    walker(1, 0.45, 4, 0, 1, false, false),
    walker(1, 0.36, 7, 2, 0, true, false),
    walker(1, 0.28, 0, 3, 2, false, true),
];

/// Walkers go this far past each edge before they come round again.
pub const WALK_MARGIN: f32 = 0.08;

/// Where a walker is `t` seconds in, in street widths from the left edge (a little outside 0..1 as
/// it enters and leaves).
pub fn walker_x(w: &Walker, t: f32) -> f32 {
    let lane = LANES[w.lane];
    let span = 1.0 + 2.0 * WALK_MARGIN;
    (w.x0 + lane.dir * lane.speed * t + WALK_MARGIN).rem_euclid(span) - WALK_MARGIN
}

// ----------------------------------------------------------------------------------- the age

/// Where the age has spread, as a path in head units (from the middle of the head, before it turns):
/// out of the monocled eye, down his right cheek (ours on the right), into the fang on that side.
pub const SPREAD: [(f32, f32); 8] = [
    (150.0, -45.0),
    (196.0, 12.0),
    (226.0, 82.0),
    (196.0, 150.0),
    (128.0, 186.0),
    (74.0, 200.0),
    (66.0, 250.0),
    (68.0, 296.0),
];

/// The spread as a dense path: `steps` points between each pair.
pub fn spread_path(steps: usize) -> Vec<(f32, f32)> {
    let steps = steps.max(1);
    let mut out = Vec::with_capacity((SPREAD.len() - 1) * steps + 1);
    for w in SPREAD.windows(2) {
        for i in 0..steps {
            let t = i as f32 / steps as f32;
            out.push((w[0].0 + (w[1].0 - w[0].0) * t, w[0].1 + (w[1].1 - w[0].1) * t));
        }
    }
    out.push(SPREAD[SPREAD.len() - 1]);
    out
}

/// How aged the fur is, 0..1, at `dist` head units from the path, where the nearest point is
/// `reach` (0 at the eye .. 1 at the tooth) of the way along it. `ripple` (-0.5..0.5) roughens the
/// edge like creeping frost. Wide round the eye, a thread by the tooth.
pub fn age_amount(dist: f32, reach: f32, ripple: f32) -> f32 {
    let width = 124.0 - 62.0 * reach.clamp(0.0, 1.0);
    let t = (1.15 - (dist + ripple * 60.0) / width).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The frost-white heart of it, 0..1.
pub fn age_core(dist: f32, reach: f32, ripple: f32) -> f32 {
    let width = 124.0 - 62.0 * reach.clamp(0.0, 1.0);
    (1.0 - (dist + ripple * 60.0) / (width * 0.45)).clamp(0.0, 1.0) * 0.55
}

/// How far from the path (head units), and how far along it (0..1), a point is.
pub fn nearest_on_path(path: &[(f32, f32)], p: (f32, f32)) -> (f32, f32) {
    let mut best = (f32::MAX, 0.0);
    for (k, q) in path.iter().enumerate() {
        let d = ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt();
        if d < best.0 {
            best = (d, k as f32 / (path.len().max(2) - 1) as f32);
        }
    }
    best
}

// ----------------------------------------------------------------------------------- the cough

/// The twitch: the dead Monarch King's own (`king::dead_pose`): this many degrees, over this long.
pub const TWITCH_DEG: f32 = 2.6;
pub const TWITCH_SECS: f32 = 0.09;
/// About one cough in this many starts with the twitch. (Never two in a row, and never the first.)
pub const TWITCH_ONE_IN: u32 = 3;
/// How long the cough itself lasts, and the pause between the twitch and it.
pub const COUGH_SECS: f32 = 1.7;
const TWITCH_GAP: f32 = 0.22;
/// Seconds between coughs.
pub const GAP_MIN: f32 = 20.0;
pub const GAP_MAX: f32 = 40.0;
/// The first cough comes this long after the scene opens (just after his first line).
pub const FIRST_AT: f32 = 4.5;

/// What the cough does to him at one moment.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CoughPose {
    /// degrees, clockwise (the head)
    pub tilt: f32,
    /// the shoulders and head jolt, in picture pixels (down is positive)
    pub shake: f32,
    /// 0..1: the eyes screwed shut
    pub squint: f32,
    /// 0..1: how far the handkerchief has come up
    pub hanky: f32,
    /// 0..1: the face of pain, brows drawn up
    pub brow: f32,
    /// 0..1: the mouth open
    pub jaw: f32,
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// His cough, on a schedule that is the same for the same seed.
#[derive(Debug, Clone, PartialEq)]
pub struct Cough {
    rng: u32,
    /// when the next one starts
    next: f32,
    /// the one under way: when it started and whether it began with the twitch
    now: Option<(f32, bool)>,
    count: u32,
    last_twitched: bool,
}

impl Cough {
    pub fn new(seed: u32) -> Cough {
        Cough { rng: seed | 1, next: FIRST_AT, now: None, count: 0, last_twitched: false }
    }

    fn roll(&mut self) -> u32 {
        // xorshift32
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x
    }

    /// The most the whole cough takes, twitch included.
    pub fn total(twitch: bool) -> f32 {
        COUGH_SECS + if twitch { TWITCH_GAP } else { 0.0 }
    }

    /// Moves time on to `t` (seconds since the scene opened). Starts the next cough when it is due.
    pub fn update(&mut self, t: f32) {
        if let Some((start, twitch)) = self.now {
            if t - start >= Cough::total(twitch) {
                self.now = None;
            }
        }
        if self.now.is_none() && t >= self.next {
            let twitch = self.count > 0 && !self.last_twitched && self.roll() % TWITCH_ONE_IN == 0;
            self.now = Some((self.next, twitch));
            self.count += 1;
            self.last_twitched = twitch;
            let gap = GAP_MIN + (GAP_MAX - GAP_MIN) * (self.roll() % 1000) as f32 / 1000.0;
            self.next += Cough::total(twitch) + gap;
        }
    }

    /// How many coughs have started.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Whether the one under way began with the twitch.
    pub fn twitching(&self) -> bool {
        matches!(self.now, Some((_, true)))
    }

    /// Whether he is coughing at `t`.
    pub fn coughing(&self, t: f32) -> bool {
        self.now.is_some_and(|(s, tw)| t >= s && t - s < Cough::total(tw))
    }

    /// What it does to him at `t`.
    pub fn pose(&self, t: f32) -> CoughPose {
        let Some((start, twitch)) = self.now else { return CoughPose::default() };
        let age = t - start;
        if age < 0.0 || age >= Cough::total(twitch) {
            return CoughPose::default();
        }
        let mut p = CoughPose::default();
        // the twitch: exactly the dead king's (a sine bump of TWITCH_DEG degrees over TWITCH_SECS)
        let body = if twitch {
            if age < TWITCH_SECS {
                p.tilt = TWITCH_DEG * (age / TWITCH_SECS * std::f32::consts::PI).sin();
            }
            age - TWITCH_GAP
        } else {
            age
        };
        if body < 0.0 {
            return p;
        }
        // the cough: the head dips, the eyes shut, the cloth comes up, the body jolts in quick bursts
        let env = smooth(body / 0.18) * (1.0 - smooth((body - (COUGH_SECS - 0.4)) / 0.4));
        p.tilt += 9.0 * env;
        p.squint = 0.9 * env;
        p.brow = 0.9 * env;
        p.jaw = 0.18 * env;
        p.hanky = smooth(body / 0.16) * (1.0 - smooth((body - (COUGH_SECS - 0.35)) / 0.35));
        // three or four bursts
        p.shake = env * 5.0 * (std::f32::consts::TAU * 3.2 * body).sin().max(-0.4);
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_all(t: &mut Talk) -> Vec<Line> {
        let mut seen = Vec::new();
        while let Some(l) = t.line() {
            seen.push(l);
            t.advance();
        }
        seen
    }

    #[test]
    fn the_talk_opens_with_his_two_lines_word_for_word_and_then_the_bars() {
        let mut t = Talk::new(0);
        let seen = read_all(&mut t);
        assert_eq!(seen.len(), 2);
        assert_eq!(
            seen[0],
            Line::new(
                Speaker::Cat,
                "Don't mind me, just trying to figure out how to buy time.",
                Mood::Calm
            )
        );
        assert_eq!(
            seen[1],
            Line::new(
                Speaker::Cat,
                "You know actually.. I can offer a fancy new UI if you help me?",
                Mood::Calm
            )
        );
        assert!(t.choosing() && t.line().is_none() && t.outcome().is_none());
        // the questions, then yes and no
        let o = t.options();
        assert_eq!(o, vec![Ask::Who, Ask::Ui, Ask::Time, Ask::Eye, Ask::Help, Ask::Decline]);
        assert_eq!(Ask::Help.label(), "I'll help you.");
    }

    #[test]
    fn a_question_is_answered_and_leaves_the_bars() {
        let mut t = Talk::new(0);
        read_all(&mut t);
        t.choose(Ask::Ui);
        let seen = read_all(&mut t);
        assert_eq!(seen[0], Line::new(Speaker::You, "What fancy UI?", Mood::Calm));
        assert!(seen[1..].iter().all(|l| l.who == Speaker::Cat) && seen.len() >= 2);
        assert_eq!(seen[1].text, "The (???) Edition. It\u{2019}s a good deal nicer than this one.");
        assert!(t.choosing());
        assert_eq!(t.asked(), 1);
        assert!(!t.options().contains(&Ask::Ui), "asked once");
        assert!(
            t.options().contains(&Ask::Help) && t.options().contains(&Ask::Decline),
            "yes and no are always there"
        );
        // asking one that is gone does nothing
        t.choose(Ask::Ui);
        assert!(t.choosing());
        // all the questions
        for q in [Ask::Who, Ask::Time, Ask::Eye] {
            t.choose(q);
            read_all(&mut t);
        }
        assert_eq!(t.options(), vec![Ask::Help, Ask::Decline]);
    }

    #[test]
    fn yes_ends_the_talk_with_the_reveal_and_starts_the_game() {
        let mut t = Talk::new(0);
        read_all(&mut t);
        t.choose(Ask::Help);
        let seen = read_all(&mut t);
        assert_eq!(seen[0].who, Speaker::You);
        let said: Vec<&str> = seen.iter().map(|l| l.text).collect();
        assert!(said.iter().any(|s| s.contains("leaving it")), "{said:?}");
        assert!(said.iter().any(|s| s.contains("wants me back")), "{said:?}");
        assert_eq!(t.outcome(), Some(Outcome::Help));
        assert!(!t.choosing() && t.options().is_empty());
    }

    #[test]
    fn no_ends_the_talk_and_the_next_one_begins_differently() {
        let mut t = Talk::new(0);
        read_all(&mut t);
        t.choose(Ask::Decline);
        read_all(&mut t);
        assert_eq!(t.outcome(), Some(Outcome::Decline));
        // coming back: he does not repeat himself
        let mut again = Talk::new(1);
        let seen = read_all(&mut again);
        assert!(seen.iter().all(|l| !OPENING.contains(&l.text)));
        assert_eq!(seen.len(), 2);
        assert!(again.choosing());
        assert_eq!(again.declines(), 1);
    }

    #[test]
    fn he_remembers_every_refusal_and_runs_out_of_patience() {
        // the second time he says so, in his own words
        assert_eq!(decline_reply(1)[0], "Yes yes.. You said that the first time.");
        // every earlier refusal gets a different answer
        let all: Vec<&[&str]> = (0..=5).map(decline_reply).collect();
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                assert_ne!(all[i], all[j], "{i} and {j} are answered the same");
            }
        }
        // the truth comes out once, at the end of his patience: half of him is dying, and he asks for help
        let truth: Vec<&&[&str]> =
            all.iter().filter(|r| r.iter().any(|l| l.contains("dying"))).collect();
        assert_eq!(truth.len(), 1);
        assert!(decline_reply(5).iter().any(|l| l.contains("need your help")));
        // after that it is only dots, however often you go on
        for n in [6u8, 7, 50, 255] {
            assert_eq!(decline_reply(n), &["..."]);
        }
        // the talk gives the answer for how many times you had said no
        for before in [1u8, 3, 5, 6] {
            let mut t = Talk::new(before);
            read_all(&mut t);
            t.choose(Ask::Decline);
            let seen = read_all(&mut t);
            let said: Vec<&str> =
                seen.iter().filter(|l| l.who == Speaker::Cat).map(|l| l.text).collect();
            assert_eq!(said, decline_reply(before), "{before}");
        }
        // and once he is silent he greets you with dots too
        let mut t = Talk::new(SILENT_AFTER);
        assert_eq!(read_all(&mut t).iter().map(|l| l.text).collect::<Vec<_>>(), vec!["..."]);
    }

    #[test]
    fn he_greets_you_differently_each_time_you_come_back_until_it_is_only_dots() {
        // the first visit is his two lines, word for word; the first return is the polite one
        assert_eq!(greeting(0), OPENING);
        assert_eq!(greeting(1), ["Oh. You came back.", "The offer still stands."]);
        // "Oh. You came back." is only the first return
        let all: Vec<&[&str]> = (0..=5).map(greeting).collect();
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                assert_ne!(all[i], all[j], "visits {i} and {j} are greeted the same");
            }
            if i > 1 {
                assert!(!all[i].contains(&"Oh. You came back."), "{i}");
            }
        }
        // the seventh visit and after: dots
        assert_eq!(greeting(SILENT_AFTER), &["..."]);
        assert_eq!(greeting(200), &["..."]);
        // and the talk opens with them
        for before in 1..=5u8 {
            let mut t = Talk::new(before);
            let said: Vec<&str> = read_all(&mut t).iter().map(|l| l.text).collect();
            assert_eq!(said, greeting(before));
        }
    }

    #[test]
    fn he_is_a_little_angry_only_the_third_time_he_is_turned_down() {
        for before in 0..=7u8 {
            let mut t = Talk::new(before);
            read_all(&mut t);
            t.choose(Ask::Decline);
            let seen = read_all(&mut t);
            for l in seen.iter().filter(|l| l.who == Speaker::Cat) {
                assert_eq!(
                    l.mood,
                    if before == 2 { Mood::Annoyed } else { Mood::Calm },
                    "{before}: {}",
                    l.text
                );
            }
            assert!(seen.iter().filter(|l| l.who == Speaker::You).all(|l| l.mood == Mood::Calm));
        }
        // the fourth answer lets the anger go and says farewell
        assert_eq!(decline_reply(3).last(), Some(&"Farewell."));
        assert!(decline_reply(3)[0].contains("annoyance"));
        // questions never make him angry
        let mut t = Talk::new(2);
        read_all(&mut t);
        t.choose(Ask::Who);
        assert!(read_all(&mut t).iter().all(|l| l.mood == Mood::Calm));
    }

    #[test]
    fn the_reveal_cannot_be_dodged_by_the_questions_and_the_bars_do_nothing_mid_line() {
        let mut t = Talk::new(0);
        // mid-line: nothing to choose
        assert!(t.options().is_empty());
        t.choose(Ask::Help);
        assert!(t.outcome().is_none() && t.line().is_some());
        for _ in 0..3 {
            t.advance();
        }
        assert!(t.choosing());
        // none of the answers give away that he is leaving time
        for q in Ask::QUESTIONS {
            assert!(
                q.answer(0).iter().all(|s| !s.contains("leaving") && !s.contains("wants me back")),
                "{q:?}"
            );
        }
    }

    #[test]
    fn lines_are_typed_out() {
        let text = "Hello there";
        assert_eq!(revealed(text, 0.0), 0);
        assert!(revealed(text, 0.1) > 0 && revealed(text, 0.1) < text.len());
        assert_eq!(revealed(text, 5.0), text.len());
        assert_eq!(revealed(text, -1.0), 0);
    }

    fn run(seed: u32, secs: f32) -> (Vec<(f32, bool)>, Cough) {
        let mut c = Cough::new(seed);
        let mut seen = Vec::new();
        let mut last = c.count();
        let mut t = 0.0;
        while t < secs {
            c.update(t);
            if c.count() != last {
                last = c.count();
                seen.push((t, c.twitching()));
            }
            t += 0.01;
        }
        (seen, c)
    }

    #[test]
    fn he_coughs_every_twenty_to_forty_seconds_and_the_first_is_plain() {
        let (coughs, _) = run(7, 1500.0);
        assert!(coughs.len() > 30, "{}", coughs.len());
        assert!(
            (coughs[0].0 - FIRST_AT).abs() < 0.05 && !coughs[0].1,
            "the first is plain, just after his first line"
        );
        for w in coughs.windows(2) {
            let gap = w[1].0 - w[0].0;
            assert!(
                gap >= GAP_MIN - 0.1 && gap <= GAP_MAX + COUGH_SECS + TWITCH_GAP + 0.1,
                "{gap}"
            );
        }
        // the same seed, the same coughs
        assert_eq!(run(7, 300.0).0, run(7, 300.0).0);
        assert_ne!(run(7, 300.0).0, run(8, 300.0).0);
    }

    #[test]
    fn only_now_and_then_does_a_cough_start_with_the_twitch_and_never_twice_running() {
        let mut total = 0;
        let mut twitched = 0;
        for seed in 1..=40u32 {
            let (coughs, _) = run(seed, 3000.0);
            let mut prev = false;
            for (_, tw) in &coughs {
                assert!(!(prev && *tw), "never two in a row");
                prev = *tw;
            }
            total += coughs.len();
            twitched += coughs.iter().filter(|c| c.1).count();
        }
        let share = twitched as f32 / total as f32;
        assert!(
            share > 0.15 && share < 0.40,
            "about one in three (a little less, as none follows another): {share}"
        );
    }

    #[test]
    fn the_twitch_is_the_dead_kings_own() {
        // the same bump: TWITCH_DEG over TWITCH_SECS, matching king::dead_pose's
        let dead = |phase: f32| crate::king::dead_pose(phase + 3.7 * 10.0, None, true).tilt;
        for k in [0.2f32, 0.5, 0.8] {
            let phase = k * TWITCH_SECS;
            let ours = TWITCH_DEG * (phase / TWITCH_SECS * std::f32::consts::PI).sin();
            assert!((ours - dead(phase)).abs() < 0.05, "{ours} vs {}", dead(phase));
        }
        // and at the end of it, nothing
        assert!(dead(TWITCH_SECS + 0.01).abs() < 0.001);
        // a twitching cough starts with the head snapping over, then the cough
        let mut c = Cough::new(1);
        let mut t = 0.0;
        let mut twitch_start = None;
        while t < 4000.0 && twitch_start.is_none() {
            c.update(t);
            if c.twitching() && c.coughing(t) {
                twitch_start = Some(t);
            }
            t += 0.005;
        }
        let s = twitch_start.expect("one twitching cough in an hour");
        let peak = c.pose(s + TWITCH_SECS / 2.0);
        assert!(
            peak.tilt > 2.0 && peak.tilt < 3.0 && peak.hanky == 0.0 && peak.squint == 0.0,
            "{peak:?}"
        );
        let mid = c.pose(s + TWITCH_GAP + 0.6);
        assert!(mid.tilt > 6.0 && mid.hanky > 0.9 && mid.squint > 0.7, "{mid:?}");
    }

    #[test]
    fn a_plain_cough_dips_the_head_shuts_the_eyes_raises_the_cloth_and_ends_calm() {
        let mut c = Cough::new(3);
        c.update(FIRST_AT);
        assert!(c.coughing(FIRST_AT + 0.01) && !c.twitching());
        let before = c.pose(FIRST_AT - 0.5);
        assert_eq!(before, CoughPose::default(), "before it starts nothing moves");
        let mid = c.pose(FIRST_AT + 0.7);
        assert!(mid.tilt > 8.0 && mid.squint > 0.8 && mid.hanky > 0.95 && mid.jaw > 0.1, "{mid:?}");
        let end = c.pose(FIRST_AT + COUGH_SECS - 0.01);
        assert!(end.tilt < 0.5 && end.hanky < 0.05 && end.squint < 0.05, "{end:?}");
        // the body jolts, mostly down (and never far up)
        let shakes: Vec<f32> = (0..170).map(|k| c.pose(FIRST_AT + k as f32 * 0.01).shake).collect();
        assert!(
            shakes.iter().cloned().fold(f32::MIN, f32::max) > 3.0
                && shakes.iter().cloned().fold(f32::MAX, f32::min) >= -2.1
        );
        c.update(FIRST_AT + COUGH_SECS + 0.1);
        assert!(!c.coughing(FIRST_AT + COUGH_SECS + 0.1));
        assert_eq!(c.pose(FIRST_AT + COUGH_SECS + 0.1), CoughPose::default());
    }

    #[test]
    fn the_clock_light_swells_every_so_often_and_otherwise_rests() {
        assert_eq!(clock_light(0.0), ClockLight::default());
        assert_eq!(clock_light(CLOCK_PULSE_FIRST - 0.01), ClockLight::default());
        // the middle of a swell is the brightest, with a ring half way out
        let mid = clock_light(CLOCK_PULSE_FIRST + CLOCK_PULSE_SECS / 2.0);
        assert!(mid.glow > 0.99 && (mid.ring.unwrap() - 0.5).abs() < 0.01, "{mid:?}");
        // between swells it rests
        assert_eq!(clock_light(CLOCK_PULSE_FIRST + CLOCK_PULSE_SECS + 0.5), ClockLight::default());
        // it comes round again
        let again = clock_light(CLOCK_PULSE_FIRST + CLOCK_PULSE_PERIOD + CLOCK_PULSE_SECS / 2.0);
        assert!((again.glow - mid.glow).abs() < 1e-4);
        // the ring only widens within a swell, the glow never leaves 0..1, and forty seconds hold six swells
        let mut last: Option<f32> = None;
        let mut swells = 0;
        for i in 0..4000 {
            let l = clock_light(i as f32 * 0.01);
            assert!((0.0..=1.0).contains(&l.glow));
            match (l.ring, last) {
                (Some(r), None) => {
                    swells += 1;
                    assert!((0.0..1.0).contains(&r));
                }
                (Some(r), Some(prev)) => assert!(r >= prev),
                _ => {}
            }
            last = l.ring;
        }
        assert_eq!(swells, 6);
        assert!(CLOCK_PULSE_SECS < CLOCK_PULSE_PERIOD);
    }

    #[test]
    fn the_cats_walk_by_in_lines_the_way_their_lane_goes() {
        for lane in 0..LANES.len() {
            let mut xs: Vec<f32> =
                WALKERS.iter().filter(|w| w.lane == lane).map(|w| w.x0).collect();
            assert!(xs.len() >= 4, "lane {lane} has {} walkers", xs.len());
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            // nobody walks into the one in front: even the gap that wraps round is wide
            let span = 1.0 + 2.0 * WALK_MARGIN;
            let mut gaps: Vec<f32> = xs.windows(2).map(|p| p[1] - p[0]).collect();
            gaps.push(span - (xs[xs.len() - 1] - xs[0]));
            assert!(gaps.iter().all(|&g| g >= 0.07), "{lane}: {gaps:?}");
        }
        for w in WALKERS.iter() {
            let lane = LANES[w.lane];
            let mut moved = 0.0;
            for i in 0..3000 {
                let x = walker_x(w, i as f32 * 0.1);
                assert!((-WALK_MARGIN - 1e-4..1.0 + WALK_MARGIN + 1e-4).contains(&x), "{x}");
                if i > 0 {
                    // one step is a small step in the lane's direction, or the wrap from one edge to the other
                    let step = x - walker_x(w, (i - 1) as f32 * 0.1);
                    if step.abs() < 0.5 {
                        assert!(step * lane.dir > 0.0, "{step}");
                        moved += step.abs();
                    }
                }
            }
            assert!(moved > 1.0, "{moved}");
        }
        // two lanes, going opposite ways, the near one faster
        assert!(LANES[0].dir == -LANES[1].dir && LANES[1].speed > LANES[0].speed);
    }

    #[test]
    fn the_age_is_widest_at_the_eye_and_thins_to_a_thread_at_the_tooth() {
        let path = spread_path(14);
        assert_eq!(path.len(), 7 * 14 + 1);
        assert_eq!(path[0], SPREAD[0]);
        assert_eq!(path[path.len() - 1], SPREAD[7]);
        // on the path: fully aged; far from it: not at all
        assert!(age_amount(0.0, 0.0, 0.0) > 0.99 && age_amount(0.0, 1.0, 0.0) > 0.99);
        assert_eq!(age_amount(300.0, 0.0, 0.0), 0.0);
        // the same distance is aged by the eye and clear by the tooth
        assert!(age_amount(90.0, 0.0, 0.0) > 0.3, "{}", age_amount(90.0, 0.0, 0.0));
        assert!(age_amount(90.0, 1.0, 0.0) < 0.05, "{}", age_amount(90.0, 1.0, 0.0));
        // monotonic away from the path
        let mut last = 1.1;
        for d in (0..200).step_by(10) {
            let a = age_amount(d as f32, 0.3, 0.0);
            assert!(a <= last + 1e-6);
            last = a;
        }
        // the frost heart is only near the path
        assert!(age_core(0.0, 0.0, 0.0) > 0.5 && age_core(100.0, 0.5, 0.0) == 0.0);
        // a point beside the eye is near the start, one beside the tooth near the end
        let (d0, r0) = nearest_on_path(&path, (160.0, -40.0));
        let (d1, r1) = nearest_on_path(&path, (70.0, 290.0));
        assert!(d0 < 15.0 && r0 < 0.05 && d1 < 8.0 && r1 > 0.95, "{d0} {r0} {d1} {r1}");
    }
}
