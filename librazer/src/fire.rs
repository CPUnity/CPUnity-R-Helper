//! The Fire keyboard effect, made to match Razer Chroma's own Fire (Chroma Studio) on the Blade 16.
//!
//! It is built from a USB capture of Synapse running Fire on this keyboard (about 800 frames), and
//! what the capture shows is this:
//!
//! * Every key has a fixed *heat*, a map that never changes: the function row is nearly dark and
//!   pure red, the bottom rows are bright and orange, and a few columns burn hotter than their
//!   neighbours. The heat sets the key's *peak* color. Several keys share one heat cell (the two
//!   rows under the function row are the same row, and a few keys on the bottom rows pair up), so
//!   they flicker together. There are 58 of them for the 80 keys that light.
//! * The flicker is the only thing that moves. Every 0.2 seconds, all together, each cell picks a
//!   new random strength between 15% and 100% of its peak, and slides to it in a straight line
//!   over the first four fifths of that time (then stays there for the last fifth).
//! * Ten positions of the 6 x 15 matrix (the second cell of a wide key, such as most of the
//!   space bar) are never lit.
//! * The color of a key is `brightness * mix(cold, hot, how hot)`: with Chroma's colors (cold
//!   red, hot orange) that gives exactly the captured peaks, and any other pair of colors can be
//!   chosen in the same way as in Chroma.
//!
//! The cooling pad's Fire (also captured) is the same effect on a strip of 18 LEDs, each its own
//! heat cell with its own peak (`PAD_PEAK`); see `pad_grid`.
//!
//! This is pure arithmetic (no device, no clock): `advance` takes the seconds that have passed and
//! `grid` is the picture to paint. The keyboard animation worker (in the app) does the painting.
use crate::keymap::{KeyGrid, COLS, ROWS};

/// Chroma's own colors for Fire, as they come out of the capture: the coldest cells are red and
/// the hottest are orange.
pub const DEFAULT_COLD: [u8; 3] = [255, 0, 0];
pub const DEFAULT_HOT: [u8; 3] = [255, 110, 0];

/// Seconds between two new random strengths (all cells together).
pub const PERIOD_SECS: f32 = 0.2037;
/// The part of a period spent sliding to the new strength; the rest is spent holding it.
const SLIDE: f32 = 0.8;
/// The weakest a cell flickers down to, as a part of its peak.
const FLOOR: f32 = 0.15;
/// The most time one `advance` takes into account, so a long pause is not played back.
const MAX_ADVANCE: f32 = 1.0;

/// The number of heat cells (keys that always flicker together count once).
pub const CELLS: usize = 58;

/// Which heat cell each key of the matrix belongs to (-1 = never lit).
#[rustfmt::skip]
const CELL_OF_KEY: [[i8; COLS]; ROWS] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    [15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, -1, 28],
    [15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, -1, 28],
    [29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, -1, -1, 41],
    [42, -1, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, -1, -1, 53],
    [42, 54, 43, -1, 44, 47, -1, -1, 50, 51, 52, 55, 56, 53, 57],
];

/// The peak color of each heat cell as captured (with Chroma's cold and hot colors).
#[rustfmt::skip]
const PEAK: [[u8; 2]; CELLS] = [
    [94, 0], [168, 0], [194, 23], [171, 0], [96, 0], [40, 0], [40, 0], [96, 0], [40, 0], [10, 0],
    [40, 0], [96, 0], [169, 0], [171, 0], [93, 0], [169, 0], [192, 23], [206, 50], [195, 23],
    [171, 0], [96, 0], [96, 0], [171, 0], [97, 0], [41, 0], [97, 0], [171, 0], [193, 23],
    [168, 0], [193, 23], [204, 50], [212, 78], [208, 51], [195, 23], [173, 0], [173, 0],
    [195, 23], [174, 0], [98, 0], [174, 0], [195, 23], [191, 23], [205, 50], [217, 94],
    [214, 79], [208, 51], [197, 23], [197, 23], [208, 51], [198, 24], [176, 0], [198, 24],
    [207, 51], [203, 50], [210, 78], [211, 78], [214, 79], [219, 95],
];

/// LEDs along the cooling pad's light strip (the capture of Synapse's Fire on the pad writes 18).
pub const PAD_LEDS: usize = 18;

/// The peak color of each LED of the cooling pad's strip as captured (with Chroma's colors).
#[rustfmt::skip]
const PAD_PEAK: [[u8; 2]; PAD_LEDS] = [
    [99, 0], [174, 0], [195, 23], [175, 0], [174, 0], [195, 23], [208, 51], [213, 79], [208, 51],
    [195, 23], [175, 0], [172, 0], [195, 23], [171, 0], [99, 0], [173, 0], [196, 23], [202, 49],
];

/// The green of Chroma's hot color against its red (110 / 255): where along cold to hot a peak
/// sits is how much of that green it has.
const HOT_GREEN_RATIO: f32 = 110.0 / 255.0;

/// How bright a peak (red, green) can get (0.0 to 1.0) and how hot it is (0.0 = cold, 1.0 = hot).
fn brightness_and_heat(peak: [u8; 2]) -> (f32, f32) {
    let [r, g] = peak;
    let brightness = r as f32 / 255.0;
    let heat = ((g as f32 / r as f32) / HOT_GREEN_RATIO).clamp(0.0, 1.0);
    (brightness, heat)
}

/// A small fast random generator (xorshift), so the fire needs nothing but itself.
#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    /// 0.0 up to (not including) 1.0.
    fn unit(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        ((x >> 40) as f32) / (1u64 << 24) as f32
    }
}

/// The flame: how strongly every heat cell burns right now.
#[derive(Clone)]
pub struct Fire {
    from: [f32; CELLS],
    to: [f32; CELLS],
    /// Seconds into the current period.
    time: f32,
    rng: Rng,
}

impl Fire {
    /// A fire that has just been lit. The same seed always burns the same way.
    pub fn new(seed: u64) -> Fire {
        let mut rng = Rng::new(seed);
        let mut from = [0.0; CELLS];
        let mut to = [0.0; CELLS];
        for i in 0..CELLS {
            from[i] = FLOOR + (1.0 - FLOOR) * rng.unit();
            to[i] = FLOOR + (1.0 - FLOOR) * rng.unit();
        }
        Fire { from, to, time: 0.0, rng }
    }

    /// Lets `seconds` pass.
    pub fn advance(&mut self, seconds: f32) {
        if !(seconds > 0.0) {
            return;
        }
        self.time += seconds.min(MAX_ADVANCE);
        while self.time >= PERIOD_SECS {
            self.time -= PERIOD_SECS;
            self.from = self.to;
            for i in 0..CELLS {
                self.to[i] = FLOOR + (1.0 - FLOOR) * self.rng.unit();
            }
        }
    }

    /// How strong heat cell `cell` burns now, as a part of its peak (`FLOOR` to 1.0).
    pub fn strength(&self, cell: usize) -> f32 {
        let slid = (self.time / (PERIOD_SECS * SLIDE)).min(1.0);
        self.from[cell] + (self.to[cell] - self.from[cell]) * slid
    }

    /// How strong the key at `row`, `col` of the matrix burns now (`FLOOR` to 1.0), or `None` for
    /// a cell with no light. Used to make a single key flicker like the fire.
    pub fn key_level(&self, row: usize, col: usize) -> Option<f32> {
        let index = *CELL_OF_KEY.get(row)?.get(col)?;
        (index >= 0).then(|| self.strength(index as usize))
    }

    /// How high the flame is at the key at `row`, `col` now, from 0.0 (at its lowest) to 1.0 (at its
    /// highest), or `None` for a cell with no light. A key that burns in two colors goes from the
    /// cold one to the hot one with it.
    pub fn key_heat(&self, row: usize, col: usize) -> Option<f32> {
        let level = self.key_level(row, col)?;
        Some(((level - FLOOR) / (1.0 - FLOOR)).clamp(0.0, 1.0))
    }

    /// The picture of the fire right now, in the given hot and cold colors (Chroma's own are
    /// `DEFAULT_HOT` and `DEFAULT_COLD`).
    pub fn grid(&self, hot: [u8; 3], cold: [u8; 3]) -> KeyGrid {
        let mut grid = [[[0u8; 3]; COLS]; ROWS];
        for (r, cells) in grid.iter_mut().enumerate() {
            for (c, cell) in cells.iter_mut().enumerate() {
                let index = CELL_OF_KEY[r][c];
                if index >= 0 {
                    *cell = color(PEAK[index as usize], self.strength(index as usize), hot, cold);
                }
            }
        }
        grid
    }

    /// The cooling pad's strip right now: LED `i` burns as heat cell `i`.
    pub fn pad_grid(&self, hot: [u8; 3], cold: [u8; 3]) -> [[u8; 3]; PAD_LEDS] {
        let mut leds = [[0u8; 3]; PAD_LEDS];
        for (i, led) in leds.iter_mut().enumerate() {
            *led = color(PAD_PEAK[i], self.strength(i), hot, cold);
        }
        leds
    }

    /// The pad's strip with every LED at its peak.
    pub fn pad_peak(hot: [u8; 3], cold: [u8; 3]) -> [[u8; 3]; PAD_LEDS] {
        let mut leds = [[0u8; 3]; PAD_LEDS];
        for (i, led) in leds.iter_mut().enumerate() {
            *led = color(PAD_PEAK[i], 1.0, hot, cold);
        }
        leds
    }

    /// The picture with every cell at its peak (what the brightest moment looks like).
    pub fn peak_grid(hot: [u8; 3], cold: [u8; 3]) -> KeyGrid {
        let mut grid = [[[0u8; 3]; COLS]; ROWS];
        for (r, cells) in grid.iter_mut().enumerate() {
            for (c, cell) in cells.iter_mut().enumerate() {
                let index = CELL_OF_KEY[r][c];
                if index >= 0 {
                    *cell = color(PEAK[index as usize], 1.0, hot, cold);
                }
            }
        }
        grid
    }
}

/// The color of a heat cell with this `peak` burning at `strength` (0.0 to 1.0 of the peak).
fn color(peak: [u8; 2], strength: f32, hot: [u8; 3], cold: [u8; 3]) -> [u8; 3] {
    let (brightness, heat) = brightness_and_heat(peak);
    let level = brightness * strength;
    let channel = |i: usize| -> u8 {
        let mixed = cold[i] as f32 + (hot[i] as f32 - cold[i] as f32) * heat;
        (mixed * level).round().clamp(0.0, 255.0) as u8
    };
    [channel(0), channel(1), channel(2)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(seed: u64, frames: usize, dt: f32) -> Vec<KeyGrid> {
        let mut fire = Fire::new(seed);
        (0..frames)
            .map(|_| {
                fire.advance(dt);
                fire.grid(DEFAULT_HOT, DEFAULT_COLD)
            })
            .collect()
    }

    /// The captured peak (red, green) of one key, from the table the capture gave.
    fn captured_peak(r: usize, c: usize) -> Option<[u8; 2]> {
        let index = CELL_OF_KEY[r][c];
        (index >= 0).then(|| PEAK[index as usize])
    }

    #[test]
    fn chromas_colors_give_the_captured_peaks_and_ten_keys_stay_dark() {
        let grid = Fire::peak_grid(DEFAULT_HOT, DEFAULT_COLD);
        let mut dark = 0;
        for r in 0..ROWS {
            for c in 0..COLS {
                match captured_peak(r, c) {
                    None => {
                        dark += 1;
                        assert_eq!(grid[r][c], [0, 0, 0]);
                    }
                    Some([red, green]) => {
                        let [gr, gg, gb] = grid[r][c];
                        assert!((gr as i32 - red as i32).abs() <= 1, "red at {r},{c}: {gr} vs {red}");
                        assert!((gg as i32 - green as i32).abs() <= 2, "green at {r},{c}: {gg} vs {green}");
                        assert_eq!(gb, 0);
                    }
                }
            }
        }
        assert_eq!(dark, 10);
        // the shape of the heat: the function row is the darkest and the bottom rows the brightest
        let total = |row: usize| -> u32 { grid[row].iter().map(|c| c[0] as u32 + c[1] as u32).sum() };
        assert!(total(0) < total(1) && total(1) < total(3) && total(3) < total(4));
        assert!(grid[5][14][0] == 219 || grid[5][14][0] == 218 || grid[5][14][0] == 220);
    }

    #[test]
    fn the_pads_strip_has_its_own_captured_peaks_and_flickers_on_its_own() {
        let peak = Fire::pad_peak(DEFAULT_HOT, DEFAULT_COLD);
        for (i, led) in peak.iter().enumerate() {
            let [red, green] = PAD_PEAK[i];
            assert!((led[0] as i32 - red as i32).abs() <= 1, "led {i}");
            assert!((led[1] as i32 - green as i32).abs() <= 2, "led {i}");
            assert_eq!(led[2], 0);
        }
        // the middle of the strip is the hottest and the ends are the dimmest
        assert!(peak[7][1] > peak[0][1] && peak[7][0] > peak[0][0] && peak[14][0] < 120);
        let mut fire = Fire::new(3);
        let mut differing = 0;
        for _ in 0..30 {
            fire.advance(0.041);
            let strip = fire.pad_grid(DEFAULT_HOT, DEFAULT_COLD);
            if strip[1] != strip[3] {
                differing += 1;
            }
            for led in strip {
                assert!(led[0] >= led[1] && led[1] >= led[2]);
            }
        }
        assert!(differing > 25);
        // the colors chosen apply to the strip too
        let blue = Fire::pad_peak([255, 255, 255], [0, 0, 255]);
        assert!(blue[0][2] > blue[0][0] && blue[0][0] == 0, "{:?}", blue[0]);
    }

    #[test]
    fn keys_that_share_a_cell_flicker_together_and_nothing_else_does() {
        let mut fire = Fire::new(4);
        for _ in 0..40 {
            fire.advance(0.037);
            let g = fire.grid(DEFAULT_HOT, DEFAULT_COLD);
            for c in 0..COLS {
                assert_eq!(g[1][c], g[2][c], "the two rows under the function row are one row");
            }
            assert_eq!(g[4][0], g[5][0]);
            assert_eq!(g[4][3], g[5][4]);
            assert_ne!(g[1][1], g[1][2]);
        }
    }

    #[test]
    fn the_flicker_is_a_slide_to_a_new_random_strength_every_fifth_of_a_second() {
        let mut fire = Fire::new(8);
        // one cell, watched often: straight lines between the strengths picked every period
        let mut last = fire.strength(20);
        let mut direction_changes = 0;
        let mut prev_step = 0.0_f32;
        let mut min = 1.0_f32;
        let mut max = 0.0_f32;
        for _ in 0..2000 {
            fire.advance(0.01);
            let now = fire.strength(20);
            min = min.min(now);
            max = max.max(now);
            let step = now - last;
            if step.abs() > 1e-4 && prev_step.abs() > 1e-4 && step.signum() != prev_step.signum() {
                direction_changes += 1;
            }
            if step.abs() > 1e-4 {
                prev_step = step;
            }
            last = now;
        }
        // 20 seconds: about 98 periods, so about as many turns (a new strength can keep the way)
        assert!(direction_changes > 30 && direction_changes < 100, "turns: {direction_changes}");
        assert!(min >= FLOOR - 1e-3 && max <= 1.0 + 1e-3, "{min} {max}");
        assert!(min < 0.3 && max > 0.85, "it uses the whole range: {min} {max}");
    }

    #[test]
    fn the_strengths_are_spread_evenly_between_the_floor_and_the_peak() {
        let mut fire = Fire::new(11);
        let mut sum = 0.0_f32;
        let mut n = 0.0_f32;
        for _ in 0..400 {
            fire.advance(PERIOD_SECS);
            for cell in 0..CELLS {
                sum += fire.from[cell];
                n += 1.0;
            }
        }
        let mean = sum / n;
        assert!((mean - (FLOOR + 1.0) / 2.0).abs() < 0.02, "mean {mean}");
    }

    #[test]
    fn the_hot_and_cold_colors_are_the_ones_given() {
        let blue = [0, 40, 255];
        let white = [255, 255, 255];
        let grid = Fire::peak_grid(white, blue);
        // the coldest bright cell leans blue, the hottest leans white, and nothing is red
        let cold_cell = grid[0][2]; // dim and cold
        let hot_cell = grid[5][14]; // the hottest
        assert!(cold_cell[2] > cold_cell[0], "{cold_cell:?}");
        assert!(hot_cell[0] > 150 && hot_cell[1] > 150 && hot_cell[2] > 150, "{hot_cell:?}");
        // a fire in one color is that color, only brighter or dimmer
        let green = [0, 200, 0];
        let one = Fire::peak_grid(green, green);
        for row in &one {
            for c in row {
                assert_eq!((c[0], c[2]), (0, 0));
            }
        }
        // black is allowed (the fire goes out)
        let out = Fire::peak_grid([0, 0, 0], [0, 0, 0]);
        assert!(out.iter().flatten().all(|c| *c == [0, 0, 0]));
    }

    #[test]
    fn the_same_seed_burns_the_same_and_another_seed_does_not() {
        assert!(run(5, 30, 0.05) == run(5, 30, 0.05));
        assert!(run(5, 30, 0.05) != run(6, 30, 0.05));
    }

    #[test]
    fn time_is_kept_and_a_long_pause_does_not_make_a_long_catch_up() {
        let mut fire = Fire::new(2);
        let before = fire.grid(DEFAULT_HOT, DEFAULT_COLD);
        fire.advance(0.0);
        fire.advance(-1.0);
        fire.advance(f32::NAN);
        assert!(fire.grid(DEFAULT_HOT, DEFAULT_COLD) == before, "no time, no change");
        fire.advance(0.05);
        assert!(fire.grid(DEFAULT_HOT, DEFAULT_COLD) != before);
        let started = std::time::Instant::now();
        fire.advance(3600.0);
        assert!(started.elapsed().as_millis() < 50);
        assert!(fire.time < PERIOD_SECS);
    }
}
