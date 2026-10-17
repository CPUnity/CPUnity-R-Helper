// Tiny persisted settings file (plain text key=value, no extra dependencies).
// Location: %APPDATA%\CPUnity-R-Helper\config.txt
use librazer::types::PerfMode;
use std::path::PathBuf;

/// Number of preset slots in each bank (keyboard, pad, both).
pub const PRESET_SLOTS: usize = 10;

/// Keyboard part of a lighting preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KbdPart {
    /// `KeyboardEffect::key()`, for example "static" or "wave_right".
    pub effect: String,
    pub color: [u8; 3],
    /// Raw keyboard brightness (0-255), same scale as the Keyboard Brightness slider.
    pub brightness: u8,
}

/// Cooling pad part of a lighting preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadPart {
    /// "Static", "Breathing" or "Off".
    pub mode: String,
    pub color: [u8; 3],
    /// Brightness in percent (0-100).
    pub brightness: u8,
}

/// A named lighting preset. Keyboard-bank slots only use `kbd`, pad-bank slots only `pad`;
/// "both" slots can hold either or both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightPreset {
    pub name: String,
    pub kbd: Option<KbdPart>,
    pub pad: Option<PadPart>,
}

/// Strips characters the config format cannot store and caps the length.
pub fn clean_preset_name(name: &str) -> String {
    let cleaned: String = name.chars().filter(|c| !c.is_control() && *c != '|').take(20).collect();
    cleaned.trim().to_string()
}

fn encode_color(c: [u8; 3]) -> String {
    format!("{},{},{}", c[0], c[1], c[2])
}

/// `name|kbd|pad`, each part `key,r,g,b,brightness` or empty when absent.
pub fn encode_preset(p: &LightPreset) -> String {
    let kbd = p
        .kbd
        .as_ref()
        .map(|k| format!("{},{},{}", k.effect, encode_color(k.color), k.brightness))
        .unwrap_or_default();
    let pad = p
        .pad
        .as_ref()
        .map(|d| format!("{},{},{}", d.mode, encode_color(d.color), d.brightness))
        .unwrap_or_default();
    format!("{}|{}|{}", clean_preset_name(&p.name), kbd, pad)
}

fn decode_part(text: &str) -> Option<(String, [u8; 3], u8)> {
    let parts: Vec<&str> = text.split(',').map(|s| s.trim()).collect();
    if parts.len() != 5 || parts[0].is_empty() {
        return None;
    }
    let r = parts[1].parse::<u8>().ok()?;
    let g = parts[2].parse::<u8>().ok()?;
    let b = parts[3].parse::<u8>().ok()?;
    let level = parts[4].parse::<u8>().ok()?;
    Some((parts[0].to_string(), [r, g, b], level))
}

pub fn decode_preset(text: &str) -> Option<LightPreset> {
    let mut it = text.splitn(3, '|');
    let name = clean_preset_name(it.next()?);
    let kbd_text = it.next()?;
    let pad_text = it.next()?;
    let kbd = decode_part(kbd_text).map(|(effect, color, brightness)| KbdPart {
        effect,
        color,
        brightness,
    });
    let pad = decode_part(pad_text).map(|(mode, color, brightness)| PadPart {
        mode,
        color,
        brightness: brightness.min(100),
    });
    if name.is_empty() || (kbd.is_none() && pad.is_none()) {
        return None;
    }
    Some(LightPreset { name, kbd, pad })
}

fn slot_index(key: &str, prefix: &str) -> Option<usize> {
    let i = key.strip_prefix(prefix)?.parse::<usize>().ok()?;
    (i < PRESET_SLOTS).then_some(i)
}

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
    /// Cooling pad fan: "" = never set by the app, "off" or "manual".
    pub pad_fan: String,
    /// Cooling pad manual fan speed (RPM).
    pub pad_rpm: u16,
    /// Cooling pad lighting color.
    pub pad_color: [u8; 3],
    /// Cooling pad lighting brightness (percent).
    pub pad_brightness: u8,
    /// Three saved cooling pad fan speeds (percent of the pad's maximum).
    pub pad_presets: [u8; 3],
    /// Opt-in: let R-Helper control the pad's lights. Off = lights are left to Chroma / Synapse.
    pub pad_lighting: bool,
    /// Opt-in: let R-Helper control the laptop keyboard's lighting effect and color.
    pub kbd_lighting: bool,
    /// Laptop keyboard effect color.
    pub kbd_color: [u8; 3],
    /// Advanced: send keyboard effects with the older "standard matrix effect" command.
    pub kbd_legacy: bool,
    /// Advanced: send keyboard effects with a random transaction id (default is the fixed 0x1F).
    pub kbd_random_id: bool,
    /// Advanced: send an "Off" effect before Static / Breathing.
    pub kbd_clear_first: bool,
    /// Advanced: paint Static as a custom frame (default on; the firmware Static effect ignores
    /// the color on some laptops).
    pub kbd_static_frame: bool,
    /// Opt-in: put the keyboard / pad lighting back as it was last used when the app starts
    /// (and when the pad is plugged in again). Needs "Toggle App Lighting Control" on.
    pub light_resume: bool,
    /// Last keyboard effect applied by the app (`KeyboardEffect::key()`), "" = none yet.
    pub kbd_last_effect: String,
    /// Last cooling pad lighting mode applied by the app, "" = none yet.
    pub pad_last_mode: String,
    /// Keyboard preset slots (always `PRESET_SLOTS` long).
    pub kbd_presets: Vec<Option<LightPreset>>,
    /// Cooling pad lighting preset slots.
    pub pad_light_presets: Vec<Option<LightPreset>>,
    /// Combined keyboard + pad preset slots.
    pub combo_presets: Vec<Option<LightPreset>>,
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
            pad_fan: String::new(),
            pad_rpm: 1500,
            pad_color: [255, 0, 0],
            pad_brightness: 100,
            pad_presets: [30, 60, 100],
            pad_lighting: false,
            kbd_lighting: false,
            kbd_color: [0, 200, 255],
            kbd_legacy: false,
            kbd_random_id: false,
            kbd_clear_first: false,
            kbd_static_frame: true,
            light_resume: false,
            kbd_last_effect: String::new(),
            pad_last_mode: String::new(),
            kbd_presets: vec![None; PRESET_SLOTS],
            pad_light_presets: vec![None; PRESET_SLOTS],
            combo_presets: vec![None; PRESET_SLOTS],
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
            "pad_fan" => {
                if value == "off" || value == "manual" {
                    cfg.pad_fan = value.to_string();
                }
            }
            "pad_rpm" => {
                if let Ok(v) = value.parse::<u16>() {
                    cfg.pad_rpm =
                        v.clamp(librazer::cooling_pad::MIN_RPM, librazer::cooling_pad::MAX_RPM);
                }
            }
            "pad_presets" => {
                let parts: Vec<u8> =
                    value.split(',').filter_map(|p| p.trim().parse::<u8>().ok()).collect();
                if parts.len() == 3 {
                    cfg.pad_presets =
                        [parts[0].clamp(15, 100), parts[1].clamp(15, 100), parts[2].clamp(15, 100)];
                }
            }
            "pad_lighting" => cfg.pad_lighting = value == "true" || value == "1",
            "kbd_lighting" => cfg.kbd_lighting = value == "true" || value == "1",
            "kbd_legacy" => cfg.kbd_legacy = value == "true" || value == "1",
            "kbd_random_id" => cfg.kbd_random_id = value == "true" || value == "1",
            "kbd_clear_first" => cfg.kbd_clear_first = value == "true" || value == "1",
            "kbd_static_frame" => cfg.kbd_static_frame = value == "true" || value == "1",
            "light_resume" => cfg.light_resume = value == "true" || value == "1",
            "kbd_last_effect" => cfg.kbd_last_effect = value.to_string(),
            "pad_last_mode" => cfg.pad_last_mode = value.to_string(),
            k if k.starts_with("kbd_slot_") => {
                if let Some(i) = slot_index(k, "kbd_slot_") {
                    cfg.kbd_presets[i] = decode_preset(value).filter(|p| p.kbd.is_some());
                }
            }
            k if k.starts_with("pad_slot_") => {
                if let Some(i) = slot_index(k, "pad_slot_") {
                    cfg.pad_light_presets[i] = decode_preset(value).filter(|p| p.pad.is_some());
                }
            }
            k if k.starts_with("combo_slot_") => {
                if let Some(i) = slot_index(k, "combo_slot_") {
                    cfg.combo_presets[i] = decode_preset(value);
                }
            }
            "kbd_color" => {
                let parts: Vec<u8> =
                    value.split(',').filter_map(|p| p.trim().parse::<u8>().ok()).collect();
                if parts.len() == 3 {
                    cfg.kbd_color = [parts[0], parts[1], parts[2]];
                }
            }
            "pad_color" => {
                let parts: Vec<u8> =
                    value.split(',').filter_map(|p| p.trim().parse::<u8>().ok()).collect();
                if parts.len() == 3 {
                    cfg.pad_color = [parts[0], parts[1], parts[2]];
                }
            }
            "pad_brightness" => {
                if let Ok(v) = value.parse::<u8>() {
                    cfg.pad_brightness = v.min(100);
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
        "# CPUnity:R-Helper settings\naccent={},{},{}\nalways_on_top={}\nac_mode={}\nbattery_mode={}\nac_overclock={}\nsynapse_keep_lighting={}\nfont={}\nsaved_colors={}\nbattery_limit={}\npad_fan={}\npad_rpm={}\npad_color={},{},{}\npad_brightness={}\npad_presets={},{},{}\npad_lighting={}\nkbd_lighting={}\nkbd_color={},{},{}\nkbd_legacy={}\nkbd_random_id={}\nkbd_clear_first={}\nkbd_static_frame={}\nlight_resume={}\nkbd_last_effect={}\npad_last_mode={}\n",
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
        cfg.pad_fan,
        cfg.pad_rpm,
        cfg.pad_color[0],
        cfg.pad_color[1],
        cfg.pad_color[2],
        cfg.pad_brightness,
        cfg.pad_presets[0],
        cfg.pad_presets[1],
        cfg.pad_presets[2],
        cfg.pad_lighting,
        cfg.kbd_lighting,
        cfg.kbd_color[0],
        cfg.kbd_color[1],
        cfg.kbd_color[2],
        cfg.kbd_legacy,
        cfg.kbd_random_id,
        cfg.kbd_clear_first,
        cfg.kbd_static_frame,
        cfg.light_resume,
        cfg.kbd_last_effect,
        cfg.pad_last_mode,
    );
    let mut text = text;
    for (prefix, bank) in [
        ("kbd_slot_", &cfg.kbd_presets),
        ("pad_slot_", &cfg.pad_light_presets),
        ("combo_slot_", &cfg.combo_presets),
    ] {
        for (i, slot) in bank.iter().enumerate().take(PRESET_SLOTS) {
            if let Some(preset) = slot {
                text.push_str(&format!("{}{}={}\n", prefix, i, encode_preset(preset)));
            }
        }
    }
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

#[cfg(test)]
mod preset_tests {
    use super::*;

    #[test]
    fn preset_round_trip() {
        let p = LightPreset {
            name: "Night".into(),
            kbd: Some(KbdPart { effect: "breathing".into(), color: [1, 2, 3], brightness: 120 }),
            pad: Some(PadPart { mode: "Static".into(), color: [9, 8, 7], brightness: 40 }),
        };
        assert_eq!(decode_preset(&encode_preset(&p)), Some(p));
    }

    #[test]
    fn single_part_round_trip() {
        let p = LightPreset {
            name: "Pad only".into(),
            kbd: None,
            pad: Some(PadPart { mode: "Off".into(), color: [0, 0, 0], brightness: 0 }),
        };
        assert_eq!(decode_preset(&encode_preset(&p)), Some(p));
    }

    #[test]
    fn bad_presets_are_rejected() {
        assert!(decode_preset("").is_none());
        assert!(decode_preset("name||").is_none());
        assert!(decode_preset("|static,1,2,3,4|").is_none());
        assert!(decode_preset("x|static,1,2,300,4|").is_none());
    }

    #[test]
    fn names_are_cleaned() {
        assert_eq!(clean_preset_name("  a|b\nc  "), "abc");
        assert_eq!(clean_preset_name(&"x".repeat(50)).chars().count(), 20);
    }

    #[test]
    fn slot_index_is_bounded() {
        assert_eq!(slot_index("kbd_slot_9", "kbd_slot_"), Some(9));
        assert_eq!(slot_index("kbd_slot_10", "kbd_slot_"), None);
        assert_eq!(slot_index("kbd_slot_x", "kbd_slot_"), None);
    }
}
