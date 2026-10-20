//! The Wave effect in colors of your own: broad bands of light travel across the keyboard, left or
//! right. With one color the keys swell and dim in it; with two the keys wash from the first color
//! to the second and back as each band passes. (The firmware's own Wave is a rainbow and cannot be
//! given colors, so with colors picked the app draws it.)
//!
//! Pure arithmetic: `advance` takes the seconds that have passed, and the drawing code asks for a grid.
use crate::keymap::{mix, KeyGrid, COLS, ROWS};

/// Seconds a band takes to cross the keyboard once (at the standard speed).
pub const CROSS_SECS: f32 = 3.2;
/// How many bands are on the keyboard at once.
const BANDS: f32 = 1.0;
/// How far each row lags the one above (a fraction of a band), so the bands lean a little.
const ROW_SLANT: f32 = 0.025;
/// With one color, the dimmest a key goes between bands.
const TROUGH: f32 = 0.12;

/// The default pair: orange and magenta.
pub const DEFAULT_A: [u8; 3] = [255, 120, 0];
pub const DEFAULT_B: [u8; 3] = [255, 0, 150];

#[derive(Debug, Clone, Copy, Default)]
pub struct Wave {
    secs: f32,
}

impl Wave {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance(&mut self, secs: f32) {
        if secs.is_finite() && secs > 0.0 {
            self.secs = (self.secs + secs.min(60.0)).rem_euclid(CROSS_SECS);
        }
    }

    /// How far into a band (0.0 between bands, 1.0 at the middle of one) the key at `row`, `col` is.
    fn swell(&self, row: usize, col: usize, left: bool) -> f32 {
        let along = (col as f32 + 0.5) / COLS as f32 * BANDS + row as f32 * ROW_SLANT;
        let moved = self.secs / CROSS_SECS;
        let phase = if left { along + moved } else { along - moved };
        0.5 + 0.5 * (phase * std::f32::consts::TAU).cos()
    }

    /// The keyboard now (only the keys that have a light). `left` sends the bands to the left.
    pub fn grid(&self, a: [u8; 3], b: Option<[u8; 3]>, left: bool) -> KeyGrid {
        let mut grid = [[[0u8; 3]; COLS]; ROWS];
        for (row, cells) in grid.iter_mut().enumerate() {
            for (col, cell) in cells.iter_mut().enumerate() {
                if !crate::tidal::has_light(row, col) {
                    continue;
                }
                let swell = self.swell(row, col, left);
                *cell = match b {
                    Some(b) => mix(b, a, swell),
                    None => {
                        let level = TROUGH + (1.0 - TROUGH) * swell;
                        a.map(|c| (f32::from(c) * level).round() as u8)
                    }
                };
            }
        }
        grid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column_means(g: &KeyGrid) -> Vec<f32> {
        (0..COLS)
            .map(|c| {
                let cells: Vec<f32> = (0..ROWS).filter(|r| crate::tidal::has_light(*r, c)).map(|r| f32::from(g[r][c][0])).collect();
                cells.iter().sum::<f32>() / cells.len().max(1) as f32
            })
            .collect()
    }

    fn brightest_column(g: &KeyGrid) -> usize {
        let m = column_means(g);
        (0..COLS).max_by(|a, b| m[*a].partial_cmp(&m[*b]).unwrap()).unwrap()
    }

    #[test]
    fn one_color_swells_and_dims_and_never_goes_past_its_color() {
        let a = [200, 100, 50];
        let mut wave = Wave::new();
        let (mut low, mut high) = (255u8, 0u8);
        for _ in 0..300 {
            wave.advance(0.03);
            for (r, row) in wave.grid(a, None, false).iter().enumerate() {
                for (c, px) in row.iter().enumerate() {
                    if crate::tidal::has_light(r, c) {
                        assert!(px[0] <= 200 && px[1] <= 100 && px[2] <= 50, "{px:?}");
                        low = low.min(px[0]);
                        high = high.max(px[0]);
                    } else {
                        assert_eq!(*px, [0, 0, 0]);
                    }
                }
            }
        }
        assert!(high > 185 && low < 60, "{low}..{high}");
    }

    #[test]
    fn the_bands_travel_to_the_right_or_to_the_left() {
        let a = [255, 0, 0];
        let mut right = Wave::new();
        let mut left = Wave::new();
        let start_r = brightest_column(&right.grid(a, None, false));
        let start_l = brightest_column(&left.grid(a, None, true));
        assert_eq!(start_r, start_l, "both start in the same place");
        right.advance(CROSS_SECS / 6.0);
        left.advance(CROSS_SECS / 6.0);
        let (r, l) = (brightest_column(&right.grid(a, None, false)), brightest_column(&left.grid(a, None, true)));
        // (the band is wide, so only the direction is asserted, and it wraps round at the edges)
        let step = |from: usize, to: usize| (to as i32 - from as i32).rem_euclid(COLS as i32);
        assert!(step(start_r, r) > 0 && step(start_r, r) < COLS as i32 / 2, "right: {start_r} -> {r}");
        assert!(step(l, start_l) > 0 && step(l, start_l) < COLS as i32 / 2, "left: {start_l} -> {l}");
    }

    #[test]
    fn two_colors_wash_from_one_to_the_other_and_stay_between_them() {
        let (a, b) = ([255, 0, 0], [0, 0, 255]);
        let mut wave = Wave::new();
        let (mut saw_a, mut saw_b, mut saw_mix) = (false, false, false);
        for _ in 0..200 {
            wave.advance(0.05);
            for (r, row) in wave.grid(a, Some(b), false).iter().enumerate() {
                for (c, px) in row.iter().enumerate() {
                    if !crate::tidal::has_light(r, c) {
                        continue;
                    }
                    assert_eq!(px[1], 0, "only blends of the two");
                    saw_a |= px[0] > 240;
                    saw_b |= px[2] > 240;
                    saw_mix |= px[0] > 80 && px[2] > 80;
                }
            }
        }
        assert!(saw_a && saw_b && saw_mix);
        // a stalled clock and odd numbers do nothing
        let mut still = Wave::new();
        still.advance(f32::NAN);
        still.advance(-1.0);
        assert_eq!(still.grid(a, None, false), Wave::new().grid(a, None, false));
    }
}
