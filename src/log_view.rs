//! Loading one of R-Helper's CSV logs back for the Log viewer (graphs and a min / average / max
//! table of the whole file).
use crate::sensor_history::{History, Series, SERIES_COUNT};
use crate::sensor_log::parse_stamp;
use std::path::{Path, PathBuf};

/// Largest log that is loaded (bytes).
const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// A log that was read.
#[derive(Debug)]
pub struct Loaded {
    pub name: String,
    pub history: History,
    /// The first time stamp as seconds (the count `sensor_log::parse_stamp` gives).
    pub start_unix: i64,
    /// First and last time stamp in the file.
    pub start: String,
    pub end: String,
    pub rows: usize,
}

/// The log files in `dir`, newest first.
pub fn list_logs(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().map_or(false, |x| x.eq_ignore_ascii_case("csv")) && p.is_file()
                })
                .collect()
        })
        .unwrap_or_default();
    // newest first: by modification time, then by name
    files.sort_by_key(|p| {
        (
            std::fs::metadata(p).and_then(|m| m.modified()).ok(),
            p.file_name().map(|n| n.to_os_string()),
        )
    });
    files.reverse();
    files
}

/// Reads a log written by R-Helper.
pub fn parse(name: &str, text: &str) -> Result<Loaded, String> {
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    let header = lines.next().ok_or("the file is empty")?;
    let columns: Vec<&str> = header.split(',').map(str::trim).collect();
    if columns.first().map(|c| c.to_ascii_lowercase()) != Some("time".to_string()) {
        return Err("this is not an R-Helper sensor log (no \"time\" column)".to_string());
    }
    // which column holds which series
    let mut slots: Vec<(usize, Series)> = Vec::new();
    for (i, column) in columns.iter().enumerate().skip(1) {
        if let Some(series) = Series::ALL.iter().copied().find(|s| s.column() == *column) {
            slots.push((i, series));
        }
    }
    if slots.is_empty() {
        return Err("none of the columns are R-Helper sensor readings".to_string());
    }

    let mut history = History::for_log();
    let mut first: Option<(i64, &str)> = None;
    let mut last = "";
    let mut previous = 0.0f64;
    let mut rows = 0;
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        let Some(stamp) = fields.first().copied() else {
            continue;
        };
        let Some(secs) = parse_stamp(stamp) else {
            continue;
        };
        let start = *first.get_or_insert((secs, stamp));
        let mut values = [None; SERIES_COUNT];
        for (i, series) in &slots {
            values[series.index()] =
                fields.get(*i).and_then(|f| f.trim().parse::<f32>().ok()).filter(|v| v.is_finite());
        }
        // a clock change must not make the graph run backwards
        let t = ((secs - start.0) as f64).max(previous);
        previous = t;
        history.push_values(t, values);
        last = stamp;
        rows += 1;
    }
    let Some((start_unix, start)) = first else {
        return Err("the log has no readings yet".to_string());
    };
    Ok(Loaded {
        name: name.to_string(),
        history,
        start_unix,
        start: start.trim().to_string(),
        end: last.trim().to_string(),
        rows,
    })
}

/// Loads a log file from disk.
pub fn load(path: &Path) -> Result<Loaded, String> {
    let size = std::fs::metadata(path).map_err(|e| format!("cannot open it: {e}"))?.len();
    if size > MAX_BYTES {
        return Err(format!("the file is too large to view here ({} MB)", size / 1_048_576));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read it: {e}"))?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    parse(&name, &String::from_utf8_lossy(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor_log::{civil_from_unix, csv_header, csv_row, stamp, unix_from_civil};
    use crate::sensors::Sample;

    fn log_text() -> String {
        let mut text = csv_header();
        for i in 0..5i64 {
            let sample = Sample {
                cpu_temp: Some(60.0 + i as f32),
                gpu_power: (i != 2).then_some(100.0),
                ..Sample::default()
            };
            text.push('\n');
            text.push_str(&csv_row(&stamp(civil_from_unix(1_790_000_000 + i * 2)), &sample));
        }
        text
    }

    #[test]
    fn stamps_round_trip_through_unix_time() {
        for secs in [0, 86_399, 1_790_000_000, 951_782_400, 4_102_444_799] {
            let c = civil_from_unix(secs);
            assert_eq!(unix_from_civil(c), secs);
            assert_eq!(parse_stamp(&stamp(c)), Some(secs));
        }
        assert_eq!(parse_stamp("nonsense"), None);
        assert_eq!(parse_stamp("2026-13-01 00:00:00"), None);
    }

    #[test]
    fn a_log_is_loaded_with_its_values_and_gaps() {
        let loaded = parse("t.csv", &log_text()).unwrap();
        assert_eq!(loaded.rows, 5);
        assert_eq!(loaded.history.duration(), 8.0);
        assert_eq!(loaded.history.stats(Series::CpuTemp), Some((60.0, 62.0, 64.0)));
        assert_eq!(loaded.history.stats(Series::GpuPower), Some((100.0, 100.0, 100.0)));
        assert_eq!(loaded.history.stats(Series::GpuLoad), None, "never logged");
        assert_eq!(loaded.history.series(Series::GpuPower, 100.0).len(), 4, "the gap is left out");
        assert!(loaded.start < loaded.end);
        assert_eq!(loaded.start_unix, 1_790_000_000);
        assert_eq!(loaded.history.all(Series::GpuPower).len(), 4);
    }

    #[test]
    fn other_files_are_refused_with_a_reason() {
        assert!(parse("x", "").is_err());
        assert!(parse("x", "a,b\n1,2").unwrap_err().contains("not an R-Helper"));
        assert!(parse("x", "time,something\n1,2").unwrap_err().contains("none of the columns"));
        assert!(parse("x", &csv_header()).unwrap_err().contains("no readings"));
    }

    #[test]
    fn logs_are_listed_newest_first() {
        let dir = std::env::temp_dir().join(format!("rh_logs_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["sensors-20261001-100000.csv", "sensors-20261017-043001.csv", "notes.txt"] {
            std::fs::write(dir.join(name), "x").unwrap();
        }
        let names: Vec<_> = list_logs(&dir)
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["sensors-20261017-043001.csv", "sensors-20261001-100000.csv"]);
        assert!(list_logs(&dir.join("missing")).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
