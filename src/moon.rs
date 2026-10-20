//! The moon that waits in the corner of The Void's backdrop at night, kept free of drawing so it can
//! be tested. From 7pm to 5am (the computer's clock) a small half moon peeks over the top right
//! corner of the page. Pull it out and it slowly becomes the full moon; drag it down and it gets
//! smaller and smaller, as if it were going away into the dark; flick it hard at the staff that
//! stands in the sand at the bottom and it strikes the hourglass on top of it, which breaks and lets
//! the sands of time out. That starts Game 3.
//!
//! Positions are in points, from the top left of the area the backdrop covers (`area` is its width
//! and height). Time is passed in (seconds), so nothing here reads a clock.

use std::f32::consts::PI;

pub type Pt = (f32, f32);

/// Half of the moon is showing over the corner: the radius it has there.
pub const HALF_R: f32 = 22.0;
/// The full moon's radius.
pub const FULL_R: f32 = 56.0;
/// The smallest it gets, when it reaches the staff.
pub const MIN_R: f32 = 9.0;
/// How far the pointer has to travel from the corner to bring the moon fully out.
pub const PULL_LEN: f32 = 150.0;
/// How near the corner the pointer has to be to take hold of the moon.
pub const HIT_R: f32 = 34.0;
/// The speed (points a second) a release needs to count as a flick.
pub const FLICK_SPEED: f32 = 650.0;
/// A flick has to point at the staff to within this much (the cosine of the angle).
pub const AIM_COS: f32 = 0.7;
/// Let go this close to the hourglass and it strikes, however slowly.
pub const TOUCH: f32 = 56.0;
/// Close to the hourglass, a gentler push in its direction is enough: this near, at this speed,
/// and aimed to within this much (the cosine).
pub const NEAR_R: f32 = 140.0;
pub const NEAR_SPEED: f32 = 200.0;
pub const NEAR_AIM_COS: f32 = 0.5;
/// The moon goes back to the corner in this long when it is let go without a flick.
pub const DRIFT_SECS: f32 = 1.1;
/// From the strike to the end of the sequence (Game 3 begins): the whole scene of the monarch.
pub const STRIKE_END: f32 = crate::monarch::REAL_END;
/// Skipping the scene jumps to here (he has just finished appearing), a moment before it ends.
pub const SKIP_TO: f32 = STRIKE_END - 1.0;
/// A skip is only taken after this long, so a stray click right at the strike does not end it.
pub const SKIP_AFTER: f32 = 1.0;
/// Smallest area it is shown in (the staff and the pull need the room).
pub const MIN_AREA: Pt = (260.0, 240.0);

/// A number from 0 to 1 that depends only on `a` and `b`.
fn hash(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B).rotate_left(13);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0xFFFF) as f32 / 65535.0
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_pt(a: Pt, b: Pt, t: f32) -> Pt {
    (lerp(a.0, b.0, t), lerp(a.1, b.1, t))
}

fn dist(a: Pt, b: Pt) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

// ---- when it is there ----------------------------------------------------------------------------

/// Is it night: from 7pm up to 5am.
pub fn is_night(hour: u32) -> bool {
    let h = hour % 24;
    h >= 19 || h < 5
}

/// Whether the moon is shown: at night (or when the Options switch pretends it is), while the staff
/// is not broken yet, and only in an area big enough for it.
pub fn appears(hour: u32, pretend_night: bool, staff_broken: bool, area: Pt) -> bool {
    (pretend_night || is_night(hour)) && !staff_broken && area.0 >= MIN_AREA.0 && area.1 >= MIN_AREA.1
}

// ---- where things are ----------------------------------------------------------------------------

/// Where the moon's middle is while it is tucked in: on the top edge, near the right corner, so the
/// lower half shows.
pub fn corner(area: Pt) -> Pt {
    (area.0 - 36.0, 0.0)
}

/// How tall the staff stands in the sand.
pub fn staff_len(area: Pt) -> f32 {
    (area.1 * 0.42).clamp(70.0, 170.0)
}

/// The foot of the staff, in the middle of the bottom edge.
pub fn staff_base(area: Pt) -> Pt {
    (area.0 * 0.5, area.1)
}

/// The middle of the hourglass on top of the staff: what the moon has to hit.
pub fn staff_head(area: Pt) -> Pt {
    (area.0 * 0.5, area.1 - staff_len(area) + 18.0)
}

/// Where the moon is, how big, and how far out of the corner (0 to 1), when the pointer is at `p`
/// and the moon is being held. It lags behind the pointer at first (it has to be pulled out) and
/// follows it exactly once it is fully out; from there, the lower it goes the smaller it gets.
pub fn held_at(p: Pt, area: Pt) -> (Pt, f32, f32) {
    let a = corner(area);
    let d = dist(p, a);
    if d < 0.001 {
        return (a, HALF_R, 0.0);
    }
    let x = (d / PULL_LEN).min(1.0);
    let pull = x;
    let moon_d = if d >= PULL_LEN { d } else { PULL_LEN * x.powf(1.7) };
    let centre = (a.0 + (p.0 - a.0) / d * moon_d, a.1 + (p.1 - a.1) / d * moon_d);
    let mut radius = lerp(HALF_R, FULL_R, smooth(pull));
    if pull >= 1.0 {
        let head = staff_head(area);
        let from = PULL_LEN * 0.7;
        let s = ((centre.1 - from) / (head.1 - from).max(1.0)).clamp(0.0, 1.0);
        radius = lerp(radius, MIN_R, smooth(s));
    }
    (centre, radius, pull)
}

/// How far the moon's colour fringe (cyan one side, magenta the other) is split, in points: a
/// couple of points all the time, jumping wider for a moment now and then.
/// How much of the staff shows with the moon's middle at height `y`: none until it is down
/// a good way, all of it near the hourglass.
pub fn reveal_at(y: f32, area: Pt) -> f32 {
    let head = staff_head(area).1;
    smooth((y - head * 0.45) / (head * 0.35))
}

pub fn fringe(t: f32) -> f32 {
    let k = (t * 6.0).floor() as i64 as u32;
    if hash(k, 77) > 0.93 {
        6.0
    } else {
        2.0
    }
}

// ---- the moon you pull ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Half a moon over the corner.
    Tucked,
    /// In the hand.
    Held,
    /// Let go without a flick: it floats back to the corner.
    Drifting,
    /// Flicked: on its way to the staff.
    Falling,
    /// It hit: the strike plays (see `strike`).
    Struck,
}

#[derive(Debug, Clone)]
pub struct Moon {
    phase: Phase,
    centre: Pt,
    radius: f32,
    pull: f32,
    /// The pointer's last positions while held: (time, position).
    samples: Vec<(f64, Pt)>,
    /// Where a drift or a fall started, how big it was then, when, and how long it takes.
    from: Pt,
    from_r: f32,
    since: f64,
    secs: f32,
    struck_at: f64,
}

impl Default for Moon {
    fn default() -> Self {
        Moon::new()
    }
}

impl Moon {
    pub fn new() -> Moon {
        Moon {
            phase: Phase::Tucked,
            centre: (0.0, 0.0),
            radius: HALF_R,
            pull: 0.0,
            samples: Vec::new(),
            from: (0.0, 0.0),
            from_r: HALF_R,
            since: 0.0,
            secs: 1.0,
            struck_at: 0.0,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn centre(&self) -> Pt {
        self.centre
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// How far out of the corner it is (0 to 1).
    pub fn pull(&self) -> f32 {
        self.pull
    }

    /// Is the pointer over the moon (so it can be taken hold of, and the cursor can be a hand)?
    pub fn hovered(&self, p: Pt, area: Pt) -> bool {
        match self.phase {
            Phase::Tucked => dist(p, corner(area)) <= HIT_R,
            Phase::Drifting => dist(p, self.centre) <= self.radius.max(HIT_R * 0.7) + 8.0,
            _ => false,
        }
    }

    /// How much of the staff shows: nothing until the moon has been dragged down a good way, then
    /// more the lower it goes (see `reveal_at`), all of it once the moon is on its way to it.
    pub fn staff_reveal(&self, area: Pt) -> f32 {
        match self.phase {
            Phase::Tucked => 0.0,
            Phase::Held | Phase::Drifting => reveal_at(self.centre.1, area),
            Phase::Falling | Phase::Struck => 1.0,
        }
    }

    /// How much everything is glitching (0 to 1): it starts the moment the moon is taken hold of,
    /// worsens as it is pulled out and dragged down, is at its worst while it flies at the staff, and
    /// then dies away slowly after the strike while the sands mend time.
    pub fn glitch_level(&self, now: f64, area: Pt) -> f32 {
        match self.phase {
            Phase::Tucked => 0.0,
            Phase::Held | Phase::Drifting => {
                let base = if self.phase == Phase::Held { 0.08 } else { 0.0 };
                (base + 0.5 * smooth(self.pull * 2.5) + 0.15 * self.pull + 0.35 * reveal_at(self.centre.1, area)).min(1.0)
            }
            Phase::Falling => 1.0,
            Phase::Struck => {
                let since = self.struck_for(now).unwrap_or(0.0);
                // the glitch dies away after the strike; the monarch's lurches and lunge bring it back
                (1.0 - smooth((since - 0.3) / 4.0)).max(crate::monarch::beat(crate::monarch::scene_time(since)).glitch)
            }
        }
    }

    /// Jumps the monarch's scene ahead to where he has just finished appearing (any key or click
    /// does this, once the scene has been going for `SKIP_AFTER`).
    pub fn skip(&mut self, now: f64) -> bool {
        if let Some(t) = self.struck_for(now) {
            if t >= SKIP_AFTER && t < SKIP_TO {
                self.struck_at -= f64::from(SKIP_TO - t);
                return true;
            }
        }
        false
    }

    /// The pointer went down at `p`: takes hold of the moon if it is there. Returns whether it did.
    pub fn press(&mut self, now: f64, p: Pt, area: Pt) -> bool {
        if !self.hovered(p, area) {
            return false;
        }
        self.phase = Phase::Held;
        self.samples.clear();
        self.drag(now, p, area);
        true
    }

    /// The pointer moved to `p` while held.
    pub fn drag(&mut self, now: f64, p: Pt, area: Pt) {
        if self.phase != Phase::Held {
            return;
        }
        self.samples.push((now, p));
        self.samples.retain(|(t, _)| now - t <= 0.3);
        let (centre, radius, pull) = held_at(p, area);
        self.centre = centre;
        self.radius = radius;
        self.pull = pull;
    }

    /// How fast the pointer is going (points a second), over the last tenth of a second or so.
    pub fn velocity(&self, now: f64) -> Pt {
        let recent: Vec<&(f64, Pt)> = self.samples.iter().filter(|(t, _)| now - t <= 0.12).collect();
        let (Some(first), Some(last)) = (recent.first(), recent.last()) else { return (0.0, 0.0) };
        let dt = (last.0 - first.0) as f32;
        if dt <= 0.0 {
            return (0.0, 0.0);
        }
        ((last.1 .0 - first.1 .0) / dt, (last.1 .1 - first.1 .1) / dt)
    }

    /// The pointer was let go. A flick that is fully out, fast enough and aimed at the staff sends
    /// the moon at it; so does letting go right on the hourglass. Anything else and it floats home.
    pub fn release(&mut self, now: f64, area: Pt) {
        if self.phase != Phase::Held {
            return;
        }
        let head = staff_head(area);
        let to = (head.0 - self.centre.0, head.1 - self.centre.1);
        let d = (to.0 * to.0 + to.1 * to.1).sqrt();
        let v = self.velocity(now);
        let speed = (v.0 * v.0 + v.1 * v.1).sqrt();
        let aimed = d > 0.001 && speed > 0.001 && (v.0 * to.0 + v.1 * to.1) / (speed * d) >= AIM_COS;
        let touching = d <= TOUCH;
        // a little push towards it from close by is enough
        let near_aimed = d > 0.001 && speed > 0.001 && (v.0 * to.0 + v.1 * to.1) / (speed * d) >= NEAR_AIM_COS;
        let nudged = d <= NEAR_R && speed >= NEAR_SPEED && near_aimed;
        self.from = self.centre;
        self.from_r = self.radius;
        self.since = now;
        self.samples.clear();
        let seen = reveal_at(self.centre.1, area) >= 0.5;
        if self.pull >= 1.0 && seen && ((speed >= FLICK_SPEED && aimed) || touching || nudged) {
            self.phase = Phase::Falling;
            // it flies at the speed it was thrown, at least fast enough to get there
            self.secs = if touching { 0.12 } else if nudged && speed < FLICK_SPEED { (d / 700.0).max(0.12) } else { (d / speed.clamp(FLICK_SPEED, 2400.0)).max(0.18) };
        } else {
            self.phase = Phase::Drifting;
            self.secs = DRIFT_SECS;
        }
    }

    /// Moves everything along to `now`. Call it every frame.
    pub fn tick(&mut self, now: f64, area: Pt) {
        match self.phase {
            Phase::Tucked => {
                self.centre = corner(area);
                self.radius = HALF_R;
                self.pull = 0.0;
            }
            Phase::Held => {}
            Phase::Drifting => {
                let u = ((now - self.since) as f32 / self.secs).clamp(0.0, 1.0);
                let e = 1.0 - (1.0 - u).powi(3);
                let a = corner(area);
                self.centre = lerp_pt(self.from, a, e);
                self.radius = lerp(self.from_r, HALF_R, e);
                self.pull = (dist(self.centre, a) / PULL_LEN).min(1.0);
                if u >= 1.0 {
                    self.phase = Phase::Tucked;
                    self.centre = a;
                    self.radius = HALF_R;
                    self.pull = 0.0;
                }
            }
            Phase::Falling => {
                let head = staff_head(area);
                let u = ((now - self.since) as f32 / self.secs).clamp(0.0, 1.0);
                self.centre = lerp_pt(self.from, head, u);
                self.radius = lerp(self.from_r, MIN_R, u);
                self.pull = 1.0;
                if u >= 1.0 {
                    self.phase = Phase::Struck;
                    self.struck_at = self.since + f64::from(self.secs);
                    self.centre = head;
                    self.radius = 0.0;
                }
            }
            Phase::Struck => {
                self.centre = staff_head(area);
                self.radius = 0.0;
            }
        }
    }

    /// How long ago the moon hit (None before that).
    pub fn struck_for(&self, now: f64) -> Option<f32> {
        (self.phase == Phase::Struck).then(|| (now - self.struck_at).max(0.0) as f32)
    }

    /// The strike has played out: Game 3 begins.
    pub fn finished(&self, now: f64) -> bool {
        self.struck_for(now).map_or(false, |t| t >= STRIKE_END)
    }
}

// ---- the strike ----------------------------------------------------------------------------------

/// How everything looks `t` seconds after the moon hit the hourglass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strike {
    /// The white-gold flash over the hourglass (0 to 1): up in a flash, then gone.
    pub flash: f32,
    /// Three shockwave rings: (radius, opacity), one after the other.
    pub rings: [(f32, f32); 3],
    /// How far the whole page is shaken, in points.
    pub shake: Pt,
    /// How far the hourglass has come apart (0 to 1) and the rod has cracked.
    pub split: f32,
    /// How much sand has poured out and piled up (0 to 1).
    pub flood: f32,
    /// How far the stars have stretched into slow trails (0 to 1).
    pub trails: f32,
}

pub fn strike(t: f32) -> Strike {
    let t = t.max(0.0);
    let flash = if t < 0.05 { t / 0.05 } else { (-(t - 0.05) / 0.22).exp() };
    let ring = |i: usize| {
        let s = t - 0.04 - 0.14 * i as f32;
        if s <= 0.0 {
            (0.0, 0.0)
        } else {
            (18.0 + 440.0 * s, (1.0 - s / 0.9).clamp(0.0, 1.0) * 0.8)
        }
    };
    let amp = if t < 1.2 { 8.0 * (-t / 0.3).exp() } else { 0.0 };
    Strike {
        flash,
        rings: [ring(0), ring(1), ring(2)],
        shake: ((t * 95.0).sin() * amp, (t * 83.0 + 1.0).cos() * amp * 0.7),
        split: smooth((t - 0.15) / 1.35),
        flood: smooth((t - 1.2) / 3.0),
        trails: smooth((t - 1.0) / 2.0),
    }
}

/// How clear the staff is kept (0 to 1) `t` seconds after the strike. The whole window glitches at
/// the hit, which would hide the very thing the person is looking at, so a calm patch is kept round
/// the staff while it breaks: fully clear for the first 2.4 seconds, then gone by 3.6, when the
/// glitch has mostly died away anyway.
pub fn staff_clear(t: f32) -> f32 {
    if t < 0.0 {
        0.0
    } else {
        1.0 - smooth((t - 2.4) / 1.2)
    }
}

/// The calm patch round the staff: its middle and how far it reaches either way (half the width,
/// half the height). It takes in the whole rod and the hourglass with room round them for the halves
/// to fall apart and the sand to be thrown out.
pub fn staff_zone(area: Pt) -> (Pt, Pt) {
    let len = staff_len(area);
    let head = staff_head(area);
    let top = head.1 - 70.0;
    let bottom = area.1;
    ((head.0, (top + bottom) / 2.0), (130.0_f32.min(area.0 / 2.0), (bottom - top) / 2.0))
}

pub const GRAINS: usize = 120;

/// A grain of sand thrown out of the hourglass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grain {
    pub pos: Pt,
    pub size: f32,
    pub alpha: f32,
    /// Which of the sand colours (0 to 3), light to dark.
    pub tone: u8,
}

/// Grain `i`, `t` seconds after the strike: thrown up and out of the hourglass at `head`, falling
/// back under gravity, and lying in the sand along `ground_y` (inside `0..width`).
pub fn grain(i: usize, t: f32, head: Pt, ground_y: f32, width: f32) -> Option<Grain> {
    let tau = t - 0.1;
    if tau < 0.0 {
        return None;
    }
    let i = i as u32;
    let angle = -PI * (0.08 + 0.84 * hash(i, 1));
    let speed = 140.0 + 330.0 * hash(i, 2);
    let (vx, vy) = (angle.cos() * speed, angle.sin() * speed);
    const G: f32 = 520.0;
    let rest = ground_y - hash(i, 4) * 10.0;
    // it lands where gravity brings it down to the sand, and stays there
    let drop = (head.1 - rest).min(0.0);
    let landing = (-vy + (vy * vy - 2.0 * G * drop).max(0.0).sqrt()) / G;
    let x = (head.0 + vx * tau.min(landing)).clamp(0.0, width);
    let flying = head.1 + vy * tau + 0.5 * G * tau * tau;
    Some(Grain {
        pos: (x, flying.min(rest)),
        size: 1.5 + 2.0 * hash(i, 3),
        alpha: (tau * 6.0).min(1.0) * (0.5 + 0.5 * hash(i, 5)),
        tone: (hash(i, 6) * 4.0) as u8 % 4,
    })
}

/// Height of a dune of the sand that piles up along the bottom: `layer` 0 is the back one, 1 the
/// front; `x` is the fraction of the width; `flood` comes from `strike`.
pub fn dune(layer: usize, x: f32, flood: f32, height: f32) -> f32 {
    let (base, wave, phase) = if layer == 0 { (0.12, 0.05, 0.8) } else { (0.06, 0.03, 2.4) };
    flood * height * (base + wave * (x * 10.7 + phase).sin()).max(0.0)
}

/// The thin streams of sand that run down from the hourglass to the ground once it has broken:
/// (top, bottom, opacity, width). They fade in with the flood.
pub fn streams(t: f32, area: Pt) -> [(Pt, Pt, f32, f32); 6] {
    let head = staff_head(area);
    let spread = [-0.46, -0.16, 0.05, 0.22, 0.4, -0.7];
    let widths = [3.0, 2.0, 3.0, 2.0, 3.0, 2.0];
    let alpha = 0.28 * smooth((t - 0.4) / 0.8);
    let mut out = [((0.0, 0.0), (0.0, 0.0), 0.0, 0.0); 6];
    for (k, s) in spread.iter().enumerate() {
        out[k] = (head, (head.0 + s * area.0 * 0.5, area.1), alpha, widths[k]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Pt = (560.0, 380.0);

    #[test]
    fn it_is_night_from_7pm_to_5am() {
        for h in [19, 20, 23, 0, 1, 4] {
            assert!(is_night(h), "{h}");
        }
        for h in [5, 6, 12, 17, 18] {
            assert!(!is_night(h), "{h}");
        }
        assert!(is_night(24 + 22), "an hour past 24 wraps");
    }

    #[test]
    fn the_moon_appears_at_night_or_when_pretending_but_not_once_the_staff_is_broken() {
        assert!(appears(21, false, false, AREA));
        assert!(!appears(12, false, false, AREA));
        assert!(appears(12, true, false, AREA));
        assert!(!appears(21, false, true, AREA));
        assert!(!appears(21, true, true, AREA));
        // a window too small for it has none
        assert!(!appears(21, false, false, (200.0, 380.0)));
        assert!(!appears(21, false, false, (560.0, 200.0)));
        assert!(appears(21, false, false, MIN_AREA));
    }

    #[test]
    fn the_staff_and_the_corner_fit_in_every_area_the_moon_is_shown_in() {
        for area in [MIN_AREA, (400.0, 300.0), AREA, (1200.0, 900.0), (900.0, 240.0)] {
            let c = corner(area);
            assert!(c.0 > 0.0 && c.0 < area.0 && c.1 == 0.0);
            let (head, base) = (staff_head(area), staff_base(area));
            assert_eq!(head.0, base.0);
            assert!(head.1 > 0.0 && head.1 < base.1 && base.1 == area.1, "{area:?}");
            assert!(staff_len(area) >= 70.0 && staff_len(area) <= 170.0);
            // the hourglass is well below where the moon comes out
            assert!(head.1 > c.1 + FULL_R + 20.0 || area.1 < 300.0, "{area:?}");
        }
    }

    #[test]
    fn it_starts_tucked_half_over_the_corner_and_follows_the_window() {
        let mut m = Moon::new();
        m.tick(0.0, AREA);
        assert_eq!(m.phase(), Phase::Tucked);
        assert_eq!((m.centre(), m.radius(), m.pull()), (corner(AREA), HALF_R, 0.0));
        assert_eq!(m.staff_reveal(AREA), 0.0);
        // the window gets wider: the moon stays in the corner
        let wide = (900.0, 380.0);
        m.tick(1.0, wide);
        assert_eq!(m.centre(), corner(wide));
        // only the corner can take hold of it
        assert!(m.hovered((corner(wide).0 + 10.0, 10.0), wide));
        assert!(!m.hovered((300.0, 200.0), wide));
        assert!(!m.press(1.0, (300.0, 200.0), wide));
        assert_eq!(m.phase(), Phase::Tucked);
    }

    #[test]
    fn pulling_it_out_is_slow_at_first_then_it_follows_the_pointer_and_grows_to_the_full_moon() {
        let a = corner(AREA);
        // a quarter of the way: the pointer is well ahead of the moon
        let (c, r, pull) = held_at((a.0 - 37.5, a.1 + 37.5), AREA);
        let pointer_d = dist((a.0 - 37.5, a.1 + 37.5), a);
        assert!(dist(c, a) < pointer_d * 0.5, "the moon lags: {} vs {}", dist(c, a), pointer_d);
        assert!(pull > 0.2 && pull < 0.5 && r > HALF_R && r < FULL_R);
        // fully out, it is exactly under the pointer and full size
        let p = (a.0 - 120.0, a.1 + 100.0);
        let (c, r, pull) = held_at(p, AREA);
        assert_eq!(pull, 1.0);
        assert!((c.0 - p.0).abs() < 0.01 && (c.1 - p.1).abs() < 0.01);
        assert!((r - FULL_R).abs() < 0.5, "{r}");
        // it never jumps: just before and after the full pull are close
        let before = held_at((a.0 - 119.0, a.1 + 99.2), AREA).0;
        assert!(dist(before, c) < 6.0, "{before:?} {c:?}");
        // at the corner it is the half moon
        assert_eq!(held_at(a, AREA), (a, HALF_R, 0.0));
        // it grows steadily as it is pulled
        let sizes: Vec<f32> = (0..=10).map(|k| held_at((a.0 - 15.0 * k as f32 * 0.7, a.1 + 15.0 * k as f32 * 0.7), AREA).1).collect();
        assert!(sizes.windows(2).all(|w| w[1] >= w[0] - 0.001), "{sizes:?}");
    }

    #[test]
    fn the_lower_it_is_dragged_the_smaller_it_gets_down_to_a_speck_at_the_staff() {
        let head = staff_head(AREA);
        let mut last = f32::MAX;
        for k in 0..=10 {
            let y = 110.0 + (head.1 - 110.0) * k as f32 / 10.0;
            let (_, r, pull) = held_at((head.0, y), AREA);
            assert_eq!(pull, 1.0);
            assert!(r <= last + 0.001, "bigger lower down at {y}: {r} > {last}");
            assert!(r >= MIN_R - 0.001 && r <= FULL_R + 0.001);
            last = r;
        }
        assert!((last - MIN_R).abs() < 0.5, "a speck at the staff: {last}");
        // dragged sideways at the same height, it keeps its size
        let a = held_at((200.0, 150.0), AREA).1;
        let b = held_at((300.0, 150.0), AREA).1;
        assert!((a - b).abs() < 1.0, "{a} {b}");
    }

    /// Takes hold of the moon, drags it along `path` (a point every 16 ms from `t0`), and lets go.
    fn drag_along(m: &mut Moon, t0: f64, path: &[Pt]) -> f64 {
        let a = corner(AREA);
        assert!(m.press(t0, (a.0, a.1 + 5.0), AREA));
        let mut t = t0;
        for p in path {
            t += 0.016;
            m.drag(t, *p, AREA);
            m.tick(t, AREA);
        }
        m.release(t, AREA);
        t
    }

    fn pull_out_path() -> Vec<Pt> {
        let a = corner(AREA);
        (1..=20).map(|k| (a.0 - 5.0 * k as f32, a.1 + 7.0 * k as f32)).collect()
    }

    #[test]
    fn let_go_slowly_it_floats_back_to_the_corner_and_tucks_in_again() {
        let mut m = Moon::new();
        let t = drag_along(&mut m, 0.0, &pull_out_path());
        assert_eq!(m.phase(), Phase::Drifting);
        // it starts where it was let go and is on its way home soon after
        m.tick(t, AREA);
        let start = m.centre();
        m.tick(t + 0.5, AREA);
        assert!(dist(m.centre(), corner(AREA)) < dist(start, corner(AREA)));
        assert!(m.radius() < FULL_R);
        // in about a second it is home
        m.tick(t + f64::from(DRIFT_SECS) + 0.01, AREA);
        assert_eq!(m.phase(), Phase::Tucked);
        assert_eq!((m.centre(), m.radius(), m.pull()), (corner(AREA), HALF_R, 0.0));
        // the staff disappears again as it goes
        assert_eq!(m.staff_reveal(AREA), 0.0);
    }

    #[test]
    fn a_drifting_moon_can_be_caught_again() {
        let mut m = Moon::new();
        let t = drag_along(&mut m, 0.0, &pull_out_path());
        m.tick(t + 0.2, AREA);
        let c = m.centre();
        assert!(m.hovered((c.0 + 3.0, c.1), AREA));
        assert!(m.press(t + 0.2, (c.0 + 3.0, c.1), AREA));
        assert_eq!(m.phase(), Phase::Held);
    }

    /// A pull out and then straight down, at `speed` points a second at the end.
    fn flick_path(speed: f32, dir: Pt) -> Vec<Pt> {
        let mut path = pull_out_path();
        let mut p = *path.last().unwrap();
        // down to a point above the staff first, slowly
        let head = staff_head(AREA);
        while p.1 < head.1 - 120.0 {
            p = (p.0 + (head.0 - p.0) * 0.1, p.1 + 8.0);
            path.push(p);
        }
        // then the flick: the last few samples at the given speed
        let step = speed * 0.016;
        for _ in 0..6 {
            p = (p.0 + dir.0 * step, p.1 + dir.1 * step);
            path.push(p);
        }
        path
    }

    fn towards_the_staff(from: Pt) -> Pt {
        let head = staff_head(AREA);
        let d = dist(from, head);
        ((head.0 - from.0) / d, (head.1 - from.1) / d)
    }

    #[test]
    fn a_fast_flick_at_the_staff_sends_the_moon_into_it() {
        let slow = flick_path(0.0, (0.0, 0.0));
        let from = *slow.last().unwrap();
        let dir = towards_the_staff(from);
        let step = 1500.0 * 0.016;
        let mut path = slow;
        for k in 1..=6 {
            path.push((from.0 + dir.0 * step * k as f32, from.1 + dir.1 * step * k as f32));
        }
        let mut m = Moon::new();
        let t = drag_along(&mut m, 0.0, &path);
        assert_eq!(m.phase(), Phase::Falling, "speed {:?}", m.velocity(t));
        // it flies at the staff, shrinking, and strikes it
        m.tick(t + 0.05, AREA);
        assert!(m.radius() < FULL_R && m.staff_reveal(AREA) == 1.0);
        let mut now = t;
        while m.phase() == Phase::Falling {
            now += 0.016;
            m.tick(now, AREA);
            assert!(now - t < 1.5, "it arrives");
        }
        assert_eq!(m.phase(), Phase::Struck);
        assert_eq!(m.centre(), staff_head(AREA));
        assert_eq!(m.struck_for(now), Some((now - m.struck_at) as f32));
        assert!(!m.finished(now));
        assert!(m.finished(now + f64::from(STRIKE_END) + 0.01));
    }

    #[test]
    fn a_flick_that_is_too_slow_or_wrongly_aimed_or_not_fully_out_does_not_count() {
        let slow = flick_path(0.0, (0.0, 0.0));
        let from = *slow.last().unwrap();
        let dir = towards_the_staff(from);
        let with = |speed: f32, dir: Pt| {
            let mut path = slow.clone();
            let step = speed * 0.016;
            for k in 1..=6 {
                path.push((from.0 + dir.0 * step * k as f32, from.1 + dir.1 * step * k as f32));
            }
            path
        };
        // too slow
        let mut m = Moon::new();
        drag_along(&mut m, 0.0, &with(100.0, dir));
        assert_eq!(m.phase(), Phase::Drifting);
        // fast, but upwards, away from it
        let mut m = Moon::new();
        drag_along(&mut m, 0.0, &with(1500.0, (0.0, -1.0)));
        assert_eq!(m.phase(), Phase::Drifting);
        // fast, but sideways
        let mut m = Moon::new();
        drag_along(&mut m, 0.0, &with(1500.0, (1.0, 0.0)));
        assert_eq!(m.phase(), Phase::Drifting);
        // fast and aimed, but let go before it was fully out
        let a = corner(AREA);
        let mut m = Moon::new();
        assert!(m.press(0.0, (a.0, a.1 + 5.0), AREA));
        for k in 1..=6 {
            m.drag(f64::from(k) * 0.016, (a.0 - 3.0 * k as f32, a.1 + 12.0 * k as f32), AREA);
        }
        m.release(0.1, AREA);
        assert_eq!(m.phase(), Phase::Drifting);
        // and the aimed one is fine
        let mut m = Moon::new();
        drag_along(&mut m, 0.0, &with(1500.0, dir));
        assert_eq!(m.phase(), Phase::Falling);
    }

    #[test]
    fn letting_go_a_little_early_with_a_push_towards_the_hourglass_still_strikes() {
        let head = staff_head(AREA);
        // a gentle push from within reach of it
        let path_to = |dist: f32, speed: f32| {
            let mut path = pull_out_path();
            let mut p = *path.last().unwrap();
            while p.1 < head.1 - dist {
                p = (p.0 + (head.0 - p.0) * 0.1, p.1 + 8.0);
                path.push(p);
            }
            // stop, then the last few samples at `speed` towards it
            let dir = towards_the_staff(p);
            for _ in 0..6 {
                path.push(p);
            }
            for k in 1..=6 {
                path.push((p.0 + dir.0 * speed * 0.016 * k as f32, p.1 + dir.1 * speed * 0.016 * k as f32));
            }
            path
        };
        let mut m = Moon::new();
        drag_along(&mut m, 0.0, &path_to(90.0, 300.0));
        assert_eq!(m.phase(), Phase::Falling, "a push from close by is enough");
        // from far away it takes a real flick
        let mut m = Moon::new();
        drag_along(&mut m, 0.0, &path_to(240.0, 300.0));
        assert_eq!(m.phase(), Phase::Drifting);
        // and the hourglass has to be the way it is pushed
        let mut m = Moon::new();
        drag_along(&mut m, 0.0, &path_to(90.0, 0.0));
        assert_eq!(m.phase(), Phase::Drifting, "no push, no strike");
    }

    #[test]
    fn dropping_it_right_on_the_hourglass_strikes_even_without_a_flick() {
        let mut m = Moon::new();
        let mut path = flick_path(0.0, (0.0, 0.0));
        let head = staff_head(AREA);
        // drag the rest of the way and stop on it
        for k in 1..=40 {
            let from = *path.last().unwrap();
            let _ = k;
            path.push((from.0 + (head.0 - from.0) * 0.2, from.1 + (head.1 - from.1) * 0.2));
        }
        for _ in 0..10 {
            path.push(head);
        }
        let t = drag_along(&mut m, 0.0, &path);
        assert_eq!(m.phase(), Phase::Falling);
        m.tick(t + 0.2, AREA);
        assert_eq!(m.phase(), Phase::Struck);
    }

    #[test]
    fn the_pointer_speed_is_taken_from_the_last_moments_only() {
        let mut m = Moon::new();
        let a = corner(AREA);
        assert!(m.press(0.0, (a.0, a.1 + 5.0), AREA));
        assert_eq!(m.velocity(0.0), (0.0, 0.0), "one sample is no speed");
        m.drag(0.45, (a.0 - 100.0, a.1 + 100.0), AREA);
        m.drag(0.6, (a.0 - 100.0, a.1 + 100.0), AREA);
        m.drag(0.62, (a.0 - 100.0, a.1 + 140.0), AREA);
        let v = m.velocity(0.62);
        assert!((v.1 - 2000.0).abs() < 1.0 && v.0.abs() < 0.01, "{v:?}");
        // a long wait afterwards, and it is standing still
        assert_eq!(m.velocity(2.0), (0.0, 0.0));
    }

    #[test]
    fn the_staff_only_shows_once_the_moon_is_dragged_down_a_good_way() {
        let mut m = Moon::new();
        let a = corner(AREA);
        assert!(m.press(0.0, (a.0, a.1 + 5.0), AREA));
        // fully out but still high up: no staff yet
        m.drag(0.1, (a.0 - 120.0, a.1 + 100.0), AREA);
        assert_eq!(m.pull(), 1.0);
        assert_eq!(m.staff_reveal(AREA), 0.0);
        // then down: more and more of it, all of it near the hourglass
        let head = staff_head(AREA);
        let mut last = -1.0;
        for k in 0..=20 {
            let y = 100.0 + (head.1 - 100.0) * k as f32 / 20.0;
            m.drag(0.2 + 0.016 * f64::from(k), (head.0, y), AREA);
            let s = m.staff_reveal(AREA);
            assert!(s >= last - 0.0001 && (0.0..=1.0).contains(&s), "{s} at {y}");
            last = s;
        }
        assert_eq!(last, 1.0);
        // lifting it back up takes the staff away again
        m.drag(1.0, (head.0, 100.0), AREA);
        assert_eq!(m.staff_reveal(AREA), 0.0);
    }

    #[test]
    fn a_flick_at_a_staff_that_has_not_appeared_yet_does_not_count() {
        // pulled out, flung downwards while the moon is still high: nothing there to hit
        let a = corner(AREA);
        let mut m = Moon::new();
        assert!(m.press(0.0, (a.0, a.1 + 5.0), AREA));
        let start = (a.0 - 120.0, a.1 + 100.0);
        let to = towards_the_staff(start);
        m.drag(0.1, start, AREA);
        m.drag(0.116, (start.0 + to.0 * 14.0, start.1 + to.1 * 14.0), AREA);
        assert!(m.velocity(0.116).1 > FLICK_SPEED * 0.9, "{:?}", m.velocity(0.116));
        assert!(m.staff_reveal(AREA) < 0.5);
        m.release(0.116, AREA);
        assert_eq!(m.phase(), Phase::Drifting);
    }

    #[test]
    fn everything_glitches_from_the_moment_the_moon_is_grabbed_and_worsens_the_further_it_goes() {
        let mut m = Moon::new();
        m.tick(0.0, AREA);
        assert_eq!(m.glitch_level(0.0, AREA), 0.0, "quiet while it sits in the corner");
        let a = corner(AREA);
        assert!(m.press(0.0, (a.0, a.1 + 5.0), AREA));
        let grabbed = m.glitch_level(0.0, AREA);
        assert!(grabbed > 0.05, "{grabbed}");
        // pulled out, it is worse; dragged down to the staff, worse still
        m.drag(0.1, (a.0 - 120.0, a.1 + 100.0), AREA);
        let out = m.glitch_level(0.1, AREA);
        assert!(out > grabbed + 0.3 && out < 1.0, "{out}");
        let head = staff_head(AREA);
        let mut last = out;
        for k in 1..=20 {
            let y = 100.0 + (head.1 - 100.0) * k as f32 / 20.0;
            m.drag(0.2 + 0.016 * f64::from(k), (head.0, y), AREA);
            let g = m.glitch_level(0.2, AREA);
            assert!(g >= last - 0.2 && (0.0..=1.0).contains(&g));
            last = g;
        }
        assert!(last > 0.95, "{last}");
        // let go (up high, where the staff is not out) and it floats home: the glitching dies with it
        m.drag(0.9, (head.0, 120.0), AREA);
        m.release(1.0, AREA);
        assert_eq!(m.phase(), Phase::Drifting);
        m.tick(1.0 + f64::from(DRIFT_SECS) + 0.01, AREA);
        assert_eq!(m.glitch_level(3.0, AREA), 0.0);
    }

    #[test]
    fn the_glitching_is_worst_in_the_flight_and_then_slowly_stops_after_the_strike() {
        let mut m = Moon::new();
        let head = staff_head(AREA);
        let a = corner(AREA);
        assert!(m.press(0.0, (a.0, a.1 + 5.0), AREA));
        m.drag(0.1, (a.0 - 120.0, a.1 + 100.0), AREA);
        m.drag(0.2, (head.0, head.1 - 4.0), AREA);
        m.release(0.2, AREA);
        assert_eq!(m.phase(), Phase::Falling);
        assert_eq!(m.glitch_level(0.21, AREA), 1.0);
        m.tick(0.6, AREA);
        assert_eq!(m.phase(), Phase::Struck);
        let hit = m.struck_at;
        assert_eq!(m.glitch_level(hit, AREA), 1.0);
        // slowly less and less, never more, and gone by the time the seams are stitched
        let mut last = 1.0;
        for k in 0..=60 {
            let g = m.glitch_level(hit + f64::from(k) * 0.1, AREA);
            assert!(g <= last + 1e-6 && g >= 0.0);
            last = g;
        }
        assert_eq!(m.glitch_level(hit + 4.5, AREA), 0.0);
        let mid = m.glitch_level(hit + 2.0, AREA);
        assert!(mid > 0.3 && mid < 0.7, "{mid}");
    }

    #[test]
    fn the_staff_is_kept_clear_while_it_breaks_and_the_break_is_slow_enough_to_see() {
        assert_eq!(staff_clear(-1.0), 0.0);
        assert_eq!(staff_clear(0.0), 1.0);
        assert_eq!(staff_clear(2.4), 1.0);
        assert_eq!(staff_clear(3.6), 0.0);
        let mut last = 1.0;
        for k in 0..=50 {
            let c = staff_clear(k as f32 * 0.1);
            assert!(c <= last + 1e-6 && (0.0..=1.0).contains(&c));
            last = c;
        }
        // the break happens inside the clear time, and takes over a second
        assert!(strike(0.15).split < 0.01 && strike(0.5).split < 0.4 && strike(1.0).split > 0.4);
        assert!(strike(1.5).split == 1.0 && staff_clear(1.5) == 1.0);
        // the patch holds the hourglass, the whole rod and room either side, in every size of area
        for area in [(460.0, 380.0), (900.0, 700.0), (1400.0, 1000.0)] {
            let ((cx, cy), (rx, ry)) = staff_zone(area);
            let (head, base) = (staff_head(area), staff_base(area));
            assert!(head.0 >= cx - rx && head.0 <= cx + rx && base.0 >= cx - rx && base.0 <= cx + rx);
            assert!(head.1 - 60.0 >= cy - ry && base.1 <= cy + ry + 0.5, "{area:?}");
            assert!(rx >= 100.0 && rx <= area.0 / 2.0);
        }
    }

    #[test]
    fn a_click_or_key_skips_ahead_to_him_but_not_in_the_first_second_and_not_twice() {
        let mut m = Moon::new();
        let head = staff_head(AREA);
        let a = corner(AREA);
        assert!(m.press(0.0, (a.0, a.1 + 5.0), AREA));
        m.drag(0.1, (head.0, head.1 - 4.0), AREA);
        m.release(0.1, AREA);
        m.tick(1.0, AREA);
        let hit = m.struck_at;
        let now = hit + 0.5;
        m.skip(now);
        assert!((m.struck_for(now).unwrap() - 0.5).abs() < 1e-3, "too early to skip");
        let now = hit + 2.0;
        m.skip(now);
        assert!((m.struck_for(now).unwrap() - SKIP_TO).abs() < 1e-3);
        assert!(!m.finished(now));
        // he stands there a moment, then Game 3
        assert!(m.finished(now + 1.5));
        // skipping again does not jump any further
        m.skip(now + 0.2);
        assert!((m.struck_for(now + 0.2).unwrap() - (SKIP_TO + 0.2)).abs() < 1e-3);
        // skipping a scene that has not started does nothing
        let mut idle = Moon::new();
        idle.skip(5.0);
        assert_eq!(idle.struck_for(5.0), None);
    }

    #[test]
    fn the_strike_flashes_shakes_and_settles_and_the_sands_come_out() {
        let s0 = strike(0.0);
        assert_eq!((s0.flash, s0.split, s0.flood, s0.trails), (0.0, 0.0, 0.0, 0.0));
        assert!(s0.rings.iter().all(|r| r.1 == 0.0));
        // up in a flash, then gone
        let peak = strike(0.05);
        assert!((peak.flash - 1.0).abs() < 1e-6);
        assert!(strike(0.6).flash < 0.1 && strike(2.0).flash < 0.001);
        // the rings spread one after another and fade
        let early = strike(0.15);
        let later = strike(0.4);
        assert!(early.rings[0].0 > 18.0 && early.rings[2].1 == 0.0 || early.rings[2].0 == 0.0);
        assert!(later.rings[0].0 > early.rings[0].0 && later.rings[2].0 > 0.0);
        assert!(strike(1.3).rings.iter().all(|r| r.1 == 0.0));
        // the page shakes at first and is still afterwards
        let amp = |t: f32| strike(t).shake.0.abs().max(strike(t).shake.1.abs());
        assert!((0..40).any(|k| amp(0.02 + k as f32 * 0.01) > 3.0));
        assert_eq!(strike(1.3).shake, (0.0, 0.0));
        // everything runs from 0 to 1 and never backwards
        let mut last = (0.0, 0.0, 0.0);
        for k in 0..=100 {
            let s = strike(STRIKE_END * k as f32 / 100.0);
            let now = (s.split, s.flood, s.trails);
            assert!(now.0 >= last.0 && now.1 >= last.1 && now.2 >= last.2);
            last = now;
        }
        let end = strike(STRIKE_END);
        assert_eq!((end.split, end.flood, end.trails), (1.0, 1.0, 1.0));
        // the sand has all come out well before the monarch has finished appearing
        assert_eq!(strike(5.0).flood, 1.0);
    }

    #[test]
    fn the_sand_is_thrown_up_falls_and_lies_in_the_dunes_inside_the_window() {
        let head = staff_head(AREA);
        let ground = AREA.1 - 14.0;
        assert!((0..GRAINS).all(|i| grain(i, 0.05, head, ground, AREA.0).is_none()), "nothing before the strike");
        let mut rose = 0;
        for i in 0..GRAINS {
            let mut highest = f32::MAX;
            for k in 0..=60 {
                let t = 0.1 + k as f32 * 0.05;
                let g = grain(i, t, head, ground, AREA.0).expect("grain");
                assert!(g.pos.0 >= 0.0 && g.pos.0 <= AREA.0, "grain {i} at {t}: {:?}", g.pos);
                assert!(g.pos.1 <= ground + 0.001, "never below the ground");
                assert!(g.size >= 1.5 && g.size <= 3.5 && g.alpha >= 0.0 && g.alpha <= 1.0 && g.tone < 4);
                highest = highest.min(g.pos.1);
            }
            if highest < head.1 - 5.0 {
                rose += 1;
            }
            // in the end it lies still in the sand
            let (a, b) = (grain(i, 3.4, head, ground, AREA.0).unwrap(), grain(i, 4.4, head, ground, AREA.0).unwrap());
            assert_eq!(a.pos, b.pos, "grain {i} is at rest");
            assert!(a.pos.1 >= ground - 10.0);
        }
        assert!(rose > GRAINS / 2, "most of them are thrown up first: {rose}");
        // the same every time
        assert_eq!(grain(7, 0.8, head, ground, AREA.0), grain(7, 0.8, head, ground, AREA.0));
    }

    #[test]
    fn the_dunes_rise_with_the_flood_and_stay_low_and_the_streams_run_from_the_hourglass_down() {
        for layer in 0..2 {
            for k in 0..=50 {
                let x = k as f32 / 50.0;
                assert_eq!(dune(layer, x, 0.0, AREA.1), 0.0);
                let h = dune(layer, x, 1.0, AREA.1);
                assert!(h >= 0.0 && h <= AREA.1 * 0.2, "{h}");
                assert!(dune(layer, x, 0.5, AREA.1) <= h + 1e-4);
            }
        }
        let st = streams(3.0, AREA);
        for (top, bottom, alpha, width) in st {
            assert_eq!(top, staff_head(AREA));
            assert_eq!(bottom.1, AREA.1);
            assert!(bottom.0 >= 0.0 && bottom.0 <= AREA.0);
            assert!(alpha > 0.0 && alpha <= 0.3 && width >= 2.0);
        }
        assert!(streams(0.0, AREA).iter().all(|s| s.2 == 0.0));
    }

    #[test]
    fn the_colour_fringe_is_two_points_and_jumps_wider_now_and_then() {
        let fringes: Vec<f32> = (0..2000).map(|k| fringe(k as f32 / 20.0)).collect();
        assert!(fringes.iter().all(|f| *f == 2.0 || *f == 6.0));
        let wide = fringes.iter().filter(|f| **f == 6.0).count();
        assert!(wide > 10 && wide < 600, "{wide}");
    }
}
