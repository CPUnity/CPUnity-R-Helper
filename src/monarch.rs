//! What the broken staff lets out, kept free of drawing so it can be tested: the timeline of the
//! scene after the moon's strike (the glitch dying away, gold sand stitching the tears shut, a ring
//! of sand turning the clock back, a hole opening in the middle of space, the shadow monarch's voice,
//! his talons, the violet fire, his eyes, and then him) and the shapes he is drawn with.
//!
//! Shapes are lists of points in the figure's own units (about 760 tall, x to the right, y down,
//! the middle of his head at the top), to be placed and scaled by the caller. Filled ones are cut
//! into triangles by `triangulate`, because egui can only fill convex shapes itself.

use std::f32::consts::PI;

pub type Pt = (f32, f32);

/// What he says first, out of the blackness, and then.
pub const LINE1: &str = "You had the patience for time...";
pub const LINE2: &str = "Do you have the patience for death?";
/// Letters a second while a line is spoken.
pub const CPS: f32 = 13.0;
/// The whole scene, in seconds after the moon hit: the end is him, standing there, and then Game 3.
pub const END: f32 = 23.0;
/// How much faster the scene plays than it is written (every time in this file is in written
/// seconds; `scene_time` turns the seconds that really went by into them).
pub const SPEED: f32 = 1.4;
/// The first seconds after the strike are not sped up: that is the staff breaking, which has to be
/// followed.
pub const NORMAL_UNTIL: f32 = 2.4;
/// When it really ends, in seconds after the strike.
pub const REAL_END: f32 = NORMAL_UNTIL + (END - NORMAL_UNTIL) / SPEED;

/// The written time that goes with `real` seconds after the strike.
pub fn scene_time(real: f32) -> f32 {
    if real <= NORMAL_UNTIL {
        real.max(0.0)
    } else {
        NORMAL_UNTIL + (real - NORMAL_UNTIL) * SPEED
    }
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn hash(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B).rotate_left(9);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0xFFFF) as f32 / 65535.0
}

// ---- the timeline --------------------------------------------------------------------------------

/// How far each part of the scene has got, `t` seconds after the moon hit the staff.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Beat {
    /// How much of the gold seams that stitch the glitch's tears shut are drawn (0 to 1).
    pub seams: f32,
    /// The ring of sand that turns the clock back: how visible it is (0 to 1), and how far it has
    /// turned (radians, backwards).
    pub ring: f32,
    pub ring_turn: f32,
    /// How open the hole in the middle of space is (0 to 1).
    pub rift: f32,
    /// How dark the page behind the hole has gone (0 to 0.8).
    pub dim: f32,
    /// His two lines: how many letters are showing, and how visible.
    pub line1: (usize, f32),
    pub line2: (usize, f32),
    /// The talons: how far they have come out (0 to 1) and how far the fingers have uncurled.
    pub talons: f32,
    pub uncurl: f32,
    /// The violet fire in the right hand.
    pub flame: f32,
    /// His eyes opening in the dark.
    pub eyes: f32,
    /// Him, coming out of the dark (0 to 1).
    pub body: f32,
    /// How far he has pulled himself through the hole (0: deep in it, small; 1: standing there): in
    /// three hard lurches with a little rearing back between them, and then the lunge (see `drive`).
    pub pull: f32,
    /// How far into the lunge at you he is (0 to 1): the last snap carries him past standing, right
    /// up at the screen, and he settles back.
    pub over: f32,
    /// How hard the window is being knocked right now (0 to 1), by each lurch and the lunge.
    pub kick: f32,
    /// A burst of glitching (0 to 1) on top of the moon's own, at each lurch and the lunge.
    pub glitch: f32,
    /// How far his arms have gone from reaching for the edge to hanging at his sides (0 to 1).
    pub hang: f32,
}

/// When he starts to come through the hole, in seconds after the moon hit.
pub const PULL_AT: f32 = 15.6;

/// How he comes through, in seconds after `PULL_AT`: each step is (when it ends, how far through he
/// is by then, whether it is a snap). A snap is hard and fast and ends on the spot; the rest are a
/// slow creep, or him rearing back a hair before the next. The last two snaps are the big ones: the
/// lunge carries him past 1 (right up at the screen) and he settles back to standing.
const DRIVE: [(f32, f32, bool); 11] = [
    (0.4, 0.0, false),
    (0.7, 0.22, true),
    (1.3, 0.232, false),
    (1.45, 0.19, true),
    (1.75, 0.55, true),
    (2.3, 0.561, false),
    (2.4, 0.5, true),
    (2.65, 0.9, true),
    (3.0, 0.907, false),
    (3.25, 1.38, true),
    (3.9, 1.0, true),
];
/// How far past standing the lunge takes him.
pub const LUNGE_PEAK: f32 = 1.38;
/// When the lunge begins, in seconds after the moon hit (and so when his face starts to change).
pub const LUNGE_AT: f32 = PULL_AT + 3.0;

/// How far through the hole he is (0 to `LUNGE_PEAK`), `s` seconds after he starts.
fn drive(s: f32) -> f32 {
    let mut from = (0.0_f32, 0.0_f32);
    for (end, to, snap) in DRIVE {
        if s < end {
            let f = ((s - from.0) / (end - from.0)).clamp(0.0, 1.0);
            let f = if snap { 1.0 - (1.0 - f).powi(3) } else { f };
            return from.1 + (to - from.1) * f;
        }
        from = (end, to);
    }
    from.1
}

/// Where each knock of the window lands (seconds after `PULL_AT`) and how hard.
pub const KICKS: [(f32, f32); 5] = [(0.45, 0.5), (1.5, 0.7), (2.45, 0.9), (3.1, 1.0), (3.25, 0.8)];

/// How hard the window is being knocked (0 to 1): each knock lands and dies away fast.
fn kick_amount(t: f32) -> f32 {
    let mut k = 0.0;
    for (at, power) in KICKS {
        let age = t - (PULL_AT + at);
        if (0.0..1.2).contains(&age) {
            k += power * (-age / 0.22).exp();
        }
    }
    k.min(1.0)
}

/// How far the window is knocked from where it stands, in points, `t` seconds after the moon hit
/// (None when it is still).
pub fn shake(t: f32) -> Option<Pt> {
    let k = kick_amount(t);
    if k < 0.02 {
        return None;
    }
    Some((k * 11.0 * (t * 61.0).sin(), k * 7.0 * (t * 47.0 + 1.0).cos()))
}

/// A burst of glitching (0 to 1) with each knock, and a long one around the lunge.
fn glitch_burst(t: f32) -> f32 {
    let mut g = 0.0_f32;
    for (at, power) in KICKS {
        let age = t - (PULL_AT + at);
        if (0.0..0.9).contains(&age) {
            g = g.max(0.45 * power * (-age / 0.25).exp());
        }
    }
    let lunge = smooth((t - LUNGE_AT) / 0.15) * (1.0 - smooth((t - LUNGE_AT - 0.35) / 0.85));
    g.max(lunge)
}

// ---- his face changing ---------------------------------------------------------------------------

/// His face as it is first (the grin and the slanted eyes) tears into the one it becomes, in the
/// moments of the lunge.
pub const FACE_SECS: f32 = 0.7;
/// How many horizontal strips the head is cut into while it changes.
pub const BANDS: usize = 24;

/// One strip of his head while it glitches from the first face to the second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceBand {
    /// Is this strip the new face (or still the old one)?
    pub new: bool,
    /// How far it has been knocked sideways, in figure units.
    pub dx: f32,
}

fn face_progress(t: f32) -> f32 {
    ((t - LUNGE_AT) / FACE_SECS).clamp(0.0, 1.0)
}

/// The strips of his head at time `t`: the old face before the lunge, the new face after it, and in
/// between each strip flickers between the two, more and more of them the new one, knocked
/// sideways. It changes every 24th of a second, and is the same every time it is asked.
pub fn face_bands(t: f32) -> [FaceBand; BANDS] {
    let p = face_progress(t);
    if p <= 0.0 {
        return [FaceBand { new: false, dx: 0.0 }; BANDS];
    }
    if p >= 1.0 {
        return [FaceBand { new: true, dx: 0.0 }; BANDS];
    }
    let frame = (t * 24.0) as u32;
    let mix = smooth(p);
    let amp = 26.0 * (PI * p).sin();
    std::array::from_fn(|i| {
        let i = i as u32;
        let moved = hash(i, frame.wrapping_add(977)) < 0.55;
        FaceBand { new: hash(i, frame) < mix, dx: if moved { (hash(i, frame.wrapping_add(31)) - 0.5) * 2.0 * amp } else { 0.0 } }
    })
}

/// How much the two colour ghosts of his head are pulled apart while it changes (0 to 1).
pub fn face_chroma(t: f32) -> f32 {
    let p = face_progress(t);
    if p > 0.0 && p < 1.0 {
        (PI * p).sin()
    } else {
        0.0
    }
}

fn typed(t: f32, start: f32, text: &str) -> usize {
    (((t - start) * CPS).max(0.0) as usize).min(text.chars().count())
}

pub fn beat(t: f32) -> Beat {
    let t = t.max(0.0);
    let hole = smooth((t - 3.0) / 0.6) * 0.05 + 0.95 * smooth((t - 3.6) / 2.8);
    let v = drive(t - PULL_AT);
    Beat {
        seams: smooth((t - 0.8) / 3.6),
        ring: smooth((t - 1.5) / 1.5) * (1.0 - smooth((t - 5.0) / 1.4)),
        ring_turn: -(t - 1.5).max(0.0) * 0.9,
        rift: hole,
        dim: 0.8 * smooth((t - 4.2) / 2.4),
        line1: (typed(t, 6.4, LINE1), 1.0 - smooth((t - 10.2) / 0.7)),
        line2: (typed(t, 12.4, LINE2), 1.0),
        talons: smooth((t - 10.6) / 3.4),
        uncurl: smooth((t - 11.2) / 4.0),
        flame: smooth((t - 14.4) / 1.6),
        eyes: smooth((t - 15.4) / 1.2),
        body: smooth((t - 16.2) / 1.8),
        pull: v.min(1.0),
        over: ((v - 1.0) / (LUNGE_PEAK - 1.0)).clamp(0.0, 1.0),
        kick: kick_amount(t),
        glitch: glitch_burst(t),
        hang: smooth((t - 19.6) / 2.4),
    }
}

// ---- where the scene is --------------------------------------------------------------------------

/// The hole in space, in an area of `area` points: its middle, its half width and half height, and
/// the scale (1 at 560 x 380) the talons and the rest are drawn at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rift {
    pub centre: Pt,
    pub rx: f32,
    pub ry: f32,
    pub k: f32,
}

pub fn rift(area: Pt) -> Rift {
    let k = (area.1 / 380.0).min(area.0 / 460.0).max(0.4);
    Rift { centre: (area.0 / 2.0, area.1 / 2.0 + 10.0 * k), rx: 122.0 * k, ry: 176.0 * k, k }
}

/// Where the monarch stands once he is shown: where his figure's origin is, and its scale.
pub fn body_place(area: Pt) -> (Pt, f32) {
    ((area.0 / 2.0, area.1 * 0.295), area.1 / 760.0)
}

/// Where he is while he pulls himself through: the origin of his figure and its scale. He starts
/// small and deep in the middle of the hole and comes forward, bigger and bigger, until he stands
/// where `body_place` puts him.
pub fn body_now(area: Pt, pull: f32) -> (Pt, f32) {
    let rift = rift(area);
    let (end, scale_end) = body_place(area);
    let deep = ((rift.centre.0), rift.centre.1 - 46.0 * rift.k);
    let scale_deep = scale_end * 0.3;
    // a little slower at first, then rushing at you
    let e = pull.clamp(0.0, 1.0).powf(1.15);
    ((deep.0 + (end.0 - deep.0) * e, deep.1 + (end.1 - deep.1) * e), scale_deep + (scale_end - scale_deep) * e)
}

/// How much bigger he gets at the peak of the lunge (1 + this).
const LUNGE_ZOOM: f32 = 1.7;

/// Where he is at this moment of the scene: `body_now`, and in the lunge his face comes at you, the
/// figure growing about the middle of his head so that it stays where it was.
pub fn body_at_beat(area: Pt, b: &Beat) -> (Pt, f32) {
    let (o, s) = body_now(area, b.pull);
    if b.over <= 0.0 {
        return (o, s);
    }
    let grow = 1.0 + b.over * LUNGE_ZOOM;
    // the middle of his head is 95 units below the figure's origin
    ((o.0, o.1 + 95.0 * s * (1.0 - grow)), s * grow)
}

/// A talon hand while it comes out of the dark. Everything is relative to the middle of the hole, in
/// units of the 560 x 380 picture (multiply by `Rift::k`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Talon {
    pub origin: Pt,
    pub rot: f32,
    pub scale: f32,
    pub mirror: f32,
}

/// The two hands (left, right) at `reach` (0: still in the dark, 1: out).
pub fn talons(reach: f32) -> [Talon; 2] {
    let r = smooth(reach);
    let mk = |sx: f32| {
        let start = (sx * 40.0, 110.0);
        let end = (sx * 92.0, 84.0);
        Talon {
            origin: (start.0 + (end.0 - start.0) * r, start.1 + (end.1 - start.1) * r),
            rot: -PI / 2.0 + sx * 0.7,
            scale: 0.8 + 0.45 * r,
            mirror: -sx,
        }
    };
    [mk(-1.0), mk(1.0)]
}

/// Where one hand is, how it is turned, how big, and whether it is mirrored (see `place`), in area
/// points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandPose {
    pub origin: Pt,
    pub rot: f32,
    pub scale: f32,
    pub mirror: f32,
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The hands (left, right) at time `t` in an area of `area` points. First they come up out of the
/// dark, reaching; as he pulls himself through they swing down to become the hands that hang from
/// his arms.
pub fn hand_poses(area: Pt, t: f32) -> [HandPose; 2] {
    let b = beat(t);
    let r = rift(area);
    let reach = talons(b.talons);
    let (origin, scale) = body_at_beat(area, &b);
    let arms = arms();
    let mk = |i: usize| {
        let talon = reach[i];
        let from = HandPose {
            origin: (r.centre.0 + talon.origin.0 * r.k, r.centre.1 + talon.origin.1 * r.k),
            rot: talon.rot,
            scale: talon.scale * r.k,
            mirror: talon.mirror,
        };
        let (bo, brot, bscale, bmirror) = body_hand(&arms[i]);
        let to = HandPose { origin: (origin.0 + bo.0 * scale, origin.1 + bo.1 * scale), rot: brot, scale: bscale * scale, mirror: bmirror };
        let h = b.hang;
        HandPose {
            origin: (lerp(from.origin.0, to.origin.0, h), lerp(from.origin.1, to.origin.1, h)),
            rot: lerp(from.rot, to.rot, h),
            scale: lerp(from.scale, to.scale, h),
            mirror: to.mirror,
        }
    };
    [mk(0), mk(1)]
}

/// One arm (left 0, right 1): shoulder, elbow, wrist in area points at time `t`, and the widths at
/// the shoulder and at the wrist. The shoulder is where his body is, even while he is only a shape
/// deep in the dark, so the arm comes out of the hole's darkness and is joined to him.
pub fn arm_points(area: Pt, t: f32, i: usize) -> ([Pt; 3], f32, f32) {
    let b = beat(t);
    let r = rift(area);
    let (origin, scale) = body_at_beat(area, &b);
    let arm = arms()[i];
    let at = |p: Pt| (origin.0 + p.0 * scale, origin.1 + p.1 * scale);
    let hand = hand_poses(area, t)[i];
    let shoulder = at(arm.shoulder);
    let wrist = hand.origin;
    // reaching, the elbow bends outwards and a little down; hanging, it is his own elbow
    let mid = ((shoulder.0 + wrist.0) / 2.0, (shoulder.1 + wrist.1) / 2.0);
    let bent = (mid.0 + arm.side * 34.0 * r.k, mid.1 + 20.0 * r.k);
    let own = at(arm.elbow);
    let elbow = (lerp(bent.0, own.0, b.hang), lerp(bent.1, own.1, b.hang));
    let thin = lerp(8.0 * r.k, 34.0 * scale, b.hang);
    let thick = lerp(26.0 * hand.scale, 34.0 * scale, b.hang);
    ([shoulder, elbow, wrist], thin, thick)
}

/// Where the violet fire hangs: out from the right hand's palm in the direction the hand points, and
/// how big. It is the size of the hand while the hand reaches, and of him once he stands.
pub fn fire_now(area: Pt, t: f32) -> (Pt, f32) {
    let b = beat(t);
    let h = hand_poses(area, t)[1];
    let d = (h.rot.cos(), h.rot.sin());
    let reach = 72.0 * h.scale;
    let (_, body_scale) = body_at_beat(area, &b);
    let size = lerp(1.3 * rift(area).k, 3.0 * body_scale, b.hang);
    ((h.origin.0 + d.0 * reach, h.origin.1 + d.1 * reach), size)
}

// ---- the sand and the seams ----------------------------------------------------------------------

pub const SEAMS: usize = 6;

/// A jagged line across the page where the glitch tore it: points as fractions of the width and
/// height. Gold sand runs along it and stitches it shut.
pub fn seam(i: usize) -> Vec<Pt> {
    let i = i as u32;
    let mut y = 0.08 + 0.84 * hash(i, 1);
    let mut x = 0.0;
    let mut out = vec![(0.0, y)];
    let mut k = 0;
    while x < 1.0 {
        x = (x + 0.03 + 0.08 * hash(i, 10 + k)).min(1.0);
        y = (y + (hash(i, 40 + k) - 0.5) * 0.14).clamp(0.03, 0.97);
        out.push((x, y));
        k += 1;
    }
    out
}

/// How much of each seam is drawn: they do not all finish together.
pub fn seam_share(i: usize, seams: f32) -> f32 {
    let i = (i % SEAMS) as f32;
    let (start, end) = (0.05 * i, 0.55 + 0.09 * i);
    ((seams - start) / (end - start)).clamp(0.0, 1.0)
}

pub const RING_GRAINS: usize = 150;

/// A grain of sand in the spiral that runs into the middle of the ring: (position in units of the
/// ring's radius, size, opacity, tone 0 to 2).
pub fn ring_grain(i: usize, turn: f32) -> (Pt, f32, f32, u8) {
    let i = i as u32;
    let u = hash(i, 1);
    let a = u * 9.0 + hash(i, 2) * 0.4 + turn;
    let rad = (1.0 - u).powf(1.3) + 0.04;
    ((a.cos() * rad, a.sin() * rad), [1.5, 2.0, 2.5][(hash(i, 3) * 3.0) as usize % 3], 0.4 + 0.6 * hash(i, 4), (hash(i, 5) * 3.0) as u8 % 3)
}

// ---- cutting shapes into triangles -----------------------------------------------------------------

fn cross(a: Pt, b: Pt, c: Pt) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// The signed area of a polygon (positive when it runs counter-clockwise in a y-up picture).
pub fn signed_area(p: &[Pt]) -> f32 {
    let mut s = 0.0;
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        s += a.0 * b.1 - b.0 * a.1;
    }
    s / 2.0
}

fn in_triangle(p: Pt, a: Pt, b: Pt, c: Pt) -> bool {
    let (d1, d2, d3) = (cross(a, b, p), cross(b, c, p), cross(c, a, p));
    d1 >= -1e-6 && d2 >= -1e-6 && d3 >= -1e-6
}

/// Cuts a simple polygon (convex or not, either way round) into triangles by clipping its ears.
/// Returns indexes into `poly`.
pub fn triangulate(poly: &[Pt]) -> Vec<[usize; 3]> {
    let n = poly.len();
    if n < 3 {
        return Vec::new();
    }
    let mut idx: Vec<usize> = (0..n).collect();
    if signed_area(poly) < 0.0 {
        idx.reverse();
    }
    let mut tris = Vec::with_capacity(n - 2);
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for k in 0..m {
            let (a, b, c) = (idx[(k + m - 1) % m], idx[k], idx[(k + 1) % m]);
            if cross(poly[a], poly[b], poly[c]) <= 1e-6 {
                continue;
            }
            let blocked = idx.iter().any(|&p| {
                p != a && p != b && p != c && poly[p] != poly[a] && poly[p] != poly[b] && poly[p] != poly[c] && in_triangle(poly[p], poly[a], poly[b], poly[c])
            });
            if blocked {
                continue;
            }
            tris.push([a, b, c]);
            idx.remove(k);
            clipped = true;
            break;
        }
        if !clipped {
            // nothing left to clip but flat corners: drop one and go on
            let flat = (0..m).find(|&k| cross(poly[idx[(k + m - 1) % m]], poly[idx[k]], poly[idx[(k + 1) % m]]).abs() <= 1e-6);
            match flat {
                Some(k) => {
                    idx.remove(k);
                }
                None => break,
            }
        }
    }
    if idx.len() == 3 {
        tris.push([idx[0], idx[1], idx[2]]);
    }
    tris
}

/// Points along a cubic curve, `n` steps, from `p0` to `p3`.
pub fn bezier(p0: Pt, p1: Pt, p2: Pt, p3: Pt, n: usize) -> Vec<Pt> {
    (0..=n)
        .map(|k| {
            let t = k as f32 / n as f32;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            (a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0, a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1)
        })
        .collect()
}

/// Joins curves into one outline without doubling the points where they meet.
fn join(parts: Vec<Vec<Pt>>) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::new();
    for part in parts {
        for p in part {
            if out.last().map_or(true, |q| (q.0 - p.0).abs() > 1e-4 || (q.1 - p.1).abs() > 1e-4) {
                out.push(p);
            }
        }
    }
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

fn mirror(p: &[Pt]) -> Vec<Pt> {
    p.iter().rev().map(|(x, y)| (-x, *y)).collect()
}

// ---- the figure ----------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// A black shape with a violet rim.
    Solid,
    /// The cloak: dark, thinning to nothing towards the hem.
    Cloak,
    /// The crown: black with a pale rim.
    Crown,
    /// A claw: pale and sharp.
    Claw,
    /// A thick black limb or finger along the points, with a violet rim (the width).
    Limb(f32),
    /// A thin violet line (the width): ribs, wisps, the chest slit.
    Thin(f32),
    /// The teeth: a thin pale line.
    Teeth,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub kind: Kind,
    pub pts: Vec<Pt>,
}

impl Piece {
    /// Is this a filled shape (to be cut into triangles) rather than a line?
    pub fn filled(&self) -> bool {
        matches!(self.kind, Kind::Solid | Kind::Cloak | Kind::Crown | Kind::Claw)
    }
}

fn solid(pts: Vec<Pt>) -> Piece {
    Piece { kind: Kind::Solid, pts }
}

fn head_scale(pts: &[Pt]) -> Vec<Pt> {
    pts.iter().map(|(x, y)| (x * 1.28, 70.0 + (y - 70.0) * 1.28)).collect()
}

/// Everything of him except the arms and hands, back to front: the cloak and the wisps rising from
/// it, the two pairs of horns, the body, the shoulders, the head with its crown and its grin.
pub fn figure() -> Vec<Piece> {
    let mut out = Vec::new();
    out.push(Piece {
        kind: Kind::Cloak,
        pts: vec![
            (-110.0, 150.0), (110.0, 150.0), (170.0, 420.0), (150.0, 560.0), (120.0, 520.0), (100.0, 640.0), (70.0, 590.0),
            (40.0, 690.0), (10.0, 620.0), (-20.0, 700.0), (-50.0, 610.0), (-80.0, 670.0), (-110.0, 560.0), (-140.0, 610.0),
            (-170.0, 440.0),
        ],
    });
    for k in 0..12u32 {
        let x0 = (hash(k, 50) - 0.5) * 300.0;
        let y0 = 380.0 + hash(k, 51) * 300.0;
        let (dx1, dy1) = ((hash(k, 52) - 0.5) * 60.0, -(50.0 + hash(k, 53) * 60.0));
        let (dx2, dy2) = ((hash(k, 54) - 0.5) * 28.0, -(110.0 + hash(k, 55) * 90.0));
        out.push(Piece { kind: Kind::Thin(1.5 + 3.0 * hash(k, 56)), pts: bezier((x0, y0), (x0 + dx1, y0 + dy1 * 0.5), (x0 + dx1, y0 + dy1), (x0 + dx2, y0 + dy2), 8) });
    }
    // the low pair of horns, then the main pair
    let low = join(vec![
        bezier((34.0, 70.0), (110.0, 86.0), (170.0, 60.0), (196.0, 0.0), 10),
        bezier((196.0, 0.0), (184.0, 46.0), (132.0, 92.0), (40.0, 92.0), 10),
    ]);
    let main = join(vec![
        bezier((30.0, 50.0), (90.0, 52.0), (132.0, 14.0), (128.0, -70.0), 10),
        bezier((128.0, -70.0), (124.0, -120.0), (112.0, -166.0), (96.0, -200.0), 10),
        bezier((96.0, -200.0), (98.0, -130.0), (84.0, -70.0), (60.0, -30.0), 10),
        vec![(32.0, 6.0)],
    ]);
    out.push(solid(low.clone()));
    out.push(solid(mirror(&low)));
    out.push(solid(main.clone()));
    out.push(solid(mirror(&main)));
    // chest and ribs
    out.push(solid(vec![(-24.0, 140.0), (24.0, 140.0), (62.0, 184.0), (72.0, 330.0), (42.0, 384.0), (-42.0, 384.0), (-72.0, 330.0), (-62.0, 184.0)]));
    for i in 0..4 {
        let y = 190.0 + i as f32 * 34.0;
        out.push(Piece { kind: Kind::Thin(1.6), pts: bezier((-34.0, y), (-17.0, y + 8.0), (17.0, y + 8.0), (34.0, y), 8) });
    }
    // spiked shoulders
    let pl = vec![
        (-20.0, 138.0), (-72.0, 126.0), (-124.0, 86.0), (-160.0, 30.0), (-136.0, 108.0), (-162.0, 128.0), (-128.0, 152.0),
        (-142.0, 182.0), (-72.0, 180.0), (-24.0, 170.0),
    ];
    out.push(solid(pl.clone()));
    out.push(solid(mirror(&pl)));
    out.push(solid(vec![(-12.0, 110.0), (12.0, 110.0), (18.0, 146.0), (-18.0, 146.0)]));
    out.push(solid(head_scale(&[(-28.0, 24.0), (-30.0, 70.0), (-22.0, 112.0), (-9.0, 134.0), (0.0, 146.0), (9.0, 134.0), (22.0, 112.0), (30.0, 70.0), (28.0, 24.0)])));
    out.push(Piece {
        kind: Kind::Crown,
        pts: head_scale(&[(-30.0, 32.0), (-27.0, -8.0), (-17.0, 14.0), (-10.0, -34.0), (0.0, 12.0), (10.0, -34.0), (17.0, 14.0), (27.0, -8.0), (30.0, 32.0)]),
    });
    // the grin and its teeth
    let my = 118.0;
    out.push(Piece { kind: Kind::Thin(1.6), pts: bezier((-20.0, my - 3.0), (-8.0, my + 9.0), (8.0, my + 9.0), (20.0, my - 3.0), 10) });
    let mut teeth = Vec::new();
    for i in -5..=5 {
        let x = i as f32 * 3.4;
        let up = i % 2 == 0;
        teeth.push((x, my + if up { 0.0 } else { 1.5 }));
        teeth.push((x + 1.7, my + if up { 7.0 } else { -1.0 }));
    }
    out.push(Piece { kind: Kind::Teeth, pts: teeth });
    // the slit in his chest, where the Void opened
    out.push(Piece { kind: Kind::Thin(2.4), pts: vec![(0.0, 205.0), (0.0, 290.0)] });
    out
}

/// One eye: its middle, the pale slit in it, and which colour it glows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eye {
    pub centre: Pt,
    pub slit: [Pt; 3],
    pub cyan: bool,
}

pub fn eyes() -> [Eye; 2] {
    let mk = |sx: f32| {
        let (ex, ey) = (sx * 16.0, 66.0);
        Eye { centre: (ex, ey), slit: [(ex - sx * 10.0, ey - 3.0), (ex + sx * 11.0, ey + 4.0), (ex - sx * 9.0, ey + 5.0)], cyan: sx < 0.0 }
    };
    [mk(-1.0), mk(1.0)]
}

/// An arm: shoulder, elbow, wrist, the spike on the elbow, and which side (-1 left, 1 right).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arm {
    pub shoulder: Pt,
    pub elbow: Pt,
    pub wrist: Pt,
    pub spike: [Pt; 3],
    pub side: f32,
}

pub fn arms() -> [Arm; 2] {
    let mk = |sx: f32| {
        let elbow = (sx * 168.0, 262.0);
        Arm {
            shoulder: (sx * 122.0, 160.0),
            elbow,
            wrist: (sx * 132.0, 352.0),
            spike: [(elbow.0 - sx * 4.0, elbow.1 - 14.0), (elbow.0 + sx * 46.0, elbow.1 - 34.0), (elbow.0 + sx * 16.0, elbow.1 + 8.0)],
            side: sx,
        }
    };
    [mk(-1.0), mk(1.0)]
}

/// Where a hand is when it hangs from the arm, and how it is turned (see `hand`).
pub fn body_hand(arm: &Arm) -> (Pt, f32, f32, f32) {
    ((arm.wrist.0, arm.wrist.1 + 4.0), PI / 2.0 - arm.side * 0.5, 0.95, -arm.side)
}

/// Where the violet fire is held in the body's right hand (figure units) and its size.
pub fn body_fire() -> (Pt, f32) {
    let right = arms()[1];
    ((right.wrist.0 - 4.0, right.wrist.1 + 62.0), 1.5)
}

/// Turns a point of a hand (which points along +x, palm in the middle) into the picture:
/// scaled, mirrored up and down, turned by `rot`, and moved to `origin`.
pub fn place(p: Pt, origin: Pt, rot: f32, scale: f32, mirror: f32) -> Pt {
    let (x, y) = (p.0 * scale, p.1 * scale * mirror);
    let (s, c) = rot.sin_cos();
    (origin.0 + x * c - y * s, origin.1 + x * s + y * c)
}

fn finger(base: Pt, ang: f32, lens: [f32; 3], curl: f32, width: f32) -> Vec<Piece> {
    let mut pts = vec![base];
    let (mut x, mut y, mut a) = (base.0, base.1, ang);
    for (i, l) in lens.iter().enumerate() {
        a += curl * (i as f32 + 1.0) * 0.5;
        x += a.cos() * l;
        y += a.sin() * l;
        pts.push((x, y));
    }
    // the claw: thin, sharp, curving on
    a += curl * 1.6;
    let tip = (x + a.cos() * lens[2] * 1.15, y + a.sin() * lens[2] * 1.15);
    let (px, py) = (-a.sin() * width * 0.45, a.cos() * width * 0.45);
    vec![Piece { kind: Kind::Limb(width), pts }, Piece { kind: Kind::Claw, pts: vec![(x + px, y + py), tip, (x - px, y - py)] }]
}

/// A talon hand pointing along +x with the palm at the origin: a plated palm, four long jointed
/// fingers ending in claws, and a thumb. `uncurl` (0 to 1): 0 has the fingers drawn in tight, 1 open.
pub fn hand(uncurl: f32) -> Vec<Piece> {
    let curl_k = 1.0 + 3.5 * (1.0 - uncurl.clamp(0.0, 1.0));
    let mut out = vec![solid(vec![(-24.0, -24.0), (6.0, -30.0), (26.0, -24.0), (30.0, 0.0), (26.0, 24.0), (6.0, 30.0), (-24.0, 24.0), (-30.0, 0.0)])];
    for k in 0..3 {
        let x = -4.0 + k as f32 * 10.0;
        out.push(Piece { kind: Kind::Thin(1.2), pts: vec![(x, -18.0), (x, 18.0)] });
    }
    let fingers = [(-0.5, [30.0, 28.0, 26.0], 0.22, 12.0), (-0.17, [36.0, 34.0, 30.0], 0.20, 12.5), (0.17, [36.0, 34.0, 30.0], 0.20, 12.5), (0.5, [28.0, 26.0, 24.0], 0.22, 11.0)];
    for (a, lens, curl, width) in fingers {
        out.extend(finger((26.0, a * 34.0), a * 0.55, lens, curl * curl_k, width));
    }
    out.extend(finger((8.0, -27.0), -1.15, [22.0, 22.0, 22.0], -0.22 * curl_k, 11.0));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(p: &[Pt]) -> f32 {
        signed_area(p).abs()
    }

    /// Do two segments cross each other (properly, not just at an end)?
    fn crosses(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
        let (d1, d2, d3, d4) = (cross(a, b, c), cross(a, b, d), cross(c, d, a), cross(c, d, b));
        d1 * d2 < -1e-6 && d3 * d4 < -1e-6
    }

    fn is_simple(p: &[Pt]) -> bool {
        let n = p.len();
        for i in 0..n {
            for j in i + 2..n {
                if i == 0 && j == n - 1 {
                    continue;
                }
                if crosses(p[i], p[(i + 1) % n], p[j], p[(j + 1) % n]) {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn the_scene_plays_faster_than_it_was_written_but_not_while_the_staff_breaks() {
        assert_eq!(scene_time(-1.0), 0.0);
        for k in 0..=24 {
            let s = k as f32 * 0.1;
            assert_eq!(scene_time(s), s, "the break is at the written speed");
        }
        let mut last = -1.0;
        for k in 0..=400 {
            let w = scene_time(k as f32 * 0.05);
            assert!(w > last, "never stops or goes back");
            last = w;
        }
        // continuous at the join, faster after it, and over sooner by the same amount everywhere
        assert!((scene_time(NORMAL_UNTIL + 0.001) - NORMAL_UNTIL - 0.001 * SPEED).abs() < 1e-3);
        assert!((scene_time(10.0) - scene_time(9.0) - SPEED).abs() < 1e-4);
        assert!((scene_time(REAL_END) - END).abs() < 1e-3);
        assert!(REAL_END < END * 0.8, "{REAL_END}");
        // and he is in his place when it ends
        let end = beat(scene_time(REAL_END));
        assert_eq!((end.body, end.hang, end.over, end.kick, end.glitch), (1.0, 1.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn the_scene_runs_in_order_and_ends_with_him_standing_there() {
        let b0 = beat(0.0);
        assert_eq!((b0.seams, b0.rift, b0.dim, b0.talons, b0.flame, b0.eyes, b0.body), (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
        assert_eq!((b0.line1.0, b0.line2.0), (0, 0));
        let end = beat(END);
        assert_eq!((end.seams, end.rift, end.talons, end.uncurl, end.flame, end.eyes, end.body), (1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0));
        assert_eq!((end.pull, end.hang, end.over), (1.0, 1.0, 0.0));
        assert!(end.kick == 0.0 && end.glitch == 0.0, "all quiet when he stands there");
        assert_eq!((b0.pull, b0.hang), (0.0, 0.0));
        assert_eq!(end.line2, (LINE2.chars().count(), 1.0));
        assert_eq!(end.line1.1, 0.0, "the first line is long gone");
        assert!(end.ring == 0.0 && end.dim > 0.79);
        // each part is only ever going forward (the ring and the first line come and go)
        let mut last = beat(0.0);
        for k in 1..=(END / 0.05) as i32 {
            let b = beat(k as f32 * 0.05);
            assert!(b.seams >= last.seams && b.rift >= last.rift && b.dim >= last.dim);
            assert!(b.talons >= last.talons && b.uncurl >= last.uncurl && b.flame >= last.flame && b.eyes >= last.eyes && b.body >= last.body);
            assert!(b.hang >= last.hang);
            // he only ever rears back a hair between lurches, and never slides back into the hole
            assert!(b.pull >= last.pull - 0.07, "{} {}", b.pull, last.pull);
            assert!(b.line1.0 >= last.line1.0 && b.line2.0 >= last.line2.0);
            assert!(b.ring_turn <= last.ring_turn, "the clock only turns back");
            last = b;
        }
    }

    #[test]
    fn he_speaks_before_anything_shows_then_the_hands_come_then_the_fire_then_the_eyes_then_him() {
        // the first words are spoken in a hole that is already open, with nothing in it yet
        let t = 6.4 + 0.3;
        let b = beat(t);
        assert!(b.line1.0 > 0 && b.rift > 0.99);
        assert_eq!((b.talons, b.flame, b.eyes, b.body), (0.0, 0.0, 0.0, 0.0));
        // the whole first line is out before the hands start, and gone before the second is done
        assert_eq!(beat(10.4).line1.0, LINE1.chars().count());
        assert_eq!(beat(10.4).talons, 0.0);
        assert!(beat(11.5).talons > 0.0 && beat(11.5).line1.1 < 1.0);
        assert!(beat(12.4 + 0.5).line2.0 > 0);
        assert_eq!(beat(10.5 + 0.2).line2.0, 0, "the second line waits for the hands");
        // the fire is lit before the eyes, the eyes before the body
        let first = |f: &dyn Fn(&Beat) -> f32| (0..2100).map(|k| k as f32 * 0.01).find(|t| f(&beat(*t)) > 0.001).unwrap();
        let (fl, ey, bo, ta) = (first(&|b| b.flame), first(&|b| b.eyes), first(&|b| b.body), first(&|b| b.talons));
        assert!(ta < fl && fl < ey && ey < bo, "{ta} {fl} {ey} {bo}");
        // and the second line is whole before he is
        assert_eq!(beat(first(&|b| b.body)).line2.0, LINE2.chars().count());
    }

    #[test]
    fn the_hole_starts_as_a_dot_and_opens_while_the_page_goes_dark() {
        assert_eq!(beat(2.9).rift, 0.0);
        let dot = beat(3.6).rift;
        assert!(dot > 0.04 && dot < 0.1, "{dot}");
        assert!(beat(5.0).rift > dot && beat(5.0).dim > 0.0);
        assert!(beat(6.5).rift > 0.99);
        // the sand ring is there while the clock turns back and is gone before the voice
        assert!(beat(3.0).ring > 0.9 && beat(6.5).ring == 0.0);
    }

    #[test]
    fn the_rift_and_the_body_fit_every_area_the_moon_is_shown_in() {
        for area in [(260.0, 240.0), (400.0, 300.0), (560.0, 380.0), (900.0, 700.0), (1400.0, 900.0), (500.0, 900.0)] {
            let r = rift(area);
            assert!(r.centre.0 - r.rx >= 0.0 && r.centre.0 + r.rx <= area.0, "{area:?} {r:?}");
            assert!(r.centre.1 - r.ry >= 0.0 && r.centre.1 + r.ry <= area.1, "{area:?} {r:?}");
            assert!(r.k > 0.0);
        }
        let (o, s) = body_place((560.0, 380.0));
        assert!((s - 0.5).abs() < 1e-6 && (o.0 - 280.0).abs() < 1e-3 && (o.1 - 112.1).abs() < 0.5, "{o:?} {s}");
    }

    #[test]
    fn cutting_a_shape_into_triangles_covers_it_exactly_once() {
        // a square, an L (not convex), and a star-ish arrow, clockwise and counter-clockwise
        let square = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let ell = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 4.0), (4.0, 4.0), (4.0, 10.0), (0.0, 10.0)];
        let arrow = vec![(0.0, 0.0), (6.0, 3.0), (12.0, 0.0), (8.0, 5.0), (12.0, 10.0), (6.0, 7.0), (0.0, 10.0), (4.0, 5.0)];
        for shape in [square, ell, arrow] {
            for flipped in [false, true] {
                let p: Vec<Pt> = if flipped { shape.iter().rev().copied().collect() } else { shape.clone() };
                let tris = triangulate(&p);
                assert_eq!(tris.len(), p.len() - 2);
                let sum: f32 = tris.iter().map(|t| area(&[p[t[0]], p[t[1]], p[t[2]]])).sum();
                assert!((sum - area(&p)).abs() < 1e-3, "{sum} vs {}", area(&p));
            }
        }
        assert!(triangulate(&[(0.0, 0.0), (1.0, 1.0)]).is_empty());
    }

    #[test]
    fn a_curve_runs_from_its_first_point_to_its_last() {
        let c = bezier((0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0), 8);
        assert_eq!(c.len(), 9);
        assert_eq!((c[0], c[8]), ((0.0, 0.0), (0.0, 10.0)));
        assert!(c.iter().all(|p| p.0 >= 0.0 && p.0 <= 10.0 && p.1 >= 0.0 && p.1 <= 10.0));
    }

    #[test]
    fn every_filled_shape_of_him_is_a_simple_outline_that_cuts_into_triangles_without_loss() {
        let mut hands = hand(1.0);
        hands.extend(hand(0.0));
        for piece in figure().into_iter().chain(hands) {
            if !piece.filled() {
                continue;
            }
            assert!(piece.pts.len() >= 3, "{:?}", piece.kind);
            assert!(is_simple(&piece.pts), "{:?} crosses itself: {:?}", piece.kind, piece.pts);
            let tris = triangulate(&piece.pts);
            let sum: f32 = tris.iter().map(|t| area(&[piece.pts[t[0]], piece.pts[t[1]], piece.pts[t[2]]])).sum();
            assert!((sum - area(&piece.pts)).abs() <= area(&piece.pts) * 1e-3 + 1e-3, "{:?}: {sum} vs {}", piece.kind, area(&piece.pts));
        }
    }

    #[test]
    fn he_fits_in_his_box_and_is_the_same_on_both_sides_except_for_the_eyes() {
        for piece in figure() {
            for (x, y) in &piece.pts {
                assert!(x.abs() <= 210.0 && *y >= -210.0 && *y <= 720.0, "{:?} at {x},{y}", piece.kind);
            }
        }
        // the horns come in mirrored pairs
        let solids: Vec<Vec<Pt>> = figure().into_iter().filter(|p| p.kind == Kind::Solid).map(|p| p.pts).collect();
        let tops = solids.iter().filter(|p| p.iter().any(|q| q.1 < -150.0)).count();
        assert_eq!(tops, 2, "two long horns");
        let [l, r] = eyes();
        assert_eq!((l.centre.0, l.centre.1), (-r.centre.0, r.centre.1));
        assert!(l.cyan && !r.cyan);
        let [al, ar] = arms();
        assert_eq!((al.wrist.0, al.wrist.1), (-ar.wrist.0, ar.wrist.1));
    }

    #[test]
    fn the_fingers_reach_further_once_they_uncurl_and_every_hand_stays_in_its_box() {
        let reach = |u: f32| {
            hand(u)
                .iter()
                .filter(|p| p.kind == Kind::Claw)
                .map(|p| p.pts[1].0)
                .fold(f32::MIN, f32::max)
        };
        assert!(reach(1.0) > reach(0.0) + 30.0, "{} vs {}", reach(1.0), reach(0.0));
        for u in [0.0, 0.5, 1.0] {
            for p in hand(u) {
                for (x, y) in p.pts {
                    assert!(x.abs() < 200.0 && y.abs() < 200.0, "{x},{y}");
                }
            }
        }
        // four fingers and a thumb, each with a claw
        assert_eq!(hand(1.0).iter().filter(|p| p.kind == Kind::Claw).count(), 5);
    }

    #[test]
    fn a_hand_is_placed_turned_and_mirrored() {
        assert_eq!(place((10.0, 0.0), (5.0, 5.0), 0.0, 1.0, 1.0), (15.0, 5.0));
        // turned a quarter, the +x points down the page
        let p = place((10.0, 0.0), (0.0, 0.0), PI / 2.0, 1.0, 1.0);
        assert!(p.0.abs() < 1e-4 && (p.1 - 10.0).abs() < 1e-4);
        // mirrored, +y becomes -y, and scale applies first
        assert_eq!(place((0.0, 4.0), (0.0, 0.0), 0.0, 2.0, -1.0), (0.0, -8.0));
    }

    #[test]
    fn the_talons_come_up_out_of_the_dark_growing_as_they_come_and_stay_either_side_of_the_middle() {
        let [l0, r0] = talons(0.0);
        let [l1, r1] = talons(1.0);
        assert!(l0.origin.1 > l1.origin.1 + 20.0 && r0.origin.1 > r1.origin.1 + 20.0, "they rise");
        assert!(l1.scale > l0.scale + 0.4, "and come nearer");
        assert_eq!((l1.origin.0, l1.origin.1), (-r1.origin.0, r1.origin.1));
        assert!(l1.origin.0 < 0.0 && r1.origin.0 > 0.0);
        assert_eq!((l1.mirror, r1.mirror), (1.0, -1.0));
        let mut last = talons(0.0)[1].origin.1;
        for k in 1..=20 {
            let y = talons(k as f32 / 20.0)[1].origin.1;
            assert!(y <= last + 1e-4);
            last = y;
        }
    }

    #[test]
    fn the_seams_cross_the_whole_page_and_are_always_the_same() {
        for i in 0..SEAMS {
            let s = seam(i);
            assert!(s.len() >= 8);
            assert_eq!((s[0].0, s.last().unwrap().0), (0.0, 1.0));
            assert!(s.iter().all(|p| (0.0..=1.0).contains(&p.0) && (0.0..=1.0).contains(&p.1)));
            assert!(s.windows(2).all(|w| w[1].0 >= w[0].0));
            assert_eq!(s, seam(i));
            // they are drawn in as the stitching goes
            assert_eq!(seam_share(i, 0.0), 0.0);
            assert_eq!(seam_share(i, 1.0), 1.0);
            assert!(seam_share(i, 0.5) <= seam_share(i, 1.0) + 1e-6);
        }
        // a seam is not all drawn at once: the first is nearly done while the last has barely started
        assert!(seam_share(0, 0.5) > seam_share(5, 0.5) + 0.4);
    }

    #[test]
    fn the_sand_ring_has_grains_in_a_spiral_that_turns_back_with_the_clock() {
        for i in 0..RING_GRAINS {
            let (p, size, alpha, tone) = ring_grain(i, 0.0);
            assert!((p.0 * p.0 + p.1 * p.1).sqrt() <= 1.05 + 0.04, "grain {i} outside the ring");
            assert!((1.5..=2.5).contains(&size) && (0.4..=1.0).contains(&alpha) && tone < 3);
        }
        assert_ne!(ring_grain(3, 0.0).0, ring_grain(3, -1.0).0);
        assert_eq!(ring_grain(3, -1.0), ring_grain(3, -1.0));
    }

    fn in_hole(area: Pt, p: Pt) -> bool {
        let r = rift(area);
        let (dx, dy) = ((p.0 - r.centre.0) / r.rx, (p.1 - r.centre.1) / r.ry);
        dx * dx + dy * dy <= 1.0
    }

    const AREAS: [Pt; 4] = [(260.0, 240.0), (560.0, 380.0), (900.0, 700.0), (1400.0, 900.0)];

    #[test]
    fn he_comes_through_in_three_hard_lurches_and_a_lunge_after_his_eyes_are_open() {
        let first_pull = (0..2400).map(|k| k as f32 * 0.01).find(|t| beat(*t).pull > 0.0).unwrap();
        let first_eyes = (0..2400).map(|k| k as f32 * 0.01).find(|t| beat(*t).eyes > 0.0).unwrap();
        assert!(first_pull > first_eyes, "the eyes open in the dark first: {first_eyes} {first_pull}");
        assert_eq!(beat(PULL_AT).pull, 0.0);
        // how fast he goes: four short, fast rushes (three lurches and the lunge) with slow stretches
        let v = |s: f32| drive(s);
        let step = 0.01;
        let fast: Vec<bool> = (0..400).map(|k| (v((k + 1) as f32 * step) - v(k as f32 * step)) / step > 0.6).collect();
        let rushes = fast.windows(2).filter(|w| !w[0] && w[1]).count() + usize::from(fast[0]);
        assert_eq!(rushes, 4, "three lurches and the lunge");
        // he rears back a hair after the first two, never more than a little
        let dips: Vec<f32> = (0..300).map(|k| v((k + 1) as f32 * step) - v(k as f32 * step)).filter(|d| *d < -1e-6).collect();
        assert!(!dips.is_empty(), "he does draw back");
        assert!(dips.iter().sum::<f32>() > -0.15, "but only a little");
        // the whole thing is over in about four seconds, well inside the scene
        assert_eq!(v(3.9), 1.0);
        assert!(PULL_AT + 3.9 < 19.6 && PULL_AT + 3.9 < END - 3.0);
        assert!(beat(END - 1.0).pull == 1.0);
    }

    #[test]
    fn the_lunge_carries_him_past_standing_up_at_the_screen_and_back() {
        let peak = (0..400).map(|k| drive(k as f32 * 0.01)).fold(0.0_f32, f32::max);
        assert!((peak - LUNGE_PEAK).abs() < 1e-3, "{peak}");
        let at_peak = beat(PULL_AT + 3.25);
        assert_eq!((at_peak.pull, at_peak.over), (1.0, 1.0));
        // the lunge is the last snap, and nothing before it goes past standing
        for k in 0..300 {
            assert_eq!(beat(PULL_AT + k as f32 * 0.01).over, 0.0, "{k}");
        }
        assert_eq!(beat(LUNGE_AT + 1.5).over, 0.0, "and he settles");
        for area in AREAS {
            let (o, s) = body_place(area);
            let near = body_at_beat(area, &at_peak);
            assert!(near.1 > s * 2.0, "he is much bigger: {} {s}", near.1);
            // about the middle of his head, which stays where it was
            let head = |(o, s): (Pt, f32)| (o.0, o.1 + 95.0 * s);
            let (a, b) = (head((o, s)), head(near));
            assert!((a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3, "{a:?} {b:?}");
            // out of the lunge he is exactly where he stands
            let end = beat(END);
            assert_eq!(body_at_beat(area, &end), body_place(area));
        }
    }

    #[test]
    fn each_lurch_knocks_the_window_and_the_picture_glitches_with_it() {
        assert!(shake(PULL_AT - 1.0).is_none() && shake(PULL_AT + 0.2).is_none(), "still until the first lurch");
        for (at, _) in KICKS {
            let hit = PULL_AT + at + 0.02;
            let (x, y) = shake(hit).expect("knocked");
            assert!(x.abs() + y.abs() > 0.5, "{x} {y}");
            assert!(beat(hit).kick > 0.3);
            assert!(beat(hit).glitch > 0.1);
        }
        // it dies away quickly and the window comes to rest
        assert!(shake(END).is_none() && shake(PULL_AT + 5.0).is_none());
        // the lunge is the worst of it
        let lunge = beat(LUNGE_AT + 0.3);
        assert!(lunge.glitch > 0.95 && lunge.kick > 0.9, "{lunge:?}");
        let first = (0..60).map(|k| beat(PULL_AT + 0.4 + k as f32 * 0.01).glitch).fold(0.0_f32, f32::max);
        assert!(first < 0.5, "the first lurch is only a flicker: {first}");
        for k in 0..2300 {
            let b = beat(k as f32 * 0.01);
            assert!((0.0..=1.0).contains(&b.kick) && (0.0..=1.0).contains(&b.glitch) && (0.0..=1.0).contains(&b.over));
        }
    }

    #[test]
    fn his_face_is_the_first_one_until_the_lunge_then_tears_into_the_second_and_stays_there() {
        let all = |bands: [FaceBand; BANDS], new: bool| bands.iter().all(|b| b.new == new && b.dx == 0.0);
        for t in [0.0, 16.0, 18.0, LUNGE_AT - 0.01, LUNGE_AT] {
            assert!(all(face_bands(t), false), "{t}");
            assert_eq!(face_chroma(t), 0.0);
        }
        for t in [LUNGE_AT + FACE_SECS, LUNGE_AT + 1.0, END] {
            assert!(all(face_bands(t), true), "{t}");
            assert_eq!(face_chroma(t), 0.0);
        }
        // in between: a mixture, knocked sideways, the same every time it is asked
        let mut sideways = 0;
        let mut share = [0.0_f32; 2];
        for (j, p) in [0.2_f32, 0.8].into_iter().enumerate() {
            for frame in 0..48 {
                let t = LUNGE_AT + p * FACE_SECS + frame as f32 * (1.0 / 24.0) * 0.001 + frame as f32 * 0.0;
                let bands = face_bands(t);
                assert_eq!(bands, face_bands(t));
                share[j] += bands.iter().filter(|b| b.new).count() as f32 / BANDS as f32 / 48.0;
                sideways += bands.iter().filter(|b| b.dx != 0.0).count();
            }
        }
        // (frames are close together here, so check the mixture over the whole change instead)
        let mut seen_old = false;
        let mut seen_new = false;
        let mut max_dx = 0.0_f32;
        for k in 0..60 {
            let t = LUNGE_AT + 0.05 + k as f32 * (FACE_SECS - 0.1) / 60.0;
            for b in face_bands(t) {
                seen_new |= b.new;
                seen_old |= !b.new;
                max_dx = max_dx.max(b.dx.abs());
            }
            assert!(face_chroma(t) > 0.0);
        }
        assert!(seen_old && seen_new && max_dx > 5.0 && max_dx <= 26.0 + 1e-3, "{max_dx}");
        assert!(sideways > 0);
        assert!(share[1] > share[0] + 0.2, "more and more of it is the new face: {share:?}");
    }

    #[test]
    fn he_starts_small_and_deep_inside_the_hole_and_ends_where_he_stands() {
        for area in AREAS {
            let (o0, s0) = body_now(area, 0.0);
            let (o1, s1) = body_now(area, 1.0);
            assert_eq!((o1, s1), body_place(area));
            assert!(s0 < s1 * 0.35, "{s0} {s1}");
            // every point of him fits inside the hole at the start: he is in it
            for piece in figure() {
                for p in &piece.pts {
                    assert!(in_hole(area, (o0.0 + p.0 * s0, o0.1 + p.1 * s0)), "{area:?} {:?} {p:?}", piece.kind);
                }
            }
            // bigger and nearer all the way
            let mut last = s0;
            for k in 1..=20 {
                let (_, s) = body_now(area, k as f32 / 20.0);
                assert!(s >= last - 1e-6);
                last = s;
            }
        }
    }

    #[test]
    fn the_arms_start_in_the_dark_of_the_hole_where_he_is_and_end_as_his_own() {
        for area in AREAS {
            // while the hands are coming out they are in the hole, and the arm is joined to his body
            for t in [10.6, 11.5, 12.5, 13.5, 14.5, 15.5] {
                let hands = hand_poses(area, t);
                for i in 0..2 {
                    assert!(in_hole(area, hands[i].origin), "{area:?} hand {i} at {t}");
                    let (pts, thin, thick) = arm_points(area, t, i);
                    assert!(in_hole(area, pts[0]), "the arm starts inside the hole: {area:?} {t}");
                    assert_eq!(pts[2], hands[i].origin);
                    assert!(thin < thick, "narrow where it comes out of the dark, thick at the hand");
                    let (o, s) = body_now(area, beat(t).pull);
                    let shoulder = arms()[i].shoulder;
                    assert_eq!(pts[0], (o.0 + shoulder.0 * s, o.1 + shoulder.1 * s));
                }
            }
            // in the end they hang from his own arms
            let t = END;
            let (o, s) = body_place(area);
            for i in 0..2 {
                let (pts, thin, thick) = arm_points(area, t, i);
                let arm = arms()[i];
                assert_eq!(pts[1], (o.0 + arm.elbow.0 * s, o.1 + arm.elbow.1 * s));
                let (ho, hrot, hscale, hmirror) = body_hand(&arm);
                let hand = hand_poses(area, t)[i];
                assert!((hand.origin.0 - (o.0 + ho.0 * s)).abs() < 1e-3 && (hand.origin.1 - (o.1 + ho.1 * s)).abs() < 1e-3);
                assert!((hand.rot - hrot).abs() < 1e-4 && (hand.scale - hscale * s).abs() < 1e-4 && hand.mirror == hmirror);
                assert!((thin - thick).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn the_fire_stays_in_the_right_hand_all_the_way() {
        for area in AREAS {
            for k in 0..=230 {
                let t = k as f32 * 0.1;
                let hand = hand_poses(area, t)[1];
                let (fire, size) = fire_now(area, t);
                let d = ((fire.0 - hand.origin.0).powi(2) + (fire.1 - hand.origin.1).powi(2)).sqrt();
                assert!((d - 72.0 * hand.scale).abs() < 1e-2, "{t}: {d}");
                assert!(size > 0.0);
            }
        }
    }
}
