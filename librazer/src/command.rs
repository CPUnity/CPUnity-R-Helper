use crate::device::Device;
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
}

impl KeyboardEffect {
    pub const ALL: [KeyboardEffect; 6] = [
        KeyboardEffect::Static,
        KeyboardEffect::Breathing,
        KeyboardEffect::Spectrum,
        KeyboardEffect::WaveRight,
        KeyboardEffect::WaveLeft,
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

/// Arguments of the extended matrix effect command (class 0x0F, id 0x02).
fn keyboard_effect_args(effect: KeyboardEffect, rgb: [u8; 3]) -> Vec<u8> {
    let [r, g, b] = rgb;
    let (vs, led) = (KBD_VARSTORE, KBD_LED_BACKLIGHT);
    match effect {
        KeyboardEffect::Off => vec![vs, led, 0x00, 0, 0, 0],
        KeyboardEffect::Static => vec![vs, led, 0x01, 0, 0, 0x01, r, g, b],
        KeyboardEffect::Breathing => vec![vs, led, 0x02, 0x01, 0, 0x01, r, g, b],
        KeyboardEffect::Spectrum => vec![vs, led, 0x03, 0, 0, 0],
        KeyboardEffect::WaveRight => vec![vs, led, 0x04, 0x01, KBD_WAVE_SPEED, 0],
        KeyboardEffect::WaveLeft => vec![vs, led, 0x04, 0x02, KBD_WAVE_SPEED, 0],
    }
}

/// Arguments of the older "standard matrix effect" command (class 0x03, id 0x0A).
fn keyboard_effect_args_standard(effect: KeyboardEffect, rgb: [u8; 3]) -> Vec<u8> {
    let [r, g, b] = rgb;
    match effect {
        KeyboardEffect::Off => vec![0x00],
        KeyboardEffect::Static => vec![0x06, r, g, b],
        KeyboardEffect::Breathing => vec![0x03, 0x01, r, g, b],
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

/// Custom frame size used for a plain color: 6 rows of 15 keys (what razer-laptop-control uses).
const KBD_FRAME_ROWS: u8 = 6;
const KBD_FRAME_KEYS: usize = 15;

fn keyboard_frame_row_args(row: u8, rgb: [u8; 3]) -> Vec<u8> {
    let mut args = vec![0u8; 7 + 3 * KBD_FRAME_KEYS];
    args[0] = 0xff;
    args[1] = row;
    args[2] = 0x00; // first column
    args[3] = 0x0f; // last column
    for key in 0..KBD_FRAME_KEYS {
        args[7 + 3 * key..10 + 3 * key].copy_from_slice(&rgb);
    }
    args
}

/// Paints the whole keyboard one color with a custom frame (class 0x03 ids 0x0B and 0x0A).
fn set_keyboard_static_frame(device: &Device, rgb: [u8; 3]) -> Result<()> {
    for row in 0..KBD_FRAME_ROWS {
        device.send(
            Packet::new(0x030b, &keyboard_frame_row_args(row, rgb)).with_id(KBD_TRANSACTION_ID),
        )?;
    }
    // effect 0x05 = custom frame, 0x00 = do not store
    device.send(Packet::new(0x030a, &[0x05, 0x00]).with_id(KBD_TRANSACTION_ID))?;
    Ok(())
}

fn keyboard_effect_packet(
    effect: KeyboardEffect,
    rgb: [u8; 3],
    opts: KeyboardEffectOptions,
) -> Packet {
    let packet = if opts.legacy {
        Packet::new(0x030a, &keyboard_effect_args_standard(effect, rgb))
    } else {
        Packet::new(0x0f02, &keyboard_effect_args(effect, rgb))
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
    rgb: [u8; 3],
    opts: KeyboardEffectOptions,
) -> Result<()> {
    if effect == KeyboardEffect::Static
        && opts.static_frame
        && set_keyboard_static_frame(device, rgb).is_ok()
    {
        return Ok(());
    }
    if opts.clear_first && effect.uses_color() {
        device.send(keyboard_effect_packet(KeyboardEffect::Off, rgb, opts))?;
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    device.send(keyboard_effect_packet(effect, rgb, opts))?;
    Ok(())
}

#[cfg(test)]
mod keyboard_effect_tests {
    use super::*;

    #[test]
    fn static_effect_layout() {
        let args = keyboard_effect_args(KeyboardEffect::Static, [1, 2, 3]);
        assert_eq!(args, vec![1, 5, 1, 0, 0, 1, 1, 2, 3]);
    }

    #[test]
    fn breathing_effect_layout() {
        let args = keyboard_effect_args(KeyboardEffect::Breathing, [9, 8, 7]);
        assert_eq!(args, vec![1, 5, 2, 1, 0, 1, 9, 8, 7]);
    }

    #[test]
    fn frame_row_layout() {
        let args = keyboard_frame_row_args(3, [1, 2, 3]);
        assert_eq!(args.len(), 52);
        assert_eq!(&args[..8], &[0xff, 3, 0, 0x0f, 0, 0, 0, 1]);
        assert_eq!(&args[49..], &[1, 2, 3]);
    }

    #[test]
    fn standard_effect_layout() {
        assert_eq!(
            keyboard_effect_args_standard(KeyboardEffect::Static, [1, 2, 3]),
            vec![6, 1, 2, 3]
        );
        assert_eq!(
            keyboard_effect_args_standard(KeyboardEffect::Breathing, [1, 2, 3]),
            vec![3, 1, 1, 2, 3]
        );
        assert_eq!(keyboard_effect_args_standard(KeyboardEffect::Spectrum, [0; 3]), vec![4]);
        assert_eq!(keyboard_effect_args_standard(KeyboardEffect::WaveLeft, [0; 3]), vec![1, 2]);
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
