//! The Tidal keyboard effect, made the way Razer Chroma's own Tidal makes it (worked out from a
//! USB capture of Chroma driving a Blade 16 keyboard).
//!
//! Every key runs the same slow cycle of 125.33 ticks (a tick is 1/25 s, so about 5 seconds):
//! its first color rises and falls as a triangle (about 1.4 s), the key rests for a bit over a
//! second, then the second color does the same, and it rests again. The two colors never show on
//! a key at the same time. What makes it a wave is that each key starts its cycle a little later
//! than its neighbour: the starts run out from a diagonal line across the keyboard (about
//! 4.5 ticks per key), so the colors jet out from that line. Reversed, they come in towards it.
//!
//! This is pure arithmetic: `advance` takes the seconds that have passed and `grid` is the
//! picture to paint. The keyboard animation worker (in the app) does the painting.
use crate::keymap::{KeyGrid, COLS, ROWS};

/// Chroma's two colors for Tidal in the capture.
pub const DEFAULT_COLOR_A: [u8; 3] = [0, 255, 0];
pub const DEFAULT_COLOR_B: [u8; 3] = [0, 0, 255];

/// Ticks per second (Chroma sends a frame per tick).
pub const TICKS_PER_SEC: f32 = 25.0;
/// One whole cycle, in ticks. Each color shows once per cycle, half a cycle apart.
const PERIOD: f32 = 125.33;
const HALF: f32 = PERIOD / 2.0;

/// The tick at which each key's first color starts rising, relative to the key at the top right
/// of the main block (row 0, column 12), as captured. Cells with no LED are 0 (see `LIT`).
#[rustfmt::skip]
const START: [[i16; COLS]; ROWS] = [
    [-25, -29, -34, -39, -37, -33, -28, -23, -19, -14, -9, -5, 0, 5, 9],
    [-20, -25, -30, -35, -39, -37, -32, -28, -23, -18, -14, -9, -4, 0, 5],
    [-16, -21, -26, -30, -35, -40, -37, -32, -27, -23, -18, -13, -8, 0, 1],
    [-12, -17, -21, -26, -31, -35, -40, -36, -32, -27, -22, -17, 0, 0, -3],
    [-8, 0, -17, -22, -26, -31, -36, -41, -36, -31, -26, -22, 0, 0, -8],
    [-3, -8, -13, 0, -17, -32, 0, 0, -35, -31, -26, -21, -17, -12, -21],
];

/// Which cells have a light (the same ones that are dark in Chroma's Fire).
#[rustfmt::skip]
const LIT: [[bool; COLS]; ROWS] = [
    [true, true, true, true, true, true, true, true, true, true, true, true, true, true, true],
    [true, true, true, true, true, true, true, true, true, true, true, true, true, false, true],
    [true, true, true, true, true, true, true, true, true, true, true, true, true, false, true],
    [true, true, true, true, true, true, true, true, true, true, true, true, false, false, true],
    [true, false, true, true, true, true, true, true, true, true, true, true, false, false, true],
    [true, true, true, false, true, true, false, false, true, true, true, true, true, true, true],
];

/// Whether the key at `row`, `col` of the matrix has a light.
pub fn has_light(row: usize, col: usize) -> bool {
    LIT.get(row).and_then(|r| r.get(col)).copied().unwrap_or(false)
}

/// The first color's brightness (0..1) `x` ticks into its turn: 17 equal steps up, a pause of two
/// ticks at the top, 17 equal steps down.
fn first_level(x: f32) -> f32 {
    if !(0.0..=37.0).contains(&x) {
        return 0.0;
    }
    (x.min(37.0 - x) / 18.0).clamp(0.0, 1.0)
}

/// The second color's brightness `x` ticks into its turn (it climbs a touch faster).
fn second_level(x: f32) -> f32 {
    if !(0.0..=36.0).contains(&x) {
        return 0.0;
    }
    ((x / 17.0).min((36.0 - x) / 18.0)).clamp(0.0, 1.0)
}

fn scale(color: [u8; 3], level: f32) -> [u8; 3] {
    // (+ a hair so that full brightness stays exact)
    color.map(|c| (f32::from(c) * level + 1e-3) as u8)
}

/// The two colors' levels for a key whose cycle starts at `start` ticks, at time `ticks`.
fn levels(ticks: f32, start: f32, inward: bool) -> (f32, f32) {
    let start = if inward { -start } else { start };
    let x = (ticks - start).rem_euclid(PERIOD);
    let xb = (x - HALF).rem_euclid(PERIOD);
    (first_level(x), second_level(xb))
}

/// The keyboard picture `ticks` ticks into the animation. `inward`: the colors come in towards
/// the diagonal instead of jetting out from it.
pub fn grid_at(ticks: f32, a: [u8; 3], b: [u8; 3], inward: bool) -> KeyGrid {
    let mut grid = [[[0u8; 3]; COLS]; ROWS];
    for (r, row) in grid.iter_mut().enumerate() {
        for (c, cell) in row.iter_mut().enumerate() {
            if !LIT[r][c] {
                continue;
            }
            let (la, lb) = levels(ticks, f32::from(START[r][c]), inward);
            let (ca, cb) = (scale(a, la), scale(b, lb));
            *cell = [ca[0].saturating_add(cb[0]), ca[1].saturating_add(cb[1]), ca[2].saturating_add(cb[2])];
        }
    }
    grid
}

/// Lights on the cooling pad's strip.
pub const PAD_LEDS: usize = crate::fire::PAD_LEDS;

/// How many ticks later each light, going out from the middle of the strip, starts its cycle
/// (the pad's capture: about 0.27 s per light, mirrored on both sides of the middle).
const PAD_STEP: f32 = 6.65;
/// The light the waves start from (the strip's middle) and where its cycle starts.
const PAD_MIDDLE: usize = 9;
const PAD_FIRST_START: f32 = -40.0;

/// The cooling pad's strip `ticks` ticks into the animation: the same two-color cycle as the
/// keyboard, running out from the middle of the strip to both ends (or, `inward`, from the ends
/// to the middle).
pub fn pad_grid_at(ticks: f32, a: [u8; 3], b: [u8; 3], inward: bool) -> [[u8; 3]; PAD_LEDS] {
    let mut leds = [[0u8; 3]; PAD_LEDS];
    for (i, led) in leds.iter_mut().enumerate() {
        let apart = (i as f32 - PAD_MIDDLE as f32).abs();
        let start = PAD_FIRST_START + PAD_STEP * apart;
        let (la, lb) = levels(ticks, start, inward);
        let (ca, cb) = (scale(a, la), scale(b, lb));
        *led = [ca[0].saturating_add(cb[0]), ca[1].saturating_add(cb[1]), ca[2].saturating_add(cb[2])];
    }
    leds
}

/// A running Tidal: how far into its cycle it is.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tidal {
    ticks: f32,
}

impl Tidal {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lets `secs` seconds go by.
    pub fn advance(&mut self, secs: f32) {
        if secs.is_finite() && secs > 0.0 {
            // (a long stall, for example the PC sleeping, is just a jump forward in the cycle)
            self.ticks = (self.ticks + secs.min(60.0) * TICKS_PER_SEC).rem_euclid(PERIOD);
        }
    }

    pub fn grid(&self, a: [u8; 3], b: [u8; 3], inward: bool) -> KeyGrid {
        grid_at(self.ticks, a, b, inward)
    }

    /// The same, along the cooling pad's strip.
    pub fn pad_grid(&self, a: [u8; 3], b: [u8; 3], inward: bool) -> [[u8; 3]; PAD_LEDS] {
        pad_grid_at(self.ticks, a, b, inward)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: [u8; 3] = DEFAULT_COLOR_A;
    const B: [u8; 3] = DEFAULT_COLOR_B;

    /// What the capture shows at the top right of the main block (row 0, column 12), a frame at a
    /// time from the first frame (which is tick 18 of this model).
    #[test]
    fn the_first_color_steps_up_and_down_like_in_the_capture() {
        let seen = [255, 255, 240, 226, 212, 198, 184, 169, 155, 141, 127, 113, 99, 84, 70, 56, 42, 28, 14, 0, 0];
        for (i, want) in seen.iter().enumerate() {
            let g = grid_at(18.0 + i as f32, A, B, false);
            // (Chroma's own rounding is a hair different here and there, so one step is allowed)
            assert_eq!([g[0][12][0], g[0][12][2]], [0, 0], "frame {}", i);
            assert!((i32::from(g[0][12][1]) - *want).abs() <= 1, "frame {}: {:?} against {}", i, g[0][12], want);
        }
    }

    #[test]
    fn the_second_color_follows_after_a_rest() {
        // the capture: 0 for a while, then 15, 30, ... 255 (twice) and back down to 14 and 0
        // (the rest between the two colors is 63 ticks one time and 62 the next, so a tick is
        // allowed here)
        let g = |frame: f32| grid_at(18.0 + frame, A, B, false)[0][12];
        let near = |px: [u8; 3], blue: i32| {
            assert_eq!(px[0], 0);
            assert_eq!(px[1], 0);
            assert!((i32::from(px[2]) - blue).abs() <= 20, "{:?} against {}", px, blue);
        };
        near(g(44.0), 0);
        near(g(47.0), 30);
        near(g(62.0), 255);
        near(g(64.0), 240);
        near(g(79.0), 28);
        near(g(83.0), 0);
        assert_eq!(g(30.0), [0, 0, 0], "resting between the colors");
        assert_eq!(g(95.0), [0, 0, 0], "resting again");
    }

    #[test]
    fn a_whole_row_matches_the_capture_to_within_a_frame() {
        // row 0 at capture frames 0 and 100 (signed: green above zero, blue below)
        let first = [0, 0, 0, 0, 0, 0, 0, 0, 0, 70, 141, 198, 255, 184, 127];
        let later = [240, 226, 155, 84, 113, 169, 240, 212, 155, 85, 14, 0, 0, 0, 0];
        for (frame, want) in [(0.0, first), (100.0, later)] {
            let g = grid_at(18.0 + frame, A, B, false);
            for c in 0..COLS {
                let got = i32::from(g[0][c][1]) - i32::from(g[0][c][2]);
                assert!((got - want[c]).abs() <= 30, "frame {} column {}: {} against {}", frame, c, got, want[c]);
            }
        }
    }

    #[test]
    fn the_colors_never_show_on_a_key_together() {
        let mut t = Tidal::new();
        for _ in 0..400 {
            t.advance(0.04);
            let g = t.grid(A, B, false);
            for row in g.iter() {
                for px in row.iter() {
                    assert!(px[1] == 0 || px[2] == 0, "{:?}", px);
                    assert_eq!(px[0], 0, "no red in green and blue");
                }
            }
        }
    }

    #[test]
    fn keys_without_a_light_stay_dark_and_the_rest_all_light_up_over_a_cycle() {
        let mut seen = [[false; COLS]; ROWS];
        let mut t = Tidal::new();
        for _ in 0..140 {
            t.advance(0.04);
            let g = t.grid(A, B, false);
            for r in 0..ROWS {
                for c in 0..COLS {
                    if g[r][c] != [0, 0, 0] {
                        seen[r][c] = true;
                    }
                }
            }
        }
        for r in 0..ROWS {
            for c in 0..COLS {
                assert_eq!(seen[r][c], LIT[r][c], "row {} column {}", r, c);
            }
        }
    }

    #[test]
    fn going_in_is_the_same_wave_run_backwards() {
        // the cell on the diagonal line starts its cycle first going out, and last going in
        let out_start = START[0][3];
        let (g, _) = levels(f32::from(out_start) + 1.0, f32::from(out_start), false);
        assert!(g > 0.0);
        // going in, the wave is the mirror image: delays change sign
        let a = grid_at(10.0, A, B, true);
        let b = grid_at(10.0, A, B, false);
        assert_ne!(a, b);
    }

    #[test]
    fn the_pad_s_strip_starts_in_the_middle_and_is_the_same_on_both_sides() {
        // the capture: both sides alike, and the ends about 2.4 s behind the middle
        for led in 1..PAD_LEDS {
            let mirror = 2 * PAD_MIDDLE - led;
            if mirror < PAD_LEDS {
                for step in 0..250 {
                    let leds = pad_grid_at(step as f32, A, B, false);
                    assert_eq!(leds[led], leds[mirror], "light {} and {}", led, mirror);
                }
            }
        }
        // measured from where the first color rises: a light 9 steps out is 9 * 6.65 ticks behind
        let rise = |led: usize| {
            let mut tick = 60.0;
            while pad_grid_at(tick, A, B, false)[led][1] == 0 {
                tick += 0.25;
                if tick > 400.0 {
                    return f32::NAN;
                }
            }
            tick
        };
        let behind = (rise(0) - rise(PAD_MIDDLE)).rem_euclid(PERIOD);
        assert!((behind - 59.85).abs() < 3.0, "{} ticks", behind);
    }

    #[test]
    fn the_pad_s_waves_go_the_other_way_when_they_come_in() {
        let out = pad_grid_at(30.0, A, B, false);
        let inward = pad_grid_at(30.0, A, B, true);
        assert_ne!(out, inward);
        assert!(out.iter().all(|led| led[0] == 0 && (led[1] == 0 || led[2] == 0)));
    }

    #[test]
    fn your_own_colors_are_used_and_black_is_dark() {
        let g = grid_at(18.0, [200, 100, 50], [1, 2, 3], false);
        assert_eq!(g[0][12], [200, 100, 50]);
        let dark = grid_at(18.0, [0, 0, 0], [0, 0, 0], false);
        assert!(dark.iter().flatten().all(|px| *px == [0, 0, 0]));
    }

    #[test]
    fn time_wraps_and_ignores_nonsense() {
        let mut t = Tidal::new();
        t.advance(f32::NAN);
        t.advance(-1.0);
        assert_eq!(t.grid(A, B, false), Tidal::new().grid(A, B, false));
        t.advance(5.0133);
        let g = t.grid(A, B, false);
        let start = Tidal::new().grid(A, B, false);
        for r in 0..ROWS {
            for c in 0..COLS {
                for k in 0..3 {
                    assert!((i32::from(g[r][c][k]) - i32::from(start[r][c][k])).abs() <= 20);
                }
            }
        }
    }
}
