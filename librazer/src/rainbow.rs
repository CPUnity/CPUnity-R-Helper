//! The Rainbow effect: the whole color wheel laid out across the keyboard (or along the cooling
//! pad's light strip), red to violet and round again, scrolling along. Unlike Spectrum, where every
//! light shows the same color at the same moment, a Rainbow shows every color at once.
//!
//! This one is the app's own (it is not a copy of a Chroma effect). Pure arithmetic: `advance` takes
//! the seconds that have passed, and the drawing code asks for a grid.
use crate::fire::PAD_LEDS;

/// Seconds the rainbow takes to travel once round (at the standard speed).
pub const CYCLE_SECS: f32 = 7.0;
/// How far the colors lag from one keyboard row to the next (a fraction of the wheel), so the
/// bands lean a little instead of standing straight up and down.
const ROW_SLANT: f32 = 0.03;

/// The fully bright color at `h` (0..1) around the color wheel, red first.
pub fn hue(h: f32) -> [u8; 3] {
    let x = h.rem_euclid(1.0) * 6.0;
    let f = x.fract();
    let (r, g, b) = match x as u32 {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    };
    [r, g, b].map(|c| (c * 255.0).round() as u8)
}

/// A running Rainbow: how far it has travelled.
#[derive(Debug, Clone, Copy, Default)]
pub struct Rainbow {
    secs: f32,
}

impl Rainbow {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lets `secs` seconds go by.
    pub fn advance(&mut self, secs: f32) {
        if secs.is_finite() && secs > 0.0 {
            self.secs = (self.secs + secs.min(60.0)).rem_euclid(CYCLE_SECS);
        }
    }

    /// How far round the wheel the colors have moved (0..1).
    fn offset(&self) -> f32 {
        self.secs / CYCLE_SECS
    }

    /// The pad's strip: the wheel along its 18 lights, travelling from the left end to the right.
    pub fn pad_grid(&self) -> [[u8; 3]; PAD_LEDS] {
        let mut leds = [[0u8; 3]; PAD_LEDS];
        for (i, led) in leds.iter_mut().enumerate() {
            *led = hue(i as f32 / PAD_LEDS as f32 - self.offset());
        }
        leds
    }

    /// The keyboard: the wheel across the columns, a little slanted by row, travelling to the
    /// right; the cells with no key stay dark.
    pub fn grid(&self) -> [[[u8; 3]; 15]; 6] {
        let mut g = [[[0u8; 3]; 15]; 6];
        for (r, row) in g.iter_mut().enumerate() {
            for (k, cell) in row.iter_mut().enumerate() {
                if crate::tidal::has_light(r, k) {
                    *cell = hue(k as f32 / 15.0 + r as f32 * ROW_SLANT - self.offset());
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
    fn the_wheel_goes_red_yellow_green_cyan_blue_magenta() {
        assert_eq!(hue(0.0), [255, 0, 0]);
        assert_eq!(hue(1.0 / 6.0), [255, 255, 0]);
        assert_eq!(hue(2.0 / 6.0), [0, 255, 0]);
        assert_eq!(hue(0.5), [0, 255, 255]);
        assert_eq!(hue(4.0 / 6.0), [0, 0, 255]);
        assert_eq!(hue(5.0 / 6.0), [255, 0, 255]);
        assert_eq!(hue(1.0), hue(0.0));
        assert_eq!(hue(-0.25), hue(0.75));
    }

    #[test]
    fn the_keyboard_shows_every_color_at_once() {
        let g = Rainbow::new().grid();
        let row: Vec<[u8; 3]> = g[0].to_vec();
        // red at the left end, and each neighbouring column a different, nearby color
        assert_eq!(row[0], [255, 0, 0]);
        assert!(row.windows(2).all(|w| w[0] != w[1]));
        let (mut green, mut blue, mut yellow) = (false, false, false);
        for c in &row {
            green |= c[1] == 255 && c[0] < 60 && c[2] < 60;
            blue |= c[2] == 255 && c[0] < 60 && c[1] < 60;
            yellow |= c[0] > 200 && c[1] > 200 && c[2] == 0;
        }
        assert!(green || blue || yellow, "{:?}", row);
        let all: std::collections::HashSet<[u8; 3]> =
            g.iter().flatten().copied().filter(|c| *c != [0, 0, 0]).collect();
        assert!(all.len() > 30, "{} different colors", all.len());
        // keys without a light stay dark, the others never do
        assert_eq!(g[1][13], [0, 0, 0]);
        let lit = g.iter().flatten().filter(|c| **c != [0, 0, 0]).count();
        assert_eq!(lit, 80);
    }

    #[test]
    fn it_scrolls_to_the_right_and_comes_round_again() {
        let mut r = Rainbow::new();
        let start = r.grid();
        r.advance(CYCLE_SECS / 15.0);
        let moved = r.grid();
        // after one column's worth of time, column 1 shows what column 0 showed
        assert_eq!(moved[0][1], start[0][0]);
        assert_ne!(moved, start);
        // a whole turn is back where it began
        let mut whole = Rainbow::new();
        whole.advance(CYCLE_SECS);
        assert_eq!(whole.grid(), start);
        // nonsense time does nothing
        whole.advance(f32::NAN);
        whole.advance(-3.0);
        assert_eq!(whole.grid(), start);
    }

    #[test]
    fn the_pad_strip_has_the_whole_wheel_along_it() {
        let leds = Rainbow::new().pad_grid();
        assert_eq!(leds[0], [255, 0, 0]);
        assert!(leds.windows(2).all(|w| w[0] != w[1]));
        let mut r = Rainbow::new();
        r.advance(CYCLE_SECS / PAD_LEDS as f32);
        assert_eq!(r.pad_grid()[1], leds[0]);
    }
}
