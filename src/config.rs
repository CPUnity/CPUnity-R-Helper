// Tiny persisted settings file (plain text key=value, no extra dependencies).
// Location: %APPDATA%\CPUnity-R-Helper\config.txt
use librazer::keymap::KeyLayout;
use librazer::types::PerfMode;
use std::path::PathBuf;

/// Number of preset slots in each bank (keyboard, pad, both).
pub const PRESET_SLOTS: usize = 20;

/// Most cooling pad fan profiles that can be saved.
pub const PAD_PROFILES_MAX: usize = 10;

/// A named cooling pad fan speed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadProfile {
    pub name: String,
    /// Exact fan speed; the pad works in steps of `RPM_STEP`.
    pub rpm: u16,
}

/// Rounds `rpm` to a speed the pad accepts (a multiple of the step, inside its range).
pub fn snap_rpm(rpm: u16) -> u16 {
    use librazer::cooling_pad::{MAX_RPM, MIN_RPM, RPM_STEP};
    let stepped = ((rpm as u32 + RPM_STEP as u32 / 2) / RPM_STEP as u32) * RPM_STEP as u32;
    (stepped.min(MAX_RPM as u32) as u16).clamp(MIN_RPM, MAX_RPM)
}

pub fn encode_profile(p: &PadProfile) -> String {
    format!("{}|{}", p.name, p.rpm)
}

pub fn decode_profile(text: &str) -> Option<PadProfile> {
    let (name, rpm) = text.rsplit_once('|')?;
    let rpm = rpm.trim().parse::<u16>().ok()?;
    let name = clean_preset_name(name);
    Some(PadProfile {
        name: if name.is_empty() { "Profile".to_string() } else { name },
        rpm: snap_rpm(rpm),
    })
}

/// The profiles a fresh install starts with (also what the old three fan presets become).
fn profiles_from_percents(percents: [u8; 3], named: bool) -> Vec<PadProfile> {
    use librazer::cooling_pad::rpm_for_percent;
    const NAMES: [&str; 3] = ["Low", "Medium", "Max"];
    (0..3)
        .map(|i| PadProfile {
            name: if named { NAMES[i].to_string() } else { format!("{}%", percents[i]) },
            rpm: rpm_for_percent(percents[i]),
        })
        .collect()
}

/// Keyboard part of a lighting preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KbdPart {
    /// `KeyboardEffect::key()`, for example "static" or "wave_right".
    pub effect: String,
    pub color: [u8; 3],
    /// Second color (two-color Breathing / Static), if used.
    pub color2: Option<[u8; 3]>,
    /// Layout of two colors: `KeyboardSplit::index()` for Static, `breath_index()` for Breathing.
    pub split: u8,
    /// Raw keyboard brightness (0-255), same scale as the Keyboard Brightness slider.
    pub brightness: u8,
    /// Custom Keys only: the lit keys, as `KeyLayout::encode()` text.
    pub layout: Option<String>,
}

/// Cooling pad part of a lighting preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadPart {
    /// "Static", "Breathing" or "Off".
    pub mode: String,
    pub color: [u8; 3],
    /// Second color (two-color Breathing / Static), if used.
    pub color2: Option<[u8; 3]>,
    /// Two-color layout: Static 0 = two halves, 1 = gradient; Breathing 0 = alternate,
    /// 1 = two halves, 2 = gradient.
    pub split: u8,
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

/// One part as `key,r,g,b,brightness`, plus `,r2,g2,b2,split` when a second color is used, plus
/// `,layout` (a tenth field) for Custom Keys. A custom part has no second color: its `r2,g2,b2`
/// fields are zeros and `split` is kept.
fn encode_part(
    key: &str,
    color: [u8; 3],
    level: u8,
    color2: Option<[u8; 3]>,
    split: u8,
    layout: Option<&str>,
) -> String {
    let mut text = format!("{},{},{}", key, encode_color(color), level);
    if let Some(layout) = layout {
        text.push_str(&format!(",0,0,0,{},{}", split, layout));
    } else if let Some(c2) = color2 {
        text.push_str(&format!(",{},{}", encode_color(c2), split));
    }
    text
}

/// `name|kbd|pad`, each part as described at `encode_part` or empty when absent.
pub fn encode_preset(p: &LightPreset) -> String {
    let kbd = p
        .kbd
        .as_ref()
        .map(|k| {
            encode_part(&k.effect, k.color, k.brightness, k.color2, k.split, k.layout.as_deref())
        })
        .unwrap_or_default();
    let pad = p
        .pad
        .as_ref()
        .map(|d| encode_part(&d.mode, d.color, d.brightness, d.color2, d.split, None))
        .unwrap_or_default();
    format!("{}|{}|{}", clean_preset_name(&p.name), kbd, pad)
}

/// `(key, color, brightness, color2, split, layout)`
type DecodedPart = (String, [u8; 3], u8, Option<[u8; 3]>, u8, Option<String>);

fn decode_part(text: &str) -> Option<DecodedPart> {
    let parts: Vec<&str> = text.split(',').map(|s| s.trim()).collect();
    if (parts.len() != 5 && parts.len() != 9 && parts.len() != 10) || parts[0].is_empty() {
        return None;
    }
    let r = parts[1].parse::<u8>().ok()?;
    let g = parts[2].parse::<u8>().ok()?;
    let b = parts[3].parse::<u8>().ok()?;
    let level = parts[4].parse::<u8>().ok()?;
    let (color2, split, layout) = match parts.len() {
        9 => {
            let r2 = parts[5].parse::<u8>().ok()?;
            let g2 = parts[6].parse::<u8>().ok()?;
            let b2 = parts[7].parse::<u8>().ok()?;
            (Some([r2, g2, b2]), parts[8].parse::<u8>().ok()?, None)
        }
        10 => {
            // Custom Keys: no second color, then the split and the layout text.
            let layout = parts[9];
            KeyLayout::decode(layout)?;
            (None, parts[8].parse::<u8>().ok()?, Some(layout.to_string()))
        }
        _ => (None, 0, None),
    };
    Some((parts[0].to_string(), [r, g, b], level, color2, split, layout))
}

pub fn decode_preset(text: &str) -> Option<LightPreset> {
    let mut it = text.splitn(3, '|');
    let name = clean_preset_name(it.next()?);
    let kbd_text = it.next()?;
    let pad_text = it.next()?;
    let kbd = decode_part(kbd_text).map(|(effect, color, brightness, color2, split, layout)| {
        KbdPart { effect, color, color2, split, brightness, layout }
    });
    let pad = decode_part(pad_text).map(|(mode, color, brightness, color2, split, _)| PadPart {
        mode,
        color,
        color2,
        split,
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

/// The tabs that can be switched off in Options: (config key, name). Main is not in the list: it is
/// always shown. The order here is the order in Options.
pub const TOGGLE_TABS: [(&str, &str); 6] = [
    ("lighting", "Lighting"),
    ("sensors", "Sensors"),
    ("files", "Files"),
    ("help", "Help"),
    ("changelog", "Changelog"),
    ("info", "Info"),
];

#[derive(Debug, Clone)]
pub struct Config {
    pub accent: [u8; 3],
    pub always_on_top: bool,
    /// The window's close button hides it to the system tray instead of quitting.
    pub close_to_tray: bool,
    /// The picture behind the tabs (Options > Backdrop). Off: plain dark background. Off until you
    /// choose one (so a new player has to go looking for the cat).
    pub backdrop: bool,
    /// Start hidden in the system tray instead of showing the window (Options > Start minimized).
    pub start_minimized: bool,
    /// The animated pictures (Fire dragons, Father Time) move. Off: they stand still.
    pub backdrop_animated: bool,
    /// The Gold Edition has been earned (beating Father Time) or unlocked for a preview.
    pub gold_unlocked: bool,
    /// The Gold Edition look is switched on (only has an effect once unlocked).
    pub gold_on: bool,
    /// The Gold Edition banner under the title.
    pub gold_banner: bool,
    /// The window glitches (pixelates for a moment) when the cat king laughs.
    pub gold_glitch: bool,
    /// The drifting sparkles (and sun rays) of the Gold Edition.
    pub gold_sparkles: bool,
    /// Game 2 has been won (or the Void was unlocked for a preview): The Void look is unlocked.
    pub void_unlocked: bool,
    /// The Void look is switched on (only has an effect once unlocked). It wins over Gold.
    pub void_on: bool,
    /// The Void's torn banner under the title.
    pub void_banner: bool,
    /// The Platinum banner under the title while the Platinum look is on.
    pub platinum_banner: bool,
    /// Platinum's moving light: the sweep over the cards, the glints and the glimmer of the facets.
    /// Off: the look stands still.
    pub platinum_effects: bool,
    /// Which of the Underworld's whispers have been heard (one bit each; see `whispers.rs`).
    pub whispers_heard: u32,
    /// How many whispers there have been in all (so every line is heard within a set number).
    pub whispers_spoken: u32,
    /// The Info tab is written in The Void's script (hover a word to read it). Off: the plain text.
    pub void_script: bool,
    /// Picks where the sentence hidden in the Info text sits (0: not picked yet). It is chosen when
    /// The Void is unlocked and kept until it is locked again, so the sentence stays put.
    pub void_seed: u64,
    /// The moon's staff has been broken (the moon struck it): the moon is gone and Game 3 has begun.
    /// Locking The Void, or Options > The Void > Mend the staff, brings it back.
    pub void_staff_broken: bool,
    /// Test switch: the moon is there by day too, as if it were night.
    pub void_pretend_night: bool,
    /// Test switch (Options > Game testing): nothing costs a life in Games 1 and 2. Never saved, so it is off at every start.
    pub dev_invincible: bool,
    /// Test switch (Options > Game testing): show the Underworld although this build holds it back (see `release.rs`). Saved.
    pub dev_show_underworld: bool,
    /// The Underworld look (lava, rock, skeletons) is switched on. It only has an effect once the
    /// Underworld is open (see `games.rs`), and it wins over The Void and the Gold Edition.
    pub underworld_on: bool,
    /// The hidden games: deaths in a row, whether Options has started offering their difficulty,
    /// and the difficulty chosen (see `games.rs`).
    pub games: crate::games::Progress,
    /// CR-Tokens: the balance, what the shop has sold and the skin that is on (see `shop.rs`).
    pub shop: crate::shop::Wallet,
    /// The Hold'em table in the Underworld: games played and won, and the soul lock (see `poker_save.rs`).
    pub poker: crate::poker_save::PokerSave,
    /// The trophies that have been earned (see `trophies.rs`).
    pub trophies: crate::trophies::Book,
    /// Not saved: a skin or backdrop from the shop being tried on, by its shop id (free).
    pub shop_preview: Option<String>,
    /// Which picture it is (Options > Backdrop style).
    pub backdrop_style: crate::info::BackdropStyle,
    /// Tabs the user switched off in Options (keys from `TOGGLE_TABS`). Main is always shown.
    /// Empty by default: every tab is there.
    pub hidden_tabs: Vec<String>,
    /// The Trophies tab in the top row. Off until the user switches it on in Options > Settings >
    /// Tabs, which only offers it once a trophy has been earned.
    pub trophies_tab: bool,
    /// The pop-up that says where the Trophies tab is has been closed with "Okay".
    pub trophies_tab_told: bool,
    /// Start elevated (Windows asks for permission once at startup).
    pub run_as_admin: bool,
    /// Perf mode applied when AC is plugged in (auto-switch). None = leave default.
    pub ac_mode: Option<String>,
    /// Perf mode applied when on battery (auto-switch).
    pub battery_mode: Option<String>,
    /// On AC power: switch to Custom mode and apply the CPU Overclock preset.
    pub ac_overclock: bool,
    /// The CPU / GPU boost last picked in Custom mode (`Low`, `Medium`, ...). A saved Custom
    /// profile puts them back at launch; the laptop forgets them (GPU back to Low) otherwise.
    pub custom_cpu: Option<String>,
    pub custom_gpu: Option<String>,
    /// The CPU and GPU boost chosen in Options for Custom mode on AC power and on battery
    /// (`None` is Default: the boosts last picked on the main page).
    pub ac_cpu: Option<String>,
    pub ac_gpu: Option<String>,
    pub battery_cpu: Option<String>,
    pub battery_gpu: Option<String>,
    /// "Last Known Settings" for AC and for battery: put back exactly what the laptop was running
    /// last on that source (`power_plan::LastState` text), instead of the choices above.
    pub ac_last: bool,
    pub battery_last: bool,
    pub ac_last_state: Option<String>,
    pub battery_last_state: Option<String>,
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
    /// Opt-in: let R-Helper control the pad's fan. Off = the fan is left to the pad / Synapse.
    pub pad_control: bool,
    /// Which sensor sources are switched on (see `sensor_ext`; the built-in one always is).
    pub sensor_enabled: [bool; crate::sensor_ext::SOURCE_COUNT],
    /// Per reading: `sensor_ext::AUTO` or the number of the source it comes from.
    pub sensor_picks: [u8; crate::sensor_ext::READING_COUNT],
    /// Write every reading to a CSV file while the sensors are on. Never saved: logging is
    /// always off after a start, so nothing is written to disk unless it is switched on again.
    pub sensor_logging: bool,
    /// Name for sensor log files (the date is put in front). Empty = "sensors-<time>".
    pub sensor_log_name: String,
    /// Saved file names for the log (Options > Logging > Name preset). The file is `<date>_<name>.csv`.
    pub sensor_log_name_presets: Vec<String>,
    /// The built-in file name preset (Date, Time, Date and time, or None).
    pub sensor_log_stamp: crate::sensor_log::Stamp,
    /// When a log starts: as soon as the switch is on, or each time `sensor_log_program` starts.
    pub sensor_log_start: crate::log_plan::Start,
    /// The program that starts the log (`Cinebench.exe`).
    pub sensor_log_program: String,
    /// Seconds after which a log stops by itself. 0 = no limit: it goes on until switched off.
    pub sensor_log_limit_secs: u64,
    /// The log stops when the program closes.
    pub sensor_log_stop_with_program: bool,
    /// "Read sensors" is on (it is switched on again at the next start).
    pub sensor_read: bool,
    /// Time span shown by the Sensors graphs (index into `GRAPH_WINDOWS`).
    pub graph_window: u8,
    /// Record and draw the Sensors graphs.
    pub sensor_graphs: bool,
    /// Milliseconds between two sensor readings.
    pub sensor_interval_ms: u64,
    /// `host:port` of the LibreHardwareMonitor web server.
    pub sensor_lhm_addr: String,
    /// Path of the CSV log file (ThrottleStop / XTU / HWiNFO).
    pub sensor_log_path: String,
    /// Settings of the log viewer window, as one line (see `log_options`).
    pub log_view_options: String,
    /// Path of PawnIO's IntelMSR.bin module (empty = look in the usual places).
    pub sensor_module_path: String,
    /// Named fan profiles (at most `PAD_PROFILES_MAX`).
    pub pad_profiles: Vec<PadProfile>,
    /// Opt-in: let R-Helper control the pad's lights. Off = lights are left to Chroma / Synapse.
    pub pad_lighting: bool,
    /// Second cooling pad color, used when `pad_two` is on.
    pub pad_color2: [u8; 3],
    /// Two colors on the pad: Breathing alternates, Static becomes two halves or a gradient.
    pub pad_two: bool,
    /// Two-color pad Static layout: 0 = two halves, 1 = gradient.
    pub pad_split: u8,
    /// Two-color pad Breathing layout: 0 = alternate (firmware), 1 = two halves, 2 = gradient
    /// (the last two are animated by the app).
    pub pad_breath_split: u8,
    /// Breathing speed for the cooling pad, a percent of the standard speed (100 = standard).
    pub pad_breath_speed: u8,
    /// Opt-in: let R-Helper control the laptop keyboard's lighting effect and color.
    pub kbd_lighting: bool,
    /// Laptop keyboard effect color.
    pub kbd_color: [u8; 3],
    /// Second keyboard color, used when `kbd_two` is on.
    pub kbd_color2: [u8; 3],
    /// The Fire effect's hot and cold colors (Chroma's own are orange and red).
    pub fire_hot: [u8; 3],
    pub fire_cold: [u8; 3],
    /// The cooling pad's own Fire colors (separate from the keyboard's; the sync card sets both).
    pub pad_fire_hot: [u8; 3],
    pub pad_fire_cold: [u8; 3],
    /// The Tidal effect's two colors (the captured ones are green and blue) and its direction:
    /// off = the colors jet out from a diagonal line, on = they come in towards it.
    pub tidal_a: [u8; 3],
    pub tidal_b: [u8; 3],
    pub tidal_in: bool,
    /// The cooling pad's own Tidal colors and direction (the sync card sets both together).
    pub pad_tidal_a: [u8; 3],
    pub pad_tidal_b: [u8; 3],
    pub pad_tidal_in: bool,
    /// The Starlight effect's color, and its second color when `starlight_two` is on (the
    /// capture used green; the pad has its own, the sync card sets both).
    pub starlight_a: [u8; 3],
    pub starlight_b: [u8; 3],
    pub starlight_two: bool,
    pub pad_starlight_a: [u8; 3],
    pub pad_starlight_b: [u8; 3],
    pub pad_starlight_two: bool,
    /// The two colors Color Dipp goes between, for the keyboard and for the pad.
    pub dipp_a: [u8; 3],
    pub dipp_b: [u8; 3],
    pub pad_dipp_a: [u8; 3],
    pub pad_dipp_b: [u8; 3],
    /// How fast each effect goes, as a percent of its own speed (100 = how it goes by default):
    /// the keyboard's, and the cooling pad's apart.
    pub fire_speed: u8,
    pub pad_fire_speed: u8,
    pub tidal_speed: u8,
    pub pad_tidal_speed: u8,
    pub starlight_speed: u8,
    pub pad_starlight_speed: u8,
    pub spectrum_speed: u8,
    pub pad_spectrum_speed: u8,
    pub dipp_speed: u8,
    pub pad_dipp_speed: u8,
    pub rainbow_speed: u8,
    pub pad_rainbow_speed: u8,
    /// Two colors: Breathing alternates between them, Static becomes a two-color picture.
    pub kbd_two: bool,
    /// Two-color Static layout (`KeyboardSplit::index()`).
    pub kbd_split: u8,
    /// Two-color Breathing layout: 0 = alternate (firmware), 1 = Left | Right, 2 = Top | Bottom,
    /// 3 = Gradient (the last three are animated by the app). `KeyboardSplit::breath_index()`.
    pub kbd_breath_split: u8,
    /// Breathing speed for the keyboard, a percent of the standard speed (100 = standard).
    pub kbd_breath_speed: u8,
    /// One set of lighting controls drives both the keyboard and the cooling pad.
    pub light_sync: bool,
    /// Open / closed state of the drop-downs the user has toggled (`key` -> open).
    pub open_tabs: Vec<(String, bool)>,
    /// Custom Keys: which keys are lit and in which color.
    pub kbd_custom: KeyLayout,
    /// Custom Keys: the color a click in the key editor paints.
    pub kbd_brush: [u8; 3],
    /// What a key does when it is painted (`KeyFx::index()`: 0 = steady, 1 = Breathing, ...).
    pub kbd_brush_fx: u8,
    /// Custom Keys: the brush's second color, and whether a key painted with an effect that can use
    /// one gets it (Color Dipp always does).
    pub kbd_brush2: [u8; 3],
    pub kbd_brush_two: bool,
    /// Wave Right / Wave Left in colors of your own (the firmware's wave is a rainbow): when
    /// `wave_custom` is on the app draws the wave washing between `wave_a` and `wave_b` (pick a very
    /// dark second color for bands of one color on dark).
    pub wave_custom: bool,
    pub wave_a: [u8; 3],
    pub wave_b: [u8; 3],
    pub wave_speed: u8,
    /// The Wheel effect. Off (the default) it is Razer's own, the rainbow turning round, which has
    /// no colors; on, R-Helper draws a hand in `wheel_a` with a tail turning into `wheel_b`.
    /// The direction (counter-clockwise when on) and the speed go with both.
    pub wheel_custom: bool,
    pub wheel_a: [u8; 3],
    pub wheel_b: [u8; 3],
    pub wheel_ccw: bool,
    pub wheel_speed: u8,
    /// The cooling pad's Wheel: its speed and direction (counter-clockwise when on).
    pub pad_wheel_speed: u8,
    pub pad_wheel_ccw: bool,
    /// The pad's Wheel in two colors of your own (off: Razer's rainbow).
    pub pad_wheel_custom: bool,
    pub pad_wheel_a: [u8; 3],
    pub pad_wheel_b: [u8; 3],
    /// Custom Keys: moves the whole key map this many columns (fine-tune, -3 to 3).
    pub kbd_key_shift: i8,
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
    /// The game preset of a look is put on when that look is unlocked or switched on (only if
    /// the lighting toggles are on).
    pub game_preset_auto: bool,
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

impl Config {
    /// Whether the tab with this key (from `TOGGLE_TABS`) is shown. Unknown keys count as shown.
    /// The Gold Edition look is showing: earned (or previewed) and switched on.
    pub fn gold_active(&self) -> bool {
        crate::release::story() && self.gold_unlocked && self.gold_on && !self.void_active() && !self.underworld_active() && !self.trying_skin()
    }

    /// A skin from the shop is being tried on: it takes the place of whichever earned look is on.
    pub fn trying_skin(&self) -> bool {
        crate::release::story() && self.shop_preview.as_deref().and_then(crate::shop::item).is_some_and(|i| i.kind == crate::shop::Kind::Skin)
    }

    /// The Void look is showing: earned (or previewed) and switched on. It takes the place of Gold.
    pub fn void_active(&self) -> bool {
        crate::release::story() && self.void_unlocked && self.void_on && !self.underworld_active() && !self.trying_skin()
    }

    /// The Underworld look is showing: the Underworld has been opened (the end of Game 3) and the
    /// look is switched on. It takes the place of The Void and the Gold Edition while it is.
    pub fn underworld_active(&self) -> bool {
        self.underworld_visible() && self.underworld_on && !self.trying_skin()
    }

    /// Whether this build shows the Underworld at all (see `release.rs`): it is held back unless the
    /// test switch is on. (Always on in the tests.)
    pub fn underworld_out(&self) -> bool {
        crate::release::UNDERWORLD || self.dev_show_underworld || cfg!(test)
    }

    /// The Underworld has been opened (the end of Game 3, kept in the save file) and this build shows it.
    pub fn underworld_visible(&self) -> bool {
        self.games.underworld() && self.underworld_out()
    }

    /// The backdrop style to draw: the chosen one, except that a style which comes with the Gold
    /// Edition (the angel cats) is not used while Gold is locked, whatever the config file says.
    pub fn shown_backdrop_style(&self) -> crate::info::BackdropStyle {
        // a backdrop from the shop being tried on shows whatever is chosen
        if let Some(tried) = self
            .shop_preview
            .as_deref()
            .and_then(|id| crate::info::BackdropStyle::ALL.into_iter().find(|s| s.shop_id() == Some(id)))
        {
            return tried;
        }
        // one from the shop that has not been bought is not used, whatever the config file says
        if self.backdrop_style.shop_id().is_some_and(|id| !self.shop.owns(id)) {
            return crate::info::BackdropStyle::default();
        }
        // the story's own pictures are not shown by a build that holds the story back
        if !self.backdrop_style.released() {
            return crate::info::BackdropStyle::default();
        }
        if (self.backdrop_style.needs_gold() && !self.gold_unlocked && crate::release::story())
            || (self.backdrop_style.needs_void() && !self.void_unlocked)
            || (self.backdrop_style.inside_void_only() && !self.void_active())
            || (self.backdrop_style.inside_underworld_only() && !self.underworld_active())
        {
            crate::info::BackdropStyle::default()
        } else {
            self.backdrop_style
        }
    }

    /// The skin from the shop that is showing: the one being tried on, or the one that is on. None
    /// while one of the earned looks is showing (they win: "Use" in the shop turns them off).
    pub fn skin_active(&self) -> Option<&str> {
        if self.trying_skin() {
            return self.shop_preview.as_deref();
        }
        if self.gold_active() || self.void_active() || self.underworld_active() {
            return None;
        }
        self.shop.skin()
    }

    /// The Fraying skin is showing.
    pub fn fraying_active(&self) -> bool {
        self.skin_active() == Some("fraying")
    }

    /// The Sakura skin is showing.
    pub fn sakura_active(&self) -> bool {
        self.skin_active() == Some("sakura")
    }

    /// The Synthwave skin is showing.
    pub fn synthwave_active(&self) -> bool {
        self.skin_active() == Some("synthwave")
    }

    /// Puts a bought skin on, in place of the looks that were showing: one look at a time. A skin
    /// that is not bought is ignored.
    pub fn use_skin(&mut self, id: &str) {
        self.shop.set_skin(Some(id));
        if self.shop.skin() == Some(id) {
            self.gold_on = false;
            self.void_on = false;
            self.underworld_on = false;
            self.shop_preview = None;
        }
    }

    /// The look the program has now, which decides which game's difficulty Options offers.
    pub fn edition(&self) -> crate::games::Edition {
        // (the Underworld does not change this: the games stay those of the look under it)
        if !crate::release::story() {
            crate::games::Edition::Standard
        } else if self.void_unlocked && self.void_on {
            crate::games::Edition::Void
        } else if self.gold_unlocked && self.gold_on {
            crate::games::Edition::Gold
        } else {
            crate::games::Edition::Standard
        }
    }

    /// Whether Options has a Games section now: the game of the look you are in has killed you
    /// often enough in a row (see `games.rs`).
    pub fn games_section_shown(&self) -> bool {
        self.games.shown(self.edition().game())
    }

    /// What the trophies are worked out from (see `trophies.rs`).
    pub fn trophy_facts(&self) -> crate::trophies::Facts {
        use crate::shop::{Kind, CATALOG};
        let skins = || CATALOG.iter().filter(|i| i.kind == Kind::Skin);
        crate::trophies::Facts {
            won: [0, 1, 2].map(|g| self.shop.has_won(g)),
            deaths: [0, 1, 2].map(|g| self.games.deaths(g)),
            cheats_earned: crate::games::Cheat::ALL.iter().filter(|c| self.games.cheat_unlocked(**c)).count(),
            cheats_total: crate::games::Cheat::ALL.len(),
            act1: self.games.cat_helped(),
            gold: self.gold_unlocked,
            void: self.void_unlocked,
            underworld: self.underworld_visible(),
            bought: CATALOG.iter().filter(|i| self.shop.owns(i.id)).count(),
            for_sale: CATALOG.len(),
            skins_bought: skins().filter(|i| self.shop.owns(i.id)).count(),
            skins_for_sale: skins().count(),
            tokens: self.shop.lifetime(),
            whispers: (self.whispers_heard & ((1u32 << crate::whispers::LINES.len()) - 1)).count_ones(),
            whispers_total: crate::whispers::LINES.len() as u32,
            poker_played: self.poker.played(),
            poker_won: self.poker.won(),
            poker_takes: self.poker.takes(),
            poker_together: self.poker.together(),
            underworld_held: !self.underworld_out(),
        }
    }

    /// Earns the trophies the program now has the facts for, and gives the Platinum skin once the
    /// Platinum trophy is earned. Returns the trophies that are new (the caller says so and saves).
    /// `now` is the time, in seconds since 1970.
    pub fn sync_trophies(&mut self, now: u64) -> Vec<&'static crate::trophies::Trophy> {
        let facts = self.trophy_facts();
        let new = self.trophies.sync(&facts, now);
        if self.trophies.has(crate::trophies::PLATINUM) {
            self.shop.grant("platinum");
        }
        new
    }

    /// Testing: gives a trophy by hand (and the Platinum skin with the Platinum trophy). Returns it
    /// when it was new.
    pub fn give_trophy(&mut self, id: &str, now: u64) -> Option<&'static crate::trophies::Trophy> {
        let new = self.trophies.give(id, now);
        if self.trophies.has(crate::trophies::PLATINUM) {
            self.shop.grant("platinum");
        }
        new
    }

    /// Testing: takes a trophy by hand (Platinum goes with it, and so does the Platinum skin).
    pub fn take_trophy(&mut self, id: &str) {
        self.trophies.take(id);
        if !self.trophies.has(crate::trophies::PLATINUM) {
            self.shop.revoke("platinum");
        }
    }

    /// What winning Game 1 does: the Gold Edition is unlocked and put on, and the backdrop becomes the
    /// Angel cats, to show it off. (Father Time was the way in, and stood where the cat king goes; to
    /// find him again you pick him in Options > Backdrop style.)
    pub fn unlock_gold_by_winning(&mut self) {
        self.gold_unlocked = true;
        self.gold_on = true;
        self.backdrop = true;
        self.backdrop_style = crate::info::BackdropStyle::AngelCats;
    }

    /// What winning Game 2 does: The Void is unlocked and put on (Gold is switched off, it comes back
    /// from Options), and the backdrop becomes the glitching stars, to show it off.
    pub fn unlock_void_by_winning(&mut self) {
        self.void_unlocked = true;
        self.void_on = true;
        if self.void_seed == 0 {
            self.void_seed = fresh_seed();
        }
        self.gold_on = false;
        self.backdrop = true;
        self.backdrop_style = crate::info::BackdropStyle::VoidStars;
    }

    /// Takes The Void away again. A backdrop that came with it goes back to the default.
    pub fn lock_void(&mut self) {
        self.void_unlocked = false;
        // unlocked again later, the hidden sentence turns up somewhere else and the staff is whole
        self.void_seed = 0;
        self.void_staff_broken = false;
        self.backdrop_style = self.shown_backdrop_style();
        // Gold, if it was earned, comes back in its place
        if self.gold_unlocked {
            self.gold_on = true;
        }
    }

    /// Takes the Gold Edition away again. A backdrop that came with it goes back to the default.
    pub fn lock_gold(&mut self) {
        self.gold_unlocked = false;
        self.backdrop_style = self.shown_backdrop_style();
    }

    /// The Trophies tab can be switched on once a trophy has been earned.
    pub fn trophies_tab_available(&self) -> bool {
        crate::release::story() && self.trophies.count() > 0
    }

    /// Whether the Trophies tab is in the top row.
    pub fn trophies_tab_shown(&self) -> bool {
        crate::release::story() && self.trophies_tab && self.trophies_tab_available()
    }

    /// Whether the pop-up about the Trophies tab is still to be shown.
    pub fn trophies_notice_due(&self) -> bool {
        crate::release::story() && self.trophies_tab_available() && !self.trophies_tab_told
    }

    pub fn tab_shown(&self, key: &str) -> bool {
        !self.hidden_tabs.iter().any(|k| k == key)
    }

    /// Shows or hides the tab with this key. Only keys from `TOGGLE_TABS` are accepted.
    pub fn set_tab_shown(&mut self, key: &str, shown: bool) {
        if !TOGGLE_TABS.iter().any(|(k, _)| *k == key) {
            return;
        }
        self.hidden_tabs.retain(|k| k != key);
        if !shown {
            self.hidden_tabs.push(key.to_string());
            // keep the file tidy: always in the same order
            self.hidden_tabs.sort_by_key(|k| TOGGLE_TABS.iter().position(|(t, _)| t == k));
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            accent: [0, 120, 60],
            always_on_top: false,
            close_to_tray: false,
            backdrop: false,
            start_minimized: false,
            backdrop_style: crate::info::BackdropStyle::default(),
            backdrop_animated: true,
            gold_unlocked: false,
            games: crate::games::Progress::default(),
            shop: crate::shop::Wallet::default(),
            poker: crate::poker_save::PokerSave::default(),
            trophies: crate::trophies::Book::default(),
            shop_preview: None,
            void_unlocked: false,
            void_on: true,
            void_banner: true,
            platinum_banner: true,
            platinum_effects: true,
            whispers_heard: 0,
            whispers_spoken: 0,
            void_script: true,
            void_seed: 0,
            void_staff_broken: false,
            void_pretend_night: false,
            dev_invincible: false,
            dev_show_underworld: false,
            underworld_on: false,
            gold_on: true,
            gold_banner: true,
            gold_sparkles: true,
            gold_glitch: true,
            hidden_tabs: Vec::new(),
            trophies_tab: false,
            trophies_tab_told: false,
            run_as_admin: false,
            ac_mode: None,
            battery_mode: None,
            ac_overclock: false,
            custom_cpu: None,
            custom_gpu: None,
            ac_cpu: None,
            ac_gpu: None,
            battery_cpu: None,
            battery_gpu: None,
            ac_last: false,
            battery_last: false,
            ac_last_state: None,
            battery_last_state: None,
            synapse_keep_lighting: true,
            font: "Bahnschrift".to_string(),
            saved_colors: Vec::new(),
            battery_limit: 80,
            pad_fan: String::new(),
            pad_rpm: 1500,
            pad_color: [255, 0, 0],
            pad_brightness: 100,
            pad_presets: [30, 60, 100],
            pad_control: false,
            sensor_enabled: [false; crate::sensor_ext::SOURCE_COUNT],
            sensor_picks: [crate::sensor_ext::AUTO; crate::sensor_ext::READING_COUNT],
            sensor_logging: false,
            sensor_read: false,
            sensor_log_name: String::new(),
            sensor_log_name_presets: Vec::new(),
            sensor_log_stamp: crate::sensor_log::Stamp::Date,
            sensor_log_start: crate::log_plan::Start::Now,
            sensor_log_program: String::new(),
            sensor_log_limit_secs: 0,
            sensor_log_stop_with_program: true,
            graph_window: 1,
            sensor_graphs: true,
            sensor_interval_ms: crate::sensors::DEFAULT_INTERVAL_MS,
            sensor_lhm_addr: crate::sensor_ext::DEFAULT_LHM_ADDR.to_string(),
            sensor_log_path: String::new(),
            log_view_options: String::new(),
            sensor_module_path: String::new(),
            pad_profiles: profiles_from_percents([30, 60, 100], true),
            pad_lighting: false,
            pad_color2: [255, 0, 255],
            pad_two: false,
            pad_split: 0,
            pad_breath_split: 0,
            pad_breath_speed: 100,
            kbd_lighting: false,
            kbd_color: [0, 200, 255],
            kbd_color2: [255, 0, 255],
            fire_hot: [255, 110, 0],
            fire_cold: [255, 0, 0],
            pad_fire_hot: [255, 110, 0],
            pad_fire_cold: [255, 0, 0],
            tidal_a: [0, 255, 0],
            tidal_b: [0, 0, 255],
            tidal_in: false,
            pad_tidal_a: [0, 255, 0],
            pad_tidal_b: [0, 0, 255],
            pad_tidal_in: false,
            starlight_a: [0, 255, 0],
            starlight_b: [0, 0, 255],
            starlight_two: false,
            pad_starlight_a: [0, 255, 0],
            pad_starlight_b: [0, 0, 255],
            pad_starlight_two: false,
            dipp_a: librazer::dipp::DEFAULT_A,
            dipp_b: librazer::dipp::DEFAULT_B,
            pad_dipp_a: librazer::dipp::DEFAULT_A,
            pad_dipp_b: librazer::dipp::DEFAULT_B,
            fire_speed: 100,
            pad_fire_speed: 100,
            tidal_speed: 100,
            pad_tidal_speed: 100,
            starlight_speed: 100,
            pad_starlight_speed: 100,
            spectrum_speed: 100,
            pad_spectrum_speed: 100,
            dipp_speed: 100,
            pad_dipp_speed: 100,
            rainbow_speed: 100,
            pad_rainbow_speed: 100,
            kbd_two: false,
            kbd_split: 0,
            kbd_breath_split: 0,
            kbd_breath_speed: 100,
            light_sync: false,
            open_tabs: Vec::new(),
            kbd_custom: KeyLayout::default(),
            kbd_brush: [255, 255, 255],
            kbd_brush_fx: 0,
            kbd_brush2: [0, 120, 255],
            kbd_brush_two: false,
            wave_custom: false,
            wave_a: librazer::wave::DEFAULT_A,
            wave_b: librazer::wave::DEFAULT_B,
            wave_speed: 100,
            wheel_custom: false,
            wheel_a: librazer::wheel::DEFAULT_A,
            wheel_b: librazer::wheel::DEFAULT_B,
            wheel_ccw: false,
            wheel_speed: 100,
            pad_wheel_speed: 100,
            pad_wheel_ccw: false,
            pad_wheel_custom: false,
            pad_wheel_a: librazer::wheel::DEFAULT_A,
            pad_wheel_b: librazer::wheel::DEFAULT_B,
            kbd_key_shift: 0,
            kbd_legacy: false,
            kbd_random_id: false,
            kbd_clear_first: true,
            kbd_static_frame: true,
            light_resume: false,
            game_preset_auto: true,
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

/// Time spans the Sensors graphs can show: label and seconds.
pub const GRAPH_WINDOWS: [(&str, u32); 5] =
    [("1 min", 60), ("5 min", 300), ("15 min", 900), ("30 min", 1800), ("1 hour", 3600)];

/// `1,4` = sources 1 and 4 are on.
pub fn encode_enabled(enabled: &[bool; crate::sensor_ext::SOURCE_COUNT]) -> String {
    let on: Vec<String> = enabled
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, e)| **e)
        .map(|(i, _)| i.to_string())
        .collect();
    on.join(",")
}

pub fn decode_enabled(text: &str) -> [bool; crate::sensor_ext::SOURCE_COUNT] {
    let mut enabled = [false; crate::sensor_ext::SOURCE_COUNT];
    for part in text.split(',') {
        if let Ok(i) = part.trim().parse::<usize>() {
            if (1..crate::sensor_ext::SOURCE_COUNT).contains(&i) {
                enabled[i] = true;
            }
        }
    }
    enabled
}

/// `cpu_temp:4,pl1:auto,...`
pub fn encode_picks(picks: &[u8; crate::sensor_ext::READING_COUNT]) -> String {
    let parts: Vec<String> = crate::sensor_ext::Reading::ALL
        .iter()
        .map(|r| {
            let pick = picks[r.index()];
            if pick == crate::sensor_ext::AUTO {
                format!("{}:auto", r.key())
            } else {
                format!("{}:{}", r.key(), pick)
            }
        })
        .collect();
    parts.join(",")
}

pub fn decode_picks(text: &str) -> [u8; crate::sensor_ext::READING_COUNT] {
    let mut picks = [crate::sensor_ext::AUTO; crate::sensor_ext::READING_COUNT];
    for part in text.split(',') {
        let Some((key, value)) = part.split_once(':') else {
            continue;
        };
        let Some(reading) = crate::sensor_ext::Reading::from_key(key.trim()) else {
            continue;
        };
        if let Ok(source) = value.trim().parse::<u8>() {
            if usize::from(source) < crate::sensor_ext::SOURCE_COUNT {
                picks[reading.index()] = source;
            }
        }
    }
    picks
}

impl Config {
    /// The sensor sources the Sensors tab should read from.
    pub fn sensor_source_config(&self) -> crate::sensor_ext::SourceConfig {
        crate::sensor_ext::SourceConfig {
            enabled: self.sensor_enabled,
            picks: self.sensor_picks,
            lhm_addr: self.sensor_lhm_addr.clone(),
            log_path: self.sensor_log_path.clone(),
            module_path: self.sensor_module_path.clone(),
        }
    }
}

impl Config {
    /// When the CSV log starts and stops.
    pub fn log_plan(&self) -> crate::log_plan::Plan {
        crate::log_plan::Plan {
            start: self.sensor_log_start,
            program: crate::log_plan::clean_program(&self.sensor_log_program),
            limit_secs: self.sensor_log_limit_secs,
            stop_with_program: self.sensor_log_stop_with_program,
            stamp: self.sensor_log_stamp,
        }
    }
}

pub fn load() -> Config {
    match std::fs::read_to_string(config_path()) {
        Ok(text) => parse_text(&text),
        Err(_) => Config::default(),
    }
}

/// Reads the text of a config file: missing keys keep their defaults, unknown ones are skipped.
pub fn parse_text(text: &str) -> Config {
    let mut cfg = Config::default();
    // Older config files have no fan profiles: their three fan presets become profiles.
    let mut profiles_seen = false;
    let mut legacy_source: Option<usize> = None;
    let mut enable_seen = false;
    let mut log_armed = false;
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
            "open_tabs" => {
                cfg.open_tabs = value
                    .split(',')
                    .filter_map(|item| {
                        let (key, open) = item.rsplit_once(':')?;
                        let key = key.trim();
                        (!key.is_empty()).then(|| (key.to_string(), open.trim() == "1"))
                    })
                    .collect();
            }
            "pad_control" => cfg.pad_control = value == "true" || value == "1",
            "sensor_source" => {
                // older versions: one external source for everything
                legacy_source = value.parse::<usize>().ok();
            }
            "sensor_enable" => {
                enable_seen = true;
                cfg.sensor_enabled = decode_enabled(value);
            }
            "sensor_picks" => cfg.sensor_picks = decode_picks(value),
            // (an older "sensor_logging" key is ignored on purpose: logging starts off)
            "sensor_log_name" => cfg.sensor_log_name = value.to_string(),
            "sensor_log_stamp" => cfg.sensor_log_stamp = crate::sensor_log::Stamp::from_key(value),
            "sensor_log_name_presets" => cfg.sensor_log_name_presets = crate::log_plan::decode_name_presets(value),
            "sensor_log_start" => cfg.sensor_log_start = crate::log_plan::Start::from_key(value),
            "sensor_log_program" => cfg.sensor_log_program = value.to_string(),
            "sensor_log_limit_secs" => {
                cfg.sensor_log_limit_secs = value.parse::<u64>().unwrap_or(0).min(crate::log_plan::MAX_LIMIT_SECS)
            }
            "sensor_log_stop_with_program" => cfg.sensor_log_stop_with_program = value != "false" && value != "0",
            "sensor_log_armed" => log_armed = value == "true" || value == "1",
            "sensor_read" => cfg.sensor_read = value == "true" || value == "1",
            "graph_window" => {
                cfg.graph_window = value
                    .parse::<u8>()
                    .ok()
                    .filter(|v| usize::from(*v) < GRAPH_WINDOWS.len())
                    .unwrap_or(1);
            }
            "sensor_graphs" => cfg.sensor_graphs = value != "false" && value != "0",
            "sensor_interval_ms" => {
                cfg.sensor_interval_ms = value
                    .parse::<u64>()
                    .map(crate::sensors::clamp_interval)
                    .unwrap_or(crate::sensors::DEFAULT_INTERVAL_MS);
            }
            "sensor_lhm_addr" => {
                if !value.is_empty() {
                    cfg.sensor_lhm_addr = value.to_string();
                }
            }
            "sensor_log_path" => cfg.sensor_log_path = value.to_string(),
            "logview_options" => cfg.log_view_options = value.to_string(),
            "sensor_module_path" => cfg.sensor_module_path = value.to_string(),
            "pad_profiles_saved" => {
                if !profiles_seen {
                    profiles_seen = true;
                    cfg.pad_profiles.clear();
                }
            }
            k if k.starts_with("pad_profile_") => {
                if let Some(profile) = decode_profile(value) {
                    if cfg.pad_profiles.len() < PAD_PROFILES_MAX {
                        cfg.pad_profiles.push(profile);
                    }
                }
            }
            "accent" => {
                let parts: Vec<u8> =
                    value.split(',').filter_map(|p| p.trim().parse::<u8>().ok()).collect();
                if parts.len() == 3 {
                    cfg.accent = [parts[0], parts[1], parts[2]];
                }
            }
            "always_on_top" => cfg.always_on_top = value == "true",
            "close_to_tray" => cfg.close_to_tray = value == "true" || value == "1",
            "run_as_admin" => cfg.run_as_admin = value == "true" || value == "1",
            "backdrop" => cfg.backdrop = value != "false" && value != "0",
            "backdrop_animated" => cfg.backdrop_animated = value != "false" && value != "0",
            "gold_unlocked" => cfg.gold_unlocked = value == "true" || value == "1",
            "gold_on" => cfg.gold_on = value != "false" && value != "0",
            "gold_banner" => cfg.gold_banner = value != "false" && value != "0",
            "gold_glitch" => cfg.gold_glitch = value != "false" && value != "0",
            "gold_sparkles" => cfg.gold_sparkles = value != "false" && value != "0",
            "void_unlocked" => cfg.void_unlocked = value == "true" || value == "1",
            "void_on" => cfg.void_on = value != "false" && value != "0",
            "void_banner" => cfg.void_banner = value != "false" && value != "0",
            "platinum_banner" => cfg.platinum_banner = value != "false" && value != "0",
            "platinum_effects" => cfg.platinum_effects = value != "false" && value != "0",
            "whispers_heard" => cfg.whispers_heard = value.trim().parse::<u32>().unwrap_or(0),
            "whispers_spoken" => cfg.whispers_spoken = value.trim().parse::<u32>().unwrap_or(0),
            "void_script" => cfg.void_script = value != "false" && value != "0",
            "void_seed" => cfg.void_seed = value.trim().parse().unwrap_or(0),
            "void_staff_broken" => cfg.void_staff_broken = value == "true" || value == "1",
            "void_pretend_night" => cfg.void_pretend_night = value == "true" || value == "1",
            "dev_show_underworld" => cfg.dev_show_underworld = value == "true" || value == "1",
            "underworld_on" => cfg.underworld_on = value == "true" || value == "1",
            // (every line the game progress writes has to be listed, or it is saved but never loaded)
            "game_deaths" | "game_shown" | "game_difficulty" | "game_cheats" | "game3_stage" | "game3_latency"
            | "game3_volume" | "game3_underworld" | "game3_playlist" | "game1_checkpoint" | "cat_helped" | "cat_declines" | "ally" => {
                cfg.games.read_line(key, value);
            }
            "shop_balance" | "shop_lifetime" | "shop_owned" | "shop_skin" | "shop_started" | "shop_first_wins"
            | "shop_revealed" => {
                cfg.shop.read_line(key, value);
            }
            "poker_lock_until" | "poker_soul_takes" | "poker_free_notice" | "poker_played" | "poker_won"
            | "poker_intro_seen" | "poker_auto_deal" | "poker_skip_dealing" | "poker_quick_table" | "poker_together" => {
                cfg.poker.read_line(key, value);
            }
            "trophies" | "trophies_manual" => {
                cfg.trophies.read_line(key, value);
            }
            "start_minimized" => cfg.start_minimized = value == "true" || value == "1",
            "backdrop_style" => cfg.backdrop_style = crate::info::BackdropStyle::from_key(value),
            "hidden_tabs" => {
                cfg.hidden_tabs = value
                    .split(',')
                    .map(|k| k.trim().to_string())
                    .filter(|k| TOGGLE_TABS.iter().any(|(key, _)| key == k))
                    .collect();
            }
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
            "custom_cpu" => cfg.custom_cpu = crate::power_plan::clean_boost_name(value),
            "custom_gpu" => cfg.custom_gpu = crate::power_plan::clean_boost_name(value),
            "ac_cpu" => cfg.ac_cpu = crate::power_plan::clean_boost_name(value),
            "ac_gpu" => cfg.ac_gpu = crate::power_plan::clean_boost_name(value),
            "battery_cpu" => cfg.battery_cpu = crate::power_plan::clean_boost_name(value),
            "battery_gpu" => cfg.battery_gpu = crate::power_plan::clean_boost_name(value),
            "trophies_tab" => cfg.trophies_tab = value == "true",
            "trophies_tab_told" => cfg.trophies_tab_told = value == "true",
            "ac_last" => cfg.ac_last = value == "true",
            "battery_last" => cfg.battery_last = value == "true",
            "ac_last_state" => {
                cfg.ac_last_state = crate::power_plan::decode_last(value).map(|_| value.trim().to_string())
            }
            "battery_last_state" => {
                cfg.battery_last_state = crate::power_plan::decode_last(value).map(|_| value.trim().to_string())
            }
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
            "game_preset_auto" => cfg.game_preset_auto = value == "true" || value == "1",
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
            "kbd_two" => cfg.kbd_two = value == "true" || value == "1",
            "pad_two" => cfg.pad_two = value == "true" || value == "1",
            "kbd_split" => cfg.kbd_split = value.parse::<u8>().unwrap_or(0).min(2),
            "kbd_breath_split" => cfg.kbd_breath_split = value.parse::<u8>().unwrap_or(0).min(3),
            "light_sync" => cfg.light_sync = value == "true" || value == "1",
            // (before the sliders these were a choice of four, 0 to 3: speed_from_saved converts)
            "kbd_breath_speed" => {
                cfg.kbd_breath_speed =
                    librazer::command::speed_from_saved(value.parse::<u8>().unwrap_or(0))
            }
            "pad_breath_speed" => {
                cfg.pad_breath_speed =
                    librazer::command::speed_from_saved(value.parse::<u8>().unwrap_or(0))
            }
            "fire_speed" | "pad_fire_speed" | "tidal_speed" | "pad_tidal_speed" | "starlight_speed"
            | "pad_starlight_speed" | "spectrum_speed" | "pad_spectrum_speed" | "dipp_speed"
            | "pad_dipp_speed" | "rainbow_speed" | "pad_rainbow_speed" => {
                let speed = librazer::command::speed_from_saved(
                    value.parse::<u8>().unwrap_or(librazer::command::STANDARD_SPEED),
                );
                match key {
                    "fire_speed" => cfg.fire_speed = speed,
                    "pad_fire_speed" => cfg.pad_fire_speed = speed,
                    "tidal_speed" => cfg.tidal_speed = speed,
                    "pad_tidal_speed" => cfg.pad_tidal_speed = speed,
                    "starlight_speed" => cfg.starlight_speed = speed,
                    "pad_starlight_speed" => cfg.pad_starlight_speed = speed,
                    "spectrum_speed" => cfg.spectrum_speed = speed,
                    "pad_spectrum_speed" => cfg.pad_spectrum_speed = speed,
                    "dipp_speed" => cfg.dipp_speed = speed,
                    "rainbow_speed" => cfg.rainbow_speed = speed,
                    "pad_rainbow_speed" => cfg.pad_rainbow_speed = speed,
                    _ => cfg.pad_dipp_speed = speed,
                }
            }
            "pad_breath_split" => cfg.pad_breath_split = value.parse::<u8>().unwrap_or(0).min(2),
            "kbd_custom" => {
                if let Some(layout) = KeyLayout::decode(value) {
                    cfg.kbd_custom = layout;
                }
            }
            "kbd_brush" => {
                let parts: Vec<u8> =
                    value.split(',').filter_map(|p| p.trim().parse::<u8>().ok()).collect();
                if parts.len() == 3 {
                    cfg.kbd_brush = [parts[0], parts[1], parts[2]];
                }
            }
            "kbd_brush2" => {
                let parts: Vec<u8> =
                    value.split(',').filter_map(|p| p.trim().parse::<u8>().ok()).collect();
                if parts.len() == 3 {
                    cfg.kbd_brush2 = [parts[0], parts[1], parts[2]];
                }
            }
            "kbd_brush_two" => cfg.kbd_brush_two = value == "true" || value == "1",
            "wave_custom" => cfg.wave_custom = value == "true",
            "wheel_ccw" => cfg.wheel_ccw = value == "true",
            "wheel_custom" => cfg.wheel_custom = value == "true",
            "pad_wheel_ccw" => cfg.pad_wheel_ccw = value == "true",
            "pad_wheel_custom" => cfg.pad_wheel_custom = value == "true",
            "wave_a" | "wave_b" | "wheel_a" | "wheel_b" | "pad_wheel_a" | "pad_wheel_b" => {
                let parts: Vec<u8> =
                    value.split(',').filter_map(|p| p.trim().parse::<u8>().ok()).collect();
                if parts.len() == 3 {
                    let rgb = [parts[0], parts[1], parts[2]];
                    match key {
                        "wave_a" => cfg.wave_a = rgb,
                        "wave_b" => cfg.wave_b = rgb,
                        "wheel_a" => cfg.wheel_a = rgb,
                        "wheel_b" => cfg.wheel_b = rgb,
                        "pad_wheel_a" => cfg.pad_wheel_a = rgb,
                        _ => cfg.pad_wheel_b = rgb,
                    }
                }
            }
            "wave_speed" | "wheel_speed" | "pad_wheel_speed" => {
                let speed = librazer::command::speed_from_saved(
                    value.parse::<u8>().unwrap_or(librazer::command::STANDARD_SPEED),
                );
                match key {
                    "wave_speed" => cfg.wave_speed = speed,
                    "wheel_speed" => cfg.wheel_speed = speed,
                    _ => cfg.pad_wheel_speed = speed,
                }
            }
            "kbd_brush_fx" => {
                cfg.kbd_brush_fx = value.parse::<u8>().unwrap_or(0).min(librazer::keymap::KeyFx::ALL.len() as u8 - 1)
            }
            "kbd_key_shift" => cfg.kbd_key_shift = value.parse::<i8>().unwrap_or(0).clamp(-3, 3),
            "pad_split" => cfg.pad_split = value.parse::<u8>().unwrap_or(0).min(1),
            "fire_hot" | "fire_cold" | "pad_fire_hot" | "pad_fire_cold" | "tidal_a" | "tidal_b"
            | "pad_tidal_a" | "pad_tidal_b" | "starlight_a" | "starlight_b" | "pad_starlight_a"
            | "pad_starlight_b" | "dipp_a" | "dipp_b" | "pad_dipp_a" | "pad_dipp_b" => {
                let parts: Vec<u8> =
                    value.split(',').filter_map(|p| p.trim().parse::<u8>().ok()).collect();
                if parts.len() == 3 {
                    let rgb = [parts[0], parts[1], parts[2]];
                    match key {
                        "fire_hot" => cfg.fire_hot = rgb,
                        "fire_cold" => cfg.fire_cold = rgb,
                        "pad_fire_hot" => cfg.pad_fire_hot = rgb,
                        "pad_fire_cold" => cfg.pad_fire_cold = rgb,
                        "tidal_a" => cfg.tidal_a = rgb,
                        "pad_tidal_a" => cfg.pad_tidal_a = rgb,
                        "pad_tidal_b" => cfg.pad_tidal_b = rgb,
                        "starlight_a" => cfg.starlight_a = rgb,
                        "starlight_b" => cfg.starlight_b = rgb,
                        "pad_starlight_a" => cfg.pad_starlight_a = rgb,
                        "pad_starlight_b" => cfg.pad_starlight_b = rgb,
                        "dipp_a" => cfg.dipp_a = rgb,
                        "dipp_b" => cfg.dipp_b = rgb,
                        "pad_dipp_a" => cfg.pad_dipp_a = rgb,
                        "pad_dipp_b" => cfg.pad_dipp_b = rgb,
                        _ => cfg.tidal_b = rgb,
                    }
                }
            }
            "tidal_in" => cfg.tidal_in = value == "true",
            "pad_tidal_in" => cfg.pad_tidal_in = value == "true",
            "starlight_two" => cfg.starlight_two = value == "true",
            "pad_starlight_two" => cfg.pad_starlight_two = value == "true",
            "kbd_color2" => {
                let parts: Vec<u8> =
                    value.split(',').filter_map(|p| p.trim().parse::<u8>().ok()).collect();
                if parts.len() == 3 {
                    cfg.kbd_color2 = [parts[0], parts[1], parts[2]];
                }
            }
            "pad_color2" => {
                let parts: Vec<u8> =
                    value.split(',').filter_map(|p| p.trim().parse::<u8>().ok()).collect();
                if parts.len() == 3 {
                    cfg.pad_color2 = [parts[0], parts[1], parts[2]];
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
    if let Some(source) = legacy_source.filter(|s| (1..crate::sensor_ext::SOURCE_COUNT).contains(s))
    {
        if !enable_seen {
            cfg.sensor_enabled[source] = true;
        }
    }
    if !profiles_seen && cfg.pad_presets != [30, 60, 100] {
        cfg.pad_profiles = profiles_from_percents(cfg.pad_presets, false);
    }
    // a Void that was unlocked before the seed existed gets one now
    if cfg.void_unlocked && cfg.void_seed == 0 {
        cfg.void_seed = fresh_seed();
    }
    // waiting for a program is remembered (nothing is written until it starts); a log that starts at
    // once never is, so nothing is written after a start unless it is switched on again
    cfg.sensor_logging = log_armed && cfg.sensor_log_start == crate::log_plan::Start::Program;
    // "Overclock on AC power" was Custom with the CPU on Overclock: now that is just what the AC drop-downs say
    if cfg.ac_overclock {
        cfg.ac_mode = Some("Custom".to_string());
        if cfg.ac_cpu.is_none() {
            cfg.ac_cpu = Some("Overclock".to_string());
        }
        cfg.ac_overclock = false;
    }
    cfg
}

/// A number that is different every time it is asked for (never 0, which means "not picked").
fn fresh_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    nanos.max(1)
}

pub fn save(cfg: &Config) {
    let dir = config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(config_path(), to_text(cfg));
}

/// The text of the config file for these settings.
pub fn to_text(cfg: &Config) -> String {
    let text = format!(
        "# CPUnity:R-Helper settings\naccent={},{},{}\nalways_on_top={}\nac_mode={}\nbattery_mode={}\nac_overclock={}\nsynapse_keep_lighting={}\nfont={}\nsaved_colors={}\nbattery_limit={}\npad_fan={}\npad_rpm={}\npad_color={},{},{}\npad_brightness={}\npad_presets={},{},{}\npad_lighting={}\nkbd_lighting={}\nkbd_color={},{},{}\nkbd_color2={},{},{}\nkbd_two={}\nkbd_split={}\npad_color2={},{},{}\npad_two={}\npad_split={}\nkbd_legacy={}\nkbd_random_id={}\nkbd_clear_first={}\nkbd_static_frame={}\nlight_resume={}\nkbd_last_effect={}\npad_last_mode={}\nkbd_breath_split={}\npad_breath_split={}\nkbd_custom={}\nkbd_brush={},{},{}\nkbd_key_shift={}\nkbd_breath_speed={}\npad_breath_speed={}\nlight_sync={}\n",
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
        cfg.kbd_color2[0],
        cfg.kbd_color2[1],
        cfg.kbd_color2[2],
        cfg.kbd_two,
        cfg.kbd_split,
        cfg.pad_color2[0],
        cfg.pad_color2[1],
        cfg.pad_color2[2],
        cfg.pad_two,
        cfg.pad_split,
        cfg.kbd_legacy,
        cfg.kbd_random_id,
        cfg.kbd_clear_first,
        cfg.kbd_static_frame,
        cfg.light_resume,
        cfg.kbd_last_effect,
        cfg.pad_last_mode,
        cfg.kbd_breath_split,
        cfg.pad_breath_split,
        cfg.kbd_custom.encode(),
        cfg.kbd_brush[0],
        cfg.kbd_brush[1],
        cfg.kbd_brush[2],
        cfg.kbd_key_shift,
        cfg.kbd_breath_speed,
        cfg.pad_breath_speed,
        cfg.light_sync,
    );
    let mut text = text;
    text.push_str(&format!("pad_control={}\npad_profiles_saved=1\n", cfg.pad_control));
    text.push_str(&format!(
        "trophies_tab={}\ntrophies_tab_told={}\ncustom_cpu={}\ncustom_gpu={}\nac_cpu={}\nac_gpu={}\nbattery_cpu={}\nbattery_gpu={}\nac_last={}\nbattery_last={}\nac_last_state={}\nbattery_last_state={}\n",
        cfg.trophies_tab,
        cfg.trophies_tab_told,
        cfg.custom_cpu.clone().unwrap_or_default(),
        cfg.custom_gpu.clone().unwrap_or_default(),
        cfg.ac_cpu.clone().unwrap_or_default(),
        cfg.ac_gpu.clone().unwrap_or_default(),
        cfg.battery_cpu.clone().unwrap_or_default(),
        cfg.battery_gpu.clone().unwrap_or_default(),
        cfg.ac_last,
        cfg.battery_last,
        cfg.ac_last_state.clone().unwrap_or_default(),
        cfg.battery_last_state.clone().unwrap_or_default()
    ));
    text.push_str(&format!(
        "sensor_enable={}\nsensor_picks={}\nsensor_read={}\nsensor_log_name={}\nsensor_log_start={}\nsensor_log_program={}\nsensor_log_limit_secs={}\nsensor_log_stop_with_program={}\nsensor_log_armed={}\ngraph_window={}\nsensor_lhm_addr={}\nsensor_log_path={}\nlogview_options={}\nsensor_module_path={}\nclose_to_tray={}\nrun_as_admin={}\nsensor_graphs={}\nsensor_interval_ms={}\nbackdrop={}\nbackdrop_style={}\nhidden_tabs={}\nstart_minimized={}\nbackdrop_animated={}\ngold_unlocked={}\ngold_on={}\ngold_banner={}\ngold_sparkles={}\ngold_glitch={}\n",
        encode_enabled(&cfg.sensor_enabled),
        encode_picks(&cfg.sensor_picks),
        cfg.sensor_read,
        cfg.sensor_log_name.replace(['\r', '\n'], ""),
        cfg.sensor_log_start.key(),
        cfg.sensor_log_program.replace(['\r', '\n'], ""),
        cfg.sensor_log_limit_secs,
        cfg.sensor_log_stop_with_program,
        cfg.sensor_logging && cfg.sensor_log_start == crate::log_plan::Start::Program,
        cfg.graph_window,
        cfg.sensor_lhm_addr.replace(['\r', '\n'], ""),
        cfg.sensor_log_path.replace(['\r', '\n'], ""),
        cfg.log_view_options.replace(['\r', '\n'], ""),
        cfg.sensor_module_path.replace(['\r', '\n'], ""),
        cfg.close_to_tray,
        cfg.run_as_admin,
        cfg.sensor_graphs,
        cfg.sensor_interval_ms,
        cfg.backdrop,
        cfg.backdrop_style.key(),
        cfg.hidden_tabs.join(","),
        cfg.start_minimized,
        cfg.backdrop_animated,
        cfg.gold_unlocked,
        cfg.gold_on,
        cfg.gold_banner,
        cfg.gold_sparkles,
        cfg.gold_glitch
    ));
    for (name, rgb) in [
        ("fire_hot", cfg.fire_hot),
        ("fire_cold", cfg.fire_cold),
        ("pad_fire_hot", cfg.pad_fire_hot),
        ("pad_fire_cold", cfg.pad_fire_cold),
        ("tidal_a", cfg.tidal_a),
        ("tidal_b", cfg.tidal_b),
        ("pad_tidal_a", cfg.pad_tidal_a),
        ("pad_tidal_b", cfg.pad_tidal_b),
        ("starlight_a", cfg.starlight_a),
        ("starlight_b", cfg.starlight_b),
        ("pad_starlight_a", cfg.pad_starlight_a),
        ("pad_starlight_b", cfg.pad_starlight_b),
        ("dipp_a", cfg.dipp_a),
        ("dipp_b", cfg.dipp_b),
        ("pad_dipp_a", cfg.pad_dipp_a),
        ("pad_dipp_b", cfg.pad_dipp_b),
    ] {
        text.push_str(&format!("{}={},{},{}\n", name, rgb[0], rgb[1], rgb[2]));
    }
    text.push_str(&format!(
        "tidal_in={}\npad_tidal_in={}\nstarlight_two={}\npad_starlight_two={}\nkbd_brush_fx={}\ngame_preset_auto={}\nfire_speed={}\npad_fire_speed={}\ntidal_speed={}\npad_tidal_speed={}\nstarlight_speed={}\npad_starlight_speed={}\nspectrum_speed={}\npad_spectrum_speed={}\ndipp_speed={}\npad_dipp_speed={}\nrainbow_speed={}\npad_rainbow_speed={}\n",
        cfg.tidal_in,
        cfg.pad_tidal_in,
        cfg.starlight_two,
        cfg.pad_starlight_two,
        cfg.kbd_brush_fx,
        cfg.game_preset_auto,
        cfg.fire_speed,
        cfg.pad_fire_speed,
        cfg.tidal_speed,
        cfg.pad_tidal_speed,
        cfg.starlight_speed,
        cfg.pad_starlight_speed,
        cfg.spectrum_speed,
        cfg.pad_spectrum_speed,
        cfg.dipp_speed,
        cfg.pad_dipp_speed,
        cfg.rainbow_speed,
        cfg.pad_rainbow_speed
    ));
    text.push_str(&cfg.games.to_text());
    text.push_str(&cfg.shop.to_text());
    text.push_str(&cfg.poker.to_text());
    text.push_str(&cfg.trophies.to_text());
    text.push_str(&format!("underworld_on={}\nplatinum_banner={}\nplatinum_effects={}\nwhispers_heard={}\nwhispers_spoken={}\nkbd_brush2={},{},{}\nkbd_brush_two={}\nwave_custom={}\nwave_a={},{},{}\nwave_b={},{},{}\nwave_speed={}\nwheel_a={},{},{}\nwheel_b={},{},{}\nwheel_ccw={}\nwheel_speed={}\nwheel_custom={}\npad_wheel_speed={}\npad_wheel_ccw={}\npad_wheel_custom={}\npad_wheel_a={},{},{}\npad_wheel_b={},{},{}\n", cfg.underworld_on, cfg.platinum_banner, cfg.platinum_effects, cfg.whispers_heard, cfg.whispers_spoken, cfg.kbd_brush2[0], cfg.kbd_brush2[1], cfg.kbd_brush2[2], cfg.kbd_brush_two, cfg.wave_custom, cfg.wave_a[0], cfg.wave_a[1], cfg.wave_a[2], cfg.wave_b[0], cfg.wave_b[1], cfg.wave_b[2], cfg.wave_speed, cfg.wheel_a[0], cfg.wheel_a[1], cfg.wheel_a[2], cfg.wheel_b[0], cfg.wheel_b[1], cfg.wheel_b[2], cfg.wheel_ccw, cfg.wheel_speed, cfg.wheel_custom, cfg.pad_wheel_speed, cfg.pad_wheel_ccw, cfg.pad_wheel_custom, cfg.pad_wheel_a[0], cfg.pad_wheel_a[1], cfg.pad_wheel_a[2], cfg.pad_wheel_b[0], cfg.pad_wheel_b[1], cfg.pad_wheel_b[2]));
    text.push_str(&format!("void_unlocked={}\nvoid_on={}\nvoid_banner={}\nvoid_script={}\nvoid_seed={}\nvoid_staff_broken={}\nvoid_pretend_night={}\n", cfg.void_unlocked, cfg.void_on, cfg.void_banner, cfg.void_script, cfg.void_seed, cfg.void_staff_broken, cfg.void_pretend_night));
    text.push_str(&format!("dev_show_underworld={}\n", cfg.dev_show_underworld as u8));
    text.push_str(&format!("sensor_log_name_presets={}\nsensor_log_stamp={}\n", crate::log_plan::encode_name_presets(&cfg.sensor_log_name_presets), cfg.sensor_log_stamp.key()));
    let open_tabs: Vec<String> =
        cfg.open_tabs.iter().map(|(key, open)| format!("{}:{}", key, u8::from(*open))).collect();
    text.push_str(&format!("open_tabs={}\n", open_tabs.join(",")));
    for (i, profile) in cfg.pad_profiles.iter().enumerate().take(PAD_PROFILES_MAX) {
        text.push_str(&format!("pad_profile_{}={}\n", i, encode_profile(profile)));
    }
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
    text
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
            kbd: Some(KbdPart {
                effect: "breathing".into(),
                color: [1, 2, 3],
                color2: Some([7, 8, 9]),
                split: 2,
                brightness: 120,
                layout: None,
            }),
            pad: Some(PadPart {
                mode: "Static".into(),
                color: [9, 8, 7],
                color2: None,
                split: 0,
                brightness: 40,
            }),
        };
        assert_eq!(decode_preset(&encode_preset(&p)), Some(p));
    }

    #[test]
    fn single_part_round_trip() {
        let p = LightPreset {
            name: "Pad only".into(),
            kbd: None,
            pad: Some(PadPart {
                mode: "Off".into(),
                color: [0, 0, 0],
                color2: None,
                split: 0,
                brightness: 0,
            }),
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
    fn custom_keys_preset_round_trip() {
        let mut layout = KeyLayout::default();
        layout.set_many(&["W", "A", "S", "D"], [255, 0, 0]);
        layout.set("SPACE", [0, 0, 255]);
        let p = LightPreset {
            name: "WASD".into(),
            kbd: Some(KbdPart {
                effect: "custom".into(),
                color: [255, 255, 255],
                color2: None,
                split: 0,
                brightness: 90,
                layout: Some(layout.encode()),
            }),
            pad: None,
        };
        let text = encode_preset(&p);
        assert!(text.starts_with("WASD|custom,255,255,255,90,0,0,0,0,"));
        assert_eq!(decode_preset(&text), Some(p));
    }

    #[test]
    fn older_presets_still_load() {
        // five fields (single color) and nine fields (two colors) as saved by earlier builds
        let p = decode_preset("Old|static,1,2,3,100|Static,4,5,6,50").unwrap();
        assert_eq!(p.kbd.as_ref().unwrap().layout, None);
        assert_eq!(p.kbd.as_ref().unwrap().color2, None);
        let q = decode_preset("Two|breathing,1,2,3,100,7,8,9,1|").unwrap();
        assert_eq!(q.kbd.as_ref().unwrap().color2, Some([7, 8, 9]));
        assert_eq!(q.kbd.as_ref().unwrap().split, 1);
        // a custom part with an unreadable layout is rejected, not half-loaded
        assert!(decode_preset("Bad|custom,1,2,3,4,0,0,0,0,zzz|").is_none());
    }

    #[test]
    fn profiles_round_trip_and_are_snapped() {
        let p = PadProfile { name: "Turbo".into(), rpm: 2850 };
        assert_eq!(encode_profile(&p), "Turbo|2850");
        assert_eq!(decode_profile("Turbo|2850"), Some(p));
        // speeds are rounded to the pad's step and kept in range
        assert_eq!(decode_profile("A|2871").unwrap().rpm, 2850);
        assert_eq!(decode_profile("A|10").unwrap().rpm, 500);
        assert_eq!(decode_profile("A|65000").unwrap().rpm, 3200);
        // an empty name gets a default, a missing speed is rejected, '|' inside a name is dropped
        assert_eq!(decode_profile("|1500").unwrap().name, "Profile");
        assert_eq!(decode_profile("a|b|1500").unwrap().name, "ab");
        assert!(decode_profile("Quiet").is_none());
        assert!(decode_profile("Quiet|fast").is_none());
    }

    #[test]
    fn fresh_config_has_three_starter_profiles() {
        let cfg = Config::default();
        assert!(!cfg.pad_control);
        let names: Vec<_> = cfg.pad_profiles.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Low", "Medium", "Max"]);
        assert_eq!(cfg.pad_profiles[2].rpm, 3200);
        assert!(cfg.pad_profiles.iter().all(|p| p.rpm % 50 == 0));
        // the old three fan presets (percent) become profiles named after the percentage
        let migrated = profiles_from_percents([25, 50, 90], false);
        assert_eq!(migrated[0].name, "25%");
        assert_eq!(migrated[1].rpm, 1600);
    }

    #[test]
    fn sensor_choices_round_trip() {
        use crate::sensor_ext::{Reading, AUTO, HWINFO, PAWN, SOURCE_COUNT};
        let mut enabled = [false; SOURCE_COUNT];
        enabled[HWINFO] = true;
        enabled[PAWN] = true;
        assert_eq!(encode_enabled(&enabled), "1,4");
        assert_eq!(decode_enabled("1,4"), enabled);
        assert_eq!(decode_enabled(""), [false; SOURCE_COUNT]);
        assert_eq!(decode_enabled("0,9,x,2"), {
            let mut e = [false; SOURCE_COUNT];
            e[2] = true;
            e
        });
        let mut picks = [AUTO; crate::sensor_ext::READING_COUNT];
        picks[Reading::CpuTemp.index()] = PAWN as u8;
        picks[Reading::GpuMemTemp.index()] = HWINFO as u8;
        let text = encode_picks(&picks);
        assert!(text.starts_with("cpu_temp:4,core_temp:auto,pl1:auto"));
        assert_eq!(decode_picks(&text), picks);
        // unknown names and out of range numbers are ignored
        assert_eq!(decode_picks("nope:1,pl1:99,pl2:x"), [AUTO; crate::sensor_ext::READING_COUNT]);
    }

    #[test]
    fn the_power_profiles_keep_their_boosts_and_last_known_settings() {
        let mut cfg = Config::default();
        assert!(!cfg.ac_last && !cfg.battery_last, "Last Known Settings starts off");
        cfg.ac_cpu = Some("Overclock".into());
        cfg.ac_gpu = Some("High".into());
        cfg.battery_cpu = Some("Low".into());
        cfg.battery_gpu = Some("Low".into());
        cfg.battery_last = true;
        cfg.battery_last_state = Some("Custom|Low|Medium".into());
        cfg.ac_last_state = Some("Balanced||".into());
        let loaded = round_trip(&cfg);
        assert_eq!((loaded.ac_cpu.as_deref(), loaded.ac_gpu.as_deref()), (Some("Overclock"), Some("High")));
        assert_eq!((loaded.battery_cpu.as_deref(), loaded.battery_gpu.as_deref()), (Some("Low"), Some("Low")));
        assert!(loaded.battery_last && !loaded.ac_last);
        assert_eq!(loaded.battery_last_state.as_deref(), Some("Custom|Low|Medium"));
        assert_eq!(loaded.ac_last_state.as_deref(), Some("Balanced||"));
        // junk is dropped
        let bad = parse_text("ac_last_state=???\nbattery_last_state=\n");
        assert_eq!((bad.ac_last_state, bad.battery_last_state), (None, None));
    }

    #[test]
    fn the_old_overclock_switch_becomes_custom_with_cpu_overclock() {
        let old = parse_text("ac_overclock=true\nac_gpu=High\n");
        assert_eq!(old.ac_mode.as_deref(), Some("Custom"));
        assert_eq!((old.ac_cpu.as_deref(), old.ac_gpu.as_deref()), (Some("Overclock"), Some("High")));
        assert!(!old.ac_overclock);
        let kept = parse_text("ac_overclock=true\nac_cpu=High\n");
        assert_eq!(kept.ac_cpu.as_deref(), Some("High"), "a CPU level that was picked is not overwritten");
    }

    #[test]
    fn the_trophies_tab_waits_for_a_trophy_and_is_told_once() {
        let mut cfg = Config::default();
        assert!(!cfg.trophies_tab && !cfg.trophies_tab_told, "off and untold by default");
        assert!(!cfg.trophies_tab_available() && !cfg.trophies_notice_due(), "nothing earned yet");
        cfg.trophies_tab = true; // even if somehow on, it is not shown without a trophy
        assert!(!cfg.trophies_tab_shown());
        cfg.trophies.give("game1_win", 100);
        assert!(cfg.trophies_tab_available() && cfg.trophies_notice_due());
        assert!(cfg.trophies_tab_shown());
        cfg.trophies_tab_told = true;
        assert!(!cfg.trophies_notice_due());
        let loaded = round_trip(&cfg);
        assert!(loaded.trophies_tab && loaded.trophies_tab_told);
        assert!(!round_trip(&Config::default()).trophies_tab);
    }

    #[test]
    fn the_custom_boosts_are_remembered() {
        let mut cfg = Config::default();
        assert_eq!((cfg.custom_cpu.clone(), cfg.custom_gpu.clone()), (None, None));
        cfg.custom_cpu = Some("Overclock".into());
        cfg.custom_gpu = Some("High".into());
        let loaded = round_trip(&cfg);
        assert_eq!(loaded.custom_cpu.as_deref(), Some("Overclock"));
        assert_eq!(loaded.custom_gpu.as_deref(), Some("High"));
        // blank or junk values mean "nothing saved"
        let old = parse_text("custom_cpu=\ncustom_gpu=a=b\n");
        assert_eq!((old.custom_cpu, old.custom_gpu), (None, None));
    }

    #[test]
    fn no_backdrop_is_selected_until_you_choose_one() {
        assert!(!Config::default().backdrop);
    }

    /// Writes the config the way the program does and reads it back.
    fn round_trip(cfg: &Config) -> Config {
        parse_text(&to_text(cfg))
    }

    #[test]
    fn the_log_file_name_presets_are_kept() {
        let mut cfg = Config::default();
        assert_eq!(cfg.sensor_log_stamp, crate::sensor_log::Stamp::Date, "the date in front, as before");
        cfg.sensor_log_stamp = crate::sensor_log::Stamp::DateTime;
        cfg.sensor_log_name_presets = vec!["3d-mark-cpu".to_string(), "Idle test".to_string()];
        let back = round_trip(&cfg);
        assert_eq!(back.sensor_log_stamp, crate::sensor_log::Stamp::DateTime);
        assert_eq!(back.sensor_log_name_presets, cfg.sensor_log_name_presets);
        assert_eq!(back.log_plan().stamp, crate::sensor_log::Stamp::DateTime);
        assert_eq!(parse_text("sensor_log_stamp=whatever\n").sensor_log_stamp, crate::sensor_log::Stamp::Date);
    }

    #[test]
    fn the_fires_hot_and_cold_colors_are_remembered() {
        let mut cfg = Config::default();
        assert_eq!((cfg.fire_hot, cfg.fire_cold), ([255, 110, 0], [255, 0, 0]), "Chroma's own colors");
        cfg.fire_hot = [10, 200, 255];
        cfg.fire_cold = [0, 0, 90];
        let loaded = round_trip(&cfg);
        assert_eq!((loaded.fire_hot, loaded.fire_cold), ([10, 200, 255], [0, 0, 90]));
        // a config from before has Chroma's colors, and a broken line changes nothing
        let old = parse_text("sensor_enable=\nfire_hot=1,2\n");
        assert_eq!((old.fire_hot, old.fire_cold), ([255, 110, 0], [255, 0, 0]));
    }

    #[test]
    fn the_pad_and_the_keyboard_keep_their_own_fire_colors() {
        let mut cfg = Config::default();
        assert_eq!((cfg.pad_fire_hot, cfg.pad_fire_cold), ([255, 110, 0], [255, 0, 0]));
        cfg.fire_hot = [1, 2, 3];
        cfg.pad_fire_hot = [4, 5, 6];
        cfg.pad_fire_cold = [7, 8, 9];
        let loaded = round_trip(&cfg);
        assert_eq!(loaded.fire_hot, [1, 2, 3]);
        assert_eq!((loaded.pad_fire_hot, loaded.pad_fire_cold), ([4, 5, 6], [7, 8, 9]));
        assert_eq!(loaded.fire_cold, [255, 0, 0], "the keyboard's cold color is not the pad's");
    }

    #[test]
    fn keys_that_move_and_the_brush_survive_a_restart() {
        use librazer::keymap::KeyFx;
        let mut cfg = Config::default();
        cfg.kbd_custom.set_with("W", [255, 0, 0], KeyFx::Breathing);
        cfg.kbd_custom.set_with("A", [0, 255, 0], KeyFx::Static);
        cfg.kbd_custom.set_with("S", [0, 0, 255], KeyFx::Fire);
        cfg.kbd_brush_fx = KeyFx::Flash.index();
        let loaded = round_trip(&cfg);
        assert_eq!(loaded.kbd_custom, cfg.kbd_custom);
        assert_eq!(loaded.kbd_custom.fx_of("W"), KeyFx::Breathing);
        assert_eq!(loaded.kbd_custom.fx_of("S"), KeyFx::Fire);
        assert_eq!(loaded.kbd_brush_fx, KeyFx::Flash.index());
        assert_eq!(parse_text("kbd_brush_fx=99\n").kbd_brush_fx, KeyFx::ALL.len() as u8 - 1);
        // the brush's second color, and whether it is used
        assert_eq!((cfg.kbd_brush2, cfg.kbd_brush_two), ([0, 120, 255], false));
        cfg.kbd_brush2 = [9, 8, 7];
        cfg.kbd_brush_two = true;
        let again = round_trip(&cfg);
        assert_eq!((again.kbd_brush2, again.kbd_brush_two), ([9, 8, 7], true));
        assert_eq!(parse_text("kbd_brush2=1,2\n").kbd_brush2, [0, 120, 255]);
        // the wave's and the wheel's colors, direction and speed
        let mut w = Config::default();
        assert!(!w.wave_custom && !w.wheel_ccw && !w.wheel_custom && !w.pad_wheel_ccw);
        assert_eq!((w.wave_speed, w.wheel_speed, w.pad_wheel_speed), (100, 100, 100));
        w.wave_custom = true;
        w.wave_a = [1, 2, 3];
        w.wave_b = [4, 5, 6];
        w.wave_speed = 60;
        w.wheel_a = [7, 8, 9];
        w.wheel_b = [10, 11, 12];
        w.wheel_ccw = true;
        w.wheel_speed = 180;
        w.wheel_custom = true;
        w.pad_wheel_speed = 70;
        w.pad_wheel_ccw = true;
        w.pad_wheel_custom = true;
        w.pad_wheel_a = [13, 14, 15];
        w.pad_wheel_b = [16, 17, 18];
        let back = round_trip(&w);
        assert!(back.wave_custom && back.wheel_ccw && back.wheel_custom && back.pad_wheel_ccw);
        assert_eq!(back.pad_wheel_speed, 70);
        assert!(back.pad_wheel_custom);
        assert_eq!((back.pad_wheel_a, back.pad_wheel_b), ([13, 14, 15], [16, 17, 18]));
        assert_eq!((back.wave_a, back.wave_b, back.wheel_a, back.wheel_b), ([1, 2, 3], [4, 5, 6], [7, 8, 9], [10, 11, 12]));
        assert_eq!((back.wave_speed, back.wheel_speed), (60, 180));
        assert_eq!(parse_text("wave_a=1,2\nwheel_speed=zz\n").wave_a, librazer::wave::DEFAULT_A);
        assert_eq!(parse_text("wheel_speed=zz\n").wheel_speed, 100);
        // a layout with second colors and the new effects survives a save
        cfg.kbd_custom.set_with_second("Q", [1, 2, 3], Some([4, 5, 6]), KeyFx::Dipp);
        cfg.kbd_custom.set_with_second("E", [7, 8, 9], None, KeyFx::Wheel);
        assert_eq!(round_trip(&cfg).kbd_custom, cfg.kbd_custom);
    }

    #[test]
    fn the_starlights_colors_are_remembered_for_the_keyboard_and_the_pad_apart() {
        let mut cfg = Config::default();
        assert_eq!((cfg.starlight_a, cfg.starlight_two), ([0, 255, 0], false));
        cfg.starlight_a = [1, 2, 3];
        cfg.starlight_b = [4, 5, 6];
        cfg.starlight_two = true;
        cfg.pad_starlight_a = [7, 8, 9];
        let loaded = round_trip(&cfg);
        assert_eq!((loaded.starlight_a, loaded.starlight_b, loaded.starlight_two), ([1, 2, 3], [4, 5, 6], true));
        assert_eq!((loaded.pad_starlight_a, loaded.pad_starlight_two), ([7, 8, 9], false));
    }

    #[test]
    fn effect_speeds_are_percents_that_are_remembered_and_old_breathing_speeds_convert() {
        let mut cfg = Config::default();
        assert_eq!((cfg.kbd_breath_speed, cfg.pad_breath_speed, cfg.dipp_speed), (100, 100, 100));
        cfg.kbd_breath_speed = 40;
        cfg.pad_breath_speed = 220;
        cfg.fire_speed = 60;
        cfg.pad_fire_speed = 150;
        cfg.tidal_speed = 70;
        cfg.pad_tidal_speed = 80;
        cfg.starlight_speed = 90;
        cfg.pad_starlight_speed = 110;
        cfg.spectrum_speed = 120;
        cfg.pad_spectrum_speed = 130;
        cfg.dipp_speed = 140;
        cfg.pad_dipp_speed = 250;
        cfg.rainbow_speed = 45;
        cfg.pad_rainbow_speed = 175;
        let loaded = round_trip(&cfg);
        assert_eq!((loaded.kbd_breath_speed, loaded.pad_breath_speed), (40, 220));
        assert_eq!((loaded.fire_speed, loaded.pad_fire_speed), (60, 150));
        assert_eq!((loaded.tidal_speed, loaded.pad_tidal_speed), (70, 80));
        assert_eq!((loaded.starlight_speed, loaded.pad_starlight_speed), (90, 110));
        assert_eq!((loaded.spectrum_speed, loaded.pad_spectrum_speed), (120, 130));
        assert_eq!((loaded.dipp_speed, loaded.pad_dipp_speed), (140, 250));
        assert_eq!((loaded.rainbow_speed, loaded.pad_rainbow_speed), (45, 175));
        // a file from before the sliders: the four buttons 0 to 3
        let old = parse_text("kbd_breath_speed=0\npad_breath_speed=1\n");
        assert_eq!(old.kbd_breath_speed, 100);
        assert!(old.pad_breath_speed < 100);
        assert!(parse_text("kbd_breath_speed=2\n").kbd_breath_speed > 100);
        // nothing saved: the standard speed
        assert_eq!(parse_text("sensor_enable=\n").fire_speed, 100);
        assert_eq!(parse_text("fire_speed=9999\n").fire_speed, 100);
    }

    #[test]
    fn the_game_preset_switch_is_on_by_default_and_remembered() {
        let mut cfg = Config::default();
        assert!(cfg.game_preset_auto);
        assert!(parse_text("sensor_enable=\n").game_preset_auto);
        cfg.game_preset_auto = false;
        assert!(!round_trip(&cfg).game_preset_auto);
    }

    #[test]
    fn color_dipps_two_colors_are_remembered_for_the_keyboard_and_the_pad_apart() {
        let mut cfg = Config::default();
        assert_eq!((cfg.dipp_a, cfg.dipp_b), ([255, 0, 0], [136, 0, 255]));
        assert_eq!((cfg.pad_dipp_a, cfg.pad_dipp_b), ([255, 0, 0], [136, 0, 255]));
        cfg.dipp_a = [1, 2, 3];
        cfg.dipp_b = [4, 5, 6];
        cfg.pad_dipp_a = [7, 8, 9];
        cfg.pad_dipp_b = [10, 11, 12];
        let loaded = round_trip(&cfg);
        assert_eq!((loaded.dipp_a, loaded.dipp_b), ([1, 2, 3], [4, 5, 6]));
        assert_eq!((loaded.pad_dipp_a, loaded.pad_dipp_b), ([7, 8, 9], [10, 11, 12]));
    }

    #[test]
    fn the_tidals_colors_and_direction_are_remembered() {
        let mut cfg = Config::default();
        assert_eq!((cfg.tidal_a, cfg.tidal_b, cfg.tidal_in), ([0, 255, 0], [0, 0, 255], false));
        cfg.tidal_a = [255, 0, 80];
        cfg.tidal_b = [10, 20, 30];
        cfg.tidal_in = true;
        let loaded = round_trip(&cfg);
        assert_eq!((loaded.tidal_a, loaded.tidal_b, loaded.tidal_in), ([255, 0, 80], [10, 20, 30], true));
        // the pad has its own
        assert_eq!((loaded.pad_tidal_a, loaded.pad_tidal_b, loaded.pad_tidal_in), ([0, 255, 0], [0, 0, 255], false));
        cfg.pad_tidal_a = [1, 2, 3];
        cfg.pad_tidal_in = true;
        let again = round_trip(&cfg);
        assert_eq!((again.pad_tidal_a, again.pad_tidal_in), ([1, 2, 3], true));
        assert_eq!(again.tidal_a, [255, 0, 80]);
    }

    #[test]
    fn everything_the_games_remember_survives_a_restart() {
        use crate::games::Cheat;
        let mut cfg = Config::default();
        for _ in 0..100 {
            cfg.games.record_death(0);
            cfg.games.record_death(1);
            cfg.games.record_death(2);
        }
        cfg.games.set_cheat(Cheat::BorrowedTime, true);
        cfg.games.set_cheat(Cheat::PerfectScore, true);
        cfg.games.set_game3_stage(1);
        cfg.games.set_latency_ms(-40);
        cfg.games.set_volume(35);
        cfg.games.open_underworld();
        cfg.games.set_playlist(vec!["abc-1".to_string(), "def-2".to_string()]);
        let loaded = round_trip(&cfg);
        assert_eq!(loaded.games, cfg.games);
        assert!(loaded.games.cheat_on(Cheat::BorrowedTime) && loaded.games.cheat_on(Cheat::PerfectScore));
        assert_eq!(loaded.games.game3_stage(), 1);
        assert_eq!(loaded.games.latency_ms(), -40);
        assert_eq!(loaded.games.volume(), 35);
        assert!(loaded.games.underworld());
        assert_eq!(loaded.games.playlist(), ["abc-1", "def-2"]);
    }

    #[test]
    fn every_tab_is_shown_by_default() {
        let cfg = Config::default();
        assert!(cfg.hidden_tabs.is_empty());
        for (key, _) in TOGGLE_TABS {
            assert!(cfg.tab_shown(key), "{} should be shown", key);
        }
        assert_eq!(cfg.backdrop_style, crate::info::BackdropStyle::Circuit);
    }

    #[test]
    fn the_window_opens_normally_unless_start_minimized_is_on() {
        assert!(!Config::default().start_minimized);
        let mut cfg = Config::default();
        cfg.start_minimized = true;
        assert!(round_trip(&cfg).start_minimized);
        assert!(!round_trip(&Config::default()).start_minimized);
        assert!(!parse_text("start_minimized=false\n").start_minimized);
        assert!(parse_text("start_minimized=1\n").start_minimized);
    }

    #[test]
    fn hiding_and_showing_tabs_keeps_a_tidy_list() {
        let mut cfg = Config::default();
        cfg.set_tab_shown("info", false);
        cfg.set_tab_shown("lighting", false);
        cfg.set_tab_shown("info", false); // twice: still once
        assert_eq!(cfg.hidden_tabs, ["lighting", "info"]);
        assert!(!cfg.tab_shown("info") && cfg.tab_shown("files"));
        cfg.set_tab_shown("info", true);
        assert_eq!(cfg.hidden_tabs, ["lighting"]);
        cfg.set_tab_shown("main", false); // Main cannot be hidden
        cfg.set_tab_shown("bogus", false);
        assert_eq!(cfg.hidden_tabs, ["lighting"]);
    }

    #[test]
    fn the_backdrop_moves_unless_switched_off() {
        assert!(Config::default().backdrop_animated);
        let mut cfg = Config::default();
        cfg.backdrop_animated = false;
        assert!(!round_trip(&cfg).backdrop_animated);
        cfg.backdrop_animated = true;
        assert!(round_trip(&cfg).backdrop_animated);
        assert!(parse_text("sensor_enable=\n").backdrop_animated);
    }

    #[test]
    fn gold_edition_is_locked_until_earned_and_its_choices_are_remembered() {
        let cfg = Config::default();
        assert!(!cfg.gold_unlocked && !cfg.gold_active());
        assert!(cfg.gold_on && cfg.gold_banner && cfg.gold_sparkles && cfg.gold_glitch);
        let mut cfg = Config::default();
        cfg.gold_unlocked = true;
        assert!(cfg.gold_active());
        cfg.gold_on = false;
        assert!(!cfg.gold_active());
        cfg.gold_banner = false;
        cfg.gold_sparkles = false;
        cfg.gold_glitch = false;
        let loaded = round_trip(&cfg);
        assert!(loaded.gold_unlocked && !loaded.gold_on && !loaded.gold_banner);
        assert!(!loaded.gold_sparkles && !loaded.gold_glitch);
        // a file that says nothing about it leaves it locked
        assert!(!parse_text("sensor_enable=\n").gold_unlocked);
    }

    #[test]
    fn the_games_section_is_for_the_look_you_are_in_and_after_enough_deaths() {
        use crate::games::{Difficulty, Edition};
        let mut cfg = Config::default();
        assert_eq!(cfg.edition(), Edition::Standard);
        assert!(!cfg.games_section_shown());
        for _ in 0..10 {
            cfg.games.record_death(0);
        }
        assert!(cfg.games_section_shown(), "ten deaths in Game 1 bring it up in the ordinary look");
        // in the Gold Edition it is Game 2's section, which has not been earned
        cfg.gold_unlocked = true;
        assert_eq!(cfg.edition(), Edition::Gold);
        assert!(!cfg.games_section_shown());
        for _ in 0..10 {
            cfg.games.record_death(1);
        }
        assert!(cfg.games_section_shown());
        // switching Gold off goes back to the ordinary look's section
        cfg.gold_on = false;
        assert_eq!(cfg.edition(), Edition::Standard);
        assert!(cfg.games_section_shown());
        // it is all remembered
        for _ in 0..25 {
            cfg.games.record_death(0);
        }
        cfg.games.set_difficulty(0, Difficulty::Easy);
        let loaded = round_trip(&cfg);
        assert_eq!(loaded.games, cfg.games);
        assert_eq!(loaded.games.difficulty(0), Difficulty::Easy);
        assert_eq!(loaded.games.deaths(0), 35);
        assert!(loaded.games.shown(0) && loaded.games.shown(1) && !loaded.games.shown(2));
        // a config from before the games has none of it
        assert_eq!(parse_text("sensor_enable=\n").games, crate::games::Progress::default());
        assert!(!parse_text("sensor_enable=\n").void_unlocked);
        cfg.void_unlocked = true;
        assert!(round_trip(&cfg).void_unlocked);
    }

    #[test]
    fn the_void_is_locked_until_game_2_is_won_then_it_takes_over_from_gold() {
        use crate::games::Edition;
        use crate::info::BackdropStyle;
        let mut cfg = Config::default();
        assert!(!cfg.void_unlocked && !cfg.void_active() && cfg.void_on && cfg.void_banner);
        cfg.unlock_gold_by_winning();
        assert!(cfg.gold_active() && cfg.edition() == Edition::Gold);
        // the glitch stars are not used while it is locked, whatever the file says
        cfg.backdrop_style = BackdropStyle::VoidStars;
        assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::default());
        // winning Game 2: the Void is on, Gold is off (and still earned), and the stars show
        cfg.backdrop = false;
        cfg.unlock_void_by_winning();
        assert!(cfg.void_active() && !cfg.gold_active() && cfg.gold_unlocked);
        assert_eq!(cfg.edition(), Edition::Void);
        assert!(cfg.backdrop && cfg.shown_backdrop_style() == BackdropStyle::VoidStars);
        // all of it is remembered
        let loaded = round_trip(&cfg);
        assert!(loaded.void_unlocked && loaded.void_on && loaded.void_banner && loaded.void_script && loaded.void_active());
        cfg.void_script = false;
        assert!(!round_trip(&cfg).void_script, "the switch is remembered");
        cfg.void_script = true;
        // the hidden sentence's place is picked when The Void is unlocked, kept, and picked anew after a lock
        assert_ne!(cfg.void_seed, 0);
        assert_eq!(round_trip(&cfg).void_seed, cfg.void_seed);
        let seed = cfg.void_seed;
        cfg.unlock_void_by_winning();
        assert_eq!(cfg.void_seed, seed, "winning again does not move it");
        assert_eq!(parse_text("void_unlocked=true\n").void_seed != 0, true, "an older save gets one");
        assert_eq!(parse_text("sensor_enable=\n").void_seed, 0);
        // the moon: the staff, once broken, stays broken until The Void is locked
        assert!(!cfg.void_staff_broken && !cfg.void_pretend_night);
        cfg.void_staff_broken = true;
        cfg.void_pretend_night = true;
        let loaded = round_trip(&cfg);
        assert!(loaded.void_staff_broken && loaded.void_pretend_night);
        assert!(!parse_text("sensor_enable=\n").void_staff_broken && !parse_text("sensor_enable=\n").void_pretend_night);
        // the Underworld look: off until switched on, and it needs the Underworld to be open
        assert!(!cfg.underworld_on && !cfg.underworld_active() && !parse_text("sensor_enable=\n").underworld_on);
        cfg.underworld_on = true;
        assert!(!cfg.underworld_active(), "switched on but not open yet");
        assert!(round_trip(&cfg).underworld_on);
        cfg.games.open_underworld();
        assert!(cfg.underworld_active() && round_trip(&cfg).underworld_active());
        // while it shows, Gold and The Void step aside, but the games stay those of the look under it
        cfg.void_unlocked = true;
        cfg.void_on = true;
        assert!(!cfg.void_active() && !cfg.gold_active());
        assert_eq!(cfg.edition(), crate::games::Edition::Void);
        cfg.underworld_on = false;
        assert!(cfg.void_active());
        assert!(!cfg.underworld_active());
        cfg.void_staff_broken = false;
        cfg.void_pretend_night = false;
        assert!(!loaded.gold_active() && loaded.backdrop_style == BackdropStyle::VoidStars);
        // switched off, Gold comes back
        cfg.void_on = false;
        cfg.gold_on = true;
        assert!(!cfg.void_active() && cfg.gold_active() && cfg.edition() == Edition::Gold);
        // locking it again takes its backdrop away and mends the staff
        cfg.void_staff_broken = true;
        cfg.lock_void();
        assert!(!cfg.void_unlocked && cfg.backdrop_style == BackdropStyle::default());
        assert!(!cfg.void_staff_broken, "a locked Void has a whole staff again");
        // a config from before the Void has it locked
        assert!(!parse_text("sensor_enable=\n").void_active());
    }

    #[test]
    fn winning_shows_off_the_angel_cats_and_the_king_instead_of_father_time() {
        use crate::info::BackdropStyle;
        for (backdrop, style) in [(true, BackdropStyle::FatherTime), (false, BackdropStyle::FatherTime), (true, BackdropStyle::Dragons), (false, BackdropStyle::Circuit)] {
            let mut cfg = Config::default();
            cfg.backdrop = backdrop;
            cfg.backdrop_style = style;
            cfg.unlock_gold_by_winning();
            assert!(cfg.gold_active());
            assert!(cfg.backdrop && cfg.backdrop_style == BackdropStyle::AngelCats);
            assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::AngelCats, "it is allowed: Gold is unlocked");
            assert!(!cfg.shown_backdrop_style().has_clock(), "Father Time is not standing where the king goes");
        }
        // it is remembered, and locking Gold again takes it away
        let mut cfg = Config::default();
        cfg.unlock_gold_by_winning();
        let loaded = round_trip(&cfg);
        assert!(loaded.backdrop && loaded.backdrop_style == BackdropStyle::AngelCats);
        cfg.lock_gold();
        assert_eq!(cfg.backdrop_style, BackdropStyle::default());
    }

    #[test]
    fn the_angel_cat_backdrop_is_only_used_while_gold_is_unlocked() {
        use crate::info::BackdropStyle::*;
        let mut cfg = Config::default();
        cfg.backdrop_style = AngelCats;
        assert_eq!(cfg.shown_backdrop_style(), Circuit, "locked: not shown, even if the file says so");
        assert_eq!(round_trip(&cfg).backdrop_style, AngelCats, "the choice itself is not thrown away");
        cfg.gold_unlocked = true;
        assert_eq!(cfg.shown_backdrop_style(), AngelCats);
        // it does not depend on the Gold look being switched on: any theme can have it
        cfg.gold_on = false;
        assert_eq!(cfg.shown_backdrop_style(), AngelCats);
        // the other styles are never held back
        for style in [Circuit, Predator, Cats, Dragons, FatherTime] {
            cfg.backdrop_style = style;
            cfg.gold_unlocked = false;
            assert_eq!(cfg.shown_backdrop_style(), style);
        }
        // locking Gold again puts the angel cats away, and only them
        cfg.gold_unlocked = true;
        cfg.backdrop_style = AngelCats;
        cfg.lock_gold();
        assert!(!cfg.gold_unlocked);
        assert_eq!(cfg.backdrop_style, Circuit);
        cfg.backdrop_style = Dragons;
        cfg.lock_gold();
        assert_eq!(cfg.backdrop_style, Dragons);
    }

    #[test]
    fn hidden_tabs_and_backdrop_style_are_remembered() {
        let mut cfg = Config::default();
        cfg.set_tab_shown("sensors", false);
        cfg.set_tab_shown("changelog", false);
        for style in crate::info::BackdropStyle::ALL {
            cfg.backdrop_style = style;
            assert_eq!(round_trip(&cfg).backdrop_style, style);
        }
        cfg.backdrop_style = crate::info::BackdropStyle::Cats;
        let loaded = round_trip(&cfg);
        assert_eq!(loaded.hidden_tabs, ["sensors", "changelog"]);
        assert_eq!(loaded.backdrop_style, crate::info::BackdropStyle::Cats);
        // an untouched config comes back with every tab
        assert!(round_trip(&Config::default()).hidden_tabs.is_empty());
    }

    #[test]
    fn junk_in_the_saved_tab_list_is_ignored() {
        let mut cfg = Config::default();
        cfg.hidden_tabs = vec!["files".into()];
        let text = "hidden_tabs=main, files ,nope,,info\nbackdrop_style=nonsense\n";
        let loaded = parse_text(text);
        assert_eq!(loaded.hidden_tabs, ["files", "info"]);
        assert_eq!(loaded.backdrop_style, crate::info::BackdropStyle::Circuit);
    }

    #[test]
    fn the_logging_plan_is_remembered_and_only_waiting_for_a_program_is_armed() {
        let mut cfg = Config::default();
        assert_eq!(cfg.log_plan(), crate::log_plan::Plan { stop_with_program: true, ..Default::default() });
        cfg.sensor_log_start = crate::log_plan::Start::Program;
        cfg.sensor_log_program = r"C:\Games\Cinebench".into();
        cfg.sensor_log_limit_secs = 5400;
        cfg.sensor_log_stop_with_program = false;
        cfg.sensor_logging = true;
        let back = round_trip(&cfg);
        assert_eq!(back.sensor_log_start, crate::log_plan::Start::Program);
        assert_eq!(back.log_plan().program, "Cinebench.exe");
        assert_eq!(back.sensor_log_limit_secs, 5400);
        assert!(!back.sensor_log_stop_with_program);
        assert!(back.sensor_logging, "waiting for the program is kept");
        // a log that starts at once is never on after a start
        cfg.sensor_log_start = crate::log_plan::Start::Now;
        assert!(!round_trip(&cfg).sensor_logging);
        // nonsense is harmless
        let odd = parse_text("sensor_log_limit_secs=-4\nsensor_log_start=x\nsensor_log_armed=true\n");
        assert_eq!((odd.sensor_log_limit_secs, odd.sensor_log_start, odd.sensor_logging), (0, crate::log_plan::Start::Now, false));
        assert_eq!(parse_text("sensor_log_limit_secs=99999999999\n").sensor_log_limit_secs, crate::log_plan::MAX_LIMIT_SECS);
    }

    #[test]
    fn log_viewer_settings_are_kept_in_one_line() {
        let mut options = crate::log_options::Options::default();
        options.size_side = 150.0;
        options.fahrenheit = true;
        options.set_colour("cpu_temp", [10, 20, 30]);
        let mut cfg = Config::default();
        cfg.log_view_options = options.encode();
        let back = parse_text(&to_text(&cfg));
        assert_eq!(crate::log_options::Options::decode(&back.log_view_options), options);
        assert_eq!(parse_text("").log_view_options, "");
    }

    #[test]
    fn sensor_source_defaults_to_built_in() {
        let cfg = Config::default();
        assert!(cfg.sensor_enabled.iter().all(|e| !e));
        assert!(!cfg.sensor_logging);
        assert!(!cfg.sensor_read);
        assert_eq!(cfg.graph_window, 1);
        assert!(cfg.sensor_graphs);
        assert!(!cfg.run_as_admin);
        assert_eq!(cfg.sensor_interval_ms, 1000, "factory refresh interval");
        let source = cfg.sensor_source_config();
        assert_eq!(source, crate::sensor_ext::SourceConfig::default());
        assert_eq!(source.lhm_addr, "127.0.0.1:8085");
        assert!(source.log_path.is_empty());
    }

    #[test]
    fn names_are_cleaned() {
        assert_eq!(clean_preset_name("  a|b\nc  "), "abc");
        assert_eq!(clean_preset_name(&"x".repeat(50)).chars().count(), 20);
    }

    #[test]
    fn slot_index_is_bounded() {
        assert_eq!(slot_index("kbd_slot_9", "kbd_slot_"), Some(9));
        assert_eq!(slot_index("kbd_slot_19", "kbd_slot_"), Some(19));
        assert_eq!(slot_index("kbd_slot_20", "kbd_slot_"), None);
        assert_eq!(slot_index("kbd_slot_x", "kbd_slot_"), None);
    }

    #[test]
    fn a_bought_skin_shows_only_when_no_earned_look_does_and_trying_one_on_is_free() {
        let mut cfg = parse_text("sensor_enable=\n");
        assert!(!cfg.fraying_active());
        cfg.shop_preview = Some("fraying".to_string());
        assert!(cfg.fraying_active(), "trying it on needs nothing");
        cfg.shop_preview = None;
        cfg.shop.earn(100);
        cfg.shop.buy("fraying").unwrap();
        assert!(!cfg.fraying_active(), "bought but not on");
        cfg.unlock_gold_by_winning();
        assert!(cfg.gold_active());
        cfg.shop.set_skin(Some("fraying"));
        assert!(!cfg.fraying_active(), "gold is showing");
        cfg.shop_preview = Some("fraying".to_string());
        assert!(cfg.fraying_active() && !cfg.gold_active(), "trying it on covers the look");
        cfg.shop_preview = None;
        assert!(cfg.gold_active());
        cfg.use_skin("fraying");
        assert!(cfg.fraying_active() && !cfg.gold_active() && cfg.gold_unlocked, "using it turns the look off but keeps it");
        assert!(round_trip(&cfg).fraying_active());
        cfg.use_skin("not_bought");
        assert!(cfg.fraying_active());
    }

    #[test]
    fn only_one_skin_shows_at_a_time_and_using_one_replaces_the_other() {
        let mut cfg = parse_text("sensor_enable=\n");
        cfg.shop.earn(200);
        cfg.shop.buy("fraying").unwrap();
        cfg.shop.buy("sakura").unwrap();
        cfg.use_skin("fraying");
        assert!(cfg.fraying_active() && !cfg.sakura_active());
        cfg.use_skin("sakura");
        assert!(cfg.sakura_active() && !cfg.fraying_active());
        cfg.shop_preview = Some("fraying".to_string());
        assert!(cfg.fraying_active() && !cfg.sakura_active(), "trying one on covers the one that is on");
        cfg.shop_preview = None;
        assert!(round_trip(&cfg).sakura_active());
    }

    #[test]
    fn a_shop_backdrop_is_only_used_once_bought_or_while_it_is_being_tried_on() {
        use crate::info::BackdropStyle;
        let mut cfg = parse_text("sensor_enable=\n");
        cfg.backdrop_style = BackdropStyle::Embers;
        assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::default());
        cfg.shop_preview = Some("embers".to_string());
        assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::Embers);
        cfg.backdrop_style = BackdropStyle::Circuit;
        cfg.shop_preview = Some("northern".to_string());
        assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::Northern);
        cfg.shop_preview = None;
        cfg.shop.earn(50);
        cfg.shop.buy("embers").unwrap();
        cfg.backdrop_style = BackdropStyle::Embers;
        assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::Embers);
    }

    #[test]
    fn the_token_wallet_is_remembered() {
        let mut cfg = parse_text("sensor_enable=\n");
        assert_eq!(cfg.shop, crate::shop::Wallet::default());
        cfg.shop.open(10);
        cfg.shop.earn(60);
        cfg.shop.buy("fraying").unwrap();
        cfg.shop.set_skin(Some("fraying"));
        let loaded = round_trip(&cfg);
        assert_eq!(loaded.shop, cfg.shop);
        assert_eq!(loaded.shop.balance(), 30);
        assert_eq!(loaded.shop.skin(), Some("fraying"));
    }

    #[test]
    fn the_dead_monarch_king_backdrop_is_only_shown_while_the_void_is_the_look() {
        use crate::info::BackdropStyle;
        let mut cfg = Config::default();
        cfg.backdrop_style = BackdropStyle::DeadKing;
        // locked: not shown, whatever the file says
        assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::default());
        cfg.unlock_gold_by_winning();
        cfg.backdrop_style = BackdropStyle::DeadKing;
        assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::default());
        // the Void earned and on: the file's choice is honoured
        cfg.unlock_void_by_winning();
        cfg.backdrop_style = BackdropStyle::DeadKing;
        assert!(cfg.void_active());
        assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::DeadKing);
        assert_eq!(round_trip(&cfg).backdrop_style, BackdropStyle::DeadKing);
        // earned but switched off: not shown (the glitch stars, by contrast, are)
        cfg.void_on = false;
        assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::default());
        cfg.backdrop_style = BackdropStyle::VoidStars;
        assert_eq!(cfg.shown_backdrop_style(), BackdropStyle::VoidStars);
    }

    #[test]
    fn the_heard_whispers_are_remembered_and_earn_a_trophy() {
        let mut cfg = Config::default();
        assert_eq!(cfg.whispers_heard, 0);
        assert_eq!(parse_text("whispers_heard=nonsense\n").whispers_heard, 0);
        cfg.whispers_heard = 0b1011;
        assert_eq!(round_trip(&cfg).whispers_heard, 0b1011);
        assert_eq!(parse_text("whispers_spoken=nonsense\n").whispers_spoken, 0);
        cfg.whispers_spoken = 17;
        assert_eq!(round_trip(&cfg).whispers_spoken, 17);
        assert_eq!(cfg.trophy_facts().whispers, 3);
        assert!(cfg.sync_trophies(1).iter().all(|t| t.id != "listener"));
        cfg.whispers_heard = u32::MAX;
        assert_eq!(cfg.trophy_facts().whispers as usize, crate::whispers::LINES.len());
        assert!(cfg.sync_trophies(2).iter().any(|t| t.id == "listener"));
    }

    #[test]
    fn the_platinum_banner_is_on_by_default_and_remembered() {
        let mut cfg = Config::default();
        assert!(cfg.platinum_banner && parse_text("sensor_enable=\n").platinum_banner);
        cfg.platinum_banner = false;
        assert!(!round_trip(&cfg).platinum_banner);
        cfg.platinum_banner = true;
        assert!(round_trip(&cfg).platinum_banner);
        // the moving light too: on unless switched off, and remembered
        assert!(cfg.platinum_effects && parse_text("sensor_enable=\n").platinum_effects);
        cfg.platinum_effects = false;
        assert!(!round_trip(&cfg).platinum_effects);
        cfg.platinum_effects = true;
        assert!(round_trip(&cfg).platinum_effects);
    }

    #[test]
    fn trophies_are_earned_from_the_facts_saved_and_platinum_gives_its_skin() {
        let mut cfg = Config::default();
        cfg.shop.set_balance(100_000);
        for _ in 0..10 {
            cfg.games.record_death(0);
        }
        let new: Vec<&str> = cfg.sync_trophies(1_760_000_000).iter().map(|t| t.id).collect();
        assert!(new.contains(&"ten_deaths"));
        assert!(cfg.sync_trophies(1_760_000_001).is_empty(), "only what is new is reported");
        // saved and read back
        let back = parse_text(&to_text(&cfg));
        assert!(back.trophies.has("ten_deaths") && back.trophies.when("ten_deaths") == Some(1_760_000_000));
        // everything the facts can earn: the shop emptied, the worlds, the games, the cheats
        for item in crate::shop::CATALOG {
            let _ = cfg.shop.buy(item.id);
        }
        cfg.shop.set_balance(1_000);
        cfg.gold_unlocked = true;
        cfg.void_unlocked = true;
        cfg.games.open_underworld();
        cfg.games.set_cat_helped(true);
        for g in 0..3 {
            for _ in 0..120 {
                cfg.games.record_death(g);
            }
            let _ = cfg.shop.take_first_win(g);
        }
        cfg.shop.earn(1_000);
        cfg.whispers_heard = u32::MAX;
        // a game won at the Devil's Table, and one lost to the Shadow Monarch
        use crate::poker_game::{Ending, BEAR, CAT};
        let _ = cfg.poker.finish(Ending::Victory { first_out: CAT, last: BEAR }, 1_760_000_050);
        let _ = cfg.poker.finish(Ending::MonarchLate, 1_760_000_060);
        // and the two lovers dealt to you together
        assert!(cfg.poker.mark_together());
        assert!(!cfg.shop.owns("platinum"));
        let new: Vec<&str> = cfg.sync_trophies(1_760_000_100).iter().map(|t| t.id).collect();
        assert_eq!(new.last(), Some(&"platinum"), "{new:?}");
        assert!(cfg.shop.owns("platinum"), "the Platinum skin comes with the trophy");
        // it is wearable, and a New Game+ that takes the worlds away leaves the trophies and the skin
        cfg.use_skin("platinum");
        assert_eq!(cfg.skin_active(), Some("platinum"));
        cfg.gold_unlocked = false;
        cfg.void_unlocked = false;
        assert!(cfg.sync_trophies(1_760_000_200).is_empty());
        assert!(cfg.trophies.has("gold") && cfg.shop.owns("platinum"));
        // testing: reset the trophies and the skin goes with them on the next sync only if earned again
        cfg.trophies.clear();
        cfg.shop.revoke("platinum");
        assert!(!cfg.shop.owns("platinum") && cfg.shop.skin().is_none());
    }
}
