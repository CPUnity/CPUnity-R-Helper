// Tiny persisted settings file (plain text key=value, no extra dependencies).
// Location: %APPDATA%\CPUnity-R-Helper\config.txt
use librazer::types::PerfMode;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub accent: [u8; 3],
    pub always_on_top: bool,
    /// Perf mode applied when AC is plugged in (auto-switch). None = leave default.
    pub ac_mode: Option<String>,
    /// Perf mode applied when on battery (auto-switch).
    pub battery_mode: Option<String>,
    /// On AC power: switch to Custom mode and apply the CPU Overclock preset.
    pub ac_overclock: bool,
    /// Synapse standby: keep the Chroma-only engine running (lighting effects keep animating).
    pub synapse_keep_lighting: bool,
    /// UI font choice (see theme::FONT_CHOICES).
    pub font: String,
    /// User-saved accent colors (max 10).
    pub saved_colors: Vec<[u8; 3]>,
    /// Last battery charge limit chosen (percent), re-applied by the UI only on user action.
    pub battery_limit: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            accent: [0, 120, 60],
            always_on_top: false,
            ac_mode: None,
            battery_mode: None,
            ac_overclock: false,
            synapse_keep_lighting: true,
            font: "Bahnschrift".to_string(),
            saved_colors: Vec::new(),
            battery_limit: 80,
        }
    }
}

pub fn config_dir() -> PathBuf {
    let base =
        std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| std::env::temp_dir());
    base.join("CPUnity-R-Helper")
}

fn config_path() -> PathBuf {
    config_dir().join("config.txt")
}

/// Looks for `<stem>.png|jpg|jpeg` next to the exe first, then in the config dir.
fn find_image(stem: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.to_path_buf());
        }
    }
    dirs.push(config_dir());
    for dir in dirs {
        for ext in ["png", "jpg", "jpeg"] {
            let p = dir.join(format!("{}.{}", stem, ext));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Optional small logo shown left of the title (logo.png / logo.jpg).
pub fn find_logo() -> Option<PathBuf> {
    find_image("logo")
}

/// Optional header banner filling the space right of the title (banner.png / banner.jpg).
pub fn find_banner() -> Option<PathBuf> {
    find_image("banner")
}

pub fn load() -> Config {
    let mut cfg = Config::default();
    let text = match std::fs::read_to_string(config_path()) {
        Ok(t) => t,
        Err(_) => return cfg,
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = match line.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };
        match key {
            "accent" => {
                let parts: Vec<u8> =
                    value.split(',').filter_map(|p| p.trim().parse::<u8>().ok()).collect();
                if parts.len() == 3 {
                    cfg.accent = [parts[0], parts[1], parts[2]];
                }
            }
            "always_on_top" => cfg.always_on_top = value == "true",
            "saved_colors" => {
                cfg.saved_colors = value
                    .split(';')
                    .filter_map(|item| {
                        let p: Vec<u8> =
                            item.split(',').filter_map(|x| x.trim().parse::<u8>().ok()).collect();
                        if p.len() == 3 {
                            Some([p[0], p[1], p[2]])
                        } else {
                            None
                        }
                    })
                    .take(10)
                    .collect();
            }
            "font" => {
                if !value.is_empty() {
                    cfg.font = value.to_string();
                }
            }
            "synapse_keep_lighting" => cfg.synapse_keep_lighting = value == "true" || value == "1",
            "ac_overclock" | "ac_undervolt" => cfg.ac_overclock = value == "true",
            "ac_mode" => {
                if !value.is_empty() && value != "default" {
                    cfg.ac_mode = Some(value.to_string());
                }
            }
            "battery_mode" => {
                if !value.is_empty() && value != "default" {
                    cfg.battery_mode = Some(value.to_string());
                }
            }
            "battery_limit" => {
                if let Ok(v) = value.parse::<u8>() {
                    cfg.battery_limit = v.clamp(
                        librazer::command::BATTERY_LIMIT_MIN,
                        librazer::command::BATTERY_LIMIT_MAX,
                    );
                }
            }
            _ => {}
        }
    }
    cfg
}

pub fn save(cfg: &Config) {
    let dir = config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let text = format!(
        "# CPUnity:R-Helper settings\naccent={},{},{}\nalways_on_top={}\nac_mode={}\nbattery_mode={}\nac_overclock={}\nsynapse_keep_lighting={}\nfont={}\nsaved_colors={}\nbattery_limit={}\n",
        cfg.accent[0],
        cfg.accent[1],
        cfg.accent[2],
        cfg.always_on_top,
        cfg.ac_mode.clone().unwrap_or_else(|| "default".into()),
        cfg.battery_mode.clone().unwrap_or_else(|| "default".into()),
        cfg.ac_overclock,
        cfg.synapse_keep_lighting,
        cfg.font,
        cfg.saved_colors
            .iter()
            .map(|c| format!("{},{},{}", c[0], c[1], c[2]))
            .collect::<Vec<_>>()
            .join(";"),
        cfg.battery_limit,
    );
    let _ = std::fs::write(config_path(), text);
}

/// Parse a stored perf-mode name ("Performance", "Hyperboost", ...) back into the enum.
pub fn perf_mode_from_name(name: &str) -> Option<PerfMode> {
    use strum::IntoEnumIterator;
    PerfMode::iter().find(|m| format!("{:?}", m) == name)
}

// ---- launch at Windows startup (HKCU\...\Run, via reg.exe, no extra crates) ----

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_NAME: &str = "CPUnity-R-Helper";

fn reg() -> std::process::Command {
    let mut cmd = std::process::Command::new("reg");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: no console flash from the GUI app
    }
    cmd
}

pub fn startup_enabled() -> bool {
    reg()
        .args(["query", RUN_KEY, "/v", RUN_NAME])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn set_startup(enabled: bool) {
    if enabled {
        if let Ok(exe) = std::env::current_exe() {
            let value = format!("\"{}\"", exe.display());
            let _ = reg()
                .args(["add", RUN_KEY, "/v", RUN_NAME, "/t", "REG_SZ", "/d", &value, "/f"])
                .output();
        }
    } else {
        let _ = reg().args(["delete", RUN_KEY, "/v", RUN_NAME, "/f"]).output();
    }
}
