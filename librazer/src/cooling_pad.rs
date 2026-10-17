//! Razer Laptop Cooling Pad (USB 1532:0F43): fan speed and Chroma lighting over HID.
//!
//! The pad is its own USB device, separate from the laptop's embedded controller. It exposes a
//! fan interface (91-byte feature report) and a Chroma interface (90-byte feature report).
//!
//! The report layouts follow the cooling pad support in Robak08/r-helper (MIT), itself a fork of
//! Fatalution/r-helper. The Chroma packet layout is the common "extended matrix effect" format
//! documented by the OpenRazer project.

use anyhow::{bail, ensure, Context, Result};
use hidapi::{HidApi, HidDevice};
use std::ffi::CString;
use std::thread;
use std::time::Duration;

pub const RAZER_VID: u16 = 0x1532;
pub const COOLING_PAD_PID: u16 = 0x0F43;

pub const MIN_RPM: u16 = 500;
pub const MAX_RPM: u16 = 3200;
pub const RPM_STEP: u16 = 50;

/// Fan speed in percent of the pad's maximum, as the nearest valid RPM (multiple of 50, within the
/// pad's range). 15% is the lowest percentage that maps above the pad's minimum speed.
pub fn rpm_for_percent(pct: u8) -> u16 {
    let raw = pct.min(100) as u32 * MAX_RPM as u32 / 100;
    let stepped = ((raw + RPM_STEP as u32 / 2) / RPM_STEP as u32) * RPM_STEP as u32;
    (stepped as u16).clamp(MIN_RPM, MAX_RPM)
}

/// The nearest 5% step for an RPM value (inverse of [`rpm_for_percent`], for display).
pub fn percent_for_rpm(rpm: u16) -> u8 {
    let pct = rpm.min(MAX_RPM) as u32 * 100 / MAX_RPM as u32;
    (((pct + 2) / 5) * 5).min(100) as u8
}

/// Lighting effects the pad supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadLightingMode {
    Off,
    Static,
    Breathing,
}

impl PadLightingMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Static => "Static",
            Self::Breathing => "Breathing",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "Off" => Some(Self::Off),
            "Static" => Some(Self::Static),
            "Breathing" => Some(Self::Breathing),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

// ---------------------------------------------------------------------------------------------
// Fan (91-byte feature report, report id 0)
// ---------------------------------------------------------------------------------------------

const FAN_REPORT_LEN: usize = 91;
const FAN_REPORT_ID: u8 = 0x00;
const FAN_REPORT_CODE: usize = 8;
const FAN_SUB_VER: usize = 9;
const FAN_CURVE_ID: usize = 10;
const FAN_RPM_L: usize = 11;
const FAN_RPM_H: usize = 12;
const FAN_CHK_L: usize = 89;
const FAN_CHK_H: usize = 90;

/// Fixed payload template (bytes 1..=90, excluding the report id).
const FAN_HEADER: [u8; 90] = {
    let mut h = [0u8; 90];
    h[1] = 0x02;
    h[5] = 0x03;
    h[6] = 0x0D;
    h[7] = 0x10;
    h[8] = 0x01;
    h[9] = 0x02;
    h[10] = 0x36;
    h
};

fn fan_buffer(
    report_code: u8,
    sub_ver: u8,
    curve_id: u8,
    rpm: u16,
    chk_l: u8,
) -> [u8; FAN_REPORT_LEN] {
    let mut buf = [0u8; FAN_REPORT_LEN];
    buf[0] = FAN_REPORT_ID;
    buf[1..].copy_from_slice(&FAN_HEADER);
    buf[FAN_REPORT_CODE] = report_code;
    buf[FAN_SUB_VER] = sub_ver;
    buf[FAN_CURVE_ID] = curve_id;
    let raw = rpm / RPM_STEP;
    buf[FAN_RPM_L] = (raw & 0xFF) as u8;
    buf[FAN_RPM_H] = (raw >> 8) as u8;
    buf[FAN_CHK_L] = chk_l;
    buf[FAN_CHK_H] = 0x00;
    buf
}

/// True when this HID interface answers like the pad's fan interface.
fn probe_fan(device: &HidDevice) -> bool {
    let mut buf = [0u8; FAN_REPORT_LEN];
    buf[0] = FAN_REPORT_ID;
    if let Ok(size) = device.get_feature_report(&mut buf) {
        if size >= FAN_REPORT_LEN {
            return true;
        }
    }
    // Some interfaces accept writes before reads succeed (right after plug-in).
    device.send_feature_report(&buf).is_ok()
}

fn fan_set_rpm(device: &HidDevice, rpm: u16) -> Result<()> {
    ensure!((MIN_RPM..=MAX_RPM).contains(&rpm), "RPM must be between {} and {}", MIN_RPM, MAX_RPM);
    let rounded = ((rpm as f32 / RPM_STEP as f32).round() as u16) * RPM_STEP;
    let raw = rounded / RPM_STEP;
    let buf = fan_buffer(0x01, 0x01, 0x05, rounded, (raw & 0xFF) as u8 ^ 0x0B);
    device.send_feature_report(&buf).context("Failed to send cooling pad fan report")?;
    Ok(())
}

fn fan_off(device: &HidDevice) -> Result<()> {
    let buf = fan_buffer(0x10, 0x00, 0x06, 0, 0x18);
    device.send_feature_report(&buf).context("Failed to send cooling pad fan report")?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Chroma lighting (90-byte feature report, report id 0)
// ---------------------------------------------------------------------------------------------

const CHROMA_REPORT_LEN: usize = 90;
const STATUS_NEW: u8 = 0x00;
const STATUS_OK: u8 = 0x02;
const TRANSACTION_ID: u8 = 0x1F;
const VARSTORE: u8 = 0x01;
const ZERO_LED: u8 = 0x00;

/// XOR of bytes 2..=87 (OpenRazer convention).
fn chroma_crc(report: &[u8]) -> u8 {
    report[2..88].iter().fold(0u8, |acc, b| acc ^ b)
}

fn chroma_request(command_class: u8, command_id: u8, args: &[u8]) -> [u8; CHROMA_REPORT_LEN] {
    let n = args.len().min(80);
    let mut report = [0u8; CHROMA_REPORT_LEN];
    report[0] = STATUS_NEW;
    report[1] = TRANSACTION_ID;
    report[5] = n as u8;
    report[6] = command_class;
    report[7] = command_id;
    report[8..8 + n].copy_from_slice(&args[..n]);
    report[88] = chroma_crc(&report);
    report
}

/// Extended matrix effect: command class 0x0F, id 0x02.
fn effect_report(arg_size: u8, effect_id: u8) -> [u8; CHROMA_REPORT_LEN] {
    let mut args = [0u8; 80];
    args[0] = VARSTORE;
    args[1] = ZERO_LED;
    args[2] = effect_id;
    chroma_request(0x0F, 0x02, &args[..arg_size as usize])
}

fn finalize(mut report: [u8; CHROMA_REPORT_LEN]) -> [u8; CHROMA_REPORT_LEN] {
    report[88] = chroma_crc(&report);
    report
}

fn report_none() -> [u8; CHROMA_REPORT_LEN] {
    effect_report(0x06, 0x00)
}

fn report_static(rgb: Rgb) -> [u8; CHROMA_REPORT_LEN] {
    let mut report = effect_report(0x09, 0x01);
    report[8 + 5] = 0x01;
    report[8 + 6] = rgb.r;
    report[8 + 7] = rgb.g;
    report[8 + 8] = rgb.b;
    finalize(report)
}

fn report_breathing(rgb: Rgb) -> [u8; CHROMA_REPORT_LEN] {
    let mut report = effect_report(0x09, 0x02);
    report[8 + 3] = 0x01;
    report[8 + 5] = 0x01;
    report[8 + 6] = rgb.r;
    report[8 + 7] = rgb.g;
    report[8 + 8] = rgb.b;
    finalize(report)
}

fn report_set_brightness(brightness: u8) -> [u8; CHROMA_REPORT_LEN] {
    finalize(chroma_request(0x0F, 0x04, &[VARSTORE, ZERO_LED, brightness]))
}

fn report_get_brightness() -> [u8; CHROMA_REPORT_LEN] {
    finalize(chroma_request(0x0F, 0x84, &[VARSTORE, ZERO_LED]))
}

/// Sends one Chroma report and checks the reply status.
fn chroma_send(device: &HidDevice, report: &[u8; CHROMA_REPORT_LEN]) -> Result<Vec<u8>> {
    let mut send_buf = vec![0u8; 1 + CHROMA_REPORT_LEN];
    send_buf[1..].copy_from_slice(report);
    device.send_feature_report(&send_buf).context("Failed to send Chroma feature report")?;

    thread::sleep(Duration::from_millis(50));

    let mut recv_buf = vec![0u8; 1 + CHROMA_REPORT_LEN];
    let size =
        device.get_feature_report(&mut recv_buf).context("Failed to read Chroma feature report")?;
    ensure!(size > 1, "Chroma response too short: {}", size);

    let response = recv_buf[1..].to_vec();
    ensure!(
        response[0] == STATUS_OK || response[0] == STATUS_NEW,
        "Chroma device returned error status {}",
        response[0]
    );
    Ok(response)
}

fn probe_chroma(device: &HidDevice) -> bool {
    chroma_send(device, &report_get_brightness()).is_ok()
}

fn set_mode(device: &HidDevice, mode: PadLightingMode, rgb: Rgb) -> Result<()> {
    let report = match mode {
        PadLightingMode::Off => report_none(),
        PadLightingMode::Static => report_static(rgb),
        PadLightingMode::Breathing => report_breathing(rgb),
    };
    chroma_send(device, &report).context("Failed to set cooling pad lighting mode")?;
    Ok(())
}

fn set_brightness_raw(device: &HidDevice, brightness: u8) -> Result<()> {
    chroma_send(device, &report_set_brightness(brightness))
        .context("Failed to set cooling pad brightness")?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Device
// ---------------------------------------------------------------------------------------------

/// An opened cooling pad. Calls block (the Chroma reports include short sleeps), so use it from a
/// worker thread, not from the UI thread.
///
/// Only the fan interface is opened by [`CoolingPad::detect`]. The Chroma (lighting) interface is
/// opened on request by [`CoolingPad::enable_lighting`], so an app that does not want to touch the
/// pad's lights never even opens it and cannot disturb Razer Chroma / Synapse.
pub struct CoolingPad {
    fan: HidDevice,
    fan_path: CString,
    chroma: Option<HidDevice>,
}

struct PadEntry {
    path: CString,
    interface_number: i32,
}

fn pad_entries(api: &HidApi) -> Vec<PadEntry> {
    let mut entries: Vec<PadEntry> = api
        .device_list()
        .filter(|info| info.vendor_id() == RAZER_VID && info.product_id() == COOLING_PAD_PID)
        .map(|info| PadEntry {
            path: info.path().to_owned(),
            interface_number: info.interface_number(),
        })
        .collect();
    // Highest interface number first (the pad's control interfaces are the later ones).
    entries.sort_by_key(|e| std::cmp::Reverse(e.interface_number));
    entries
}

/// Cheap presence check (HID enumeration only, nothing is opened).
pub fn is_present() -> bool {
    HidApi::new().map(|api| !pad_entries(&api).is_empty()).unwrap_or(false)
}

impl CoolingPad {
    pub fn detect() -> Result<Self> {
        let api = HidApi::new().context("Failed to create hid api")?;
        let entries = pad_entries(&api);
        if entries.is_empty() {
            bail!("Razer Laptop Cooling Pad not found");
        }

        // Fan interface: the first one that answers a fan feature report.
        let mut fan: Option<(HidDevice, CString)> = None;
        for entry in &entries {
            if let Ok(device) = api.open_path(entry.path.as_c_str()) {
                if probe_fan(&device) {
                    fan = Some((device, entry.path.clone()));
                    break;
                }
            }
        }
        let (fan, fan_path) = fan.context("Failed to open the cooling pad fan interface")?;

        Ok(Self { fan, fan_path, chroma: None })
    }

    /// Opens the pad's Chroma interface (another interface that answers a Chroma query, else the
    /// fan one). Returns whether lighting is available. Does nothing if it is already open.
    pub fn enable_lighting(&mut self) -> bool {
        if self.chroma.is_some() {
            return true;
        }
        let api = match HidApi::new() {
            Ok(api) => api,
            Err(_) => return false,
        };
        let entries = pad_entries(&api);
        for entry in entries.iter().filter(|e| e.path != self.fan_path) {
            if let Ok(device) = api.open_path(entry.path.as_c_str()) {
                if probe_chroma(&device) {
                    self.chroma = Some(device);
                    return true;
                }
            }
        }
        if let Ok(device) = api.open_path(self.fan_path.as_c_str()) {
            if probe_chroma(&device) {
                self.chroma = Some(device);
                return true;
            }
        }
        false
    }

    /// Closes the Chroma interface again; the pad's lights are left entirely to other software.
    pub fn disable_lighting(&mut self) {
        self.chroma = None;
    }

    pub fn lighting_available(&self) -> bool {
        self.chroma.is_some()
    }

    /// False when the HID handle went stale (for example after the PC resumed from sleep).
    pub fn is_responsive(&self) -> bool {
        probe_fan(&self.fan)
    }

    pub fn set_fan_rpm(&self, rpm: u16) -> Result<()> {
        fan_set_rpm(&self.fan, rpm)
    }

    pub fn fan_off(&self) -> Result<()> {
        fan_off(&self.fan)
    }

    fn chroma(&self) -> Result<&HidDevice> {
        self.chroma.as_ref().context("Cooling pad lighting is not available")
    }

    /// Applies mode, color and brightness. The pad is reliable only when the current effect is
    /// cleared first, so this always clears before setting a new effect.
    pub fn apply_lighting(&self, mode: PadLightingMode, rgb: Rgb, brightness: u8) -> Result<()> {
        let chroma = self.chroma()?;
        if mode == PadLightingMode::Off {
            return set_mode(chroma, mode, rgb);
        }
        chroma_send(chroma, &report_none()).context("Failed to clear cooling pad lighting")?;
        thread::sleep(Duration::from_millis(50));
        set_mode(chroma, mode, rgb)?;
        thread::sleep(Duration::from_millis(50));
        set_brightness_raw(chroma, brightness)
    }

    /// Re-applies a color by cycling off, the other effect, then the target effect. The pad
    /// firmware ignores color-only updates within the same effect.
    pub fn apply_color_change(
        &self,
        mode: PadLightingMode,
        rgb: Rgb,
        brightness: u8,
    ) -> Result<()> {
        let chroma = self.chroma()?;
        let alternate = match mode {
            PadLightingMode::Off => return Ok(()),
            PadLightingMode::Static => PadLightingMode::Breathing,
            PadLightingMode::Breathing => PadLightingMode::Static,
        };
        set_mode(chroma, PadLightingMode::Off, rgb)?;
        thread::sleep(Duration::from_millis(100));
        set_mode(chroma, alternate, rgb)?;
        thread::sleep(Duration::from_millis(100));
        set_mode(chroma, mode, rgb)?;
        thread::sleep(Duration::from_millis(50));
        set_brightness_raw(chroma, brightness)
    }

    pub fn set_brightness(&self, brightness: u8) -> Result<()> {
        set_brightness_raw(self.chroma()?, brightness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fan_report_layout() {
        let buf = fan_buffer(0x01, 0x01, 0x05, 1650, ((1650 / RPM_STEP) as u8) ^ 0x0B);
        assert_eq!(buf.len(), FAN_REPORT_LEN);
        assert_eq!(buf[FAN_REPORT_CODE], 0x01);
        assert_eq!(buf[FAN_CURVE_ID], 0x05);
        assert_eq!(buf[FAN_RPM_L] as u16, 1650 / RPM_STEP);
    }

    #[test]
    fn percent_rpm_mapping() {
        assert_eq!(rpm_for_percent(100), 3200);
        assert_eq!(rpm_for_percent(15), 500);
        assert_eq!(rpm_for_percent(5), 500); // clamped to the pad's minimum
        assert_eq!(rpm_for_percent(50), 1600);
        for pct in (15u8..=100).step_by(5) {
            let rpm = rpm_for_percent(pct);
            assert!((MIN_RPM..=MAX_RPM).contains(&rpm));
            assert_eq!(rpm % RPM_STEP, 0);
        }
        assert_eq!(percent_for_rpm(3200), 100);
        assert_eq!(percent_for_rpm(1600), 50);
    }

    #[test]
    fn chroma_crc_is_xor_of_payload() {
        let r = report_set_brightness(0x80);
        assert_eq!(r[88], chroma_crc(&r));
        assert_eq!(r[6], 0x0F);
        assert_eq!(r[7], 0x04);
        assert_eq!(r[10], 0x80);
    }
}
