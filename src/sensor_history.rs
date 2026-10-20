//! Recent sensor history for the Sensors graphs: one point per reading round, the last hour.
//! Pure data and maths (no drawing), so it can be tested without a window.
use crate::sensors::Sample;
use std::collections::VecDeque;

/// How much time the history keeps.
pub const HISTORY_SECONDS: f64 = 3600.0;
/// Upper limit of points (an hour of the fastest refresh interval, 4 per second).
pub const MAX_POINTS: usize = 14_400;

/// Every value that is graphed and logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Series {
    CpuTemp,
    CpuCoreTemp,
    CpuZoneTemp,
    CpuPower,
    Pl1,
    Pl2,
    CpuClock,
    CpuClockTop,
    CpuClockP,
    CpuClockE,
    CpuLoad,
    GpuTemp,
    GpuMemTemp,
    GpuPower,
    GpuClock,
    GpuLoad,
}

pub const SERIES_COUNT: usize = 16;

impl Series {
    pub const ALL: [Series; SERIES_COUNT] = [
        Series::CpuTemp,
        Series::CpuCoreTemp,
        Series::CpuZoneTemp,
        Series::CpuPower,
        Series::Pl1,
        Series::Pl2,
        Series::CpuClock,
        Series::CpuClockTop,
        Series::CpuClockP,
        Series::CpuClockE,
        Series::CpuLoad,
        Series::GpuTemp,
        Series::GpuMemTemp,
        Series::GpuPower,
        Series::GpuClock,
        Series::GpuLoad,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            Series::CpuTemp => "CPU package",
            Series::CpuCoreTemp => "Hottest core",
            Series::CpuZoneTemp => "Thermal zone",
            Series::CpuPower => "CPU package",
            Series::Pl1 => "PL1",
            Series::Pl2 => "PL2",
            Series::CpuClock => "CPU (all threads)",
            Series::CpuClockTop => "Fastest core",
            Series::CpuClockP => "P-cores",
            Series::CpuClockE => "E-cores",
            Series::CpuLoad => "CPU",
            Series::GpuTemp => "GPU",
            Series::GpuMemTemp => "GPU memory junction",
            Series::GpuPower => "GPU",
            Series::GpuClock => "GPU",
            Series::GpuLoad => "GPU",
        }
    }

    /// A name that is clear on its own (the chart legends rely on the chart title instead).
    pub fn full_label(self) -> &'static str {
        match self {
            Series::CpuTemp => "CPU package temperature",
            Series::CpuCoreTemp => "CPU core temperature (hottest core)",
            Series::CpuZoneTemp => "Hottest thermal zone",
            Series::CpuPower => "CPU package power",
            Series::Pl1 => "PL1 (long term limit)",
            Series::Pl2 => "PL2 (short term limit)",
            Series::CpuClock => "CPU clock (all threads)",
            Series::CpuClockTop => "CPU clock, fastest core",
            Series::CpuClockP => "CPU clock, performance cores",
            Series::CpuClockE => "CPU clock, efficiency cores",
            Series::CpuLoad => "CPU load",
            Series::GpuTemp => "GPU temperature",
            Series::GpuMemTemp => "GPU memory junction",
            Series::GpuPower => "GPU power",
            Series::GpuClock => "GPU clock",
            Series::GpuLoad => "GPU load",
        }
    }

    /// Unit shown after a value.
    pub fn unit(self) -> &'static str {
        match self {
            Series::CpuTemp
            | Series::CpuCoreTemp
            | Series::CpuZoneTemp
            | Series::GpuTemp
            | Series::GpuMemTemp => "\u{b0}C",
            Series::CpuPower | Series::Pl1 | Series::Pl2 | Series::GpuPower => "W",
            Series::CpuClock
            | Series::CpuClockTop
            | Series::CpuClockP
            | Series::CpuClockE
            | Series::GpuClock => "MHz",
            Series::CpuLoad | Series::GpuLoad => "%",
        }
    }

    /// Name of the column in the CSV log (with the unit).
    pub fn column(self) -> &'static str {
        match self {
            Series::CpuTemp => "cpu_package_temp_c",
            Series::CpuCoreTemp => "cpu_core_max_temp_c",
            Series::CpuZoneTemp => "cpu_thermal_zone_c",
            Series::CpuPower => "cpu_package_power_w",
            Series::Pl1 => "cpu_pl1_w",
            Series::Pl2 => "cpu_pl2_w",
            Series::CpuClock => "cpu_clock_mhz",
            Series::CpuClockTop => "cpu_clock_fastest_core_mhz",
            Series::CpuClockP => "cpu_clock_p_cores_mhz",
            Series::CpuClockE => "cpu_clock_e_cores_mhz",
            Series::CpuLoad => "cpu_load_pct",
            Series::GpuTemp => "gpu_temp_c",
            Series::GpuMemTemp => "gpu_memory_junction_c",
            Series::GpuPower => "gpu_power_w",
            Series::GpuClock => "gpu_clock_mhz",
            Series::GpuLoad => "gpu_load_pct",
        }
    }

    pub fn value(self, s: &Sample) -> Option<f32> {
        match self {
            Series::CpuTemp => s.cpu_temp,
            Series::CpuCoreTemp => s.core_temp,
            Series::CpuZoneTemp => s.cpu_zone_temp,
            Series::CpuPower => s.cpu_power,
            Series::Pl1 => s.pl1,
            Series::Pl2 => s.pl2,
            Series::CpuClock => s.cpu_clock,
            Series::CpuClockTop => s.cpu_clock_top,
            Series::CpuClockP => s.cpu_clock_p,
            Series::CpuClockE => s.cpu_clock_e,
            Series::CpuLoad => s.cpu_load,
            Series::GpuTemp => s.gpu_temp,
            Series::GpuMemTemp => s.gpu_mem_temp,
            Series::GpuPower => s.gpu_power,
            Series::GpuClock => s.gpu_clock,
            Series::GpuLoad => s.gpu_load,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Point {
    /// Seconds since the sensors were switched on.
    pub t: f64,
    pub values: [Option<f32>; SERIES_COUNT],
}

#[derive(Debug, Clone, Default)]
pub struct History {
    points: VecDeque<Point>,
    /// Keeps everything (a loaded log) instead of the last hour.
    unlimited: bool,
}

impl History {
    /// A history that keeps every point, for viewing a whole log file.
    pub fn for_log() -> Self {
        Self { points: VecDeque::new(), unlimited: true }
    }

    pub fn push(&mut self, t: f64, sample: &Sample) {
        let mut values = [None; SERIES_COUNT];
        for series in Series::ALL {
            values[series.index()] = series.value(sample).filter(|v| v.is_finite());
        }
        self.push_values(t, values);
    }

    pub fn push_values(&mut self, t: f64, values: [Option<f32>; SERIES_COUNT]) {
        self.points.push_back(Point { t, values });
        // keep the last hour, however often readings are taken
        while !self.unlimited
            && (self.points.front().map_or(false, |p| t - p.t > HISTORY_SECONDS)
                || self.points.len() > MAX_POINTS)
        {
            self.points.pop_front();
        }
    }

    /// (lowest, average, highest) of a series over everything kept; None without values.
    pub fn stats(&self, series: Series) -> Option<(f32, f32, f32)> {
        let values: Vec<f32> =
            self.points.iter().filter_map(|p| p.values[series.index()]).collect();
        if values.is_empty() {
            return None;
        }
        let min = values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let avg = (values.iter().map(|v| f64::from(*v)).sum::<f64>() / values.len() as f64) as f32;
        Some((min, avg, max))
    }

    /// Seconds from the first to the last point.
    pub fn duration(&self) -> f64 {
        match (self.points.front(), self.points.back()) {
            (Some(a), Some(b)) => (b.t - a.t).max(0.0),
            _ => 0.0,
        }
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Time of the newest point.
    pub fn latest_time(&self) -> Option<f64> {
        self.points.back().map(|p| p.t)
    }

    /// Every value of one series as (seconds since the first point, value) pairs, oldest first.
    /// Points where the value was missing are left out.
    pub fn all(&self, series: Series) -> Vec<(f64, f32)> {
        let Some(first) = self.points.front().map(|p| p.t) else {
            return Vec::new();
        };
        self.points
            .iter()
            .filter_map(|p| p.values[series.index()].map(|v| (p.t - first, v)))
            .collect()
    }

    /// The values of one series in the last `seconds`, as (seconds before now, value) pairs, oldest
    /// first. Rounds where the value was missing are left out.
    pub fn series(&self, series: Series, seconds: f64) -> Vec<(f64, f32)> {
        let Some(now) = self.latest_time() else {
            return Vec::new();
        };
        self.points
            .iter()
            .filter(|p| now - p.t <= seconds)
            .filter_map(|p| p.values[series.index()].map(|v| (p.t - now, v)))
            .collect()
    }
}

/// Averages neighbouring points so that at most `max` remain (keeps the graph light and smooth).
pub fn decimate(points: &[(f64, f32)], max: usize) -> Vec<(f64, f32)> {
    if max == 0 || points.len() <= max {
        return points.to_vec();
    }
    let per = (points.len() as f64 / max as f64).ceil() as usize;
    points
        .chunks(per)
        .map(|chunk| {
            let n = chunk.len() as f64;
            let t = chunk.iter().map(|p| p.0).sum::<f64>() / n;
            let v = chunk.iter().map(|p| f64::from(p.1)).sum::<f64>() / n;
            (t, v as f32)
        })
        .collect()
}

/// A graph's vertical axis: bounds that sit on round numbers and the tick positions.
#[derive(Debug, Clone, PartialEq)]
pub struct Axis {
    pub lo: f32,
    pub hi: f32,
    pub ticks: Vec<f32>,
}

/// 1, 2 or 5 times a power of ten, the first that is not smaller than `raw`.
fn nice_step(raw: f32) -> f32 {
    if raw <= 0.0 || !raw.is_finite() {
        return 1.0;
    }
    let magnitude = 10f32.powf(raw.log10().floor());
    for factor in [1.0, 2.0, 5.0, 10.0] {
        if magnitude * factor >= raw {
            return magnitude * factor;
        }
    }
    magnitude * 10.0
}

/// The axis for values between `min` and `max`, with about `target` ticks. `min_span` keeps a
/// flat line from filling the whole height with noise.
pub fn axis(min: f32, max: f32, target: usize, min_span: f32) -> Axis {
    let (mut lo, mut hi) = (min.min(max), max.max(min));
    if hi - lo < min_span {
        let mid = (hi + lo) / 2.0;
        lo = mid - min_span / 2.0;
        hi = mid + min_span / 2.0;
    }
    let pad = (hi - lo) * 0.08;
    let step = nice_step((hi - lo + 2.0 * pad) / target.max(1) as f32);
    let lo_t = ((lo - pad) / step).floor() * step;
    let hi_t = ((hi + pad) / step).ceil() * step;
    let mut ticks = Vec::new();
    let mut tick = lo_t;
    while tick <= hi_t + step * 0.001 && ticks.len() < 32 {
        ticks.push(tick);
        tick += step;
    }
    Axis { lo: lo_t, hi: hi_t, ticks }
}

/// Smallest and largest value of a list of (time, value) pairs.
pub fn value_range(points: &[(f64, f32)]) -> Option<(f32, f32)> {
    let mut iter = points.iter().map(|p| p.1);
    let first = iter.next()?;
    Some(iter.fold((first, first), |(lo, hi), v| (lo.min(v), hi.max(v))))
}

/// Value at the time closest to `t` (seconds before now), if one is within `reach` seconds.
pub fn value_near(points: &[(f64, f32)], t: f64, reach: f64) -> Option<(f64, f32)> {
    points
        .iter()
        .min_by(|a, b| (a.0 - t).abs().total_cmp(&(b.0 - t).abs()))
        .filter(|p| (p.0 - t).abs() <= reach)
        .copied()
}

/// "-90 s" / "-5 min" style label for a time before now.
pub fn ago_label(seconds_before: f64) -> String {
    let s = (-seconds_before).max(0.0).round() as u64;
    if s == 0 {
        "now".to_string()
    } else if s < 120 {
        format!("-{} s", s)
    } else if s % 60 == 0 {
        format!("-{} min", s / 60)
    } else {
        format!("-{}:{:02}", s / 60, s % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(temp: Option<f32>, power: Option<f32>) -> Sample {
        Sample { gpu_temp: temp, gpu_power: power, ..Sample::default() }
    }

    #[test]
    fn history_keeps_the_last_hour_and_skips_missing_values() {
        let mut h = History::default();
        assert!(h.is_empty());
        assert!(h.series(Series::GpuTemp, 60.0).is_empty());
        for i in 0..3710 {
            let temp = (i % 7 != 0).then_some(50.0 + (i % 5) as f32);
            h.push(i as f64, &sample(temp, Some(100.0)));
        }
        assert_eq!(h.len(), 3601, "the last hour, both ends included");
        assert_eq!(h.latest_time(), Some(3709.0));
        // missing values are left out, the window is counted back from the newest point
        let temps = h.series(Series::GpuTemp, 59.0);
        assert!(temps.len() < 60 && temps.len() >= 50);
        assert!(temps.iter().all(|(t, _)| *t <= 0.0 && *t >= -59.0));
        assert_eq!(h.series(Series::GpuPower, 59.0).len(), 60);
        assert!(h.series(Series::CpuLoad, 59.0).is_empty());
        h.clear();
        assert!(h.is_empty());

        // fast readings are limited by the point count, not just by time
        for i in 0..(MAX_POINTS + 50) {
            h.push(i as f64 * 0.01, &sample(Some(50.0), None));
        }
        assert_eq!(h.len(), MAX_POINTS);
    }

    #[test]
    fn decimation_averages_down_to_the_limit() {
        let points: Vec<(f64, f32)> = (0..1000).map(|i| (i as f64, (i % 10) as f32)).collect();
        let small = decimate(&points, 100);
        assert!(small.len() <= 100 && small.len() >= 50);
        assert_eq!(decimate(&points, 5000).len(), 1000);
        // a flat signal stays flat
        let flat: Vec<(f64, f32)> = (0..500).map(|i| (i as f64, 42.0)).collect();
        assert!(decimate(&flat, 50).iter().all(|p| (p.1 - 42.0).abs() < 1e-4));
        assert!(decimate(&[], 10).is_empty());
    }

    #[test]
    fn axis_sits_on_round_numbers() {
        let a = axis(43.0, 87.0, 5, 10.0);
        assert!(a.lo <= 43.0 && a.hi >= 87.0);
        assert_eq!(a.lo % 10.0, 0.0);
        assert!(a.ticks.len() >= 3 && a.ticks.len() <= 8);
        assert_eq!(*a.ticks.first().unwrap(), a.lo);
        assert!((*a.ticks.last().unwrap() - a.hi).abs() < 0.01);
        // a flat line still gets a usable span
        let flat = axis(60.0, 60.0, 4, 10.0);
        assert!(flat.hi - flat.lo >= 10.0);
        // nonsense in, something drawable out
        let weird = axis(f32::MAX / 2.0, 0.0, 4, 1.0);
        assert!(weird.ticks.len() <= 32);
        assert_eq!(nice_step(0.0), 1.0);
        assert_eq!(nice_step(7.0), 10.0);
        assert_eq!(nice_step(1.4), 2.0);
    }

    #[test]
    fn range_nearest_value_and_labels() {
        let pts = [(-30.0, 40.0), (-20.0, 55.0), (-10.0, 47.0), (0.0, 50.0)];
        assert_eq!(value_range(&pts), Some((40.0, 55.0)));
        assert_eq!(value_range(&[]), None);
        assert_eq!(value_near(&pts, -12.0, 5.0), Some((-10.0, 47.0)));
        assert_eq!(value_near(&pts, -100.0, 5.0), None);
        assert_eq!(ago_label(0.0), "now");
        assert_eq!(ago_label(-45.0), "-45 s");
        assert_eq!(ago_label(-300.0), "-5 min");
        assert_eq!(ago_label(-150.0), "-2:30");
    }

    #[test]
    fn all_gives_every_value_counted_from_the_first_point() {
        let mut h = History::for_log();
        assert!(h.all(Series::GpuTemp).is_empty());
        h.push(100.0, &sample(Some(50.0), None));
        h.push(102.0, &sample(None, Some(7.0)));
        h.push(105.0, &sample(Some(52.0), Some(8.0)));
        assert_eq!(h.all(Series::GpuTemp), vec![(0.0, 50.0), (5.0, 52.0)]);
        assert_eq!(h.all(Series::GpuPower), vec![(2.0, 7.0), (5.0, 8.0)]);
    }

    #[test]
    fn every_series_has_a_value_and_a_column() {
        // the values go 1, 2, 3 ... in the order of the series
        let all = Sample {
            cpu_temp: Some(1.0),
            core_temp: Some(2.0),
            cpu_zone_temp: Some(3.0),
            cpu_power: Some(4.0),
            pl1: Some(5.0),
            pl2: Some(6.0),
            cpu_clock: Some(7.0),
            cpu_clock_top: Some(8.0),
            cpu_clock_p: Some(9.0),
            cpu_clock_e: Some(10.0),
            cpu_load: Some(11.0),
            gpu_temp: Some(12.0),
            gpu_mem_temp: Some(13.0),
            gpu_power: Some(14.0),
            gpu_clock: Some(15.0),
            gpu_load: Some(16.0),
        };
        for (i, series) in Series::ALL.iter().enumerate() {
            assert_eq!(series.index(), i);
            assert_eq!(series.value(&all), Some(i as f32 + 1.0));
            assert!(series.column().ends_with(|c: char| c.is_ascii_lowercase()));
        }
    }
}
