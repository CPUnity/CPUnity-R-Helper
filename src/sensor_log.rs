//! Optional CSV log of every sensor reading, written while the sensors are on and logging is
//! switched on. One file per start of logging, one line per reading round.
use crate::sensor_history::Series;
use crate::sensors::Sample;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

/// Calendar time split into fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Civil {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

/// Seconds since 1970-01-01 as a calendar time (proleptic Gregorian, no time zone).
pub fn civil_from_unix(secs: i64) -> Civil {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400) as u32;
    // Howard Hinnant's days -> civil algorithm
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    Civil { year, month, day, hour: rem / 3600, minute: rem % 3600 / 60, second: rem % 60 }
}

#[cfg(windows)]
pub fn now() -> Civil {
    #[repr(C)]
    #[derive(Default)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetLocalTime(time: *mut SystemTime);
    }
    let mut t = SystemTime::default();
    unsafe { GetLocalTime(&mut t) };
    Civil {
        year: i64::from(t.year),
        month: u32::from(t.month),
        day: u32::from(t.day),
        hour: u32::from(t.hour),
        minute: u32::from(t.minute),
        second: u32::from(t.second),
    }
}

#[cfg(not(windows))]
pub fn now() -> Civil {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    civil_from_unix(secs)
}

/// Seconds since 1970-01-01 for a calendar time (the opposite of `civil_from_unix`).
pub fn unix_from_civil(c: Civil) -> i64 {
    let y = c.year - i64::from(c.month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = i64::from(if c.month > 2 { c.month - 3 } else { c.month + 9 });
    let doy = (153 * mp + 2) / 5 + i64::from(c.day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    days * 86_400 + i64::from(c.hour) * 3600 + i64::from(c.minute) * 60 + i64::from(c.second)
}

/// The seconds of a `2026-10-17 04:30:01` stamp (as written by `stamp`).
pub fn parse_stamp(text: &str) -> Option<i64> {
    let (date, time) = text.trim().split_once(' ')?;
    let mut d = date.split('-');
    let mut t = time.split(':');
    let civil = Civil {
        year: d.next()?.parse().ok()?,
        month: d.next()?.parse().ok()?,
        day: d.next()?.parse().ok()?,
        hour: t.next()?.parse().ok()?,
        minute: t.next()?.parse().ok()?,
        second: t.next()?.parse().ok()?,
    };
    let valid = (1..=12).contains(&civil.month)
        && (1..=31).contains(&civil.day)
        && civil.hour < 24
        && civil.minute < 60
        && civil.second < 61;
    valid.then(|| unix_from_civil(civil))
}

/// `2026-10-17 04:30:01`
pub fn stamp(c: Civil) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        c.year, c.month, c.day, c.hour, c.minute, c.second
    )
}

/// The local clock time now as seconds, in the same count `parse_stamp` gives the times of a log.
pub fn unix_now() -> i64 {
    parse_stamp(&stamp(now())).unwrap_or(0)
}

/// Today's date and the time now, as the computer shows them.
pub fn today() -> Civil {
    now()
}

/// `name` made safe for a file name: no characters Windows forbids, no ".csv" typed by the
/// user, no leading / trailing dots or spaces, at most 80 characters.
pub fn clean_name(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .collect();
    let mut cleaned = cleaned;
    if cleaned.to_ascii_lowercase().ends_with(".csv") {
        cleaned.truncate(cleaned.len() - 4);
    }
    let cleaned = cleaned.trim_matches(|c: char| c == '.' || c == ' ').to_string();
    cleaned.chars().take(80).collect::<String>().trim_end_matches(['.', ' ']).to_string()
}

/// The built-in file name presets: what is put around the typed name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stamp {
    /// Only the name.
    None,
    /// The date before the name (`2026-10-17_Cinebench run.csv`).
    #[default]
    Date,
    /// The time after the name (`Cinebench run_043001.csv`).
    Time,
    /// The date before the name and the time after it (`2026-10-17_Cinebench run_043001.csv`).
    DateTime,
}

impl Stamp {
    pub const ALL: [Stamp; 4] = [Stamp::None, Stamp::Date, Stamp::Time, Stamp::DateTime];

    pub fn label(self) -> &'static str {
        match self {
            Stamp::None => "None",
            Stamp::Date => "Date",
            Stamp::Time => "Time",
            Stamp::DateTime => "Date and time",
        }
    }

    /// The word in the config file.
    pub fn key(self) -> &'static str {
        match self {
            Stamp::None => "none",
            Stamp::Date => "date",
            Stamp::Time => "time",
            Stamp::DateTime => "datetime",
        }
    }

    pub fn from_key(key: &str) -> Stamp {
        Stamp::ALL.into_iter().find(|s| s.key() == key.trim()).unwrap_or_default()
    }

    pub fn has_date(self) -> bool {
        matches!(self, Stamp::Date | Stamp::DateTime)
    }

    pub fn has_time(self) -> bool {
        matches!(self, Stamp::Time | Stamp::DateTime)
    }
}

/// The log's file name: the date first and the time last when the built-in preset has them, the chosen
/// name between, all joined with `_` (`2026-10-17_Cinebench run_043001.csv`). With nothing at all (no
/// stamp and no name) it is `sensors-043001.csv`.
pub fn file_name_stamped(c: Civil, name: &str, stamp: Stamp) -> String {
    let name = clean_name(name);
    let time = format!("{:02}{:02}{:02}", c.hour, c.minute, c.second);
    let mut parts: Vec<String> = Vec::new();
    if stamp.has_date() {
        parts.push(format!("{:04}-{:02}-{:02}", c.year, c.month, c.day));
    }
    if !name.is_empty() {
        parts.push(name.clone());
    }
    if stamp.has_time() {
        parts.push(time.clone());
    }
    if parts.is_empty() {
        parts.push(format!("sensors-{}", time));
    }
    format!("{}.csv", parts.join("_"))
}

/// The log's file name with the date in front (the `Date` preset).
pub fn file_name(c: Civil, name: &str) -> String {
    file_name_stamped(c, name, Stamp::Date)
}

/// `file_name` inside `dir`; when it already exists, `-2`, `-3`... is added before `.csv` so an
/// earlier log is never overwritten.
pub fn unique_path(dir: &Path, file_name: &str) -> PathBuf {
    let first = dir.join(file_name);
    if !first.exists() {
        return first;
    }
    let stem = file_name.strip_suffix(".csv").unwrap_or(file_name);
    (2..10_000)
        .map(|n| dir.join(format!("{}-{}.csv", stem, n)))
        .find(|p| !p.exists())
        .unwrap_or(first)
}

pub fn csv_header() -> String {
    let mut line = String::from("time");
    for series in Series::ALL {
        line.push(',');
        line.push_str(series.column());
    }
    line
}

/// One line: the time and every reading (empty where a value is not available).
pub fn csv_row(time: &str, sample: &Sample) -> String {
    let mut line = String::from(time);
    for series in Series::ALL {
        line.push(',');
        if let Some(v) = series.value(sample).filter(|v| v.is_finite()) {
            line.push_str(&format!("{:.1}", v));
        }
    }
    line
}

/// Folder the logs go to: `logs` next to the settings.
pub fn log_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("CPUnity-R-Helper").join("logs")
}

pub struct Logger {
    out: BufWriter<File>,
    path: PathBuf,
}

impl Logger {
    /// Starts a new log file in `dir` (created when missing).
    pub fn create(dir: &Path, name: &str, stamp: Stamp) -> io::Result<Logger> {
        std::fs::create_dir_all(dir)?;
        let path = unique_path(dir, &file_name_stamped(now(), name, stamp));
        let mut out = BufWriter::new(File::create(&path)?);
        writeln!(out, "{}", csv_header())?;
        out.flush()?;
        Ok(Logger { out, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends one line stamped with the current time and flushes it, so the file is complete
    /// even if R-Helper is closed or crashes.
    pub fn write(&mut self, sample: &Sample) -> io::Result<()> {
        writeln!(self.out, "{}", csv_row(&stamp(now()), sample))?;
        self.out.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_time_becomes_a_calendar_time() {
        assert_eq!(
            civil_from_unix(0),
            Civil { year: 1970, month: 1, day: 1, hour: 0, minute: 0, second: 0 }
        );
        // 2026-10-17 09:30:01 UTC
        assert_eq!(
            civil_from_unix(1_792_229_401),
            Civil { year: 2026, month: 10, day: 17, hour: 9, minute: 30, second: 1 }
        );
        // leap day and the end of a year
        assert_eq!(civil_from_unix(951_782_400).day, 29); // 2000-02-29
        let end = civil_from_unix(1_798_761_599); // 2026-12-31 23:59:59
        assert_eq!((end.year, end.month, end.day, end.hour), (2026, 12, 31, 23));
        // before 1970 does not panic
        assert_eq!(civil_from_unix(-1).year, 1969);
    }

    #[test]
    fn stamps_and_names() {
        let c = Civil { year: 2026, month: 3, day: 7, hour: 4, minute: 5, second: 6 };
        assert_eq!(stamp(c), "2026-03-07 04:05:06");
        assert_eq!(file_name(c, ""), "2026-03-07.csv");
        assert_eq!(file_name(c, "  Cinebench run "), "2026-03-07_Cinebench run.csv");
        assert_eq!(file_name(c, "gaming.CSV"), "2026-03-07_gaming.csv");
    }

    #[test]
    fn the_built_in_presets_go_around_the_name() {
        let c = Civil { year: 2026, month: 3, day: 7, hour: 4, minute: 5, second: 6 };
        assert_eq!(file_name_stamped(c, "3d-mark-cpu", Stamp::DateTime), "2026-03-07_3d-mark-cpu_040506.csv");
        assert_eq!(file_name_stamped(c, "3d-mark-cpu", Stamp::Date), "2026-03-07_3d-mark-cpu.csv");
        assert_eq!(file_name_stamped(c, "3d-mark-cpu", Stamp::Time), "3d-mark-cpu_040506.csv");
        assert_eq!(file_name_stamped(c, "3d-mark-cpu", Stamp::None), "3d-mark-cpu.csv");
        assert_eq!(file_name_stamped(c, "", Stamp::DateTime), "2026-03-07_040506.csv");
        assert_eq!(file_name_stamped(c, "", Stamp::Date), "2026-03-07.csv");
        assert_eq!(file_name_stamped(c, "", Stamp::Time), "040506.csv");
        assert_eq!(file_name_stamped(c, "  ", Stamp::None), "sensors-040506.csv", "never an empty name");
        for s in Stamp::ALL {
            assert_eq!(Stamp::from_key(s.key()), s);
        }
        assert_eq!(Stamp::from_key("nonsense"), Stamp::Date, "the old behaviour");
    }

    #[test]
    fn typed_names_are_made_safe() {
        assert_eq!(clean_name("a/b\\c:d*e?f\"g<h>i|j"), "a_b_c_d_e_f_g_h_i_j");
        assert_eq!(clean_name("  ..hidden.. "), "hidden");
        assert_eq!(clean_name(""), "");
        assert_eq!(clean_name(".csv"), "");
        assert_eq!(clean_name(&"x".repeat(200)).len(), 80);
        assert_eq!(clean_name("tab\there"), "tab_here");
    }

    #[test]
    fn an_existing_log_is_never_overwritten() {
        let dir = std::env::temp_dir().join(format!("rh_unique_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let name = "2026-03-07_run.csv";
        assert_eq!(unique_path(&dir, name), dir.join(name));
        std::fs::write(dir.join(name), "x").unwrap();
        assert_eq!(unique_path(&dir, name), dir.join("2026-03-07_run-2.csv"));
        std::fs::write(dir.join("2026-03-07_run-2.csv"), "x").unwrap();
        assert_eq!(unique_path(&dir, name), dir.join("2026-03-07_run-3.csv"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rows_match_the_header() {
        let header = csv_header();
        assert!(header.starts_with("time,cpu_package_temp_c,"));
        let sample = Sample { cpu_temp: Some(81.04), gpu_power: Some(120.0), ..Sample::default() };
        let row = csv_row("t", &sample);
        assert_eq!(header.split(',').count(), row.split(',').count());
        let cells: Vec<&str> = row.split(',').collect();
        assert_eq!(cells[1], "81.0");
        assert_eq!(cells[2], "", "missing values are empty cells");
        let gpu_power = header.split(',').position(|c| c == "gpu_power_w").unwrap();
        assert_eq!(cells[gpu_power], "120.0");
    }

    #[test]
    fn logger_writes_a_header_and_flushed_lines() {
        let dir = std::env::temp_dir().join(format!("rh_logger_{}", std::process::id()));
        let mut logger = Logger::create(&dir, "test", Stamp::Date).unwrap();
        let path = logger.path().to_path_buf();
        logger.write(&Sample { cpu_temp: Some(70.0), ..Sample::default() }).unwrap();
        logger.write(&Sample { cpu_temp: Some(71.0), ..Sample::default() }).unwrap();
        // readable while the logger is still open
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], csv_header());
        assert!(lines[1].contains(",70.0,"));
        assert!(lines[2].contains(",71.0,"));
        drop(logger);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
