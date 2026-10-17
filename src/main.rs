#![windows_subsystem = "windows"]

const WINDOW_WIDTH: f32 = 570.0;

mod config;
mod device;
mod dlog;
mod messaging;
mod power;
mod synapse;
mod system;
mod theme;
mod ui;
mod utils;

use eframe::egui;
use egui::IconData;

use anyhow::Result;
use std::sync::mpsc;

use librazer::types::{
    BatteryCare, CpuBoost, FanMode, GpuBoost, LightsAlwaysOn, LogoMode, MaxFanSpeedMode, PerfMode,
};
use librazer::{command, device::Device};
use strum::IntoEnumIterator;

use device::CompleteDeviceState;
use messaging::{error_message, status_message, MessageManager};
use power::get_power_state;
use system::{get_system_specs, SystemSpecs};
use utils::{execute_device_command_simple, DeviceStateReader};

// Dynamic app metadata from Cargo
const APP_NAME: &str = "CPUnity:R-Helper";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
enum InitMessage {
    SystemSpecsComplete(SystemSpecs),
    PowerStateRead(bool),
    InitializationComplete,
    DeviceDetectionComplete(bool),
}

#[derive(Debug, Clone)]
struct DeviceStatus {
    performance_mode: String,
    fan_speed: String,
    fan_rpm: Option<u16>,
    fan_actual_rpm: Option<u16>,
    logo_mode: String,
    keyboard_brightness: u8,
    lights_always_on: bool,
    battery_care: bool,
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self {
            performance_mode: "Reading...".to_string(),
            fan_speed: "Reading...".to_string(),
            fan_rpm: None,
            fan_actual_rpm: None,
            logo_mode: "Reading...".to_string(),
            keyboard_brightness: 0,
            lights_always_on: false,
            battery_care: true,
        }
    }
}

struct RazerGuiApp {
    status: DeviceStatus,
    device: Option<Device>,
    device_state: Option<CompleteDeviceState>,
    system_specs: SystemSpecs,
    available_performance_modes: Vec<PerfMode>,
    base_performance_modes: Vec<PerfMode>,

    ac_power: bool,
    ac_profile: CompleteDeviceState,
    battery_profile: CompleteDeviceState,

    loading: bool,
    fully_initialized: bool,
    init_receiver: Option<mpsc::Receiver<InitMessage>>,
    message_manager: MessageManager,
    last_refresh_time: std::time::Instant,
    last_state_check_time: std::time::Instant,
    /// Last device poll was slow (HID channel busy, e.g. Chroma running): poll less often.
    slow_poll: bool,
    /// Performance mode the device reported on the previous poll when it differed from the UI.
    perf_mismatch_seen: Option<String>,
    /// The saved AC/Battery profile has been applied (or skipped) once for this launch.
    launch_profile_done: bool,
    last_fan_enforce_time: std::time::Instant,
    status_messages: bool,

    manual_fan_rpm: u16,
    temp_brightness_step: usize,
    brightness_slider_active: bool,
    should_quit: bool,

    init_power_read: bool,
    init_specs_complete: bool,
    last_perf_poll_time: std::time::Instant,
    cpu_boost: CpuBoost,
    gpu_boost: GpuBoost,
    base_window_height: f32,
    expanded_window_height: Option<f32>,
    custom_controls_visible_last: bool,
    // Device detection state
    detecting_device: bool,
    device_detection_done: bool,
    min_detecting_until: std::time::Instant,

    // CPUnity additions
    cfg: config::Config,
    startup_enabled: bool,
    show_options: bool,
    logo: Option<egui::TextureHandle>,
    banner: Option<egui::TextureHandle>,
    logo_loaded: bool,
    battery_limit: u8,
    applied_accent: Option<[u8; 3]>,
    synapse: synapse::Handle,
    applied_font: Option<String>,
    /// Last window height we requested, so we only resize when the content height really changes.
    last_fit_height: f32,
}

impl RazerGuiApp {
    fn perf_mode_to_string(mode: PerfMode) -> String {
        format!("{:?}", mode)
    }

    fn string_to_perf_mode(mode: &str) -> Option<PerfMode> {
        PerfMode::iter().find(|m| format!("{:?}", m) == mode)
    }

    fn logo_mode_to_string(mode: LogoMode) -> &'static str {
        match mode {
            LogoMode::Static => "Static",
            LogoMode::Breathing => "Breathing",
            LogoMode::Off => "Off",
        }
    }

    fn string_to_logo_mode(mode: &str) -> Option<LogoMode> {
        match mode {
            "Static" => Some(LogoMode::Static),
            "Breathing" => Some(LogoMode::Breathing),
            "Off" => Some(LogoMode::Off),
            _ => None,
        }
    }

    fn read_current_fan_state(device: &Device) -> (FanMode, Option<u16>) {
        // Read the current fan mode from the combined perf/fan query.
        // (We intentionally avoid a second immediate retry; caller logic tolerates fallback to Auto.)
        let fan_mode = command::get_perf_mode(device).map(|(_, fm)| fm).unwrap_or_else(|_| {
            eprintln!("Warning: Failed to read device fan mode, assuming Auto");
            FanMode::Auto
        });
        let set_rpm = get_fan_rpm_set(device, librazer::types::FanZone::Zone1);
        (fan_mode, set_rpm)
    }

    fn get_fan_status_from_mode(fan_mode: FanMode, device: &Device) -> (String, Option<u16>) {
        match fan_mode {
            FanMode::Auto => ("Auto".to_string(), None),
            FanMode::Manual => {
                let set_rpm = get_fan_rpm_set(device, librazer::types::FanZone::Zone1);
                ("Manual".to_string(), set_rpm)
            }
        }
    }

    fn set_no_device_message(&mut self) {
        self.set_status_message("No device connected".to_string());
    }

    fn new() -> Self {
        // Profiles kept in-memory so we can auto-switch on AC/Battery changes.
        let cfg = config::load();
        theme::set_accent(egui::Color32::from_rgb(cfg.accent[0], cfg.accent[1], cfg.accent[2]));

        let mut ac_profile = CompleteDeviceState::default();
        if let Some(m) = cfg.ac_mode.as_deref().and_then(config::perf_mode_from_name) {
            ac_profile.perf_mode = m;
        }
        if cfg.ac_overclock {
            ac_profile.perf_mode = PerfMode::Custom;
        }
        let mut battery_profile =
            CompleteDeviceState { perf_mode: PerfMode::Battery, ..CompleteDeviceState::default() };
        if let Some(m) = cfg.battery_mode.as_deref().and_then(config::perf_mode_from_name) {
            battery_profile.perf_mode = m;
        }

        let (init_sender, init_receiver) = mpsc::channel();

        let now = std::time::Instant::now();
        let mut app = Self {
            status: DeviceStatus::default(),
            device: None,
            device_state: None,
            system_specs: SystemSpecs::default(),
            available_performance_modes: Vec::new(),
            base_performance_modes: Vec::new(),
            ac_power: true,
            ac_profile,
            battery_profile,
            loading: true,
            fully_initialized: false,
            init_receiver: Some(init_receiver),
            message_manager: MessageManager::new(),
            last_refresh_time: std::time::Instant::now(),
            last_state_check_time: std::time::Instant::now(),
            slow_poll: false,
            perf_mismatch_seen: None,
            launch_profile_done: false,
            last_fan_enforce_time: std::time::Instant::now(),
            status_messages: false,

            manual_fan_rpm: 2000,
            temp_brightness_step: 0,
            brightness_slider_active: false,

            should_quit: false,

            init_power_read: false,
            init_specs_complete: false,
            last_perf_poll_time: std::time::Instant::now(),
            cpu_boost: CpuBoost::Low,
            gpu_boost: GpuBoost::Low,
            base_window_height: 0.0,
            expanded_window_height: None,
            custom_controls_visible_last: false,
            detecting_device: true,
            device_detection_done: false,
            min_detecting_until: now + std::time::Duration::from_secs(1),

            battery_limit: cfg.battery_limit,
            applied_accent: None,
            synapse: synapse::Handle::spawn(),
            applied_font: None,
            last_fit_height: 0.0,
            startup_enabled: config::startup_enabled(),
            show_options: false,
            logo: None,
            banner: None,
            logo_loaded: false,
            cfg,
        };

        // Kick off async device detection so the UI can show a clear “Detecting device…” state.
        app.start_device_detection(init_sender.clone());

        // Start other background initialization (power state, system specs)
        app.start_background_initialization(init_sender);

        app
    }

    fn start_device_detection(&mut self, sender: mpsc::Sender<InitMessage>) {
        self.detecting_device = true;
        std::thread::spawn(move || {
            let present = match Device::detect() {
                Ok(_dev) => true,
                Err(e) => {
                    eprintln!("Failed to connect to Razer device: {}", e);
                    false
                }
            };
            let _ = sender.send(InitMessage::DeviceDetectionComplete(present));
        });
    }

    fn detect_available_performance_modes(&mut self) {
        // Prefer firmware-advertised list; fallback to full enum when unknown.
        if let Some(ref device) = self.device {
            if let Some(list) = device.info().perf_modes {
                self.available_performance_modes = list.to_vec();
                if self.base_performance_modes.is_empty() {
                    self.base_performance_modes = self.available_performance_modes.clone();
                }
                return;
            }
        }
        self.available_performance_modes = PerfMode::iter().collect();
        if self.base_performance_modes.is_empty() {
            self.base_performance_modes = self.available_performance_modes.clone();
        }
    }

    fn get_descriptor_allowed_boosts(
        &self,
    ) -> (Vec<CpuBoost>, Vec<GpuBoost>, Vec<(CpuBoost, GpuBoost)>) {
        if let Some(ref device) = self.device {
            let d = device.info();
            let cpus: Vec<CpuBoost> = d.cpu_boosts.map(|s| s.to_vec()).unwrap_or_else(|| {
                vec![CpuBoost::Low, CpuBoost::Medium, CpuBoost::High, CpuBoost::Boost]
            });
            let gpus: Vec<GpuBoost> = d
                .gpu_boosts
                .map(|s| s.to_vec())
                .unwrap_or_else(|| vec![GpuBoost::Low, GpuBoost::Medium, GpuBoost::High]);
            let pairs: Vec<(CpuBoost, GpuBoost)> =
                d.disallowed_boost_pairs.map(|p| p.to_vec()).unwrap_or_default();
            (cpus, gpus, pairs)
        } else {
            (
                vec![CpuBoost::Low, CpuBoost::Medium, CpuBoost::High, CpuBoost::Boost],
                vec![GpuBoost::Low, GpuBoost::Medium, GpuBoost::High],
                Vec::new(),
            )
        }
    }

    fn read_initial_device_state(&mut self) {
        if let Some(ref device) = self.device {
            let mut reader = DeviceStateReader::new(device);
            // Use batched reader helper to gather as much as possible without early abort.

            if let Some(brightness) =
                reader.read(|d| command::get_keyboard_brightness(d), "keyboard brightness")
            {
                self.status.keyboard_brightness = brightness;
                self.temp_brightness_step = ui::lighting::raw_brightness_to_step_index(brightness);
            }

            if let Some((perf_mode, fan_mode)) =
                reader.read(|d| command::get_perf_mode(d), "performance mode")
            {
                self.status.performance_mode = Self::perf_mode_to_string(perf_mode).to_string();

                let (fan_speed, fan_rpm) = Self::get_fan_status_from_mode(fan_mode, device);
                self.status.fan_speed = fan_speed;
                self.status.fan_rpm = fan_rpm;

                if let Some(rpm) = fan_rpm {
                    self.manual_fan_rpm = rpm;
                }

                if matches!(perf_mode, PerfMode::Custom) {
                    if let Ok(v) = command::get_cpu_boost(device) {
                        self.cpu_boost = v;
                    }
                    if let Ok(v) = command::get_gpu_boost(device) {
                        self.gpu_boost = v;
                    }
                }
            }

            if self.status.fan_speed == "Reading..." {
                // Fallback: if earlier combined call failed but later succeeds, fill fan info.
                if let Ok((_, fan_mode)) = command::get_perf_mode(device) {
                    let (fan_speed, fan_rpm) = Self::get_fan_status_from_mode(fan_mode, device);
                    self.status.fan_speed = fan_speed;
                    self.status.fan_rpm = fan_rpm;

                    if let Some(rpm) = fan_rpm {
                        self.manual_fan_rpm = rpm;
                    }
                }
            }

            if let Some(lights_always_on) =
                reader.read(|d| command::get_lights_always_on(d), "lights always on")
            {
                self.status.lights_always_on = matches!(lights_always_on, LightsAlwaysOn::Enable);
            }

            if let Some(battery_care) =
                reader.read(|d| command::get_battery_care(d), "battery care")
            {
                self.status.battery_care = matches!(battery_care, BatteryCare::Enable);
            }

            if let Ok((_, pct)) = command::get_battery_limit(device) {
                self.battery_limit = pct.clamp(
                    librazer::command::BATTERY_LIMIT_MIN,
                    librazer::command::BATTERY_LIMIT_MAX,
                );
            }

            let errors = reader.finish();
            if !errors.is_empty() && cfg!(debug_assertions) {
                eprintln!("Device state reading errors: {:?}", errors);
            }
        }
    }

    fn start_background_initialization(&mut self, sender: mpsc::Sender<InitMessage>) {
        // Device may not be known yet; pass None here. We'll still display specs we can read.
        let device_name: Option<String> = None;

        std::thread::spawn(move || {
            if let Ok(ac_power) = get_power_state() {
                let _ = sender.send(InitMessage::PowerStateRead(ac_power));
            }

            let _ = sender.send(InitMessage::InitializationComplete);

            let device_name_ref = device_name.as_deref();
            let system_specs = get_system_specs(device_name_ref);
            let _ = sender.send(InitMessage::SystemSpecsComplete(system_specs));
        });

        self.loading = false;
    }

    fn process_background_initialization(&mut self) {
        let mut messages_to_process = Vec::new();
        // Drain all pending init messages this frame (non-blocking).

        if let Some(ref receiver) = self.init_receiver {
            while let Ok(message) = receiver.try_recv() {
                messages_to_process.push(message);
            }
        }

        for message in messages_to_process {
            match message {
                InitMessage::DeviceDetectionComplete(present) => {
                    self.device_detection_done = true;
                    // If a device is found, switch immediately. If not, keep detecting until grace expires.
                    if present {
                        self.detecting_device = false;
                    }
                    if present {
                        // Acquire the device on the UI thread.
                        if let Ok(dev) = Device::detect() {
                            dlog::log(&format!(
                                "device: {} pid={:#06x}",
                                dev.info().name,
                                dev.info().pid
                            ));
                            self.device = Some(dev);
                        }
                    }
                    self.detect_available_performance_modes();
                    if self.device.is_some() {
                        self.read_initial_device_state();
                        // Now that the device is known, we can show a brief init message.
                        self.set_status_message("Initializing...".to_string());
                    }
                }
                InitMessage::SystemSpecsComplete(specs) => {
                    self.system_specs = specs;
                    self.init_specs_complete = true;
                    if self.fully_initialized && self.init_power_read && self.init_specs_complete {
                        // Avoid flashing a transient message on unsupported devices.
                        if self.device.is_some() {
                            self.set_status_message("Initialization complete".to_string());
                        } else {
                            self.set_optional_status_message("Initialization complete".to_string());
                        }
                    } else {
                        self.set_optional_status_message(
                            "System specifications loaded".to_string(),
                        );
                    }
                }
                InitMessage::PowerStateRead(ac_power) => {
                    self.ac_power = ac_power;
                    self.init_power_read = true;
                }
                InitMessage::InitializationComplete => {
                    self.fully_initialized = true;
                    if self.device.is_some() {
                        if let Err(e) = self.read_device_status() {
                            self.set_error_message(format!("Failed to read device status: {}", e));
                        } else {
                            self.update_stored_device_state();
                            self.sync_ui_with_device_state();
                            self.init_fan_slider_from_device();
                        }
                    }
                }
            }
        }
    }
}

fn get_fan_rpm_actual(device: &Device, zone: librazer::types::FanZone) -> Option<u16> {
    match command::get_fan_actual_rpm(device, zone) {
        Ok(rpm) => Some(rpm),
        Err(_) => None,
    }
}

fn get_fan_rpm_set(device: &Device, zone: librazer::types::FanZone) -> Option<u16> {
    match command::get_fan_rpm(device, zone) {
        Ok(rpm) => Some(rpm),
        Err(_) => None,
    }
}

impl RazerGuiApp {
    fn read_device_status(&mut self) -> Result<()> {
        let device = self.device.as_ref().unwrap();
        // Core perf + fan query (single device round-trip).
        let (perf_mode, fan_mode) = command::get_perf_mode(device)?;
        self.status.performance_mode = Self::perf_mode_to_string(perf_mode).to_string();
        let (fan_speed, fan_rpm) = Self::get_fan_status_from_mode(fan_mode, device);
        self.status.fan_speed = fan_speed;
        self.status.fan_rpm = fan_rpm;
        if let Some(rpm) = fan_rpm {
            self.manual_fan_rpm = rpm;
        }
        self.status.fan_actual_rpm = get_fan_rpm_actual(device, librazer::types::FanZone::Zone1);
        if let Ok(logo_mode) = command::get_logo_mode(device) {
            self.status.logo_mode = Self::logo_mode_to_string(logo_mode).to_string();
        }

        if let Ok(brightness) = command::get_keyboard_brightness(device) {
            self.status.keyboard_brightness = brightness;
            self.temp_brightness_step = ui::lighting::raw_brightness_to_step_index(brightness);
        }

        if let Ok(lights_always_on) = command::get_lights_always_on(device) {
            self.status.lights_always_on = matches!(lights_always_on, LightsAlwaysOn::Enable);
        }

        if let Ok(battery_care) = command::get_battery_care(device) {
            self.status.battery_care = matches!(battery_care, BatteryCare::Enable);
        }

        Ok(())
    }

    fn sync_ui_with_device_state(&mut self) {
        if let Some(ref device) = self.device {
            // Refresh only fields that can drift externally (brightness skipped if user dragging slider).
            if !self.brightness_slider_active {
                if let Ok(brightness) = command::get_keyboard_brightness(device) {
                    self.status.keyboard_brightness = brightness;
                    self.temp_brightness_step =
                        ui::lighting::raw_brightness_to_step_index(brightness);
                }
            }
            let (fan_mode, set_rpm) = Self::read_current_fan_state(device);
            let (fan_speed, fan_rpm) = Self::get_fan_status_from_mode(fan_mode, device);
            self.status.fan_speed = fan_speed;
            self.status.fan_rpm = fan_rpm;
            if let Some(rpm) = set_rpm {
                self.manual_fan_rpm = rpm;
            }
            if let Ok(lights_always_on) = command::get_lights_always_on(device) {
                self.status.lights_always_on = matches!(lights_always_on, LightsAlwaysOn::Enable);
            }
            if let Ok(battery_care) = command::get_battery_care(device) {
                self.status.battery_care = matches!(battery_care, BatteryCare::Enable);
            }
        }
    }

    #[allow(dead_code)]
    fn sync_other_dynamic_state(&mut self) {
        if let Some(ref device) = self.device {
            // Light-weight periodic poll for simple toggles.
            if let Ok(lights_always_on) = command::get_lights_always_on(device) {
                self.status.lights_always_on = matches!(lights_always_on, LightsAlwaysOn::Enable);
            }
            if let Ok(battery_care) = command::get_battery_care(device) {
                self.status.battery_care = matches!(battery_care, BatteryCare::Enable);
            }
        }
    }

    fn init_fan_slider_from_device(&mut self) {
        if let Some(ref device) = self.device {
            // Initializes manual fan slider to currently set RPM if in Manual.
            let (fan_mode, set_rpm) = Self::read_current_fan_state(device);
            let (fan_speed, fan_rpm) = Self::get_fan_status_from_mode(fan_mode, device);
            self.status.fan_speed = fan_speed;
            self.status.fan_rpm = fan_rpm;
            if let Some(rpm) = set_rpm {
                self.manual_fan_rpm = rpm;
            }
        }
    }

    fn check_device_state_changes(&mut self) -> Result<()> {
        if let Some(ref device) = self.device {
            // Track the charge limit too (it is not part of CompleteDeviceState).
            if let Ok((_, pct)) = command::get_battery_limit(device) {
                self.battery_limit = pct.clamp(
                    librazer::command::BATTERY_LIMIT_MIN,
                    librazer::command::BATTERY_LIMIT_MAX,
                );
            }

            // CPU/GPU boost presets only exist in Custom mode. Refresh them on every snapshot so a
            // failed or too-early read at launch (autostart at logon) corrects itself in seconds.
            if self.status.performance_mode == "Custom" {
                if let Ok(v) = command::get_cpu_boost(device) {
                    self.cpu_boost = v;
                }
                if let Ok(v) = command::get_gpu_boost(device) {
                    self.gpu_boost = v;
                }
            }

            // Full snapshot comparison to detect external changes.
            let current_state = CompleteDeviceState::read_from_device(device)?;

            if let Some(ref stored_state) = self.device_state {
                if current_state != *stored_state {
                    let old_perf_mode = Self::perf_mode_to_string(stored_state.perf_mode);
                    let new_perf_mode = Self::perf_mode_to_string(current_state.perf_mode);
                    dlog::log(&format!(
                        "external change detected: perf {} -> {}",
                        old_perf_mode, new_perf_mode
                    ));

                    self.device_state = Some(current_state.clone());

                    // Convert the low-level state to our UI format
                    self.status.performance_mode =
                        Self::perf_mode_to_string(current_state.perf_mode).to_string();

                    let (fan_speed, fan_rpm) =
                        Self::get_fan_status_from_mode(current_state.fan_mode, device);
                    self.status.fan_speed = fan_speed;
                    self.status.fan_rpm = fan_rpm;
                    if let Some(rpm) = fan_rpm {
                        self.manual_fan_rpm = rpm;
                    }

                    self.status.logo_mode =
                        Self::logo_mode_to_string(current_state.logo_mode).to_string();

                    self.status.keyboard_brightness = current_state.keyboard_brightness;
                    self.temp_brightness_step = ui::lighting::raw_brightness_to_step_index(
                        current_state.keyboard_brightness,
                    );

                    self.status.lights_always_on =
                        matches!(current_state.lights_always_on, LightsAlwaysOn::Enable);
                    self.status.battery_care =
                        matches!(current_state.battery_care, BatteryCare::Enable);

                    if old_perf_mode != new_perf_mode {
                        self.set_optional_status_message("Mode updated".to_string());
                    } else if self.status_messages {
                        self.set_optional_status_message(
                            "Device state updated externally".to_string(),
                        );
                    }
                }
            } else {
                self.device_state = Some(current_state);
            }
        }
        Ok(())
    }

    fn set_status_message(&mut self, message: String) {
        self.message_manager.add_message(status_message(message));
    }

    fn set_optional_status_message(&mut self, message: String) {
        if self.status_messages {
            self.message_manager.add_message(status_message(message));
        }
    }

    fn set_error_message(&mut self, message: String) {
        self.message_manager.add_message(error_message(message));
    }

    fn update_stored_device_state(&mut self) {
        if let Some(ref device) = self.device {
            if let Ok(current_state) = CompleteDeviceState::read_from_device(device) {
                self.device_state = Some(current_state);
            }
        }
    }

    /// Applies the saved AC/Battery profile once at launch. Only an explicit choice counts:
    /// "Default" (and no AC overclock) leaves the laptop in whatever mode it is already in.
    fn apply_launch_profile(&mut self) {
        let explicit = if self.ac_power {
            self.cfg.ac_overclock || self.cfg.ac_mode.is_some()
        } else {
            self.cfg.battery_mode.is_some()
        };
        dlog::log(&format!(
            "launch: power={} explicit_profile={} device_mode={} cpu={:?} gpu={:?}",
            if self.ac_power { "AC" } else { "battery" },
            explicit,
            self.status.performance_mode,
            self.cpu_boost,
            self.gpu_boost
        ));
        if explicit {
            self.auto_switch_profile();
        }
    }

    fn auto_switch_profile(&mut self) {
        // Mode we just asked the EC for; it needs a moment to commit, so an immediate read-back
        // still returns the OLD mode. The poll loop corrects the UI if the EC really ignored it.
        let mut expected_mode: Option<String> = None;
        if let Some(ref device) = self.device {
            // Choose which saved profile to apply based on current AC state.
            let target_profile =
                if self.ac_power { self.ac_profile.clone() } else { self.battery_profile.clone() };

            let profile_name = if self.ac_power { "AC" } else { "Battery" };
            dlog::log(&format!(
                "auto_switch_profile: {} -> {:?}",
                profile_name, target_profile.perf_mode
            ));

            if let Err(e) = command::set_perf_mode(device, target_profile.perf_mode) {
                self.set_error_message(format!(
                    "Failed to switch to {} profile: {}",
                    profile_name, e
                ));
                return;
            }

            self.status.performance_mode =
                Self::perf_mode_to_string(target_profile.perf_mode).to_string();
            expected_mode = Some(self.status.performance_mode.clone());

            // Optional: apply the CPU Overclock preset when on AC power.
            if self.ac_power && self.cfg.ac_overclock {
                match command::set_cpu_boost(device, CpuBoost::Overclock) {
                    Ok(_) => {
                        self.cpu_boost = CpuBoost::Overclock;
                        dlog::log("AC overclock applied");
                    }
                    Err(e) => {
                        dlog::log(&format!("AC overclock ERR: {}", e));
                        self.set_error_message(format!("Failed to apply AC overclock: {}", e));
                    }
                }
            }

            self.set_status_message(format!("⚡ Auto-switched to {} profile", profile_name));
        }

        // Read current device state to preserve user settings
        if let Err(_) = self.read_device_status() {
            // If we can't read device status, try to apply minimal fallback
            if let Some(ref device) = self.device {
                let target_profile = if self.ac_power {
                    self.ac_profile.clone()
                } else {
                    self.battery_profile.clone()
                };

                if let Err(e) = self.apply_profile(device, &target_profile) {
                    self.set_error_message(format!("Failed to apply fallback profile: {}", e));
                }
            }
        }

        if let Some(mode) = expected_mode {
            self.status.performance_mode = mode;
            // Forget the (stale) snapshot; the next 5 s snapshot stores a fresh one.
            self.device_state = None;
            self.perf_mismatch_seen = None;
        } else {
            self.update_stored_device_state();
        }

        self.sync_ui_with_device_state();
    }

    fn apply_profile(&self, device: &Device, profile: &CompleteDeviceState) -> Result<()> {
        command::set_perf_mode(device, profile.perf_mode)?;

        command::set_logo_mode(device, profile.logo_mode)?;

        if let Ok(current_brightness) = command::get_keyboard_brightness(device) {
            if current_brightness != profile.keyboard_brightness {
                command::set_keyboard_brightness(device, profile.keyboard_brightness)?;
            }
        } else {
            command::set_keyboard_brightness(device, profile.keyboard_brightness)?;
        }

        command::set_lights_always_on(device, profile.lights_always_on)?;

        command::set_battery_care(device, profile.battery_care)?;

        Ok(())
    }

    fn set_performance_mode(&mut self, mode: &str) {
        let perf_mode = match Self::string_to_perf_mode(mode) {
            Some(m) => m,
            None => return,
        };

        let mut restore_manual = None::<u16>;
        let mut read_boosts = false;
        let mut set_mode_ok = false;
        let mut error_msg: Option<String> = None;

        if let Some(ref device) = self.device {
            let (current_fan_mode, set_rpm) = Self::read_current_fan_state(device);

            let set_result = command::set_perf_mode(device, perf_mode);
            match &set_result {
                Ok(_) => dlog::log(&format!("set_perf_mode({:?}) ok", perf_mode)),
                Err(e) => dlog::log(&format!("set_perf_mode({:?}) ERR: {}", perf_mode, e)),
            }
            // Diagnostic read-back: does the EC keep the mode we just set?
            std::thread::sleep(std::time::Duration::from_millis(300));
            match command::get_perf_mode(device) {
                Ok((m, f)) => dlog::log(&format!("read-back after set: perf={:?} fan={:?}", m, f)),
                Err(e) => dlog::log(&format!("read-back after set ERR: {}", e)),
            }

            match set_result {
                Ok(_) => {
                    set_mode_ok = true;
                    // Preserve manual fan RPM if user had manual mode before switching.
                    if matches!(current_fan_mode, FanMode::Manual) {
                        restore_manual = set_rpm;
                    }
                    // Only query boost states for Custom (other modes ignore those values).
                    if mode == "Custom" {
                        read_boosts = true;
                    }
                }
                Err(e) => {
                    error_msg = Some(format!("Failed to set performance mode: {}", e));
                }
            }

            if set_mode_ok {
                if let Some(rpm) = restore_manual {
                    // Short delays give firmware time to commit mode before restoring manual fan state.
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    if command::set_fan_mode(device, FanMode::Manual).is_ok() {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        if command::set_fan_rpm(device, rpm, true).is_err() {
                            error_msg = Some(
                                "Failed to restore fan RPM after performance mode change".into(),
                            );
                        } else {
                            restore_manual = Some(rpm);
                        }
                    } else {
                        error_msg = Some(
                            "Failed to restore manual fan mode after performance mode change"
                                .into(),
                        );
                    }
                }
                if read_boosts {
                    // Populate boost controls so UI reflects actual device values.
                    if let Ok(v) = command::get_cpu_boost(device) {
                        self.cpu_boost = v;
                    }
                    if let Ok(v) = command::get_gpu_boost(device) {
                        self.gpu_boost = v;
                    }
                }
            }
        } else {
            self.set_no_device_message();
            return;
        }

        if let Some(msg) = error_msg {
            self.set_error_message(msg);
        }
        if set_mode_ok {
            self.status.performance_mode = mode.to_string();
            if let Some(rpm) = restore_manual {
                self.status.fan_speed = "Manual".into();
                self.status.fan_rpm = Some(rpm);
                self.manual_fan_rpm = rpm;
            }
            self.set_optional_status_message("Mode changed".into());
            self.update_stored_device_state();
        }
    }

    fn render_performance_section(&mut self, ui: &mut egui::Ui) {
        use ui::performance::{render_performance_section, PerformanceAction};
        let (mut allowed_cpu, mut allowed_gpu, disallowed_pairs) =
            self.get_descriptor_allowed_boosts();
        let base_cpu = allowed_cpu.clone();
        let base_gpu = allowed_gpu.clone();

        // If user toggled the eye icon (hidden modes view), also reveal all boost options.
        let showing_hidden =
            ui.ctx().data(|d| d.get_temp::<bool>("perf_hidden_show".into()).unwrap_or(false));
        if showing_hidden {
            // Full canonical sets (Overclock is always shown as its own button)
            let full_cpu = [CpuBoost::Low, CpuBoost::Medium, CpuBoost::High, CpuBoost::Boost];
            let full_gpu = [GpuBoost::Low, GpuBoost::Medium, GpuBoost::High];
            for b in full_cpu {
                if !allowed_cpu.contains(&b) {
                    allowed_cpu.push(b);
                }
            }
            for b in full_gpu {
                if !allowed_gpu.contains(&b) {
                    allowed_gpu.push(b);
                }
            }
            // Maintain a stable displayed order
            let order_cpu = |b: &CpuBoost| match b {
                CpuBoost::Low => 0,
                CpuBoost::Medium => 1,
                CpuBoost::High => 2,
                CpuBoost::Boost => 3,
                CpuBoost::Overclock => 4,
            };
            allowed_cpu.sort_by_key(order_cpu);
            let order_gpu = |b: &GpuBoost| match b {
                GpuBoost::Low => 0,
                GpuBoost::Medium => 1,
                GpuBoost::High => 2,
            };
            allowed_gpu.sort_by_key(order_gpu);
        }
        let action = render_performance_section(
            ui,
            &self.status.performance_mode,
            self.ac_power,
            &self.available_performance_modes,
            &self.base_performance_modes,
            self.status_messages, // debug flag reuse
            self.cpu_boost,
            self.gpu_boost,
            &allowed_cpu,
            &allowed_gpu,
            &disallowed_pairs,
            &base_cpu,
            &base_gpu,
            self.device.is_none(),
        );

        match action {
            PerformanceAction::None => {}
            PerformanceAction::SetPerformanceMode(mode) => {
                self.set_performance_mode(&mode);
            }
            PerformanceAction::ToggleHidden => {
                let current = ui
                    .ctx()
                    .data(|d| d.get_temp::<bool>("perf_hidden_show".into()).unwrap_or(false));
                ui.ctx().data_mut(|d| d.insert_temp("perf_hidden_show".into(), !current));
            }
            PerformanceAction::SetCpuBoost(boost) => {
                if self.status.performance_mode == "Custom" {
                    if let Some(ref device) = self.device {
                        if let Err(e) = command::set_cpu_boost(device, boost) {
                            self.set_error_message(format!("Failed CPU boost: {}", e));
                        } else {
                            self.cpu_boost = boost;
                            self.set_optional_status_message(format!("CPU {:?}", boost));
                        }
                    }
                }
            }
            PerformanceAction::SetGpuBoost(boost) => {
                if self.status.performance_mode == "Custom" {
                    if let Some(ref device) = self.device {
                        if let Err(e) = command::set_gpu_boost(device, boost) {
                            self.set_error_message(format!("Failed GPU boost: {}", e));
                        } else {
                            self.gpu_boost = boost;
                            self.set_optional_status_message(format!("GPU {:?}", boost));
                        }
                    }
                }
            }
        }
    }

    fn set_fan_mode(&mut self, mode: &str, rpm: Option<u16>) {
        if let Some(ref device) = self.device {
            let result = match mode {
                "auto" => match command::set_fan_mode(device, FanMode::Auto) {
                    Ok(_) => {
                        self.status.fan_speed = "Auto".to_string();
                        self.status.fan_rpm = None;
                        Ok(())
                    }
                    Err(e) => Err(e),
                },
                "manual" => match command::set_fan_mode(device, FanMode::Manual) {
                    Ok(_) => {
                        let rpm_val = rpm.unwrap_or(2000);
                        match command::set_fan_rpm(device, rpm_val, true) {
                            Ok(_) => {
                                self.status.fan_speed = "Manual".to_string();
                                self.status.fan_rpm = Some(rpm_val);
                                Ok(())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    Err(e) => Err(e),
                },
                _ => return,
            };

            match result {
                Ok(_) => {
                    self.set_optional_status_message(format!("Fan set to {} mode", mode));
                }
                Err(e) => {
                    self.set_status_message(format!("Failed to set fan: {}", e));
                }
            }
        } else {
            self.set_no_device_message();
        }
    }

    fn set_fan_rpm_only(&mut self, rpm: u16) {
        match execute_device_command_simple(
            self.device.as_ref(),
            |device| command::set_fan_rpm(device, rpm, true),
            &format!("Fans RPM set to: {}", rpm),
            "Failed to set fan RPM",
        ) {
            Ok(message) => {
                self.status.fan_rpm = Some(rpm);
                self.set_optional_status_message(message);
            }
            Err(message) => {
                self.set_error_message(message);
            }
        }
    }

    fn enforce_manual_fan_rpm(&mut self) {
        if self.status.fan_speed == "Manual" {
            if let Some(ref device) = self.device {
                // Periodically re-set manual RPM (device may drift after perf mode changes).
                if let Some(current_set_rpm) =
                    get_fan_rpm_set(device, librazer::types::FanZone::Zone1)
                {
                    if let Ok(_) = command::set_fan_rpm(device, current_set_rpm, false) {
                        self.manual_fan_rpm = current_set_rpm;
                        self.status.fan_rpm = Some(current_set_rpm);
                        self.last_fan_enforce_time = std::time::Instant::now();
                    }
                }
            }
        }
    }

    fn render_fan_section(&mut self, ui: &mut egui::Ui) {
        use ui::fan::{render_fan_section, FanAction};

        let key = egui::Id::new("max_fan_speed_enabled");
        let mut max_enabled = ui.ctx().data(|d| d.get_temp::<bool>(key).unwrap_or(false));
        let (action, new_toggle) = render_fan_section(
            ui,
            &self.status.fan_speed,
            self.status.fan_actual_rpm,
            self.status.fan_rpm,
            &mut self.manual_fan_rpm,
            self.status_messages,
            self.status.performance_mode == "Custom",
            max_enabled,
        );
        if new_toggle != max_enabled && self.status.performance_mode == "Custom" {
            if let Some(ref device) = self.device {
                let result = if new_toggle {
                    command::set_max_fan_speed_mode(device, MaxFanSpeedMode::Enable)
                } else {
                    command::set_max_fan_speed_mode(device, MaxFanSpeedMode::Disable)
                };
                match result {
                    Ok(_) => {
                        max_enabled = new_toggle;
                        self.set_optional_status_message(if new_toggle {
                            "Max fan enabled".into()
                        } else {
                            "Max fan disabled".into()
                        });
                    }
                    Err(e) => self.set_error_message(format!("Failed to toggle max fan: {}", e)),
                }
            }
        }
        ui.ctx().data_mut(|d| d.insert_temp(key, max_enabled));

        match action {
            FanAction::None => {}
            FanAction::SetAutoMode => {
                self.set_fan_mode("auto", None);
            }
            FanAction::SetManualMode(rpm) => {
                self.set_fan_mode("manual", Some(rpm));
            }
            FanAction::SetManualRpm(rpm) => {
                self.set_fan_rpm_only(rpm);
            }
            FanAction::SetMaxManual => {
                self.manual_fan_rpm = ui::fan::MAX_MANUAL_RPM;
                self.set_fan_mode("manual", Some(ui::fan::MAX_MANUAL_RPM));
            }
            FanAction::SliderDragging(_) => {}
        }
    }

    fn set_logo_mode(&mut self, mode: &str) {
        let logo_mode = match Self::string_to_logo_mode(mode) {
            Some(mode) => mode,
            None => return,
        };

        match execute_device_command_simple(
            self.device.as_ref(),
            |device| command::set_logo_mode(device, logo_mode),
            &format!("Logo mode set to {}", mode),
            "Failed to set logo mode",
        ) {
            Ok(message) => {
                self.status.logo_mode = mode.to_string();
                self.set_optional_status_message(message);
            }
            Err(message) => {
                self.set_error_message(message);
            }
        }
    }

    fn set_brightness(&mut self, brightness: u8) {
        match execute_device_command_simple(
            self.device.as_ref(),
            |device| command::set_keyboard_brightness(device, brightness),
            &format!(
                "Brightness set to step {}",
                ui::lighting::raw_brightness_to_step_index(brightness)
            ),
            "Failed to set brightness",
        ) {
            Ok(message) => {
                self.status.keyboard_brightness = brightness;
                self.temp_brightness_step = ui::lighting::raw_brightness_to_step_index(brightness);
                self.set_optional_status_message(message);
            }
            Err(message) => {
                self.set_error_message(message);
            }
        }
    }

    fn toggle_lights_always_on(&mut self) {
        let lights_always_on = if self.status.lights_always_on {
            LightsAlwaysOn::Enable
        } else {
            LightsAlwaysOn::Disable
        };

        if let Some(ref device) = self.device {
            match command::set_lights_always_on(device, lights_always_on) {
                Ok(_) => {
                    self.set_optional_status_message(format!(
                        "Keyboard Backlight Always On {}",
                        if self.status.lights_always_on { "enabled" } else { "disabled" }
                    ));
                    self.update_stored_device_state();
                }
                Err(e) => {
                    self.set_status_message(format!("Failed to set lights always on: {}", e));
                    self.status.lights_always_on = !self.status.lights_always_on;
                }
            }
        } else {
            self.set_no_device_message();
        }
    }

    fn render_lighting_section(&mut self, ui: &mut egui::Ui) {
        use ui::lighting::render_lighting_section;

        let action = render_lighting_section(
            ui,
            &self.status.logo_mode,
            &mut self.temp_brightness_step,
            &mut self.status.lights_always_on,
        );

        if let Some(active) = action.slider_active {
            self.brightness_slider_active = active;
        }

        if let Some(mode) = action.logo_mode {
            self.set_logo_mode(&mode);
        }

        if let Some(brightness) = action.brightness {
            self.set_brightness(brightness);
        }

        if action.lights_always_on {
            self.toggle_lights_always_on();
        }
    }

    fn toggle_battery_care(&mut self) {
        if let Some(ref device) = self.device {
            match command::set_battery_limit(device, self.status.battery_care, self.battery_limit) {
                Ok(_) => {
                    self.set_optional_status_message(format!(
                        "Battery care {}",
                        if self.status.battery_care { "enabled" } else { "disabled" }
                    ));
                }
                Err(e) => {
                    self.set_status_message(format!("Failed to set battery care: {}", e));
                    // Revert the UI change on error
                    self.status.battery_care = !self.status.battery_care;
                }
            }
        } else {
            self.set_no_device_message();
        }
    }

    fn render_synapse_section(&mut self, ui: &mut egui::Ui) {
        let status = self.synapse.snapshot();
        let mut keep = self.cfg.synapse_keep_lighting;
        let action = ui::synapse::render_synapse_section(ui, &status, &mut keep);
        if keep != self.cfg.synapse_keep_lighting {
            self.cfg.synapse_keep_lighting = keep;
            config::save(&self.cfg);
        }
        use ui::synapse::SynapseAction;
        match action {
            SynapseAction::None => {}
            SynapseAction::Standby => {
                self.synapse.run(synapse::Action::Standby, keep);
                self.set_optional_status_message("Synapse standby: approve the UAC prompt".into());
            }
            SynapseAction::Resume => {
                self.synapse.run(synapse::Action::Resume, keep);
                self.set_optional_status_message("Resuming Synapse: approve the UAC prompt".into());
            }
            SynapseAction::GuardOn => self.synapse.run(synapse::Action::GuardOn, keep),
            SynapseAction::GuardOff => self.synapse.run(synapse::Action::GuardOff, keep),
        }
    }

    fn set_battery_limit_pct(&mut self, pct: u8) {
        if let Some(ref device) = self.device {
            match command::set_battery_limit(device, self.status.battery_care, pct) {
                Ok(_) => {
                    self.battery_limit = pct;
                    self.cfg.battery_limit = pct;
                    config::save(&self.cfg);
                    self.set_optional_status_message(format!("Battery limit set to {}%", pct));
                }
                Err(e) => {
                    // Show what the device actually has, then report the failure.
                    if let Ok((_, p)) = command::get_battery_limit(device) {
                        self.battery_limit = p.clamp(
                            librazer::command::BATTERY_LIMIT_MIN,
                            librazer::command::BATTERY_LIMIT_MAX,
                        );
                    }
                    self.set_error_message(format!("Failed to set battery limit: {}", e));
                }
            }
        } else {
            self.set_no_device_message();
        }
    }

    fn render_battery_section(&mut self, ui: &mut egui::Ui) {
        use ui::battery::{render_battery_section, BatteryAction};

        let action =
            render_battery_section(ui, &mut self.status.battery_care, &mut self.battery_limit);

        match action {
            BatteryAction::None => {}
            BatteryAction::ToggleBatteryCare => {
                self.toggle_battery_care();
            }
            BatteryAction::SetLimit(pct) => {
                self.set_battery_limit_pct(pct);
            }
        }
    }
}

impl eframe::App for RazerGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // Apply the accent color to egui's selection/highlight color when it changes.
        let accent_rgb = [theme::accent().r(), theme::accent().g(), theme::accent().b()];
        if self.applied_accent != Some(accent_rgb) {
            theme::apply(ctx);
            self.applied_accent = Some(accent_rgb);
        }
        if self.applied_font.as_deref() != Some(self.cfg.font.as_str()) {
            theme::install_fonts(ctx, &self.cfg.font);
            self.applied_font = Some(self.cfg.font.clone());
        }

        // Load the optional logo + banner once. A file next to the exe / in the config folder wins
        // (easy to tweak); otherwise the copy baked in from assets/ at build time is used.
        if !self.logo_loaded {
            self.logo_loaded = true;
            let logo_img = config::find_logo()
                .and_then(|p| image::open(p).ok())
                .or_else(|| embedded_logo().and_then(|b| image::load_from_memory(b).ok()));
            if let Some(img) = logo_img {
                let img = img.resize(128, 128, image::imageops::FilterType::Lanczos3).to_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
                self.logo =
                    Some(ctx.load_texture("cpunity_logo", color, egui::TextureOptions::LINEAR));
            }
            let banner_img = config::find_banner()
                .and_then(|p| image::open(p).ok())
                .or_else(|| embedded_banner().and_then(|b| image::load_from_memory(b).ok()));
            if let Some(img) = banner_img {
                // Keep full detail for sharp HiDPI rendering; only shrink huge files.
                let img = if img.width() > 2048 || img.height() > 2048 {
                    img.resize(2048, 2048, image::imageops::FilterType::Lanczos3)
                } else {
                    img
                };
                let img = img.to_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
                self.banner =
                    Some(ctx.load_texture("cpunity_banner", color, egui::TextureOptions::LINEAR));
                dlog::log(&format!("banner loaded ({}x{} px)", size[0], size[1]));
            }
        }

        self.process_background_initialization();

        // The background specs thread starts before the device is known, so its model name is
        // always "Unknown". Fill it in from the detected Razer device (cheap check per frame).
        if self.system_specs.device_model == "Unknown" {
            if let Some(dev) = &self.device {
                self.system_specs.device_model = system::simplify_model_name(dev.info().name);
            }
        }

        // Once per launch, after the device and power state are known.
        if self.fully_initialized && self.init_power_read && !self.launch_profile_done {
            if self.device.is_some() {
                self.launch_profile_done = true;
                self.apply_launch_profile();
            } else if self.device_detection_done && !self.detecting_device {
                self.launch_profile_done = true; // no device: nothing to apply
            }
        }

        let hidden_on =
            ctx.data(|d| d.get_temp::<bool>("perf_hidden_show".into()).unwrap_or(false));
        if self.device.is_some() {
            if hidden_on {
                self.available_performance_modes = PerfMode::iter().collect();
            } else {
                self.detect_available_performance_modes();
            }
        }

        self.message_manager.update();

        // When minimized, poll infrequently to catch external performance mode changes
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        if minimized && self.fully_initialized {
            const PERF_POLL_INTERVAL: f32 = 2.5; // seconds
            if self.last_perf_poll_time.elapsed().as_secs_f32() >= PERF_POLL_INTERVAL {
                if let Some(ref device) = self.device {
                    if let Ok((perf_mode, fan_mode)) = command::get_perf_mode(device) {
                        let new_mode = Self::perf_mode_to_string(perf_mode).to_string();
                        if self.status.performance_mode != new_mode {
                            self.status.performance_mode = new_mode;
                            let (fan_speed, fan_rpm) =
                                Self::get_fan_status_from_mode(fan_mode, device);
                            self.status.fan_speed = fan_speed;
                            self.status.fan_rpm = fan_rpm;
                        }
                    }
                }
                self.last_perf_poll_time = std::time::Instant::now();
            }
        }

        // Handle close request from X button
        if ctx.input(|i| i.viewport().close_requested()) {
            self.should_quit = true;
        }

        // Handle quit
        if self.should_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Only update when window is not minimized to save resources
        if !ctx.input(|i| i.viewport().minimized.unwrap_or(false)) {
            // Only do regular updates if fully initialized to avoid slow operations during startup
            if self.fully_initialized {
                // Auto-refresh device status based on backlight setting
                // Back off to 4 s / 20 s when the previous poll was slow so a busy HID channel
                // (Razer Chroma running) can't keep the window frozen.
                let refresh_interval: f32 = if self.slow_poll { 4.0 } else { 1.0 };
                let snapshot_interval: f32 = if self.slow_poll { 20.0 } else { 5.0 };
                if self.last_refresh_time.elapsed().as_secs_f32() >= refresh_interval {
                    let poll_started = std::time::Instant::now();
                    let retries_before = librazer::device::retry_count();
                    if self.device.is_some() && !self.loading {
                        // High-frequency AC power detection: switching triggers profile application.
                        if let Ok(current_ac_power) = get_power_state() {
                            if current_ac_power != self.ac_power {
                                self.ac_power = current_ac_power;
                                self.auto_switch_profile();
                            }
                        }

                        if let Some(ref device) = self.device {
                            self.status.fan_actual_rpm =
                                get_fan_rpm_actual(device, librazer::types::FanZone::Zone1);

                            // One perf/fan query (2 packets) instead of 4+ per tick.
                            if let Ok((perf_now, current_fan_mode)) = command::get_perf_mode(device)
                            {
                                self.status.fan_speed = match current_fan_mode {
                                    FanMode::Auto => "Auto".to_string(),
                                    FanMode::Manual => "Manual".to_string(),
                                };

                                // Keep the highlighted mode equal to what the device really
                                // reports. Needs two identical polls in a row so a just-clicked
                                // mode (EC still committing) cannot flip the UI back and forth.
                                let now_str = Self::perf_mode_to_string(perf_now).to_string();
                                if now_str != self.status.performance_mode {
                                    if self.perf_mismatch_seen.as_deref() == Some(now_str.as_str())
                                    {
                                        dlog::log(&format!(
                                            "UI mode {} -> {} (device reports it)",
                                            self.status.performance_mode, now_str
                                        ));
                                        self.status.performance_mode = now_str;
                                        self.perf_mismatch_seen = None;
                                        if perf_now == PerfMode::Custom {
                                            if let Ok(v) = command::get_cpu_boost(device) {
                                                self.cpu_boost = v;
                                            }
                                            if let Ok(v) = command::get_gpu_boost(device) {
                                                self.gpu_boost = v;
                                            }
                                        }
                                    } else {
                                        self.perf_mismatch_seen = Some(now_str);
                                    }
                                } else {
                                    self.perf_mismatch_seen = None;
                                }
                            }
                        }

                        if self.last_fan_enforce_time.elapsed().as_secs_f32() >= 1.0 {
                            self.enforce_manual_fan_rpm();
                        }

                        // Brightness / lights-always-on / battery are refreshed by the
                        // snapshot below instead of being polled every tick.
                        if self.device.is_some() {
                            if self.last_state_check_time.elapsed().as_secs_f32()
                                >= snapshot_interval
                            {
                                if let Err(_e) = self.check_device_state_changes() {
                                    // Fallback: read full device status instead of minimal subset
                                    let _ = self.read_device_status();
                                }
                                self.last_state_check_time = std::time::Instant::now();
                            }
                        }
                    }

                    let poll_ms = poll_started.elapsed().as_millis();
                    let retries = librazer::device::retry_count() - retries_before;
                    if poll_ms > 250 {
                        dlog::log(&format!(
                            "slow device poll: {} ms, {} HID retries (poll interval now longer)",
                            poll_ms, retries
                        ));
                    }
                    self.slow_poll = poll_ms > 250;

                    self.last_refresh_time = std::time::Instant::now();
                }
            }
        }
        // Enforce a minimum detecting period before showing "No device detected"
        if self.detecting_device && self.device.is_none() && self.device_detection_done {
            if std::time::Instant::now() >= self.min_detecting_until {
                self.detecting_device = false;
            }
        }
        // (clear_status_message_if_disabled removed)
        let device_label = if self.system_specs.device_model != "Unknown" {
            format!("💻 {}", self.system_specs.device_model)
        } else if self.device.is_some() {
            "💻 Connected Device".to_string()
        } else {
            "💻 No Razer Device".to_string()
        };
        let footer_height = egui::TopBottomPanel::bottom("footer")
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER))
                    .inner_margin(egui::Margin::symmetric(12, 2)),
            )
            .show(ctx, |ui| {
                ui::footer::render_footer(ui, &mut self.show_options, &device_label);
            })
            .response
            .rect
            .height();

        let mut content_height = 0.0_f32;
        let central_response = egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin::same(12)))
            .show(ctx, |ui| {
                let scroll_out =
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        // Header with device name and status messages
                        ui::header::render_header(
                            ui,
                            ctx,
                            self.loading,
                            &self.system_specs,
                            &self.device,
                            &self.message_manager,
                            self.detecting_device,
                            self.logo.as_ref(),
                            self.banner.as_ref(),
                        );
                        ui.add_space(8.0);

                        self.render_performance_section(ui);
                        ui.add_space(8.0);

                        self.render_fan_section(ui);
                        ui.add_space(8.0);

                        self.render_lighting_section(ui);
                        ui.add_space(8.0);

                        self.render_battery_section(ui);
                        ui.add_space(8.0);

                        self.render_synapse_section(ui);
                    });
                content_height = scroll_out.content_size.y;
            });
        // Options window (accent color, always-on-top, startup, auto-switch modes)
        if self.show_options {
            let mut open = true;
            let modes = self.available_performance_modes.clone();
            let mut cfg = self.cfg.clone();
            let mut startup = self.startup_enabled;
            let mut res = ui::options::OptionsResult::default();
            egui::Window::new("⚙ Options")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .vscroll(true)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    res = ui::options::render_options(
                        ui,
                        &mut cfg,
                        &mut startup,
                        &modes,
                        &mut self.status_messages,
                    );
                });
            self.cfg = cfg;
            self.startup_enabled = startup;

            if res.accent_changed {
                theme::set_accent(egui::Color32::from_rgb(
                    self.cfg.accent[0],
                    self.cfg.accent[1],
                    self.cfg.accent[2],
                ));
                self.applied_accent = None; // re-apply egui selection color next frame
            }
            if res.on_top_changed {
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                    if self.cfg.always_on_top {
                        egui::WindowLevel::AlwaysOnTop
                    } else {
                        egui::WindowLevel::Normal
                    },
                ));
            }
            if let Some(v) = res.startup_toggled {
                config::set_startup(v);
            }
            if res.modes_changed {
                self.ac_profile.perf_mode = if self.cfg.ac_overclock {
                    PerfMode::Custom
                } else {
                    self.cfg
                        .ac_mode
                        .as_deref()
                        .and_then(config::perf_mode_from_name)
                        .unwrap_or(PerfMode::Performance)
                };
                self.battery_profile.perf_mode = self
                    .cfg
                    .battery_mode
                    .as_deref()
                    .and_then(config::perf_mode_from_name)
                    .unwrap_or(PerfMode::Battery);
            }
            if res.changed {
                config::save(&self.cfg);
            }
            if res.apply_now && self.device.is_some() {
                self.auto_switch_profile();
            }
            if !open {
                self.show_options = false;
            }
        }

        // Fit the window height to the content (capped to the monitor) whenever the content
        // height changes, e.g. when the Custom boost rows appear. The user can still resize.
        if content_height > 0.0 && !minimized && self.fully_initialized {
            let monitor_h = ctx.input(|i| i.viewport().monitor_size.map(|s| s.y)).unwrap_or(1000.0);
            let desired = content_height + footer_height + 28.0;
            let target = desired.min((monitor_h - 90.0).max(420.0)).max(420.0);
            if (target - self.last_fit_height).abs() > 6.0 {
                self.last_fit_height = target;
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    WINDOW_WIDTH,
                    target,
                )));
            }
        }
    }
}
/// Logo baked into the exe when `assets/logo.png` existed at build time (see build.rs).
fn embedded_logo() -> Option<&'static [u8]> {
    #[cfg(embedded_logo)]
    {
        Some(include_bytes!("../assets/logo.png"))
    }
    #[cfg(not(embedded_logo))]
    {
        None
    }
}

/// Banner baked into the exe when `assets/banner.png` existed at build time (see build.rs).
fn embedded_banner() -> Option<&'static [u8]> {
    #[cfg(embedded_banner)]
    {
        Some(include_bytes!("../assets/banner.png"))
    }
    #[cfg(not(embedded_banner))]
    {
        None
    }
}

fn load_icon() -> IconData {
    // Small-size variant of the CPUnity icon (the full multi-size icon lives in rhelper.ico).
    const ICON_DATA: &[u8] = include_bytes!("../assets/icon-32.png");

    if let Ok(image) = image::load_from_memory(ICON_DATA) {
        let image = image.resize_exact(32, 32, image::imageops::FilterType::Lanczos3).to_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();

        IconData { rgba, width, height }
    } else {
        let size = 32;
        let mut rgba = vec![0u8; (size * size * 4) as usize];

        for i in 0..(size * size) as usize {
            let base = i * 4;
            rgba[base] = 0;
            rgba[base + 1] = 150;
            rgba[base + 2] = 255;
            rgba[base + 3] = 255;
        }

        IconData { rgba, width: size, height: size }
    }
}

#[cfg(windows)]
fn set_windows_app_id() {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
    // Build a per-version AppUserModelID so taskbar grouping updates with releases
    let app_id = format!("CPUnity.RHelper.{}\0", APP_VERSION).encode_utf16().collect::<Vec<u16>>();
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(PCWSTR(app_id.as_ptr()));
    }
}

#[cfg(not(windows))]
fn set_windows_app_id() {}

fn main() -> Result<(), eframe::Error> {
    set_windows_app_id();
    let cfg = config::load();
    let keep_on_top = cfg.always_on_top;
    let initial_height = 780.0;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, initial_height])
            .with_min_inner_size([500.0, 420.0])
            .with_resizable(true)
            .with_maximize_button(false)
            .with_fullscreen(false)
            .with_title(APP_NAME)
            .with_icon(load_icon())
            .with_always_on_top()
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(500));
                if !keep_on_top {
                    ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                        egui::WindowLevel::Normal,
                    ));
                }
            });

            let mut app = RazerGuiApp::new();
            app.base_window_height = initial_height as f32;
            Ok(Box::new(app))
        }),
    )
}
