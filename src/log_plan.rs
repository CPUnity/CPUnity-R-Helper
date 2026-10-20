//! When the CSV log starts and stops: right now or when a program starts, after an exact length
//! of time or when that program closes, or never (endless). The decisions are made here, apart
//! from the sensor thread, so they can be tested.
use crate::sensor_log::Civil;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Start {
    /// As soon as the switch is on.
    #[default]
    Now,
    /// Each time the chosen program starts (a new file every time).
    Program,
}

impl Start {
    pub fn key(self) -> &'static str {
        match self {
            Start::Now => "now",
            Start::Program => "program",
        }
    }

    pub fn from_key(key: &str) -> Start {
        if key.trim().eq_ignore_ascii_case("program") {
            Start::Program
        } else {
            Start::Now
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plan {
    pub start: Start,
    /// The program that starts the log, as `clean_program` makes it (`Cinebench.exe`).
    pub program: String,
    /// Seconds after which the log stops by itself. 0 = no limit: it goes on until switched off.
    pub limit_secs: u64,
    /// The log stops when the program closes (only with `Start::Program`).
    pub stop_with_program: bool,
    /// The built-in file name preset: the date before the name, the time after it, both or neither.
    pub stamp: crate::sensor_log::Stamp,
}

/// Why a log stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    SwitchedOff,
    TimeUp,
    ProgramClosed,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Step {
    /// Finish the open file before anything else.
    pub close: Option<Why>,
    /// Start a new file for this reading.
    pub open: bool,
}

/// Follows one logging session reading by reading. Times are seconds on any steady clock.
#[derive(Debug, Default)]
pub struct Runner {
    since: Option<f64>,
    /// A log ended by its time limit: no new one until the program has closed (or the switch has
    /// been turned off and on), so one run of the program is one file.
    blocked: bool,
    running: bool,
}

impl Runner {
    pub fn recording(&self) -> bool {
        self.since.is_some()
    }

    /// Seconds left until the time limit, when there is one and a log is open.
    pub fn remaining(&self, plan: &Plan, now: f64) -> Option<f64> {
        let since = self.since?;
        (plan.limit_secs > 0).then(|| (since + plan.limit_secs as f64 - now).max(0.0))
    }

    /// Called for every reading before it is written. `armed` is the logging switch; `program` is
    /// whether the program is running (`None` = not looked at this time: the last answer holds).
    pub fn before(&mut self, plan: &Plan, armed: bool, now: f64, program: Option<bool>) -> Step {
        if let Some(running) = program {
            self.running = running;
        }
        if !armed {
            self.blocked = false;
            return Step { close: self.since.take().map(|_| Why::SwitchedOff), open: false };
        }
        let mut step = Step::default();
        match plan.start {
            Start::Now => step.open = self.since.is_none() && !self.blocked,
            Start::Program => {
                if !self.running {
                    self.blocked = false;
                    if plan.stop_with_program && self.since.take().is_some() {
                        step.close = Some(Why::ProgramClosed);
                    }
                } else {
                    step.open = self.since.is_none() && !self.blocked;
                }
            }
        }
        if step.open {
            self.since = Some(now);
        }
        step
    }

    /// The file could not be opened or written: nothing is open and no new one is tried until the
    /// switch has been turned off and on again.
    pub fn abort(&mut self) {
        self.since = None;
        self.blocked = true;
    }

    /// Called after a reading is written: the end of the time limit.
    pub fn after(&mut self, plan: &Plan, now: f64) -> Option<Why> {
        let since = self.since?;
        if plan.limit_secs > 0 && now - since >= plan.limit_secs as f64 - 0.05 {
            self.since = None;
            self.blocked = true;
            return Some(Why::TimeUp);
        }
        None
    }
}

/// The program as tasklist knows it: `C:\Games\Foo.exe ` -> `Foo.exe`, `foo` -> `foo.exe`.
/// Empty when nothing usable was typed.
pub fn clean_program(text: &str) -> String {
    let name = text.trim().trim_matches('"').trim();
    let name = name.rsplit(['\\', '/']).next().unwrap_or("").trim();
    if name.is_empty() || name.chars().any(|c| c.is_control() || "<>:\"|?*".contains(c)) {
        return String::new();
    }
    let name: String = name.chars().take(120).collect();
    if name.to_ascii_lowercase().ends_with(".exe") {
        name
    } else {
        format!("{}.exe", name)
    }
}

/// Whether tasklist's output (`/FO CSV /NH`) lists the program.
pub fn listed(output: &str, exe: &str) -> bool {
    !exe.is_empty()
        && output.lines().any(|line| image_name(line).is_some_and(|n| n.eq_ignore_ascii_case(exe)))
}

fn image_name(line: &str) -> Option<&str> {
    let line = line.trim();
    let rest = line.strip_prefix('"')?;
    rest.split('"').next().filter(|n| !n.is_empty())
}

/// Every program in tasklist's output, once each, sorted, without the Windows ones nobody would
/// log for.
pub fn programs_in(output: &str) -> Vec<String> {
    const SYSTEM: &[&str] = &[
        "system",
        "registry",
        "smss.exe",
        "csrss.exe",
        "wininit.exe",
        "services.exe",
        "lsass.exe",
        "svchost.exe",
        "winlogon.exe",
        "fontdrvhost.exe",
        "dwm.exe",
        "conhost.exe",
        "dllhost.exe",
        "sihost.exe",
        "ctfmon.exe",
        "taskhostw.exe",
        "runtimebroker.exe",
        "searchhost.exe",
        "textinputhost.exe",
        "tasklist.exe",
        "wmiprvse.exe",
        "audiodg.exe",
        "spoolsv.exe",
        "memcompression",
        "secure system",
        "system idle process",
        "unsecapp.exe",
        "backgroundtaskhost.exe",
        "applicationframehost.exe",
        "startmenuexperiencehost.exe",
        "shellexperiencehost.exe",
    ];
    let names: BTreeSet<String> = output
        .lines()
        .filter_map(image_name)
        .filter(|n| n.to_ascii_lowercase().ends_with(".exe"))
        .filter(|n| !SYSTEM.contains(&n.to_ascii_lowercase().as_str()))
        .map(str::to_string)
        .collect();
    let mut seen = BTreeSet::new();
    names.into_iter().filter(|n| seen.insert(n.to_ascii_lowercase())).collect()
}

fn tasklist(args: &[&str]) -> Option<String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("tasklist")
            .args(args)
            .creation_flags(0x0800_0000)
            .output()
            .ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        None
    }
}

/// Whether the program is running right now (`None` when that could not be found out).
pub fn is_running(exe: &str) -> Option<bool> {
    if exe.is_empty() {
        return Some(false);
    }
    let filter = format!("IMAGENAME eq {}", exe);
    tasklist(&["/FO", "CSV", "/NH", "/FI", &filter]).map(|out| listed(&out, exe))
}

/// The programs running right now, for the picker.
pub fn running_programs() -> Vec<String> {
    tasklist(&["/FO", "CSV", "/NH"]).map(|out| programs_in(&out)).unwrap_or_default()
}

/// The name part of the next log's file: the typed name; with a program as the start and no typed
/// name, the program and the time.
pub fn file_label(plan: &Plan, typed: &str, now: Civil) -> String {
    // (the time a program's name gets by default is not added twice when the preset has the time)
    if !crate::sensor_log::clean_name(typed).is_empty() || plan.start != Start::Program {
        return typed.to_string();
    }
    let stem = plan.program.rsplit_once('.').map(|(s, _)| s).unwrap_or(&plan.program);
    if stem.is_empty() {
        return String::new();
    }
    if plan.stamp.has_time() {
        return stem.to_string();
    }
    format!("{}-{:02}{:02}{:02}", stem, now.hour, now.minute, now.second)
}

/// How many saved file names (name presets) the Logging card keeps.
pub const MAX_NAME_PRESETS: usize = 50;

/// Where `name` (as typed) is in the saved file names, if it is one of them.
pub fn name_preset_index(presets: &[String], name: &str) -> Option<usize> {
    let name = crate::sensor_log::clean_name(name);
    if name.is_empty() {
        return None;
    }
    presets.iter().position(|p| p.eq_ignore_ascii_case(&name))
}

/// Saves `name` (made safe for a file name) as a preset. False when there is nothing to save, it is saved
/// already, or the list is full.
pub fn add_name_preset(presets: &mut Vec<String>, name: &str) -> bool {
    let clean = crate::sensor_log::clean_name(name);
    if clean.is_empty() || presets.len() >= MAX_NAME_PRESETS || name_preset_index(presets, &clean).is_some() {
        return false;
    }
    presets.push(clean);
    true
}

/// Takes the preset that `name` is out of the list. False when it is not in it.
pub fn remove_name_preset(presets: &mut Vec<String>, name: &str) -> bool {
    match name_preset_index(presets, name) {
        Some(i) => {
            presets.remove(i);
            true
        }
        None => false,
    }
}

/// The saved file names as written to the config file (one line, `|` between them: a name cannot hold one).
pub fn encode_name_presets(presets: &[String]) -> String {
    presets.join("|")
}

/// The saved file names from the config file: made safe, no empty ones, no repeats, at most `MAX_NAME_PRESETS`.
pub fn decode_name_presets(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in text.split('|') {
        add_name_preset(&mut out, part);
    }
    out
}

/// A length of time as the Logging card says it: `10 min`, `1 h 30 min`, `45 s`, `No limit`.
pub fn limit_text(secs: u64) -> String {
    match secs {
        0 => "No limit".to_string(),
        s if s < 60 => format!("{} s", s),
        s if s < 3600 => match s % 60 {
            0 => format!("{} min", s / 60),
            r => format!("{} min {} s", s / 60, r),
        },
        s => match (s % 3600) / 60 {
            0 if s % 60 == 0 => format!("{} h", s / 3600),
            m if s % 60 == 0 => format!("{} h {} min", s / 3600, m),
            m => format!("{} h {} min {} s", s / 3600, m, s % 60),
        },
    }
}

/// The longest limit: 99 days.
pub const MAX_LIMIT_SECS: u64 = 99 * 86_400;

/// The three boxes of the limit (hours, minutes, seconds) put together.
pub fn limit_from_parts(hours: u64, minutes: u64, seconds: u64) -> u64 {
    (hours.min(2376) * 3600 + minutes.min(59) * 60 + seconds.min(59)).min(MAX_LIMIT_SECS)
}

/// A limit split into the three boxes (hours, minutes, seconds).
pub fn limit_parts(secs: u64) -> (u64, u64, u64) {
    (secs / 3600, secs % 3600 / 60, secs % 60)
}

/// `1:02:03`, or `2:03` under an hour.
pub fn hms(secs: u64) -> String {
    if secs >= 3600 {
        format!("{}:{:02}:{:02}", secs / 3600, secs % 3600 / 60, secs % 60)
    } else {
        format!("{}:{:02}", secs / 60, secs % 60)
    }
}

/// What the Logging card knows when it writes its status line.
pub struct Facts<'a> {
    /// The logging switch.
    pub armed: bool,
    /// "Read sensors" is on.
    pub reading: bool,
    pub plan: &'a Plan,
    /// The file being written (its name) and how long ago it was started.
    pub writing: Option<(&'a str, u64)>,
    pub rows: u64,
    /// Armed and waiting for the program.
    pub waiting: bool,
    pub failed: bool,
    /// The last file that was finished, and why.
    pub last: Option<(&'a str, Why)>,
    /// Where the files go (shown when there is nothing else to say).
    pub folder: &'a str,
}

/// The one line under the logging switch.
pub fn status(f: &Facts) -> String {
    if f.failed {
        return "Could not write the log file.".to_string();
    }
    if let Some((file, elapsed)) = f.writing {
        let mut line =
            format!("Recording {} \u{b7} {} \u{b7} {} readings", file, hms(elapsed), f.rows);
        if f.plan.limit_secs > 0 {
            line.push_str(&format!(
                " \u{b7} stops in {}",
                hms(f.plan.limit_secs.saturating_sub(elapsed))
            ));
        }
        return line;
    }
    if f.armed && !f.reading {
        return "Starts as soon as Read sensors is switched on.".to_string();
    }
    if f.armed && f.plan.start == Start::Program {
        return if f.plan.program.is_empty() {
            "Choose the program that starts the log.".to_string()
        } else if f.waiting {
            format!("Waiting for {} to start.", f.plan.program)
        } else {
            "Starting...".to_string()
        };
    }
    if f.armed {
        return "Starting...".to_string();
    }
    match f.last {
        Some((file, Why::TimeUp)) => {
            format!("Finished {}: {} reached.", file, limit_text(f.plan.limit_secs))
        }
        Some((file, Why::ProgramClosed)) => {
            format!("Finished {}: {} closed.", file, f.plan.program)
        }
        Some((file, Why::SwitchedOff)) => format!("Stopped {}.", file),
        None => format!("Files go to {}", f.folder),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn file_name_presets_are_saved_once_cleaned_and_found_again() {
        let mut list = Vec::new();
        assert!(add_name_preset(&mut list, "  Cinebench run "));
        assert!(add_name_preset(&mut list, "Idle: 5 min/test"));
        assert_eq!(list, ["Cinebench run", "Idle_ 5 min_test"]);
        assert!(!add_name_preset(&mut list, "cinebench RUN"), "the same name is not saved twice");
        assert!(!add_name_preset(&mut list, "   ") && !add_name_preset(&mut list, ".csv"), "nothing to save");
        assert_eq!(name_preset_index(&list, "Cinebench run.csv"), Some(0));
        assert_eq!(name_preset_index(&list, ""), None, "no name is no preset");
        assert_eq!(name_preset_index(&list, "Gaming"), None);
        assert!(remove_name_preset(&mut list, "cinebench run") && !remove_name_preset(&mut list, "cinebench run"));
        assert_eq!(list.len(), 1);
        for n in 0..80 {
            add_name_preset(&mut list, &format!("run {n}"));
        }
        assert_eq!(list.len(), MAX_NAME_PRESETS);
    }

    #[test]
    fn file_name_presets_survive_the_config_file() {
        let list = vec!["Cinebench run".to_string(), "Idle".to_string()];
        assert_eq!(decode_name_presets(&encode_name_presets(&list)), list);
        assert_eq!(decode_name_presets("a||A|b|"), ["a", "b"], "empty and repeated ones are dropped");
        assert!(decode_name_presets("").is_empty());
        assert_eq!(decode_name_presets(&vec!["x"; 50].join("|")), ["x"]);
    }

    use super::*;

    fn now_plan(limit: u64) -> Plan {
        Plan { limit_secs: limit, ..Plan::default() }
    }

    fn program_plan(limit: u64, stop: bool) -> Plan {
        Plan {
            start: Start::Program,
            program: "Game.exe".into(),
            limit_secs: limit,
            stop_with_program: stop,
            ..Plan::default()
        }
    }

    #[test]
    fn a_log_that_starts_now_runs_until_the_time_is_up() {
        let plan = now_plan(10);
        let mut r = Runner::default();
        assert_eq!(r.before(&plan, true, 100.0, None), Step { close: None, open: true });
        assert!(r.recording());
        assert_eq!(r.after(&plan, 100.0), None);
        assert_eq!(r.remaining(&plan, 104.0), Some(6.0));
        assert_eq!(r.before(&plan, true, 105.0, None), Step::default());
        assert_eq!(r.after(&plan, 109.0), None);
        assert_eq!(r.after(&plan, 110.0), Some(Why::TimeUp));
        assert!(!r.recording());
        // it does not start again by itself while the switch stays on
        assert_eq!(r.before(&plan, true, 111.0, None), Step::default());
        // switching off and on starts a new one
        assert_eq!(r.before(&plan, false, 112.0, None), Step::default());
        assert!(r.before(&plan, true, 113.0, None).open);
    }

    #[test]
    fn without_a_limit_a_log_never_ends_by_itself() {
        let plan = now_plan(0);
        let mut r = Runner::default();
        assert!(r.before(&plan, true, 0.0, None).open);
        for t in 1..2000 {
            let t = f64::from(t) * 3600.0;
            assert_eq!(r.before(&plan, true, t, None), Step::default());
            assert_eq!(r.after(&plan, t), None);
        }
        assert_eq!(r.remaining(&plan, 5.0), None);
        // only the switch ends it
        assert_eq!(
            r.before(&plan, false, 1.0e9, None),
            Step { close: Some(Why::SwitchedOff), open: false }
        );
    }

    #[test]
    fn a_log_waits_for_its_program_and_follows_it() {
        let plan = program_plan(0, true);
        let mut r = Runner::default();
        // armed, program not running: nothing is written
        assert_eq!(r.before(&plan, true, 0.0, Some(false)), Step::default());
        assert!(!r.recording());
        // the program starts: a new file
        assert!(r.before(&plan, true, 5.0, Some(true)).open);
        // not checked this time: the last answer holds
        assert_eq!(r.before(&plan, true, 6.0, None), Step::default());
        assert!(r.recording());
        // it closes: the file is finished
        assert_eq!(
            r.before(&plan, true, 60.0, Some(false)),
            Step { close: Some(Why::ProgramClosed), open: false }
        );
        assert!(!r.recording());
        // and the next start is a new file again
        assert!(r.before(&plan, true, 90.0, Some(true)).open);
    }

    #[test]
    fn a_program_log_with_a_limit_is_one_file_per_run() {
        let plan = program_plan(30, true);
        let mut r = Runner::default();
        assert!(r.before(&plan, true, 0.0, Some(true)).open);
        assert_eq!(r.after(&plan, 30.0), Some(Why::TimeUp));
        // the program is still running: no second file
        assert_eq!(r.before(&plan, true, 31.0, Some(true)), Step::default());
        assert!(!r.recording());
        // it closes and starts again: a new one
        assert_eq!(r.before(&plan, true, 40.0, Some(false)), Step::default());
        assert!(r.before(&plan, true, 50.0, Some(true)).open);
    }

    #[test]
    fn without_stop_with_program_the_file_goes_on_after_the_program_closes() {
        let plan = program_plan(0, false);
        let mut r = Runner::default();
        assert!(r.before(&plan, true, 0.0, Some(true)).open);
        assert_eq!(r.before(&plan, true, 10.0, Some(false)), Step::default());
        assert!(r.recording());
        // and the program starting again does not open a second file over it
        assert_eq!(r.before(&plan, true, 20.0, Some(true)), Step::default());
        assert_eq!(r.before(&plan, false, 30.0, None).close, Some(Why::SwitchedOff));
    }

    #[test]
    fn programs_are_named_the_way_tasklist_knows_them() {
        assert_eq!(clean_program("  Cinebench "), "Cinebench.exe");
        assert_eq!(clean_program("cinebench.EXE"), "cinebench.EXE");
        assert_eq!(clean_program(r#""C:\Program Files\Maxon\Cinebench.exe""#), "Cinebench.exe");
        assert_eq!(clean_program("D:/Games/Foo Bar.exe"), "Foo Bar.exe");
        assert_eq!(clean_program(""), "");
        assert_eq!(clean_program(r"C:\folder\"), "");
        assert_eq!(clean_program("a|b"), "");
    }

    #[test]
    fn tasklist_output_is_read_for_one_program_and_for_the_list() {
        let out = "\"System Idle Process\",\"0\",\"Services\",\"0\",\"8 K\"\r\n\
                   \"svchost.exe\",\"900\",\"Services\",\"0\",\"9,000 K\"\r\n\
                   \"Cinebench.exe\",\"4120\",\"Console\",\"1\",\"300,000 K\"\r\n\
                   \"cinebench.exe\",\"4121\",\"Console\",\"1\",\"3,000 K\"\r\n\
                   \"firefox.exe\",\"77\",\"Console\",\"1\",\"500,000 K\"\r\n";
        assert!(listed(out, "cinebench.exe"), "case does not matter");
        assert!(!listed(out, "cine.exe"));
        assert!(!listed(out, ""));
        assert!(!listed(
            "INFO: No tasks are running which match the specified criteria.\r\n",
            "Cinebench.exe"
        ));
        assert_eq!(programs_in(out), ["Cinebench.exe", "firefox.exe"]);
    }

    #[test]
    fn the_file_is_named_after_the_program_when_nothing_was_typed() {
        let at = Civil { year: 2026, month: 10, day: 20, hour: 4, minute: 5, second: 6 };
        let plan = program_plan(0, true);
        assert_eq!(file_label(&plan, "", at), "Game-040506");
        assert_eq!(file_label(&plan, "my run", at), "my run");
        assert_eq!(
            file_label(&plan, " .. ", at),
            "Game-040506",
            "a name that is nothing once cleaned counts as empty"
        );
        // with the time in the preset it is not put in the name as well
        let timed = Plan { stamp: crate::sensor_log::Stamp::DateTime, ..program_plan(0, true) };
        assert_eq!(file_label(&timed, "", at), "Game");
        assert_eq!(file_label(&timed, "my run", at), "my run");
        assert_eq!(file_label(&now_plan(0), "", at), "");
        assert_eq!(file_label(&now_plan(0), "x", at), "x");
    }

    #[test]
    fn lengths_are_written_and_put_together() {
        assert_eq!(limit_text(0), "No limit");
        assert_eq!(limit_text(45), "45 s");
        assert_eq!(limit_text(600), "10 min");
        assert_eq!(limit_text(630), "10 min 30 s");
        assert_eq!(limit_text(3600), "1 h");
        assert_eq!(limit_text(5400), "1 h 30 min");
        assert_eq!(limit_text(5405), "1 h 30 min 5 s");
        assert_eq!(limit_from_parts(1, 30, 0), 5400);
        assert_eq!(limit_from_parts(0, 0, 45), 45);
        assert_eq!(limit_from_parts(0, 0, 0), 0);
        assert_eq!(limit_from_parts(u64::MAX / 8, u64::MAX / 8, u64::MAX / 8), MAX_LIMIT_SECS);
        assert_eq!(limit_parts(5405), (1, 30, 5));
        assert_eq!(Start::from_key("PROGRAM"), Start::Program);
        assert_eq!(Start::from_key("x"), Start::Now);
        assert_eq!(Start::from_key(Start::Program.key()), Start::Program);
    }

    #[test]
    fn the_status_line_says_what_the_log_is_doing() {
        let now = now_plan(0);
        let timed = now_plan(600);
        let prog = program_plan(0, true);
        let facts = |plan: &'static Plan| Facts {
            armed: false,
            reading: true,
            plan,
            writing: None,
            rows: 0,
            waiting: false,
            failed: false,
            last: None,
            folder: "C:\\logs",
        };
        let now: &'static Plan = Box::leak(Box::new(now));
        let timed: &'static Plan = Box::leak(Box::new(timed));
        let prog: &'static Plan = Box::leak(Box::new(prog));
        assert_eq!(status(&facts(now)), "Files go to C:\\logs");
        assert_eq!(
            status(&Facts { failed: true, writing: Some(("a.csv", 1)), ..facts(now) }),
            "Could not write the log file."
        );
        assert_eq!(
            status(&Facts {
                armed: true,
                writing: Some(("a.csv", 3723)),
                rows: 3724,
                ..facts(now)
            }),
            "Recording a.csv \u{b7} 1:02:03 \u{b7} 3724 readings",
            "no limit: no countdown, it just goes on"
        );
        assert_eq!(
            status(&Facts { armed: true, writing: Some(("a.csv", 65)), rows: 66, ..facts(timed) }),
            "Recording a.csv \u{b7} 1:05 \u{b7} 66 readings \u{b7} stops in 8:55"
        );
        assert_eq!(
            status(&Facts { armed: true, reading: false, ..facts(now) }),
            "Starts as soon as Read sensors is switched on."
        );
        assert_eq!(
            status(&Facts { armed: true, waiting: true, ..facts(prog) }),
            "Waiting for Game.exe to start."
        );
        let blank: &'static Plan =
            Box::leak(Box::new(Plan { start: Start::Program, ..Plan::default() }));
        assert_eq!(
            status(&Facts { armed: true, waiting: true, ..facts(blank) }),
            "Choose the program that starts the log."
        );
        assert_eq!(status(&Facts { armed: true, ..facts(now) }), "Starting...");
        assert_eq!(
            status(&Facts { last: Some(("a.csv", Why::TimeUp)), ..facts(timed) }),
            "Finished a.csv: 10 min reached."
        );
        assert_eq!(
            status(&Facts { last: Some(("a.csv", Why::ProgramClosed)), ..facts(prog) }),
            "Finished a.csv: Game.exe closed."
        );
        assert_eq!(
            status(&Facts { last: Some(("a.csv", Why::SwitchedOff)), ..facts(now) }),
            "Stopped a.csv."
        );
        assert_eq!(hms(59), "0:59");
        assert_eq!(hms(3600), "1:00:00");
    }

    #[test]
    fn a_failed_file_is_not_tried_again_until_the_switch_is_turned_off_and_on() {
        let plan = now_plan(0);
        let mut r = Runner::default();
        assert!(r.before(&plan, true, 0.0, None).open);
        r.abort();
        assert!(!r.recording());
        assert_eq!(r.before(&plan, true, 1.0, None), Step::default());
        assert_eq!(r.before(&plan, false, 2.0, None), Step::default());
        assert!(r.before(&plan, true, 3.0, None).open);
    }
}
