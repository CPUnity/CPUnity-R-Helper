//! The Color Dipp effect: the whole light goes from one picked color to another through every hue
//! in between, rests a moment on it, and goes back the same way. Red to violet, for example, passes
//! orange, yellow, green and blue on the way (the hue is moved in a straight line along the rainbow,
//! never round the other side of the wheel).
//!
//! This one is the app's own (Razer Chroma has no such effect). Like the others it is pure
//! arithmetic: `advance` takes the seconds that have passed, and the drawing code asks for a grid.
use crate::fire::PAD_LEDS;

/// How fast the hue moves, in degrees a second (Chroma's own Spectrum goes about 10, so this is
/// about three times as fast).
pub const DEGREES_PER_SEC: f32 = 30.0;
/// The shortest time one way takes, so that two similar colors do not flicker back and forth.
pub const MIN_LEG_SECS: f32 = 1.5;
/// The rest at each end.
pub const HOLD_SECS: f32 = 3.0;
/// Below this saturation a color has no hue worth following (white, gray).
const GRAY: f32 = 0.05;

/// The default pair: red and violet.
pub const DEFAULT_A: [u8; 3] = [255, 0, 0];
pub const DEFAULT_B: [u8; 3] = [136, 0, 255];

fn to_hsv(c: [u8; 3]) -> (f32, f32, f32) {
    let [r, g, b] = c.map(|v| f32::from(v) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if max == 0.0 { 0.0 } else { d / max }, max)
}

fn from_hsv(h: f32, s: f32, v: f32) -> [u8; 3] {
    let x = (h / 60.0).rem_euclid(6.0);
    let f = x.fract();
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    let (r, g, b) = match x as u32 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [r, g, b].map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Both colors as (hue, saturation, value); a color with no hue (white, gray, black) takes the
/// other one's, so the sweep is a plain fade of brightness and saturation.
fn ends(a: [u8; 3], b: [u8; 3]) -> ((f32, f32, f32), (f32, f32, f32)) {
    let (mut ha, sa, va) = to_hsv(a);
    let (mut hb, sb, vb) = to_hsv(b);
    match (sa < GRAY || va == 0.0, sb < GRAY || vb == 0.0) {
        (true, false) => ha = hb,
        (false, true) => hb = ha,
        _ => {}
    }
    ((ha, sa, va), (hb, sb, vb))
}

/// Seconds one way takes for this pair.
pub fn leg_secs(a: [u8; 3], b: [u8; 3]) -> f32 {
    let ((ha, ..), (hb, ..)) = ends(a, b);
    ((hb - ha).abs() / DEGREES_PER_SEC).max(MIN_LEG_SECS)
}

/// Seconds for the whole there-and-back.
pub fn period(a: [u8; 3], b: [u8; 3]) -> f32 {
    2.0 * (leg_secs(a, b) + HOLD_SECS)
}

/// The color `secs` seconds into the there-and-back (any number: it wraps).
pub fn color_at(a: [u8; 3], b: [u8; 3], secs: f32) -> [u8; 3] {
    let leg = leg_secs(a, b);
    let p = secs.rem_euclid(2.0 * (leg + HOLD_SECS));
    // how far from `a` to `b` (0..1)
    let u = if p < HOLD_SECS {
        0.0
    } else if p < HOLD_SECS + leg {
        (p - HOLD_SECS) / leg
    } else if p < 2.0 * HOLD_SECS + leg {
        1.0
    } else {
        1.0 - (p - 2.0 * HOLD_SECS - leg) / leg
    };
    blend(a, b, u)
}

/// The color `u` of the way from `a` to `b` (0.0 is `a`, 1.0 is `b`), through every hue between
/// them, in a straight line along the rainbow.
pub fn blend(a: [u8; 3], b: [u8; 3], u: f32) -> [u8; 3] {
    if u <= 0.0 {
        return a;
    }
    if u >= 1.0 {
        return b;
    }
    let ((ha, sa, va), (hb, sb, vb)) = ends(a, b);
    from_hsv(ha + (hb - ha) * u, sa + (sb - sa) * u, va + (vb - va) * u)
}

/// A running Color Dipp: how long it has been going.
#[derive(Debug, Clone, Copy, Default)]
pub struct Dipp {
    secs: f32,
}

impl Dipp {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lets `secs` seconds go by.
    pub fn advance(&mut self, secs: f32) {
        if secs.is_finite() && secs > 0.0 {
            self.secs = (self.secs + secs.min(60.0)).rem_euclid(3600.0);
        }
    }

    pub fn color(&self, a: [u8; 3], b: [u8; 3]) -> [u8; 3] {
        color_at(a, b, self.secs)
    }

    /// The pad's strip: every light the same color.
    pub fn pad_grid(&self, a: [u8; 3], b: [u8; 3]) -> [[u8; 3]; PAD_LEDS] {
        [self.color(a, b); PAD_LEDS]
    }

    /// The keyboard: every key that has a light shows the color; the cells with no key stay dark.
    pub fn grid(&self, a: [u8; 3], b: [u8; 3]) -> [[[u8; 3]; 15]; 6] {
        let c = self.color(a, b);
        let mut g = [[[0u8; 3]; 15]; 6];
        for (r, row) in g.iter_mut().enumerate() {
            for (k, cell) in row.iter_mut().enumerate() {
                if crate::tidal::has_light(r, k) {
                    *cell = c;
                }
            }
        }
        g
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_starts_on_the_first_color_and_rests_on_each_end() {
        let (a, b) = ([255, 0, 0], [136, 0, 255]);
        assert_eq!(color_at(a, b, 0.0), a);
        assert_eq!(color_at(a, b, HOLD_SECS * 0.5), a);
        let leg = leg_secs(a, b);
        assert_eq!(color_at(a, b, HOLD_SECS + leg + 0.1), b);
        // and it is back on the first color after the whole there-and-back
        assert_eq!(color_at(a, b, period(a, b)), a);
        assert_eq!(color_at(a, b, period(a, b) * 3.0 + 0.1), a);
    }

    #[test]
    fn red_to_violet_goes_through_every_hue_in_between() {
        let (a, b) = ([255, 0, 0], [136, 0, 255]);
        let leg = leg_secs(a, b);
        let (mut yellow, mut green, mut cyan, mut blue) = (false, false, false, false);
        let mut orange_first = None;
        let mut x = HOLD_SECS;
        while x < HOLD_SECS + leg {
            let c = color_at(a, b, x);
            let (h, ..) = to_hsv(c);
            yellow |= (h - 60.0).abs() < 4.0;
            green |= (h - 120.0).abs() < 4.0;
            cyan |= (h - 180.0).abs() < 4.0;
            blue |= (h - 240.0).abs() < 4.0;
            if orange_first.is_none() && h > 20.0 {
                orange_first = Some(h);
            }
            // the hue only ever goes up on the way there (never round the magenta side)
            assert!(h < 275.0, "{:?} at {}", c, x);
            x += 0.05;
        }
        assert!(yellow && green && cyan && blue);
        // no jumps
        let mut worst = 0i32;
        let mut prev = color_at(a, b, 0.0);
        let mut x = 0.0;
        while x < period(a, b) * 2.0 {
            x += 0.04;
            let c = color_at(a, b, x);
            for i in 0..3 {
                worst = worst.max((i32::from(c[i]) - i32::from(prev[i])).abs());
            }
            prev = c;
        }
        assert!(worst <= 8, "biggest step {}", worst);
    }

    #[test]
    fn the_way_back_retraces_the_way_there() {
        let (a, b) = ([255, 0, 0], [136, 0, 255]);
        let leg = leg_secs(a, b);
        let there = color_at(a, b, HOLD_SECS + leg * 0.3);
        let back = color_at(a, b, 2.0 * HOLD_SECS + leg + leg * 0.7);
        for i in 0..3 {
            assert!(
                (i32::from(there[i]) - i32::from(back[i])).abs() <= 1,
                "{:?} {:?}",
                there,
                back
            );
        }
    }

    #[test]
    fn close_colors_still_take_their_time_and_gray_ends_just_fade() {
        let a = [255, 0, 0];
        let b = [255, 40, 0];
        assert_eq!(leg_secs(a, b), MIN_LEG_SECS);
        // white to red: no hue wandering, the red stays red while the white fills in
        let mid = color_at([255, 255, 255], [255, 0, 0], HOLD_SECS + MIN_LEG_SECS * 0.5);
        assert_eq!(mid[0], 255);
        assert!(mid[1] == mid[2] && mid[1] > 100 && mid[1] < 160, "{:?}", mid);
        // the same color twice never moves
        assert_eq!(color_at(a, a, 1.7), a);
    }

    #[test]
    fn the_light_covers_the_keys_and_the_strip_and_time_moves_it() {
        let (a, b) = (DEFAULT_A, DEFAULT_B);
        let mut d = Dipp::new();
        let pad = d.pad_grid(a, b);
        assert!(pad.iter().all(|led| *led == a));
        let keys = d.grid(a, b);
        assert_eq!(keys[0][0], a);
        assert_eq!(keys[1][13], [0, 0, 0]);
        d.advance(10.0);
        assert_eq!(d.color(a, b), color_at(a, b, 10.0));
        d.advance(f32::NAN);
        d.advance(-1.0);
        assert_eq!(d.color(a, b), color_at(a, b, 10.0));
    }
}
