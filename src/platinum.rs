//! The Platinum skin's geometry and motion: a cut-gem look. Behind the window a lattice of crystal
//! facets, each glimmering on its own, with a band of light that sweeps across the whole window every
//! few seconds; slow rays from above, a slowly turning rosette, and glints that flare in prism
//! colours. On the cards a bevelled frame, a medallion gem at the top and a gem at each corner that
//! catch the same sweep. Nothing here draws anything, so all of it is tested; the painting is in
//! `ui/backdrop.rs` and `theme.rs`.
use std::f32::consts::TAU;

/// The banner under the title while Platinum is the look (1600x240, baked art).
pub const BANNER_PNG: &[u8] = include_bytes!("../assets/platinum-banner.png");

/// A deterministic 0..1 value for item `n`, stream `k`.
fn hash(n: usize, k: u32) -> f32 {
    let mut h = (n as u32).wrapping_mul(0x9E37_79B1) ^ k.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0xFF_FFFF) as f32 / 0x100_0000 as f32
}

// ---- brushed stainless steel: what the cards are made of ----

/// How tall one brushed streak is, in points.
pub const STEEL_ROW: f32 = 2.0;
/// The steel's colour at the top of a card, in the middle and at the bottom (it is darkest low down).
const STEEL_TOP: [f32; 3] = [68.0, 74.0, 86.0];
const STEEL_MID: [f32; 3] = [54.0, 59.0, 70.0];
const STEEL_LOW: [f32; 3] = [45.0, 49.0, 59.0];
/// How far a streak may go lighter or darker than the steel under it (colour levels), and how far
/// the odd bright one goes.
const STREAK: f32 = 9.0;
const STREAK_BRIGHT: f32 = 14.0;

/// One streak of the brushed steel: from `top` to `bottom` points below the card's top, with the
/// colour at its left end, in the middle and at its right end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SteelRow {
    pub top: f32,
    pub bottom: f32,
    pub colours: [[u8; 3]; 3],
}

/// The plain steel colour `v` of the way down a card (0 at the top, 1 at the bottom).
pub fn steel_base(v: f32) -> [f32; 3] {
    let v = if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
    let (a, b, t) = if v < 0.5 { (STEEL_TOP, STEEL_MID, v * 2.0) } else { (STEEL_MID, STEEL_LOW, v * 2.0 - 1.0) };
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// The streaks of a card `height` points tall, top to bottom, with no gaps between them. The same
/// height always gives the same streaks, so the steel does not shimmer or crawl.
pub fn steel_rows(height: f32) -> Vec<SteelRow> {
    if !(height.is_finite() && height >= 1.0) {
        return Vec::new();
    }
    let n = ((height / STEEL_ROW).round() as usize).max(1);
    let step = height / n as f32;
    (0..n)
        .map(|i| {
            let base = steel_base((i as f32 + 0.5) / n as f32);
            // a streak has its own tone, and drifts a little along its length
            let tone = (hash(i, 11) - 0.5) * 2.0;
            let bright = if hash(i, 14) > 0.92 { STREAK_BRIGHT } else { 0.0 };
            let colours = [12u32, 13, 15].map(|k| {
                let drift = (hash(i, k) - 0.5) * 0.9;
                let shift = (tone * 0.7 + drift).clamp(-1.0, 1.0) * STREAK + bright;
                [0, 1, 2].map(|c| (base[c] + shift).round().clamp(0.0, 255.0) as u8)
            });
            SteelRow { top: i as f32 * step, bottom: (i as f32 + 1.0) * step, colours }
        })
        .collect()
}

/// How far in from the card's left and right edges a streak at `y` points from the top starts and
/// stops, so that the steel keeps to the card's rounded corners.
pub fn corner_inset(y: f32, height: f32, radius: f32) -> f32 {
    if !(radius > 0.0 && height > 0.0 && y.is_finite()) {
        return 0.0;
    }
    let edge = y.min(height - y).max(0.0);
    if edge >= radius {
        return 0.0;
    }
    let up = radius - edge;
    radius - (radius * radius - up * up).max(0.0).sqrt()
}

// ---- the crystal lattice behind the window ----

/// Columns and rows of the lattice (cells; each is two triangles).
pub const COLS: usize = 8;
pub const ROWS: usize = 11;

/// The facets: jittered grid points (fractions of the window's width and height) and the triangles
/// between them, each with a fixed tone (0..1) that decides how bright it is when it glimmers.
#[derive(Debug, Clone, PartialEq)]
pub struct Lattice {
    pub points: Vec<(f32, f32)>,
    pub tris: Vec<[usize; 3]>,
    pub tones: Vec<f32>,
}

fn point_index(col: usize, row: usize) -> usize {
    row * (COLS + 1) + col
}

pub fn lattice() -> Lattice {
    let mut points = Vec::with_capacity((COLS + 1) * (ROWS + 1));
    for row in 0..=ROWS {
        for col in 0..=COLS {
            let n = point_index(col, row);
            let (mut x, mut y) = (col as f32 / COLS as f32, row as f32 / ROWS as f32);
            // the points on the border stay on it; the others wander up to a third of a cell
            if col > 0 && col < COLS {
                x += (hash(n, 1) - 0.5) * 0.66 / COLS as f32;
            }
            if row > 0 && row < ROWS {
                y += (hash(n, 2) - 0.5) * 0.66 / ROWS as f32;
            }
            points.push((x, y));
        }
    }
    let mut tris = Vec::new();
    for row in 0..ROWS {
        for col in 0..COLS {
            let (a, b, c, d) = (point_index(col, row), point_index(col + 1, row), point_index(col, row + 1), point_index(col + 1, row + 1));
            // the diagonal goes whichever way the cell's hash says
            if hash(a, 3) < 0.5 {
                tris.push([a, b, d]);
                tris.push([a, d, c]);
            } else {
                tris.push([a, b, c]);
                tris.push([b, d, c]);
            }
        }
    }
    let tones = (0..tris.len()).map(|i| hash(i, 4)).collect();
    Lattice { points, tris, tones }
}

impl Lattice {
    /// The middle of triangle `i`.
    pub fn centre(&self, i: usize) -> (f32, f32) {
        let [a, b, c] = self.tris[i];
        let (pa, pb, pc) = (self.points[a], self.points[b], self.points[c]);
        ((pa.0 + pb.0 + pc.0) / 3.0, (pa.1 + pb.1 + pc.1) / 3.0)
    }

    /// Every side once, as pairs of point indexes.
    pub fn edges(&self) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = Vec::new();
        for [a, b, c] in &self.tris {
            for (p, q) in [(*a, *b), (*b, *c), (*c, *a)] {
                out.push((p.min(q), p.max(q)));
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// How bright facet `i` glimmers at time `t`: 0..1, each on its own slow beat.
pub fn glimmer(i: usize, t: f32) -> f32 {
    0.5 + 0.5 * (t * (0.25 + 0.5 * hash(i, 5)) + hash(i, 6) * TAU).sin()
}

// ---- the sweep of light ----

/// Seconds from the start of one sweep to the start of the next.
pub const SWEEP_EVERY: f32 = 9.0;
/// Seconds one sweep takes to cross the window.
pub const SWEEP_TAKES: f32 = 2.8;
/// How wide the band of light is, in the same 0..1 as `diagonal`.
pub const SWEEP_WIDTH: f32 = 0.11;

/// Where a point is along the sweep's direction: 0 at the top left corner, 1 at the bottom right
/// (`x` and `y` are fractions of the window).
pub fn diagonal(x: f32, y: f32) -> f32 {
    ((x + 0.6 * y) / 1.6).clamp(0.0, 1.0)
}

/// Where the middle of the band is at time `t`, as a `diagonal` value from just before the top left
/// to just past the bottom right; `None` while it rests between sweeps.
pub fn sweep_position(t: f32) -> Option<f32> {
    let phase = t.rem_euclid(SWEEP_EVERY);
    if phase >= SWEEP_TAKES {
        return None;
    }
    let p = phase / SWEEP_TAKES;
    // it starts and ends gently
    let eased = p * p * (3.0 - 2.0 * p);
    Some(-SWEEP_WIDTH + eased * (1.0 + 2.0 * SWEEP_WIDTH))
}

/// How much of the sweep's light falls on something at `diagonal` value `d`: 0..1.
pub fn sweep_light(d: f32, position: Option<f32>) -> f32 {
    match position {
        None => 0.0,
        Some(p) => {
            let x = (d - p) / SWEEP_WIDTH;
            (-x * x).exp()
        }
    }
}

/// The light on a point at (`x`, `y`) of the window at time `t`.
pub fn light_at(x: f32, y: f32, t: f32) -> f32 {
    sweep_light(diagonal(x, y), sweep_position(t))
}

// ---- rays from above ----

/// How many rays: the odd ones are thin and crisp, the even ones wide and soft.
pub const RAYS: usize = 17;

/// One ray: its angle from straight down (radians), half its width (radians) and how bright (0..1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub angle: f32,
    pub half: f32,
    pub bright: f32,
}

pub fn ray(n: usize, t: f32) -> Ray {
    let base = (n as f32 / (RAYS - 1) as f32 * 2.0 - 1.0) * 1.3;
    let thin = n % 2 == 1;
    Ray {
        // the whole fan breathes a little, and each ray sways on its own
        angle: base + 0.05 * (t * 0.11 + n as f32 * 0.7).sin() + 0.02 * (t * 0.3 + hash(n, 7) * TAU).sin(),
        half: if thin { 0.012 + 0.008 * hash(n, 8) } else { 0.05 + 0.03 * hash(n, 8) },
        bright: (0.7 + 0.3 * (t * 0.6 + n as f32 * 1.1 + hash(n, 9) * TAU).sin()).clamp(0.0, 1.0),
    }
}

// ---- the rosette ----

/// The corners of a star with `points` points, as offsets from its middle in units of its outer
/// radius, turned by `rotation` radians. The inner corners are at `inner` times the radius.
pub fn star(points: usize, inner: f32, rotation: f32) -> Vec<(f32, f32)> {
    (0..points * 2)
        .map(|i| {
            let r = if i % 2 == 0 { 1.0 } else { inner };
            let a = rotation + i as f32 * TAU / (points * 2) as f32;
            (a.sin() * r, -a.cos() * r)
        })
        .collect()
}

/// How far the rosette has turned at time `t`: very slowly.
pub fn rosette_turn(t: f32) -> f32 {
    t * 0.018
}

// ---- glints ----

/// How many glints drift about.
pub const GLINTS: usize = 26;

/// A glint: a flare that twinkles. The colour is picked from `PALETTE` by `tint`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glint {
    pub x: f32,
    pub y: f32,
    /// Half the length of the arms, in points.
    pub size: f32,
    pub glow: f32,
    pub tint: usize,
}

/// The colours a glint can have (r, g, b): white, ice, lilac, mint.
pub const PALETTE: [[u8; 3]; 4] = [[255, 255, 255], [190, 222, 255], [226, 204, 255], [200, 246, 232]];

pub fn glints(t: f32) -> Vec<Glint> {
    (0..GLINTS)
        .map(|n| {
            let life = 8.0 + 9.0 * hash(n, 11);
            let age = (t / life + hash(n, 12)).fract();
            let x = hash(n, 13) + 0.02 * (t * (0.3 + 0.4 * hash(n, 14)) + hash(n, 15) * TAU).sin();
            let y = 1.05 - 1.1 * age;
            let envelope = (age * (1.0 - age) * 4.0).clamp(0.0, 1.0);
            let twinkle = 0.55 + 0.45 * (t * (1.2 + 2.8 * hash(n, 16)) + hash(n, 17) * TAU).sin();
            // a glint that the sweep is passing flares up
            let flare = 0.6 * light_at(x.clamp(0.0, 1.0), y.clamp(0.0, 1.0), t);
            Glint {
                x: x.clamp(0.0, 1.0),
                y,
                size: 3.0 + 5.0 * hash(n, 18),
                glow: (envelope * twinkle + flare * envelope).clamp(0.0, 1.0),
                tint: (hash(n, 19) * PALETTE.len() as f32) as usize % PALETTE.len(),
            }
        })
        .collect()
}

// ---- gems ----

/// The four facets of a cut gem seen from above, as triangles of offsets from its middle (radius
/// `r`) with how bright each is (0..1): the upper left catches the light, the lower right is dark.
pub fn gem_facets(r: f32) -> [([(f32, f32); 3], f32); 4] {
    let (top, right, bottom, left, mid) = ((0.0, -r), (r * 0.78, 0.0), (0.0, r), (-r * 0.78, 0.0), (0.0, 0.0));
    [([top, left, mid], 0.95), ([top, right, mid], 0.7), ([bottom, left, mid], 0.45), ([bottom, right, mid], 0.25)]
}

/// How much a gem is flashing at time `t`: it catches the sweep when it passes, and has a small
/// glimmer of its own besides. `x` and `y` are where it is in the window (fractions), `n` which gem.
pub fn gem_flash(n: usize, x: f32, y: f32, t: f32) -> f32 {
    let own = 0.18 + 0.12 * (t * (0.8 + hash(n, 21)) + hash(n, 22) * TAU).sin();
    (own + 0.9 * light_at(x, y, t)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lattice_covers_the_window_and_keeps_its_border() {
        let l = lattice();
        assert_eq!(l.points.len(), (COLS + 1) * (ROWS + 1));
        assert_eq!(l.tris.len(), COLS * ROWS * 2);
        assert_eq!(l.tones.len(), l.tris.len());
        for (i, (x, y)) in l.points.iter().enumerate() {
            assert!((0.0..=1.0).contains(x) && (0.0..=1.0).contains(y), "point {i} is outside");
            let (col, row) = (i % (COLS + 1), i / (COLS + 1));
            if col == 0 {
                assert_eq!(*x, 0.0);
            }
            if col == COLS {
                assert_eq!(*x, 1.0);
            }
            if row == 0 {
                assert_eq!(*y, 0.0);
            }
            if row == ROWS {
                assert_eq!(*y, 1.0);
            }
        }
        // the triangles add up to the whole window: the area of every one, summed, is 1
        let area: f32 = l
            .tris
            .iter()
            .map(|[a, b, c]| {
                let (pa, pb, pc) = (l.points[*a], l.points[*b], l.points[*c]);
                ((pb.0 - pa.0) * (pc.1 - pa.1) - (pc.0 - pa.0) * (pb.1 - pa.1)).abs() / 2.0
            })
            .sum();
        assert!((area - 1.0).abs() < 1e-3, "{area}");
        assert_eq!(l, lattice(), "the same every time");
        assert!(l.tones.iter().all(|t| (0.0..=1.0).contains(t)));
    }

    #[test]
    fn every_edge_is_listed_once() {
        let l = lattice();
        let edges = l.edges();
        // a grid of cells with one diagonal each: horizontal + vertical + diagonal sides
        let expected = ROWS * COLS + (ROWS + 1) * COLS + ROWS * (COLS + 1);
        assert_eq!(edges.len(), expected);
        assert!(edges.iter().all(|(a, b)| a < b && *b < l.points.len()));
        let c = l.centre(0);
        assert!((0.0..=1.0).contains(&c.0) && (0.0..=1.0).contains(&c.1));
    }

    #[test]
    fn glimmer_stays_in_range_and_differs_between_facets() {
        let mut seen = Vec::new();
        for i in 0..20 {
            for step in 0..40 {
                let g = glimmer(i, step as f32 * 0.7);
                assert!((0.0..=1.0).contains(&g));
            }
            seen.push(glimmer(i, 3.0));
        }
        let (lo, hi) = seen.iter().fold((1.0f32, 0.0f32), |(l, h), v| (l.min(*v), h.max(*v)));
        assert!(hi - lo > 0.4, "they do not all beat together");
    }

    #[test]
    fn the_sweep_crosses_the_window_then_rests() {
        assert!(sweep_position(0.0).is_some_and(|p| p < 0.0), "starts before the top left");
        let late = sweep_position(SWEEP_TAKES - 0.001).unwrap();
        assert!(late > 1.0, "ends past the bottom right: {late}");
        assert_eq!(sweep_position(SWEEP_TAKES + 0.5), None);
        assert_eq!(sweep_position(SWEEP_EVERY - 0.1), None);
        assert!(sweep_position(SWEEP_EVERY + 0.1).is_some(), "and again");
        // it only ever moves forward
        let mut last = f32::MIN;
        for i in 0..28 {
            let p = sweep_position(i as f32 * 0.1).unwrap();
            assert!(p >= last);
            last = p;
        }
    }

    #[test]
    fn the_light_is_brightest_under_the_band_and_gone_away_from_it() {
        assert_eq!(sweep_light(0.5, None), 0.0);
        assert!((sweep_light(0.5, Some(0.5)) - 1.0).abs() < 1e-6);
        assert!(sweep_light(0.5, Some(0.5)) > sweep_light(0.55, Some(0.5)));
        assert!(sweep_light(0.9, Some(0.3)) < 0.001);
        // the top left is lit before the bottom right
        let t = SWEEP_TAKES * 0.2;
        assert!(light_at(0.1, 0.1, t) > light_at(0.9, 0.9, t));
        assert!(diagonal(0.0, 0.0) == 0.0 && diagonal(1.0, 1.0) == 1.0);
        assert!(light_at(0.5, 0.5, SWEEP_TAKES + 1.0) == 0.0, "nothing between sweeps");
    }

    #[test]
    fn the_rays_fan_out_and_alternate_soft_and_crisp() {
        let (first, last) = (ray(0, 0.0), ray(RAYS - 1, 0.0));
        assert!(first.angle < -1.2 && last.angle > 1.2);
        for n in 0..RAYS {
            for step in 0..30 {
                let r = ray(n, step as f32 * 1.3);
                assert!(r.half > 0.0 && r.half < 0.1 && (0.0..=1.0).contains(&r.bright), "ray {n}");
                assert!(r.angle.abs() < 1.45);
            }
            let wide = ray(n, 2.0).half;
            if n % 2 == 1 {
                assert!(wide < 0.03, "odd rays are thin");
            } else {
                assert!(wide >= 0.05, "even rays are wide");
            }
        }
    }

    #[test]
    fn the_star_has_its_corners_in_order_and_turns() {
        let s = star(8, 0.45, 0.0);
        assert_eq!(s.len(), 16);
        assert!((s[0].0).abs() < 1e-6 && (s[0].1 + 1.0).abs() < 1e-6, "the first point is straight up");
        for (i, (x, y)) in s.iter().enumerate() {
            let r = (x * x + y * y).sqrt();
            let expect = if i % 2 == 0 { 1.0 } else { 0.45 };
            assert!((r - expect).abs() < 1e-5, "corner {i}");
        }
        let turned = star(8, 0.45, 0.3);
        assert!(turned[0] != s[0]);
        assert!(rosette_turn(100.0) > rosette_turn(1.0) && rosette_turn(100.0) < 3.0, "slow");
    }

    #[test]
    fn glints_stay_in_the_window_range_and_twinkle() {
        for step in 0..60 {
            let t = step as f32 * 2.3;
            let gs = glints(t);
            assert_eq!(gs.len(), GLINTS);
            for g in &gs {
                assert!((0.0..=1.0).contains(&g.x) && (0.0..=1.0).contains(&g.glow));
                assert!(g.size >= 3.0 && g.size <= 8.0 && g.tint < PALETTE.len());
                assert!(g.y <= 1.05 && g.y >= -0.06, "{}", g.y);
            }
        }
        let tints: std::collections::BTreeSet<usize> = glints(10.0).iter().map(|g| g.tint).collect();
        assert!(tints.len() >= 3, "a mix of colours");
        let lit = |t: f32| glints(t).iter().filter(|g| g.glow > 0.2).count();
        assert!(lit(5.0) >= 4, "some are always showing");
    }

    #[test]
    fn a_gem_has_four_facets_lit_from_the_upper_left() {
        let f = gem_facets(5.0);
        assert!(f[0].1 > f[1].1 && f[1].1 > f[2].1 && f[2].1 > f[3].1);
        for (tri, shade) in f {
            assert!((0.0..=1.0).contains(&shade));
            assert!(tri.iter().all(|(x, y)| x.abs() <= 5.0 && y.abs() <= 5.0));
        }
        // it flashes when the sweep passes and glimmers a little on its own otherwise
        let t = SWEEP_TAKES * 0.5;
        let p = sweep_position(t).unwrap();
        // a point on the band's diagonal: x on it, y = 0
        let x = p * 1.6;
        assert!(x <= 1.0, "{x}");
        assert!(gem_flash(1, x, 0.0, t) > 0.8);
        let rest = gem_flash(1, 0.5, 0.5, SWEEP_TAKES + 2.0);
        assert!(rest > 0.03 && rest < 0.35, "{rest}");
    }

    fn luminance(c: [u8; 3]) -> f32 {
        let lin = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
    }

    fn contrast(a: [u8; 3], b: [u8; 3]) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn the_steel_streaks_fill_the_card_without_gaps_and_never_change() {
        for height in [1.0_f32, 7.0, 40.0, 133.5, 600.0] {
            let rows = steel_rows(height);
            assert!(!rows.is_empty());
            assert_eq!(rows[0].top, 0.0);
            assert!((rows.last().unwrap().bottom - height).abs() < 0.01, "{height}");
            for pair in rows.windows(2) {
                assert!((pair[0].bottom - pair[1].top).abs() < 1e-3, "no gap, no overlap");
            }
            assert_eq!(rows, steel_rows(height), "the same card, the same streaks");
        }
        assert!(steel_rows(0.0).is_empty() && steel_rows(f32::NAN).is_empty() && steel_rows(-4.0).is_empty());
    }

    #[test]
    fn the_steel_is_brushed_lighter_at_the_top_and_still_reads_as_a_surface_for_text() {
        let top = steel_base(0.0);
        let low = steel_base(1.0);
        assert!(top[0] > low[0] && top[2] > low[2], "lit from above");
        assert_eq!(steel_base(f32::NAN), top);
        let rows = steel_rows(300.0);
        // streaks differ from each other, so that it looks brushed rather than flat
        let distinct: std::collections::HashSet<[u8; 3]> = rows.iter().map(|r| r.colours[1]).collect();
        assert!(distinct.len() > 8, "{} tones", distinct.len());
        // and none of them gets so light that the card's text can no longer be read
        let (text, dim) = ([242, 246, 252], [170, 180, 198]);
        for r in &rows {
            for c in r.colours {
                assert!(contrast(text, c) >= 5.5, "text on {c:?}");
                assert!(contrast(dim, c) >= 3.0, "dim text on {c:?}");
            }
        }
    }

    #[test]
    fn the_steel_keeps_to_the_rounded_corners() {
        assert_eq!(corner_inset(0.0, 100.0, 6.0), 6.0, "the very top row starts a whole radius in");
        assert_eq!(corner_inset(100.0, 100.0, 6.0), 6.0);
        assert_eq!(corner_inset(50.0, 100.0, 6.0), 0.0);
        assert_eq!(corner_inset(6.0, 100.0, 6.0), 0.0);
        let mut last = f32::MAX;
        for y in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0] {
            let v = corner_inset(y, 100.0, 6.0);
            assert!(v <= last, "it eases out towards the straight side");
            last = v;
        }
        assert_eq!(corner_inset(1.0, 100.0, 0.0), 0.0);
        assert_eq!(corner_inset(f32::NAN, 100.0, 6.0), 0.0);
    }
}
