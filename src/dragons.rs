//! Fire dragons, the animated backdrop: fire runs round circuits that join the dragons.
//!
//! The picture (`assets/backdrop-dragons.png`, drawn by tools/dragons.py) has the dragons and their
//! circuits, dim. The circuits are also in `dragon_net.rs` as rings of hops (a hop runs from one
//! dragon's mouth, along a trace, through the next dragon, to its mouth). This file works out, from
//! the time, where the fire is: a bright head that runs round each ring without ever stopping, a
//! lit trail behind it, and a puff of flame at each mouth as the head leaves it, which throws the
//! head forward. Everything is in picture pixels; nothing here touches the screen.
use crate::dragon_net::LOOPS;
use std::sync::OnceLock;

pub type Pt = (f32, f32);

/// Size of the picture in pixels.
pub const PICTURE: (f32, f32) = (1200.0, 1600.0);
/// Average speed of the fire along a trace, in picture pixels a second.
const SPEED: f32 = 210.0;
/// How long the lit trail behind the head is, and how many pieces it is drawn in.
pub const TRAIL: f32 = 340.0;
pub const TRAIL_PIECES: usize = 14;
/// The puff lasts for this part of a hop.
const PUFF_PART: f32 = 0.2;
/// How much of a hop's speed is steady: the rest is the kick the puff gives, which dies away.
const STEADY: f32 = 0.5;

/// Where the picture goes in a view `view` points big that it fills completely (cropped equally
/// on the sides that overflow): (offset from the view's top left, points per picture pixel).
pub fn cover(view: (f32, f32)) -> (Pt, f32) {
    let scale = (view.0 / PICTURE.0).max(view.1 / PICTURE.1);
    (((view.0 - PICTURE.0 * scale) / 2.0, (view.1 - PICTURE.1 * scale) / 2.0), scale)
}

fn length(poly: &[Pt]) -> f32 {
    poly.windows(2).map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt()).sum()
}

/// One ring of hops with everything worked out once.
pub struct Ring {
    hops: &'static [&'static [Pt]],
    /// Length of every hop.
    len: Vec<f32>,
    /// Arc length at the start of every hop (and the total at the end).
    arc: Vec<f32>,
    /// Seconds the head needs for every hop.
    secs: Vec<f32>,
    /// Seconds at the start of every hop (and the total at the end).
    start: Vec<f32>,
    /// How many heads run round at once.
    pub heads: usize,
}

/// All the rings.
pub fn rings() -> &'static [Ring] {
    static RINGS: OnceLock<Vec<Ring>> = OnceLock::new();
    RINGS.get_or_init(|| {
        LOOPS
            .iter()
            .map(|hops| {
                let len: Vec<f32> = hops.iter().map(|h| length(h)).collect();
                let secs: Vec<f32> = len.iter().map(|l| l / SPEED).collect();
                let (mut arc, mut start) = (vec![0.0], vec![0.0]);
                for (l, s) in len.iter().zip(&secs) {
                    arc.push(arc.last().unwrap() + l);
                    start.push(start.last().unwrap() + s);
                }
                let heads = ((start.last().unwrap() / 4.5).round() as usize).max(2);
                Ring { hops, len, arc, secs, start, heads }
            })
            .collect()
    })
}

/// How far along a hop (0..1) the head is, `x` being how far through the hop's time it is. It
/// starts fast, thrown by the puff, and slows to half the average speed by the end, never to a
/// stop; the next hop's puff then throws it again.
pub fn along(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    STEADY * x + (1.0 - STEADY) * (1.0 - (1.0 - x).powi(3))
}

/// A head's place in time: which hop it is on, how far through that hop's time, and its arc length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Head {
    pub hop: usize,
    pub x: f32,
    pub arc: f32,
}

impl Ring {
    pub fn total_arc(&self) -> f32 {
        *self.arc.last().unwrap()
    }

    pub fn total_secs(&self) -> f32 {
        *self.start.last().unwrap()
    }

    /// Head number `n` (of `heads`, evenly spread round the ring) at time `t` seconds.
    pub fn head(&self, n: usize, t: f32) -> Head {
        let total = self.total_secs();
        let tau = (t + n as f32 * total / self.heads as f32).rem_euclid(total);
        let hop = self.start.partition_point(|s| *s <= tau).saturating_sub(1).min(self.hops.len() - 1);
        let x = ((tau - self.start[hop]) / self.secs[hop]).clamp(0.0, 1.0);
        Head { hop, x, arc: self.arc[hop] + self.len[hop] * along(x) }
    }

    /// The point `arc` along the ring (it wraps round).
    pub fn point(&self, arc: f32) -> Pt {
        let arc = arc.rem_euclid(self.total_arc());
        let hop = self.arc.partition_point(|a| *a <= arc).saturating_sub(1).min(self.hops.len() - 1);
        let mut left = arc - self.arc[hop];
        let poly = self.hops[hop];
        for w in poly.windows(2) {
            let l = length(w);
            if left <= l && l > 0.0 {
                let u = left / l;
                return (w[0].0 + (w[1].0 - w[0].0) * u, w[0].1 + (w[1].1 - w[0].1) * u);
            }
            left -= l;
        }
        *poly.last().unwrap()
    }

    /// The ring's points from arc `from` to arc `to` (from <= to <= from + the ring), corners included.
    pub fn slice(&self, from: f32, to: f32) -> Vec<Pt> {
        let mut out = vec![self.point(from)];
        let total = self.total_arc();
        let base = from.rem_euclid(total);
        let end = base + (to - from);
        // walk every corner of every hop, twice round so a slice can cross the start
        let mut offset = 0.0;
        for _ in 0..2 {
            for (hop, poly) in self.hops.iter().enumerate() {
                let mut at = offset + self.arc[hop];
                for w in poly.windows(2) {
                    at += length(w);
                    if at > base && at < end {
                        out.push(w[1]);
                    }
                }
            }
            offset += total;
        }
        out.push(self.point(to));
        out
    }

    /// The lit trail behind a head at arc length `arc`: pieces from the head backwards, each with
    /// how bright it is (1 at the head, fading).
    pub fn trail(&self, arc: f32) -> Vec<(Vec<Pt>, f32)> {
        let step = TRAIL / TRAIL_PIECES as f32;
        (0..TRAIL_PIECES)
            .map(|j| {
                let strength = (1.0 - j as f32 / TRAIL_PIECES as f32).powf(1.5);
                (self.slice(arc - (j as f32 + 1.0) * step, arc - j as f32 * step), strength)
            })
            .collect()
    }
}

/// A puff of flame at a dragon's mouth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Puff {
    pub pos: Pt,
    /// The way the fire leaves the mouth (a unit vector).
    pub dir: Pt,
    /// How big the flame is right now (0 to 1): it blooms and dies away.
    pub size: f32,
}

/// How big a puff is a part `x` of the way through the hop that starts at the mouth.
pub fn puff_size(x: f32) -> f32 {
    if !(0.0..PUFF_PART).contains(&x) {
        return 0.0;
    }
    (std::f32::consts::PI * (x / PUFF_PART).powf(0.5)).sin().max(0.0)
}

/// The puffs at time `t`: one at the mouth of every hop a head has only just left.
pub fn puffs(t: f32) -> Vec<Puff> {
    let mut out = Vec::new();
    for ring in rings() {
        for n in 0..ring.heads {
            let head = ring.head(n, t);
            let size = puff_size(head.x);
            if size > 0.0 {
                let poly = ring.hops[head.hop];
                let (a, b) = (poly[0], poly[1]);
                let l = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt().max(0.001);
                out.push(Puff { pos: a, dir: ((b.0 - a.0) / l, (b.1 - a.1) / l), size });
            }
        }
    }
    out
}

/// A tongue of flame, pointing along +x from (0, 0), about one unit long. It is a star shape around
/// (0.45, 0), so it can be filled as a fan from there.
pub const FLAME: [Pt; 13] = [
    (0.0, 0.0),
    (0.10, 0.20),
    (0.30, 0.34),
    (0.52, 0.30),
    (0.60, 0.14),
    (0.74, 0.22),
    (0.92, 0.12),
    (1.04, 0.04),
    (0.80, -0.04),
    (0.66, -0.22),
    (0.46, -0.36),
    (0.22, -0.30),
    (0.08, -0.16),
];
/// The middle the flame is filled from.
pub const FLAME_HEART: Pt = (0.45, 0.0);

/// The flame tongues of a puff: (the shape in picture pixels, how strongly it is drawn). Three
/// tongues fan out from the mouth, each with a smaller, brighter one inside.
pub fn tongues(puff: &Puff) -> Vec<(Vec<Pt>, f32)> {
    let mut out = Vec::new();
    for (angle, size) in [(-0.42_f32, 56.0_f32), (0.0, 78.0), (0.42, 52.0)] {
        let (s, c) = angle.sin_cos();
        let dir = (puff.dir.0 * c - puff.dir.1 * s, puff.dir.0 * s + puff.dir.1 * c);
        let normal = (-dir.1, dir.0);
        for (scale, strength) in [(1.0_f32, 0.7_f32), (0.55, 1.0)] {
            let k = size * scale * puff.size;
            let shape = FLAME
                .iter()
                .map(|&(a, b)| {
                    (
                        puff.pos.0 + dir.0 * 4.0 + dir.0 * k * a + normal.0 * k * b * 0.75,
                        puff.pos.1 + dir.1 * 4.0 + dir.1 * k * a + normal.1 * k * b * 0.75,
                    )
                })
                .collect();
            out.push((shape, strength));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Pt, b: Pt) -> bool {
        (a.0 - b.0).abs() < 0.6 && (a.1 - b.1).abs() < 0.6
    }

    #[test]
    fn every_ring_is_closed_and_joined_up() {
        assert!(rings().len() >= 2);
        for ring in rings() {
            assert!(ring.hops.len() >= 3);
            for (i, hop) in ring.hops.iter().enumerate() {
                assert!(hop.len() >= 3);
                let next = ring.hops[(i + 1) % ring.hops.len()];
                assert!(close(*hop.last().unwrap(), next[0]), "hop {} ends where the next begins", i);
                assert!(hop.iter().all(|p| (0.0..=PICTURE.0).contains(&p.0) && (-50.0..=PICTURE.1 + 50.0).contains(&p.1)));
            }
            assert!(ring.heads >= 2 && ring.total_arc() > 500.0);
        }
    }

    #[test]
    fn the_head_never_stops_and_is_thrown_forward_at_every_mouth() {
        assert!(along(0.0) == 0.0 && (along(1.0) - 1.0).abs() < 1e-6);
        let mut last = 0.0;
        for i in 1..=100 {
            let x = i as f32 / 100.0;
            assert!(along(x) > last, "always moving forward");
            last = along(x);
        }
        // the speed at the start of a hop is well above the speed at its end, and neither is zero
        let (start, end) = (along(0.01) / 0.01, (along(1.0) - along(0.99)) / 0.01);
        assert!(start > 1.5 * end && end > 0.3, "start {} end {}", start, end);
        // over a minute, in steps of a tenth of a second, the head only ever goes forward, with no
        // jump bigger than the fastest speed allows
        let ring = &rings()[0];
        let mut prev = ring.head(0, 0.0);
        for i in 1..600 {
            let h = ring.head(0, i as f32 * 0.1);
            let step = (h.arc - prev.arc).rem_euclid(ring.total_arc());
            assert!(step > 0.0 && step < SPEED * 2.1 * 0.1 + 0.5, "step {} at {}", step, i);
            prev = h;
        }
    }

    #[test]
    fn the_head_runs_along_the_trace_without_jumping() {
        for ring in rings() {
            let mut prev = ring.point(ring.head(0, 0.0).arc);
            for i in 1..400 {
                let p = ring.point(ring.head(0, i as f32 * 0.05).arc);
                let moved = ((p.0 - prev.0).powi(2) + (p.1 - prev.1).powi(2)).sqrt();
                assert!(moved < SPEED * 2.1 * 0.05 + 1.0, "moved {} px", moved);
                prev = p;
            }
        }
    }

    #[test]
    fn the_point_and_the_slice_follow_the_ring() {
        let ring = &rings()[0];
        assert!(close(ring.point(0.0), ring.hops[0][0]));
        assert!(close(ring.point(ring.total_arc()), ring.hops[0][0]), "it wraps");
        assert!(close(ring.point(ring.arc[1]), ring.hops[1][0]), "a hop starts where the last ended");
        // a slice starts and ends on the ring, and its length is what was asked for (or a bit more
        // for the corners it cuts)
        for (from, to) in [(10.0, 200.0), (ring.arc[2] - 30.0, ring.arc[2] + 90.0), (-100.0, 50.0)] {
            let pts = ring.slice(from, to);
            assert!(close(pts[0], ring.point(from)) && close(*pts.last().unwrap(), ring.point(to)));
            let l = length(&pts);
            assert!(l > (to - from) * 0.9 && l < (to - from) * 1.001 + 1.0, "{} for {}", l, to - from);
        }
    }

    #[test]
    fn the_trail_fades_behind_the_head() {
        let ring = &rings()[0];
        let trail = ring.trail(ring.arc[3] + 20.0);
        assert_eq!(trail.len(), TRAIL_PIECES);
        assert!(trail.windows(2).all(|w| w[0].1 > w[1].1));
        assert_eq!(trail[0].1, 1.0);
        assert!(trail.iter().all(|(pts, _)| pts.len() >= 2));
    }

    #[test]
    fn a_puff_blooms_at_the_start_of_a_hop_and_dies_away() {
        assert_eq!(puff_size(0.0), 0.0);
        assert!(puff_size(0.05) > 0.8);
        assert_eq!(puff_size(PUFF_PART), 0.0);
        assert_eq!(puff_size(0.5), 0.0);
        // there is nearly always some dragon puffing somewhere
        let busy = (0..1200).filter(|i| !puffs(*i as f32 * 0.05).is_empty()).count();
        assert!(busy > 1000, "{} of 1200 moments have a puff", busy);
        for p in puffs(3.3) {
            assert!((p.dir.0.powi(2) + p.dir.1.powi(2) - 1.0).abs() < 0.01);
            assert!(p.size > 0.0 && p.size <= 1.0);
        }
    }

    #[test]
    fn the_flame_is_a_star_shape_so_it_can_be_filled_from_its_heart() {
        let angle = |p: Pt| (p.1 - FLAME_HEART.1).atan2(p.0 - FLAME_HEART.0);
        // going round the outline the angle about the heart only ever falls (or wraps once)
        let mut falls = 0;
        for i in 0..FLAME.len() {
            let (a, b) = (angle(FLAME[i]), angle(FLAME[(i + 1) % FLAME.len()]));
            if b > a {
                falls += 1; // the wrap from -pi to pi
            }
        }
        assert_eq!(falls, 1);
        let tongues = tongues(&Puff { pos: (100.0, 100.0), dir: (1.0, 0.0), size: 1.0 });
        assert_eq!(tongues.len(), 6);
        assert!(tongues.iter().all(|(pts, s)| pts.len() == FLAME.len() && *s > 0.0));
        // a tongue points the way the puff does
        let far = tongues[2].0.iter().map(|p| p.0).fold(f32::MIN, f32::max);
        assert!(far > 140.0);
    }

    #[test]
    fn the_picture_fills_any_view() {
        // the same shape as the picture: fills it exactly
        let (o, s) = cover((600.0, 800.0));
        assert!((s - 0.5).abs() < 1e-6 && o == (0.0, 0.0));
        // wider: scaled to the width, the top and bottom spill over equally
        let (o, s) = cover((1200.0, 1200.0));
        assert!((s - 1.0).abs() < 1e-6 && o.0 == 0.0 && (o.1 + 200.0).abs() < 1e-3);
        // taller: scaled to the height, the sides spill over
        let (o, s) = cover((300.0, 1600.0));
        assert!((s - 1.0).abs() < 1e-6 && (o.0 + 450.0).abs() < 1e-3 && o.1 == 0.0);
    }

    /// Writes a frame as text for tools/preview_dragons.py (`DR_DUMP=file DR_T=7.5 cargo test
    /// dump_dragons -- --ignored`).
    #[test]
    #[ignore]
    fn dump_dragons() {
        let (Ok(path), t) = (std::env::var("DR_DUMP"), std::env::var("DR_T").ok().and_then(|v| v.parse().ok()).unwrap_or(5.0_f32))
        else {
            return;
        };
        let mut text = String::new();
        for ring in rings() {
            for n in 0..ring.heads {
                let head = ring.head(n, t);
                let p = ring.point(head.arc);
                text += &format!("H {} {}\n", p.0, p.1);
                for (pts, s) in ring.trail(head.arc) {
                    let pts: Vec<String> = pts.iter().map(|q| format!("{},{}", q.0, q.1)).collect();
                    text += &format!("T {} {}\n", s, pts.join(" "));
                }
            }
        }
        for puff in puffs(t) {
            for (pts, s) in tongues(&puff) {
                let pts: Vec<String> = pts.iter().map(|q| format!("{},{}", q.0, q.1)).collect();
                text += &format!("F {} {}\n", s, pts.join(" "));
            }
        }
        std::fs::write(path, text).unwrap();
    }
}
