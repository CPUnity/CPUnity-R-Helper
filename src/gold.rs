//! The Gold Edition's moving parts, kept free of drawing so they can be tested: the sparkles that
//! drift up the window, the glint that sweeps across the banner and the shimmer of the sun rays.
//! Positions are fractions of the area (0..1) so the painter fits them to any window size.

use std::f32::consts::TAU;

/// The Gold Edition pictures (`assets/gold-*.png`): the header logo, the banner and the empty
/// colosseum the cat king sits in on the Info tab (he is drawn live, see `king.rs`).
pub const LOGO_PNG: &[u8] = include_bytes!("../assets/gold-logo.png");
pub const BANNER_PNG: &[u8] = include_bytes!("../assets/gold-banner.png");
pub const ARENA_PNG: &[u8] = include_bytes!("../assets/gold-arena.png");
/// Father Time without his staff (the front and mask layers), and the staff by itself, so the staff
/// can be held out to you (see `smoker.rs`). In this order, to match `ui::backdrop::paint_father`.
/// The angel cats (three side by side), for Game 2.
pub const ANGELS_PNG: &[u8] = include_bytes!("../assets/backdrop-angels.png");
pub const STAFF_PNGS: [&[u8]; 3] = [
    include_bytes!("../assets/backdrop-father-front-bare.png"),
    include_bytes!("../assets/backdrop-father-mask-bare.png"),
    include_bytes!("../assets/backdrop-staff.png"),
];

/// Gold, from deep bronze (0) through bright gold to pale highlights (1): the same ramp the
/// artwork was made with, so the live drawing matches it.
pub fn ramp(v: f32) -> [u8; 3] {
    const STOPS: [(f32, [f32; 3]); 5] = [
        (0.0, [8.0, 5.0, 2.0]),
        (0.22, [80.0, 46.0, 8.0]),
        (0.5, [186.0, 128.0, 28.0]),
        (0.75, [244.0, 198.0, 88.0]),
        (1.0, [255.0, 243.0, 205.0]),
    ];
    let v = v.clamp(0.0, 1.0);
    for w in STOPS.windows(2) {
        let ((a, ca), (b, cb)) = (w[0], w[1]);
        if v <= b {
            let t = (v - a) / (b - a);
            let m = |i: usize| (ca[i] + (cb[i] - ca[i]) * t).round() as u8;
            return [m(0), m(1), m(2)];
        }
    }
    [255, 243, 205]
}

/// How many sparkles drift about.
pub const SPARKLES: usize = 22;
/// Seconds between the start of one glint across the banner and the next.
pub const GLINT_EVERY: f32 = 7.0;
/// Seconds a glint takes to cross the banner.
pub const GLINT_TAKES: f32 = 1.6;
/// How many sun rays are drawn from the top of the window.
pub const RAYS: usize = 14;

pub struct Sparkle {
    /// Fractions of the width and height.
    pub x: f32,
    pub y: f32,
    /// Half the length of the star's arms, in points.
    pub size: f32,
    /// 0..1 how bright it is right now (twinkles).
    pub glow: f32,
}

/// A deterministic 0..1 value for sparkle `n`, stream `k`.
fn hash(n: usize, k: u32) -> f32 {
    let mut h = (n as u32).wrapping_mul(0x9E37_79B1) ^ k.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0xFFFF) as f32 / 65535.0
}

/// The sparkles at time `t` (seconds). Each rises slowly from the bottom, sways a little, and
/// fades in and out over its life, so the picture never repeats in lockstep.
pub fn sparkles(t: f32) -> Vec<Sparkle> {
    (0..SPARKLES)
        .map(|n| {
            let life = 9.0 + 9.0 * hash(n, 1);
            let age = (t / life + hash(n, 2)).fract();
            let x = hash(n, 3) + 0.018 * (t * (0.35 + 0.4 * hash(n, 4)) + hash(n, 5) * TAU).sin();
            let y = 1.05 - 1.1 * age;
            // fade in, hold, fade out, with a quick twinkle on top
            let envelope = (age * (1.0 - age) * 4.0).clamp(0.0, 1.0);
            let twinkle = 0.65 + 0.35 * (t * (1.5 + 2.5 * hash(n, 6)) + hash(n, 7) * TAU).sin();
            Sparkle {
                x: x.clamp(0.0, 1.0),
                y,
                size: 2.5 + 4.5 * hash(n, 8),
                glow: (envelope * twinkle).clamp(0.0, 1.0),
            }
        })
        .collect()
}

/// Where the glint is on the banner at time `t`: 0 at the left edge to 1 at the right edge, or
/// `None` between sweeps.
pub fn glint(t: f32) -> Option<f32> {
    let phase = t.rem_euclid(GLINT_EVERY);
    if phase < GLINT_TAKES {
        let x = phase / GLINT_TAKES;
        // ease in and out so it speeds up in the middle
        Some(x * x * (3.0 - 2.0 * x))
    } else {
        None
    }
}

/// The brightness (0.6..1) of sun ray `n` at time `t`; neighbouring rays shimmer out of step.
pub fn ray_shimmer(n: usize, t: f32) -> f32 {
    0.8 + 0.2 * (t * 0.7 + n as f32 * 1.3 + hash(n, 9) * TAU).sin()
}

/// The angle of ray `n` in radians from straight down, fanned across the window.
pub fn ray_angle(n: usize) -> f32 {
    let spread = 1.25;
    (n as f32 / (RAYS - 1) as f32 * 2.0 - 1.0) * spread
}

// ---- the glitch: the whole window breaks into pixels for a moment ----

/// How long the glitch lasts, in seconds.
pub const GLITCH_LEN: f32 = 1.5;
/// How many different glitch pictures there are per second (it jumps, it does not glide).
const GLITCH_FPS: f32 = 14.0;

/// A strip of the window pushed sideways. Fractions of the window height and width.
#[derive(Debug, Clone, PartialEq)]
pub struct Band {
    pub top: f32,
    pub bottom: f32,
    pub shift: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlitchFrame {
    /// Show the whole window as big blocks of this many screen pixels (None: not this frame).
    pub block: Option<u32>,
    pub bands: Vec<Band>,
    /// 0..1 a flash of gold laid over the window.
    pub flash: f32,
}

/// The block sizes the pixelated pictures are made in, coarse to fine.
pub const BLOCKS: [u32; 3] = [28, 14, 7];

/// What the glitch looks like `t` seconds after it began; `None` once it is over. Everything is a
/// function of `t`, so it can be tested and replays the same way every time.
pub fn glitch_frame(t: f32) -> Option<GlitchFrame> {
    if !(0.0..GLITCH_LEN).contains(&t) {
        return None;
    }
    // 0 to 1 and back: the glitch builds, peaks about a third of the way in, and dies away
    let x = t / GLITCH_LEN;
    let strength = if x < 0.35 { x / 0.35 } else { 1.0 - (x - 0.35) / 0.65 };
    Some(glitch_frame_with(t, strength))
}

/// One picture of a glitch of the given `strength` (0 to 1) at time `t` (seconds, only used to pick
/// which of the jumping pictures it is). The glitch of The Void's moon is this with a strength that
/// follows the moon around instead of a fixed curve.
pub fn glitch_frame_with(t: f32, strength: f32) -> GlitchFrame {
    let frame = (t.max(0.0) * GLITCH_FPS) as usize;
    let strength = strength.clamp(0.0, 1.0);
    let roll = hash(frame, 21);
    // the pixelated whole-window frames come in at the peak, coarser the stronger it is
    let block = if roll < 0.10 + 0.45 * strength && strength > 0.25 {
        let pick = hash(frame, 22);
        Some(if strength > 0.75 && pick < 0.5 {
            BLOCKS[0]
        } else if pick < 0.7 {
            BLOCKS[1]
        } else {
            BLOCKS[2]
        })
    } else {
        None
    };
    let count = (1.0 + strength * 6.0) as usize;
    let bands = (0..count)
        .map(|k| {
            let top = hash(frame * 8 + k, 23) * 0.95;
            let height = 0.008 + hash(frame * 8 + k, 24) * 0.07 * (0.4 + strength);
            let shift = (hash(frame * 8 + k, 25) - 0.5) * 0.12 * (0.3 + 0.7 * strength);
            Band { top, bottom: (top + height).min(1.0), shift }
        })
        .collect();
    let flash = if hash(frame, 26) < 0.12 * strength { 0.10 + 0.12 * strength } else { 0.0 };
    GlitchFrame { block, bands, flash }
}

/// The picture made of big blocks: every `block` x `block` square of the RGBA image `rgba`
/// (`width` x `height`) becomes one pixel of the result, which is returned as (width, height,
/// RGBA bytes). The colour of a block is the average of its pixels.
pub fn pixelate(width: usize, height: usize, rgba: &[u8], block: usize) -> (usize, usize, Vec<u8>) {
    let block = block.max(1);
    let (w, h) = (width.div_ceil(block), height.div_ceil(block));
    let mut out = vec![0u8; w * h * 4];
    for by in 0..h {
        for bx in 0..w {
            let mut sum = [0u32; 4];
            let mut n = 0u32;
            for y in (by * block)..((by + 1) * block).min(height) {
                for x in (bx * block)..((bx + 1) * block).min(width) {
                    let i = (y * width + x) * 4;
                    for c in 0..4 {
                        sum[c] += u32::from(rgba[i + c]);
                    }
                    n += 1;
                }
            }
            if n > 0 {
                let o = (by * w + bx) * 4;
                for c in 0..4 {
                    out[o + c] = (sum[c] / n) as u8;
                }
            }
        }
    }
    (w, h, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_the_right_number_of_sparkles_all_in_range() {
        for t in [0.0, 3.3, 100.0, 3599.9] {
            let list = sparkles(t);
            assert_eq!(list.len(), SPARKLES);
            for s in &list {
                assert!((0.0..=1.0).contains(&s.x));
                assert!((0.0..=1.0).contains(&s.glow));
                assert!(s.size >= 2.5 && s.size <= 7.0);
            }
        }
    }

    #[test]
    fn sparkles_rise_and_are_not_all_the_same() {
        let a = sparkles(10.0);
        let b = sparkles(10.5);
        // a rising sparkle has a smaller y later (unless it just wrapped around)
        let rising = a.iter().zip(&b).filter(|(p, q)| q.y < p.y).count();
        assert!(rising >= SPARKLES - 2, "only {} of {} rose", rising, SPARKLES);
        let first = a[0].x;
        assert!(a.iter().any(|s| (s.x - first).abs() > 0.05));
    }

    #[test]
    fn some_sparkles_are_visible_at_any_moment() {
        for t in (0..400).map(|k| k as f32 * 0.37) {
            let lit = sparkles(t).iter().filter(|s| s.glow > 0.2 && (0.0..=1.0).contains(&s.y)).count();
            assert!(lit >= 4, "only {} lit at t={}", lit, t);
        }
    }

    #[test]
    fn the_glint_sweeps_left_to_right_then_rests() {
        assert_eq!(glint(0.0), Some(0.0));
        let mid = glint(GLINT_TAKES / 2.0).unwrap();
        assert!((mid - 0.5).abs() < 1e-4);
        let near_end = glint(GLINT_TAKES - 0.01).unwrap();
        assert!(near_end > 0.98);
        assert_eq!(glint(GLINT_TAKES + 0.5), None);
        assert_eq!(glint(GLINT_EVERY - 0.1), None);
        assert_eq!(glint(GLINT_EVERY), Some(0.0));
        let mut last = -1.0;
        for k in 0..16 {
            let x = glint(k as f32 * GLINT_TAKES / 16.0).unwrap();
            assert!(x >= last);
            last = x;
        }
    }

    #[test]
    fn rays_fan_out_evenly_and_shimmer_gently() {
        assert!((ray_angle(0) + ray_angle(RAYS - 1)).abs() < 1e-5);
        assert!(ray_angle(0) < ray_angle(1));
        for n in 0..RAYS {
            for t in [0.0, 1.0, 50.0] {
                let s = ray_shimmer(n, t);
                assert!((0.6..=1.0).contains(&s));
            }
        }
    }

    #[test]
    fn the_ramp_runs_from_dark_bronze_to_pale_gold() {
        assert_eq!(ramp(0.0), [8, 5, 2]);
        assert_eq!(ramp(1.0), [255, 243, 205]);
        assert_eq!(ramp(0.5), [186, 128, 28]);
        assert_eq!(ramp(-3.0), ramp(0.0));
        assert_eq!(ramp(9.0), ramp(1.0));
        let mut last = 0u32;
        for k in 0..=20 {
            let c = ramp(k as f32 / 20.0);
            let sum = u32::from(c[0]) + u32::from(c[1]) + u32::from(c[2]);
            assert!(sum >= last);
            last = sum;
        }
    }

    #[test]
    fn the_glitch_ends_and_is_the_same_every_time() {
        assert!(glitch_frame(-0.1).is_none());
        assert!(glitch_frame(GLITCH_LEN).is_none());
        assert!(glitch_frame(GLITCH_LEN + 5.0).is_none());
        for k in 0..60 {
            let t = k as f32 * GLITCH_LEN / 60.0;
            assert_eq!(glitch_frame(t), glitch_frame(t));
            assert!(glitch_frame(t).is_some());
        }
    }

    #[test]
    fn the_glitch_has_tears_and_a_few_pixelated_frames_but_not_too_many() {
        let mut pixelated = 0;
        let frames = (GLITCH_LEN * GLITCH_FPS) as usize;
        for k in 0..frames {
            let f = glitch_frame((k as f32 + 0.5) / GLITCH_FPS).unwrap();
            assert!(!f.bands.is_empty());
            for b in &f.bands {
                assert!(b.top >= 0.0 && b.bottom <= 1.0 && b.top < b.bottom);
                assert!(b.shift.abs() <= 0.08);
            }
            assert!((0.0..=0.25).contains(&f.flash));
            if let Some(block) = f.block {
                assert!(BLOCKS.contains(&block));
                pixelated += 1;
            }
        }
        assert!(pixelated >= 2, "only {} pixelated frames", pixelated);
        assert!(pixelated * 10 <= frames * 6, "too many pixelated frames: {}", pixelated);
    }

    #[test]
    fn the_glitch_is_slight_at_the_start_and_end() {
        let first = glitch_frame(0.0).unwrap();
        assert!(first.block.is_none());
        assert!(first.bands.len() <= 2);
        let last = glitch_frame(GLITCH_LEN - 0.01).unwrap();
        assert!(last.block.is_none());
    }

    #[test]
    fn blocks_average_their_pixels() {
        // 4 x 2 image: left 2x2 block black/white pairs, right block all red
        let mut rgba = Vec::new();
        for y in 0..2 {
            for x in 0..4 {
                let px: [u8; 4] = if x < 2 {
                    if (x + y) % 2 == 0 { [0, 0, 0, 255] } else { [200, 100, 50, 255] }
                } else {
                    [255, 0, 0, 255]
                };
                rgba.extend_from_slice(&px);
            }
        }
        let (w, h, out) = pixelate(4, 2, &rgba, 2);
        assert_eq!((w, h), (2, 1));
        assert_eq!(&out[0..4], &[100, 50, 25, 255]);
        assert_eq!(&out[4..8], &[255, 0, 0, 255]);
    }

    #[test]
    fn blocks_cover_an_image_that_does_not_divide_evenly() {
        let rgba = vec![9u8; 5 * 3 * 4];
        let (w, h, out) = pixelate(5, 3, &rgba, 2);
        assert_eq!((w, h), (3, 2));
        assert_eq!(out.len(), 3 * 2 * 4);
        assert!(out.iter().all(|&b| b == 9));
        let (w1, h1, same) = pixelate(5, 3, &rgba, 1);
        assert_eq!((w1, h1), (5, 3));
        assert_eq!(same, rgba);
    }

    #[test]
    fn a_glitch_of_a_given_strength_is_the_same_every_time_and_gets_worse_with_strength() {
        for k in 0..60 {
            let t = k as f32 / GLITCH_FPS;
            assert_eq!(glitch_frame_with(t, 0.6), glitch_frame_with(t, 0.6));
        }
        // the old timeline is this with its curve
        let t = 0.4;
        let x = t / GLITCH_LEN / 0.35;
        assert_eq!(glitch_frame(t).unwrap(), glitch_frame_with(t, x));
        // nothing pixelated or flashing when it is only just starting, more and more as it grows
        let frames = |s: f32| (0..200).map(|k| glitch_frame_with(k as f32 / GLITCH_FPS, s)).collect::<Vec<_>>();
        assert!(frames(0.1).iter().all(|f| f.block.is_none()));
        let blocks = |s: f32| frames(s).iter().filter(|f| f.block.is_some()).count();
        assert!(blocks(0.5) > 0 && blocks(1.0) > blocks(0.5));
        let bands = |s: f32| frames(s).iter().map(|f| f.bands.len()).sum::<usize>();
        assert!(bands(1.0) > bands(0.5) && bands(0.5) > bands(0.1));
        // strength outside 0..1 is held to it
        assert_eq!(glitch_frame_with(1.0, 7.0), glitch_frame_with(1.0, 1.0));
        assert_eq!(glitch_frame_with(1.0, -3.0), glitch_frame_with(1.0, 0.0));
    }
}
