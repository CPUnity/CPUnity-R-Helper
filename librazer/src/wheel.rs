//! The Wheel effect: a hand like a clock's sweeps round the middle of the keyboard, lighting each key
//! as it passes in the first color; behind it the light fades away, turning into the second color
//! along the tail. Clockwise by default, or the other way round.
//!
//! There are two looks. The hand and its tail is the app's own (it is not a copy of a Chroma effect);
//! it is the `Wheel` key effect of Custom Keys (see `keymap::wheel_color`) and the keyboard's Wheel
//! when "Choose colors" is on. The other is Razer's own Wheel, worked out from USB captures of
//! Synapse driving the keyboard and the cooling pad (both ways round): the Spectrum rainbow
//! (see `spectrum::color_at`) laid round a point on the Shift row, a whole turn a second, so that
//! every key shows the color of the rainbow that is at its angle. The pad's 18 lights are two
//! halves of nine, one at the left of that point and one at the right, half a rainbow apart.
//!
//! Pure arithmetic: `advance` takes the seconds that have passed, and the drawing code asks for a grid.
use crate::keymap::{wheel_color_dir, KeyGrid, COLS, ROWS, WHEEL_TURN_SECS};
use crate::{dipp, spectrum};

/// The default pair: a cyan hand with a violet tail.
pub const DEFAULT_A: [u8; 3] = [0, 200, 255];
pub const DEFAULT_B: [u8; 3] = [136, 0, 255];

/// Seconds for Razer's rainbow Wheel to go once round (at the speed Synapse starts with). It must
/// go into `WHEEL_TURN_SECS` a whole number of times, since `Wheel` wraps its clock there.
pub const RAINBOW_TURN_SECS: f32 = 1.0;

/// Where on the rainbow the straight-up direction is (a part of a turn), as the captures show.
const RAINBOW_UP: f32 = -5.0 / 12.0;

/// How far round the middle the key at `row`, `col` is, as a part of a turn clockwise from straight
/// up (0.0..1.0), where Synapse puts it. The middle is the point between the Shift row's B and N keys
/// (column 7, row 4), so rows 0 to 3 are plain geometry; the keys of row 4 are level with it (the
/// left ones point left and the right ones right) and the bottom row's keys sit at the angles the
/// captures show (that row's keys are not where their cells would put them).
fn rainbow_angle(row: usize, col: usize) -> f32 {
    /// The bottom row's measured angles (0.0: no light there).
    const BOTTOM: [f32; COLS] = [
        0.7267, 0.7242, 0.7167, 0.0, 0.7117, 0.6242, 0.0, 0.0, 0.3242, 0.3017, 0.2892, 0.2817, 0.2767, 0.2717, 0.3017,
    ];
    match row {
        4 => {
            if col < 7 {
                0.75
            } else {
                0.25
            }
        }
        5 => BOTTOM.get(col).copied().unwrap_or(0.0),
        _ => {
            let dx = col as f32 - 7.0;
            let dy = row as f32 - 4.0;
            dx.atan2(-dy).rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU
        }
    }
}

/// The rainbow at `angle` (a part of a turn clockwise from up), `secs` into the spin: the rainbow
/// is laid round the middle and goes round with it, clockwise or (`ccw`) the other way.
fn rainbow_at(angle: f32, secs: f32, ccw: bool) -> [u8; 3] {
    let turn = (secs / RAINBOW_TURN_SECS).rem_euclid(1.0);
    let spin = if ccw { turn } else { -turn };
    spectrum::color_at((angle + RAINBOW_UP + spin).rem_euclid(1.0) * spectrum::CYCLE_SECS)
}

/// The color of the key at `row`, `col` for Razer's Wheel `secs` into the spin.
pub fn rainbow_color(row: usize, col: usize, secs: f32, ccw: bool) -> [u8; 3] {
    rainbow_at(rainbow_angle(row, col), secs, ccw)
}

/// How much of the way round each of the two colors rests, in the two-color wheel (a part of a half
/// turn), so that the colors are seen as themselves and not only passed through.
const DIPP_REST: f32 = 0.12;

/// The two-color Wheel at `angle` (a part of a turn clockwise from up): the same wheel, but with
/// the colors going from `a` to `b` through the hues between (as Color Dipp does) over half a turn
/// and back over the other half. `a` is at the top at the start.
fn dipp_at(angle: f32, secs: f32, a: [u8; 3], b: [u8; 3], ccw: bool) -> [u8; 3] {
    let turn = (secs / RAINBOW_TURN_SECS).rem_euclid(1.0);
    let f = (angle + if ccw { turn } else { -turn }).rem_euclid(1.0);
    let there_and_back = if f < 0.5 { 2.0 * f } else { 2.0 * (1.0 - f) };
    dipp::blend(a, b, (there_and_back * (1.0 + 2.0 * DIPP_REST) - DIPP_REST).clamp(0.0, 1.0))
}

/// The color of the key at `row`, `col` for the Wheel in two colors of your own.
pub fn dipp_color(row: usize, col: usize, secs: f32, a: [u8; 3], b: [u8; 3], ccw: bool) -> [u8; 3] {
    dipp_at(rainbow_angle(row, col), secs, a, b, ccw)
}

/// The cooling pad's 18 lights for the Wheel in two colors of your own (laid out as the rainbow's).
pub fn dipp_pad(secs: f32, a: [u8; 3], b: [u8; 3], ccw: bool) -> [[u8; 3]; crate::fire::PAD_LEDS] {
    let half = crate::fire::PAD_LEDS / 2;
    std::array::from_fn(|i| dipp_at(if i < half { 0.75 } else { 0.25 }, secs, a, b, ccw))
}

/// The cooling pad's 18 lights for Razer's Wheel: the first nine at the left of the middle, the
/// last nine at the right (so the two halves are half a rainbow apart).
pub fn rainbow_pad(secs: f32, ccw: bool) -> [[u8; 3]; crate::fire::PAD_LEDS] {
    let half = crate::fire::PAD_LEDS / 2;
    std::array::from_fn(|i| rainbow_at(if i < half { 0.75 } else { 0.25 }, secs, ccw))
}

/// A running Wheel: how long it has been turning.
#[derive(Debug, Clone, Copy, Default)]
pub struct Wheel {
    secs: f32,
}

impl Wheel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lets `secs` seconds go by (the hand wraps after a turn, so a long stall is only a moment).
    pub fn advance(&mut self, secs: f32) {
        if secs.is_finite() && secs > 0.0 {
            self.secs = (self.secs + secs.min(60.0)).rem_euclid(WHEEL_TURN_SECS);
        }
    }

    /// Razer's Wheel on the keyboard now: every key that has a light, the others dark.
    pub fn rainbow_grid(&self, ccw: bool) -> KeyGrid {
        let mut grid = [[[0u8; 3]; COLS]; ROWS];
        for (row, cells) in grid.iter_mut().enumerate() {
            for (col, cell) in cells.iter_mut().enumerate() {
                if crate::tidal::has_light(row, col) {
                    *cell = rainbow_color(row, col, self.secs, ccw);
                }
            }
        }
        grid
    }

    /// The Wheel in two colors of your own, on the keyboard now.
    pub fn dipp_grid(&self, a: [u8; 3], b: [u8; 3], ccw: bool) -> KeyGrid {
        let mut grid = [[[0u8; 3]; COLS]; ROWS];
        for (row, cells) in grid.iter_mut().enumerate() {
            for (col, cell) in cells.iter_mut().enumerate() {
                if crate::tidal::has_light(row, col) {
                    *cell = dipp_color(row, col, self.secs, a, b, ccw);
                }
            }
        }
        grid
    }

    /// The Wheel in two colors of your own, on the cooling pad now.
    pub fn dipp_pad(&self, a: [u8; 3], b: [u8; 3], ccw: bool) -> [[u8; 3]; crate::fire::PAD_LEDS] {
        dipp_pad(self.secs, a, b, ccw)
    }

    /// Razer's Wheel on the cooling pad now.
    pub fn rainbow_pad(&self, ccw: bool) -> [[u8; 3]; crate::fire::PAD_LEDS] {
        rainbow_pad(self.secs, ccw)
    }

    /// The keyboard now: every key that has a light, the cells with no key stay dark. `b` is the
    /// color of the tail's end, if there is one; `ccw` sends the hand the other way round.
    pub fn grid(&self, a: [u8; 3], b: Option<[u8; 3]>, ccw: bool) -> KeyGrid {
        let mut grid = [[[0u8; 3]; COLS]; ROWS];
        for (row, cells) in grid.iter_mut().enumerate() {
            for (col, cell) in cells.iter_mut().enumerate() {
                if crate::tidal::has_light(row, col) {
                    *cell = wheel_color_dir(row, col, self.secs, a, b, ccw);
                }
            }
        }
        grid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(g: &KeyGrid) -> usize {
        g.iter().flatten().filter(|c| **c != [0, 0, 0]).count()
    }

    #[test]
    fn a_part_of_the_keyboard_is_lit_and_the_rest_is_dark_and_it_comes_round_again() {
        let mut wheel = Wheel::new();
        let white = [255, 255, 255];
        let mut counts = Vec::new();
        for _ in 0..200 {
            wheel.advance(0.04);
            let n = lit(&wheel.grid(white, None, false));
            assert!(n > 0, "something is always lit");
            counts.push(n);
        }
        let (least, most) = (counts.iter().min().unwrap(), counts.iter().max().unwrap());
        assert!(*most < 70 && *most > 8 && most > least, "a slice of the keyboard: {least}..{most}");
        // a whole turn later it is the same picture
        let before = wheel.grid(white, Some([0, 0, 255]), false);
        wheel.advance(WHEEL_TURN_SECS);
        assert_eq!(wheel.grid(white, Some([0, 0, 255]), false), before);
    }

    #[test]
    fn the_hand_goes_clockwise_or_the_other_way_and_never_lights_a_key_without_a_light() {
        // a quarter turn: the hand points right, so clockwise lights the right side and the other
        // way round lights the left
        let mut wheel = Wheel::new();
        wheel.advance(WHEEL_TURN_SECS / 4.0);
        let red = [255, 0, 0];
        let side = |g: &KeyGrid, right: bool| {
            let range = if right { 8..15 } else { 0..7 };
            g.iter().flat_map(|row| range.clone().map(move |c| row[c])).filter(|c| *c != [0, 0, 0]).count()
        };
        let cw = wheel.grid(red, None, false);
        let ccw = wheel.grid(red, None, true);
        assert!(side(&cw, true) > side(&cw, false), "clockwise is at the right");
        assert!(side(&ccw, false) > side(&ccw, true), "counter-clockwise is at the left");
        for g in [&cw, &ccw] {
            for (r, row) in g.iter().enumerate() {
                for (c, px) in row.iter().enumerate() {
                    if !crate::tidal::has_light(r, c) {
                        assert_eq!(*px, [0, 0, 0]);
                    }
                }
            }
        }
    }

    #[test]
    fn odd_times_do_nothing_and_the_second_color_shows_in_the_tail() {
        let mut wheel = Wheel::new();
        wheel.advance(f32::NAN);
        wheel.advance(-3.0);
        wheel.advance(f32::INFINITY);
        assert_eq!(lit(&wheel.grid([9, 9, 9], None, false)), lit(&Wheel::new().grid([9, 9, 9], None, false)));
        wheel.advance(1.3);
        let g = wheel.grid([255, 0, 0], Some([0, 0, 255]), false);
        assert!(g.iter().flatten().any(|c| c[0] > 150 && c[2] < 60), "the first color at the hand");
        assert!(g.iter().flatten().any(|c| c[2] > 0 && c[0] < c[2]), "and the second along the tail");
    }

    // Keys from Synapse's own frames (the keyboard capture, clockwise): (row, col, 25ths of a turn
    // in, the color that was sent). The model is a table read off the rainbow, so a few steps off
    // is the most that can be asked of it.
    const CAPTURED: [(usize, usize, usize, [u8; 3]); 24] = [
        (0, 0, 0, [0x10, 0x8c, 0xff]),
        (0, 14, 0, [0xc0, 0x29, 0xff]),
        (2, 7, 0, [0x18, 0x58, 0xff]),
        (3, 3, 0, [0x07, 0xc9, 0xff]),
        (5, 13, 0, [0xff, 0x16, 0xc6]),
        (5, 0, 0, [0x00, 0xff, 0x3b]),
        (0, 0, 7, [0xf5, 0xcc, 0x00]),
        (0, 14, 7, [0x18, 0x58, 0xff]),
        (2, 7, 7, [0x00, 0xff, 0x00]),
        (3, 3, 7, [0xff, 0x7e, 0x00]),
        (5, 13, 7, [0x18, 0x58, 0xff]),
        (5, 0, 7, [0xff, 0x39, 0x0a]),
        (0, 0, 13, [0xff, 0x14, 0x91]),
        (0, 14, 13, [0xdf, 0xff, 0x00]),
        (2, 7, 13, [0xff, 0x5f, 0x04]),
        (3, 3, 13, [0xff, 0x16, 0xcc]),
        (5, 13, 13, [0x00, 0xfa, 0xff]),
        (5, 0, 13, [0xe8, 0x1e, 0xff]),
        (0, 0, 20, [0x38, 0x4f, 0xff]),
        (0, 14, 20, [0xff, 0x11, 0x48]),
        (2, 7, 20, [0xe1, 0x20, 0xff]),
        (3, 3, 20, [0x18, 0x58, 0xff]),
        (5, 13, 20, [0xff, 0x56, 0x05]),
        (5, 0, 20, [0x18, 0x58, 0xff]),
    ];

    #[test]
    fn razers_wheel_matches_the_keys_in_the_capture() {
        let mut off = 0;
        for (row, col, step, want) in CAPTURED {
            let got = rainbow_color(row, col, step as f32 / 25.0, false);
            let worst = (0..3).map(|i| (i32::from(got[i]) - i32::from(want[i])).abs()).max().unwrap();
            if worst > 12 {
                off += 1;
            }
            assert!(worst <= 24, "key {row},{col} at {step}/25: {got:?} against {want:?}");
        }
        // (a key can be a step or so off where the rainbow changes fast, never most of them)
        assert!(off <= 3, "{off} keys are off");
    }

    #[test]
    fn the_wheel_turns_once_a_second_and_the_other_way_is_the_same_picture_backwards() {
        let wheel = |t: f32, ccw: bool| {
            let mut w = Wheel::new();
            w.advance(t);
            w.rainbow_grid(ccw)
        };
        assert_eq!(wheel(0.0, false), wheel(1.0, false), "a turn a second");
        assert_eq!(wheel(0.0, false), wheel(3.0, true));
        // going the other way round is the same moment run backwards
        for t in [0.1_f32, 0.37, 0.62] {
            let (a, b) = (wheel(t, true), wheel(1.0 - t, false));
            for (ra, rb) in a.iter().zip(&b) {
                for (pa, pb) in ra.iter().zip(rb) {
                    assert!((0..3).all(|i| pa[i].abs_diff(pb[i]) <= 1), "t = {t}: {pa:?} {pb:?}");
                }
            }
        }
        // keys without a light stay dark, the others always show a color
        let g = wheel(0.3, false);
        for (r, row) in g.iter().enumerate() {
            for (c, px) in row.iter().enumerate() {
                assert_eq!(crate::tidal::has_light(r, c), *px != [0, 0, 0], "key {r},{c}");
            }
        }
    }

    #[test]
    fn the_rainbow_rests_on_one_blue_and_the_pad_s_halves_are_half_a_rainbow_apart() {
        // on the way round, a key spends a few steps on the blue (24, 88, 255)
        let near = |c: [u8; 3]| c[0].abs_diff(24) <= 2 && c[1].abs_diff(88) <= 2 && c[2] == 255;
        let blues = (0..25).filter(|s| near(rainbow_color(0, 0, *s as f32 / 25.0, false))).count();
        assert!((3..=5).contains(&blues), "{blues} steps on the blue");
        // the pad: nine lights alike on each side, the two sides half a turn of the rainbow apart
        for step in 0..25 {
            let t = step as f32 / 25.0;
            let pad = rainbow_pad(t, false);
            assert!(pad[..9].iter().all(|c| *c == pad[0]) && pad[9..].iter().all(|c| *c == pad[9]));
            let apart = crate::spectrum::color_at(
                crate::spectrum::CYCLE_SECS * (0.75 - 5.0 / 12.0 - t) - crate::spectrum::CYCLE_SECS / 2.0,
            );
            assert!((0..3).all(|i| pad[9][i].abs_diff(apart[i]) <= 1), "at {step}/25: {:?} {apart:?}", pad[9]);
        }
        // left and right swap places along the rainbow when it goes the other way
        assert_ne!(rainbow_pad(0.2, false), rainbow_pad(0.2, true));
    }

    #[test]
    fn the_wheel_in_two_colors_shows_both_and_the_hues_between_and_turns_like_the_rainbow() {
        let (red, violet) = ([255, 0, 0], [136, 0, 255]);
        let mut seen_a = 0;
        let mut seen_b = 0;
        let mut between = 0;
        for row in 0..ROWS {
            for col in 0..COLS {
                if !crate::tidal::has_light(row, col) {
                    continue;
                }
                let c = dipp_color(row, col, 0.0, red, violet, false);
                // (the bottom has no key straight under the middle, so the second color is only close)
                let close = |x: [u8; 3]| (0..3).map(|i| c[i].abs_diff(x[i]) as u32).sum::<u32>() < 60;
                if close(red) {
                    seen_a += 1;
                } else if close(violet) {
                    seen_b += 1;
                } else {
                    between += 1;
                }
            }
        }
        assert!(seen_a > 3 && between > 10, "{seen_a} {seen_b} {between}");
        // (the second color starts under the keyboard, where there are hardly any keys; it comes round)
        let violet_somewhere = (0..20).any(|s| {
            let t = s as f32 / 20.0;
            (0..ROWS).any(|r| (0..COLS).any(|c| crate::tidal::has_light(r, c) && dipp_color(r, c, t, red, violet, false) == violet))
        });
        assert!(violet_somewhere);
        let _ = seen_b;
        // a turn a second, the other way round is the same moment backwards
        assert_eq!(dipp_color(1, 3, 0.25, red, violet, false), dipp_color(1, 3, 1.25, red, violet, false));
        let near = |x: [u8; 3], y: [u8; 3]| (0..3).all(|i| x[i].abs_diff(y[i]) <= 1);
        assert!(near(dipp_color(2, 9, 0.3, red, violet, true), dipp_color(2, 9, 0.7, red, violet, false)));
        // the top at the start is the first color; half a turn later it is the second
        assert_eq!(dipp_color(0, 7, 0.0, red, violet, false), red);
        assert_eq!(dipp_color(0, 7, 0.5, red, violet, false), violet);
        // the pad: two halves of nine, and the keyboard's grid keeps dark keys dark
        let pad = dipp_pad(0.2, red, violet, false);
        assert!(pad[..9].iter().all(|c| *c == pad[0]) && pad[9..].iter().all(|c| *c == pad[9]));
        let mut wheel = Wheel::new();
        wheel.advance(0.4);
        let g = wheel.dipp_grid(red, violet, false);
        assert!(g.iter().enumerate().all(|(r, row)| row.iter().enumerate().all(|(c, px)| crate::tidal::has_light(r, c) == (*px != [0, 0, 0]))));
    }
}
