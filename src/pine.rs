//! Geometry for the Frost skin's winter forest: pine trees (tiers of boughs with snow on top), the
//! rows of them along the bottom of the window, a night sky, and the snow, icicles and cracks on
//! the cards. Pure, so it is tested; `ui/backdrop.rs` and `theme.rs` draw it.

fn hash(i: u32, salt: u32) -> f32 {
    let mut x = i.wrapping_mul(2_654_435_761).wrapping_add(salt.wrapping_mul(40_503)) | 1;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

pub type Tri = [(f32, f32); 3];

/// One layer of a pine: the main triangle, the boughs that droop out of its sides, and the snow
/// lying on its top.
#[derive(Debug, Clone, PartialEq)]
pub struct Tier {
    pub body: Tri,
    pub boughs: Vec<Tri>,
    pub snow: Tri,
}

/// A pine one unit tall, standing on (0, 0) with its tip at (0, -1) (y goes down). The tiers are
/// listed from the bottom up, so drawing them in order puts the upper ones in front.
#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
    /// The trunk: left, top, right, bottom.
    pub trunk: (f32, f32, f32, f32),
    pub tiers: Vec<Tier>,
}

/// A pine with `tiers` layers (at least 3), its details decided by `seed`.
pub fn tree(seed: u32, tiers: u32) -> Tree {
    let n = tiers.max(3);
    let list = (0..n)
        .map(|j| {
            let f = j as f32 / (n - 1) as f32;
            let base = -0.10 - 0.62 * f;
            let height = 0.40 - 0.12 * f;
            let half = (0.30 - 0.20 * f) * (0.94 + 0.12 * hash(seed, 10 + j));
            let apex = base - height;
            let body = [(0.0, apex), (-half, base), (half, base)];
            let mut boughs = Vec::new();
            for side in [-1.0, 1.0] {
                // one drooping out of the bottom corner, one halfway up
                boughs.push([(side * half * 0.72, base - 0.04), (side * half * 1.16, base + 0.035), (side * half * 0.5, base + 0.02)]);
                let mid = base - height * 0.5;
                let w = half * 0.5;
                boughs.push([(side * w * 0.7, mid - 0.02), (side * w * 1.4, mid + 0.04), (side * w * 0.5, mid + 0.03)]);
            }
            let snow = [(0.0, apex), (-half * 0.45, apex + height * 0.45), (half * 0.45, apex + height * 0.45)];
            Tier { body, boughs, snow }
        })
        .collect();
    Tree { trunk: (-0.04, -0.14, 0.04, 0.0), tiers: list }
}

/// A tree standing in a row: how far across (0 to 1), how tall as a fraction of the window's
/// height, and which tree it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Planted {
    pub x: f32,
    pub height: f32,
    pub seed: u32,
    pub tiers: u32,
}

/// `count` trees spread along a row, each a little off from even and each its own height between
/// `low` and `high`. `row` tells one row from another.
pub fn row(count: u32, row: u32, low: f32, high: f32) -> Vec<Planted> {
    (0..count)
        .map(|k| {
            let id = row * 100 + k;
            let x = (k as f32 + 0.5 + (hash(id, 1) - 0.5) * 0.7) / count as f32;
            Planted { x, height: low + (high - low) * hash(id, 2), seed: id, tiers: 4 + (hash(id, 3) * 2.99) as u32 }
        })
        .collect()
}

/// How high the snow lies along the bottom of the window at `x` (0 to 1 across), as a fraction of
/// the window's height: a low rolling drift.
pub fn drift(x: f32) -> f32 {
    0.045 + 0.014 * (x * 7.0 + 1.0).sin() + 0.008 * (x * 17.0).sin()
}

/// A star in the night sky: where (fractions of the window, all in the upper half), how big and
/// where it is in its twinkle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Star {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub phase: f32,
}

pub fn stars(count: u32) -> Vec<Star> {
    (0..count).map(|i| Star { x: hash(i, 21), y: hash(i, 22) * 0.5, size: 0.6 + hash(i, 23) * 1.0, phase: hash(i, 24) * 6.28 }).collect()
}

/// How bright a star is at time `t`, 0.3 to 1.
pub fn twinkle(star: &Star, t: f32) -> f32 {
    0.65 + 0.35 * (t * 0.7 + star.phase).sin()
}

// ---- on the cards ----

/// Where the snow lies in lumps along the top of a card `w` wide: (across, radius).
pub fn snow_lumps(w: f32) -> Vec<(f32, f32)> {
    let seed = w as u32;
    let mut out = Vec::new();
    let mut x = 10.0;
    let mut k = 0;
    while x < w - 10.0 {
        out.push((x, 2.5 + hash(seed, 30 + k) * 2.5));
        x += 8.0 + hash(seed, 60 + k) * 10.0;
        k += 1;
    }
    out
}

/// Icicles hanging off the bottom edge of a `w` by `h` card: (across, length), spread unevenly and
/// kept clear of the right-hand corner, where the pine stands.
pub fn icicles(w: f32, h: f32) -> Vec<(f32, f32)> {
    let seed = (w as u32).wrapping_mul(31).wrapping_add(h as u32);
    let mut out = Vec::new();
    let mut x = 14.0 + hash(seed, 40) * 20.0;
    let mut k = 0;
    while x < w - 40.0 {
        out.push((x, 4.0 + hash(seed, 80 + k) * 8.0));
        x += 18.0 + hash(seed, 120 + k) * 34.0;
        k += 1;
    }
    out
}

/// Cracks running into the bottom left corner of a card `h` tall, as lines from the corner: each is
/// a handful of points, measured from the card's top left.
pub fn cracks(h: f32) -> Vec<Vec<(f32, f32)>> {
    let seed = h as u32;
    (0..3)
        .map(|c| {
            let angle = -0.25 - 0.5 * c as f32 - hash(seed, 150 + c) * 0.25;
            let (dx, dy) = (angle.cos(), angle.sin());
            let len = 22.0 + hash(seed, 160 + c) * 16.0;
            let mut pts = vec![(1.0, h - 1.0)];
            for s in 1..=3 {
                let reach = len * s as f32 / 3.0;
                let wobble = (hash(seed, 170 + c * 4 + s) - 0.5) * 6.0;
                pts.push(((1.0 + dx * reach - dy * wobble).max(1.0), (h - 1.0 + dy * reach + dx * wobble).min(h - 1.0)));
            }
            pts
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(t: &Tri) -> f32 {
        ((t[1].0 - t[0].0) * (t[2].1 - t[0].1) - (t[2].0 - t[0].0) * (t[1].1 - t[0].1)).abs() / 2.0
    }

    #[test]
    fn a_pine_is_one_unit_tall_with_narrower_tiers_going_up() {
        for seed in 0..40 {
            for n in [3, 4, 5, 6] {
                let t = tree(seed, n);
                assert_eq!(t.tiers.len(), n as usize);
                let top = t.tiers.last().unwrap().body[0];
                assert!((top.0).abs() < 1e-6 && (top.1 + 1.0).abs() < 1e-4, "the tip is at (0, -1): {top:?}");
                let widths: Vec<f32> = t.tiers.iter().map(|tier| tier.body[2].0).collect();
                assert!(widths.windows(2).all(|w| w[1] < w[0]), "{widths:?}");
                for tier in &t.tiers {
                    assert!(area(&tier.body) > 0.01);
                    assert!(area(&tier.snow) > 0.0 && area(&tier.snow) < area(&tier.body));
                    assert_eq!(tier.boughs.len(), 4);
                    for bough in &tier.boughs {
                        assert!(area(bough) > 0.0);
                    }
                    for p in tier.boughs.iter().flatten().chain(tier.body.iter()).chain(tier.snow.iter()) {
                        assert!(p.0.abs() < 0.45 && p.1 > -1.01 && p.1 < 0.1, "{p:?}");
                    }
                }
                assert!(t.trunk.0 < t.trunk.2 && t.trunk.1 < t.trunk.3);
            }
        }
        assert_eq!(tree(1, 1).tiers.len(), 3, "at least three tiers");
        assert_eq!(tree(5, 4), tree(5, 4));
        assert_ne!(tree(5, 4), tree(6, 4));
    }

    #[test]
    fn a_row_of_trees_is_spread_across_in_order() {
        let r = row(11, 0, 0.1, 0.17);
        assert_eq!(r.len(), 11);
        assert!(r.windows(2).all(|w| w[1].x > w[0].x));
        assert!(r.iter().all(|p| p.x > 0.0 && p.x < 1.0 && p.height >= 0.1 && p.height <= 0.17 && (4..=6).contains(&p.tiers)));
        assert_ne!(row(11, 0, 0.1, 0.17), row(11, 1, 0.1, 0.17), "another row is another set of trees");
    }

    #[test]
    fn the_drift_is_low_and_always_there() {
        for k in 0..=100 {
            let d = drift(k as f32 / 100.0);
            assert!(d > 0.02 && d < 0.07, "{d}");
        }
    }

    #[test]
    fn the_stars_are_in_the_upper_half_and_twinkle_but_never_go_out() {
        let s = stars(30);
        assert_eq!(s.len(), 30);
        assert!(s.iter().all(|s| s.x >= 0.0 && s.x <= 1.0 && s.y >= 0.0 && s.y <= 0.5 && s.size >= 0.6));
        for star in &s {
            for t in [0.0, 1.3, 7.9] {
                let b = twinkle(star, t);
                assert!(b >= 0.3 - 1e-6 && b <= 1.0 + 1e-6, "{b}");
            }
        }
    }

    #[test]
    fn snow_lies_along_the_top_and_icicles_hang_clear_of_the_pine() {
        for w in [200.0, 380.0, 544.0] {
            let lumps = snow_lumps(w);
            assert!(lumps.len() >= 5);
            assert!(lumps.iter().all(|(x, r)| *x > 0.0 && *x < w && *r >= 2.5 && *r <= 5.0));
            assert!(lumps.windows(2).all(|p| p[1].0 > p[0].0));
            for h in [60.0, 120.0, 260.0] {
                let ic = icicles(w, h);
                assert!(!ic.is_empty());
                assert!(ic.iter().all(|(x, len)| *x > 0.0 && *x < w - 40.0 && *len >= 4.0 && *len <= 12.0));
                assert!(ic.windows(2).all(|p| p[1].0 - p[0].0 >= 18.0), "not bunched");
            }
        }
    }

    #[test]
    fn the_cracks_start_in_the_corner_and_stay_on_the_card() {
        for h in [50.0, 120.0, 300.0] {
            let c = cracks(h);
            assert_eq!(c.len(), 3);
            for line in &c {
                assert_eq!(line.len(), 4);
                assert_eq!(line[0], (1.0, h - 1.0));
                assert!(line.iter().all(|p| p.0 >= 0.0 && p.0 < 46.0 && p.1 > h - 46.0 && p.1 <= h), "{line:?}");
            }
        }
    }
}
