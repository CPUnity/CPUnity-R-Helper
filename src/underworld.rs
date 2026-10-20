// The Underworld look, as numbers only: the outlines of the rock slabs, the lava and rock
// pictures made from noise, the bubbles, the souls that rise in spurts, and the skeletons on the
// causeway. Nothing here draws; `ui/underworld.rs` does that with what this file works out.
// (No egui in this file, so everything in it can be tested without a window.)

use std::f32::consts::TAU;

/// How far the jagged edge of a slab reaches beyond the card it sits behind, so the spikes stand
/// in the gap between two cards instead of cutting into the text.
pub const ZONE_SIDE: f32 = 6.0;
pub const ZONE_TOP: f32 = 7.0;
pub const ZONE_BOTTOM: f32 = 9.0;
/// The gap between two cards while the Underworld look is on (lava shows in it).
pub const CARD_GAP: f32 = 24.0;
/// The size of the square pictures the lava and the rock are made as; they repeat.
pub const TEXTURE_SIZE: usize = 192;

/// A small, quick random number generator (splitmix64), so every slab, bubble and soul comes out
/// the same each time it is made from the same seed.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// A number from 0 up to (not including) 1.
    pub fn unit(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) / ((1u64 << 24) as f32)
    }
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
}

// ---- the slabs ----

/// The jagged outline of a rock slab `w` wide and `h` high (points in order, clockwise on the
/// screen, starting at the top left). Spikes point outwards along all four sides, as far as the
/// `ZONE_*` margins allow, so give it the card's size plus those margins on each side.
pub fn rock_outline(seed: u64, w: f32, h: f32) -> Vec<(f32, f32)> {
    let mut r = Rng::new(seed);
    let mut out: Vec<(f32, f32)> = Vec::new();
    let w = w.max(40.0);
    let h = h.max(40.0);
    let top_deep = ZONE_TOP + 9.0;
    let bottom_deep = ZONE_BOTTOM + 11.0;
    let side_deep = ZONE_SIDE + 7.0;
    let step_unit = w / 100.0;
    // the top, left to right: sharp peaks at the edge, valleys cut in between them
    let mut x = 0.0_f32;
    let mut peak = true;
    while x < w - step_unit {
        let y = if peak {
            r.range(0.0, 3.0)
        } else {
            let chip = if r.unit() < 0.2 { 3.0 } else { 0.0 };
            (ZONE_TOP + r.range(0.0, top_deep - ZONE_TOP) + chip).min(top_deep + 3.0)
        };
        out.push((x, y));
        x += if peak { r.range(1.0, 4.0) * step_unit } else { r.range(1.5, 6.0) * step_unit };
        peak = !peak;
    }
    out.push((w, r.range(1.0, 5.0)));
    // the right side, top to bottom
    let rows = 12;
    for i in 1..rows {
        let peak = i % 2 == 1;
        let inset = if peak { r.range(0.0, 3.0) } else { r.range(ZONE_SIDE, side_deep) };
        let y = (i as f32 / rows as f32) * h + r.range(-0.015, 0.015) * h;
        out.push((w - inset, y));
    }
    // the bottom, right to left
    out.push((w, h - r.range(1.0, 6.0)));
    let mut x = w;
    let mut peak = true;
    while x > step_unit {
        let y = if peak {
            r.range(0.0, 4.0)
        } else {
            let chip = if r.unit() < 0.2 { 4.0 } else { 0.0 };
            (ZONE_BOTTOM + r.range(0.0, bottom_deep - ZONE_BOTTOM) + chip).min(bottom_deep + 4.0)
        };
        out.push((x, h - y));
        x -= if peak { r.range(1.0, 3.2) * step_unit } else { r.range(1.5, 5.0) * step_unit };
        peak = !peak;
    }
    out.push((0.0, h - r.range(2.0, 7.0)));
    // the left side, bottom to top
    for i in (1..rows).rev() {
        let peak = i % 2 == 1;
        let inset = if peak { r.range(0.0, 3.0) } else { r.range(ZONE_SIDE, side_deep) };
        let y = (i as f32 / rows as f32) * h + r.range(-0.015, 0.015) * h;
        out.push((inset, y));
    }
    out
}

/// The area a closed outline encloses (positive when it runs clockwise on the screen).
pub fn outline_area(points: &[(f32, f32)]) -> f32 {
    let n = points.len();
    if n < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..n {
        let (x1, y1) = points[i];
        let (x2, y2) = points[(i + 1) % n];
        sum += x1 * y2 - x2 * y1;
    }
    sum * 0.5
}

/// A flat plane on the face of a slab: a triangle, lighter or darker than the rock around it.
#[derive(Clone, Debug, PartialEq)]
pub struct Facet {
    pub points: [(f32, f32); 3],
    pub light: bool,
}

/// `count` facets spread over a slab `w` by `h` (kept inside it, away from the spiky edge).
pub fn facets(seed: u64, w: f32, h: f32, count: usize) -> Vec<Facet> {
    let mut r = Rng::new(seed ^ 0xFACE_7);
    let mut out = Vec::new();
    let (mx, my) = (ZONE_SIDE + 4.0, ZONE_TOP + 4.0);
    let (iw, ih) = ((w - 2.0 * mx).max(10.0), (h - 2.0 * my).max(10.0));
    for i in 0..count {
        let left = mx + r.range(0.0, 0.7) * iw;
        let top = my + r.range(0.0, 0.55) * ih;
        let fw = r.range(0.22, 0.5) * iw;
        let fh = r.range(0.35, 0.9) * ih;
        let p = |fx: f32, fy: f32| ((left + fx * fw).min(w - mx), (top + fy * fh).min(h - my));
        let points = [p(r.range(0.0, 0.4), 0.0), p(1.0, r.range(0.0, 0.6)), p(r.range(0.0, 1.0), 1.0)];
        out.push(Facet { points, light: i % 2 == 0 });
    }
    out
}

/// `count` cracks: jagged lines running mostly downwards across a slab `w` by `h`.
pub fn cracks(seed: u64, w: f32, h: f32, count: usize) -> Vec<Vec<(f32, f32)>> {
    let mut r = Rng::new(seed ^ 0xC8AC_4);
    let mut out = Vec::new();
    for _ in 0..count {
        let len = r.range(0.45, 0.95) * h;
        let segments = 7;
        let step = len / segments as f32;
        let mut x = r.range(0.06, 0.94) * w;
        let y0 = r.range(0.0, (h - len).max(1.0) * 0.6);
        let mut line = Vec::new();
        for j in 0..=segments {
            line.push((x.clamp(2.0, w - 2.0), y0 + j as f32 * step));
            x += (r.unit() - 0.45) * 14.0;
        }
        out.push(line);
    }
    out
}

/// How much a slab breathes: its outline grows and shrinks a little, as if it were rising and
/// sinking in the lava. Each slab has its own pace (from its seed). Around 1.0.
pub fn breathe(seed: u64, t: f32) -> f32 {
    let mut r = Rng::new(seed ^ 0xB0B);
    let period = r.range(5.0, 8.0);
    let phase = r.unit() * TAU;
    1.0 + 0.006 * (TAU * t / period + phase).sin()
}

/// How far a slab is from where it was laid, in points (right, down): it floats on the lava, drifting
/// a few points each way on two slow swings of its own. `seed` gives each slab its own pace, and the
/// same slab at the same time is always in the same place. What is written on a slab moves with it.
pub fn float(seed: u64, t: f32) -> (f32, f32) {
    let mut r = Rng::new(seed ^ 0xF10A7);
    let (px, py) = (r.range(6.0, 10.0), r.range(4.5, 8.0));
    let (ax, ay) = (r.unit() * TAU, r.unit() * TAU);
    (FLOAT_X * (TAU * t / px + ax).sin(), FLOAT_Y * (TAU * t / py + ay).sin())
}

/// The most a slab floats sideways and up and down, in points. Small beside the gap between slabs,
/// so that two never close up on each other.
pub const FLOAT_X: f32 = 2.5;
pub const FLOAT_Y: f32 = 4.5;

/// How bright the lava under a slab glows, 0.7 to 1.0, pulsing with the breathing.
pub fn glow(seed: u64, t: f32) -> f32 {
    let mut r = Rng::new(seed ^ 0xB0B);
    let period = r.range(5.0, 8.0);
    let phase = r.unit() * TAU;
    0.85 + 0.15 * (TAU * t / period + phase + 1.1).sin()
}

// ---- the pictures made from noise (they repeat, so they can scroll) ----

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// One layer of smooth noise on a grid that wraps round, so the picture repeats without a seam.
fn value_noise(size: usize, cells: usize, seed: u64) -> Vec<f32> {
    let cells = cells.max(1);
    let mut r = Rng::new(seed);
    let lattice: Vec<f32> = (0..cells * cells).map(|_| r.unit()).collect();
    let at = |cx: usize, cy: usize| lattice[(cy % cells) * cells + (cx % cells)];
    let mut out = vec![0.0; size * size];
    let scale = cells as f32 / size as f32;
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 * scale;
            let fy = y as f32 * scale;
            let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
            let (tx, ty) = (smooth(fx - fx.floor()), smooth(fy - fy.floor()));
            let top = at(x0, y0) * (1.0 - tx) + at(x0 + 1, y0) * tx;
            let bottom = at(x0, y0 + 1) * (1.0 - tx) + at(x0 + 1, y0 + 1) * tx;
            out[y * size + x] = top * (1.0 - ty) + bottom * ty;
        }
    }
    out
}

/// Several layers of noise, each finer and fainter than the one before; values stay in 0 to 1.
pub fn fractal_noise(size: usize, base_cells: usize, octaves: usize, seed: u64) -> Vec<f32> {
    let mut out = vec![0.0; size * size];
    let mut total = 0.0;
    let mut amp = 1.0;
    let mut cells = base_cells.max(1);
    for o in 0..octaves.max(1) {
        let layer = value_noise(size, cells, seed.wrapping_add(o as u64 * 7919));
        for (acc, v) in out.iter_mut().zip(layer.iter()) {
            *acc += v * amp;
        }
        total += amp;
        amp *= 0.5;
        cells *= 2;
        if cells > size {
            break;
        }
    }
    for v in out.iter_mut() {
        *v /= total;
    }
    out
}

/// The crust that floats on the lava, as an RGBA picture: dark plates with glowing, see-through
/// seams between them (the molten lava shows through those), the plate edges burning red.
pub fn lava_crust_rgba(size: usize, seed: u64) -> Vec<[u8; 4]> {
    let n = fractal_noise(size, 4, 4, seed);
    n.iter()
        .map(|&v| {
            let d = (v - 0.5).abs();
            // seam: see-through; rim: glowing; plate: dark crust
            let alpha = ((d - 0.018) / 0.07).clamp(0.0, 1.0);
            let alpha = smooth(alpha);
            let rim = (1.0 - ((d - 0.06) / 0.07).clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let shade = 0.65 + 0.7 * (v - 0.3).clamp(0.0, 0.4);
            let (r, g, b) = (
                (24.0 * shade + 150.0 * rim * alpha).min(255.0),
                (8.0 * shade + 34.0 * rim * alpha).min(255.0),
                (6.0 * shade + 6.0 * rim * alpha).min(255.0),
            );
            [r as u8, g as u8, b as u8, (alpha * 255.0).round() as u8]
        })
        .collect()
}

/// A second, coarser crust, drawn fainter over the first and moving the other way, so the lava
/// seems to flow in more than one direction.
pub fn lava_crust_far_rgba(size: usize, seed: u64) -> Vec<[u8; 4]> {
    lava_crust_rgba(size, seed.wrapping_add(991))
        .into_iter()
        .map(|[r, g, b, a]| [r, g, b, (a as f32 * 0.55) as u8])
        .collect()
}

/// The surface of the rock, as an opaque picture: bumpy, lit from the upper left, in dark
/// browns. It is drawn tinted lighter or darker, so it stays dark enough for text on top.
pub fn rock_rgba(size: usize, seed: u64) -> Vec<[u8; 4]> {
    let height = fractal_noise(size, 6, 5, seed);
    let at = |x: isize, y: isize| -> f32 {
        let s = size as isize;
        height[(y.rem_euclid(s) as usize) * size + x.rem_euclid(s) as usize]
    };
    let mut out = Vec::with_capacity(size * size);
    // light from the upper left, a little above
    let (lx, ly, lz) = (-0.55_f32, -0.6_f32, 0.58_f32);
    for y in 0..size as isize {
        for x in 0..size as isize {
            let dx = (at(x + 1, y) - at(x - 1, y)) * 9.0;
            let dy = (at(x, y + 1) - at(x, y - 1)) * 9.0;
            let len = (dx * dx + dy * dy + 1.0).sqrt();
            let (nx, ny, nz) = (-dx / len, -dy / len, 1.0 / len);
            let lit = (nx * lx + ny * ly + nz * lz).clamp(0.0, 1.0);
            let v = (0.22 + 0.78 * lit).clamp(0.0, 1.0);
            let grain = 0.92 + 0.16 * at(x * 3, y * 3);
            let k = v * grain;
            out.push([(122.0 * k).min(255.0) as u8, (86.0 * k).min(255.0) as u8, (76.0 * k).min(255.0) as u8, 255]);
        }
    }
    out
}

// ---- the bubbles ----

/// A bubble on the lava. `x` and `y` are fractions of the window (0 to 1).
#[derive(Clone, Debug)]
pub struct Bubble {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub period: f32,
    pub phase: f32,
}

pub fn bubbles(seed: u64, count: usize) -> Vec<Bubble> {
    let mut r = Rng::new(seed ^ 0xB0BB1E);
    (0..count)
        .map(|_| Bubble {
            x: r.unit(),
            y: r.unit(),
            radius: r.range(4.5, 13.0),
            period: r.range(3.4, 7.6),
            phase: r.unit(),
        })
        .collect()
}

/// What a bubble looks like at one moment: swelling, then bursting into a ring.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BubbleLook {
    /// Size of the bubble itself, as a fraction of its radius (0 when there is none).
    pub scale: f32,
    pub alpha: f32,
    /// Size of the ring thrown out when it bursts, as a fraction of its radius, and how visible.
    pub ring: f32,
    pub ring_alpha: f32,
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Where a bubble is in its life at time `t` (seconds); it starts over every `period`.
pub fn bubble_look(b: &Bubble, t: f32) -> BubbleLook {
    let p = (t / b.period + b.phase).rem_euclid(1.0);
    let (scale, alpha) = if p < 0.08 {
        (lerp(0.0, 0.12, p / 0.08), p / 0.08)
    } else if p < 0.74 {
        (lerp(0.12, 1.0, (p - 0.08) / 0.66), 1.0)
    } else if p < 0.82 {
        (lerp(1.0, 1.28, (p - 0.74) / 0.08), 1.0)
    } else if p < 0.88 {
        (lerp(1.28, 1.5, (p - 0.82) / 0.06), 1.0 - (p - 0.82) / 0.06)
    } else {
        (0.0, 0.0)
    };
    let (ring, ring_alpha) = if p < 0.80 {
        (0.0, 0.0)
    } else {
        let k = (p - 0.80) / 0.20;
        (lerp(0.9, 2.7, k), 0.95 * (1.0 - k))
    };
    BubbleLook { scale, alpha, ring, ring_alpha }
}

// ---- the souls ----

/// Where a spurt of souls comes up (fractions of the window), and how it repeats.
#[derive(Clone, Debug)]
pub struct Spurt {
    pub x: f32,
    pub y: f32,
    pub cycle: f32,
    pub offset: f32,
    pub souls: Vec<SoulSeed>,
}

#[derive(Clone, Debug)]
pub struct SoulSeed {
    /// Seconds after the start of the spurt that this soul comes up.
    pub delay: f32,
    /// How far sideways it drifts, in pixels.
    pub drift: f32,
    /// Where it starts, a few pixels off the spurt's middle.
    pub jitter: (f32, f32),
}

/// `count` spurts, each of five souls. The windows' lava channels are not known here, so the
/// places are given: (x, y) as fractions of the window.
pub fn spurts(seed: u64, places: &[(f32, f32)]) -> Vec<Spurt> {
    let mut r = Rng::new(seed ^ 0x5011);
    places
        .iter()
        .map(|&(x, y)| {
            let cycle = r.range(10.0, 13.5);
            let offset = r.range(0.0, cycle);
            let souls = (0..5)
                .map(|k| SoulSeed {
                    delay: k as f32 * 0.16 + r.range(0.0, 0.12),
                    drift: (r.unit() - 0.5) * 34.0,
                    jitter: ((r.unit() - 0.5) * 16.0, (r.unit() - 0.5) * 8.0),
                })
                .collect();
            Spurt { x, y, cycle, offset, souls }
        })
        .collect()
}

/// A soul at one moment: how far it has risen and drifted (pixels, up is negative), its size
/// (1 is full), and how visible it is. `None` while it is not there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoulLook {
    pub dx: f32,
    pub dy: f32,
    pub scale: f32,
    pub alpha: f32,
    /// The ring on the lava where it comes up: size and how visible (0 once it is gone).
    pub splash: f32,
    pub splash_alpha: f32,
}

pub fn soul_look(spurt: &Spurt, soul: &SoulSeed, t: f32) -> Option<SoulLook> {
    let p = ((t + spurt.offset - soul.delay) / spurt.cycle).rem_euclid(1.0);
    // (progress, sideways share, up in pixels, size, visibility)
    const KEYS: [(f32, f32, f32, f32, f32); 6] = [
        (0.00, 0.0, 0.0, 0.35, 0.0),
        (0.03, 0.0, -3.0, 0.4, 1.0),
        (0.14, 0.4, -34.0, 0.7, 1.0),
        (0.26, -0.3, -72.0, 1.0, 1.0),
        (0.36, 1.0, -108.0, 1.15, 0.75),
        (0.42, 1.0, -124.0, 1.2, 0.0),
    ];
    let splash = if p < 0.09 {
        let k = p / 0.09;
        (lerp(0.2, 2.6, k), if k < 0.1 { k * 9.5 } else { 0.95 * (1.0 - (k - 0.1) / 0.9) })
    } else {
        (0.0, 0.0)
    };
    if p >= 0.42 {
        return None;
    }
    let mut i = 0;
    while i + 2 < KEYS.len() && p >= KEYS[i + 1].0 {
        i += 1;
    }
    let (a, b) = (KEYS[i], KEYS[i + 1]);
    let k = ((p - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
    Some(SoulLook {
        dx: lerp(a.1, b.1, k) * soul.drift,
        dy: lerp(a.2, b.2, k),
        scale: lerp(a.3, b.3, k),
        alpha: lerp(a.4, b.4, k),
        splash: splash.0,
        splash_alpha: splash.1,
    })
}

// ---- the causeway ----

/// One bone of a skeleton, a line between two points in a box 30 wide and 56 high, as thick as `w`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bone {
    pub a: (f32, f32),
    pub b: (f32, f32),
    pub w: f32,
}

/// Where the skull is in that box, and its size across.
pub const SKULL: ((f32, f32), (f32, f32)) = ((17.0, 7.0), (5.2, 5.6));
/// Where the eyes are (they glow) and where the chest is (where the carried heart hangs).
pub const EYES: [(f32, f32); 2] = [(15.6, 6.2), (19.4, 6.2)];
pub const CHEST: (f32, f32) = (14.5, 20.0);

/// The bones of a walking skeleton in one of its two steps (0 or 1); bent a little forward.
pub fn skeleton(step: usize) -> Vec<Bone> {
    let lean = 7.0_f32.to_radians();
    let (cx, cy) = (15.0_f32, 30.0_f32);
    let rot = |p: (f32, f32)| -> (f32, f32) {
        let (dx, dy) = (p.0 - cx, p.1 - cy);
        (cx + dx * lean.cos() - dy * lean.sin(), cy + dx * lean.sin() + dy * lean.cos())
    };
    let line = |a: (f32, f32), b: (f32, f32), w: f32| Bone { a: rot(a), b: rot(b), w };
    let mut bones = vec![
        // the neck and spine
        line((17.0, 12.0), (15.0, 14.0), 3.0),
        line((15.0, 14.0), (13.8, 21.0), 1.9),
        line((13.8, 21.0), (14.5, 33.0), 1.9),
        // the ribs
        line((15.0, 17.0), (8.5, 19.0), 1.3),
        line((15.0, 20.5), (9.5, 22.8), 1.3),
        line((15.0, 24.0), (11.0, 26.0), 1.3),
        line((15.0, 17.0), (21.0, 19.0), 1.3),
        line((15.0, 20.5), (20.0, 23.0), 1.3),
        // the hip
        line((10.5, 34.0), (18.5, 34.0), 3.4),
    ];
    if step == 0 {
        bones.push(line((10.0, 15.5), (6.5, 25.0), 1.4));
        bones.push(line((6.5, 25.0), (8.0, 35.0), 1.4));
        bones.push(line((19.0, 15.5), (22.5, 23.0), 1.4));
        bones.push(line((22.5, 23.0), (21.0, 33.0), 1.4));
        bones.push(line((12.6, 37.0), (9.4, 46.5), 1.5));
        bones.push(line((9.4, 46.5), (11.6, 55.0), 1.5));
        bones.push(line((16.4, 37.0), (20.4, 45.5), 1.5));
        bones.push(line((20.4, 45.5), (17.6, 55.0), 1.5));
    } else {
        bones.push(line((10.0, 15.5), (8.0, 26.0), 1.4));
        bones.push(line((8.0, 26.0), (9.5, 35.0), 1.4));
        bones.push(line((19.0, 15.5), (21.0, 25.0), 1.4));
        bones.push(line((21.0, 25.0), (23.0, 33.0), 1.4));
        bones.push(line((12.6, 37.0), (14.8, 46.5), 1.5));
        bones.push(line((14.8, 46.5), (11.6, 55.0), 1.5));
        bones.push(line((16.4, 37.0), (15.8, 46.0), 1.5));
        bones.push(line((15.8, 46.0), (20.4, 55.0), 1.5));
    }
    bones
}

/// The skeletons of the line on the causeway at time `t`: where each stands across a band
/// `width` wide (it can be a little past either edge, where they fade), and which step it is on.
/// They walk to the right at `speed` pixels a second, `spacing` apart, and never run out.
pub fn walkers(t: f32, width: f32, spacing: f32, speed: f32) -> Vec<(f32, usize)> {
    let spacing = spacing.max(8.0);
    let shift = (t * speed).rem_euclid(spacing);
    let count = (width / spacing).ceil() as i32 + 3;
    (0..count)
        .map(|i| {
            let x = -spacing + i as f32 * spacing + shift;
            // the step changes about three times a second, each skeleton out of step with its neighbour
            let step = (((t * 2.6 + i as f32 * 0.37).floor() as i64).rem_euclid(2)) as usize;
            (x, step)
        })
        .collect()
}

/// How visible a skeleton is at `x` across a band `width` wide: they fade in and out at the ends.
pub fn walker_fade(x: f32, width: f32) -> f32 {
    let edge = (width * 0.14).max(1.0);
    let k = (x / edge).min((width - x) / edge);
    k.clamp(0.0, 1.0)
}

/// Where the one that carries the heart is: it crosses the band slowly and starts again.
pub fn bearer_x(t: f32, width: f32, speed: f32) -> f32 {
    let span = width + 100.0;
    -50.0 + (t * speed).rem_euclid(span)
}

/// A beat of the carried heart: 1.0 at rest, up to 1.25 on the beat (a double beat, like the logo's).
pub fn heartbeat(t: f32) -> f32 {
    let p = (t / 1.4).rem_euclid(1.0);
    let bump = |c: f32, w: f32, h: f32| -> f32 {
        let d = (p - c).abs();
        if d < w { h * (1.0 - d / w) } else { 0.0 }
    };
    1.0 + bump(0.12, 0.1, 0.22) + bump(0.36, 0.1, 0.14)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_makes_the_same_slab() {
        assert_eq!(rock_outline(7, 400.0, 150.0), rock_outline(7, 400.0, 150.0));
        assert_ne!(rock_outline(7, 400.0, 150.0), rock_outline(8, 400.0, 150.0));
    }

    #[test]
    fn a_slab_outline_stays_inside_its_box_and_has_many_spikes() {
        for seed in 0..40u64 {
            for &(w, h) in &[(520.0_f32, 170.0_f32), (300.0, 90.0), (560.0, 260.0)] {
                let o = rock_outline(seed, w, h);
                assert!(o.len() > 40, "{} points", o.len());
                for &(x, y) in &o {
                    assert!(x >= -0.01 && x <= w + 0.01, "x {x} outside 0..{w}");
                    assert!(y >= -0.01 && y <= h + 0.01, "y {y} outside 0..{h}");
                }
                // it fills most of the box, but is not a plain rectangle
                let area = outline_area(&o);
                assert!(area > 0.0, "runs clockwise");
                assert!(area > 0.72 * w * h && area < 0.995 * w * h, "area {area} of {}", w * h);
            }
        }
    }

    #[test]
    fn the_spikes_stay_out_of_the_text() {
        // the deepest valley cut into the top, bottom and sides stays within the margins the
        // cards leave (a few pixels over is the point of a spike, but not more)
        for seed in 0..60u64 {
            let (w, h) = (500.0_f32, 160.0_f32);
            let o = rock_outline(seed, w, h);
            let deepest_top = o.iter().filter(|p| p.1 < h * 0.5 && p.0 > 20.0 && p.0 < w - 20.0).map(|p| p.1).fold(0.0, f32::max);
            let deepest_bottom = o.iter().filter(|p| p.1 > h * 0.5 && p.0 > 20.0 && p.0 < w - 20.0).map(|p| h - p.1).fold(0.0, f32::max);
            assert!(deepest_top <= ZONE_TOP + 9.0 + 3.1, "top {deepest_top}");
            assert!(deepest_bottom <= ZONE_BOTTOM + 11.0 + 4.1, "bottom {deepest_bottom}");
        }
    }

    #[test]
    fn facets_and_cracks_stay_on_the_slab() {
        let (w, h) = (500.0, 160.0);
        for seed in 0..30u64 {
            for f in facets(seed, w, h, 5) {
                for &(x, y) in &f.points {
                    assert!(x >= 0.0 && x <= w && y >= 0.0 && y <= h);
                }
            }
            for line in cracks(seed, w, h, 2) {
                assert_eq!(line.len(), 8);
                for &(x, y) in &line {
                    assert!(x >= 0.0 && x <= w && y >= 0.0 && y <= h + 1.0, "({x},{y})");
                }
            }
        }
        let f = facets(3, w, h, 5);
        assert_eq!(f.iter().filter(|f| f.light).count(), 3);
    }

    #[test]
    fn slabs_breathe_a_little_and_each_in_its_own_time() {
        for seed in 0..10u64 {
            for k in 0..200 {
                let t = k as f32 * 0.37;
                let b = breathe(seed, t);
                assert!(b > 0.993 && b < 1.007, "{b}");
                let g = glow(seed, t);
                assert!(g >= 0.69 && g <= 1.01, "{g}");
            }
        }
        assert_ne!(breathe(1, 2.0), breathe(2, 2.0));
    }

    #[test]
    fn noise_repeats_without_a_seam_and_has_variety() {
        let n = fractal_noise(64, 4, 4, 5);
        assert_eq!(n.len(), 64 * 64);
        assert!(n.iter().all(|v| *v >= 0.0 && *v <= 1.0));
        let mean: f32 = n.iter().sum::<f32>() / n.len() as f32;
        let var: f32 = n.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n.len() as f32;
        assert!(var.sqrt() > 0.05, "flat noise: {}", var.sqrt());
        // the right edge runs on into the left edge as smoothly as any two neighbours do
        let mut worst_inside = 0.0_f32;
        for y in 0..64 {
            for x in 0..63 {
                worst_inside = worst_inside.max((n[y * 64 + x] - n[y * 64 + x + 1]).abs());
            }
        }
        for y in 0..64 {
            let seam = (n[y * 64 + 63] - n[y * 64]).abs();
            assert!(seam <= worst_inside * 1.5 + 0.02, "seam {seam} vs {worst_inside}");
        }
        for x in 0..64 {
            let seam = (n[63 * 64 + x] - n[x]).abs();
            assert!(seam <= worst_inside * 1.5 + 0.1, "seam {seam}");
        }
    }

    #[test]
    fn the_crust_has_plates_and_glowing_seams() {
        let c = lava_crust_rgba(TEXTURE_SIZE, 11);
        assert_eq!(c.len(), TEXTURE_SIZE * TEXTURE_SIZE);
        let clear = c.iter().filter(|p| p[3] < 30).count();
        let solid = c.iter().filter(|p| p[3] == 255).count();
        let total = c.len();
        assert!(clear * 100 > total * 3, "too few seams: {clear} of {total}");
        assert!(clear * 100 < total * 40, "too many seams: {clear} of {total}");
        assert!(solid * 100 > total * 40, "too little crust: {solid} of {total}");
        // plates are dark; some of their edges glow red
        assert!(c.iter().any(|p| p[3] == 255 && p[0] > 90 && p[1] < 60));
        assert!(c.iter().filter(|p| p[3] == 255).all(|p| p[0] <= 200));
        let far = lava_crust_far_rgba(TEXTURE_SIZE, 11);
        assert!(far.iter().all(|p| p[3] <= 141));
    }

    #[test]
    fn the_rock_is_opaque_bumpy_and_dark_enough_for_text() {
        let r = rock_rgba(TEXTURE_SIZE, 3);
        assert_eq!(r.len(), TEXTURE_SIZE * TEXTURE_SIZE);
        assert!(r.iter().all(|p| p[3] == 255));
        let lum: Vec<f32> = r.iter().map(|p| (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0).collect();
        let mean = lum.iter().sum::<f32>() / lum.len() as f32;
        let var = lum.iter().map(|l| (l - mean) * (l - mean)).sum::<f32>() / lum.len() as f32;
        assert!(var.sqrt() > 10.0, "flat rock: {}", var.sqrt());
        assert!(mean > 25.0 && mean < 110.0, "mean brightness {mean}");
        assert!(r.iter().all(|p| p[0] >= p[2]), "warm, not blue");
    }

    #[test]
    fn a_bubble_swells_bursts_and_throws_a_ring() {
        let b = Bubble { x: 0.5, y: 0.5, radius: 10.0, period: 5.0, phase: 0.0 };
        let at = |p: f32| bubble_look(&b, p * 5.0);
        assert_eq!(at(0.0).alpha, 0.0);
        assert!(at(0.05).scale < at(0.5).scale);
        assert!((at(0.5).scale - 0.12 - (1.0 - 0.12) * ((0.5 - 0.08) / 0.66)).abs() < 1e-4);
        assert!(at(0.78).scale > 1.0, "it pops outwards");
        assert!(at(0.85).alpha < 1.0 && at(0.85).alpha > 0.0);
        assert_eq!(at(0.9).alpha, 0.0);
        assert_eq!(at(0.5).ring_alpha, 0.0);
        assert!(at(0.84).ring_alpha > 0.5);
        assert!(at(0.99).ring > at(0.84).ring);
        assert!(at(0.99).ring_alpha < at(0.84).ring_alpha);
        // starts over
        let (again, first) = (bubble_look(&b, 5.0 + 1.0), bubble_look(&b, 1.0));
        assert!((again.scale - first.scale).abs() < 1e-4 && (again.alpha - first.alpha).abs() < 1e-4);
    }

    #[test]
    fn bubbles_are_spread_out_and_repeatable() {
        let a = bubbles(9, 46);
        let b = bubbles(9, 46);
        assert_eq!(a.len(), 46);
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!((x.x, x.y, x.period), (y.x, y.y, y.period));
            assert!(x.x >= 0.0 && x.x < 1.0 && x.y >= 0.0 && x.y < 1.0);
            assert!(x.radius >= 4.5 && x.radius <= 13.0 && x.period >= 3.4 && x.period <= 7.6);
        }
        let mean_x = a.iter().map(|b| b.x).sum::<f32>() / 46.0;
        assert!(mean_x > 0.3 && mean_x < 0.7);
    }

    #[test]
    fn souls_rise_in_a_spurt_then_rest() {
        let sp = spurts(4, &[(0.2, 0.1), (0.8, 0.5)]);
        assert_eq!(sp.len(), 2);
        let s = &sp[0];
        assert_eq!(s.souls.len(), 5);
        // delays go up one after another, within about a second
        for w in s.souls.windows(2) {
            assert!(w[1].delay > w[0].delay);
        }
        assert!(s.souls.last().unwrap().delay < 1.0);
        let soul = &s.souls[0];
        let mut shown = 0;
        let mut highest = 0.0_f32;
        let mut t = 0.0;
        while t < s.cycle {
            if let Some(l) = soul_look(s, soul, t) {
                shown += 1;
                highest = highest.max(-l.dy);
                assert!(l.alpha >= 0.0 && l.alpha <= 1.0 && l.scale > 0.0);
            }
            t += 0.05;
        }
        let share = shown as f32 * 0.05 / s.cycle;
        assert!(share > 0.3 && share < 0.45, "visible for {share} of the cycle");
        assert!(highest > 100.0, "rises {highest}px");
        // and it is not shown for the rest: the cycle is mostly rest
        assert!(soul_look(s, soul, -s.offset + soul.delay + s.cycle * 0.6).is_none());
        // it begins on the lava and ends higher up
        let start = soul_look(s, soul, -s.offset + soul.delay + 0.001).unwrap();
        let end = soul_look(s, soul, -s.offset + soul.delay + s.cycle * 0.4).unwrap();
        assert!(start.dy > -4.0 && end.dy < -100.0);
        assert!(start.scale < end.scale);
        assert!(start.splash_alpha > 0.0, "a ring on the lava as it comes up");
    }

    #[test]
    fn a_skeleton_has_two_steps_inside_its_box() {
        let a = skeleton(0);
        let b = skeleton(1);
        assert_eq!(a.len(), b.len());
        assert_eq!(a.len(), 17);
        assert_ne!(a, b);
        for bone in a.iter().chain(b.iter()) {
            for p in [bone.a, bone.b] {
                assert!(p.0 > -2.0 && p.0 < 32.0 && p.1 > -2.0 && p.1 < 58.0, "{p:?}");
            }
            assert!(bone.w > 0.5 && bone.w < 4.0);
        }
        // the heart hangs inside the ribs, between the shoulders and the hip
        assert!(CHEST.1 > 14.0 && CHEST.1 < 34.0);
    }

    #[test]
    fn the_line_of_walkers_never_runs_out_and_moves_on() {
        let width = 560.0;
        for k in 0..50 {
            let t = k as f32 * 0.7;
            let w = walkers(t, width, 64.0, 18.0);
            let inside = w.iter().filter(|(x, _)| *x >= 0.0 && *x <= width).count();
            assert!(inside >= 8, "{inside} on the causeway at {t}");
            // evenly spaced
            for pair in w.windows(2) {
                assert!((pair[1].0 - pair[0].0 - 64.0).abs() < 0.01);
            }
        }
        let a = walkers(0.0, width, 64.0, 18.0);
        let b = walkers(1.0, width, 64.0, 18.0);
        assert!((b[0].0 - a[0].0 - 18.0).abs() < 0.01, "walks right at 18 px a second");
        // they take steps: a walker's step changes over a second
        let steps: Vec<usize> = (0..10).map(|k| walkers(k as f32 * 0.2, width, 64.0, 18.0)[3].1).collect();
        assert!(steps.iter().any(|s| *s == 0) && steps.iter().any(|s| *s == 1));
    }

    #[test]
    fn walkers_fade_at_the_ends_and_the_bearer_crosses() {
        assert_eq!(walker_fade(0.0, 560.0), 0.0);
        assert_eq!(walker_fade(560.0, 560.0), 0.0);
        assert_eq!(walker_fade(280.0, 560.0), 1.0);
        assert!(walker_fade(40.0, 560.0) > 0.0 && walker_fade(40.0, 560.0) < 1.0);
        assert!((bearer_x(0.0, 560.0, 18.0) + 50.0).abs() < 0.01);
        let mut x = bearer_x(0.0, 560.0, 18.0);
        let mut wrapped = false;
        for k in 1..400 {
            let n = bearer_x(k as f32, 560.0, 18.0);
            if n < x {
                wrapped = true;
            }
            x = n;
            assert!(x >= -50.0 && x <= 610.0);
        }
        assert!(wrapped, "starts again at the left");
    }

    #[test]
    fn the_carried_heart_beats_twice() {
        let mut max = 1.0_f32;
        let mut beats = 0;
        let mut up = false;
        for k in 0..1400 {
            let v = heartbeat(k as f32 * 0.001);
            max = max.max(v);
            if v > 1.08 && !up {
                beats += 1;
                up = true;
            }
            if v < 1.02 {
                up = false;
            }
        }
        assert_eq!(beats, 2);
        assert!(max > 1.15 && max < 1.3);
    }

    #[test]
    fn slabs_float_a_few_points_each_in_its_own_time() {
        let mut far = (0.0_f32, 0.0_f32);
        for seed in [1000_u64, 8919, 16838] {
            for step in 0..400 {
                let t = step as f32 * 0.5;
                let (x, y) = float(seed, t);
                assert!(x.abs() <= FLOAT_X + 1e-4 && y.abs() <= FLOAT_Y + 1e-4);
                far = (far.0.max(x.abs()), far.1.max(y.abs()));
                assert_eq!(float(seed, t), (x, y), "the same time, the same place");
            }
        }
        assert!(far.0 > FLOAT_X * 0.9 && far.1 > FLOAT_Y * 0.9, "it does reach out: {far:?}");
        assert_ne!(float(1, 3.0), float(2, 3.0));
        assert!(2.0 * FLOAT_Y < CARD_GAP * 0.5, "two neighbours never close up more than half the gap");
    }
}
