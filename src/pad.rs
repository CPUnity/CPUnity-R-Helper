//! Background worker for the optional Razer Laptop Cooling Pad (USB 1532:0F43).
//!
//! All HID traffic to the pad happens on this thread, so the UI never waits on it. The worker
//! notices when the pad is plugged in or removed and re-applies the last *fan* setting when the
//! pad appears again (or its handle went stale after sleep).
//!
//! Lighting is strictly opt-in: unless the user switched "Toggle App Lighting Control" on, the
//! pad's Chroma interface is never even opened. Even when it is on, lighting is only changed by an
//! explicit click, so Razer Chroma / Synapse keep control. The one exception is the separate
//! opt-in "Restore lighting at startup" switch: with it on (and the toggle on), the last lighting
//! the user applied is put back when the pad connects or reconnects.
use librazer::cooling_pad::{CoolingPad, PadLightingMode, Rgb};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// How often to look for the pad while it is not connected.
const DETECT_EVERY: Duration = Duration::from_secs(5);
/// How often to check that a connected pad still answers.
const PROBE_EVERY: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanTarget {
    Off,
    Rpm(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightTarget {
    pub mode: PadLightingMode,
    pub rgb: [u8; 3],
    /// Raw Chroma brightness, 0-255.
    pub brightness: u8,
}

/// The fan setting the user last chose. `None` = never touched, so the pad is left alone.
#[derive(Debug, Clone, Copy, Default)]
pub struct Desired {
    pub fan: Option<FanTarget>,
    /// The lighting the user last applied. Only used when "Restore lighting" is switched on.
    pub light: Option<LightTarget>,
}

impl Desired {
    pub fn from_config(cfg: &crate::config::Config) -> Self {
        let fan = match cfg.pad_fan.as_str() {
            "off" => Some(FanTarget::Off),
            "manual" => Some(FanTarget::Rpm(cfg.pad_rpm)),
            _ => None,
        };
        let light = PadLightingMode::from_name(&cfg.pad_last_mode).map(|mode| LightTarget {
            mode,
            rgb: cfg.pad_color,
            brightness: pct_to_raw(cfg.pad_brightness),
        });
        Self { fan, light }
    }
}

/// Brightness percent (0-100) to the raw Chroma value (0-255).
pub fn pct_to_raw(pct: u8) -> u8 {
    ((pct.min(100) as u16 * 255 + 50) / 100) as u8
}

#[derive(Debug, Clone, Copy)]
pub enum PadCmd {
    Fan(FanTarget),
    /// Let R-Helper control the pad's lights (opens / closes the pad's Chroma interface).
    LightingEnabled(bool),
    /// Switch "restore the last lighting when the pad connects" on / off.
    ResumeLighting(bool),
    /// Set mode, color and brightness (clears the current effect first).
    Light(LightTarget),
    /// Re-apply the color of the current effect (needs the pad's off / alternate / target cycle).
    LightColor(LightTarget),
    Brightness(u8),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PadStatus {
    pub present: bool,
    /// Lighting control is switched on and the pad's Chroma interface was found.
    pub lighting_available: bool,
}

#[derive(Default)]
struct Shared {
    status: Mutex<PadStatus>,
    /// `(is_error, message)` lines for the UI's status line.
    events: Mutex<Vec<(bool, String)>>,
}

pub struct Handle {
    tx: mpsc::Sender<PadCmd>,
    shared: Arc<Shared>,
}

impl Handle {
    pub fn spawn(desired: Desired, lighting_enabled: bool, resume_lighting: bool) -> Self {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Shared::default());
        let worker_shared = Arc::clone(&shared);
        let _ = thread::Builder::new()
            .name("cooling-pad".into())
            .spawn(move || run(rx, worker_shared, desired, lighting_enabled, resume_lighting));
        Self { tx, shared }
    }

    pub fn snapshot(&self) -> PadStatus {
        self.shared.status.lock().map(|s| *s).unwrap_or_default()
    }

    pub fn send(&self, cmd: PadCmd) {
        let _ = self.tx.send(cmd);
    }

    pub fn drain_events(&self) -> Vec<(bool, String)> {
        match self.shared.events.lock() {
            Ok(mut events) => std::mem::take(&mut *events),
            Err(_) => Vec::new(),
        }
    }
}

fn rgb(c: [u8; 3]) -> Rgb {
    Rgb { r: c[0], g: c[1], b: c[2] }
}

fn set_status(shared: &Shared, present: bool, lighting_available: bool) {
    if let Ok(mut s) = shared.status.lock() {
        s.present = present;
        s.lighting_available = lighting_available;
    }
}

fn push_event(shared: &Shared, is_error: bool, message: String) {
    crate::dlog::log(&format!("cooling pad: {}", message));
    if let Ok(mut events) = shared.events.lock() {
        events.push((is_error, message));
    }
}

fn apply_fan(pad: &CoolingPad, target: FanTarget) -> anyhow::Result<()> {
    match target {
        FanTarget::Off => pad.fan_off(),
        FanTarget::Rpm(rpm) => pad.set_fan_rpm(rpm),
    }
}

struct Worker {
    shared: Arc<Shared>,
    pad: Option<CoolingPad>,
    desired: Desired,
    lighting_enabled: bool,
    resume_lighting: bool,
}

impl Worker {
    /// Opens or closes the pad's Chroma interface to match the opt-in switch.
    fn sync_lighting(&mut self) {
        let enabled = self.lighting_enabled;
        let mut found = false;
        let mut missing = false;
        if let Some(pad) = self.pad.as_mut() {
            if enabled {
                found = pad.enable_lighting();
                missing = !found;
            } else {
                pad.disable_lighting();
            }
        }
        if self.pad.is_some() {
            set_status(&self.shared, true, found);
        }
        if missing {
            push_event(&self.shared, true, "Cooling pad lighting interface not found".to_string());
        }
    }

    fn handle(&mut self, cmd: PadCmd) {
        match cmd {
            PadCmd::Fan(target) => self.desired.fan = Some(target),
            PadCmd::LightingEnabled(on) => {
                self.lighting_enabled = on;
                self.sync_lighting();
                return;
            }
            PadCmd::ResumeLighting(on) => {
                self.resume_lighting = on;
                return;
            }
            PadCmd::Light(t) | PadCmd::LightColor(t) => self.desired.light = Some(t),
            PadCmd::Brightness(b) => {
                if let Some(light) = self.desired.light.as_mut() {
                    light.brightness = b;
                }
            }
        }

        let pad = match self.pad.as_ref() {
            Some(p) => p,
            None => return,
        };
        let result = match cmd {
            PadCmd::Fan(target) => apply_fan(pad, target),
            PadCmd::Light(t) => pad.apply_lighting(t.mode, rgb(t.rgb), t.brightness),
            PadCmd::LightColor(t) => pad.apply_color_change(t.mode, rgb(t.rgb), t.brightness),
            PadCmd::Brightness(b) => pad.set_brightness(b),
            PadCmd::LightingEnabled(_) | PadCmd::ResumeLighting(_) => return,
        };
        if let Err(e) = result {
            push_event(&self.shared, true, format!("Cooling pad: {}", e));
        }
    }

    /// Looks for the pad; on success re-applies the remembered fan setting. The lights are only
    /// restored when the user opted in to "Restore lighting at startup".
    fn try_connect(&mut self) -> bool {
        let found = match CoolingPad::detect() {
            Ok(found) => found,
            Err(_) => return false,
        };
        self.pad = Some(found);
        set_status(&self.shared, true, false);
        push_event(&self.shared, false, "Cooling pad connected".to_string());
        self.sync_lighting();

        if let (Some(fan), Some(pad)) = (self.desired.fan, self.pad.as_ref()) {
            if let Err(e) = apply_fan(pad, fan) {
                push_event(&self.shared, true, format!("Cooling pad fan: {}", e));
            }
        }

        if self.resume_lighting && self.lighting_enabled {
            if let (Some(light), Some(pad)) = (self.desired.light, self.pad.as_ref()) {
                if pad.lighting_available() {
                    if let Err(e) = pad.apply_lighting(light.mode, rgb(light.rgb), light.brightness)
                    {
                        push_event(&self.shared, true, format!("Cooling pad lighting: {}", e));
                    }
                }
            }
        }
        true
    }

    fn drop_pad(&mut self) {
        self.pad = None;
        set_status(&self.shared, false, false);
        push_event(&self.shared, false, "Cooling pad disconnected".to_string());
    }
}

fn run(
    rx: mpsc::Receiver<PadCmd>,
    shared: Arc<Shared>,
    desired: Desired,
    lighting_enabled: bool,
    resume_lighting: bool,
) {
    let mut worker = Worker { shared, pad: None, desired, lighting_enabled, resume_lighting };
    let mut last_detect: Option<Instant> = None;
    let mut last_probe = Instant::now();

    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(cmd) => {
                worker.handle(cmd);
                while let Ok(next) = rx.try_recv() {
                    worker.handle(next);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        if worker.pad.is_none() {
            let due = last_detect.map_or(true, |t| t.elapsed() >= DETECT_EVERY);
            if due {
                last_detect = Some(Instant::now());
                if worker.try_connect() {
                    last_probe = Instant::now();
                }
            }
        } else if last_probe.elapsed() >= PROBE_EVERY {
            last_probe = Instant::now();
            let alive = worker.pad.as_ref().map(|p| p.is_responsive()).unwrap_or(false);
            if !alive {
                worker.drop_pad();
                last_detect = None; // look again right away (the pad may just have re-enumerated)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brightness_percent_maps_to_raw_range() {
        assert_eq!(pct_to_raw(0), 0);
        assert_eq!(pct_to_raw(100), 255);
        assert_eq!(pct_to_raw(50), 128);
        assert_eq!(pct_to_raw(250), 255);
    }
}
