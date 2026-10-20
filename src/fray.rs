//! The Fraying skin's ragged card edges: the bites taken out of the edge of a card and the loose
//! threads hanging from it. Pure geometry, so it is tested; `theme.rs` draws it.
//!
//! Everything is in pixels from the card's top left corner, and the same card size always gives
//! the same edge (so nothing shimmers when the window repaints).

/// A bite out of an edge: a triangle with two points on the edge and the tip inside the card.
pub type Notch = [(f32, f32); 3];

/// A thread: a few points, the first on the card's edge.
pub type Thread = Vec<(f32, f32)>;

/// The deepest a bite goes, in pixels.
pub const MAX_BITE: f32 = 5.0;
/// How far a thread reaches out past the card (there are only a few pixels between cards).
pub const MAX_REACH: f32 = 7.0;

fn next(state: &mut u32) -> f32 {
    // xorshift32
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

fn seeded(w: f32, h: f32, salt: u32) -> u32 {
    ((w.round() as u32).wrapping_mul(2_654_435_761) ^ (h.round() as u32).wrapping_mul(40_503).wrapping_add(salt)) | 1
}

/// The bites along the four edges of a `w` by `h` card.
pub fn notches(w: f32, h: f32) -> Vec<Notch> {
    let mut rng = seeded(w, h, 17);
    let mut out = Vec::new();
    // (start, direction along the edge, inward direction, length)
    let edges = [((0.0, 0.0), (1.0, 0.0), (0.0, 1.0), w), ((w, 0.0), (0.0, 1.0), (-1.0, 0.0), h), ((w, h), (-1.0, 0.0), (0.0, -1.0), w), ((0.0, h), (0.0, -1.0), (1.0, 0.0), h)];
    for (start, along, inward, len) in edges {
        // keep clear of the corners, which stay whole
        let mut at = 10.0 + next(&mut rng) * 10.0;
        while at < len - 22.0 {
            let width = 5.0 + next(&mut rng) * 11.0;
            let depth = 1.5 + next(&mut rng) * (MAX_BITE - 1.5);
            if next(&mut rng) < 0.6 {
                let p = |t: f32, d: f32| (start.0 + along.0 * t + inward.0 * d, start.1 + along.1 * t + inward.1 * d);
                // the tip leans to one side, so the bite is ragged and not a neat tooth
                let lean = (next(&mut rng) - 0.5) * width * 0.8;
                out.push([p(at, 0.0), p(at + width * 0.5 + lean, depth), p(at + width, 0.0)]);
            }
            at += width + 6.0 + next(&mut rng) * 14.0;
        }
    }
    out
}

/// The loose threads: a few of them hang from the bottom edge and a couple from the right.
pub fn threads(w: f32, h: f32) -> Vec<Thread> {
    let mut rng = seeded(w, h, 91);
    let mut out = Vec::new();
    for edge in 0..2 {
        let count = if edge == 0 { 3 + (next(&mut rng) * 3.0) as usize } else { 1 + (next(&mut rng) * 2.0) as usize };
        for _ in 0..count {
            let (len, out_dir, slide) = if edge == 0 { (w, (0.0, 1.0), (1.0, 0.0)) } else { (h, (1.0, 0.0), (0.0, 1.0)) };
            let along = 14.0 + next(&mut rng) * (len - 28.0).max(1.0);
            let origin = if edge == 0 { (along, h) } else { (w, along) };
            let reach = 3.0 + next(&mut rng) * (MAX_REACH - 3.0);
            let drift = (next(&mut rng) - 0.5) * 16.0;
            let mut points = Vec::new();
            for i in 0..4 {
                let f = i as f32 / 3.0;
                let sway = (f * 6.0 + along).sin() * 1.2 * f;
                points.push((
                    origin.0 + out_dir.0 * reach * f + slide.0 * (drift * f + sway),
                    origin.1 + out_dir.1 * reach * f + slide.1 * (drift * f + sway),
                ));
            }
            out.push(points);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_size_always_gives_the_same_edge() {
        assert_eq!(notches(400.0, 120.0), notches(400.0, 120.0));
        assert_eq!(threads(400.0, 120.0), threads(400.0, 120.0));
        assert_ne!(notches(400.0, 120.0), notches(401.0, 120.0), "another card, another edge");
    }

    #[test]
    fn every_bite_starts_on_an_edge_and_stays_shallow_and_inside() {
        for (w, h) in [(300.0, 90.0), (600.0, 400.0), (120.0, 60.0)] {
            let all = notches(w, h);
            assert!(!all.is_empty(), "{w}x{h}");
            for n in all {
                for (x, y) in n {
                    assert!(x >= -0.01 && x <= w + 0.01 && y >= -0.01 && y <= h + 0.01, "{n:?} inside {w}x{h}");
                }
                let on_edge = |(x, y): (f32, f32)| x.abs() < 0.01 || y.abs() < 0.01 || (x - w).abs() < 0.01 || (y - h).abs() < 0.01;
                assert!(on_edge(n[0]) && on_edge(n[2]), "{n:?}");
                let tip = n[1];
                let depth = tip.0.min(tip.1).min(w - tip.0).min(h - tip.1);
                assert!(depth <= MAX_BITE + 0.01 && depth >= 0.0, "depth {depth}");
            }
        }
    }

    #[test]
    fn the_corners_stay_whole() {
        for n in notches(500.0, 200.0) {
            for (x, y) in [n[0], n[2]] {
                let near_corner = (x < 9.0 || x > 491.0) && (y < 9.0 || y > 191.0);
                assert!(!near_corner, "{n:?}");
            }
        }
    }

    #[test]
    fn threads_hang_from_the_edge_and_do_not_reach_far() {
        for (w, h) in [(300.0, 90.0), (640.0, 320.0)] {
            let all = threads(w, h);
            assert!(all.len() >= 4);
            for t in all {
                assert_eq!(t.len(), 4);
                let (x, y) = t[0];
                assert!((y - h).abs() < 0.01 || (x - w).abs() < 0.01, "starts on the bottom or right edge: {t:?}");
                for (px, py) in &t {
                    assert!(*px <= w + MAX_REACH + 0.01 && *py <= h + MAX_REACH + 0.01, "{t:?}");
                }
            }
        }
    }
}
