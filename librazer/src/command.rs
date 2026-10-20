use crate::device::Device;
use crate::keymap::{KeyGrid, KeyLayout};
use crate::packet::Packet;
use crate::types::{
    BatteryCare, Cluster, CpuBoost, FanMode, FanZone, GpuBoost, LightsAlwaysOn, LogoMode,
    MaxFanSpeedMode, PerfMode,
};

use anyhow::{bail, ensure, Result};

fn _send_command(device: &Device, command: u16, args: &[u8]) -> Result<Packet> {
    let response = device.send(Packet::new(command, args))?;
    ensure!(response.get_args().starts_with(args));
    Ok(response)
}

fn _set_perf_mode(device: &Device, perf_mode: PerfMode, fan_mode: FanMode) -> Result<()> {
    [1, 2].into_iter().try_for_each(|zone| {
        _send_command(device, 0x0d02, &[0x01, zone, perf_mode as u8, fan_mode as u8]).map(|_| ())
    })
}

fn _set_boost(device: &Device, cluster: Cluster, boost: u8) -> Result<()> {
    let args = &[0x01, cluster as u8, boost];
    ensure!(
        get_perf_mode(device)?.0 == PerfMode::Custom,
        "Performance mode must be {:?}",
        PerfMode::Custom
    );
    ensure!(device.send(Packet::new(0x0d07, args))?.get_args().starts_with(args));
    Ok(())
}

fn _get_boost(device: &Device, cluster: Cluster) -> Result<u8> {
    let response = device.send(Packet::new(0x0d87, &[0, cluster as u8, 0]))?;
    ensure!(response.get_args()[1] == cluster as u8);
    Ok(response.get_args()[2])
}

pub fn set_perf_mode(device: &Device, perf_mode: PerfMode) -> Result<()> {
    _set_perf_mode(device, perf_mode, FanMode::Auto)
}

pub fn get_perf_mode(device: &Device) -> Result<(PerfMode, FanMode)> {
    let [r1, r2]: [Result<(PerfMode, FanMode)>; 2] = [1, 2].map(|zone| {
        let response = device.send(Packet::new(0x0d82, &[0, zone, 0, 0]))?;
        Ok((
            PerfMode::try_from(response.get_args()[2])?,
            FanMode::try_from(response.get_args()[3])?,
        ))
    });

    ensure!(
        r1.is_ok() && r2.is_ok(),
        "Failed to get performance mode and fan mode: r1 = {:?}, r2 = {:?}",
        r1,
        r2
    );

    let r1 = r1?;
    let r2 = r2?;

    //let r1 = r1?;
    ensure!(r1 == r2, "Modes do not match: r1 = {:?}, r2 = {:?}", r1, r2);

    Ok(r1)
}

pub fn set_cpu_boost(device: &Device, boost: CpuBoost) -> Result<()> {
    _set_boost(device, Cluster::Cpu, boost as u8)
}

pub fn set_gpu_boost(device: &Device, boost: GpuBoost) -> Result<()> {
    _set_boost(device, Cluster::Gpu, boost as u8)
}

pub fn get_cpu_boost(device: &Device) -> Result<CpuBoost> {
    CpuBoost::try_from(_get_boost(device, Cluster::Cpu)?)
}

pub fn get_gpu_boost(device: &Device) -> Result<GpuBoost> {
    GpuBoost::try_from(_get_boost(device, Cluster::Gpu)?)
}

pub fn set_fan_rpm(device: &Device, rpm: u16, check_mode: bool) -> Result<()> {
    ensure!((0..=5500).contains(&rpm));
    if check_mode {
        ensure!(
            matches!(get_perf_mode(device)?, (_, FanMode::Manual)),
            "Fan mode must be set to {:?}",
            FanMode::Manual
        );
    }
    [FanZone::Zone1, FanZone::Zone2].into_iter().try_for_each(|zone| {
        _send_command(device, 0x0d01, &[0, zone as u8, (rpm / 100) as u8]).map(|_| ())
    })
}

pub fn get_fan_rpm(device: &Device, fan_zone: FanZone) -> Result<u16> {
    let response = device.send(Packet::new(0x0d81, &[0, fan_zone as u8, 0]))?;
    ensure!(response.get_args()[1] == fan_zone as u8);
    Ok(response.get_args()[2] as u16 * 100)
}

pub fn get_fan_actual_rpm(device: &Device, fan_zone: FanZone) -> Result<u16> {
    let response = device.send(Packet::new(0x0d88, &[0, fan_zone as u8, 0]))?;
    ensure!(response.get_args()[1] == fan_zone as u8);
    Ok(response.get_args()[2] as u16 * 100)
}

pub fn send_command(device: &Device, command: u16, args: &[u8]) -> Result<Packet> {
    let response = device.send(Packet::new(command, args))?;
    Ok(response)
}

pub fn set_max_fan_speed_mode(device: &Device, mode: MaxFanSpeedMode) -> Result<()> {
    ensure!(
        get_perf_mode(device)?.0 == PerfMode::Custom,
        "Performance mode must be {:?}",
        PerfMode::Custom
    );
    _send_command(device, 0x070f, &[mode as u8]).map(|_| ())
}

pub fn get_max_fan_speed_mode(device: &Device) -> Result<MaxFanSpeedMode> {
    device.send(Packet::new(0x078f, &[0]))?.get_args()[0].try_into()
}

pub fn set_fan_mode(device: &Device, mode: FanMode) -> Result<()> {
    _set_perf_mode(device, get_perf_mode(device)?.0, mode)
}

pub fn custom_command(device: &Device, command: u16, args: &[u8]) -> Result<()> {
    let report = Packet::new(command, args);
    println!("Report   {:?}", report);
    let response = device.send(report)?;
    println!("Response {:?}", response);
    Ok(())
}

fn _set_logo_power(device: &Device, mode: LogoMode) -> Result<Packet> {
    match mode {
        LogoMode::Off => _send_command(device, 0x0300, &[1, 4, 0]),
        LogoMode::Static | LogoMode::Breathing => _send_command(device, 0x0300, &[1, 4, 1]),
    }
}

fn _set_logo_mode(device: &Device, mode: LogoMode) -> Result<Packet> {
    match mode {
        LogoMode::Static => _send_command(device, 0x0302, &[1, 4, 0]),
        LogoMode::Breathing => _send_command(device, 0x0302, &[1, 4, 2]),
        _ => bail!("Invalid logo mode"),
    }
}

fn _get_logo_power(device: &Device) -> Result<bool> {
    match device.send(Packet::new(0x0380, &[1, 4, 0]))?.get_args()[2] {
        0 => Ok(false),
        1 => Ok(true),
        _ => bail!("Invalid logo power state"),
    }
}

fn _get_logo_mode(device: &Device) -> Result<LogoMode> {
    match device.send(Packet::new(0x0382, &[1, 4, 0]))?.get_args()[2] {
        0 => Ok(LogoMode::Static),
        2 => Ok(LogoMode::Breathing),
        _ => bail!("Invalid logo power state"),
    }
}

pub fn get_logo_mode(device: &Device) -> Result<LogoMode> {
    let power = _get_logo_power(device)?;
    match power {
        true => _get_logo_mode(device),
        false => Ok(LogoMode::Off),
    }
}

pub fn set_logo_mode(device: &Device, mode: LogoMode) -> Result<()> {
    if mode != LogoMode::Off {
        _set_logo_mode(device, mode)?;
    }
    _set_logo_power(device, mode)?;
    Ok(())
}

pub fn get_keyboard_brightness(device: &Device) -> Result<u8> {
    let response = device.send(Packet::new(0x0383, &[1, 5, 0]))?;
    ensure!(response.get_args()[1] == 5);
    Ok(response.get_args()[2])
}

pub fn set_keyboard_brightness(device: &Device, brightness: u8) -> Result<()> {
    let args = &[1, 5, brightness];
    ensure!(device.send(Packet::new(0x0303, args))?.get_args().starts_with(args));
    Ok(())
}

pub fn get_lights_always_on(device: &Device) -> Result<LightsAlwaysOn> {
    device.send(Packet::new(0x0084, &[0, 0]))?.get_args()[0].try_into()
}

pub fn set_lights_always_on(device: &Device, lights_always_on: LightsAlwaysOn) -> Result<()> {
    let args = &[lights_always_on as u8, 0];
    ensure!(device.send(Packet::new(0x0004, args))?.get_args().starts_with(args));
    Ok(())
}

/// Lowest / highest charge limit the Blade firmware is expected to accept (percent).
pub const BATTERY_LIMIT_MIN: u8 = 50;
pub const BATTERY_LIMIT_MAX: u8 = 80;

// Battery Health Optimizer register layout (0x0792 get / 0x0712 set), inferred from the two
// values the original code used: 0x50 (disabled, 80%) and 0xd0 (enabled, 80%).
//   bit 7      = optimizer enabled
//   bits 0..=6 = charge limit in percent
// Verified on a Blade 16 (2023): 50..=80 in 5% steps hold. The set call checks the echoed reply.
// Firmware only resumes charging once the battery is ~10 points below the limit (hysteresis).

/// Returns (enabled, limit_percent).
pub fn get_battery_limit(device: &Device) -> Result<(bool, u8)> {
    let raw = device.send(Packet::new(0x0792, &[0]))?.get_args()[0];
    Ok((raw & 0x80 != 0, raw & 0x7f))
}

pub fn set_battery_limit(device: &Device, enabled: bool, percent: u8) -> Result<()> {
    let pct = percent.clamp(BATTERY_LIMIT_MIN, BATTERY_LIMIT_MAX);
    let flag: u8 = if enabled { 0x80 } else { 0x00 };
    let args = &[pct | flag];
    ensure!(device.send(Packet::new(0x0712, args))?.get_args().starts_with(args));
    Ok(())
}

pub fn get_battery_care(device: &Device) -> Result<BatteryCare> {
    let (enabled, _) = get_battery_limit(device)?;
    Ok(if enabled { BatteryCare::Enable } else { BatteryCare::Disable })
}

pub fn set_battery_care(device: &Device, mode: BatteryCare) -> Result<()> {
    // Preserve the user's chosen limit; only flip the enable bit.
    let pct = get_battery_limit(device).map(|(_, p)| p).unwrap_or(BATTERY_LIMIT_MAX);
    set_battery_limit(device, matches!(mode, BatteryCare::Enable), pct)
}

/// Laptop keyboard lighting effects (Razer "extended matrix effect", class 0x0F id 0x02).
///
/// Opt-in only: the app sends one of these when the user clicks an effect, never at launch.
/// The argument layout follows the Razer extended-matrix-effect format used by openrazer and
/// razer-laptop-control; it has not been verified on every Blade model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardEffect {
    Off,
    Static,
    Breathing,
    Spectrum,
    WaveRight,
    WaveLeft,
    /// Individually chosen keys (painted as a custom frame, see `keymap`), not a firmware effect.
    Custom,
    /// A flickering flame, drawn by the app a frame at a time (see `fire`), not a firmware effect.
    Fire,
    /// Waves of two colors across the keys, drawn by the app a frame at a time (see `tidal`).
    Tidal,
    /// Stars twinkling on random keys, drawn by the app a frame at a time (see `starlight`).
    Starlight,
    /// The whole keyboard going from one picked color to another through the hues between,
    /// drawn by the app a frame at a time (see `dipp`).
    Dipp,
    /// The whole color wheel across the keys, scrolling along, drawn by the app a frame at a
    /// time (see `rainbow`).
    Rainbow,
    /// A hand like a clock's sweeping round the middle of the keyboard with a fading tail, drawn by
    /// the app a frame at a time (see `wheel`).
    Wheel,
}

impl KeyboardEffect {
    pub const ALL: [KeyboardEffect; 13] = [
        KeyboardEffect::Static,
        KeyboardEffect::Breathing,
        KeyboardEffect::Spectrum,
        KeyboardEffect::WaveRight,
        KeyboardEffect::WaveLeft,
        KeyboardEffect::Custom,
        KeyboardEffect::Fire,
        KeyboardEffect::Tidal,
        KeyboardEffect::Starlight,
        KeyboardEffect::Dipp,
        KeyboardEffect::Rainbow,
        KeyboardEffect::Wheel,
        KeyboardEffect::Off,
    ];

    pub fn name(self) -> &'static str {
        match self {
            KeyboardEffect::Off => "Off",
            KeyboardEffect::Static => "Static",
            KeyboardEffect::Breathing => "Breathing",
            KeyboardEffect::Spectrum => "Spectrum",
            KeyboardEffect::WaveRight => "Wave Right",
            KeyboardEffect::WaveLeft => "Wave Left",
            KeyboardEffect::Custom => "Custom Keys",
            KeyboardEffect::Fire => "Fire",
            KeyboardEffect::Tidal => "Tidal",
            KeyboardEffect::Starlight => "Starlight",
            KeyboardEffect::Dipp => "Color Dipp",
            KeyboardEffect::Rainbow => "Rainbow",
            KeyboardEffect::Wheel => "Wheel",
        }
    }

    pub fn from_name(name: &str) -> Option<KeyboardEffect> {
        Self::ALL.into_iter().find(|e| e.name() == name)
    }

    /// Plain ASCII identifier used when saving to the config file.
    pub fn key(self) -> &'static str {
        match self {
            KeyboardEffect::Off => "off",
            KeyboardEffect::Static => "static",
            KeyboardEffect::Breathing => "breathing",
            KeyboardEffect::Spectrum => "spectrum",
            KeyboardEffect::WaveRight => "wave_right",
            KeyboardEffect::WaveLeft => "wave_left",
            KeyboardEffect::Custom => "custom",
            KeyboardEffect::Fire => "fire",
            KeyboardEffect::Tidal => "tidal",
            KeyboardEffect::Starlight => "starlight",
            KeyboardEffect::Dipp => "dipp",
            KeyboardEffect::Rainbow => "rainbow",
            KeyboardEffect::Wheel => "wheel",
        }
    }

    pub fn from_key(key: &str) -> Option<KeyboardEffect> {
        Self::ALL.into_iter().find(|e| e.key() == key)
    }

    /// Whether the effect uses the picked color (Spectrum and Wave cycle through colors).
    pub fn uses_color(self) -> bool {
        matches!(self, KeyboardEffect::Static | KeyboardEffect::Breathing)
    }
}

const KBD_VARSTORE: u8 = 0x01;
/// The keyboard backlight LED, the same id `set_keyboard_brightness` uses ([1, 5, level]).
const KBD_LED_BACKLIGHT: u8 = 0x05;
const KBD_WAVE_SPEED: u8 = 0x28;

/// How a two-color Static is laid out on the keyboard.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KeyboardSplit {
    #[default]
    LeftRight,
    TopBottom,
    Gradient,
    /// Breathing only: the two colors take turns on the whole keyboard (the firmware effect).
    Alternate,
}

impl KeyboardSplit {
    pub const ALL: [KeyboardSplit; 3] =
        [KeyboardSplit::LeftRight, KeyboardSplit::TopBottom, KeyboardSplit::Gradient];

    pub fn name(self) -> &'static str {
        match self {
            KeyboardSplit::LeftRight => "Left | Right",
            KeyboardSplit::TopBottom => "Top | Bottom",
            KeyboardSplit::Gradient => "Gradient",
            KeyboardSplit::Alternate => "Alternate",
        }
    }

    /// Stored in the config file; unknown values fall back to Left | Right.
    pub fn from_index(i: u8) -> KeyboardSplit {
        match i {
            1 => KeyboardSplit::TopBottom,
            2 => KeyboardSplit::Gradient,
            3 => KeyboardSplit::Alternate,
            _ => KeyboardSplit::LeftRight,
        }
    }

    pub fn index(self) -> u8 {
        match self {
            KeyboardSplit::LeftRight => 0,
            KeyboardSplit::TopBottom => 1,
            KeyboardSplit::Gradient => 2,
            KeyboardSplit::Alternate => 3,
        }
    }

    /// The layouts offered for Breathing, in the order of the setting `breath_index` (0 = Alternate).
    pub const BREATHING: [KeyboardSplit; 4] = [
        KeyboardSplit::Alternate,
        KeyboardSplit::LeftRight,
        KeyboardSplit::TopBottom,
        KeyboardSplit::Gradient,
    ];

    /// Setting stored for Breathing: 0 = Alternate, 1 = Left | Right, 2 = Top | Bottom, 3 = Gradient.
    pub fn breath_index(self) -> u8 {
        match self {
            KeyboardSplit::Alternate => 0,
            other => other.index() + 1,
        }
    }

    pub fn from_breath_index(i: u8) -> KeyboardSplit {
        match i {
            0 => KeyboardSplit::Alternate,
            n => KeyboardSplit::from_index(n - 1),
        }
    }
}

/// The colors of a keyboard effect. `secondary` makes Breathing alternate between two colors and
/// Static a two-color picture (see `KeyboardSplit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardColors {
    pub primary: [u8; 3],
    pub secondary: Option<[u8; 3]>,
    pub split: KeyboardSplit,
}

impl KeyboardColors {
    pub fn single(primary: [u8; 3]) -> Self {
        Self { primary, secondary: None, split: KeyboardSplit::default() }
    }
}

/// Arguments of the extended matrix effect command (class 0x0F, id 0x02).
fn keyboard_effect_args(effect: KeyboardEffect, rgb: [u8; 3], rgb2: Option<[u8; 3]>) -> Vec<u8> {
    let [r, g, b] = rgb;
    let (vs, led) = (KBD_VARSTORE, KBD_LED_BACKLIGHT);
    match effect {
        KeyboardEffect::Off
        | KeyboardEffect::Custom
        | KeyboardEffect::Fire
        | KeyboardEffect::Tidal
        | KeyboardEffect::Starlight
        | KeyboardEffect::Dipp
        | KeyboardEffect::Rainbow
        | KeyboardEffect::Wheel => {
            vec![vs, led, 0x00, 0, 0, 0]
        }
        KeyboardEffect::Static => vec![vs, led, 0x01, 0, 0, 0x01, r, g, b],
        KeyboardEffect::Breathing => match rgb2 {
            Some([r2, g2, b2]) => {
                vec![vs, led, 0x02, 0x02, 0, 0x02, r, g, b, r2, g2, b2]
            }
            None => vec![vs, led, 0x02, 0x01, 0, 0x01, r, g, b],
        },
        KeyboardEffect::Spectrum => vec![vs, led, 0x03, 0, 0, 0],
        KeyboardEffect::WaveRight => vec![vs, led, 0x04, 0x01, KBD_WAVE_SPEED, 0],
        KeyboardEffect::WaveLeft => vec![vs, led, 0x04, 0x02, KBD_WAVE_SPEED, 0],
    }
}

/// Arguments of the older "standard matrix effect" command (class 0x03, id 0x0A).
fn keyboard_effect_args_standard(
    effect: KeyboardEffect,
    rgb: [u8; 3],
    rgb2: Option<[u8; 3]>,
) -> Vec<u8> {
    let [r, g, b] = rgb;
    match effect {
        KeyboardEffect::Off
        | KeyboardEffect::Custom
        | KeyboardEffect::Fire
        | KeyboardEffect::Tidal
        | KeyboardEffect::Starlight
        | KeyboardEffect::Dipp
        | KeyboardEffect::Rainbow
        | KeyboardEffect::Wheel => vec![0x00],
        KeyboardEffect::Static => vec![0x06, r, g, b],
        KeyboardEffect::Breathing => match rgb2 {
            Some([r2, g2, b2]) => vec![0x03, 0x02, r, g, b, r2, g2, b2],
            None => vec![0x03, 0x01, r, g, b],
        },
        KeyboardEffect::Spectrum => vec![0x04],
        KeyboardEffect::WaveRight => vec![0x01, 0x01],
        KeyboardEffect::WaveLeft => vec![0x01, 0x02],
    }
}

/// Transaction id that newer Razer firmware expects on lighting commands.
const KBD_TRANSACTION_ID: u8 = 0x1F;

/// How a keyboard effect is sent. The defaults are the most likely to work; the other
/// combinations exist so a model that ignores them can be tried from Options > Advanced.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyboardEffectOptions {
    /// Use the older standard matrix effect command (0x030A) instead of the extended one (0x0F02).
    pub legacy: bool,
    /// Use a random transaction id (like the other commands) instead of the fixed 0x1F.
    pub random_id: bool,
    /// Send an "Off" effect first for Static / Breathing, for firmware that ignores a new color
    /// while the same effect is already showing.
    pub clear_first: bool,
    /// Paint Static as a custom frame (every key set to the color) instead of the firmware's
    /// Static effect, which ignores the color on some laptops.
    pub static_frame: bool,
}

/// Custom frame size: 6 rows of 15 keys (what razer-laptop-control uses).
const KBD_FRAME_ROWS: usize = crate::keymap::ROWS;
const KBD_FRAME_KEYS: usize = crate::keymap::COLS;

/// First key column that gets the second color in "Left | Right". The keyboard's right half is
/// physically wider than an even 7/8 split of the 15 columns, so the right color starts earlier
/// (hardware-tested on the Blade 16 2023: an even 8/7 split looked short on the right, two
/// columns earlier overshot, one column earlier is right).
const KBD_SPLIT_COL: usize = 7;
/// "Gradient": pure first color up to this column, pure second color from `KBD_RAMP_END` on,
/// with a smooth, brightness-weighted blend in between.
const KBD_RAMP_START: f32 = 2.0;
const KBD_RAMP_END: f32 = 12.0;

/// Smoothstep from 0 (at `start`) to 1 (at `end`): holds both ends, so neither color fades out.
fn ramp(pos: f32, start: f32, end: f32) -> f32 {
    let x = ((pos - start) / (end - start)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// How bright an LED color looks. Green is far brighter to the eye than red or blue, so a plain
/// RGB mix lets a green-heavy color (cyan, yellow) win visually after only a little blend.
fn luma(r: u8, g: u8, b: u8) -> f32 {
    (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32).max(20.0)
}

/// Mix weight of the second color at gradient position `p` (0 = first color, 1 = second),
/// compensated for how bright each color looks: the brighter one is mixed in more slowly, so
/// both colors keep a visible share of the gradient (works for any pair of colors).
fn weighted_mix(luma_a: f32, luma_b: f32, p: f32) -> f32 {
    let r = (luma_b / luma_a).powf(0.75);
    p / (p + (1.0 - p) * r)
}

fn blend(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [mix(a[0], b[0]), mix(a[1], b[1]), mix(a[2], b[2])]
}

/// Color of one key in the custom frame.
fn frame_color(colors: &KeyboardColors, row: usize, key: usize) -> [u8; 3] {
    let a = colors.primary;
    let Some(b) = colors.secondary else {
        return a;
    };
    match colors.split {
        KeyboardSplit::LeftRight | KeyboardSplit::Alternate => {
            if key < KBD_SPLIT_COL {
                a
            } else {
                b
            }
        }
        KeyboardSplit::TopBottom => {
            if row < KBD_FRAME_ROWS / 2 {
                a
            } else {
                b
            }
        }
        KeyboardSplit::Gradient => {
            let p = ramp(key as f32, KBD_RAMP_START, KBD_RAMP_END);
            let t = weighted_mix(luma(a[0], a[1], a[2]), luma(b[0], b[1], b[2]), p);
            blend(a, b, t)
        }
    }
}

/// Arguments of one custom-frame row (class 0x03 id 0x0B): `color_of(key)` for each column.
fn frame_row_args(row: usize, color_of: impl Fn(usize) -> [u8; 3]) -> Vec<u8> {
    let mut args = vec![0u8; 7 + 3 * KBD_FRAME_KEYS];
    args[0] = 0xff;
    args[1] = row as u8;
    args[2] = 0x00; // first column
    args[3] = 0x0f; // last column
    for key in 0..KBD_FRAME_KEYS {
        let c = color_of(key);
        args[7 + 3 * key..10 + 3 * key].copy_from_slice(&c);
    }
    args
}

/// Paints the keyboard with a custom frame (class 0x03 ids 0x0B and 0x0A): one packet per row,
/// then the packet that shows the frame.
pub fn set_keyboard_grid(device: &Device, grid: &KeyGrid) -> Result<()> {
    paint_grid(device, grid, false)
}

/// Like `set_keyboard_grid`, but with the quick, single-attempt exchange (for animation frames,
/// where a dropped frame does not matter but a slow one does).
pub fn set_keyboard_grid_quick(device: &Device, grid: &KeyGrid) -> Result<()> {
    paint_grid(device, grid, true)
}

fn paint_grid(device: &Device, grid: &KeyGrid, quick: bool) -> Result<()> {
    let send = |packet: Packet| if quick { device.send_quick(packet) } else { device.send(packet) };
    for (row, cells) in grid.iter().enumerate() {
        send(
            Packet::new(0x030b, &frame_row_args(row, |key| cells[key])).with_id(KBD_TRANSACTION_ID),
        )?;
    }
    // effect 0x05 = custom frame, 0x00 = do not store
    send(Packet::new(0x030a, &[0x05, 0x00]).with_id(KBD_TRANSACTION_ID))?;
    Ok(())
}

/// The two-color picture (or plain color) of Static / split Breathing at full brightness.
fn set_keyboard_static_frame(device: &Device, colors: &KeyboardColors) -> Result<()> {
    set_keyboard_grid(device, &breathing_grid(colors, 1.0))
}

/// Seconds one software-Breathing cycle (fade up and down) takes at the standard speed.
pub const BREATH_PERIOD_SECS: f32 = 3.6;

/// Speeds are percentages of the standard speed: 100 is how the effect goes by default (for plain
/// Breathing, the firmware's own speed), 50 is half as fast, 200 twice as fast.
pub const STANDARD_SPEED: u8 = 100;
/// The slowest and the fastest speed the sliders give.
pub const SPEED_MIN: u8 = 25;
pub const SPEED_MAX: u8 = 250;

/// A speed (percent) as a factor to multiply time by (out-of-range values are clamped).
pub fn speed_factor(speed: u8) -> f32 {
    f32::from(speed.clamp(SPEED_MIN, SPEED_MAX)) / 100.0
}

/// A speed from before the sliders: the index of the old four buttons (Standard, Slow, Fast,
/// Faster) becomes the percent that was closest to it; any other number is a percent already.
pub fn speed_from_saved(value: u8) -> u8 {
    match value {
        0 => STANDARD_SPEED,
        1 => 60,
        2 => 160,
        3 => 250,
        v => v.clamp(SPEED_MIN, SPEED_MAX),
    }
}

/// The cycle length of Breathing at `speed` (a percent).
pub fn breath_period(speed: u8) -> f32 {
    BREATH_PERIOD_SECS / speed_factor(speed)
}

/// Brightness (0.0 to 1.0) of software Breathing `t` seconds after it started at the standard
/// speed.
pub fn breath_level(t: f32) -> f32 {
    breath_level_with(t, BREATH_PERIOD_SECS)
}

/// Brightness (0.0 to 1.0) of software Breathing `t` seconds after it started, with one cycle
/// taking `period` seconds: a smooth fade up and down, a little more time spent dim than bright
/// (like the firmware effect).
pub fn breath_level_with(t: f32, period: f32) -> f32 {
    let period = period.max(0.2);
    let phase = (t / period).fract();
    let s = 0.5 - 0.5 * (phase * std::f32::consts::TAU).cos();
    s.powf(1.5)
}

/// Which fade-in/out cycle (0, 1, 2, ...) software Breathing is in `t` seconds after it started.
pub fn breath_cycle(t: f32, period: f32) -> u32 {
    (t / period.max(0.2)).floor().max(0.0) as u32
}

/// The colors to draw in `cycle`: the Alternate layout shows one color per cycle (like the
/// firmware effect), everything else draws `colors` as they are.
pub fn breath_cycle_colors(colors: &KeyboardColors, cycle: u32) -> KeyboardColors {
    match (colors.secondary, colors.split) {
        (Some(second), KeyboardSplit::Alternate) => {
            KeyboardColors::single(if cycle % 2 == 0 { colors.primary } else { second })
        }
        _ => *colors,
    }
}

/// The keyboard picture for `colors`, with every color scaled to `level` (0.0 = dark, 1.0 = full).
pub fn breathing_grid(colors: &KeyboardColors, level: f32) -> KeyGrid {
    let level = level.clamp(0.0, 1.0);
    let scale = |v: u8| (v as f32 * level).round() as u8;
    let mut grid = [[[0u8; 3]; KBD_FRAME_KEYS]; KBD_FRAME_ROWS];
    for (row, cells) in grid.iter_mut().enumerate() {
        for (key, cell) in cells.iter_mut().enumerate() {
            let c = frame_color(colors, row, key);
            *cell = [scale(c[0]), scale(c[1]), scale(c[2])];
        }
    }
    grid
}

/// True when Breathing has two colors laid out as halves / a gradient. The firmware cannot do
/// that, so the app animates it (a frame at a time) instead of sending the Breathing effect.
pub fn is_split_breathing(effect: KeyboardEffect, colors: &KeyboardColors) -> bool {
    effect == KeyboardEffect::Breathing
        && colors.secondary.is_some()
        && colors.split != KeyboardSplit::Alternate
}

/// True when the app has to draw Breathing itself: a split layout, or any non-standard speed (the
/// firmware effect has a fixed speed).
pub fn is_software_breathing(effect: KeyboardEffect, colors: &KeyboardColors, speed: u8) -> bool {
    effect == KeyboardEffect::Breathing && (speed != STANDARD_SPEED || is_split_breathing(effect, colors))
}

/// Lights exactly the keys of `layout` (every other key gets its background color). `shift`
/// moves the whole map by that many columns, see `KeyLayout::grid`.
pub fn set_keyboard_layout(device: &Device, layout: &KeyLayout, shift: i8) -> Result<()> {
    set_keyboard_grid(device, &layout.grid(shift))
}

fn keyboard_effect_packet(
    effect: KeyboardEffect,
    rgb: [u8; 3],
    rgb2: Option<[u8; 3]>,
    opts: KeyboardEffectOptions,
) -> Packet {
    let packet = if opts.legacy {
        Packet::new(0x030a, &keyboard_effect_args_standard(effect, rgb, rgb2))
    } else {
        Packet::new(0x0f02, &keyboard_effect_args(effect, rgb, rgb2))
    };
    if opts.random_id {
        packet
    } else {
        packet.with_id(KBD_TRANSACTION_ID)
    }
}

/// Sets the laptop keyboard's lighting effect. Brightness is separate (`set_keyboard_brightness`).
pub fn set_keyboard_effect(
    device: &Device,
    effect: KeyboardEffect,
    colors: KeyboardColors,
    opts: KeyboardEffectOptions,
) -> Result<()> {
    ensure!(
        effect != KeyboardEffect::Custom,
        "Custom Keys is painted with set_keyboard_layout, not as an effect"
    );
    ensure!(
        !matches!(
            effect,
            KeyboardEffect::Fire
                | KeyboardEffect::Tidal
                | KeyboardEffect::Starlight
                | KeyboardEffect::Dipp
                | KeyboardEffect::Rainbow
                | KeyboardEffect::Wheel
        ),
        "Fire, Tidal, Starlight, Color Dipp, Rainbow and Wheel are drawn frame by frame by the app, not sent as effects"
    );
    let rgb = colors.primary;
    let two_color_static = effect == KeyboardEffect::Static && colors.secondary.is_some();
    if effect == KeyboardEffect::Static
        && (opts.static_frame || two_color_static)
        && set_keyboard_static_frame(device, &colors).is_ok()
    {
        return Ok(());
    }
    if opts.clear_first && effect.uses_color() {
        device.send(keyboard_effect_packet(KeyboardEffect::Off, rgb, None, opts))?;
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let second = if effect == KeyboardEffect::Breathing { colors.secondary } else { None };
    device.send(keyboard_effect_packet(effect, rgb, second, opts))?;
    Ok(())
}

#[cfg(test)]
mod keyboard_effect_tests {
    use super::*;

    #[test]
    fn static_effect_layout() {
        let args = keyboard_effect_args(KeyboardEffect::Static, [1, 2, 3], None);
        assert_eq!(args, vec![1, 5, 1, 0, 0, 1, 1, 2, 3]);
    }

    #[test]
    fn breathing_effect_layout() {
        let args = keyboard_effect_args(KeyboardEffect::Breathing, [9, 8, 7], None);
        assert_eq!(args, vec![1, 5, 2, 1, 0, 1, 9, 8, 7]);
    }

    #[test]
    fn dual_breathing_layout() {
        let args = keyboard_effect_args(KeyboardEffect::Breathing, [1, 2, 3], Some([4, 5, 6]));
        assert_eq!(args, vec![1, 5, 2, 2, 0, 2, 1, 2, 3, 4, 5, 6]);
        let legacy =
            keyboard_effect_args_standard(KeyboardEffect::Breathing, [1, 2, 3], Some([4, 5, 6]));
        assert_eq!(legacy, vec![3, 2, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn frame_row_layout() {
        let colors = KeyboardColors::single([1, 2, 3]);
        let args = frame_row_args(3, |key| frame_color(&colors, 3, key));
        assert_eq!(args.len(), 52);
        assert_eq!(&args[..8], &[0xff, 3, 0, 0x0f, 0, 0, 0, 1]);
        assert_eq!(&args[49..], &[1, 2, 3]);
    }

    #[test]
    fn split_layouts() {
        let a = [255, 0, 0];
        let b = [0, 0, 255];
        let mut colors =
            KeyboardColors { primary: a, secondary: Some(b), split: KeyboardSplit::LeftRight };
        assert_eq!(frame_color(&colors, 0, 0), a);
        assert_eq!(frame_color(&colors, 5, KBD_SPLIT_COL - 1), a);
        assert_eq!(frame_color(&colors, 0, KBD_SPLIT_COL), b);
        assert_eq!(frame_color(&colors, 5, 14), b);
        colors.split = KeyboardSplit::TopBottom;
        assert_eq!(frame_color(&colors, 2, 14), a);
        assert_eq!(frame_color(&colors, 3, 0), b);
        colors.split = KeyboardSplit::Gradient;
        assert_eq!(frame_color(&colors, 0, 0), a);
        assert_eq!(frame_color(&colors, 0, 14), b);
        // both ends hold their color; the blend sits in between and is symmetric about the middle
        assert_eq!(frame_color(&colors, 0, 2), a);
        assert_eq!(frame_color(&colors, 0, 12), b);
        let mid = frame_color(&colors, 0, 5);
        assert!(mid[0] > 0 && mid[0] < 255 && mid[2] > 0 && mid[2] < 255);
        // magenta -> cyan: cyan looks much brighter, so the middle of the gradient stays mostly
        // magenta (red channel still above green) instead of washing out to white
        colors.primary = [255, 0, 255];
        colors.secondary = Some([0, 255, 255]);
        let c = frame_color(&colors, 0, 7);
        assert!(c[0] > c[1]);
        colors.primary = a;
        colors.secondary = None;
        assert_eq!(frame_color(&colors, 3, 9), a);
    }

    #[test]
    fn split_index_round_trip() {
        for s in KeyboardSplit::ALL {
            assert_eq!(KeyboardSplit::from_index(s.index()), s);
        }
        assert_eq!(KeyboardSplit::from_index(200), KeyboardSplit::LeftRight);
        assert_eq!(
            KeyboardSplit::from_index(KeyboardSplit::Alternate.index()),
            KeyboardSplit::Alternate
        );
    }

    #[test]
    fn breathing_layout_setting_round_trip() {
        for (i, split) in KeyboardSplit::BREATHING.iter().enumerate() {
            assert_eq!(split.breath_index() as usize, i);
            assert_eq!(KeyboardSplit::from_breath_index(i as u8), *split);
        }
        assert_eq!(KeyboardSplit::from_breath_index(0), KeyboardSplit::Alternate);
        assert_eq!(KeyboardSplit::from_breath_index(1), KeyboardSplit::LeftRight);
        assert_eq!(KeyboardSplit::from_breath_index(3), KeyboardSplit::Gradient);
        // out of range falls back to the first real layout, never panics
        assert_eq!(KeyboardSplit::from_breath_index(99), KeyboardSplit::LeftRight);
    }

    #[test]
    fn split_breathing_is_only_for_two_colors_with_a_layout() {
        let mut colors = KeyboardColors {
            primary: [255, 0, 0],
            secondary: Some([0, 0, 255]),
            split: KeyboardSplit::LeftRight,
        };
        assert!(is_split_breathing(KeyboardEffect::Breathing, &colors));
        assert!(!is_split_breathing(KeyboardEffect::Static, &colors));
        colors.split = KeyboardSplit::Alternate;
        assert!(!is_split_breathing(KeyboardEffect::Breathing, &colors));
        colors.split = KeyboardSplit::Gradient;
        colors.secondary = None;
        assert!(!is_split_breathing(KeyboardEffect::Breathing, &colors));
    }

    #[test]
    fn breath_level_swings_between_dark_and_full() {
        assert!(breath_level(0.0) < 0.001);
        let peak = breath_level(BREATH_PERIOD_SECS / 2.0);
        assert!(peak > 0.999 && peak <= 1.0);
        let mut previous = 0.0;
        for step in 1..=18 {
            let level = breath_level(BREATH_PERIOD_SECS / 2.0 * step as f32 / 18.0);
            assert!(level >= previous, "fade up must not dip");
            previous = level;
        }
        assert!(breath_level(BREATH_PERIOD_SECS * 3.0 + 0.5) < 1.0);
    }

    #[test]
    fn old_speed_choices_become_percents() {
        assert_eq!(speed_from_saved(0), STANDARD_SPEED);
        assert!(speed_from_saved(1) < STANDARD_SPEED);
        assert!(speed_from_saved(2) > STANDARD_SPEED);
        assert!(speed_from_saved(3) > speed_from_saved(2));
        assert_eq!(speed_from_saved(120), 120);
        assert_eq!(speed_from_saved(255), SPEED_MAX);
        assert_eq!(speed_from_saved(10), SPEED_MIN);
        assert_eq!(speed_factor(50), 0.5);
        assert_eq!(speed_factor(0), 0.25);
    }

    #[test]
    fn breath_speeds_change_the_cycle_length() {
        assert_eq!(breath_period(STANDARD_SPEED), BREATH_PERIOD_SECS);
        assert!(breath_period(50) > breath_period(STANDARD_SPEED));
        assert!(breath_period(160) < breath_period(STANDARD_SPEED));
        assert!(breath_period(250) < breath_period(160));
        assert_eq!(breath_period(200), BREATH_PERIOD_SECS / 2.0);
        // out of range is clamped
        assert_eq!(breath_period(1), breath_period(SPEED_MIN));
        assert_eq!(breath_period(255), breath_period(SPEED_MAX));
        // half a cycle is the peak whatever the speed
        for speed in [SPEED_MIN, 60, STANDARD_SPEED, 160, SPEED_MAX] {
            let p = breath_period(speed);
            assert!(breath_level_with(0.0, p) < 0.001);
            assert!(breath_level_with(p / 2.0, p) > 0.99);
        }
        assert_eq!(breath_level(1.0), breath_level_with(1.0, BREATH_PERIOD_SECS));
        assert_eq!(breath_cycle(0.0, 2.0), 0);
        assert_eq!(breath_cycle(1.99, 2.0), 0);
        assert_eq!(breath_cycle(2.0, 2.0), 1);
        assert_eq!(breath_cycle(-5.0, 2.0), 0);
    }

    #[test]
    fn alternate_breathing_swaps_the_color_every_cycle() {
        let two = KeyboardColors {
            primary: [255, 0, 0],
            secondary: Some([0, 0, 255]),
            split: KeyboardSplit::Alternate,
        };
        assert_eq!(breath_cycle_colors(&two, 0), KeyboardColors::single([255, 0, 0]));
        assert_eq!(breath_cycle_colors(&two, 1), KeyboardColors::single([0, 0, 255]));
        assert_eq!(breath_cycle_colors(&two, 2), KeyboardColors::single([255, 0, 0]));
        let halves = KeyboardColors { split: KeyboardSplit::LeftRight, ..two };
        assert_eq!(breath_cycle_colors(&halves, 1), halves);
        let single = KeyboardColors::single([1, 2, 3]);
        assert_eq!(breath_cycle_colors(&single, 1), single);
    }

    #[test]
    fn software_breathing_is_used_for_layouts_and_custom_speeds() {
        let single = KeyboardColors::single([1, 2, 3]);
        let alternate = KeyboardColors {
            secondary: Some([4, 5, 6]),
            split: KeyboardSplit::Alternate,
            ..single
        };
        let halves = KeyboardColors { split: KeyboardSplit::LeftRight, ..alternate };
        assert!(!is_software_breathing(KeyboardEffect::Breathing, &single, STANDARD_SPEED));
        assert!(!is_software_breathing(KeyboardEffect::Breathing, &alternate, STANDARD_SPEED));
        assert!(is_software_breathing(KeyboardEffect::Breathing, &halves, STANDARD_SPEED));
        assert!(is_software_breathing(KeyboardEffect::Breathing, &single, 160));
        assert!(is_software_breathing(KeyboardEffect::Breathing, &alternate, 60));
        assert!(!is_software_breathing(KeyboardEffect::Static, &single, 160));
    }

    #[test]
    fn breathing_grid_scales_every_color() {
        let colors = KeyboardColors {
            primary: [200, 100, 50],
            secondary: Some([0, 0, 255]),
            split: KeyboardSplit::LeftRight,
        };
        let full = breathing_grid(&colors, 1.0);
        assert_eq!(full[0][0], [200, 100, 50]);
        assert_eq!(full[0][14], [0, 0, 255]);
        let half = breathing_grid(&colors, 0.5);
        assert_eq!(half[0][0], [100, 50, 25]);
        assert_eq!(half[5][14], [0, 0, 128]);
        let dark = breathing_grid(&colors, -3.0);
        assert!(dark.iter().flatten().all(|c| *c == [0, 0, 0]));
        let over = breathing_grid(&colors, 7.0);
        assert_eq!(over, full);
    }

    #[test]
    fn custom_keys_is_not_a_firmware_effect() {
        assert_eq!(KeyboardEffect::Custom.name(), "Custom Keys");
        assert_eq!(KeyboardEffect::from_key("custom"), Some(KeyboardEffect::Custom));
        assert!(!KeyboardEffect::Custom.uses_color());
        assert!(KeyboardEffect::ALL.contains(&KeyboardEffect::Custom));
    }

    #[test]
    fn fire_is_drawn_by_the_app_and_is_not_a_firmware_effect() {
        assert_eq!(KeyboardEffect::Fire.name(), "Fire");
        assert_eq!(KeyboardEffect::from_key("fire"), Some(KeyboardEffect::Fire));
        assert!(!KeyboardEffect::Fire.uses_color());
        assert!(KeyboardEffect::ALL.contains(&KeyboardEffect::Fire));
    }

    #[test]
    fn tidal_is_drawn_by_the_app_and_is_not_a_firmware_effect() {
        assert_eq!(KeyboardEffect::Tidal.name(), "Tidal");
        assert_eq!(KeyboardEffect::from_key("tidal"), Some(KeyboardEffect::Tidal));
        assert!(!KeyboardEffect::Tidal.uses_color());
        assert!(KeyboardEffect::ALL.contains(&KeyboardEffect::Tidal));
    }

    #[test]
    fn rainbow_is_drawn_by_the_app_and_is_not_a_firmware_effect() {
        assert_eq!(KeyboardEffect::Rainbow.name(), "Rainbow");
        assert_eq!(KeyboardEffect::from_key("rainbow"), Some(KeyboardEffect::Rainbow));
        assert!(!KeyboardEffect::Rainbow.uses_color());
        assert!(KeyboardEffect::ALL.contains(&KeyboardEffect::Rainbow));
    }

    #[test]
    fn color_dipp_is_drawn_by_the_app_and_is_not_a_firmware_effect() {
        assert_eq!(KeyboardEffect::Dipp.name(), "Color Dipp");
        assert_eq!(KeyboardEffect::from_key("dipp"), Some(KeyboardEffect::Dipp));
        assert_eq!(KeyboardEffect::from_name("Color Dipp"), Some(KeyboardEffect::Dipp));
        assert!(!KeyboardEffect::Dipp.uses_color());
        assert!(KeyboardEffect::ALL.contains(&KeyboardEffect::Dipp));
    }

    #[test]
    fn the_wheel_is_drawn_by_the_app_and_is_not_a_firmware_effect() {
        assert_eq!(KeyboardEffect::Wheel.name(), "Wheel");
        assert_eq!(KeyboardEffect::from_key("wheel"), Some(KeyboardEffect::Wheel));
        assert_eq!(KeyboardEffect::from_name("Wheel"), Some(KeyboardEffect::Wheel));
        assert!(!KeyboardEffect::Wheel.uses_color());
        assert!(KeyboardEffect::ALL.contains(&KeyboardEffect::Wheel));
        // it is before Off, which stays last
        assert_eq!(KeyboardEffect::ALL.last(), Some(&KeyboardEffect::Off));
        // sending it as a firmware effect is refused, and its packets are the empty effect
        let device = crate::device::Device::default();
        let colors = KeyboardColors { primary: [1, 2, 3], secondary: None, split: KeyboardSplit::default() };
        let opts = KeyboardEffectOptions { legacy: false, random_id: false, clear_first: false, static_frame: false };
        assert!(set_keyboard_effect(&device, KeyboardEffect::Wheel, colors, opts).is_err());
        assert!(device.sent.lock().unwrap().is_empty());
        assert_eq!(keyboard_effect_args_standard(KeyboardEffect::Wheel, [0; 3], None), vec![0x00]);
    }

    #[test]
    fn starlight_is_drawn_by_the_app_and_is_not_a_firmware_effect() {
        assert_eq!(KeyboardEffect::Starlight.name(), "Starlight");
        assert_eq!(KeyboardEffect::from_key("starlight"), Some(KeyboardEffect::Starlight));
        assert!(!KeyboardEffect::Starlight.uses_color());
        assert!(KeyboardEffect::ALL.contains(&KeyboardEffect::Starlight));
    }

    #[test]
    fn standard_effect_layout() {
        assert_eq!(
            keyboard_effect_args_standard(KeyboardEffect::Static, [1, 2, 3], None),
            vec![6, 1, 2, 3]
        );
        assert_eq!(
            keyboard_effect_args_standard(KeyboardEffect::Breathing, [1, 2, 3], None),
            vec![3, 1, 1, 2, 3]
        );
        assert_eq!(keyboard_effect_args_standard(KeyboardEffect::Spectrum, [0; 3], None), vec![4]);
        assert_eq!(
            keyboard_effect_args_standard(KeyboardEffect::WaveLeft, [0; 3], None),
            vec![1, 2]
        );
    }

    #[test]
    fn names_round_trip() {
        for e in KeyboardEffect::ALL {
            assert_eq!(KeyboardEffect::from_name(e.name()), Some(e));
            assert_eq!(KeyboardEffect::from_key(e.key()), Some(e));
        }
        assert!(KeyboardEffect::from_name("nope").is_none());
    }
}
