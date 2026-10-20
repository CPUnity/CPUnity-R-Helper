//! Angel cats, the Gold Edition's backdrop: a scatter of small winged cats drifting slowly down
//! from above, each with a faint streak of light behind it, the far ones small and dim, the near
//! ones big, bright and quicker.
//!
//! The picture (`assets/backdrop-angels.png`, baked by tools/angel_atlas.py) is three cats side by
//! side (an archer, a harpist and a trumpeter), drawn in full colour: it is not tinted with the
//! accent colour. This file works out, from the size of the view and the time, where every cat is,
//! how big and how bright. Nothing here touches the screen, and the same view and time always give
//! the same picture, so with the animation off (time 0) the cats simply stand where they are.

pub type Pt = (f32, f32);

/// Size of one cat's cell in the picture, in pixels. The picture is `POSES` cells side by side.
pub const CELL: Pt = (417.0, 440.0);
/// How many different cats the picture holds.
pub const POSES: usize = 3;
/// How many cats are in the sky at once.
pub const COUNT: usize = 13;
/// Smallest and biggest height of a cat, in points.
pub const MIN_HEIGHT: f32 = 56.0;
pub const MAX_HEIGHT: f32 = 230.0;
/// Slowest and quickest fall, in points a second (the far cats are slow, the near ones quicker).
pub const SLOWEST: f32 = 14.0;
pub const QUICKEST: f32 = 44.0;
/// A cat fades in over the first part of its way down, and out over the last.
const FADE: f32 = 0.12;

/// One cat to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angel {
    /// Which cat of the sky it is (0..COUNT); the same cat always keeps its number.
    pub id: usize,
    /// Which cell of the picture (0..POSES).
    pub pose: usize,
    /// Middle of the cat, in points from the view's top left.
    pub centre: Pt,
    /// Width and height to draw it at, in points.
    pub size: Pt,
    /// How bright it is (0..1): near cats are brighter, and all fade at the top and the bottom.
    pub alpha: f32,
    /// How long its streak of light is, as a part of its height.
    pub trail: f32,
    /// 0 for the farthest cat, 1 for the nearest. They are drawn far first.
    pub depth: f32,
}

fn frac(x: f32) -> f32 {
    x - x.floor()
}

/// 0 at the ends, 1 in the middle: a smooth fade in and out over `FADE` of the way.
fn envelope(s: f32) -> f32 {
    let e = (s.min(1.0 - s) / FADE).clamp(0.0, 1.0);
    e * e * (3.0 - 2.0 * e)
}

/// The part of the picture (u0, v0, u1, v1) that holds cat `pose`.
pub fn uv(pose: usize) -> [f32; 4] {
    let n = POSES as f32;
    let p = (pose % POSES) as f32;
    [p / n, 0.0, (p + 1.0) / n, 1.0]
}

/// Where every cat is in a view `view` points big at `t` seconds, drawn far first. An empty or
/// invisibly small view has none.
pub fn placements(view: Pt, t: f32) -> Vec<Angel> {
    if !(view.0 >= 8.0 && view.1 >= 8.0) {
        return Vec::new();
    }
    let t = if t.is_finite() { t } else { 0.0 };
    let mut angels: Vec<Angel> = (0..COUNT)
        .map(|i| {
            let n = i as f32;
            // three low-discrepancy sequences: spread evenly, never in rows or columns
            let lane = frac(0.137 + n * 0.618_034);
            let depth = frac(0.31 + n * 0.754_878);
            let phase = frac(0.05 + n * 0.324_718);
            let height = (view.1 * (0.13 + 0.17 * depth)).clamp(MIN_HEIGHT, MAX_HEIGHT);
            let width = height * CELL.0 / CELL.1;
            // the way down runs from just above the view to just below it
            let travel = view.1 + height;
            let speed = SLOWEST + (QUICKEST - SLOWEST) * depth;
            let period = travel / speed;
            let s = frac(phase + t / period);
            let bob = (t * 0.7 + n * 1.9).sin() * 4.0;
            let sway = (t * 0.16 / (1.0 + depth) + n * 2.3).sin() * (0.015 * view.0 + 6.0);
            let (lo, hi) = (width * 0.3, view.0 - width * 0.3);
            let x = if hi > lo { lo + lane * (hi - lo) } else { view.0 / 2.0 } + sway;
            Angel {
                id: i,
                pose: i % POSES,
                centre: (x, -height * 0.5 + s * travel + bob),
                size: (width, height),
                alpha: (0.45 + 0.55 * depth) * envelope(s),
                trail: 0.6 + 0.4 * depth,
                depth,
            }
        })
        .collect();
    angels.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    angels
}

// ---- inside The Void ----------------------------------------------------------------------------
//
// There the cats are dark bronze, with hieroglyphs on their instruments and dead black eyes with a
// small white X in each. The picture (`assets/backdrop-angels-void.png`) is twice as tall as the
// gold one: the top half is the cats with plain black eyes, the bottom half is only the crosses, so
// that they can be left out. And the cats glitch: now and then a cat's picture is torn into strips
// that slip sideways, and now and then the crosses glitch out, leave the eyes black for a moment,
// and come back.

/// The part of the Void picture (u0, v0, u1, v1) that holds cat `pose` (`crosses` false) or only
/// the crosses of its eyes (`crosses` true).
pub fn uv_void(pose: usize, crosses: bool) -> [f32; 4] {
    let [u0, _, u1, _] = uv(pose);
    if crosses {
        [u0, 0.5, u1, 1.0]
    } else {
        [u0, 0.0, u1, 0.5]
    }
}

/// A number from 0 to 1 that is always the same for the same `a` and `b`.
fn hash(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B).wrapping_add(0x27D4_EB2F);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0xFF_FFFF) as f32 / 0x100_0000 as f32
}

/// One strip of a cat that has slipped sideways: from `top` to `bottom` (0 to 1 of its height),
/// moved `shift` of its width (negative is left).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tear {
    pub top: f32,
    pub bottom: f32,
    pub shift: f32,
}

/// How a cat is glitching right now.
#[derive(Debug, Clone, PartialEq)]
pub struct Glitch {
    /// The strips that are out of place, top to bottom, never overlapping.
    pub tears: Vec<Tear>,
    /// How bright it is for the moment (it flickers): 0.5 to 1.
    pub flicker: f32,
}

/// Seconds a cat goes between one glitch and the next, on average, and how long one lasts.
const GLITCH_EVERY: f32 = 2.4;

/// Whether cat `id` is glitching `t` seconds in, and how. Each cat has its own rhythm, so they do
/// not all do it together, and a glitch is a few frames of torn strips (a new picture 20 times a
/// second) that dies away.
pub fn glitch(id: usize, t: f32) -> Option<Glitch> {
    if !(t.is_finite() && t > 0.0) {
        return None;
    }
    let id = id as u32;
    let period = GLITCH_EVERY * (0.7 + 0.9 * hash(id, 1));
    let shifted = t + period * hash(id, 2);
    let cycle = (shifted / period).floor();
    let s = shifted - cycle * period;
    let length = 0.14 + 0.3 * hash(id * 31 + cycle as u32, 3);
    if s >= length {
        return None;
    }
    let frame = (t * 20.0) as u32;
    let key = id.wrapping_mul(7919).wrapping_add(frame);
    // fewer strips as it dies away
    let strength = 1.0 - s / length;
    let count = 1 + (strength * 3.0 * hash(key, 4)) as usize;
    let mut tears = Vec::new();
    let mut cursor = 0.0_f32;
    for k in 0..count {
        let top = cursor + (1.0 - cursor) * 0.5 * hash(key, 10 + k as u32);
        let bottom = (top + 0.03 + 0.14 * hash(key, 20 + k as u32)).min(1.0);
        if bottom - top < 0.02 {
            break;
        }
        let side = if hash(key, 30 + k as u32) < 0.5 { -1.0 } else { 1.0 };
        tears.push(Tear { top, bottom, shift: side * (0.03 + 0.13 * hash(key, 40 + k as u32)) * (0.5 + 0.5 * strength) });
        cursor = bottom;
    }
    Some(Glitch { tears, flicker: 0.5 + 0.5 * hash(key, 5) })
}

/// How much of the crosses in cat `id`'s eyes shows at time `t` (0 to 1): all of it, nearly always.
/// Every so often they flicker, glitch out so that only black eyes are left for a moment, flicker
/// again and come back.
pub fn crosses(id: usize, t: f32) -> f32 {
    if !(t.is_finite() && t > 0.0) {
        return 1.0;
    }
    let id = id as u32;
    let period = 4.5 + 4.5 * hash(id, 11);
    let shifted = t + period * hash(id, 12);
    let cycle = (shifted / period).floor();
    let s = shifted - cycle * period;
    // the last second of every round: flicker, black, flicker
    let from = period - 1.0;
    if s < from {
        return 1.0;
    }
    let local = s - from;
    let frame = (t * 24.0) as u32;
    let flick = |salt: u32| if hash(id.wrapping_add(frame.wrapping_mul(131)), salt) < 0.5 { 1.0 } else { 0.0 };
    if local < 0.3 {
        flick(13)
    } else if local < 0.75 {
        0.0
    } else {
        flick(14)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: Pt = (1000.0, 700.0);

    fn png_size(png: &[u8]) -> (u32, u32) {
        assert_eq!(&png[1..4], b"PNG");
        let be = |at: usize| u32::from_be_bytes([png[at], png[at + 1], png[at + 2], png[at + 3]]);
        (be(16), be(20))
    }

    #[test]
    fn the_picture_is_the_cells_side_by_side() {
        let pngs = crate::info::BackdropStyle::AngelCats.pngs();
        assert_eq!(pngs.len(), 1);
        assert_eq!(png_size(pngs[0]), ((CELL.0 as u32) * POSES as u32, CELL.1 as u32));
    }

    #[test]
    fn the_void_picture_holds_the_cats_over_their_crosses() {
        let dead = crate::info::BackdropStyle::AngelCats.pngs_in(true);
        assert_eq!(dead.len(), 1);
        assert_eq!(png_size(dead[0]), ((CELL.0 as u32) * POSES as u32, (CELL.1 as u32) * 2));
        for p in 0..POSES {
            let (cat, x) = (uv_void(p, false), uv_void(p, true));
            assert_eq!((cat[0], cat[2]), (uv(p)[0], uv(p)[2]));
            assert_eq!((x[0], x[2]), (cat[0], cat[2]));
            assert_eq!((cat[1], cat[3], x[1], x[3]), (0.0, 0.5, 0.5, 1.0));
        }
    }

    #[test]
    fn every_cat_has_its_number_and_keeps_it() {
        let mut ids: Vec<usize> = placements(VIEW, 5.0).iter().map(|a| a.id).collect();
        ids.sort();
        assert_eq!(ids, (0..COUNT).collect::<Vec<_>>());
    }

    #[test]
    fn cats_glitch_now_and_then_each_in_its_own_time_with_strips_that_do_not_overlap() {
        let mut together = 0;
        let mut any = 0;
        for k in 0..6000 {
            let t = 1.0 + k as f32 * 0.01;
            let now: Vec<usize> = (0..COUNT).filter(|i| glitch(*i, t).is_some()).collect();
            any += !now.is_empty() as u32;
            together += (now.len() == COUNT) as u32;
            for i in now {
                let g = glitch(i, t).unwrap();
                assert!(!g.tears.is_empty() && g.tears.len() <= 4);
                assert!((0.5..=1.0).contains(&g.flicker));
                let mut last = 0.0;
                for tear in &g.tears {
                    assert!(tear.top >= last - 1e-6 && tear.bottom > tear.top && tear.bottom <= 1.0, "{tear:?}");
                    assert!(tear.shift.abs() > 0.0 && tear.shift.abs() <= 0.16 + 1e-6);
                    last = tear.bottom;
                }
            }
        }
        assert!(any > 3000, "there is nearly always a cat glitching: {any}");
        assert_eq!(together, 0, "never all of them at once");
        // one cat: glitching a small part of the time, and the same every time it is asked
        let mine = (0..6000).filter(|k| glitch(3, 1.0 + *k as f32 * 0.01).is_some()).count();
        assert!(mine > 300 && mine < 3000, "{mine}");
        assert_eq!(glitch(3, 12.34), glitch(3, 12.34));
        // standing still (time 0) and odd times: no glitch
        assert_eq!(glitch(3, 0.0), None);
        assert_eq!(glitch(3, f32::NAN), None);
        assert_eq!(glitch(3, -5.0), None);
    }

    #[test]
    fn the_crosses_are_there_nearly_always_and_glitch_out_to_black_eyes_for_a_moment() {
        for id in 0..COUNT {
            let samples: Vec<f32> = (0..20000).map(|k| crosses(id, 1.0 + k as f32 * 0.005)).collect();
            assert!(samples.iter().all(|v| *v == 0.0 || *v == 1.0));
            let gone = samples.iter().filter(|v| **v == 0.0).count() as f32 / samples.len() as f32;
            assert!(gone > 0.03 && gone < 0.2, "cat {id}: {gone}");
            // black for a whole stretch (well over a tenth of a second), not just a blink
            let mut run = 0;
            let mut longest = 0;
            for v in &samples {
                run = if *v == 0.0 { run + 1 } else { 0 };
                longest = longest.max(run);
            }
            assert!(longest as f32 * 0.005 > 0.3, "cat {id}: {}", longest as f32 * 0.005);
        }
        assert_eq!(crosses(2, 0.0), 1.0, "standing still they stay");
        assert_eq!(crosses(2, f32::NAN), 1.0);
    }

    #[test]
    fn the_cells_tile_the_picture() {
        assert_eq!(uv(0)[0], 0.0);
        assert_eq!(uv(POSES - 1)[2], 1.0);
        for p in 1..POSES {
            assert_eq!(uv(p - 1)[2], uv(p)[0], "no gap and no overlap");
        }
        assert_eq!(uv(POSES), uv(0), "a pose past the end wraps instead of leaving the picture");
        assert!(uv(1).iter().all(|v| (0.0..=1.0).contains(v)));
    }

    #[test]
    fn there_are_a_fixed_number_of_cats_and_every_cat_is_used() {
        let a = placements(VIEW, 12.0);
        assert_eq!(a.len(), COUNT);
        for pose in 0..POSES {
            assert!(a.iter().filter(|x| x.pose == pose).count() >= 3, "pose {pose} is common");
        }
    }

    #[test]
    fn the_same_time_gives_the_same_sky() {
        assert_eq!(placements(VIEW, 33.3), placements(VIEW, 33.3));
        assert_ne!(placements(VIEW, 0.0), placements(VIEW, 5.0));
    }

    #[test]
    fn cats_are_drawn_far_first_and_near_ones_are_bigger_and_brighter() {
        let a = placements(VIEW, 7.0);
        assert!(a.windows(2).all(|w| w[0].depth <= w[1].depth));
        assert!(a[0].size.1 < a[COUNT - 1].size.1);
        assert!(a.iter().all(|x| (0.0..=1.0).contains(&x.depth)));
        // alpha at the middle of the way is 0.45..1 by depth
        let mid = a.iter().filter(|x| x.alpha > 0.9 * (0.45 + 0.55 * x.depth)).count();
        assert!(mid >= 4, "plenty are fully faded in: {mid}");
    }

    #[test]
    fn sizes_stay_inside_their_limits_and_keep_the_pictures_shape() {
        for view in [(300.0, 200.0), (1000.0, 700.0), (3840.0, 2160.0)] {
            for x in placements(view, 3.0) {
                assert!(x.size.1 >= MIN_HEIGHT && x.size.1 <= MAX_HEIGHT);
                assert!((x.size.0 / x.size.1 - CELL.0 / CELL.1).abs() < 1e-4);
                assert!((0.0..=1.0).contains(&x.alpha));
                assert!(x.centre.0.is_finite() && x.centre.1.is_finite());
            }
        }
    }

    #[test]
    fn a_good_number_are_on_screen_when_it_stands_still() {
        let seen = placements(VIEW, 0.0)
            .into_iter()
            .filter(|x| x.alpha > 0.25 && x.centre.0 > 0.0 && x.centre.0 < VIEW.0 && x.centre.1 > 0.0 && x.centre.1 < VIEW.1)
            .count();
        assert!(seen >= 6, "only {seen} visible");
    }

    #[test]
    fn they_fall_at_their_own_speed_and_never_climb() {
        let (a, b) = (placements(VIEW, 100.0), placements(VIEW, 101.0));
        let mut moving = 0;
        for (p, q) in a.iter().zip(&b) {
            assert_eq!(p.depth, q.depth, "the same cat in the same place in the list");
            let dy = q.centre.1 - p.centre.1;
            if dy < -100.0 {
                continue; // it reached the bottom and came back in at the top
            }
            // the speed, plus the little bob
            let speed = SLOWEST + (QUICKEST - SLOWEST) * p.depth;
            assert!((dy - speed).abs() < 6.0, "dy {dy} vs {speed}");
            moving += 1;
        }
        assert!(moving >= COUNT - 2);
    }

    #[test]
    fn a_cat_that_leaves_at_the_bottom_is_invisible_and_comes_back_at_the_top() {
        for t in [0.0, 17.0, 60.0, 333.0, 5000.0] {
            for x in placements(VIEW, t) {
                let top = x.centre.1 + x.size.1 / 2.0;
                let bottom = x.centre.1 - x.size.1 / 2.0;
                if top < 0.0 || bottom > VIEW.1 {
                    assert!(x.alpha < 0.05, "off-screen cats cannot pop in: {x:?}");
                }
            }
        }
    }

    #[test]
    fn a_cat_comes_back_to_the_same_place_after_its_whole_way() {
        // the lowest speed's period is the longest: after a common multiple everything repeats,
        // so check one cat alone (the nearest) over its own period
        let a = placements(VIEW, 40.0);
        let near = a[COUNT - 1];
        let period = (VIEW.1 + near.size.1) / (SLOWEST + (QUICKEST - SLOWEST) * near.depth);
        let b = placements(VIEW, 40.0 + period);
        let again = b.iter().find(|x| x.depth == near.depth).unwrap();
        assert!((again.centre.1 - near.centre.1).abs() < 8.0 + 0.1, "back at the same height (bob only)");
        assert!((again.alpha - near.alpha).abs() < 1e-3);
    }

    #[test]
    fn a_missing_or_tiny_view_draws_nothing_and_odd_times_do_not_break_it() {
        assert!(placements((0.0, 0.0), 1.0).is_empty());
        assert!(placements((f32::NAN, 500.0), 1.0).is_empty());
        assert!(placements((4.0, 900.0), 1.0).is_empty());
        let odd = placements(VIEW, f32::NAN);
        assert_eq!(odd, placements(VIEW, 0.0));
        assert_eq!(placements(VIEW, f32::INFINITY), placements(VIEW, 0.0));
        // a very narrow window still places every cat on a finite spot
        assert!(placements((60.0, 900.0), 2.0).iter().all(|x| x.centre.0.is_finite()));
    }
}
