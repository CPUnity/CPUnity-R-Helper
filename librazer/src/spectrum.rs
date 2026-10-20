//! The Spectrum effect on the cooling pad, made the way Razer Chroma's own Spectrum makes it
//! (worked out from a USB capture of Chroma driving the pad).
//!
//! Every light of the strip shows the same color, and the color slowly goes round the rainbow: a
//! whole turn takes about 42 seconds. The capture follows the color for its first 14 seconds
//! (from a pink through red and orange to a yellow-green); those colors are used exactly as
//! captured. The rest of the way round (green, blue, violet and back to the pink) is a plain
//! rainbow that starts where the captured part ends and joins its start again, at the same pace.
//!
//! The Spectrum effect, made the way Razer Chroma's own Spectrum makes it (worked out from USB
//! captures of Chroma driving the keyboard and the cooling pad, which show the same colors).
//!
//! Every light shows the same color and the color goes round the rainbow: a whole turn takes about
//! 37.7 seconds, and the color rests on one blue for about 5 seconds of it. The color table below
//! is that turn as captured (straight lines between the listed points).
//!
//! This is pure arithmetic (no device, no clock): `advance` takes the seconds that have passed.
use crate::fire::PAD_LEDS;

/// Seconds for the color to go once round.
pub const CYCLE_SECS: f32 = 37.72;

/// The color at a few moments of the turn; in between, it moves in a straight line.
#[rustfmt::skip]
const CAPTURED: [(f32, [u8; 3]); 30] = [
    (0.00, [255, 24, 14]),
    (3.06, [255, 114, 1]),
    (3.58, [254, 129, 0]),
    (6.14, [239, 255, 0]),
    (8.62, [223, 255, 0]),
    (8.80, [223, 255, 0]),
    (8.82, [222, 255, 0]),
    (9.36, [175, 255, 0]),
    (9.62, [154, 255, 0]),
    (11.36, [4, 255, 0]),
    (11.42, [0, 255, 0]),
    (11.46, [0, 255, 3]),
    (11.50, [0, 255, 10]),
    (11.74, [0, 255, 68]),
    (12.34, [0, 255, 208]),
    (12.50, [0, 255, 248]),
    (12.54, [0, 255, 255]),
    (12.82, [1, 246, 255]),
    (14.52, [10, 181, 255]),
    (14.86, [12, 170, 255]),
    (16.96, [23, 91, 255]),
    (17.06, [24, 88, 255]),
    (22.06, [25, 87, 255]),
    (23.72, [69, 75, 255]),
    (30.64, [255, 24, 255]),
    (30.70, [255, 24, 255]),
    (32.48, [255, 21, 190]),
    (37.34, [255, 16, 18]),
    (37.48, [255, 17, 15]),
    (37.72, [255, 24, 14])
];

fn lerp(a: [u8; 3], b: [u8; 3], x: f32) -> [u8; 3] {
    let x = x.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| (f32::from(a[i]) + (f32::from(b[i]) - f32::from(a[i])) * x).round() as u8)
}

/// The color `secs` seconds into the cycle (any number of seconds: it wraps).
pub fn color_at(secs: f32) -> [u8; 3] {
    let x = secs.rem_euclid(CYCLE_SECS);
    let i = CAPTURED.iter().rposition(|(t, _)| *t <= x).unwrap_or(0);
    let (t0, c0) = CAPTURED[i];
    match CAPTURED.get(i + 1) {
        Some((t1, c1)) if t1 > &t0 => lerp(c0, *c1, (x - t0) / (t1 - t0)),
        _ => c0,
    }
}

/// A running Spectrum: how far round the rainbow it is.
#[derive(Debug, Clone, Copy, Default)]
pub struct Spectrum {
    secs: f32,
}

impl Spectrum {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lets `secs` seconds go by.
    pub fn advance(&mut self, secs: f32) {
        if secs.is_finite() && secs > 0.0 {
            self.secs = (self.secs + secs.min(60.0)).rem_euclid(CYCLE_SECS);
        }
    }

    pub fn color(&self) -> [u8; 3] {
        color_at(self.secs)
    }

    /// The pad's strip: every light the same color.
    pub fn pad_grid(&self) -> [[u8; 3]; PAD_LEDS] {
        [self.color(); PAD_LEDS]
    }

    /// The keyboard: every key that has a light shows the color; the cells with no key stay dark.
    pub fn grid(&self) -> [[[u8; 3]; 15]; 6] {
        let c = self.color();
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
    fn the_captured_colors_come_out_as_captured() {
        for (t, color) in CAPTURED.iter() {
            assert_eq!(color_at(*t), *color, "at {} s", t);
        }
        assert_eq!(color_at(0.0), [255, 24, 14], "the cycle starts on a red");
        // in the middle of the rest on the blue, it does not move
        let (a, b) = (color_at(18.0), color_at(21.0));
        assert!((0..3).all(|i| (i32::from(a[i]) - i32::from(b[i])).abs() <= 1), "{:?} {:?}", a, b);
        // the pad capture started 33.18 s into this cycle on this pink
        let pink = color_at(33.18);
        assert!(pink[0] == 255 && (pink[2] as i32 - 165).abs() <= 3, "{:?}", pink);
    }

    #[test]
    fn the_color_goes_all_the_way_round_without_a_jump() {
        let mut worst = 0i32;
        let mut previous = color_at(0.0);
        let (mut blue, mut green) = (false, false);
        let mut x = 0.0;
        while x < CYCLE_SECS * 2.0 {
            x += 0.04;
            let now = color_at(x);
            for i in 0..3 {
                worst = worst.max((i32::from(now[i]) - i32::from(previous[i])).abs());
            }
            blue |= now[2] == 255 && now[0] < 60 && now[1] < 100;
            green |= now[1] == 255 && now[0] < 20 && now[2] < 20;
            previous = now;
        }
        assert!(worst <= 14, "biggest step {}", worst);
        assert!(blue && green, "the rainbow has blue and green in it");
        let (a, b) = (color_at(0.0), color_at(CYCLE_SECS - 0.001));
        for i in 0..3 {
            assert!((i32::from(a[i]) - i32::from(b[i])).abs() <= 3, "{:?} against {:?}", a, b);
        }
    }

    #[test]
    fn every_light_shows_the_same_color_and_time_moves_it_on() {
        let mut s = Spectrum::new();
        let first = s.pad_grid();
        assert!(first.iter().all(|led| *led == first[0]));
        s.advance(5.0);
        assert_ne!(s.color(), first[0]);
        assert_eq!(s.color(), color_at(5.0));
        s.advance(f32::NAN);
        s.advance(-2.0);
        assert_eq!(s.color(), color_at(5.0));
        s.advance(CYCLE_SECS);
        assert_eq!(s.color(), color_at(5.0));
    }

    #[test]
    fn the_keyboard_lights_only_its_keys() {
        let g = Spectrum::new().grid();
        assert_eq!(g[0][0], [255, 24, 14]);
        assert_eq!(g[1][13], [0, 0, 0], "a cell with no key stays dark");
        let lit = g.iter().flatten().filter(|c| **c != [0, 0, 0]).count();
        assert_eq!(lit, 90 - 10);
    }
}
