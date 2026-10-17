// Built-in "Synapse standby": parks Razer Synapse (services, tasks, autostarts, processes) so it
// stops using resources while the hardware keeps its last applied settings. The heavy lifting is
// done by the embedded PowerShell script (assets/synapse-standby.ps1), run elevated via UAC.
// Status polling is read-only, needs no admin rights, and runs on a background thread so the UI
// never waits on it.
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SCRIPT: &str = include_str!("../assets/synapse-standby.ps1");
const GUARD_TASK: &str = "SynapseStandby-Guard";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Standby,
    Resume,
    GuardOn,
    GuardOff,
}

#[derive(Debug, Clone, Default)]
pub struct Status {
    /// state.json exists: standby has been applied and not yet resumed.
    pub parked: bool,
    /// Boot/logon guard task is registered.
    pub guard: bool,
    /// Razer processes currently running (excluding razer-ctl / razer-tray / R-Helper).
    pub procs: usize,
    pub mem_mb: u64,
    /// An elevated action is in flight.
    pub busy: bool,
    pub message: Option<String>,
}

#[derive(Clone)]
pub struct Handle {
    inner: Arc<Mutex<Status>>,
}

fn hidden(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

fn state_file() -> std::path::PathBuf {
    std::env::var_os("ProgramData")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\ProgramData"))
        .join("SynapseStandby")
        .join("state.json")
}

/// Parse `tasklist /FO CSV /NH` output into (process count, MB) for Razer software.
fn parse_tasklist(out: &str) -> (usize, u64) {
    let mut count = 0usize;
    let mut kb = 0u64;
    for line in out.lines() {
        // "Image Name","PID","Session Name","Session#","Mem Usage"
        let fields: Vec<&str> = line.trim().trim_matches('"').split("\",\"").collect();
        if fields.len() < 5 {
            continue;
        }
        let name = fields[0].to_ascii_lowercase();
        let name = name.trim_end_matches(".exe");
        if !(name.starts_with("razer") || name.starts_with("rz")) {
            continue;
        }
        if matches!(name, "razer-ctl" | "razer-tray") || name.contains("rhelper") {
            continue;
        }
        count += 1;
        let digits: String = fields[4].chars().filter(|c| c.is_ascii_digit()).collect();
        kb += digits.parse::<u64>().unwrap_or(0);
    }
    (count, kb / 1024)
}

fn refresh(inner: &Arc<Mutex<Status>>) {
    let parked = state_file().is_file();
    let guard = hidden("schtasks")
        .args(["/Query", "/TN", GUARD_TASK])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let (procs, mem_mb) = hidden("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
        .map(|o| parse_tasklist(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or((0, 0));
    if let Ok(mut s) = inner.lock() {
        s.parked = parked;
        s.guard = guard;
        s.procs = procs;
        s.mem_mb = mem_mb;
    }
}

fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18 & 63) as usize] as char);
        out.push(T[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6 & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
    }
    out
}

/// Write the embedded script to the config folder (only if it changed) and return its path.
fn install_script() -> Result<std::path::PathBuf, String> {
    let dir = crate::config::config_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {}", dir.display(), e))?;
    let path = dir.join("synapse-standby.ps1");
    // PowerShell 5.1 reads BOM-less files as ANSI; add a BOM so any non-ASCII stays intact.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(SCRIPT.as_bytes());
    if std::fs::read(&path).map(|b| b != bytes).unwrap_or(true) {
        std::fs::write(&path, &bytes).map_err(|e| format!("write script: {}", e))?;
    }
    Ok(path)
}

/// Run the script elevated (UAC prompt) and wait for it to finish.
fn run_elevated(action: Action, keep_lighting: bool) -> Result<(), String> {
    let script = install_script()?;
    let light: &[&str] = if keep_lighting { &["-KeepLighting"] } else { &[] };
    let (verb, extra): (&str, &[&str]) = match action {
        Action::Standby => ("standby", light),
        Action::Resume => ("resume", &["-DropGuard"]),
        Action::GuardOn => ("guard-on", light),
        Action::GuardOff => ("guard-off", &[]),
    };
    let q = |s: &str| s.replace('\'', "''");
    let mut args = vec![
        "'-NoProfile'".to_string(),
        "'-ExecutionPolicy'".to_string(),
        "'Bypass'".to_string(),
        "'-WindowStyle'".to_string(),
        "'Hidden'".to_string(),
        "'-File'".to_string(),
        format!("'\"{}\"'", q(&script.display().to_string())),
        format!("'{}'", verb),
        "'-Quiet'".to_string(),
    ];
    args.extend(extra.iter().map(|a| format!("'{}'", a)));
    let ps = format!(
        "try {{ Start-Process powershell -Verb RunAs -WindowStyle Hidden -Wait -ArgumentList {} }} catch {{ exit 2 }}",
        args.join(",")
    );
    // -EncodedCommand = base64 of UTF-16LE, which avoids every quoting problem.
    let utf16: Vec<u8> = ps.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let status = hidden("powershell")
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-EncodedCommand"])
        .arg(base64(&utf16))
        .status()
        .map_err(|e| format!("could not start powershell: {}", e))?;
    if status.success() {
        Ok(())
    } else {
        Err("Administrator approval was declined or the script could not start".to_string())
    }
}

impl Handle {
    /// Create the handle and start the background status poller.
    pub fn spawn() -> Self {
        let inner = Arc::new(Mutex::new(Status::default()));
        let poll = inner.clone();
        std::thread::spawn(move || loop {
            refresh(&poll);
            std::thread::sleep(Duration::from_secs(6));
        });
        Self { inner }
    }

    pub fn snapshot(&self) -> Status {
        self.inner.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Run an action on a worker thread; UI keeps running and shows `busy`.
    pub fn run(&self, action: Action, keep_lighting: bool) {
        {
            let mut s = match self.inner.lock() {
                Ok(s) => s,
                Err(_) => return,
            };
            if s.busy {
                return;
            }
            s.busy = true;
            s.message = None;
        }
        let inner = self.inner.clone();
        std::thread::spawn(move || {
            let result = run_elevated(action, keep_lighting);
            crate::dlog::log(&format!(
                "synapse {:?} keep_lighting={} -> {:?}",
                action, keep_lighting, result
            ));
            // Give processes / the Chroma engine a moment to settle, then re-read.
            std::thread::sleep(Duration::from_millis(1500));
            refresh(&inner);
            if let Ok(mut s) = inner.lock() {
                s.busy = false;
                s.message = result.err();
            }
        });
    }
}
