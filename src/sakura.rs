//! The Sakura skin's blossom-tree art: a branch grown from a seed, with clusters of blossoms, that
//! the skin draws in the corners of the window and, small, on every card. Pure geometry, so it is
//! tested; `theme.rs` and `ui/backdrop.rs` draw it.
//!
//! Coordinates are in units of the first (trunk) segment's length, with y pointing down, and the
//! root at (0, 0).

/// One piece of branch: its two ends and how thick it is (in units of the trunk's thickness).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seg {
    pub a: (f32, f32),
    pub b: (f32, f32),
    pub width: f32,
}

/// A blossom: where, how big (units), how deep its pink is (0 pale, 1 deep) and how it is turned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bloom {
    pub at: (f32, f32),
    pub size: f32,
    pub tone: f32,
    pub turn: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Art {
    pub segs: Vec<Seg>,
    pub blooms: Vec<Bloom>,
}

fn next(state: &mut u32) -> f32 {
    // xorshift32
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

fn bloom(art: &mut Art, rng: &mut u32, at: (f32, f32), size: f32) {
    let (tone, turn) = (next(rng), next(rng) * std::f32::consts::TAU);
    art.blooms.push(Bloom { at, size, tone, turn });
}

fn grow(art: &mut Art, rng: &mut u32, from: (f32, f32), angle: f32, len: f32, width: f32, depth: u32) {
    // a branch bends a little as it goes, and sags towards the ground (y is down)
    let bend = (next(rng) - 0.5) * 0.55;
    let angle = angle + bend + 0.04;
    let to = (from.0 + angle.cos() * len, from.1 + angle.sin() * len);
    art.segs.push(Seg { a: from, b: to, width });
    if depth == 0 {
        // the end of a twig: a small cluster of blossoms
        bloom(art, rng, to, 0.085);
        for _ in 0..3 {
            let o = ((next(rng) - 0.5) * 0.18, (next(rng) - 0.5) * 0.18);
            let size = 0.05 + next(rng) * 0.03;
            bloom(art, rng, (to.0 + o.0, to.1 + o.1), size);
        }
        return;
    }
    if depth <= 3 {
        // blossoms all along the thinner branches
        for k in 1..=2 {
            if next(rng) < 0.8 {
                let f = k as f32 / 3.0;
                let off = (next(rng) - 0.5) * 0.06;
                bloom(art, rng, (from.0 + (to.0 - from.0) * f + off, from.1 + (to.1 - from.1) * f - off), 0.07);
            }
        }
    }
    // the branch goes on, and sends a side branch off to one side or the other
    let forward = len * (0.68 + next(rng) * 0.1);
    grow(art, rng, to, angle, forward, width * 0.75, depth - 1);
    let side = if next(rng) < 0.5 { -1.0 } else { 1.0 };
    let (turn, shorter) = (0.5 + next(rng) * 0.45, 0.55 + next(rng) * 0.12);
    grow(art, rng, to, angle + side * turn, len * shorter, width * 0.6, depth - 1);
}

/// A blossoming branch from `seed`, growing from the root at `heading` (radians, y down: 0 is
/// right, a quarter turn is down) with `depth` levels of forking (4 is a big branch, 2 a twig).
pub fn branch(seed: u32, heading: f32, depth: u32) -> Art {
    let mut rng = seed.wrapping_mul(2_654_435_761) | 1;
    let mut art = Art::default();
    grow(&mut art, &mut rng, (0.0, 0.0), heading, 1.0, 1.0, depth);
    art
}

/// Where a point of the art is while the wind blows: the further from the root, the more it sways
/// (`t` is the time in seconds, `phase` keeps two branches from swaying together).
pub fn swayed(p: (f32, f32), t: f32, phase: f32) -> (f32, f32) {
    let reach = (p.0 * p.0 + p.1 * p.1).min(9.0);
    let s = (t * 0.45 + phase).sin() * 0.018 * reach;
    (p.0 + s * 0.4, p.1 + s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_branch_is_the_same_every_time_and_another_seed_grows_another() {
        assert_eq!(branch(5, 2.4, 4), branch(5, 2.4, 4));
        assert_ne!(branch(5, 2.4, 4), branch(6, 2.4, 4));
    }

    #[test]
    fn a_big_branch_forks_and_blossoms_and_a_twig_is_smaller() {
        let big = branch(3, 2.4, 4);
        let twig = branch(3, 2.4, 2);
        // 4 levels of two branches each: 1 + 2 + 4 + 8 + 16 pieces
        assert_eq!(big.segs.len(), 31);
        assert_eq!(twig.segs.len(), 7);
        assert!(big.blooms.len() >= 64 && big.blooms.len() < 200, "{} blossoms", big.blooms.len());
        assert!(twig.blooms.len() >= 12 && twig.blooms.len() < big.blooms.len());
    }

    #[test]
    fn it_starts_at_the_root_and_the_pieces_join_up() {
        let art = branch(9, 0.7, 4);
        assert_eq!(art.segs[0].a, (0.0, 0.0));
        for s in &art.segs[1..] {
            assert!(art.segs.iter().any(|p| p.b == s.a), "every piece grows from the end of another");
        }
    }

    #[test]
    fn it_stays_a_sensible_size_and_thins_out_towards_the_ends() {
        for seed in 1..40 {
            let art = branch(seed, 2.4, 4);
            for s in &art.segs {
                for p in [s.a, s.b] {
                    assert!(p.0.is_finite() && p.1.is_finite());
                    assert!((p.0 * p.0 + p.1 * p.1).sqrt() < 5.0, "seed {seed}: {p:?}");
                }
                assert!(s.width > 0.0 && s.width <= 1.0);
            }
            assert_eq!(art.segs[0].width, 1.0);
            assert!(art.segs.iter().skip(1).all(|s| s.width < 1.0));
            for b in &art.blooms {
                assert!(b.size > 0.04 && b.size < 0.1 && (0.0..=1.0).contains(&b.tone));
            }
        }
    }

    #[test]
    fn the_main_branch_heads_where_it_was_sent() {
        for heading in [0.0_f32, 1.0, 2.4, -1.0] {
            let art = branch(2, heading, 3);
            let end = art.segs[0].b;
            let angle = end.1.atan2(end.0);
            assert!((angle - heading).abs() < 0.3, "heading {heading}, went {angle}");
        }
    }

    #[test]
    fn the_wind_moves_the_ends_more_than_the_root_and_never_far() {
        assert_eq!(swayed((0.0, 0.0), 3.0, 0.0), (0.0, 0.0));
        let near = swayed((0.5, 0.0), 2.0, 0.0);
        let far = swayed((2.0, 0.0), 2.0, 0.0);
        assert!((far.1).abs() > (near.1).abs());
        for i in 0..100 {
            let (x, y) = swayed((2.5, 1.0), i as f32 * 0.37, 1.3);
            assert!((x - 2.5).abs() < 0.2 && (y - 1.0).abs() < 0.3);
        }
        assert_ne!(swayed((2.0, 1.0), 1.0, 0.0), swayed((2.0, 1.0), 1.0, 2.0));
    }
}
