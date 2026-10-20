//! Colour and geometry for the shop skins' animated card edges (Prism and Aurora) and the Prism
//! accent. Pure, so it is tested; `theme.rs` draws it.

/// A colour from hue (0 to 1, round the wheel), saturation and value (both 0 to 1).
pub fn hsv(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let (s, v) = (s.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
    let f = h - h.floor();
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    let (r, g, b) = match h as u32 % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [(r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8]
}

/// How long Prism takes to go once round the colour wheel, in seconds.
pub const PRISM_CYCLE: f32 = 60.0;
/// The hue is held in this many steps, so the accent (and the theme built from it) only changes
/// that often.
pub const PRISM_STEPS: f32 = 120.0;

/// Prism's hue at time `t`, in `PRISM_STEPS` steps.
pub fn prism_hue(t: f32) -> f32 {
    ((t / PRISM_CYCLE).rem_euclid(1.0) * PRISM_STEPS).floor() / PRISM_STEPS
}

/// A point `t` (0 to 1) of the way clockwise round the outline of a `w` by `h` rectangle, starting
/// at the top left.
pub fn perimeter_point(w: f32, h: f32, t: f32) -> (f32, f32) {
    let total = 2.0 * (w + h);
    let d = t.rem_euclid(1.0) * total;
    if d < w {
        (d, 0.0)
    } else if d < w + h {
        (w, d - w)
    } else if d < 2.0 * w + h {
        (w - (d - w - h), h)
    } else {
        (0.0, h - (d - 2.0 * w - h))
    }
}

/// The outline cut into `n` pieces: each is its two ends and how far round it is (0 to 1).
pub fn outline_pieces(w: f32, h: f32, n: usize) -> Vec<((f32, f32), (f32, f32), f32)> {
    (0..n)
        .map(|i| {
            let (a, b) = (i as f32 / n as f32, (i + 1) as f32 / n as f32);
            (perimeter_point(w, h, a), perimeter_point(w, h, b), (a + b) / 2.0)
        })
        .collect()
}

/// Prism's edge colour: the colour wheel laid round the card, turning slowly.
pub fn prism_edge(t_along: f32, time: f32) -> [u8; 3] {
    hsv(t_along + time / PRISM_CYCLE, 0.75, 1.0)
}

/// Aurora's edge colour: green at one end of the card sliding to violet at the other and back, the
/// shimmer drifting round slowly.
pub fn aurora_edge(t_along: f32, time: f32) -> [u8; 3] {
    let wave = ((t_along * 2.0 + time / 14.0) * std::f32::consts::TAU).sin() * 0.5 + 0.5;
    let (a, b) = ([60.0, 240.0, 150.0], [150.0, 90.0, 255.0]);
    [0, 1, 2].map(|k| (a[k] + (b[k] - a[k]) * wave).round() as u8)
}

/// Bubbles on a `w` by `h` card for the Deep Sea skin: a string of small ones along the bottom edge
/// and a column rising up the right side, getting bigger as they go. Each is (across, down, radius),
/// measured from the card's top left, and sits on the card.
pub fn card_bubbles(w: f32, h: f32) -> Vec<(f32, f32, f32)> {
    let seed = (w as u32).wrapping_mul(37).wrapping_add(h as u32);
    let rnd = |i: u32, salt: u32| {
        let mut x = i.wrapping_mul(2_654_435_761).wrapping_add(salt.wrapping_mul(40_503)) | 1;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        (x >> 8) as f32 / (1u32 << 24) as f32
    };
    let mut out = Vec::new();
    let mut x = 12.0;
    let mut k = 0;
    while x < w - 40.0 {
        let r = 1.6 + rnd(seed, k) * 2.6;
        out.push((x, h - 2.0 - r - rnd(seed, 50 + k) * 3.0, r));
        x += 14.0 + rnd(seed, 100 + k) * 20.0;
        k += 1;
    }
    let rising = ((h - 30.0) / 18.0).clamp(0.0, 5.0) as u32;
    for j in 0..rising {
        let r = 1.8 + j as f32 * 0.55;
        out.push((w - 14.0 + (rnd(seed, 200 + j) - 0.5) * 8.0, h - 16.0 - j as f32 * 18.0, r));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_gives_the_primary_colours_and_greys() {
        assert_eq!(hsv(0.0, 1.0, 1.0), [255, 0, 0]);
        assert_eq!(hsv(1.0 / 3.0, 1.0, 1.0), [0, 255, 0]);
        assert_eq!(hsv(2.0 / 3.0, 1.0, 1.0), [0, 0, 255]);
        assert_eq!(hsv(0.5, 0.0, 0.5), [128, 128, 128]);
        assert_eq!(hsv(1.0, 1.0, 1.0), hsv(0.0, 1.0, 1.0), "the wheel closes");
        assert_eq!(hsv(-0.5, 1.0, 1.0), hsv(0.5, 1.0, 1.0));
    }

    #[test]
    fn prism_goes_round_once_a_minute_in_steps() {
        assert_eq!(prism_hue(0.0), 0.0);
        assert!((prism_hue(PRISM_CYCLE / 2.0) - 0.5).abs() < 0.01);
        assert_eq!(prism_hue(PRISM_CYCLE), 0.0, "and starts again");
        let distinct: std::collections::BTreeSet<u32> = (0..600).map(|i| (prism_hue(i as f32 * 0.1) * 1000.0) as u32).collect();
        assert_eq!(distinct.len(), PRISM_STEPS as usize);
        assert_eq!(prism_hue(1.0), prism_hue(1.2), "it holds between steps");
    }

    #[test]
    fn the_outline_runs_clockwise_from_the_top_left_and_closes() {
        let (w, h) = (200.0, 100.0);
        assert_eq!(perimeter_point(w, h, 0.0), (0.0, 0.0));
        assert_eq!(perimeter_point(w, h, 200.0 / 600.0), (200.0, 0.0));
        assert_eq!(perimeter_point(w, h, 300.0 / 600.0), (200.0, 100.0));
        assert_eq!(perimeter_point(w, h, 500.0 / 600.0), (0.0, 100.0));
        let end = perimeter_point(w, h, 0.99999);
        assert!(end.0.abs() < 0.1 && end.1 < 0.1 + 0.0 || (end.0 < 0.1 && end.1 > 99.0), "{end:?}");
    }

    #[test]
    fn the_pieces_join_end_to_end_all_the_way_round() {
        let pieces = outline_pieces(300.0, 120.0, 90);
        assert_eq!(pieces.len(), 90);
        for pair in pieces.windows(2) {
            assert_eq!(pair[0].1, pair[1].0);
        }
        assert_eq!(pieces[0].0, pieces[89].1, "the last piece ends where the first begins");
        assert!(pieces.windows(2).all(|p| p[1].2 > p[0].2));
    }

    #[test]
    fn the_edges_move_with_time_and_stay_in_their_colours() {
        assert_ne!(prism_edge(0.3, 0.0), prism_edge(0.3, 10.0));
        assert_eq!(prism_edge(0.3, 0.0), prism_edge(0.3, PRISM_CYCLE));
        for i in 0..50 {
            let c = aurora_edge(i as f32 / 50.0, i as f32);
            assert!(c[0] >= 60 && c[0] <= 150 && c[1] >= 90 && c[1] <= 240 && c[2] >= 150 && c[2] <= 255, "{c:?}");
        }
        assert_ne!(aurora_edge(0.0, 0.0), aurora_edge(0.125, 0.0));
    }

    #[test]
    fn the_bubbles_sit_on_the_card_along_the_bottom_and_up_the_right() {
        for (w, h) in [(544.0, 60.0), (544.0, 140.0), (380.0, 300.0), (200.0, 40.0)] {
            let b = card_bubbles(w, h);
            assert!(b.len() >= 4, "{w}x{h}");
            for (x, y, r) in &b {
                assert!(*r >= 1.6 && *r <= 5.0 && *x - r >= 0.0 && *x + r <= w && *y - r >= 0.0 && *y + r <= h, "{x} {y} {r} on {w}x{h}");
            }
        }
        let tall = card_bubbles(400.0, 200.0);
        let column: Vec<_> = tall.iter().filter(|(x, _, _)| *x > 380.0).collect();
        assert_eq!(column.len(), 5);
        assert!(column.windows(2).all(|p| p[1].2 > p[0].2 && p[1].1 < p[0].1), "bigger and higher");
        assert_eq!(card_bubbles(400.0, 200.0), tall);
    }
}
