//! The Void's moving parts, kept free of drawing so they can be tested: the glitching stars and the
//! black and dark-purple clouds of its backdrop, the bars of colour that tear across it now and then,
//! and the torn title of its banner. Everything is a pure function of the time, so a picture of any
//! moment can be made again. Positions are fractions of the area (0..1); sizes and offsets are in
//! points.

use std::f32::consts::TAU;

/// The Void's logo (`assets/void-logo.png`): the CPUnity badge in violet, split in cyan and magenta.
pub const LOGO_PNG: &[u8] = include_bytes!("../assets/void-logo.png");

pub const STARS: usize = 140;
pub const CLOUDS: usize = 9;
pub const TEARS: usize = 5;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Star {
    pub x: f32,
    pub y: f32,
    /// Side of the little square, in points.
    pub size: f32,
    /// A violet one with no ghosts, far away.
    pub dim: bool,
    /// One that glitches now and then (about a quarter of them).
    pub glitchy: bool,
    /// How bright it is at rest.
    pub base: f32,
}

pub fn star(i: usize) -> Star {
    let i = i as u32;
    let big = hash(i, 11);
    let kind = hash(i, 12);
    Star {
        x: hash(i, 1),
        y: hash(i, 2),
        size: if big > 0.92 { 4.5 } else if big > 0.6 { 3.0 } else { 2.0 },
        dim: kind < 0.3,
        glitchy: kind > 0.74,
        base: 0.35 + 0.65 * hash(i, 3),
    }
}

/// What a star looks like right now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flicker {
    pub alpha: f32,
    /// How far the cyan ghost sits to the left and the magenta to the right (0: no ghosts).
    pub split: f32,
    /// How far it has jumped from its place.
    pub jump: (f32, f32),
    /// A sideways streak of light behind it, in points (0: none).
    pub streak: f32,
}

/// How long a glitch lasts.
pub const GLITCH_LEN: f32 = 0.22;

pub fn flicker(s: &Star, i: usize, t: f32) -> Flicker {
    let i = i as u32;
    let twinkle = 0.78 + 0.22 * (t * (0.5 + hash(i, 4)) * 2.0 + hash(i, 5) * TAU).sin();
    let mut f = Flicker { alpha: (s.base * twinkle).clamp(0.0, 1.0), split: if s.dim { 0.0 } else { 1.5 }, jump: (0.0, 0.0), streak: 0.0 };
    if s.glitchy {
        let period = 2.5 + 6.0 * hash(i, 6);
        let shifted = t + hash(i, 7) * period;
        let n = (shifted / period).floor();
        let local = shifted - n * period;
        if local < GLITCH_LEN {
            let u = local / GLITCH_LEN;
            let k = (n as u32).wrapping_mul(3);
            f.split = 2.0 + 5.0 * (1.0 - u);
            f.jump = ((hash(i, k + 100) - 0.5) * 12.0, (hash(i, k + 101) - 0.5) * 5.0);
            f.streak = if hash(i, k + 102) > 0.55 { (20.0 + 60.0 * hash(i, k + 103)) * (1.0 - u) } else { 0.0 };
            f.alpha = 1.0;
        }
    }
    f
}

/// A soft cloud: black ones and dark-purple ones drift past each other so the sky is never one
/// colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cloud {
    /// Centre, as fractions of the area.
    pub x: f32,
    pub y: f32,
    /// Radii, as fractions of the width and the height.
    pub rx: f32,
    pub ry: f32,
    pub dark: bool,
    /// How thick it is at its middle, 0..1.
    pub alpha: f32,
}

pub fn cloud(i: usize, t: f32) -> Cloud {
    let n = i as u32;
    let dark = i % 2 == 0;
    let speed = 0.08 + 0.08 * hash(n, 23);
    let phase = hash(n, 24) * TAU;
    // the two kinds drift the opposite way round, so they weave through each other
    let dir = if dark { 1.0 } else { -1.0 };
    Cloud {
        x: 0.15 + 0.7 * hash(n, 20) + 0.12 * (t * speed * dir + phase).sin(),
        y: 0.05 + 0.9 * hash(n, 21) + 0.07 * (t * speed * 0.8 + phase * 1.7).cos(),
        rx: 0.28 + 0.34 * hash(n, 22),
        ry: 0.14 + 0.2 * hash(n, 25),
        dark,
        alpha: if dark { 0.9 } else { 0.35 + 0.25 * hash(n, 26) },
    }
}

/// A bar of cyan or magenta that tears across the window for a moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tear {
    pub y: f32,
    pub x: f32,
    /// Width, as a fraction of the area's.
    pub w: f32,
    /// Height, in points.
    pub h: f32,
    pub cyan: bool,
    pub alpha: f32,
}

pub const TEAR_LEN: f32 = 0.3;

pub fn tear(k: usize, t: f32) -> Option<Tear> {
    let k = k as u32;
    let period = 3.0 + 5.0 * hash(k, 30);
    let shifted = t + hash(k, 31) * period;
    let n = (shifted / period).floor();
    let local = shifted - n * period;
    if local >= TEAR_LEN {
        return None;
    }
    let u = local / TEAR_LEN;
    let j = (n as u32).wrapping_mul(5);
    let w = 0.12 + 0.3 * hash(k, j + 42);
    Some(Tear {
        y: hash(k, j + 40),
        x: hash(k, j + 41) * (1.0 - w),
        w,
        h: 2.0 + 8.0 * hash(k, j + 43),
        cyan: hash(k, j + 44) > 0.5,
        alpha: 0.5 * (1.0 - u),
    })
}

/// How the halves of the banner's title are placed, in points sideways.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TitleTear {
    /// The upper half and the lower half of the letters.
    pub top: f32,
    pub bottom: f32,
    /// The slivers of cyan and magenta between them.
    pub cyan: f32,
    pub magenta: f32,
    /// 0..1: a jolt, when everything jumps for a moment.
    pub jolt: f32,
}

pub const JOLT_EVERY: f32 = 5.3;
pub const JOLT_LEN: f32 = 0.18;

pub fn title_tear(t: f32) -> TitleTear {
    let local = t % JOLT_EVERY;
    let jolt = if local < JOLT_LEN { 1.0 - local / JOLT_LEN } else { 0.0 };
    let m = 1.0 + 0.9 * jolt;
    TitleTear {
        top: (-3.0 + 1.5 * (t * 1.3).sin()) * m,
        bottom: (4.0 + 1.5 * (t * 1.1 + 1.0).sin()) * m,
        cyan: (8.0 + 3.0 * (t * 2.1).sin()) * m,
        magenta: (-9.0 + 3.0 * (t * 1.7 + 2.0).sin()) * m,
        jolt,
    }
}

/// A line of fake data for the banner's sides (plain ASCII, so any font has it), new every quarter
/// of a second.
pub fn data_line(row: u32, side: u32, t: f32) -> String {
    let step = (t * 4.0).floor() as u32;
    let key = row.wrapping_mul(31).wrapping_add(side.wrapping_mul(977)).wrapping_add(step.wrapping_mul(7919));
    let mut out = String::new();
    for c in 0..11u32 {
        let h = hash(key, c);
        out.push(match (h * 6.0) as u32 {
            0 | 1 => '0',
            2 | 3 => '1',
            4 => '#',
            _ => if c % 3 == 0 { ':' } else { ' ' },
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_logo_is_a_png() {
        assert_eq!(&LOGO_PNG[1..4], b"PNG");
    }

    #[test]
    fn the_stars_are_the_same_every_time_and_spread_over_the_whole_sky() {
        for i in 0..STARS {
            assert_eq!(star(i), star(i));
            let s = star(i);
            assert!((0.0..=1.0).contains(&s.x) && (0.0..=1.0).contains(&s.y), "{s:?}");
            assert!((2.0..=4.5).contains(&s.size) && (0.3..=1.0).contains(&s.base));
        }
        let stars: Vec<Star> = (0..STARS).map(star).collect();
        let glitchy = stars.iter().filter(|s| s.glitchy).count();
        let dim = stars.iter().filter(|s| s.dim).count();
        assert!(glitchy > STARS / 8 && glitchy < STARS / 2, "{glitchy}");
        assert!(dim > STARS / 8 && dim < STARS / 2, "{dim}");
        // all four quarters of the sky have stars
        for (qx, qy) in [(false, false), (true, false), (false, true), (true, true)] {
            assert!(stars.iter().any(|s| (s.x > 0.5) == qx && (s.y > 0.5) == qy));
        }
    }

    #[test]
    fn only_the_glitchy_stars_glitch_and_only_for_a_moment() {
        let stars: Vec<Star> = (0..STARS).map(star).collect();
        let (mut glitching, mut samples) = (0, 0);
        for (i, s) in stars.iter().enumerate() {
            for step in 0..400 {
                let f = flicker(s, i, step as f32 * 0.05);
                assert!((0.0..=1.0).contains(&f.alpha));
                let glitch = f.jump != (0.0, 0.0);
                if !s.glitchy {
                    assert!(!glitch && f.streak == 0.0, "a calm star stays put");
                    assert!(f.split <= 1.5);
                } else {
                    samples += 1;
                    if glitch {
                        glitching += 1;
                        assert!(f.split >= 2.0 && f.jump.0.abs() <= 6.0 && f.jump.1.abs() <= 2.5 && f.streak <= 80.0);
                    }
                }
                if s.dim {
                    assert!(f.split == 0.0 || s.glitchy, "a dim star has no ghosts");
                }
            }
        }
        let share = glitching as f32 / samples as f32;
        assert!(share > 0.02 && share < 0.2, "glitches are brief and now and then: {share}");
    }

    #[test]
    fn the_clouds_drift_slowly_black_and_purple_in_turn() {
        for i in 0..CLOUDS {
            let (a, b) = (cloud(i, 0.0), cloud(i, 60.0));
            assert_eq!(a.dark, i % 2 == 0);
            assert!(a.rx > 0.2 && a.ry > 0.1 && (0.0..=1.0).contains(&a.alpha));
            let moved = ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt();
            assert!(moved > 0.0 && moved < 0.4, "{moved}");
            for t in [0.0, 13.0, 200.0, 3000.0] {
                let c = cloud(i, t);
                assert!((-0.1..=1.1).contains(&c.x) && (-0.1..=1.1).contains(&c.y), "{c:?}");
            }
        }
        assert!((0..CLOUDS).any(|i| cloud(i, 0.0).dark) && (0..CLOUDS).any(|i| !cloud(i, 0.0).dark));
    }

    #[test]
    fn the_tears_come_and_go_and_stay_inside_the_window() {
        let mut seen = [false; TEARS];
        let mut both = (false, false);
        for step in 0..4000 {
            let t = step as f32 * 0.05;
            for k in 0..TEARS {
                if let Some(x) = tear(k, t) {
                    seen[k] = true;
                    both = (both.0 || x.cyan, both.1 || !x.cyan);
                    assert!(x.x + x.w <= 1.0 && (0.0..=1.0).contains(&x.y) && x.h >= 2.0 && x.h <= 10.0);
                    assert!(x.alpha > 0.0 && x.alpha <= 0.5);
                }
            }
        }
        assert!(seen.iter().all(|s| *s), "every bar shows up in the end");
        assert!(both.0 && both.1, "some are cyan and some magenta");
        let showing = (0..4000).filter(|s| tear(0, *s as f32 * 0.05).is_some()).count();
        assert!(showing > 0 && showing < 600, "a bar is mostly not there: {showing}");
    }

    #[test]
    fn the_title_halves_sit_apart_the_way_the_mock_shows_and_jolt_now_and_then() {
        let calm = title_tear(2.0);
        assert_eq!(calm.jolt, 0.0);
        assert!(calm.top < -1.0 && calm.bottom > 2.0, "the halves are on opposite sides");
        assert!(calm.cyan > 4.0 && calm.magenta < -4.0, "the slivers too");
        let jolt = title_tear(JOLT_EVERY * 3.0 + 0.01);
        assert!(jolt.jolt > 0.9 && jolt.top != calm.top);
        // it never gets far enough to make the words unreadable
        for step in 0..2000 {
            let x = title_tear(step as f32 * 0.03);
            assert!(x.top.abs() < 10.0 && x.bottom.abs() < 12.0 && x.cyan.abs() < 25.0 && x.magenta.abs() < 25.0);
        }
    }

    #[test]
    fn the_data_on_the_banners_sides_changes_four_times_a_second_in_plain_ascii() {
        let a = data_line(0, 0, 1.0);
        assert_eq!(a, data_line(0, 0, 1.1), "the same within a quarter of a second");
        assert_ne!(a, data_line(0, 0, 1.3));
        assert_ne!(data_line(0, 0, 1.0), data_line(1, 0, 1.0));
        assert_ne!(data_line(0, 0, 1.0), data_line(0, 1, 1.0));
        for row in 0..4 {
            let line = data_line(row, 0, 2.0);
            assert_eq!(line.len(), 11);
            assert!(line.is_ascii());
        }
    }
    /// Prints the backdrop at a few moments for tools/void_check.py. Run with
    /// `cargo test dump_void -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn dump_void() {
        for t in [3.0f32, 7.4, 12.1] {
            println!("== t{t}");
            for i in 0..CLOUDS {
                let c = cloud(i, t);
                println!("CLOUD {} {} {} {} {} {}", c.x, c.y, c.rx, c.ry, c.dark as u8, c.alpha);
            }
            for i in 0..STARS {
                let s = star(i);
                let f = flicker(&s, i, t);
                println!("STAR {} {} {} {} {} {} {} {} {}", s.x, s.y, s.size, s.dim as u8, f.alpha, f.split, f.jump.0, f.jump.1, f.streak);
            }
            for k in 0..TEARS {
                if let Some(x) = tear(k, t) {
                    println!("TEAR {} {} {} {} {} {}", x.x, x.y, x.w, x.h, x.cyan as u8, x.alpha);
                }
            }
            let tt = title_tear(t);
            println!("TITLE {} {} {} {} {}", tt.top, tt.bottom, tt.cyan, tt.magenta, tt.jolt);
        }
    }
}
