//! The maths behind the full-window log viewer: which stretch of the log is on show, moving and
//! zooming it, picking the points of that stretch, the numbers over it, and the labels of the time
//! axis. Nothing here draws anything, so all of it is tested.
use crate::sensor_log::{civil_from_unix, stamp};

/// The least time (seconds) the viewer zooms in to.
pub const MIN_SPAN: f64 = 2.0;

/// The stretch of the log that is on show: seconds since the first reading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub lo: f64,
    pub hi: f64,
}

impl Span {
    /// The whole log (`total` seconds long).
    pub fn all(total: f64) -> Span {
        Span { lo: 0.0, hi: total.max(MIN_SPAN) }
    }

    pub fn width(&self) -> f64 {
        self.hi - self.lo
    }

    /// Kept inside the log, and not narrower than `MIN_SPAN` (a log shorter than that is shown whole).
    pub fn clamped(self, total: f64) -> Span {
        let total = total.max(MIN_SPAN);
        let width = self.width().clamp(MIN_SPAN, total);
        let lo = self.lo.clamp(0.0, total - width);
        Span { lo, hi: lo + width }
    }

    /// Zoomed by `factor` (below 1 zooms in) with the moment `around` staying where it is on the screen.
    pub fn zoomed(self, factor: f64, around: f64, total: f64) -> Span {
        let width = self.width() * factor;
        let fraction = ((around - self.lo) / self.width().max(f64::EPSILON)).clamp(0.0, 1.0);
        let lo = around - fraction * width;
        Span { lo, hi: lo + width }.clamped(total)
    }

    /// Moved by `seconds` (positive goes later).
    pub fn panned(self, seconds: f64, total: f64) -> Span {
        Span { lo: self.lo + seconds, hi: self.hi + seconds }.clamped(total)
    }

    /// The same width with `t` in the middle.
    pub fn centred(self, t: f64, total: f64) -> Span {
        let half = self.width() / 2.0;
        Span { lo: t - half, hi: t + half }.clamped(total)
    }

    /// Whether the whole log is on show.
    pub fn is_all(&self, total: f64) -> bool {
        self.lo <= 0.001 && self.hi >= total.max(MIN_SPAN) - 0.001
    }
}

/// The points that are on show: those between `lo` and `hi`, and one more on each side so that a
/// line runs off the edge of the graph instead of stopping short of it.
pub fn visible(points: &[(f64, f32)], lo: f64, hi: f64) -> &[(f64, f32)] {
    let first = points.partition_point(|p| p.0 < lo).saturating_sub(1);
    let last = (points.partition_point(|p| p.0 <= hi) + 1).min(points.len());
    &points[first.min(last)..last]
}

/// (lowest, average, highest) of the points whose time is between `lo` and `hi`.
pub fn stats_in(points: &[(f64, f32)], lo: f64, hi: f64) -> Option<(f32, f32, f32)> {
    let start = points.partition_point(|p| p.0 < lo);
    let end = points.partition_point(|p| p.0 <= hi);
    let slice = points.get(start..end)?;
    if slice.is_empty() {
        return None;
    }
    let min = slice.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let max = slice.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
    let avg = (slice.iter().map(|p| f64::from(p.1)).sum::<f64>() / slice.len() as f64) as f32;
    Some((min, avg, max))
}

/// The point whose time is closest to `t`.
pub fn nearest(points: &[(f64, f32)], t: f64) -> Option<(f64, f32)> {
    if points.is_empty() {
        return None;
    }
    let after = points.partition_point(|p| p.0 < t);
    let before = after.checked_sub(1).map(|i| points[i]);
    let next = points.get(after).copied();
    match (before, next) {
        (Some(a), Some(b)) => Some(if (t - a.0).abs() <= (b.0 - t).abs() { a } else { b }),
        (Some(a), None) => Some(a),
        (None, b) => b,
    }
}

/// At most about `2 * buckets` points: the points are cut into `buckets` runs and each run keeps its
/// lowest and its highest, in time order, so that a spike that lasts one reading is not averaged away.
pub fn downsample(points: &[(f64, f32)], buckets: usize) -> Vec<(f64, f32)> {
    if buckets == 0 || points.len() <= buckets * 2 {
        return points.to_vec();
    }
    let per = points.len().div_ceil(buckets);
    let mut out = Vec::with_capacity(buckets * 2 + 2);
    for chunk in points.chunks(per) {
        let (mut lo, mut hi) = (0usize, 0usize);
        for (i, p) in chunk.iter().enumerate() {
            if p.1 < chunk[lo].1 {
                lo = i;
            }
            if p.1 > chunk[hi].1 {
                hi = i;
            }
        }
        if lo == hi {
            out.push(chunk[lo]);
        } else {
            out.push(chunk[lo.min(hi)]);
            out.push(chunk[lo.max(hi)]);
        }
    }
    out
}

/// The steps the time axis moves in (seconds).
const STEPS: [f64; 19] = [
    1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, 3600.0, 7200.0,
    10_800.0, 21_600.0, 43_200.0, 86_400.0, 172_800.0,
];

/// The step of the time axis for `span` seconds with about `target` labels.
pub fn tick_step(span: f64, target: usize) -> f64 {
    let wanted = span / target.max(1) as f64;
    STEPS.iter().copied().find(|s| *s >= wanted).unwrap_or(STEPS[STEPS.len() - 1])
}

/// Where the labels of the time axis go between `lo` and `hi`: on round clock times (`start_unix` is
/// the clock time of the first reading, 0 to count elapsed time instead).
pub fn time_ticks(start_unix: i64, lo: f64, hi: f64, target: usize) -> Vec<f64> {
    let step = tick_step(hi - lo, target);
    let offset = start_unix as f64;
    let mut tick = ((lo + offset) / step).ceil() * step - offset;
    let mut ticks = Vec::new();
    while tick <= hi + 1e-6 && ticks.len() < 64 {
        ticks.push(tick);
        tick += step;
    }
    ticks
}

/// The clock time `t` seconds into a log that began at `start_unix`: `04:27:41`, or `04:27` when
/// `seconds` is false.
pub fn clock_text(start_unix: i64, t: f64, seconds: bool) -> String {
    let full = stamp(civil_from_unix(start_unix + t.round() as i64));
    let time = &full[11..];
    if seconds {
        time.to_string()
    } else {
        time[..5].to_string()
    }
}

/// The date `t` seconds into the log: `2026-10-20`.
pub fn date_text(start_unix: i64, t: f64) -> String {
    stamp(civil_from_unix(start_unix + t.round() as i64))[..10].to_string()
}

/// Time since the first reading: `+1:05`, `+1:02:03`.
pub fn elapsed_text(t: f64) -> String {
    let s = t.max(0.0).round() as u64;
    if s >= 3600 {
        format!("+{}:{:02}:{:02}", s / 3600, s % 3600 / 60, s % 60)
    } else {
        format!("+{}:{:02}", s / 60, s % 60)
    }
}

/// How long a stretch is, in words: `45 s`, `12 min 30 s`, `3 h 5 min`.
pub fn length_text(seconds: f64) -> String {
    let s = seconds.round().max(0.0) as u64;
    match s {
        0..=59 => format!("{} s", s),
        60..=3599 => format!("{} min {} s", s / 60, s % 60),
        _ => format!("{} h {} min", s / 3600, s % 3600 / 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line() -> Vec<(f64, f32)> {
        (0..100).map(|i| (f64::from(i) * 2.0, i as f32)).collect()
    }

    #[test]
    fn the_span_stays_inside_the_log_and_never_gets_too_narrow() {
        let s = Span::all(600.0);
        assert!(s.is_all(600.0));
        assert_eq!(s.panned(50.0, 600.0), s, "nowhere to go while everything is on show");
        let z = s.zoomed(0.5, 300.0, 600.0);
        assert_eq!((z.lo, z.hi), (150.0, 450.0));
        assert!(!z.is_all(600.0));
        // zooming in a long way stops at the least span
        let mut t = s;
        for _ in 0..40 {
            t = t.zoomed(0.5, 10.0, 600.0);
        }
        assert!((t.width() - MIN_SPAN).abs() < 1e-9);
        // panning stops at both ends
        let p = z.panned(1000.0, 600.0);
        assert_eq!(p.hi, 600.0);
        assert_eq!(p.width(), 300.0);
        assert_eq!(z.panned(-1000.0, 600.0).lo, 0.0);
        // zooming out comes back to the whole
        assert!(z.zoomed(8.0, 300.0, 600.0).is_all(600.0));
        // a very short log is shown whole
        assert_eq!(Span::all(0.5).clamped(0.5), Span { lo: 0.0, hi: MIN_SPAN });
    }

    #[test]
    fn the_moment_under_the_cursor_stays_put_when_zooming() {
        let s = Span { lo: 100.0, hi: 400.0 };
        let around = 160.0;
        let before = (around - s.lo) / s.width();
        let z = s.zoomed(0.4, around, 1000.0);
        let after = (around - z.lo) / z.width();
        assert!((before - after).abs() < 1e-9);
        // and the span can be centred on a moment
        let c = s.centred(500.0, 1000.0);
        assert_eq!((c.lo, c.hi), (350.0, 650.0));
        assert_eq!(s.centred(0.0, 1000.0).lo, 0.0);
    }

    #[test]
    fn the_visible_points_run_off_the_edges() {
        let l = line();
        let v = visible(&l, 50.0, 60.0);
        assert_eq!(v.first().map(|p| p.0), Some(48.0));
        assert_eq!(v.last().map(|p| p.0), Some(62.0));
        assert_eq!(visible(&l, -10.0, 1000.0).len(), l.len());
        assert!(visible(&[], 0.0, 1.0).is_empty());
        // beyond the end: just the last point
        assert_eq!(visible(&l, 500.0, 600.0).len(), 1);
    }

    #[test]
    fn the_numbers_over_a_stretch_cover_only_that_stretch() {
        let l = line();
        assert_eq!(stats_in(&l, 10.0, 20.0), Some((5.0, 7.5, 10.0)));
        assert_eq!(stats_in(&l, 0.0, 1000.0).map(|s| s.2), Some(99.0));
        assert_eq!(stats_in(&l, 1.0, 1.5), None);
        assert_eq!(stats_in(&[], 0.0, 1.0), None);
    }

    #[test]
    fn the_nearest_reading_is_found_from_either_side() {
        let l = line();
        assert_eq!(nearest(&l, 7.0), Some((6.0, 3.0)));
        assert_eq!(nearest(&l, 7.1), Some((8.0, 4.0)));
        assert_eq!(nearest(&l, -50.0), Some((0.0, 0.0)));
        assert_eq!(nearest(&l, 9999.0), Some((198.0, 99.0)));
        assert_eq!(nearest(&[], 1.0), None);
    }

    #[test]
    fn downsampling_keeps_the_spikes() {
        let mut l: Vec<(f64, f32)> = (0..10_000).map(|i| (f64::from(i), 50.0)).collect();
        l[4321].1 = 99.0;
        l[7777].1 = 1.0;
        let d = downsample(&l, 200);
        assert!(d.len() <= 400 + 2, "{}", d.len());
        assert!(d.iter().any(|p| p.1 == 99.0) && d.iter().any(|p| p.1 == 1.0));
        assert!(d.windows(2).all(|w| w[0].0 <= w[1].0), "still in time order");
        // little data is left alone
        assert_eq!(downsample(&line(), 200), line());
    }

    #[test]
    fn the_time_axis_falls_on_round_clock_times() {
        // a log that began at 04:27:41
        let start = 1_790_000_000i64;
        let begin = (start % 86_400) as f64;
        for target in [3usize, 6, 10] {
            let ticks = time_ticks(start, 0.0, 1800.0, target);
            assert!(!ticks.is_empty() && ticks.len() <= target + 2, "{target}: {}", ticks.len());
            let step = ticks[1] - ticks[0];
            for t in &ticks {
                assert!(
                    ((t + begin) / step - ((t + begin) / step).round()).abs() < 1e-6,
                    "{t} is not on a multiple of {step}"
                );
                assert!(*t >= 0.0 && *t <= 1800.0);
            }
        }
        // elapsed time counts from the start
        let e = time_ticks(0, 0.0, 120.0, 6);
        assert_eq!(e[0], 0.0);
        assert_eq!(tick_step(120.0, 6), 30.0);
        assert!(tick_step(1.0e9, 6) >= 86_400.0);
    }

    #[test]
    fn times_are_written_as_clock_and_as_elapsed() {
        let start = crate::sensor_log::parse_stamp("2026-10-20 04:27:41").unwrap();
        assert_eq!(clock_text(start, 0.0, true), "04:27:41");
        assert_eq!(clock_text(start, 19.0, true), "04:28:00");
        assert_eq!(clock_text(start, 19.0, false), "04:28");
        assert_eq!(date_text(start, 0.0), "2026-10-20");
        assert_eq!(date_text(start, 3600.0 * 20.0), "2026-10-21", "past midnight");
        assert_eq!(elapsed_text(65.0), "+1:05");
        assert_eq!(elapsed_text(3723.0), "+1:02:03");
        assert_eq!(length_text(45.0), "45 s");
        assert_eq!(length_text(750.0), "12 min 30 s");
        assert_eq!(length_text(11_100.0), "3 h 5 min");
    }
}
