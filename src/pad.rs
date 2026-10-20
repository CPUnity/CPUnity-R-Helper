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
use librazer::command::{
    breath_cycle, breath_level_with, breath_period, speed_factor, STANDARD_SPEED,
};
use librazer::cooling_pad::{CoolingPad, PadColors, PadLightingMode, Rgb};
use librazer::fire::Fire;
use librazer::dipp::Dipp;
use librazer::rainbow::Rainbow;
use librazer::wheel::Wheel;
use librazer::spectrum::Spectrum;
use librazer::starlight::Starlight;
use librazer::tidal::Tidal;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// How often to look for the pad while it is not connected.
const DETECT_EVERY: Duration = Duration::from_secs(5);
/// How often to check that a connected pad still answers.
const PROBE_EVERY: Duration = Duration::from_secs(5);
/// Pause between two frames of the split Breathing animation (a frame itself takes a while).
const ANIM_GAP: Duration = Duration::from_millis(10);
/// The fire is drawn about 25 times a second, like Razer Chroma does.
const FIRE_FRAME_GAP: Duration = Duration::from_millis(40);
/// Consecutive failed frames after which the animation falls back to the firmware Breathing.
const MAX_FRAME_FAILS: u8 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanTarget {
    Off,
    Rpm(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightTarget {
    pub mode: PadLightingMode,
    pub rgb: [u8; 3],
    /// Second color (two-color Breathing / Static), if used.
    pub rgb2: Option<[u8; 3]>,
    /// Two-color Static as a gradient instead of two halves.
    pub gradient: bool,
    /// Two-color Breathing: 0 = alternate (firmware), 1 = two halves, 2 = gradient (animated).
    pub breath_layout: u8,
    /// Breathing speed, a percent of the standard speed (100 = standard / the firmware's).
    pub breath_speed: u8,
    /// How fast the drawn effects (Fire, Tidal, Starlight, Spectrum, Color Dipp) go, a percent.
    pub speed: u8,
    /// Raw Chroma brightness, 0-255.
    pub brightness: u8,
    /// Tidal: the waves come in towards the middle instead of running out from it. Wheel: it goes
    /// counter-clockwise.
    pub inward: bool,
}

impl LightTarget {
    /// Breathing with two colors laid out as halves / a gradient, or at a non-standard speed:
    /// the firmware cannot do that, so the worker animates it frame by frame.
    pub fn animated(&self) -> bool {
        matches!(
            self.mode,
            PadLightingMode::Fire
                | PadLightingMode::Tidal
                | PadLightingMode::Starlight
                | PadLightingMode::Spectrum
                | PadLightingMode::Dipp
                | PadLightingMode::Rainbow
                | PadLightingMode::Wheel
        ) || (self.mode == PadLightingMode::Breathing
                && ((self.rgb2.is_some() && self.breath_layout != 0) || self.breath_speed != STANDARD_SPEED))
    }
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
        // Without the "Toggle App Control" opt-in the fan is never touched.
        let fan = match cfg.pad_fan.as_str() {
            _ if !cfg.pad_control => None,
            "off" => Some(FanTarget::Off),
            "manual" => Some(FanTarget::Rpm(cfg.pad_rpm)),
            _ => None,
        };
        let light =
            PadLightingMode::from_name(&cfg.pad_last_mode).map(|mode| light_target(cfg, mode));
        Self { fan, light }
    }
}

/// The lighting to send for `mode` from the saved settings. The Fire takes its two colors from
/// the pad's own Fire settings (hot as the first color, cold as the second).
pub fn light_target(cfg: &crate::config::Config, mode: PadLightingMode) -> LightTarget {
    let (rgb, rgb2) = if mode == PadLightingMode::Fire {
        (cfg.pad_fire_hot, Some(cfg.pad_fire_cold))
    } else if mode == PadLightingMode::Tidal {
        (cfg.pad_tidal_a, Some(cfg.pad_tidal_b))
    } else if mode == PadLightingMode::Dipp {
        (cfg.pad_dipp_a, Some(cfg.pad_dipp_b))
    } else if mode == PadLightingMode::Starlight {
        (cfg.pad_starlight_a, cfg.pad_starlight_two.then_some(cfg.pad_starlight_b))
    } else if mode == PadLightingMode::Wheel {
        // (no second color: Razer's rainbow)
        (cfg.pad_wheel_a, cfg.pad_wheel_custom.then_some(cfg.pad_wheel_b))
    } else {
        (cfg.pad_color, light_second_color(cfg.pad_two, mode, cfg.pad_color2))
    };
    LightTarget {
        mode,
        rgb,
        rgb2,
        gradient: cfg.pad_split == 1,
        breath_layout: cfg.pad_breath_split.min(2),
        breath_speed: cfg.pad_breath_speed,
        speed: match mode {
            PadLightingMode::Fire => cfg.pad_fire_speed,
            PadLightingMode::Tidal => cfg.pad_tidal_speed,
            PadLightingMode::Starlight => cfg.pad_starlight_speed,
            PadLightingMode::Spectrum => cfg.pad_spectrum_speed,
            PadLightingMode::Dipp => cfg.pad_dipp_speed,
            PadLightingMode::Rainbow => cfg.pad_rainbow_speed,
            PadLightingMode::Wheel => cfg.pad_wheel_speed,
            _ => STANDARD_SPEED,
        },
        brightness: pct_to_raw(cfg.pad_brightness),
        // (Tidal comes in; the Wheel goes counter-clockwise)
        inward: if mode == PadLightingMode::Wheel { cfg.pad_wheel_ccw } else { cfg.pad_tidal_in },
    }
}

/// The second color only matters for Static and Breathing.
pub fn light_second_color(two: bool, mode: PadLightingMode, color2: [u8; 3]) -> Option<[u8; 3]> {
    (two && mode != PadLightingMode::Off).then_some(color2)
}

/// Brightness percent (0-100) to the raw Chroma value (0-255).
pub fn pct_to_raw(pct: u8) -> u8 {
    ((pct.min(100) as u16 * 255 + 50) / 100) as u8
}

#[derive(Debug, Clone, Copy)]
pub enum PadCmd {
    Fan(FanTarget),
    /// "Toggle App Control" for the fan was switched on / off. Off forgets the remembered fan
    /// setting, so a re-plugged pad is left alone.
    FanControl(bool),
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

fn colors(t: &LightTarget) -> PadColors {
    // "gradient" means the Static gradient, or for animated Breathing the gradient layout.
    let gradient = if t.animated() { t.breath_layout == 2 } else { t.gradient };
    PadColors { primary: rgb(t.rgb), secondary: t.rgb2.map(rgb), gradient }
}

/// The animation that is running (split Breathing, or the Fire).
struct PadAnim {
    target: LightTarget,
    started: Instant,
    fails: u8,
    /// The flame, when the mode is Fire, and when it was last moved on.
    flame: Fire,
    /// The waves, when the mode is Tidal.
    tide: Tidal,
    /// The stars, when the mode is Starlight.
    sky: Starlight,
    /// The rainbow, when the mode is Spectrum.
    rainbow: Spectrum,
    /// The slide between two colors, when the mode is Color Dipp.
    dipper: Dipp,
    /// The scrolling colors, when the mode is Rainbow.
    arch: Rainbow,
    /// The turning rainbow, when the mode is Wheel.
    wheel: Wheel,
    last_frame: Instant,
}

/// Applies a lighting target. Firmware effects go through `apply_lighting` /
/// `apply_color_change`; two-color Breathing with halves / a gradient starts (or updates) the
/// animation in `anim`. `color_only` = only the colors or layout changed, not the mode.
fn apply_light(
    pad: &CoolingPad,
    anim: &mut Option<PadAnim>,
    t: LightTarget,
    color_only: bool,
) -> anyhow::Result<()> {
    if t.animated() {
        if let Some(running) = anim.as_mut() {
            if color_only {
                running.target = t; // the next frame already uses the new colors
                return Ok(());
            }
        }
        *anim = None;
        pad.start_animation(t.brightness)?;
        let now = Instant::now();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        *anim = Some(PadAnim {
            target: t,
            started: now,
            fails: 0,
            flame: Fire::new(seed),
            tide: Tidal::new(),
            sky: Starlight::new_pad(seed),
            rainbow: Spectrum::new(),
            dipper: Dipp::new(),
            arch: Rainbow::new(),
            wheel: Wheel::new(),
            last_frame: now,
        });
        return Ok(());
    }
    let was_animated = anim.take().is_some();
    if color_only && !was_animated {
        pad.apply_color_change(t.mode, &colors(&t), t.brightness)
    } else {
        pad.apply_lighting(t.mode, &colors(&t), t.brightness)
    }
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
    /// Split Breathing animation, if one is running.
    anim: Option<PadAnim>,
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
            PadCmd::FanControl(on) => {
                if !on {
                    self.desired.fan = None;
                }
                return;
            }
            PadCmd::LightingEnabled(on) => {
                self.lighting_enabled = on;
                if !on {
                    self.anim = None;
                }
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
                if let Some(running) = self.anim.as_mut() {
                    running.target.brightness = b;
                }
            }
        }

        let pad = match self.pad.as_ref() {
            Some(p) => p,
            None => return,
        };
        let result = match cmd {
            PadCmd::Fan(target) => apply_fan(pad, target),
            PadCmd::Light(t) => apply_light(pad, &mut self.anim, t, false),
            PadCmd::LightColor(t) => apply_light(pad, &mut self.anim, t, true),
            PadCmd::Brightness(b) => pad.set_brightness(b),
            PadCmd::FanControl(_) | PadCmd::LightingEnabled(_) | PadCmd::ResumeLighting(_) => {
                return
            }
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
                    if let Err(e) = apply_light(pad, &mut self.anim, light, false) {
                        push_event(&self.shared, true, format!("Cooling pad lighting: {}", e));
                    }
                }
            }
        }
        true
    }

    /// Sends the next frame of the split Breathing animation. After repeated failures it falls
    /// back to the firmware's two-color Breathing so the pad never ends up dark.
    fn animate_tick(&mut self) {
        if self.pad.is_none() {
            self.anim = None;
            return;
        }
        if let Some(running) = self.anim.as_ref() {
            if matches!(
                running.target.mode,
                PadLightingMode::Fire
                    | PadLightingMode::Tidal
                    | PadLightingMode::Starlight
                    | PadLightingMode::Spectrum
                    | PadLightingMode::Dipp
                    | PadLightingMode::Rainbow
                    | PadLightingMode::Wheel
            ) && running.last_frame.elapsed() < FIRE_FRAME_GAP
            {
                return;
            }
        }
        let result = match (self.anim.as_mut(), self.pad.as_ref()) {
            (Some(running), Some(pad)) if running.target.mode == PadLightingMode::Rainbow => {
                let now = Instant::now();
                running.arch.advance(
                    now.duration_since(running.last_frame).as_secs_f32()
                        * speed_factor(running.target.speed),
                );
                running.last_frame = now;
                pad.send_led_frame(&running.arch.pad_grid())
            }
            (Some(running), Some(pad)) if running.target.mode == PadLightingMode::Wheel => {
                let now = Instant::now();
                running.wheel.advance(
                    now.duration_since(running.last_frame).as_secs_f32() * speed_factor(running.target.speed),
                );
                running.last_frame = now;
                // (`inward` is the Wheel's counter-clockwise too)
                pad.send_led_frame(&match running.target.rgb2 {
                    Some(b) => running.wheel.dipp_pad(running.target.rgb, b, running.target.inward),
                    None => running.wheel.rainbow_pad(running.target.inward),
                })
            }
            (Some(running), Some(pad)) if running.target.mode == PadLightingMode::Dipp => {
                let now = Instant::now();
                running.dipper.advance(
                    now.duration_since(running.last_frame).as_secs_f32() * speed_factor(running.target.speed),
                );
                running.last_frame = now;
                let a = running.target.rgb;
                let b = running.target.rgb2.unwrap_or(a);
                pad.send_led_frame(&running.dipper.pad_grid(a, b))
            }
            (Some(running), Some(pad)) if running.target.mode == PadLightingMode::Spectrum => {
                let now = Instant::now();
                running.rainbow.advance(
                    now.duration_since(running.last_frame).as_secs_f32() * speed_factor(running.target.speed),
                );
                running.last_frame = now;
                pad.send_led_frame(&running.rainbow.pad_grid())
            }
            (Some(running), Some(pad)) if running.target.mode == PadLightingMode::Starlight => {
                let now = Instant::now();
                running.sky.advance(
                    now.duration_since(running.last_frame).as_secs_f32() * speed_factor(running.target.speed),
                );
                running.last_frame = now;
                pad.send_led_frame(&running.sky.pad_grid(running.target.rgb, running.target.rgb2))
            }
            (Some(running), Some(pad)) if running.target.mode == PadLightingMode::Tidal => {
                let now = Instant::now();
                running.tide.advance(
                    now.duration_since(running.last_frame).as_secs_f32() * speed_factor(running.target.speed),
                );
                running.last_frame = now;
                let a = running.target.rgb;
                let b = running.target.rgb2.unwrap_or(a);
                pad.send_led_frame(&running.tide.pad_grid(a, b, running.target.inward))
            }
            (Some(running), Some(pad)) if running.target.mode == PadLightingMode::Fire => {
                let now = Instant::now();
                running.flame.advance(
                    now.duration_since(running.last_frame).as_secs_f32() * speed_factor(running.target.speed),
                );
                running.last_frame = now;
                let hot = running.target.rgb;
                let cold = running.target.rgb2.unwrap_or(hot);
                pad.send_led_frame(&running.flame.pad_grid(hot, cold))
            }
            (Some(running), Some(pad)) => {
                let t = running.started.elapsed().as_secs_f32();
                let period = breath_period(running.target.breath_speed);
                let mut frame_colors = colors(&running.target);
                // The Alternate layout shows one color per fade (like the firmware effect).
                if running.target.breath_layout == 0 && running.target.rgb2.is_some() {
                    if breath_cycle(t, period) % 2 == 1 {
                        frame_colors.primary =
                            rgb(running.target.rgb2.unwrap_or(running.target.rgb));
                    }
                    frame_colors.secondary = None;
                }
                pad.send_breath_frame(&frame_colors, breath_level_with(t, period))
            }
            _ => return,
        };
        match result {
            Ok(()) => {
                if let Some(running) = self.anim.as_mut() {
                    running.fails = 0;
                }
            }
            Err(e) => {
                let mut give_up = false;
                if let Some(running) = self.anim.as_mut() {
                    running.fails += 1;
                    give_up = running.fails >= MAX_FRAME_FAILS;
                }
                if give_up {
                    let target = self.anim.take().map(|running| running.target);
                    push_event(
                        &self.shared,
                        true,
                        format!("Cooling pad: the animated lighting failed ({}); using the pad's own", e),
                    );
                    if let (Some(mut t), Some(pad)) = (target, self.pad.as_ref()) {
                        t.breath_layout = 0;
                        t.breath_speed = STANDARD_SPEED;
                        if matches!(
                            t.mode,
                            PadLightingMode::Fire
                                | PadLightingMode::Tidal
                                | PadLightingMode::Starlight
                                | PadLightingMode::Spectrum
                                | PadLightingMode::Dipp
                                | PadLightingMode::Rainbow
                                | PadLightingMode::Wheel
                        ) {
                            // the pad cannot do these itself: a plain color keeps it lit
                            t.mode = PadLightingMode::Static;
                            t.rgb2 = None;
                        }
                        self.desired.light = Some(t);
                        let _ = pad.apply_lighting(t.mode, &colors(&t), t.brightness);
                    }
                }
            }
        }
    }

    fn drop_pad(&mut self) {
        self.anim = None;
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
    let mut worker =
        Worker { shared, pad: None, desired, lighting_enabled, resume_lighting, anim: None };
    let mut last_detect: Option<Instant> = None;
    let mut last_probe = Instant::now();

    loop {
        let wait = if worker.anim.is_some() { ANIM_GAP } else { Duration::from_millis(500) };
        match rx.recv_timeout(wait) {
            Ok(cmd) => {
                worker.handle(cmd);
                while let Ok(next) = rx.try_recv() {
                    worker.handle(next);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        if worker.anim.is_some() {
            worker.animate_tick();
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

    fn target(mode: PadLightingMode, rgb2: Option<[u8; 3]>, breath_layout: u8) -> LightTarget {
        LightTarget {
            mode,
            rgb: [255, 0, 0],
            rgb2,
            gradient: true,
            breath_layout,
            breath_speed: STANDARD_SPEED,
            speed: STANDARD_SPEED,
            brightness: 200,
            inward: false,
        }
    }

    #[test]
    fn only_two_color_breathing_with_a_layout_is_animated() {
        let blue = Some([0, 0, 255]);
        assert!(target(PadLightingMode::Breathing, blue, 1).animated());
        assert!(target(PadLightingMode::Breathing, blue, 2).animated());
        assert!(!target(PadLightingMode::Breathing, blue, 0).animated());
        assert!(!target(PadLightingMode::Breathing, None, 2).animated());
        assert!(!target(PadLightingMode::Static, blue, 2).animated());
        assert!(!target(PadLightingMode::Off, blue, 2).animated());
    }

    #[test]
    fn the_fire_is_animated_and_takes_the_fire_colors() {
        let mut cfg = crate::config::Config::default();
        cfg.pad_color = [1, 2, 3];
        cfg.fire_hot = [9, 9, 9];
        cfg.pad_fire_hot = [255, 110, 0];
        cfg.pad_fire_cold = [255, 0, 0];
        cfg.pad_breath_split = 0;
        let fire = light_target(&cfg, PadLightingMode::Fire);
        assert!(fire.animated());
        assert_eq!(fire.rgb, [255, 110, 0]);
        assert_eq!(fire.rgb2, Some([255, 0, 0]));
        let still = light_target(&cfg, PadLightingMode::Static);
        assert!(!still.animated());
        assert_eq!(still.rgb, [1, 2, 3]);
    }

    #[test]
    fn the_starlight_is_animated_and_takes_the_pad_s_own_colors() {
        let mut cfg = crate::config::Config::default();
        cfg.starlight_a = [9, 9, 9];
        cfg.pad_starlight_a = [1, 2, 3];
        cfg.pad_starlight_b = [4, 5, 6];
        let one = light_target(&cfg, PadLightingMode::Starlight);
        assert!(one.animated());
        assert_eq!((one.rgb, one.rgb2), ([1, 2, 3], None));
        cfg.pad_starlight_two = true;
        let two = light_target(&cfg, PadLightingMode::Starlight);
        assert_eq!(two.rgb2, Some([4, 5, 6]));
    }

    #[test]
    fn each_drawn_mode_takes_its_own_speed_from_the_pad_settings() {
        let mut cfg = crate::config::Config::default();
        cfg.fire_speed = 10;
        cfg.pad_fire_speed = 60;
        cfg.pad_tidal_speed = 70;
        cfg.pad_starlight_speed = 80;
        cfg.pad_spectrum_speed = 90;
        cfg.pad_dipp_speed = 200;
        cfg.pad_rainbow_speed = 55;
        cfg.pad_wheel_speed = 130;
        cfg.pad_breath_speed = 140;
        let speed = |mode| light_target(&cfg, mode).speed;
        assert_eq!(speed(PadLightingMode::Fire), 60);
        assert_eq!(speed(PadLightingMode::Tidal), 70);
        assert_eq!(speed(PadLightingMode::Starlight), 80);
        assert_eq!(speed(PadLightingMode::Spectrum), 90);
        assert_eq!(speed(PadLightingMode::Dipp), 200);
        assert_eq!(speed(PadLightingMode::Rainbow), 55);
        // the Wheel: its own speed, animated by the app, and its direction rides in `inward`
        assert_eq!(speed(PadLightingMode::Wheel), 130);
        assert!(light_target(&cfg, PadLightingMode::Wheel).animated());
        assert_eq!(PadLightingMode::from_name("Wheel"), Some(PadLightingMode::Wheel));
        assert!(!light_target(&cfg, PadLightingMode::Wheel).inward);
        assert_eq!(light_target(&cfg, PadLightingMode::Wheel).rgb2, None, "Razer's rainbow has no colors");
        let mut own = crate::config::Config::default();
        own.pad_wheel_custom = true;
        own.pad_wheel_a = [1, 2, 3];
        own.pad_wheel_b = [4, 5, 6];
        let t = light_target(&own, PadLightingMode::Wheel);
        assert_eq!((t.rgb, t.rgb2), ([1, 2, 3], Some([4, 5, 6])));
        let mut turned = crate::config::Config::default();
        turned.pad_wheel_ccw = true;
        assert!(light_target(&turned, PadLightingMode::Wheel).inward);
        assert!(!light_target(&turned, PadLightingMode::Tidal).inward, "the tide is not turned round by the wheel");
        assert!(light_target(&cfg, PadLightingMode::Rainbow).animated());
        assert_eq!(PadLightingMode::from_name("Rainbow"), Some(PadLightingMode::Rainbow));
        assert_eq!(speed(PadLightingMode::Static), STANDARD_SPEED);
        // the Breathing speed is its own, and a speed that is not the standard makes the app draw it
        let breathing = light_target(&cfg, PadLightingMode::Breathing);
        assert_eq!(breathing.breath_speed, 140);
        assert!(breathing.animated());
        cfg.pad_breath_speed = STANDARD_SPEED;
        assert!(!light_target(&cfg, PadLightingMode::Breathing).animated());
    }

    #[test]
    fn the_spectrum_is_animated_and_needs_no_color() {
        let cfg = crate::config::Config::default();
        let rainbow = light_target(&cfg, PadLightingMode::Spectrum);
        assert!(rainbow.animated());
        assert_eq!(PadLightingMode::from_name("Spectrum"), Some(PadLightingMode::Spectrum));
    }

    #[test]
    fn color_dipp_is_animated_and_takes_the_pad_s_own_two_colors() {
        let mut cfg = crate::config::Config::default();
        cfg.dipp_a = [9, 9, 9];
        cfg.pad_dipp_a = [1, 2, 3];
        cfg.pad_dipp_b = [4, 5, 6];
        let dip = light_target(&cfg, PadLightingMode::Dipp);
        assert!(dip.animated());
        assert_eq!((dip.rgb, dip.rgb2), ([1, 2, 3], Some([4, 5, 6])));
        assert_eq!(PadLightingMode::from_name("Color Dipp"), Some(PadLightingMode::Dipp));
    }

    #[test]
    fn the_tidal_is_animated_and_takes_the_pad_s_own_tidal_colors() {
        let mut cfg = crate::config::Config::default();
        cfg.tidal_a = [9, 9, 9];
        cfg.pad_tidal_a = [1, 2, 3];
        cfg.pad_tidal_b = [4, 5, 6];
        cfg.pad_tidal_in = true;
        let tide = light_target(&cfg, PadLightingMode::Tidal);
        assert!(tide.animated());
        assert_eq!((tide.rgb, tide.rgb2, tide.inward), ([1, 2, 3], Some([4, 5, 6]), true));
    }

    #[test]
    fn the_fan_is_only_restored_with_app_control_on() {
        let mut cfg = crate::config::Config::default();
        cfg.pad_fan = "manual".to_string();
        cfg.pad_rpm = 1800;
        assert_eq!(Desired::from_config(&cfg).fan, None);
        cfg.pad_control = true;
        assert_eq!(Desired::from_config(&cfg).fan.map(|f| f == FanTarget::Rpm(1800)), Some(true));
        cfg.pad_fan = "off".to_string();
        assert_eq!(Desired::from_config(&cfg).fan.map(|f| f == FanTarget::Off), Some(true));
        cfg.pad_fan = String::new();
        assert!(Desired::from_config(&cfg).fan.is_none());
    }

    #[test]
    fn a_custom_speed_animates_any_breathing() {
        let mut single = target(PadLightingMode::Breathing, None, 0);
        assert!(!single.animated());
        single.breath_speed = 160;
        assert!(single.animated());
        let mut alternate = target(PadLightingMode::Breathing, Some([0, 0, 255]), 0);
        assert!(!alternate.animated());
        alternate.breath_speed = 60;
        assert!(alternate.animated());
        // speed means nothing outside Breathing
        let mut fixed = target(PadLightingMode::Static, None, 0);
        fixed.breath_speed = 250;
        assert!(!fixed.animated());
    }

    #[test]
    fn gradient_flag_follows_the_mode() {
        let blue = Some([0, 0, 255]);
        // Static uses the Static gradient setting ...
        assert!(colors(&target(PadLightingMode::Static, blue, 1)).gradient);
        // ... animated Breathing uses its own layout (1 = halves, 2 = gradient)
        assert!(!colors(&target(PadLightingMode::Breathing, blue, 1)).gradient);
        assert!(colors(&target(PadLightingMode::Breathing, blue, 2)).gradient);
    }
}
