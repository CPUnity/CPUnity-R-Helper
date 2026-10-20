//! The camera at the Devil's Table: where you are looking, how close, and what your body does about
//! it. You are in the chair, so the camera is your head: it turns and leans in on whoever is
//! talking, snaps onto the Bear when he roars, and then you flinch, screw your eyes up and look
//! away at the felt. When it is Father Time's face you are close to, he blows smoke at you.
//!
//! Logic only: numbers in, numbers out, with time always passed in, so all of it can be tested. The
//! table screen (`ui/poker.rs`) turns a `Frame` into the picture.
use crate::poker_game::{BEAR, CAT, FATHER, MONARCH};

/// A view: how much the board is magnified, and the board point that the view is centred on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shot {
    pub zoom: f32,
    pub x: f32,
    pub y: f32,
}

/// The whole table.
pub const WIDE: Shot = Shot { zoom: 1.0, x: 550.0, y: 350.0 };
/// Looking down at your own cards after you flinch from the Bear.
pub const AWAY: Shot = Shot { zoom: 1.12, x: 470.0, y: 440.0 };

/// How much of the board's width the camera shows when it is all the way back, in board pixels (the
/// people are spread over 1100 of it, so that nobody at the edges is cut off).
pub const VIEW_W: f32 = 1200.0;

/// Who the camera can turn to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    Cat,
    Dealer,
    Bear,
    Monarch,
    Father,
}

impl Who {
    pub fn of_seat(seat: usize) -> Option<Who> {
        match seat {
            CAT => Some(Who::Cat),
            BEAR => Some(Who::Bear),
            FATHER => Some(Who::Father),
            MONARCH => Some(Who::Monarch),
            _ => None,
        }
    }

    /// Where their face is on the board (1100 x 700).
    pub fn face(self) -> (f32, f32) {
        match self {
            Who::Cat => (60.0, 255.0),
            Who::Dealer => (255.0, 250.0),
            Who::Bear => (470.0, 232.0),
            Who::Monarch => (715.0, 222.0),
            Who::Father => (985.0, 210.0),
        }
    }

    /// How much of the board is in the picture when the camera is close on them (it is the width, in
    /// board pixels, so that nobody next to them is in shot).
    pub fn span(self) -> f32 {
        match self {
            Who::Cat => 270.0,
            Who::Dealer => 220.0,
            Who::Bear => 340.0,
            Who::Monarch => 340.0,
            Who::Father => 330.0,
        }
    }

    /// The zoom that is a close shot of them: just them, in the middle.
    pub fn close(self) -> f32 {
        VIEW_W / self.span()
    }

    pub fn shot(self, zoom: f32) -> Shot {
        let (x, y) = self.face();
        Shot { zoom, x, y }
    }
}

/// How much closer than a close shot the camera goes for a reaction, and for the Bear's roar.
pub const REACT_CLOSER: f32 = 1.12;
pub const ROAR_CLOSER: f32 = 1.22;

/// How long the camera stays on the Bear after he roars, and when the flinch comes.
pub const ROAR_LEN_RAGE: f64 = 4.0;
pub const ROAR_LEN_JOY: f64 = 3.6;
pub const FLINCH_AT: f32 = 1.9;
/// How long Father Time's breath takes, from the breath in to the last of the haze.
pub const BLOW_LEN: f32 = 3.6;
/// The least time between two breaths of smoke, and how long the camera is on him before the first.
pub const BLOW_EVERY: f64 = 26.0;
pub const BLOW_AFTER: f64 = 0.9;

/// What the picture needs this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub zoom: f32,
    pub x: f32,
    pub y: f32,
    /// A tremor, in board pixels.
    pub shake: (f32, f32),
    /// How far your eyes are screwed shut: 0 open, 1 nearly closed.
    pub squint: f32,
    /// Seconds since Father Time began to blow (None when he is not).
    pub blow: Option<f32>,
}

/// Rises from 0 at `a` to `peak` at `b`, holds to `c`, and is gone again at `d` (all smooth).
pub fn bump(x: f32, a: f32, b: f32, c: f32, d: f32, peak: f32) -> f32 {
    let smooth = |t: f32| {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    if x <= a || x >= d {
        0.0
    } else if x < b {
        peak * smooth((x - a) / (b - a).max(1e-6))
    } else if x <= c {
        peak
    } else {
        peak * (1.0 - smooth((x - c) / (d - c).max(1e-6)))
    }
}

/// What your body does `age` seconds into the Bear's roar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Roar {
    pub squint: f32,
    pub shake: f32,
    /// You have turned away from him.
    pub away: bool,
}

pub fn roar_pose(rage: bool, age: f32) -> Roar {
    if rage {
        let blink = bump(age, FLINCH_AT, FLINCH_AT + 0.15, FLINCH_AT + 0.25, FLINCH_AT + 0.7, 0.92);
        let wince = bump(age, FLINCH_AT + 0.1, FLINCH_AT + 0.6, 3.4, 4.2, 0.42);
        // a tremor that builds while he bellows, and a jolt when you flinch
        let build = bump(age, 0.3, 1.5, FLINCH_AT, FLINCH_AT + 0.15, 4.0);
        let jolt = if age >= FLINCH_AT { 10.0 * (-(age - FLINCH_AT) * 8.0).exp() } else { 0.0 };
        Roar { squint: blink.max(wince), shake: build + jolt, away: (FLINCH_AT + 0.05..ROAR_LEN_RAGE as f32).contains(&age) }
    } else {
        Roar { squint: bump(age, 1.2, 1.7, 2.2, 3.0, 0.28), shake: bump(age, 0.3, 1.0, 2.4, 3.2, 1.2), away: false }
    }
}

pub struct Director {
    cur: Shot,
    target: Shot,
    prio: u8,
    until: f64,
    /// Who the camera is on, since when.
    on: Option<(Who, f64)>,
    speaker: Option<Who>,
    roar: Option<(bool, f64)>,
    blow: Option<f64>,
    blow_ready: f64,
    /// Father Time may breathe smoke on his own when the camera is on him (off while the story is read
    /// to you, which says when he does).
    auto_blow: bool,
    /// Where the camera goes on its own when the flinch is over (instead of back to the Bear or out wide).
    then: Option<Option<Who>>,
    /// How close the camera gets to whoever it turns to, as a share of `Who::close` (less when it is only
    /// waiting on them to make up their mind).
    zoom_k: f32,
    /// How fast the camera moves to where it is going (higher is quicker).
    ease: f32,
}

impl Default for Director {
    fn default() -> Director {
        Director::new()
    }
}

impl Director {
    pub fn new() -> Director {
        Director { cur: WIDE, target: WIDE, prio: 0, until: 0.0, on: None, speaker: None, roar: None, blow: None, blow_ready: 0.0, auto_blow: true, then: None, zoom_k: 1.0, ease: 2.4 }
    }

    /// Turns to `who`, at `zoom`, for `hold` seconds. A look of lower priority does not cut in on one
    /// of higher priority that is still going.
    pub fn look(&mut self, who: Who, zoom: f32, hold: f64, prio: u8, now: f64) {
        if prio < self.prio && now < self.until {
            return;
        }
        self.target = who.shot(zoom);
        self.prio = prio;
        self.until = now + hold;
        if self.on.map(|(w, _)| w) != Some(who) {
            self.on = Some((who, now));
        }
    }

    /// Whoever is talking now (None when nobody is): the camera turns to them and stays while they
    /// talk.
    pub fn speak(&mut self, who: Option<Who>, now: f64) {
        // while you are flinching the camera is not to go back to the Bear: whoever talks next is where it goes after
        if self.then.is_some() {
            if who != Some(Who::Bear) {
                self.then = Some(who);
            }
            return;
        }
        match who {
            Some(w) => {
                if self.speaker != Some(w) {
                    self.look(w, w.close() * self.zoom_k, 3600.0, 1, now);
                    if self.prio == 1 {
                        self.speaker = Some(w);
                    }
                }
            }
            None => {
                if self.speaker.take().is_some() && self.prio == 1 {
                    self.until = now;
                }
            }
        }
    }

    /// Something worth looking at happened to `who` (the King's cackle, Father Time's beard, ...).
    pub fn react(&mut self, who: Who, hold: f64, now: f64) {
        self.look(who, who.close() * REACT_CLOSER, hold, 2, now);
    }

    /// How close the camera gets to whoever it turns to next, as a share of the full close shot (1.0 for
    /// someone talking, a little less for someone it is waiting on).
    pub fn set_zoom_k(&mut self, k: f32) {
        self.zoom_k = k.clamp(0.5, 1.0);
    }

    /// How quickly the camera glides to its next shot (2.4 is the usual; a gentler table uses less). Roars and flinches
    /// keep their own speed.
    pub fn set_ease(&mut self, rate: f32) {
        self.ease = rate.clamp(0.6, 6.0);
    }

    /// Whether Father Time may blow smoke at you without being told to.
    pub fn set_auto_blow(&mut self, on: bool) {
        self.auto_blow = on;
    }

    /// Father Time blows smoke at you now (the story says so): the camera is already on him.
    pub fn exhale(&mut self, now: f64) {
        if self.blow.is_none() {
            self.blow = Some(now);
            self.blow_ready = now + BLOW_EVERY;
            self.until = self.until.max(now + BLOW_LEN as f64 + 0.3);
        }
    }

    /// Father Time blows smoke in your face right now (you have just read his line): the camera goes to
    /// him, the smoke comes, and then the camera pulls back out of it.
    pub fn puff(&mut self, now: f64) {
        self.prio = 0;
        self.then = None;
        self.look(Who::Father, Who::Father.close(), BLOW_LEN as f64 + 0.9, 3, now);
        self.speaker = None;
        self.blow = Some(now);
        self.blow_ready = now + BLOW_EVERY;
    }

    /// You flinch from the Bear now (the story says so): your eyes shut and you look away at the felt,
    /// and then the camera moves on by itself to `then` (or out wide, for None).
    pub fn startle(&mut self, now: f64, then: Option<Who>) {
        self.prio = 0;
        self.then = Some(then);
        self.look(Who::Bear, Who::Bear.close(), ROAR_LEN_RAGE - (FLINCH_AT as f64 - 0.4), 3, now);
        self.roar = Some((true, now - (FLINCH_AT as f64 - 0.4)));
    }

    /// The Bear roars (`rage`) or crows (not `rage`): the camera snaps onto him.
    pub fn roar(&mut self, rage: bool, now: f64) {
        let len = if rage { ROAR_LEN_RAGE } else { ROAR_LEN_JOY };
        self.prio = 0;
        self.look(Who::Bear, Who::Bear.close() * ROAR_CLOSER, len, 3, now);
        self.roar = Some((rage, now));
    }

    /// Where the camera is now. `calm` is true when it is your move: the camera lets go of anything
    /// that is only talk, so that you can see the table.
    pub fn frame(&mut self, dt: f32, now: f64, calm: bool) -> Frame {
        if calm && self.prio <= 1 {
            self.until = now.min(self.until);
            self.speaker = None;
        }
        if now >= self.until {
            self.target = WIDE;
            self.prio = 0;
            self.on = None;
            self.speaker = None;
            if let Some(next) = self.then.take() {
                if let Some(w) = next {
                    self.look(w, w.close(), 3600.0, 1, now);
                    self.speaker = Some(w);
                }
            }
        }
        let mut target = self.target;
        let mut rate = self.ease;
        let mut squint = 0.0_f32;
        let mut shake = 0.0_f32;

        if let Some((rage, start)) = self.roar {
            let age = (now - start) as f32;
            if age > 5.5 {
                self.roar = None;
            } else {
                let p = roar_pose(rage, age);
                squint = p.squint;
                shake = p.shake;
                if p.away {
                    target = AWAY;
                    rate = 9.0;
                } else if age < 0.6 {
                    rate = 6.0;
                }
            }
        }

        // Father Time breathes smoke at you when you have been looking at him a moment
        if let Some(start) = self.blow {
            if now - start > BLOW_LEN as f64 {
                self.blow = None;
            }
        }
        if self.auto_blow && self.blow.is_none() && now >= self.blow_ready {
            if let Some((Who::Father, since)) = self.on {
                if self.cur.zoom >= 1.5 && now - since >= BLOW_AFTER {
                    self.blow = Some(now);
                    self.blow_ready = now + BLOW_EVERY;
                    self.until = self.until.max(now + BLOW_LEN as f64 + 0.3);
                }
            }
        }
        let blow = self.blow.map(|s| (now - s) as f32);
        if let Some(a) = blow {
            squint = squint.max(bump(a, 0.8, 1.3, 2.0, 3.0, 0.5));
            shake = shake.max(bump(a, 1.2, 1.4, 2.0, 2.4, 2.2));
        }

        let k = 1.0 - (-rate * dt.clamp(0.0, 0.25)).exp();
        self.cur.zoom += (target.zoom - self.cur.zoom) * k;
        self.cur.x += (target.x - self.cur.x) * k;
        self.cur.y += (target.y - self.cur.y) * k;
        let wob = (
            (now as f32 * 47.0).sin() * shake,
            (now as f32 * 39.0).cos() * shake * 0.7,
        );
        Frame { zoom: self.cur.zoom, x: self.cur.x, y: self.cur.y, shake: wob, squint: squint.clamp(0.0, 0.95), blow }
    }

    /// The camera is on this person (zoomed in at least a little).
    pub fn on(&self) -> Option<Who> {
        self.on.map(|(w, _)| w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(d: &mut Director, from: f64, to: f64, calm: bool) -> Frame {
        let mut now = from;
        let mut f = d.frame(0.0, now, calm);
        while now < to {
            now += 1.0 / 60.0;
            f = d.frame(1.0 / 60.0, now, calm);
        }
        f
    }

    #[test]
    fn waiting_on_someone_turns_to_them_a_little_less_close_than_when_they_talk() {
        let mut talk = Director::new();
        talk.speak(Some(Who::Bear), 0.0);
        let a = run(&mut talk, 0.0, 4.0, false);
        let mut wait = Director::new();
        wait.set_zoom_k(0.82);
        wait.speak(Some(Who::Bear), 0.0);
        let b = run(&mut wait, 0.0, 4.0, false);
        assert!(b.zoom > 2.0 && b.zoom < a.zoom - 0.2, "{b:?} v {a:?}");
        assert!((b.x - Who::Bear.face().0).abs() < 12.0, "on the Bear: {b:?}");
    }

    #[test]
    fn a_gentler_ease_takes_longer_to_arrive() {
        let go = |ease: f32| {
            let mut d = Director::new();
            d.set_ease(ease);
            d.look(Who::Bear, 2.0, 100.0, 1, 0.0);
            let mut f = d.frame(0.0, 0.0, false);
            let mut now = 0.0;
            while now < 0.8 {
                now += 0.02;
                f = d.frame(0.02, now, false);
            }
            f.zoom
        };
        assert!(go(1.2) < go(2.4), "{} {}", go(1.2), go(2.4));
    }

    #[test]
    fn starts_wide_and_stays() {
        let mut d = Director::new();
        let f = run(&mut d, 0.0, 5.0, false);
        assert!((f.zoom - 1.0).abs() < 1e-4 && f.squint == 0.0 && f.blow.is_none());
    }

    #[test]
    fn turns_to_the_speaker_and_back() {
        let mut d = Director::new();
        d.speak(Some(Who::Bear), 0.0);
        let f = run(&mut d, 0.0, 3.0, false);
        assert!(f.zoom > 2.5 && (f.x - Who::Bear.face().0).abs() < 10.0, "{f:?}");
        d.speak(None, 3.0);
        let f = run(&mut d, 3.0, 9.0, false);
        assert!(f.zoom < 1.02, "{f:?}");
    }

    #[test]
    fn a_new_speaker_takes_the_camera_and_your_move_lets_go() {
        let mut d = Director::new();
        d.speak(Some(Who::Cat), 0.0);
        run(&mut d, 0.0, 2.0, false);
        d.speak(Some(Who::Father), 2.0);
        let f = run(&mut d, 2.0, 4.5, false);
        assert!(f.x > 900.0, "{f:?}");
        let f = run(&mut d, 4.5, 9.0, true);
        assert!(f.zoom < 1.05, "{f:?}");
    }

    #[test]
    fn the_bear_roars_you_flinch_and_look_away() {
        let mut d = Director::new();
        d.roar(true, 0.0);
        let f = run(&mut d, 0.0, 1.5, false);
        assert!(f.zoom > 3.0 && (f.x - Who::Bear.face().0).abs() < 25.0, "close on him: {f:?}");
        let mut peak = 0.0_f32;
        let mut now = 1.5;
        let mut last = f;
        while now < 2.6 {
            now += 1.0 / 60.0;
            last = d.frame(1.0 / 60.0, now, false);
            peak = peak.max(last.squint);
        }
        assert!(peak > 0.85, "eyes shut: {peak}");
        let f = run(&mut d, 2.6, 3.6, false);
        assert!(f.y > 380.0 && f.zoom < 1.4, "looking down at the felt: {f:?} (was {last:?})");
        let f = run(&mut d, 3.6, 12.0, false);
        assert!(f.zoom < 1.02 && f.squint == 0.0, "back to normal: {f:?}");
    }

    #[test]
    fn the_roar_is_not_cut_off_by_talk_or_your_move() {
        let mut d = Director::new();
        d.roar(false, 0.0);
        d.speak(Some(Who::Cat), 0.5);
        let f = run(&mut d, 0.5, 1.5, true);
        assert!(f.x > 420.0 && f.x < 520.0 && f.zoom > 2.5, "{f:?}");
    }

    #[test]
    fn joy_only_makes_you_uneasy() {
        for age in [0.0, 0.5, 1.0, 1.7, 2.2, 3.0, 4.0] {
            let p = roar_pose(false, age);
            assert!(!p.away && p.squint <= 0.3, "{age}: {p:?}");
        }
    }

    #[test]
    fn father_time_blows_smoke_once_in_a_while() {
        let mut d = Director::new();
        d.react(Who::Father, 5.0, 0.0);
        let mut blew = Vec::new();
        let mut now = 0.0;
        let mut squint = 0.0_f32;
        while now < 40.0 {
            now += 1.0 / 60.0;
            if now > 27.0 && now < 27.02 {
                d.react(Who::Father, 5.0, now);
            }
            let f = d.frame(1.0 / 60.0, now, false);
            squint = squint.max(f.squint);
            if let Some(a) = f.blow {
                if a == 0.0 {
                    blew.push(now);
                }
            }
        }
        assert_eq!(blew.len(), 2, "{blew:?}");
        assert!(blew[0] > 0.9 && blew[0] < 2.5, "{blew:?}");
        assert!(blew[1] - blew[0] >= BLOW_EVERY, "{blew:?}");
        assert!(squint > 0.4, "your eyes sting: {squint}");
    }

    #[test]
    fn no_smoke_when_you_are_not_looking_at_him() {
        let mut d = Director::new();
        d.react(Who::Bear, 5.0, 0.0);
        let mut now = 0.0;
        while now < 20.0 {
            now += 1.0 / 60.0;
            assert!(d.frame(1.0 / 60.0, now, false).blow.is_none());
        }
    }

    #[test]
    fn a_close_shot_has_only_one_person_in_it() {
        // the width of the picture, in board pixels, must be less than the gap to the next person on each side
        let all = [Who::Cat, Who::Dealer, Who::Bear, Who::Monarch, Who::Father];
        for w in all {
            let shown = VIEW_W / w.close();
            assert!((shown - w.span()).abs() < 0.01);
        }
        for pair in all.windows(2) {
            let gap = pair[1].face().0 - pair[0].face().0;
            assert!(gap > 180.0, "{:?} and {:?} are only {gap} apart", pair[0], pair[1]);
        }
    }

    #[test]
    fn a_puff_of_smoke_holds_the_camera_on_him_whoever_talks_next_and_then_it_pulls_back() {
        let mut d = Director::new();
        d.speak(Some(Who::Father), 0.0);
        run(&mut d, 0.0, 2.0, false);
        d.puff(2.0);
        // the dealer speaks on the very next click: the camera stays on the smoke
        d.speak(Some(Who::Dealer), 2.0);
        let mut now = 2.0;
        let mut saw_smoke = false;
        while now < 2.0 + BLOW_LEN as f64 {
            now += 1.0 / 60.0;
            d.speak(Some(Who::Dealer), now);
            let f = d.frame(1.0 / 60.0, now, false);
            saw_smoke |= f.blow.is_some();
            assert!(f.x > 900.0 && f.zoom > 2.0, "still on Father Time at {now}: {f:?}");
        }
        assert!(saw_smoke);
        // then it goes on to whoever is talking
        let mut f = d.frame(0.0, now, false);
        while now < 9.0 {
            now += 1.0 / 60.0;
            d.speak(Some(Who::Dealer), now);
            f = d.frame(1.0 / 60.0, now, false);
        }
        assert!(f.blow.is_none() && f.x < 400.0, "{f:?}");
    }

    #[test]
    fn the_story_can_make_you_flinch_and_make_him_exhale() {
        let mut d = Director::new();
        d.set_auto_blow(false);
        d.speak(Some(Who::Father), 0.0);
        let mut now = 0.0;
        while now < 3.0 {
            now += 1.0 / 60.0;
            assert!(d.frame(1.0 / 60.0, now, false).blow.is_none(), "not on his own while the story is read");
        }
        d.exhale(now);
        assert!(d.frame(1.0 / 60.0, now, false).blow == Some(0.0));
        let f = run(&mut d, now, now + 2.0, false);
        assert!(f.squint > 0.2);
        // and the flinch
        let mut d = Director::new();
        d.startle(0.0, None);
        let mut peak = 0.0_f32;
        let mut now = 0.0;
        while now < 1.6 {
            now += 1.0 / 60.0;
            peak = peak.max(d.frame(1.0 / 60.0, now, false).squint);
        }
        assert!(peak > 0.85, "{peak}");
        let f = run(&mut d, now, 8.0, false);
        assert!(f.zoom < 1.05);
    }

    #[test]
    fn after_the_flinch_the_camera_goes_to_father_time_and_not_back_to_the_bear() {
        let mut d = Director::new();
        d.speak(Some(Who::Bear), 0.0);
        run(&mut d, 0.0, 2.0, false);
        d.startle(2.0, Some(Who::Father));
        // (a line with a cue does not steer the camera itself; the next lines are Father Time's)
        let mut now = 2.0;
        let mut looked_down = false;
        let mut x_after = 0.0;
        while now < 9.0 {
            now += 1.0 / 60.0;
            if now > 3.0 {
                d.speak(Some(Who::Father), now);
            }
            let f = d.frame(1.0 / 60.0, now, false);
            if f.y > 380.0 && f.zoom < 1.4 {
                looked_down = true;
            }
            if now > 6.0 {
                x_after = f.x;
                assert!(f.zoom > 2.5, "{f:?}");
            }
        }
        assert!(looked_down);
        assert!(x_after > 900.0, "on Father Time: {x_after}");
        // and if the story moves on to someone else meanwhile, it is them
        let mut d = Director::new();
        d.startle(0.0, Some(Who::Father));
        d.speak(Some(Who::Dealer), 0.5);
        let f = run(&mut d, 0.5, 8.0, false);
        assert!((f.x - Who::Dealer.face().0).abs() < 20.0, "{f:?}");
    }

    #[test]
    fn bump_is_a_bump() {
        assert_eq!(bump(0.0, 1.0, 2.0, 3.0, 4.0, 0.5), 0.0);
        assert!((bump(2.5, 1.0, 2.0, 3.0, 4.0, 0.5) - 0.5).abs() < 1e-6);
        assert_eq!(bump(5.0, 1.0, 2.0, 3.0, 4.0, 0.5), 0.0);
        let mut last = 0.0;
        for i in 0..100 {
            let v = bump(1.0 + i as f32 * 0.01, 1.0, 2.0, 3.0, 4.0, 0.5);
            assert!(v >= last);
            last = v;
        }
    }
}
