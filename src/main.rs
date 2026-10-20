#![windows_subsystem = "windows"]

const WINDOW_WIDTH: f32 = 570.0;

mod admin;
mod ally;
mod ally_room;
mod alley;
mod angels;
mod audio;
mod chartgen;
mod config;
mod device;
mod dlog;
mod doc;
mod help;
mod log_chart;
mod log_options;
mod log_plan;
mod dragon_net;
mod dragons;
mod father;
mod game_presets;
mod cat_talk;
mod game1;
mod game2;
mod game3;
mod game3_sound;
mod monarch;
mod moon;
mod void;
mod void_script;
mod underworld;
mod games;
mod shop;
mod fray;
mod skins;
mod sakura;
mod ambient;
mod pine;
mod pairs;
mod poker_ai;
mod poker_cam;
mod poker_cards;
mod poker_chips;
mod poker_deal;
mod poker_fair;
mod poker_game;
mod poker_react;
mod poker_run;
mod poker_save;
mod poker_sha;
mod poker_story;
mod poker_table;
mod release;
mod trophies;
mod platinum;
mod whispers;
mod gold;
mod import;
mod king;
mod info;
mod kbd_anim;
mod log_view;
mod login_task;
mod messaging;
mod pad;
mod picker;
mod places;
mod power;
mod power_plan;
mod sensor_ext;
mod sensor_history;
mod sensor_log;
mod sensors;
mod sfx;
mod smoker;
mod songs;
mod synapse;
mod synth;
mod system;
mod theme;
mod tray;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Main,
    Lighting,
    Sensors,
    Files,
    Trophies,
    Help,
    Changelog,
    Info,
}

impl Tab {
    /// The key Options and the config file use for a tab that can be switched off (see
    /// `config::TOGGLE_TABS`). Main cannot be switched off.
    fn config_key(self) -> Option<&'static str> {
        match self {
            Tab::Main => None,
            Tab::Lighting => Some("lighting"),
            Tab::Sensors => Some("sensors"),
            Tab::Files => Some("files"),
            // its own switch (Options > Settings > Tabs, once a trophy is earned): see `Config::trophies_tab_shown`
            Tab::Trophies => None,
            Tab::Help => Some("help"),
            Tab::Changelog => Some("changelog"),
            Tab::Info => Some("info"),
        }
    }
}

/// The program's window changing size (and place) over a moment.
struct WindowStretch {
    from: (egui::Vec2, Option<egui::Pos2>),
    to: (egui::Vec2, Option<egui::Pos2>),
    start: f64,
}

/// What the Underworld looks like, held still while the Shadow Monarch has your soul (the look can
/// be looked at and nothing else: see `poker_save.rs`).
#[derive(Debug, Clone, PartialEq)]
struct HeldLook {
    underworld_on: bool,
    void_on: bool,
    gold_on: bool,
    backdrop: bool,
    backdrop_style: info::BackdropStyle,
    skin: Option<String>,
}

impl HeldLook {
    fn of(cfg: &config::Config) -> HeldLook {
        HeldLook {
            underworld_on: cfg.underworld_on,
            void_on: cfg.void_on,
            gold_on: cfg.gold_on,
            backdrop: cfg.backdrop,
            backdrop_style: cfg.backdrop_style,
            skin: cfg.shop.skin().map(str::to_string),
        }
    }

    fn put_back(&self, cfg: &mut config::Config) {
        cfg.underworld_on = self.underworld_on;
        cfg.void_on = self.void_on;
        cfg.gold_on = self.gold_on;
        cfg.backdrop = self.backdrop;
        cfg.backdrop_style = self.backdrop_style;
        cfg.shop.set_skin(self.skin.as_deref());
        cfg.shop_preview = None;
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
    /// Confirms a charger change with two matching readings before the profile switches.
    power_watch: power_plan::PowerWatch,
    /// "Last Known Settings": confirms what the laptop is running before it is written down, and
    /// holds off for a few seconds after a profile is applied so a mode still settling is not saved.
    last_watch: power_plan::Remember,
    last_hold_until: std::time::Instant,
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
    /// The header logo pixels (width, height, RGBA) and the accent it was last coloured for.
    logo_src: Option<(usize, usize, Vec<u8>)>,
    logo_accent: Option<[u8; 3]>,
    /// The logo is the built-in one, so it takes the accent colour.
    logo_recolor: bool,
    /// The logo at full resolution, for the top of the Info tab.
    info_logo: Option<egui::TextureHandle>,
    /// The logo pixels (width, height, RGBA) the Info tab recolors, and the accent it was last
    /// colored for.
    info_logo_src: Option<(usize, usize, Vec<u8>)>,
    info_logo_accent: Option<[u8; 3]>,
    /// The picture behind the tabs (white, the pattern in its transparency).
    backdrop: Vec<egui::TextureHandle>,
    /// Father Time's clock pictures for Game 1, loaded when the game first needs them (the game must not
    /// depend on which backdrop is chosen: with The Cat as the backdrop there is no clock in `backdrop`).
    game_clock: Vec<egui::TextureHandle>,
    /// Which style `backdrop` holds, so a change in Options loads the new picture.
    backdrop_loaded: Option<info::BackdropStyle>,
    /// Whether the picture in `backdrop` is the one made for The Void (the angel cats differ).
    backdrop_void: bool,
    /// The souls of the Soul Whispers backdrop (the Underworld's).
    whispers: whispers::Field,
    /// Whether the picture loaded is the Soul Catsters of the Platinum look.
    backdrop_soul: bool,
    /// When Father Time's beard was last clicked (egui time), while he is reacting.
    father_click: Option<f64>,
    /// When the dead monarch king's current laugh began.
    dead_king_laugh: Option<f64>,
    /// Gold Edition: Father Time smoking, and what the player has done to his cigarette.
    smoker: smoker::Smoker,
    banner: Option<egui::TextureHandle>,
    logo_loaded: bool,
    battery_limit: u8,
    applied_accent: Option<[u8; 3]>,
    /// The Gold Edition look the egui theme was last applied for.
    applied_gold: bool,
    /// The Void look the egui theme was last applied for.
    applied_void: bool,
    /// The Underworld look the egui theme was last applied for.
    applied_underworld: bool,
    applied_skin: theme::Skin,
    /// The lava and the rock of the Underworld, made the first time it is on.
    underworld_tex: Option<ui::underworld::Textures>,
    /// The Void's header logo, loaded the first time The Void is on.
    void_logo: Option<egui::TextureHandle>,
    /// Gold Edition pictures, loaded the first time the Gold Edition is on: the header logo, the
    /// banner and the cat king of the Info tab.
    gold_logo: Option<egui::TextureHandle>,
    gold_banner: Option<egui::TextureHandle>,
    /// The Platinum banner, loaded the first time Platinum is the look.
    platinum_banner: Option<egui::TextureHandle>,
    gold_arena: Option<egui::TextureHandle>,
    /// Father Time's front and mask layers without the staff, and the staff (`gold::STAFF_PNGS`).
    gold_staff: Vec<egui::TextureHandle>,
    /// Father Time in blocks, for his glitching in The Void.
    void_father: Option<ui::backdrop::VoidTextures>,
    /// When the cat king's current laugh began (egui time), if he is laughing.
    king_laugh: Option<f64>,
    /// Game 1 (the cat inside Father Time's clock), while it is being played.
    game: Option<game1::Game>,
    /// The talk with the cat before the crown (the way into Game 1), while it is on the screen.
    cat_scene: Option<ui::cat_talk::Scene>,
    /// The Ally of Darkness (the alley escape room), while it is on the screen.
    ally_scene: Option<ui::ally::Scene>,
    /// The cat as a backdrop (far back in the alley, watching, coughing).
    cat_backdrop: ui::cat_talk::Backdrop,
    game_runs: u32,
    /// Game 2 (the staff of time), while it is being played.
    game2: Option<game2::Game>,
    /// The "Void unlocked" popup, shown once after the cat king's death.
    void_popup: bool,
    /// The shop's welcome is up (the first CR-Token was just earned).
    shop_popup: bool,
    /// Tokens earned and the starting tokens, not announced yet: the message waits until no game
    /// or scene is on the screen.
    token_notice: (u32, u32),
    /// Trophies that were just earned and have not been announced yet (the announcement waits for
    /// a quiet moment, like the tokens').
    trophy_notice: Vec<&'static trophies::Trophy>,
    /// The trophy announcement on the screen, and when it came up (egui time).
    trophy_toast: Option<(&'static trophies::Trophy, f64)>,
    /// No new announcement before this time, so that two in a row do not run together.
    trophy_gap_until: f64,
    /// The CR-Token coin (the logo, small), loaded the first time the shop shows.
    coin_tex: Option<egui::TextureHandle>,
    /// The moon in the corner of The Void at night (see moon.rs), and where the window was before the
    /// strike began to shake it.
    moon: moon::Moon,
    moon_shake_base: Option<egui::Pos2>,
    /// How much the moon makes everything glitch right now (0 to 1), and the glitch itself.
    moon_level: f32,
    moon_glitch: ui::glitch::Glitch,
    moon_shot_at: f64,
    /// Game 3 (the shadow monarch's rhythm game), while it is being played.
    game3: Option<ui::game3::Run>,
    /// The Hold'em table in the Underworld, while a game is being played.
    poker: Option<ui::poker::Run>,
    /// The look the Underworld is held in while the Shadow Monarch has your soul.
    held_look: Option<HeldLook>,
    /// Where the "you broke free" scene is (None: not on the screen).
    free_scene: Option<usize>,
    /// The sound card, opened the first time something needs it (None if there is none).
    audio: Option<audio::Audio>,
    audio_tried: bool,
    /// Whether the portal's sound has been started for the moon's strike that is going on.
    portal_sounded: bool,
    /// The popup after the ending of Game 3: the Underworld is open.
    underworld_popup: bool,
    /// A song being added to Game 3 (it is listened to on a thread; this gets the result).
    song_import: Option<mpsc::Receiver<Result<songs::Item, String>>>,
    /// The angel cats picture, for Game 2.
    gold_angels: Option<egui::TextureHandle>,
    /// While the game has the window: how big it was, and where, before it stretched wider.
    game_restore: Option<(egui::Vec2, Option<egui::Pos2>)>,
    /// The window stretching to the game or back.
    window_stretch: Option<WindowStretch>,
    /// The laugh the glitch has already been run for, so each laugh glitches once.
    glitch_for: Option<f64>,
    glitch: ui::glitch::Glitch,
    synapse: synapse::Handle,
    /// Optional Razer Laptop Cooling Pad (background worker, see pad.rs).
    pad: pad::Handle,
    /// Lighting mode last chosen in the Cooling Pad card this session ("" = not touched yet).
    /// Not saved: the pad lights are only set at launch via the opt-in "Restore lighting".
    pad_light_mode: String,
    /// Keyboard effect last chosen this session ("" = not touched yet). Not saved: the keyboard
    /// is only touched at launch via the opt-in "Restore lighting".
    kbd_effect: String,
    /// Software animation of Breathing with halves / a gradient (background worker).
    kbd_anim: kbd_anim::Handle,
    /// Opt-in hardware readings (only run while switched on).
    sensors: sensors::Handle,
    /// System tray icon (only while "Close button minimizes to tray" is on).
    tray: Option<tray::Tray>,
    /// Starting the tray icon failed this session, so the close button quits as usual.
    tray_failed: bool,
    /// The window is hidden in the tray right now.
    in_tray: bool,
    login_task: login_task::State,
    /// The Files tab's "Browse..." dialog is open (what it picks, and its answer).
    file_pick: Option<(picker::Target, mpsc::Receiver<Option<String>>)>,
    /// The Info tab's text, read when the tab is first shown (and on "Reload").
    /// The Sensors tab's Log viewer.
    log_view: ui::log_view::LogView,
    /// The log viewer window (open while a log is being looked at).
    log_window: Option<ui::log_window::LogWindow>,
    /// The programs that were running when the picker of the logging card was last opened.
    log_programs: Vec<String>,
    /// Which page is shown (not saved).
    tab: Tab,
    /// State of the Lighting Presets card (not saved).
    preset_ui: ui::presets::PresetsUiState,
    /// The look that was on at the last check, so a new one (unlocked or switched on) is noticed.
    last_look: Option<game_presets::Look>,
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
        theme::load_open_states(&cfg.open_tabs);
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
            power_watch: power_plan::PowerWatch::default(),
            last_watch: power_plan::Remember::default(),
            last_hold_until: std::time::Instant::now(),
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
            applied_gold: false,
            applied_void: false,
            applied_underworld: false,
            applied_skin: theme::Skin::None,
            underworld_tex: None,
            void_logo: None,
            gold_logo: None,
            gold_banner: None,
            platinum_banner: None,
            gold_arena: None,
            gold_staff: Vec::new(),
            void_father: None,
            king_laugh: None,
            game: None,
            cat_scene: None,
            ally_scene: None,
            cat_backdrop: ui::cat_talk::Backdrop::new(0x00C0_FFEE),
            game_runs: 0,
            game2: None,
            void_popup: false,
            shop_popup: false,
            token_notice: (0, 0),
            trophy_notice: Vec::new(),
            trophy_toast: None,
            trophy_gap_until: 0.0,
            coin_tex: None,
            moon: moon::Moon::new(),
            moon_shake_base: None,
            moon_level: 0.0,
            moon_glitch: ui::glitch::Glitch::Off,
            moon_shot_at: 0.0,
            game3: None,
            poker: None,
            held_look: None,
            free_scene: None,
            audio: None,
            audio_tried: false,
            portal_sounded: false,
            underworld_popup: false,
            song_import: None,
            gold_angels: None,
            game_restore: None,
            window_stretch: None,
            glitch_for: None,
            glitch: ui::glitch::Glitch::Off,
            synapse: synapse::Handle::spawn(),
            pad: pad::Handle::spawn(
                pad::Desired::from_config(&cfg),
                cfg.pad_lighting,
                cfg.light_resume,
            ),
            // With "Restore lighting at startup" the pad worker puts the last lighting back, so
            // the card should show it. Otherwise nothing was applied and no mode is selected.
            pad_light_mode: if cfg.light_resume && cfg.pad_lighting {
                cfg.pad_last_mode.clone()
            } else {
                String::new()
            },
            kbd_effect: String::new(),
            kbd_anim: kbd_anim::Handle::spawn(),
            sensors: {
                let handle = sensors::Handle::new();
                handle.set_log_plan(cfg.log_plan());
                handle.set_logging(cfg.sensor_logging);
                handle.set_log_name(&cfg.sensor_log_name);
                handle.set_history(cfg.sensor_graphs);
                handle.set_interval(cfg.sensor_interval_ms);
                if cfg.sensor_read && cfg.tab_shown("sensors") {
                    // it was on when R-Helper was closed (unless the Sensors tab is switched off: see `keep_sensors_in_step`)
                    handle.start(cfg.sensor_source_config());
                }
                handle
            },
            tray: None,
            tray_failed: false,
            login_task: login_task::State::default(),
            file_pick: None,
            log_view: ui::log_view::LogView::default(),
            log_window: None,
            log_programs: Vec::new(),
            // "Start minimized to tray": the window starts hidden (see main) and the icon is in the tray
            in_tray: cfg.start_minimized,
            tab: Tab::Main,
            preset_ui: ui::presets::PresetsUiState::default(),
            last_look: game_presets::current_look(&cfg),
            applied_font: None,
            last_fit_height: 0.0,
            startup_enabled: config::startup_enabled(),
            show_options: false,
            logo: None,
            logo_src: None,
            logo_accent: None,
            logo_recolor: false,
            info_logo: None,
            info_logo_src: None,
            info_logo_accent: None,
            backdrop: Vec::new(),
            game_clock: Vec::new(),
            backdrop_loaded: None,
            backdrop_void: false,
            whispers: whispers::Field::new(2026),
            backdrop_soul: false,
            father_click: None,
            dead_king_laugh: None,
            smoker: smoker::Smoker::new(),
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
            // Right after logon or wake Windows may not know the charger state yet. Ask again for
            // a couple of seconds before guessing (a guess of "battery" used to start the wrong
            // profile).
            let mut ac_power = None;
            for _ in 0..8 {
                if let Ok(v) = get_power_state() {
                    ac_power = Some(v);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            let ac_power = ac_power.unwrap_or_else(|| {
                dlog::log("power: charger state unknown at start, assuming AC");
                true
            });
            let _ = sender.send(InitMessage::PowerStateRead(ac_power));

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

/// Waits (about 2 s at most) until the laptop reports `mode` on both zones. It commits a mode
/// change a moment after accepting it, and until then the boost commands refuse to run.
fn wait_for_perf_mode(device: &Device, mode: PerfMode) -> bool {
    for _ in 0..power_plan::MODE_WAIT_TRIES {
        if let Ok((m, _)) = command::get_perf_mode(device) {
            if m == mode {
                return true;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(power_plan::MODE_WAIT_MS));
    }
    false
}

/// Tries a device write up to three times, with a short pause, before giving up.
fn with_retries(mut f: impl FnMut() -> Result<()>) -> Result<()> {
    let mut last = f();
    for _ in 0..2 {
        if last.is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
        last = f();
    }
    last
}

/// A boost saved in the settings by its name (`Low`, `High`, ...).
fn boost_by_name<T: IntoEnumIterator + std::fmt::Debug>(name: Option<&str>) -> Option<T> {
    let name = power_plan::clean_boost_name(name?)?;
    T::iter().find(|b| format!("{:?}", b) == name)
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

    /// Has the user picked a profile for this power source? "Default" leaves the laptop alone.
    fn profile_managed(&self, on_ac: bool) -> bool {
        let (last_on, last) = if on_ac {
            (self.cfg.ac_last, self.cfg.ac_last_state.as_deref())
        } else {
            (self.cfg.battery_last, self.cfg.battery_last_state.as_deref())
        };
        let last_ready = last_on && last.and_then(power_plan::decode_last).is_some();
        last_ready
            || power_plan::profile_is_explicit(
                on_ac,
                self.cfg.ac_mode.is_some(),
                self.cfg.ac_overclock,
                self.cfg.battery_mode.is_some(),
            )
    }

    /// The mode (and for Custom the CPU and GPU boosts) to put on a power source: what it was
    /// running last if "Last Known Settings" is on for it, otherwise the choices in Options.
    fn target_for(&self, on_ac: bool) -> power_plan::Target {
        let c = &self.cfg;
        let (last_on, last, mode, cpu, gpu) = if on_ac {
            (c.ac_last, c.ac_last_state.as_deref(), c.ac_mode.as_deref(), c.ac_cpu.as_deref(), c.ac_gpu.as_deref())
        } else {
            (
                c.battery_last,
                c.battery_last_state.as_deref(),
                c.battery_mode.as_deref(),
                c.battery_cpu.as_deref(),
                c.battery_gpu.as_deref(),
            )
        };
        power_plan::resolve(&power_plan::Pick {
            last_on,
            last,
            mode,
            overclock: on_ac && c.ac_overclock,
            cpu,
            gpu,
            fallback_cpu: c.custom_cpu.as_deref(),
            fallback_gpu: c.custom_gpu.as_deref(),
            default_mode: if on_ac { "Performance" } else { "Battery" },
        })
    }

    /// "Last Known Settings": writes down what the laptop is running on this power source (the
    /// mode, and for Custom the CPU and GPU boosts) so the next launch or plug/unplug puts it back.
    fn remember_last(&mut self) {
        if self.device.is_none()
            || !self.launch_profile_done
            || std::time::Instant::now() < self.last_hold_until
        {
            return;
        }
        let on_ac = self.ac_power;
        if !(if on_ac { self.cfg.ac_last } else { self.cfg.battery_last }) {
            return;
        }
        let mode = self.status.performance_mode.clone();
        if config::perf_mode_from_name(&mode).is_none() {
            return;
        }
        let custom = mode == "Custom";
        let cpu = custom.then(|| format!("{:?}", self.cpu_boost));
        let gpu = custom.then(|| format!("{:?}", self.gpu_boost));
        let now = power_plan::encode_last(&mode, cpu.as_deref(), gpu.as_deref());
        let saved = if on_ac { self.cfg.ac_last_state.clone() } else { self.cfg.battery_last_state.clone() };
        if let Some(s) = self.last_watch.see(&now, saved.as_deref()) {
            dlog::log(&format!(
                "last known on {}: {}",
                if on_ac { "AC" } else { "battery" },
                s
            ));
            if on_ac {
                self.cfg.ac_last_state = Some(s);
            } else {
                self.cfg.battery_last_state = Some(s);
            }
            config::save(&self.cfg);
        }
    }

    /// Notices the charger being plugged in or pulled out (two matching readings in a row) and
    /// switches to that source's profile. Runs while the window is open and while it is minimized
    /// or in the tray, so unplugging with the window hidden still switches to Battery mode.
    fn watch_power(&mut self) {
        if self.device.is_none() || !self.init_power_read {
            return;
        }
        let Ok(reading) = get_power_state() else { return };
        if let Some(now_ac) = self.power_watch.see(self.ac_power, reading) {
            let was_ac = self.ac_power;
            self.ac_power = now_ac;
            // switch when either side is a profile of the user's: leaving an AC Overclock must
            // not stay on when the charger comes out, even if Battery is left on "Default"
            let manage = self.profile_managed(now_ac) || self.profile_managed(was_ac);
            dlog::log(&format!(
                "power: now {} (profile switch {})",
                if now_ac { "AC" } else { "battery" },
                if manage { "yes" } else { "no, both on Default" }
            ));
            if manage {
                self.auto_switch_profile();
            }
        }
    }

    /// Applies the saved AC/Battery profile once at launch. Only an explicit choice counts:
    /// "Default" (and no AC overclock) leaves the laptop in whatever mode it is already in.
    fn apply_launch_profile(&mut self) {
        let explicit = self.profile_managed(self.ac_power);
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
            // Choose what to apply from the current power source: its Last Known Settings, or the
            // mode and boosts chosen in Options.
            let target = self.target_for(self.ac_power);
            let target_mode = config::perf_mode_from_name(&target.mode)
                .unwrap_or(if self.ac_power { PerfMode::Performance } else { PerfMode::Battery });
            self.last_hold_until = std::time::Instant::now() + std::time::Duration::from_secs(10);
            self.last_watch.reset();

            let profile_name = if self.ac_power { "AC" } else { "Battery" };
            dlog::log(&format!(
                "auto_switch_profile: {} -> {:?} (cpu {:?}, gpu {:?})",
                profile_name, target_mode, target.cpu, target.gpu
            ));

            if let Err(e) = command::set_perf_mode(device, target_mode) {
                self.set_error_message(format!(
                    "Failed to switch to {} profile: {}",
                    profile_name, e
                ));
                return;
            }

            self.status.performance_mode = Self::perf_mode_to_string(target_mode).to_string();
            expected_mode = Some(self.status.performance_mode.clone());

            // Custom mode: the CPU and GPU boosts only take once the laptop has committed the
            // mode, and it forgets them on a restart (the GPU falls back to Low while the CPU
            // sits in Custom). So wait for the mode, then put the boosts back.
            let mut problems: Vec<String> = Vec::new();
            if target_mode == PerfMode::Custom {
                let committed = wait_for_perf_mode(device, PerfMode::Custom);
                dlog::log(&format!("custom mode committed by the laptop: {}", committed));
                let want_cpu: Option<CpuBoost> = boost_by_name(target.cpu.as_deref());
                let want_gpu: Option<GpuBoost> = boost_by_name(target.gpu.as_deref());
                if let Some(b) = want_cpu {
                    match with_retries(|| command::set_cpu_boost(device, b)) {
                        Ok(_) => dlog::log(&format!("custom CPU boost {:?} applied", b)),
                        Err(e) => {
                            dlog::log(&format!("custom CPU boost {:?} ERR: {}", b, e));
                            problems.push(format!("Failed to apply CPU {:?}: {}", b, e));
                        }
                    }
                }
                if let Some(b) = want_gpu {
                    match with_retries(|| command::set_gpu_boost(device, b)) {
                        Ok(_) => dlog::log(&format!("custom GPU boost {:?} applied", b)),
                        Err(e) => {
                            dlog::log(&format!("custom GPU boost {:?} ERR: {}", b, e));
                            problems.push(format!("Failed to apply GPU {:?}: {}", b, e));
                        }
                    }
                }
                // show what the laptop really holds now
                if let Ok(v) = command::get_cpu_boost(device) {
                    self.cpu_boost = v;
                }
                if let Ok(v) = command::get_gpu_boost(device) {
                    self.gpu_boost = v;
                }
                dlog::log(&format!(
                    "custom boosts now: cpu={:?} gpu={:?}",
                    self.cpu_boost, self.gpu_boost
                ));
            }
            for p in problems {
                self.set_error_message(p);
            }

            self.set_status_message(format!("⚡ Auto-switched to {} profile", profile_name));
        }

        // Refresh the panel from the laptop. A failed read straight after a mode change is normal
        // (the two zones disagree for a moment). It is never answered by writing a default profile
        // (logo off, keyboard 50, lights, battery care) over the laptop's real settings, which
        // is what used to happen here.
        if self.read_device_status().is_err() {
            dlog::log("auto_switch_profile: status read failed, keeping the panel as it is");
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

    #[allow(dead_code)]
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
                            self.cfg.custom_cpu = Some(format!("{:?}", boost));
                            config::save(&self.cfg);
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
                            self.cfg.custom_gpu = Some(format!("{:?}", boost));
                            config::save(&self.cfg);
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

    /// The colors to send for `effect`, from the saved settings.
    fn kbd_colors(
        &self,
        effect: librazer::command::KeyboardEffect,
    ) -> librazer::command::KeyboardColors {
        if effect == librazer::command::KeyboardEffect::Fire {
            // the fire's own two colors: hot as the first, cold as the second
            return librazer::command::KeyboardColors {
                primary: self.cfg.fire_hot,
                secondary: Some(self.cfg.fire_cold),
                split: librazer::command::KeyboardSplit::default(),
            };
        }
        if effect == librazer::command::KeyboardEffect::Tidal {
            return librazer::command::KeyboardColors {
                primary: self.cfg.tidal_a,
                secondary: Some(self.cfg.tidal_b),
                split: librazer::command::KeyboardSplit::default(),
            };
        }
        if effect == librazer::command::KeyboardEffect::Starlight {
            return librazer::command::KeyboardColors {
                primary: self.cfg.starlight_a,
                secondary: self.cfg.starlight_two.then_some(self.cfg.starlight_b),
                split: librazer::command::KeyboardSplit::default(),
            };
        }
        if effect == librazer::command::KeyboardEffect::Dipp {
            return librazer::command::KeyboardColors {
                primary: self.cfg.dipp_a,
                secondary: Some(self.cfg.dipp_b),
                split: librazer::command::KeyboardSplit::default(),
            };
        }
        if effect == librazer::command::KeyboardEffect::Wheel {
            return librazer::command::KeyboardColors {
                primary: self.cfg.wheel_a,
                secondary: Some(self.cfg.wheel_b),
                split: librazer::command::KeyboardSplit::default(),
            };
        }
        if self.cfg.wave_custom
            && matches!(
                effect,
                librazer::command::KeyboardEffect::WaveRight | librazer::command::KeyboardEffect::WaveLeft
            )
        {
            return librazer::command::KeyboardColors {
                primary: self.cfg.wave_a,
                secondary: Some(self.cfg.wave_b),
                split: librazer::command::KeyboardSplit::default(),
            };
        }
        librazer::command::KeyboardColors {
            primary: self.cfg.kbd_color,
            secondary: (self.cfg.kbd_two && effect.uses_color()).then_some(self.cfg.kbd_color2),
            split: if effect == librazer::command::KeyboardEffect::Breathing {
                librazer::command::KeyboardSplit::from_breath_index(self.cfg.kbd_breath_split)
            } else {
                librazer::command::KeyboardSplit::from_index(self.cfg.kbd_split)
            },
        }
    }

    fn set_keyboard_effect(&mut self, effect: librazer::command::KeyboardEffect) -> bool {
        let colors = self.kbd_colors(effect);
        let opts = librazer::command::KeyboardEffectOptions {
            legacy: self.cfg.kbd_legacy,
            random_id: self.cfg.kbd_random_id,
            clear_first: self.cfg.kbd_clear_first,
            static_frame: self.cfg.kbd_static_frame,
        };
        dlog::log(&format!("keyboard effect: {} {:?} {:?}", effect.name(), colors, opts));
        // Whatever was animating stops first (this waits for the frame being sent).
        self.kbd_anim.stop();
        let layout = self.cfg.kbd_custom.clone();
        let shift = self.cfg.kbd_key_shift;
        let speed = self.cfg.kbd_breath_speed;
        let (fire_speed, tidal_speed, starlight_speed, spectrum_speed, dipp_speed, rainbow_speed) = (
            self.cfg.fire_speed,
            self.cfg.tidal_speed,
            self.cfg.starlight_speed,
            self.cfg.spectrum_speed,
            self.cfg.dipp_speed,
            self.cfg.rainbow_speed,
        );
        let (wave_speed, wheel_speed, wheel_ccw, wheel_rainbow) =
            (self.cfg.wave_speed, self.cfg.wheel_speed, self.cfg.wheel_ccw, !self.cfg.wheel_custom);
        let own_wave = self.cfg.wave_custom
            && matches!(
                effect,
                librazer::command::KeyboardEffect::WaveRight | librazer::command::KeyboardEffect::WaveLeft
            );
        let kbd_anim = &self.kbd_anim;
        let result = if effect == librazer::command::KeyboardEffect::Custom && layout.is_animated() {
            // keys that move are drawn by the worker, a frame at a time
            let moving = layout.clone();
            execute_device_command_simple(
                self.device.as_ref(),
                |device| {
                    kbd_anim.start_keys(device.info().clone(), moving, shift, speed);
                    Ok(())
                },
                "Keyboard set to Custom Keys",
                "Failed to set keyboard effect",
            )
        } else if effect == librazer::command::KeyboardEffect::Custom {
            execute_device_command_simple(
                self.device.as_ref(),
                |device| command::set_keyboard_layout(device, &layout, shift),
                "Keyboard set to Custom Keys",
                "Failed to set keyboard effect",
            )
        } else if effect == librazer::command::KeyboardEffect::Fire {
            // drawn by the worker, a frame at a time, like split Breathing
            execute_device_command_simple(
                self.device.as_ref(),
                |device| {
                    kbd_anim.start_fire(
                        device.info().clone(),
                        colors.primary,
                        colors.secondary.unwrap_or(colors.primary),
                        fire_speed,
                    );
                    Ok(())
                },
                "Keyboard effect set to Fire",
                "Failed to set keyboard effect",
            )
        } else if effect == librazer::command::KeyboardEffect::Tidal {
            // drawn by the worker too
            let inward = self.cfg.tidal_in;
            execute_device_command_simple(
                self.device.as_ref(),
                |device| {
                    kbd_anim.start_tidal(
                        device.info().clone(),
                        colors.primary,
                        colors.secondary.unwrap_or(colors.primary),
                        inward,
                        tidal_speed,
                    );
                    Ok(())
                },
                "Keyboard effect set to Tidal",
                "Failed to set keyboard effect",
            )
        } else if effect == librazer::command::KeyboardEffect::Starlight {
            // drawn by the worker too
            execute_device_command_simple(
                self.device.as_ref(),
                |device| {
                    kbd_anim.start_starlight(
                        device.info().clone(),
                        colors.primary,
                        colors.secondary,
                        starlight_speed,
                    );
                    Ok(())
                },
                "Keyboard effect set to Starlight",
                "Failed to set keyboard effect",
            )
        } else if effect == librazer::command::KeyboardEffect::Dipp {
            // drawn by the worker too
            execute_device_command_simple(
                self.device.as_ref(),
                |device| {
                    kbd_anim.start_dipp(
                        device.info().clone(),
                        colors.primary,
                        colors.secondary.unwrap_or(colors.primary),
                        dipp_speed,
                    );
                    Ok(())
                },
                "Keyboard effect set to Color Dipp",
                "Failed to set keyboard effect",
            )
        } else if effect == librazer::command::KeyboardEffect::Wheel {
            // drawn by the worker too: a hand sweeping round the middle of the keyboard
            execute_device_command_simple(
                self.device.as_ref(),
                |device| {
                    kbd_anim.start_wheel(
                        device.info().clone(),
                        colors.primary,
                        colors.secondary.unwrap_or(colors.primary),
                        wheel_ccw,
                        wheel_rainbow,
                        wheel_speed,
                    );
                    Ok(())
                },
                "Keyboard effect set to Wheel",
                "Failed to set keyboard effect",
            )
        } else if own_wave {
            // the wave in colors of your own is drawn by the worker (the firmware's is a rainbow)
            let left = effect == librazer::command::KeyboardEffect::WaveLeft;
            execute_device_command_simple(
                self.device.as_ref(),
                |device| {
                    kbd_anim.start_wave(device.info().clone(), colors.primary, colors.secondary, left, wave_speed);
                    Ok(())
                },
                &format!("Keyboard effect set to {}", effect.name()),
                "Failed to set keyboard effect",
            )
        } else if effect == librazer::command::KeyboardEffect::Rainbow {
            // drawn by the worker too: the whole wheel across the keys, scrolling along
            execute_device_command_simple(
                self.device.as_ref(),
                |device| {
                    kbd_anim.start_rainbow(device.info().clone(), rainbow_speed);
                    Ok(())
                },
                "Keyboard effect set to Rainbow",
                "Failed to set keyboard effect",
            )
        } else if effect == librazer::command::KeyboardEffect::Spectrum {
            // drawn by the worker, so it goes round the rainbow like Razer Chroma's
            execute_device_command_simple(
                self.device.as_ref(),
                |device| {
                    kbd_anim.start_spectrum(device.info().clone(), spectrum_speed);
                    Ok(())
                },
                "Keyboard effect set to Spectrum",
                "Failed to set keyboard effect",
            )
        } else if command::is_software_breathing(effect, &colors, speed) {
            // The firmware cannot do halves / a gradient / a custom speed: the worker animates it.
            execute_device_command_simple(
                self.device.as_ref(),
                |device| {
                    kbd_anim.start(device.info().clone(), colors, speed);
                    Ok(())
                },
                "Keyboard effect set to Breathing",
                "Failed to set keyboard effect",
            )
        } else {
            execute_device_command_simple(
                self.device.as_ref(),
                |device| command::set_keyboard_effect(device, effect, colors, opts),
                &format!("Keyboard effect set to {}", effect.name()),
                "Failed to set keyboard effect",
            )
        };
        match result {
            Ok(message) => {
                self.kbd_effect = effect.name().to_string();
                self.cfg.kbd_last_effect = effect.key().to_string();
                self.set_optional_status_message(message);
                config::save(&self.cfg);
                true
            }
            Err(message) => {
                self.set_error_message(message);
                false
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

        // Sync only takes over the keyboard controls while the pad (the other half) is connected.
        let sync_active = self.cfg.light_sync && self.pad.snapshot().present;

        let action = render_lighting_section(
            ui,
            &self.status.logo_mode,
            &mut self.temp_brightness_step,
            &mut self.status.lights_always_on,
            &mut self.cfg,
            &self.kbd_effect,
            sync_active,
        );

        if let Some(on) = action.kbd_lighting_toggled {
            if !on {
                self.kbd_anim.stop();
                self.kbd_effect.clear();
            }
            config::save(&self.cfg);
        }
        if let Some(effect) = action.kbd_effect {
            self.set_keyboard_effect(effect);
        }
        if action.kbd_apply_color {
            if let Some(effect) = librazer::command::KeyboardEffect::from_name(&self.kbd_effect) {
                if effect == librazer::command::KeyboardEffect::Fire {
                    // a new hot or cold color: the flame keeps burning, only its colors change
                    if let Some(device) = self.device.as_ref() {
                        self.kbd_anim.start_fire(
                            device.info().clone(),
                            self.cfg.fire_hot,
                            self.cfg.fire_cold,
                            self.cfg.fire_speed,
                        );
                    }
                    config::save(&self.cfg);
                } else if effect == librazer::command::KeyboardEffect::Tidal {
                    // new colors or direction: the waves keep running
                    if let Some(device) = self.device.as_ref() {
                        self.kbd_anim.start_tidal(
                            device.info().clone(),
                            self.cfg.tidal_a,
                            self.cfg.tidal_b,
                            self.cfg.tidal_in,
                            self.cfg.tidal_speed,
                        );
                    }
                    config::save(&self.cfg);
                } else if effect == librazer::command::KeyboardEffect::Dipp {
                    // new colors: the slide goes on from where it is
                    if let Some(device) = self.device.as_ref() {
                        self.kbd_anim.start_dipp(
                            device.info().clone(),
                            self.cfg.dipp_a,
                            self.cfg.dipp_b,
                            self.cfg.dipp_speed,
                        );
                    }
                    config::save(&self.cfg);
                } else if effect == librazer::command::KeyboardEffect::Wheel {
                    // new colors, direction or speed: the hand keeps going round
                    if let Some(device) = self.device.as_ref() {
                        self.kbd_anim.start_wheel(
                            device.info().clone(),
                            self.cfg.wheel_a,
                            self.cfg.wheel_b,
                            self.cfg.wheel_ccw,
                            !self.cfg.wheel_custom,
                            self.cfg.wheel_speed,
                        );
                    }
                    config::save(&self.cfg);
                } else if matches!(
                    effect,
                    librazer::command::KeyboardEffect::WaveRight | librazer::command::KeyboardEffect::WaveLeft
                ) {
                    if self.cfg.wave_custom && self.kbd_anim.is_running() {
                        // new colors or speed: the bands keep travelling
                        if let Some(device) = self.device.as_ref() {
                            self.kbd_anim.start_wave(
                                device.info().clone(),
                                self.cfg.wave_a,
                                Some(self.cfg.wave_b),
                                effect == librazer::command::KeyboardEffect::WaveLeft,
                                self.cfg.wave_speed,
                            );
                        }
                        config::save(&self.cfg);
                    } else {
                        // colors switched on or off: the effect is sent again
                        self.set_keyboard_effect(effect);
                    }
                } else if effect == librazer::command::KeyboardEffect::Rainbow {
                    // a new speed: the rainbow keeps scrolling
                    if let Some(device) = self.device.as_ref() {
                        self.kbd_anim.start_rainbow(device.info().clone(), self.cfg.rainbow_speed);
                    }
                    config::save(&self.cfg);
                } else if effect == librazer::command::KeyboardEffect::Spectrum {
                    // a new speed: the rainbow keeps going round
                    if let Some(device) = self.device.as_ref() {
                        self.kbd_anim.start_spectrum(device.info().clone(), self.cfg.spectrum_speed);
                    }
                    config::save(&self.cfg);
                } else if effect == librazer::command::KeyboardEffect::Starlight {
                    // new colors: the stars keep twinkling
                    if let Some(device) = self.device.as_ref() {
                        self.kbd_anim.start_starlight(
                            device.info().clone(),
                            self.cfg.starlight_a,
                            self.cfg.starlight_two.then_some(self.cfg.starlight_b),
                            self.cfg.starlight_speed,
                        );
                    }
                    config::save(&self.cfg);
                } else if effect.uses_color() {
                    self.set_keyboard_effect(effect);
                }
            }
        }
        if action.kbd_custom_changed {
            let custom = librazer::command::KeyboardEffect::Custom;
            if librazer::command::KeyboardEffect::from_name(&self.kbd_effect) == Some(custom) {
                self.set_keyboard_effect(custom);
            }
        }

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

    fn pad_light_target(&self) -> Option<pad::LightTarget> {
        let mode = librazer::cooling_pad::PadLightingMode::from_name(&self.pad_light_mode)?;
        Some(pad::light_target(&self.cfg, mode))
    }

    /// Cooling Pad fan card, shown only while a Razer Laptop Cooling Pad is connected.
    /// Returns true when the card was drawn.
    fn render_cooling_pad_section(&mut self, ui: &mut egui::Ui) -> bool {
        let status = self.pad.snapshot();
        if !status.present {
            return false;
        }

        let fan_mode = self.cfg.pad_fan.clone();
        let action = ui::cooling_pad::render_cooling_pad_section(ui, &fan_mode, &mut self.cfg);

        let mut save = false;
        if let Some(on) = action.control_toggled {
            self.pad.send(pad::PadCmd::FanControl(on));
            if on {
                // Put the last fan setting back right away (it is also restored at startup).
                match self.cfg.pad_fan.as_str() {
                    "manual" => {
                        self.pad.send(pad::PadCmd::Fan(pad::FanTarget::Rpm(self.cfg.pad_rpm)))
                    }
                    "off" => self.pad.send(pad::PadCmd::Fan(pad::FanTarget::Off)),
                    _ => {}
                }
            }
            save = true;
        }
        if self.cfg.pad_control {
            if action.fan_manual {
                self.cfg.pad_fan = "manual".to_string();
                self.pad.send(pad::PadCmd::Fan(pad::FanTarget::Rpm(self.cfg.pad_rpm)));
                save = true;
            }
            if action.fan_off {
                self.cfg.pad_fan = "off".to_string();
                self.pad.send(pad::PadCmd::Fan(pad::FanTarget::Off));
                save = true;
            }
            if let Some(r) = action.fan_rpm {
                if self.cfg.pad_fan == "manual" {
                    self.pad.send(pad::PadCmd::Fan(pad::FanTarget::Rpm(r)));
                }
                save = true;
            }
            if let Some(i) = action.apply_profile {
                if let Some(profile) = self.cfg.pad_profiles.get(i) {
                    let rpm = profile.rpm;
                    self.cfg.pad_rpm = rpm;
                    self.cfg.pad_fan = "manual".to_string();
                    self.pad.send(pad::PadCmd::Fan(pad::FanTarget::Rpm(rpm)));
                    save = true;
                }
            }
        }
        if action.profiles_changed {
            save = true;
        }
        if save {
            config::save(&self.cfg);
        }
        true
    }

    /// Keeps the tray icon in line with the options and handles what the icon asked for. The icon
    /// is there while "Close button minimizes to tray" or "Start minimized to tray" is on.
    /// The Gold Edition glitch: when the king's laugh reaches its peak, take a screenshot of the
    /// window, then play it back broken into pixels and torn strips for a moment.
    fn run_glitch(&mut self, ctx: &egui::Context, minimized: bool) {
        use ui::glitch::Glitch;
        let now = ctx.input(|i| i.time);
        // start one at the peak of a laugh
        if let Some(c) = self.king_laugh {
            if matches!(self.glitch, Glitch::Off)
                && self.glitch_for != Some(c)
                && now - c >= f64::from(king::GLITCH_AT)
                && self.cfg.gold_active()
                && self.cfg.gold_glitch
                && !minimized
            {
                self.glitch_for = Some(c);
                ui::glitch::request(ctx);
                self.glitch = Glitch::Waiting(now);
                ctx.request_repaint();
            }
        }
        match std::mem::take(&mut self.glitch) {
            Glitch::Off => {}
            Glitch::Waiting(since) => {
                if let Some(image) = ui::glitch::arrived(ctx) {
                    let shots = ui::glitch::shots(ctx, &image);
                    self.glitch = Glitch::Running(now, shots);
                    ctx.request_repaint();
                } else if now - since < 1.5 {
                    self.glitch = Glitch::Waiting(since);
                    ctx.request_repaint();
                }
                // no screenshot came: give up quietly
            }
            Glitch::Running(start, shots) => {
                if ui::glitch::paint(ctx, &shots, (now - start) as f32) {
                    self.glitch = Glitch::Running(start, shots);
                }
            }
        }
    }

    /// The glitch of The Void's moon: it starts the moment the moon is taken hold of and gets worse
    /// the further it is pulled, in violet and cyan. It is the same screenshot trick as the Gold
    /// Edition's, but the strength follows the moon, and the picture is taken again now and then so
    /// that what the strips show is not too far behind.
    fn run_moon_glitch(&mut self, ctx: &egui::Context, minimized: bool) {
        use ui::glitch::{Glitch, Tint};
        let now = ctx.input(|i| i.time);
        let level = self.moon_level;
        let wanted = level >= 0.03 && !minimized && matches!(self.glitch, Glitch::Off);
        if !wanted {
            self.moon_glitch = Glitch::Off;
            return;
        }
        match std::mem::take(&mut self.moon_glitch) {
            Glitch::Off => {
                ui::glitch::request(ctx);
                self.moon_glitch = Glitch::Waiting(now);
                ctx.request_repaint();
            }
            Glitch::Waiting(since) => {
                if let Some(image) = ui::glitch::arrived(ctx) {
                    let shots = ui::glitch::shots(ctx, &image);
                    self.moon_shot_at = now;
                    self.moon_glitch = Glitch::Running(now, shots);
                } else if now - since < 1.5 {
                    self.moon_glitch = Glitch::Waiting(since);
                }
                // (no screenshot came: give up quietly, it asks again next frame)
                ctx.request_repaint();
            }
            Glitch::Running(start, shots) => {
                if now - self.moon_shot_at > 0.8 {
                    // a fresh picture; the strips are not drawn meanwhile so that they are not in it
                    ui::glitch::request(ctx);
                    self.moon_glitch = Glitch::Waiting(now);
                    ctx.request_repaint();
                } else {
                    ui::glitch::paint_level(ctx, &shots, (now % 3600.0) as f32, level, Tint::Void);
                    self.moon_glitch = Glitch::Running(start, shots);
                }
            }
        }
    }

    fn update_tray(&mut self, ctx: &egui::Context) {
        if self.cfg.close_to_tray || self.cfg.start_minimized {
            if self.tray.is_none() && !self.tray_failed {
                self.tray = tray::Tray::start(ctx.clone());
                self.tray_failed = self.tray.is_none();
                if self.tray_failed {
                    self.set_error_message(
                        "Could not add the tray icon; the close button will quit.".to_string(),
                    );
                    if self.in_tray {
                        // started hidden, but there is no icon to bring the window back with
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        self.in_tray = false;
                    }
                }
            }
        } else {
            self.tray_failed = false;
            if self.tray.take().is_some() && self.in_tray {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            }
            self.in_tray = false;
        }
        if let Some(tray) = &self.tray {
            if tray.take_shown() {
                self.in_tray = false;
            }
            if tray.quit_requested() {
                self.should_quit = true;
            }
        }
    }

    /// Reacts to the Log viewer (inside the Logging card of the Sensors tab).
    fn handle_log_view_action(&mut self, action: ui::log_view::LogViewAction) {
        if action.refresh {
            self.log_view.listed = false;
        }
        if action.open_folder {
            let dir = sensor_log::log_dir();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::process::Command::new("explorer").arg(&dir).spawn();
        }
        if let Some(path) = action.open {
            self.open_log(&path);
        }
        if action.open_window {
            if let Some(path) = self.log_view.path.clone() {
                self.open_log(&path);
            }
        }
        if action.browse {
            let start = sensor_log::log_dir().to_string_lossy().into_owned();
            self.file_pick =
                Some((picker::Target::ViewLog, picker::spawn(picker::Target::ViewLog, &start)));
        }
    }

    /// The log viewer window: reads a log that is still being written again every few seconds.
    fn render_log_window(&mut self, ctx: &egui::Context) {
        let Some(win) = self.log_window.as_mut() else {
            return;
        };
        win.live = self.sensors.log_info().path.as_deref() == Some(win.path.as_path());
        if win.due_reload(ctx.input(|i| i.time)) {
            if let Ok(loaded) = log_view::load(&win.path) {
                win.reloaded(&loaded);
            }
        }
        let keep = ui::log_window::show(ctx, win);
        // the settings of the window are remembered between launches
        if let Some(text) = win.take_options() {
            self.cfg.log_view_options = text;
            config::save(&self.cfg);
        }
        if !keep {
            self.log_window = None;
        }
    }

    /// Loads a log into the viewer (and shows why when it cannot).
    fn open_log(&mut self, path: &std::path::Path) {
        match log_view::load(path) {
            Ok(loaded) => {
                // the graphs get a window of their own (so that they can be big)
                self.log_window = Some(ui::log_window::LogWindow::new(
                    &loaded,
                    path.to_path_buf(),
                    log_options::Options::decode(&self.cfg.log_view_options),
                ));
                self.log_view.loaded = Some(loaded);
                self.log_view.path = Some(path.to_path_buf());
                self.log_view.error = None;
            }
            Err(message) => {
                self.log_view.loaded = None;
                self.log_view.path = None;
                self.log_view.error = Some(message);
            }
        }
    }

    /// Info tab: the author's note (built in, read-only).
    fn render_info_tab(&mut self, ui: &mut egui::Ui) {
        if let Some((width, height, source)) = &self.info_logo_src {
            let accent = theme::accent();
            let accent =
                if self.logo_recolor { [accent.r(), accent.g(), accent.b()] } else { [0, 0, 0] };
            if self.info_logo_accent != Some(accent) {
                let pixels = if self.logo_recolor {
                    info::recolor_logo(source, accent)
                } else {
                    source.clone()
                };
                self.info_logo = Some(ui.ctx().load_texture(
                    "cpunity_logo_full",
                    egui::ColorImage::from_rgba_unmultiplied([*width, *height], &pixels),
                    egui::TextureOptions::LINEAR,
                ));
                self.info_logo_accent = Some(accent);
            }
        }
        // The Void shows the text in its own script (hover a word to read it)
        let script = self.cfg.void_active() && self.cfg.void_script;
        // with Father Time's clock at the top of the page the big logo is not shown
        let father = self.cfg.backdrop
            && !self.cfg.underworld_active()
            && (self.cfg.backdrop_style.has_clock()
                || self.cfg.shown_backdrop_style() == crate::info::BackdropStyle::DeadKing
                || self.cfg.shown_backdrop_style() == crate::info::BackdropStyle::Cat);
        // the Gold Edition shows its cat king in the logo's place
        if !father && self.cfg.gold_active() && self.logo_recolor && self.gold_arena.is_some() {
            let minimized = ui.ctx().input(|i| i.viewport().minimized.unwrap_or(false)) || self.in_tray;
            if let Some(arena) = &self.gold_arena {
                ui::king::paint_king(ui, arena, &mut self.king_laugh, !minimized, self.cfg.backdrop_animated);
            }
            ui.add_space(8.0);
            ui::info::render_info(ui, None, script, self.cfg.void_seed);
        } else {
            ui::info::render_info(ui, if father { None } else { self.info_logo.as_ref() }, script, self.cfg.void_seed);
        }
    }

    /// Reacts to what happened in the Files tab: saves, restarts the readings, opens "Browse...".
    fn apply_files_action(&mut self, action: ui::files::FilesAction) {
        if action.changed {
            config::save(&self.cfg);
        }
        if action.restart && self.sensors.is_on() {
            self.sensors.stop();
            self.sensors.start(self.cfg.sensor_source_config());
        }
        if let Some(target) = action.pick {
            let initial = match target {
                picker::Target::LogFile => self.cfg.sensor_log_path.clone(),
                picker::Target::ModuleFile => self.cfg.sensor_module_path.clone(),
                picker::Target::ViewLog | picker::Target::Song => String::new(),
            };
            self.file_pick = Some((target, picker::spawn(target, &initial)));
        }
    }

    /// Collects the answer of the "Browse..." dialog.
    fn poll_file_pick(&mut self, ctx: &egui::Context) {
        let Some((target, receiver)) = &self.file_pick else {
            return;
        };
        let target = *target;
        match receiver.try_recv() {
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
            Err(mpsc::TryRecvError::Disconnected) | Ok(None) => self.file_pick = None,
            Ok(Some(path)) => {
                self.file_pick = None;
                // choosing a file means the source should be used
                match target {
                    picker::Target::LogFile => {
                        self.cfg.sensor_log_path = path;
                        self.cfg.sensor_enabled[sensor_ext::LOG] = true;
                    }
                    picker::Target::ModuleFile => {
                        self.cfg.sensor_module_path = path;
                        self.cfg.sensor_enabled[sensor_ext::PAWN] = true;
                    }
                    picker::Target::ViewLog => {
                        // only looked at: no setting changes, nothing to restart
                        self.open_log(std::path::Path::new(&path));
                        return;
                    }
                    picker::Target::Song => {
                        // an mp3 for Game 3: it is listened to on a thread of its own
                        self.start_song_import(path);
                        return;
                    }
                }
                config::save(&self.cfg);
                if self.sensors.is_on() {
                    self.sensors.stop();
                    self.sensors.start(self.cfg.sensor_source_config());
                }
            }
        }
    }

    /// Sensors tab: opt-in readings. Nothing is read until the switch is on.
    fn render_sensors_section(&mut self, ui: &mut egui::Ui) {
        let mut on = self.sensors.is_on();
        let readings = self.sensors.snapshot();
        let log = self.sensors.log_info();
        // a log with a time limit switches the logging switch off by itself when it is done
        self.cfg.sensor_logging = self.sensors.logging();
        let mut logging = self.cfg.sensor_logging;
        let mut log_name = self.cfg.sensor_log_name.clone();
        let mut log_settings = ui::sensors::LogSettings::of(&self.cfg);
        let mut graphs = self.cfg.sensor_graphs;
        let mut window = self.cfg.graph_window;
        let mut interval = self.cfg.sensor_interval_ms;
        if !self.log_view.listed {
            self.log_view.files = log_view::list_logs(&sensor_log::log_dir());
            self.log_view.listed = true;
        }
        let picking = self.file_pick.is_some();
        let log_view_state = &mut self.log_view;
        let mut action = self.sensors.with_history(|history| {
            let view = ui::sensors::SensorsView { readings: &readings, history, log: &log, programs: &self.log_programs };
            ui::sensors::render_sensors_section(
                ui,
                &mut on,
                &mut logging,
                &mut log_name,
                &mut log_settings,
                &mut graphs,
                &mut window,
                &mut interval,
                log_view_state,
                picking,
                &view,
            )
        });
        self.handle_log_view_action(std::mem::take(&mut action.log_view));
        let mut save = false;
        if let Some(wanted) = action.toggled {
            self.cfg.sensor_read = wanted;
            save = true;
            if wanted {
                self.sensors.start(self.cfg.sensor_source_config());
            } else {
                // reading off means logging off too: nothing is written, and no log starts by itself
                // when the readings are switched on again
                self.sensors.set_logging(false);
                self.cfg.sensor_logging = false;
                self.sensors.stop();
            }
        }
        if action.reset {
            self.sensors.reset();
        }
        if let Some(wanted) = action.logging {
            self.cfg.sensor_logging = wanted;
            self.sensors.set_logging(wanted);
            save = true;
        }
        if let Some(changed) = action.log_settings {
            changed.apply(&mut self.cfg);
            self.sensors.set_log_plan(self.cfg.log_plan());
        }
        if let Some(path) = action.view_log {
            self.open_log(&path);
        }
        if action.log_settings_commit {
            save = true;
        }
        if action.refresh_programs {
            self.log_programs = log_plan::running_programs();
        }
        if let Some(name) = action.log_name {
            self.sensors.set_log_name(&name);
            self.cfg.sensor_log_name = name;
        }
        if action.log_name_commit {
            save = true;
        }
        if action.window_changed {
            self.cfg.graph_window = window;
            save = true;
        }
        if let Some(wanted) = action.graphs {
            self.cfg.sensor_graphs = wanted;
            self.sensors.set_history(wanted);
            save = true;
        }
        if let Some(ms) = action.interval {
            self.cfg.sensor_interval_ms = ms;
            self.sensors.set_interval(ms);
        }
        // while the slider is dragged only the running sampler follows; the file is written once
        // when it is released
        if action.interval_commit {
            save = true;
        }
        if action.open_logs {
            let dir = sensor_log::log_dir();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::process::Command::new("explorer").arg(&dir).spawn();
        }
        if save {
            config::save(&self.cfg);
        }
    }

    /// Sync card (Lighting tab): one set of controls for the keyboard and the cooling pad.
    /// Returns true when the card was drawn (only while a pad is connected).
    fn render_sync_section(&mut self, ui: &mut egui::Ui) -> bool {
        let status = self.pad.snapshot();
        if !status.present {
            return false;
        }

        let action = ui::lighting::render_sync_section(ui, &mut self.cfg, &self.kbd_effect);

        let mut save = false;
        if let Some(on) = action.toggled {
            if on {
                // Fold the separate keyboard and pad lighting drop-downs away.
                theme::set_open("kbd_lighting_header", false);
                theme::set_open("pad_lighting_header", false);
                // The sync switch is the opt-in for both devices.
                self.cfg.kbd_lighting = true;
                if !self.cfg.pad_lighting {
                    self.cfg.pad_lighting = true;
                    self.pad.send(pad::PadCmd::LightingEnabled(true));
                }
            }
            save = true;
        }
        if let Some(effect) = action.effect {
            self.apply_sync(effect, true, status.lighting_available);
        } else if action.apply {
            if let Some(effect) = librazer::command::KeyboardEffect::from_name(&self.kbd_effect) {
                self.apply_sync(effect, false, status.lighting_available);
            }
        }
        if let Some(p) = action.pad_brightness {
            self.pad.send(pad::PadCmd::Brightness(pad::pct_to_raw(p)));
            save = true;
        }
        if save {
            config::save(&self.cfg);
        }
        true
    }

    /// Sends the shared (keyboard) colors, layout and speed to the keyboard and the pad.
    /// `mode_changed` = the effect itself changed, not only its colors.
    fn apply_sync(
        &mut self,
        effect: librazer::command::KeyboardEffect,
        mode_changed: bool,
        pad_available: bool,
    ) {
        let Some(pad_mode) = ui::lighting::pad_mode_for(effect) else {
            return;
        };

        // The pad's settings follow the keyboard's. The pad has two halves and a gradient;
        // Top | Bottom becomes two halves there.
        self.cfg.pad_color = self.cfg.kbd_color;
        self.cfg.pad_color2 = self.cfg.kbd_color2;
        self.cfg.pad_two = self.cfg.kbd_two;
        self.cfg.pad_split = u8::from(
            librazer::command::KeyboardSplit::from_index(self.cfg.kbd_split)
                == librazer::command::KeyboardSplit::Gradient,
        );
        self.cfg.pad_breath_split = match self.cfg.kbd_breath_split {
            0 => 0,
            1 | 2 => 1,
            _ => 2,
        };
        self.cfg.pad_breath_speed = self.cfg.kbd_breath_speed;
        // (Sync: the pad goes at the keyboard's speed)
        self.cfg.pad_fire_speed = self.cfg.fire_speed;
        self.cfg.pad_tidal_speed = self.cfg.tidal_speed;
        self.cfg.pad_starlight_speed = self.cfg.starlight_speed;
        self.cfg.pad_spectrum_speed = self.cfg.spectrum_speed;
        self.cfg.pad_dipp_speed = self.cfg.dipp_speed;
        self.cfg.pad_rainbow_speed = self.cfg.rainbow_speed;
        self.cfg.pad_wheel_speed = self.cfg.wheel_speed;
        self.cfg.pad_wheel_ccw = self.cfg.wheel_ccw;
        self.cfg.pad_wheel_custom = self.cfg.wheel_custom;
        self.cfg.pad_wheel_a = self.cfg.wheel_a;
        self.cfg.pad_wheel_b = self.cfg.wheel_b;
        if effect == librazer::command::KeyboardEffect::Fire {
            // Sync: the pad burns in the keyboard's Fire colors. (Without Sync the two have
            // colors of their own.)
            self.cfg.pad_fire_hot = self.cfg.fire_hot;
            self.cfg.pad_fire_cold = self.cfg.fire_cold;
        }
        if effect == librazer::command::KeyboardEffect::Starlight {
            self.cfg.pad_starlight_a = self.cfg.starlight_a;
            self.cfg.pad_starlight_b = self.cfg.starlight_b;
            self.cfg.pad_starlight_two = self.cfg.starlight_two;
        }
        if effect == librazer::command::KeyboardEffect::Dipp {
            self.cfg.pad_dipp_a = self.cfg.dipp_a;
            self.cfg.pad_dipp_b = self.cfg.dipp_b;
        }
        if effect == librazer::command::KeyboardEffect::Tidal {
            self.cfg.pad_tidal_a = self.cfg.tidal_a;
            self.cfg.pad_tidal_b = self.cfg.tidal_b;
            self.cfg.pad_tidal_in = self.cfg.tidal_in;
        }

        if self.cfg.kbd_lighting {
            self.set_keyboard_effect(effect);
        }
        if self.cfg.pad_lighting && pad_available {
            self.pad_light_mode = pad_mode.name().to_string();
            self.cfg.pad_last_mode = pad_mode.name().to_string();
            if let Some(target) = self.pad_light_target() {
                if mode_changed {
                    self.pad.send(pad::PadCmd::Light(target));
                } else if pad_mode != librazer::cooling_pad::PadLightingMode::Off {
                    self.pad.send(pad::PadCmd::LightColor(target));
                }
            }
        }
        config::save(&self.cfg);
    }

    /// Cooling pad lighting card (Lighting tab). Returns true when the card was drawn.
    fn render_pad_lighting_section(&mut self, ui: &mut egui::Ui) -> bool {
        let status = self.pad.snapshot();
        if !status.present {
            return false;
        }

        let action = ui::cooling_pad::render_pad_lighting_section(
            ui,
            &mut self.cfg,
            &self.pad_light_mode,
            status.lighting_available,
        );

        let mut save = false;
        if let Some(on) = action.lighting_toggled {
            if !on {
                self.pad_light_mode.clear();
            }
            self.pad.send(pad::PadCmd::LightingEnabled(on));
            save = true;
        }
        if let Some(mode) = action.light_mode {
            self.pad_light_mode = mode.name().to_string();
            self.cfg.pad_last_mode = mode.name().to_string();
            if let Some(target) = self.pad_light_target() {
                self.pad.send(pad::PadCmd::Light(target));
            }
            save = true;
        }
        if action.apply_color {
            if let Some(target) = self.pad_light_target() {
                if target.mode != librazer::cooling_pad::PadLightingMode::Off {
                    self.pad.send(pad::PadCmd::LightColor(target));
                }
            }
            save = true;
        }
        if let Some(p) = action.brightness {
            self.pad.send(pad::PadCmd::Brightness(pad::pct_to_raw(p)));
            save = true;
        }
        if save {
            config::save(&self.cfg);
        }
        true
    }

    fn preset_bank_mut(
        &mut self,
        bank: ui::presets::Bank,
    ) -> &mut Vec<Option<config::LightPreset>> {
        match bank {
            ui::presets::Bank::Keyboard => &mut self.cfg.kbd_presets,
            ui::presets::Bank::Pad => &mut self.cfg.pad_light_presets,
            ui::presets::Bank::Both => &mut self.cfg.combo_presets,
        }
    }

    /// The keyboard lighting as it is right now, if the app has applied something this session.
    fn current_kbd_part(&self) -> Option<config::KbdPart> {
        let effect = librazer::command::KeyboardEffect::from_name(&self.kbd_effect)?;
        let two = self.cfg.kbd_two && effect.uses_color();
        let breathing = effect == librazer::command::KeyboardEffect::Breathing;
        let custom = effect == librazer::command::KeyboardEffect::Custom;
        let fire = effect == librazer::command::KeyboardEffect::Fire;
        let tidal = effect == librazer::command::KeyboardEffect::Tidal;
        let starlight = effect == librazer::command::KeyboardEffect::Starlight;
        let dipp = effect == librazer::command::KeyboardEffect::Dipp;
        let wheel = effect == librazer::command::KeyboardEffect::Wheel;
        let own_wave = self.cfg.wave_custom
            && matches!(
                effect,
                librazer::command::KeyboardEffect::WaveRight | librazer::command::KeyboardEffect::WaveLeft
            );
        // (the fire keeps its hot and cold colors in the two colors, the tide its two colors and,
        // in the layout, its direction: 1 = in; the stars their color and their second, if any;
        // Color Dipp its two colors; the wheel its two colors and its direction: 1 = counter-
        // clockwise; a wave in colors of your own its colors, with 3 in the layout)
        let (color, color2) = if wheel {
            (self.cfg.wheel_a, Some(self.cfg.wheel_b))
        } else if own_wave {
            (self.cfg.wave_a, Some(self.cfg.wave_b))
        } else if dipp {
            (self.cfg.dipp_a, Some(self.cfg.dipp_b))
        } else if fire {
            (self.cfg.fire_hot, Some(self.cfg.fire_cold))
        } else if starlight {
            (self.cfg.starlight_a, self.cfg.starlight_two.then_some(self.cfg.starlight_b))
        } else if tidal {
            (self.cfg.tidal_a, Some(self.cfg.tidal_b))
        } else {
            (self.cfg.kbd_color, two.then_some(self.cfg.kbd_color2))
        };
        Some(config::KbdPart {
            effect: effect.key().to_string(),
            color,
            color2,
            split: if breathing {
                self.cfg.kbd_breath_split
            } else if tidal {
                u8::from(self.cfg.tidal_in)
            } else if wheel {
                // (1 = counter-clockwise, 2 = the hand and tail in your own colors)
                u8::from(self.cfg.wheel_ccw) | (u8::from(self.cfg.wheel_custom) << 1)
            } else if own_wave {
                3
            } else {
                self.cfg.kbd_split
            },
            brightness: self.status.keyboard_brightness,
            layout: custom.then(|| self.cfg.kbd_custom.encode()),
        })
    }

    /// The pad lighting as it is right now, if the app has applied something this session.
    fn current_pad_part(&self) -> Option<config::PadPart> {
        let mode = librazer::cooling_pad::PadLightingMode::from_name(&self.pad_light_mode)?;
        let fire = mode == librazer::cooling_pad::PadLightingMode::Fire;
        let tidal = mode == librazer::cooling_pad::PadLightingMode::Tidal;
        let starlight = mode == librazer::cooling_pad::PadLightingMode::Starlight;
        let dipp = mode == librazer::cooling_pad::PadLightingMode::Dipp;
        let wheel = mode == librazer::cooling_pad::PadLightingMode::Wheel;
        // (the fire keeps its hot color here and its cold color in the second; the tide its two
        // colors and, in the layout, its direction: 1 = in; Color Dipp its two colors)
        let (color, color2) = if dipp {
            (self.cfg.pad_dipp_a, Some(self.cfg.pad_dipp_b))
        } else if wheel {
            // (the wheel in two colors of your own, or Razer's rainbow when there is no second)
            (self.cfg.pad_wheel_a, self.cfg.pad_wheel_custom.then_some(self.cfg.pad_wheel_b))
        } else if fire {
            (self.cfg.pad_fire_hot, Some(self.cfg.pad_fire_cold))
        } else if starlight {
            (self.cfg.pad_starlight_a, self.cfg.pad_starlight_two.then_some(self.cfg.pad_starlight_b))
        } else if tidal {
            (self.cfg.pad_tidal_a, Some(self.cfg.pad_tidal_b))
        } else {
            (self.cfg.pad_color, pad::light_second_color(self.cfg.pad_two, mode, self.cfg.pad_color2))
        };
        Some(config::PadPart {
            mode: mode.name().to_string(),
            color,
            color2,
            split: if mode == librazer::cooling_pad::PadLightingMode::Breathing {
                self.cfg.pad_breath_split
            } else if tidal {
                u8::from(self.cfg.pad_tidal_in)
            } else if mode == librazer::cooling_pad::PadLightingMode::Wheel {
                u8::from(self.cfg.pad_wheel_ccw)
            } else {
                self.cfg.pad_split
            },
            brightness: self.cfg.pad_brightness,
        })
    }

    fn save_preset(&mut self, bank: ui::presets::Bank, slot: usize) {
        use ui::presets::Bank;
        let (want_kbd, want_pad) = match bank {
            Bank::Keyboard => (true, false),
            Bank::Pad => (false, true),
            Bank::Both => (self.preset_ui.include_kbd, self.preset_ui.include_pad),
        };
        if !want_kbd && !want_pad {
            self.set_error_message("Tick Keyboard and/or Cooling pad to save".to_string());
            return;
        }
        let kbd = if want_kbd {
            match self.current_kbd_part() {
                Some(part) => Some(part),
                None => {
                    self.set_error_message(
                        "Keyboard: turn on Toggle App Lighting Control and pick an effect first"
                            .to_string(),
                    );
                    return;
                }
            }
        } else {
            None
        };
        let pad = if want_pad {
            match self.current_pad_part() {
                Some(part) => Some(part),
                None => {
                    self.set_error_message(
                        "Cooling pad: turn on Toggle App Lighting Control and pick a light mode first"
                            .to_string(),
                    );
                    return;
                }
            }
        } else {
            None
        };

        let typed = config::clean_preset_name(&self.preset_ui.name);
        let old_name =
            self.preset_bank_mut(bank).get(slot).and_then(|p| p.as_ref()).map(|p| p.name.clone());
        let name = if !typed.is_empty() {
            typed
        } else {
            old_name.unwrap_or_else(|| format!("Preset {}", slot + 1))
        };

        if let Some(entry) = self.preset_bank_mut(bank).get_mut(slot) {
            *entry = Some(config::LightPreset { name: name.clone(), kbd, pad });
        }
        self.preset_ui.name.clear();
        config::save(&self.cfg);
        self.set_status_message(format!("Saved \"{}\" in slot {}", name, slot + 1));
    }

    /// Applies the keyboard part of a preset. Needs the keyboard's lighting toggle to be on.
    fn apply_kbd_part(&mut self, part: &config::KbdPart) -> bool {
        if !self.cfg.kbd_lighting {
            self.set_error_message(
                "Keyboard: turn on Toggle App Lighting Control to apply this".to_string(),
            );
            return false;
        }
        let Some(effect) = librazer::command::KeyboardEffect::from_key(&part.effect) else {
            self.set_error_message("Keyboard: unknown effect in this preset".to_string());
            return false;
        };
        let is_wave = matches!(
            effect,
            librazer::command::KeyboardEffect::WaveRight | librazer::command::KeyboardEffect::WaveLeft
        );
        if is_wave {
            // (a wave saved with colors of its own has 3 in the layout, which no other wave has)
            self.cfg.wave_custom = part.split == 3;
        }
        if effect == librazer::command::KeyboardEffect::Wheel {
            self.cfg.wheel_a = part.color;
            if let Some(b) = part.color2 {
                self.cfg.wheel_b = b;
            }
            self.cfg.wheel_ccw = part.split & 1 == 1;
            self.cfg.wheel_custom = part.split & 2 == 2;
        } else if is_wave && part.split == 3 {
            self.cfg.wave_a = part.color;
            if let Some(b) = part.color2 {
                self.cfg.wave_b = b;
            }
        } else if effect == librazer::command::KeyboardEffect::Dipp {
            self.cfg.dipp_a = part.color;
            if let Some(b) = part.color2 {
                self.cfg.dipp_b = b;
            }
        } else if effect == librazer::command::KeyboardEffect::Fire {
            self.cfg.fire_hot = part.color;
            if let Some(cold) = part.color2 {
                self.cfg.fire_cold = cold;
            }
        } else if effect == librazer::command::KeyboardEffect::Starlight {
            self.cfg.starlight_a = part.color;
            self.cfg.starlight_two = part.color2.is_some();
            if let Some(b) = part.color2 {
                self.cfg.starlight_b = b;
            }
        } else if effect == librazer::command::KeyboardEffect::Tidal {
            self.cfg.tidal_a = part.color;
            if let Some(b) = part.color2 {
                self.cfg.tidal_b = b;
            }
            self.cfg.tidal_in = part.split == 1;
        } else {
            self.cfg.kbd_color = part.color;
            self.cfg.kbd_two = part.color2.is_some();
            if let Some(c2) = part.color2 {
                self.cfg.kbd_color2 = c2;
            }
        }
        if effect == librazer::command::KeyboardEffect::Breathing {
            self.cfg.kbd_breath_split = part.split.min(3);
        } else if !is_wave
            && !matches!(
                effect,
                librazer::command::KeyboardEffect::Tidal
                    | librazer::command::KeyboardEffect::Starlight
                    | librazer::command::KeyboardEffect::Dipp
                    | librazer::command::KeyboardEffect::Wheel
            )
        {
            self.cfg.kbd_split = part.split.min(2);
        }
        if effect == librazer::command::KeyboardEffect::Custom {
            if let Some(layout) =
                part.layout.as_deref().and_then(librazer::keymap::KeyLayout::decode)
            {
                self.cfg.kbd_custom = layout;
            }
        }
        if !self.set_keyboard_effect(effect) {
            return false; // the effect command failed; its error is already shown
        }
        if part.brightness != self.status.keyboard_brightness {
            self.set_brightness(part.brightness);
        }
        true
    }

    /// Applies the pad part of a preset. Needs the pad's lighting toggle on and a connected pad.
    fn apply_pad_part(&mut self, part: &config::PadPart) -> bool {
        use librazer::cooling_pad::PadLightingMode;
        let status = self.pad.snapshot();
        if !status.present {
            self.set_error_message("Cooling pad: not connected".to_string());
            return false;
        }
        if !self.cfg.pad_lighting || !status.lighting_available {
            self.set_error_message(
                "Cooling pad: turn on Toggle App Lighting Control to apply this".to_string(),
            );
            return false;
        }
        let Some(mode) = PadLightingMode::from_name(&part.mode) else {
            self.set_error_message("Cooling pad: unknown mode in this preset".to_string());
            return false;
        };
        if mode == PadLightingMode::Dipp {
            self.cfg.pad_dipp_a = part.color;
            if let Some(b) = part.color2 {
                self.cfg.pad_dipp_b = b;
            }
        } else if mode == PadLightingMode::Fire {
            self.cfg.pad_fire_hot = part.color;
            if let Some(cold) = part.color2 {
                self.cfg.pad_fire_cold = cold;
            }
        } else if mode == PadLightingMode::Starlight {
            self.cfg.pad_starlight_a = part.color;
            self.cfg.pad_starlight_two = part.color2.is_some();
            if let Some(b) = part.color2 {
                self.cfg.pad_starlight_b = b;
            }
        } else if mode == PadLightingMode::Tidal {
            self.cfg.pad_tidal_a = part.color;
            if let Some(b) = part.color2 {
                self.cfg.pad_tidal_b = b;
            }
            self.cfg.pad_tidal_in = part.split == 1;
        } else if mode == PadLightingMode::Wheel {
            self.cfg.pad_wheel_ccw = part.split == 1;
            self.cfg.pad_wheel_custom = part.color2.is_some();
            self.cfg.pad_wheel_a = part.color;
            if let Some(b) = part.color2 {
                self.cfg.pad_wheel_b = b;
            }
        } else {
            self.cfg.pad_color = part.color;
            self.cfg.pad_two = part.color2.is_some();
            if let Some(c2) = part.color2 {
                self.cfg.pad_color2 = c2;
            }
        }
        if mode == PadLightingMode::Breathing {
            self.cfg.pad_breath_split = part.split.min(2);
        } else if !matches!(
            mode,
            PadLightingMode::Tidal | PadLightingMode::Starlight | PadLightingMode::Dipp | PadLightingMode::Wheel
        ) {
            self.cfg.pad_split = part.split.min(1);
        }
        self.cfg.pad_brightness = part.brightness;
        self.pad_light_mode = mode.name().to_string();
        self.cfg.pad_last_mode = mode.name().to_string();
        if let Some(target) = self.pad_light_target() {
            self.pad.send(pad::PadCmd::Light(target));
        }
        true
    }

    /// Puts on a game preset. `quiet` (the automatic one) skips a device whose lighting is not
    /// switched on or that is not there, without an error; the Apply button reports instead.
    fn apply_game_preset(&mut self, preset: &game_presets::GamePreset, quiet: bool) {
        let built = game_presets::build(preset, self.status.keyboard_brightness, self.cfg.pad_brightness);
        let pad_status = self.pad.snapshot();
        let pad_ready = self.cfg.pad_lighting && pad_status.present && pad_status.lighting_available;
        let kbd_ready = self.cfg.kbd_lighting && self.device.is_some();
        let mut ok = true;
        let mut applied = false;
        if let Some(part) = &built.kbd {
            if !quiet || kbd_ready {
                ok &= self.apply_kbd_part(part);
                applied = true;
            }
        }
        if let Some(part) = &built.pad {
            if !quiet || pad_ready {
                ok &= self.apply_pad_part(part);
                applied = true;
            }
        }
        if applied {
            config::save(&self.cfg);
            if ok {
                self.set_status_message(format!("Applied \"{}\"", preset.name));
            }
        }
    }

    /// Once a frame: when a look of the program has just been unlocked or switched on, its game
    /// preset goes on (with the lighting toggles on and the automatic switch on).
    fn watch_look(&mut self) {
        let look = game_presets::current_look(&self.cfg);
        if look == self.last_look {
            return;
        }
        self.last_look = look;
        if let Some(look) = look {
            if self.cfg.game_preset_auto && self.fully_initialized {
                self.apply_game_preset(game_presets::for_look(look), true);
            }
        }
    }

    fn apply_preset(&mut self, bank: ui::presets::Bank, slot: usize) {
        let Some(preset) = self.preset_bank_mut(bank).get(slot).cloned().flatten() else {
            return;
        };
        let mut ok = true;
        if let Some(part) = &preset.kbd {
            ok &= self.apply_kbd_part(part);
        }
        if let Some(part) = &preset.pad {
            ok &= self.apply_pad_part(part);
        }
        config::save(&self.cfg);
        if ok {
            self.set_status_message(format!("Applied \"{}\"", preset.name));
        }
    }

    /// Lighting Presets card (Lighting tab).
    fn render_presets_section(&mut self, ui: &mut egui::Ui) {
        let pad_present = self.pad.snapshot().present;
        let mut resume = self.cfg.light_resume;
        let action = ui::presets::render_presets_card(
            ui,
            &mut self.preset_ui,
            &self.cfg,
            &mut resume,
            pad_present,
        );

        if let Some(on) = action.resume_toggled {
            self.cfg.light_resume = on;
            self.pad.send(pad::PadCmd::ResumeLighting(on));
            config::save(&self.cfg);
        }
        if let Some((bank, slot)) = action.clear {
            if let Some(entry) = self.preset_bank_mut(bank).get_mut(slot) {
                *entry = None;
            }
            config::save(&self.cfg);
        }
        if let Some((bank, slot)) = action.save {
            self.save_preset(bank, slot);
        }
        if let Some(bank) = action.save_new {
            match ui::presets::first_free_slot(&self.cfg, bank) {
                Some(slot) => self.save_preset(bank, slot),
                None => self.set_error_message(format!(
                    "All {} preset slots are used: update or delete one first",
                    config::PRESET_SLOTS
                )),
            }
        }
        if let Some((bank, slot)) = action.apply {
            self.apply_preset(bank, slot);
        }
        if let Some(on) = action.game_auto_toggled {
            self.cfg.game_preset_auto = on;
            config::save(&self.cfg);
        }
        if let Some(id) = action.apply_game {
            if let Some(preset) = game_presets::by_id(id) {
                self.apply_game_preset(preset, false);
            }
        }
    }

    /// Once per launch: with "Restore lighting at startup" on (and the keyboard's lighting toggle
    /// on), put the last keyboard effect back. The pad does the same on its own thread.
    fn resume_keyboard_lighting(&mut self) {
        if !(self.cfg.light_resume && self.cfg.kbd_lighting) {
            return;
        }
        if let Some(effect) = librazer::command::KeyboardEffect::from_key(&self.cfg.kbd_last_effect)
        {
            let _ = self.set_keyboard_effect(effect);
        }
    }

    /// The AC / battery profile (it used to be in Options): what to put on each power source.
    fn render_power_profile_section(&mut self, ui: &mut egui::Ui) {
        let res = ui::options::render_power_profile(ui, &mut self.cfg, &self.available_performance_modes);
        if res.modes_changed {
            self.ac_profile.perf_mode =
                self.cfg.ac_mode.as_deref().and_then(config::perf_mode_from_name).unwrap_or(PerfMode::Performance);
            self.battery_profile.perf_mode =
                self.cfg.battery_mode.as_deref().and_then(config::perf_mode_from_name).unwrap_or(PerfMode::Battery);
        }
        if res.changed {
            config::save(&self.cfg);
        }
        if res.apply_now && self.device.is_some() {
            self.auto_switch_profile();
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

impl RazerGuiApp {
    /// Starts the window changing to `size` (and `pos`, if it has a place) over a moment.
    fn stretch_window_to(&mut self, ctx: &egui::Context, size: egui::Vec2, pos: Option<egui::Pos2>) {
        let (now_size, now_pos) = ctx.input(|i| {
            let v = i.viewport();
            (v.inner_rect.map(|r| r.size()), v.outer_rect.map(|r| r.min))
        });
        self.window_stretch = Some(WindowStretch {
            from: (now_size.unwrap_or(size), now_pos),
            to: (size, pos),
            start: ctx.input(|i| i.time),
        });
        ctx.request_repaint();
    }

    /// Moves the window along its stretch, once a frame.
    fn step_window_stretch(&mut self, ctx: &egui::Context) {
        let Some(st) = &self.window_stretch else { return };
        let t = (ctx.input(|i| i.time) - st.start) / father::WINDOW_STRETCH_LEN;
        let k = father::ease(t);
        let size = st.from.0 + (st.to.0 - st.from.0) * k;
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        if let (Some(a), Some(b)) = (st.from.1, st.to.1) {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(a + (b - a) * k));
        }
        if t >= 1.0 {
            self.window_stretch = None;
        }
        ctx.request_repaint();
    }

    /// The hit of the moon shakes the window itself: `shake` is how far from where it stood (None
    /// when it is not to move; the window is put back where it was).
    fn shake_window(&mut self, ctx: &egui::Context, shake: Option<egui::Vec2>) {
        match shake {
            Some(offset) => {
                let (pos, still) = ctx.input(|i| {
                    let v = i.viewport();
                    (v.outer_rect.map(|r| r.min), !(v.maximized.unwrap_or(false) || v.fullscreen.unwrap_or(false)))
                });
                if !still {
                    return;
                }
                let Some(base) = self.moon_shake_base.or(pos) else { return };
                self.moon_shake_base = Some(base);
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(base + offset));
            }
            None => {
                if let Some(base) = self.moon_shake_base.take() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(base));
                }
            }
        }
    }

    /// The sound card, opened the first time something needs it.
    fn audio(&mut self) -> Option<audio::Audio> {
        if !self.audio_tried {
            self.audio_tried = true;
            self.audio = audio::Audio::new(self.cfg.games.volume());
        }
        self.audio.clone()
    }

    /// Counts a death in game `g` (0 to 2) and says what it unlocked, if the player is on that game's look.
    fn count_death(&mut self, g: usize) {
        let unlocked = self.cfg.games.record_death(g);
        config::save(&self.cfg);
        let current = self.cfg.edition().game() == g;
        if let (Some(c), true) = (self.cfg.games.new_cheat(g), current) {
            self.message_manager.add_message(status_message(format!("Cheat unlocked: {} (Options > Games).", c.label())));
        }
        if let (Some(d), true) = (unlocked, current) {
            let text = match d {
                games::Difficulty::Hard => "Options now has a Games section.".to_string(),
                other if g == 2 => format!("{} mode is unlocked.", other.label()),
                other => format!("{} mode is unlocked in Options > Games.", other.label()),
            };
            self.message_manager.add_message(status_message(text));
        }
    }

    /// Pays the CR-Tokens for a run that has just ended, says so in the status line, and keeps a
    /// line in the log (tokens and seconds played) so the prices can be tuned. A run with a cheat on
    /// pays nothing.
    fn pay_run(&mut self, run: shop::Run, d: games::Difficulty, cheated: bool, secs: f32) {
        let g = run.game();
        let first = !cheated && run.won() && self.cfg.shop.take_first_win(g);
        let n = shop::payout(run, d, first, cheated);
        dlog::log(&format!("tokens: game {} {:?} won={} cheated={} -> {} in {:.0}s", g + 1, d, run.won(), cheated, n, secs));
        if n == 0 {
            if first {
                config::save(&self.cfg);
            }
            return;
        }
        let deaths: u32 = (0..games::GAMES).map(|i| self.cfg.games.deaths(i)).sum();
        let start = self.cfg.shop.open(deaths);
        self.cfg.shop.earn(n);
        config::save(&self.cfg);
        // (said later, when nothing is going on: see `announce_tokens`)
        self.token_notice.0 = self.token_notice.0.saturating_add(n);
        self.token_notice.1 = self.token_notice.1.saturating_add(start);
    }

    /// Whether a game or a scene (a game, the endings, the king's laugh, the moon's strike, a popup)
    /// is on the screen, and so is not to be interrupted.
    fn scene_playing(&self) -> bool {
        self.game.is_some()
            || self.cat_scene.is_some()
            || self.ally_scene.is_some()
            || self.game2.is_some()
            || self.game3.is_some()
            || self.poker.is_some()
            || self.free_scene.is_some()
            || self.void_popup
            || self.underworld_popup
            || self.king_laugh.is_some()
            || self.dead_king_laugh.is_some()
            || self.moon.phase() != moon::Phase::Tucked
    }

    /// Says what the last runs paid (and brings up the shop's welcome the first time), once nothing
    /// is going on: never in the middle of a game or a scene.
    fn announce_tokens(&mut self) {
        if !release::story() {
            self.token_notice = (0, 0);
            return;
        }
        if self.token_notice == (0, 0) || self.scene_playing() {
            return;
        }
        let (n, start) = std::mem::take(&mut self.token_notice);
        let mut text = format!("+{n} CR-Tokens");
        if start > 0 {
            text.push_str(&format!(" (and {start} for the deaths so far)"));
        }
        self.message_manager.add_message(status_message(text));
        if !self.cfg.shop.revealed() {
            self.cfg.shop.mark_revealed();
            config::save(&self.cfg);
            self.shop_popup = true;
        }
    }

    /// Keeps the sampler in line with what uses it (see the note inside).
    fn keep_sensors_in_step(&mut self) {
        // With the Sensors tab switched off nothing about sensors runs: the sampler stops and closes its
        // sources, and logging is switched off with it (nothing is written, and no log waits for a program).
        // When the tab is back, Read sensors is still as it was, but logging stays off until it is switched on.
        let shown = self.cfg.tab_shown("sensors");
        if shown && self.cfg.sensor_read && !self.sensors.is_on() {
            self.sensors.start(self.cfg.sensor_source_config());
        } else if !shown && (self.sensors.is_on() || self.sensors.logging()) {
            self.sensors.set_logging(false);
            self.cfg.sensor_logging = false;
            self.sensors.stop();
        }
    }

    /// Earns the trophies the program now has the facts for (see `trophies.rs`), saves, and keeps
    /// their names for the announcement.
    fn check_trophies(&mut self) {
        if !release::story() {
            return;
        }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let new = self.cfg.sync_trophies(now);
        if !new.is_empty() {
            self.trophy_notice.extend(new);
            config::save(&self.cfg);
        }
    }

    /// Whether this is a good moment for a trophy announcement: no game or scene, no popup to dismiss
    /// first, the tokens already said, and the window actually on the screen.
    fn quiet_moment(&self, ctx: &egui::Context) -> bool {
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false)) || self.in_tray;
        !self.scene_playing() && !self.shop_popup && self.token_notice == (0, 0) && !minimized
    }

    /// Announces the trophies that were earned, one at a time, and only at a good moment (see
    /// `quiet_moment`): one that is up when a scene starts goes back in line and comes again later.
    fn run_trophy_toast(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let quiet = self.quiet_moment(ctx);
        if let Some((trophy, since)) = self.trophy_toast {
            if !quiet {
                self.trophy_notice.insert(0, trophy);
                self.trophy_toast = None;
                return;
            }
            let age = (now - since) as f32;
            if ui::trophies::render_toast(ctx, trophy, age) || age > ui::trophies::TOAST_SECONDS {
                self.trophy_toast = None;
                self.trophy_gap_until = now + 0.6;
            }
            ctx.request_repaint();
        } else if quiet && now >= self.trophy_gap_until && !self.trophy_notice.is_empty() {
            let next = self.trophy_notice.remove(0);
            self.trophy_toast = Some((next, now));
            ctx.request_repaint();
        }
    }

    /// A fresh go at the song the story is on (Game 3 begins once the moon has struck the staff).
    fn start_game3(&mut self) {
        let game = game3::Game::new(self.cfg.games.game3_stage(), self.cfg.games.difficulty(2), self.cfg.games.active(2));
        let audio = self.audio();
        if let Some(a) = &audio {
            a.set_volume(self.cfg.games.volume());
        }
        let mut run = ui::game3::Run::new(game, audio, self.cfg.games.latency_ms(), self.cfg.games.available(2), self.cfg.games.deaths(2));
        if self.cfg.games.underworld() {
            // the Underworld is open: songs of your own, and a playlist
            run = run.with_songs(songs::library(&songs::dir()), self.cfg.games.playlist().to_vec());
        }
        self.game3 = Some(run);
    }

    /// Starts listening to the mp3 at `path` for Game 3 (the chart is made on a thread of its own).
    fn start_song_import(&mut self, path: String) {
        if self.song_import.is_some() {
            return;
        }
        if let Some(run) = &mut self.game3 {
            run.set_busy(Some(format!("Listening to \"{}\"...", songs::title_of(&path))));
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(import::add(&songs::dir(), &path));
        });
        self.song_import = Some(rx);
    }

    /// Games 3's songs screen asked for things, and an added song may be done.
    fn serve_game3_songs(&mut self, ctx: &egui::Context) {
        self.poll_file_pick(ctx);
        let mut done: Option<Result<songs::Item, String>> = None;
        if let Some(rx) = &self.song_import {
            match rx.try_recv() {
                Ok(result) => done = Some(result),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => done = Some(Err("Something went wrong while listening to that song.".to_string())),
            }
        }
        if done.is_some() {
            self.song_import = None;
        }
        let mut requests = Vec::new();
        if let Some(run) = &mut self.game3 {
            requests = run.take_requests();
            if let Some(result) = done {
                run.set_busy(None);
                run.set_library(songs::library(&songs::dir()));
                run.set_note(match result {
                    Ok(item) => {
                        let n = |d| item.song.notes(d).len();
                        format!(
                            "Added \"{}\": {:.0} BPM, {} / {} / {} notes (easy / normal / hard).",
                            item.song.title,
                            item.song.bpm,
                            n(games::Difficulty::Easy),
                            n(games::Difficulty::Normal),
                            n(games::Difficulty::Hard)
                        )
                    }
                    Err(e) => e,
                });
            }
        }
        for request in requests {
            match request {
                ui::game3::Request::AddSong => {
                    if self.file_pick.is_none() && self.song_import.is_none() {
                        self.file_pick = Some((picker::Target::Song, picker::spawn(picker::Target::Song, "")));
                    }
                }
                ui::game3::Request::RemoveSong(id) => {
                    songs::remove(&songs::dir(), &id);
                    if let Some(run) = &mut self.game3 {
                        run.set_library(songs::library(&songs::dir()));
                    }
                }
                ui::game3::Request::Playlist(ids) => {
                    self.cfg.games.set_playlist(ids);
                    config::save(&self.cfg);
                }
            }
        }
    }

    /// Game 3 (the shadow monarch's rhythm game), in the same window. Clearing the first song saves
    /// the checkpoint; clearing the second is his kill, and opens the Underworld.
    /// Sits down at the Devil's Table: `free` is the test seat, otherwise the buy-in is taken from the
    /// wallet first. Nothing happens (and nothing is taken) while the Shadow Monarch holds your soul.
    fn start_poker(&mut self, free: bool) {
        let now = poker_save::unix_now();
        if self.cfg.poker.locked(now) {
            self.set_status_message("The table is closed while your soul is held.".to_string());
            return;
        }
        if !free && !self.cfg.shop.spend(poker_game::BUY_IN as u32) {
            self.set_status_message(format!("The Devil's Table costs {} CR-Tokens.", poker_game::BUY_IN));
            return;
        }
        let audio = self.audio();
        let mut run = ui::poker::Run::new(audio);
        run.opts = self.cfg.poker.options();
        if !free {
            run.s.notice = Some((poker_story::buy_in_notice(poker_game::BUY_IN), 6.0));
        }
        self.cfg.poker.mark_intro_seen();
        config::save(&self.cfg);
        self.poker = Some(run);
    }

    /// The Hold'em table, a frame of it: pays out and starts the soul lock the moment a game is decided.
    fn play_poker(&mut self, ctx: &egui::Context) {
        let mut event = ui::poker::Event::None;
        if let Some(run) = &mut self.poker {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(egui::Color32::from_rgb(8, 5, 6)).inner_margin(egui::Margin::ZERO))
                .show(ctx, |ui| {
                    event = run.show(ui);
                });
        }
        // the gear's settings are kept as soon as they are changed
        if let Some(run) = &mut self.poker {
            if std::mem::take(&mut run.opts_changed) {
                self.cfg.poker.set_options(run.opts);
                config::save(&self.cfg);
            }
            // the two lovers dealt to you together, in a real hand
            if std::mem::take(&mut run.together) && self.cfg.poker.mark_together() {
                config::save(&self.cfg);
            }
        }
        match event {
            ui::poker::Event::None => {}
            ui::poker::Event::Finished(f) => {
                if f.payout > 0 {
                    self.cfg.shop.earn(f.payout as u32);
                }
                let now = poker_save::unix_now();
                if self.cfg.poker.finish(f.ending, now) > 0 {
                    // the Underworld is where the Monarch keeps you: it stays on and is held as it is
                    self.cfg.underworld_on = true;
                    self.held_look = None;
                }
                config::save(&self.cfg);
            }
            ui::poker::Event::Quit => self.poker = None,
        }
    }

    /// Keeps the soul lock: while it is on, the Underworld's look is put back if anything changed it;
    /// when it runs out the break-free scene is owed (and shown at a quiet moment).
    fn keep_soul_lock(&mut self) {
        let now = poker_save::unix_now();
        if self.cfg.poker.tick(now) {
            self.held_look = None;
            config::save(&self.cfg);
        }
        if self.cfg.poker.locked(now) {
            match &self.held_look {
                None => {
                    if !self.cfg.games.underworld() {
                        self.cfg.games.open_underworld();
                    }
                    self.cfg.underworld_on = true;
                    self.cfg.shop_preview = None;
                    self.held_look = Some(HeldLook::of(&self.cfg));
                }
                Some(held) => {
                    if *held != HeldLook::of(&self.cfg) || self.cfg.shop_preview.is_some() {
                        let held = held.clone();
                        held.put_back(&mut self.cfg);
                        config::save(&self.cfg);
                    }
                }
            }
        } else {
            self.held_look = None;
        }
        if self.cfg.poker.free_notice() && !self.scene_playing() && self.free_scene.is_none() {
            self.cfg.poker.take_free_notice();
            config::save(&self.cfg);
            self.free_scene = Some(0);
        }
    }

    fn play_game3(&mut self, ctx: &egui::Context) {
        let mut event = ui::game3::Event::None;
        if let Some(run) = &mut self.game3 {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(egui::Color32::from_rgb(5, 2, 11)).inner_margin(egui::Margin::ZERO))
                .show(ctx, |ui| {
                    event = run.show(ui);
                });
        }
        let (mut ran_out, mut cleared, mut chosen) = (false, None, None);
        let mut stats = (0.0_f32, 0.0_f32, games::Difficulty::Hard, false, 0.0_f32);
        if let Some(run) = &mut self.game3 {
            stats = (
                run.game.progress(),
                run.game.accuracy(),
                run.game.difficulty,
                run.game.cheated(),
                run.game.song_len(),
            );
            ran_out = run.game.take_death() && run.game.story();
            cleared = run.take_cleared();
            chosen = run.take_chosen();
        }
        // the difficulty picked on the first screen is the one Options shows, and the next run starts on
        if let Some(d) = chosen {
            self.cfg.games.set_difficulty(2, d);
            config::save(&self.cfg);
        }
        // every death counts towards the Games section in Options and its difficulties (see games.rs)
        if ran_out {
            self.count_death(2);
            let run = shop::Run::Game3 { progress: stats.0, accuracy: stats.1, cleared: false };
            self.pay_run(run, stats.2, stats.3, stats.0 * stats.4);
        }
        match cleared {
            Some(game3::Outcome::Checkpoint) => {
                self.cfg.games.set_game3_stage(1);
                config::save(&self.cfg);
                let run = shop::Run::Game3 { progress: 1.0, accuracy: stats.1, cleared: true };
                self.pay_run(run, stats.2, stats.3, stats.4);
            }
            Some(game3::Outcome::Death) => {
                self.cfg.games.set_game3_stage(0);
                self.cfg.games.open_underworld();
                config::save(&self.cfg);
                self.count_death(2);
                let run = shop::Run::Game3 { progress: 1.0, accuracy: stats.1, cleared: true };
                self.pay_run(run, stats.2, stats.3, stats.4);
            }
            Some(game3::Outcome::Practice) | None => {}
        }
        self.serve_game3_songs(ctx);
        match event {
            ui::game3::Event::None => {}
            ui::game3::Event::Quit => self.game3 = None,
            ui::game3::Event::Retry | ui::game3::Event::Next => self.start_game3(),
            ui::game3::Event::Ended => {
                self.game3 = None;
                self.underworld_popup = true;
            }
        }
        ctx.request_repaint();
    }

    /// The Ally of Darkness, in the game window. Leaving (Esc) goes back to the market; the hatch is the
    /// way out, and it is kept: the clock tower's door is open for good once the key is used.
    fn play_ally(&mut self, ctx: &egui::Context) {
        let animated = self.cfg.backdrop_animated;
        let mut frame = ui::ally::Frame { exit: ui::ally::Exit::None, changed: false };
        if let Some(scene) = &mut self.ally_scene {
            let state = self.cfg.games.ally_mut();
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(egui::Color32::from_rgb(4, 6, 10)).inner_margin(egui::Margin::ZERO))
                .show(ctx, |ui| {
                    frame = ui::ally::show(ui, scene, state, animated);
                });
        }
        if frame.changed || frame.exit != ui::ally::Exit::None {
            // saved when anything changes, and always on the way out
            config::save(&self.cfg);
        }
        if frame.exit != ui::ally::Exit::None {
            self.ally_scene = None;
        }
        ctx.request_repaint();
    }

    /// The talk with the cat before the crown, in the game window. Yes starts Game 1 (and is kept: he
    /// is not asked again); no leaves it, and he greets you differently next time.
    fn play_cat_talk(&mut self, ctx: &egui::Context) {
        let animated = self.cfg.backdrop_animated;
        let mut exit = ui::cat_talk::Exit::None;
        if let Some(scene) = &mut self.cat_scene {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(egui::Color32::from_rgb(10, 6, 20)).inner_margin(egui::Margin::ZERO))
                .show(ctx, |ui| {
                    exit = ui::cat_talk::show(ui, scene, animated);
                });
        }
        match exit {
            ui::cat_talk::Exit::None => {}
            ui::cat_talk::Exit::Leave => self.cat_scene = None,
            ui::cat_talk::Exit::Done(cat_talk::Outcome::Decline) => {
                self.cat_scene = None;
                self.cfg.games.add_cat_decline();
                config::save(&self.cfg);
            }
            ui::cat_talk::Exit::Done(cat_talk::Outcome::Help) => {
                self.cat_scene = None;
                self.cfg.games.set_cat_helped(true);
                config::save(&self.cfg);
                self.game_runs += 1;
                let now = ctx.input(|i| i.time);
                self.game = Some(game1::Game::start(
                    self.game_runs.wrapping_mul(7919).wrapping_add((now * 1000.0) as u32),
                    self.cfg.games.difficulty(0),
                    self.cfg.games.active(0),
                    game1::Part::from_checkpoint(self.cfg.games.game1_checkpoint()),
                ));
            }
        }
        ctx.request_repaint();
    }

    /// Game 2 (the staff of time), in the same window. When it is over he gets his staff back.
    fn play_game2(&mut self, ctx: &egui::Context) {
        let mut outcome = ui::game::Outcome::Playing;
        let angels = self.gold_angels.as_ref();
        if let Some(g) = &mut self.game2 {
            // (the test switch in Options > Game testing)
            if self.cfg.dev_invincible {
                g.infinite = true;
            }
        }
        if let Some(game) = &mut self.game2 {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(egui::Color32::from_rgb(6, 5, 3)).inner_margin(egui::Margin::ZERO))
                .show(ctx, |ui| {
                    outcome = ui::game2::show(ui, angels, game);
                });
        }
        let now = ctx.input(|i| i.time);
        let stats2 = self.game2.as_ref().map(|g| (g.kills, g.difficulty, g.cheated(), g.t));
        if self.game2.as_mut().is_some_and(|g| g.take_win()) {
            if let Some((kills, d, cheated, secs)) = stats2 {
                self.pay_run(shop::Run::Game2 { kills, won: true }, d, cheated, secs);
            }
        }
        // every death counts towards the Games section in Options and its difficulties (see games.rs)
        if self.game2.as_mut().is_some_and(|g| g.take_death()) {
            let unlocked = self.cfg.games.record_death(1);
            config::save(&self.cfg);
            if let (Some(c), true) = (self.cfg.games.new_cheat(1), self.cfg.edition().game() == 1) {
                self.message_manager.add_message(status_message(format!("Cheat unlocked: {} (Options > Games).", c.label())));
            }
            if let (Some(d), true) = (unlocked, self.cfg.edition().game() == 1) {
                let text = match d {
                    games::Difficulty::Hard => "Options now has a Games section.".to_string(),
                    other => format!("{} mode is unlocked in Options > Games.", other.label()),
                };
                self.message_manager.add_message(status_message(text));
            }
            if let Some((kills, d, cheated, secs)) = stats2 {
                self.pay_run(shop::Run::Game2 { kills, won: false }, d, cheated, secs);
            }
        }
        match outcome {
            // (Part2 is Game 1's: Game 2 never sends it)
            ui::game::Outcome::Playing | ui::game::Outcome::Part2 => {}
            ui::game::Outcome::Quit => {
                self.game2 = None;
                self.smoker.give_back();
            }
            ui::game::Outcome::Retry => {
                self.game_runs += 1;
                self.game2 = Some(game2::Game::with_cheats(
                    self.game_runs.wrapping_mul(7919).wrapping_add((now * 1000.0) as u32),
                    self.cfg.games.difficulty(1),
                    self.cfg.games.active(1),
                ));
            }
            ui::game::Outcome::Finished => {
                self.game2 = None;
                self.smoker.give_back();
                if !self.cfg.void_unlocked {
                    // the prize: The Void, switched on, with its glitch stars, and a popup
                    self.cfg.unlock_void_by_winning();
                    config::save(&self.cfg);
                    self.void_popup = true;
                }
            }
        }
        ctx.request_repaint();
    }

    /// Game 1, in the program's own window: the window stretches wide when it starts (about its
    /// middle, so it stays on the screen) and the game fills it.
    fn run_game(&mut self, ctx: &egui::Context) {
        if self.game_restore.is_none() {
            let (size, pos, monitor) = ctx.input(|i| {
                let v = i.viewport();
                (v.inner_rect.map(|r| r.size()), v.outer_rect.map(|r| r.min), v.monitor_size)
            });
            let size = size.unwrap_or(egui::vec2(WINDOW_WIDTH, self.last_fit_height.max(420.0)));
            self.game_restore = Some((size, pos));
            let m = monitor.map_or((1600.0, 900.0), |m| (m.x, m.y));
            let (w, h) = father::game_window_size(m);
            let to = egui::vec2(w.max(size.x), h.max(size.y));
            let to_pos = pos.map(|p| {
                let x = p.x - (to.x - size.x) / 2.0;
                egui::pos2(if p.x >= 0.0 { x.max(0.0) } else { x }, p.y.max(0.0))
            });
            self.stretch_window_to(ctx, to, to_pos);
        }
        if self.cat_scene.is_some() {
            self.play_cat_talk(ctx);
            return;
        }
        if self.ally_scene.is_some() {
            self.play_ally(ctx);
            return;
        }
        if self.game2.is_some() {
            self.play_game2(ctx);
            return;
        }
        if self.game3.is_some() {
            self.play_game3(ctx);
            return;
        }
        if self.poker.is_some() {
            self.play_poker(ctx);
            return;
        }
        let unlocking = !self.cfg.gold_unlocked;
        let strength = self.cfg.shown_backdrop_style().info_strength();
        let mut outcome = ui::game::Outcome::Playing;
        if let Some(g) = &mut self.game {
            // (the test switch in Options > Game testing)
            g.invincible = self.cfg.dev_invincible;
        }
        if self.game.is_some() && self.game_clock.len() != 3 {
            self.game_clock = info::BackdropStyle::FatherTime
                .pngs()
                .iter()
                .enumerate()
                .filter_map(|(i, png)| {
                    let img = image::load_from_memory(png).ok()?.to_rgba8();
                    let size = [img.width() as usize, img.height() as usize];
                    Some(ctx.load_texture(format!("cpunity_game_clock_{}", i), egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()), egui::TextureOptions::LINEAR))
                })
                .collect();
        }
        if let Some(game) = &mut self.game {
            let textures = &self.game_clock;
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(theme::bg()).inner_margin(egui::Margin::ZERO))
                .show(ctx, |ui| {
                    outcome = ui::game::show(ui, textures, strength, game, unlocking);
                });
        }
        let now = ctx.input(|i| i.time);
        let stats1 = self.game.as_ref().map(|g| (g.paid_secs(), g.difficulty, g.cheated()));
        // part one survived: the checkpoint is kept, so a retry (even after leaving) starts at part two
        if self.game.as_mut().is_some_and(|g| g.take_checkpoint()) {
            self.cfg.games.set_game1_checkpoint(true);
            config::save(&self.cfg);
        }
        if self.game.as_mut().is_some_and(|g| g.take_win()) {
            // the whole game is won: the next go starts from the beginning again
            self.cfg.games.set_game1_checkpoint(false);
            config::save(&self.cfg);
            if let Some((secs, d, cheated)) = stats1 {
                self.pay_run(shop::Run::Game1 { secs, won: true }, d, cheated, secs);
            }
        }
        // every death counts towards the Games section in Options and its difficulties (see games.rs)
        if self.game.as_mut().is_some_and(|g| g.take_death()) {
            let unlocked = self.cfg.games.record_death(0);
            config::save(&self.cfg);
            if let (Some(c), true) = (self.cfg.games.new_cheat(0), self.cfg.edition().game() == 0) {
                self.message_manager.add_message(status_message(format!("Cheat unlocked: {} (Options > Games).", c.label())));
            }
            if let (Some(d), true) = (unlocked, self.cfg.edition().game() == 0) {
                let text = match d {
                    games::Difficulty::Hard => "Options now has a Games section.".to_string(),
                    other => format!("{} mode is unlocked in Options > Games.", other.label()),
                };
                self.message_manager.add_message(status_message(text));
            }
            if let Some((secs, d, cheated)) = stats1 {
                self.pay_run(shop::Run::Game1 { secs, won: false }, d, cheated, secs);
            }
        }
        match outcome {
            ui::game::Outcome::Playing => {}
            ui::game::Outcome::Quit => self.game = None,
            ui::game::Outcome::Retry => {
                self.game_runs += 1;
                self.game = Some(game1::Game::start(
                    self.game_runs.wrapping_mul(7919).wrapping_add((now * 1000.0) as u32),
                    self.cfg.games.difficulty(0),
                    self.cfg.games.active(0),
                    game1::Part::from_checkpoint(self.cfg.games.game1_checkpoint()),
                ));
            }
            ui::game::Outcome::Part2 => {
                if let Some(g) = &mut self.game {
                    g.begin_part_two();
                }
            }
            ui::game::Outcome::Finished => {
                self.game = None;
                if !self.cfg.gold_unlocked {
                    // the prize: the Gold Edition, and the king laughs
                    self.cfg.unlock_gold_by_winning();
                    self.cfg.hidden_tabs.retain(|k| k != "info");
                    config::save(&self.cfg);
                    self.tab = Tab::Info;
                    self.king_laugh = Some(now);
                }
            }
        }
        ctx.request_repaint();
    }
}

impl eframe::App for RazerGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Minimized or in the tray nothing is drawn or animated: the app only wakes about once a
        // second to watch the laptop (mode, charger) and the tray icon.
        let idle = ctx.input(|i| i.viewport().minimized.unwrap_or(false)) || self.in_tray;
        ctx.request_repaint_after(std::time::Duration::from_millis(if idle { 1000 } else { 500 }));
        self.keep_soul_lock();

        // Remember which drop-downs are open.
        if theme::take_open_dirty() {
            self.cfg.open_tabs = theme::open_states();
            config::save(&self.cfg);
        }

        self.render_log_window(ctx);

        // Cooling pad worker messages (connected / disconnected / errors).
        for (is_error, message) in self.pad.drain_events() {
            if is_error {
                self.set_error_message(message);
            } else {
                self.set_optional_status_message(message);
            }
        }

        // The keyboard animation gave up (the keyboard stopped accepting frames).
        if let Some(message) = self.kbd_anim.take_error() {
            self.set_error_message(message);
        }

        // Apply the accent color to egui's selection/highlight color when it changes.
        let gold = self.cfg.gold_active();
        let void = self.cfg.void_active();
        let underworld = self.cfg.underworld_active();
        theme::set_gold(gold);
        theme::set_void(void);
        theme::set_underworld(underworld);
        theme::set_platinum_effects(self.cfg.platinum_effects);
        // egui puts text on whole pixels, so text on a floating slab would step a pixel at a time while
        // the rock glides under it; in the Underworld it is let sit between pixels and move smoothly
        ctx.tessellation_options_mut(|o| o.round_text_to_pixels = !underworld);
        if self.coin_tex.is_none() && release::story() && self.cfg.shop.visible() {
            self.coin_tex = ui::shop::load_coin(ctx);
        }
        let skin = theme::Skin::from_id(self.cfg.skin_active());
        theme::set_skin(skin);
        if skin == theme::Skin::Prism {
            theme::set_prism_time(ctx.input(|i| i.time) as f32);
        }
        if skin.is_animated() && !idle {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
        let accent_rgb = [theme::accent().r(), theme::accent().g(), theme::accent().b()];
        if self.applied_accent != Some(accent_rgb)
            || self.applied_gold != gold
            || self.applied_void != void
            || self.applied_underworld != underworld
            || self.applied_skin != skin
            || (skin == theme::Skin::Prism && self.applied_accent != Some(accent_rgb))
        {
            theme::apply(ctx);
            self.applied_accent = Some(accent_rgb);
            self.applied_gold = gold;
            self.applied_void = void;
            self.applied_underworld = underworld;
            self.applied_skin = skin;
        }
        if underworld && self.underworld_tex.is_none() {
            let tex = ui::underworld::Textures::load(ctx);
            ui::underworld::set_rock(tex.rock.id());
            self.underworld_tex = Some(tex);
        }
        if void && self.void_logo.is_none() {
            if let Ok(img) = image::load_from_memory(crate::void::LOGO_PNG) {
                let img = img.to_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                self.void_logo = Some(ctx.load_texture(
                    "cpunity_void_logo",
                    egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
        // The Void's Father Time has no staff, and falls apart into blocks: needs the layers without it
        if self.cfg.void_active() && self.cfg.backdrop && self.cfg.shown_backdrop_style().has_clock() {
            if self.gold_staff.is_empty() {
                self.gold_staff = gold::STAFF_PNGS
                    .iter()
                    .enumerate()
                    .filter_map(|(i, png)| {
                        let img = image::load_from_memory(png).ok()?;
                        let img = if img.width() > 2048 { img.resize(2048, 2048, image::imageops::FilterType::Lanczos3) } else { img };
                        let img = img.to_rgba8();
                        let size = [img.width() as usize, img.height() as usize];
                        Some(ctx.load_texture(
                            format!("cpunity_gold_staff_{i}"),
                            egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
                            egui::TextureOptions::LINEAR,
                        ))
                    })
                    .collect();
            }
            if self.void_father.is_none() {
                let pngs = self.cfg.shown_backdrop_style().pngs();
                if pngs.len() == 3 {
                    // (the back layer, then the mask and front without the staff)
                    self.void_father = Some(ui::backdrop::load_void_father(ctx, [pngs[0], gold::STAFF_PNGS[1], gold::STAFF_PNGS[0]]));
                }
            }
        }
        if skin == theme::Skin::Platinum && self.platinum_banner.is_none() {
            if let Ok(img) = image::load_from_memory(platinum::BANNER_PNG) {
                let img = img.to_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                self.platinum_banner = Some(ctx.load_texture(
                    "cpunity_platinum_banner",
                    egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
        if gold && self.gold_banner.is_none() {
            let load = |name: &str, bytes: &[u8], width: u32| {
                let img = image::load_from_memory(bytes).ok()?;
                let img = if img.width() > width { img.resize(width, width, image::imageops::FilterType::Lanczos3) } else { img };
                let img = img.to_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                Some(ctx.load_texture(
                    name,
                    egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
                    egui::TextureOptions::LINEAR,
                ))
            };
            self.gold_logo = load("cpunity_gold_logo", gold::LOGO_PNG, 160);
            self.gold_banner = load("cpunity_gold_banner", gold::BANNER_PNG, 2048);
            self.gold_arena = load("cpunity_gold_arena", gold::ARENA_PNG, 2048);
            self.gold_angels = load("cpunity_gold_angels", gold::ANGELS_PNG, 2048);
            self.gold_staff = gold::STAFF_PNGS
                .iter()
                .enumerate()
                .filter_map(|(i, png)| load(&format!("cpunity_gold_staff_{i}"), png, 2048))
                .collect();
        }
        if self.applied_font.as_deref() != Some(self.cfg.font.as_str()) {
            theme::install_fonts(ctx, &self.cfg.font);
            self.applied_font = Some(self.cfg.font.clone());
        }

        // Load the optional logo + banner once. A file next to the exe / in the config folder wins
        // (easy to tweak); otherwise the copy baked in from assets/ at build time is used.
        if !self.logo_loaded {
            self.logo_loaded = true;
            let custom_logo = config::find_logo().and_then(|p| image::open(p).ok());
            // only the built-in logo (drawn in red) is recoloured; a logo.png you supply is
            // shown exactly as it is
            self.logo_recolor = custom_logo.is_none();
            let logo_img = custom_logo
                .or_else(|| embedded_logo().and_then(|b| image::load_from_memory(b).ok()));
            if let Some(img) = logo_img {
                // full detail for the Info tab (only a huge file is shrunk)
                let full = if img.width() > 1024 || img.height() > 1024 {
                    img.resize(1024, 1024, image::imageops::FilterType::Lanczos3)
                } else {
                    img.clone()
                }
                .to_rgba8();
                // the Info tab colours it with the accent colour (see render_info_tab)
                self.info_logo_src =
                    Some((full.width() as usize, full.height() as usize, full.into_raw()));
                // the small header logo is also coloured with the accent colour (below)
                let img = img.resize(128, 128, image::imageops::FilterType::Lanczos3).to_rgba8();
                self.logo_src = Some((img.width() as usize, img.height() as usize, img.into_raw()));
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

        // The header logo takes the accent colour (again whenever the accent changes).
        if let Some((width, height, source)) = &self.logo_src {
            let accent = theme::accent();
            // a logo that is not recoloured never needs redoing
            let accent =
                if self.logo_recolor { [accent.r(), accent.g(), accent.b()] } else { [0, 0, 0] };
            if self.logo_accent != Some(accent) {
                let pixels = if self.logo_recolor {
                    info::recolor_logo(source, accent)
                } else {
                    source.clone()
                };
                self.logo = Some(ctx.load_texture(
                    "cpunity_logo",
                    egui::ColorImage::from_rgba_unmultiplied([*width, *height], &pixels),
                    egui::TextureOptions::LINEAR,
                ));
                self.logo_accent = Some(accent);
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

        self.watch_look();

        // Once per launch, after the device and power state are known.
        if self.fully_initialized && self.init_power_read && !self.launch_profile_done {
            if self.device.is_some() {
                self.launch_profile_done = true;
                self.apply_launch_profile();
                self.resume_keyboard_lighting();
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

        self.update_tray(ctx);

        // When minimized (or hidden in the tray), poll infrequently to catch external performance
        // mode changes
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false)) || self.in_tray;
        if minimized && self.fully_initialized {
            const PERF_POLL_INTERVAL: f32 = 2.5; // seconds
            if self.last_perf_poll_time.elapsed().as_secs_f32() >= PERF_POLL_INTERVAL {
                self.watch_power();
                self.remember_last();
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

        self.poll_file_pick(ctx);

        // "Start at login without the permission prompt": background job result and dialog
        if let Some(job) = self.login_task.poll() {
            match job {
                login_task::Job::Create => {
                    // the task replaces the Run-key entry (otherwise two copies would start)
                    config::set_startup(false);
                    self.startup_enabled = false;
                    self.set_status_message(
                        "R-Helper will start at login without a permission prompt".to_string(),
                    );
                }
                login_task::Job::Remove => {
                    self.set_status_message("Login task removed".to_string());
                }
            }
        }
        if !self.login_task.dialog_open {
            if let Some(error) = self.login_task.error.take() {
                self.set_status_message(format!("Login task: {}", error));
            }
        }
        if self.login_task.busy.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        ui::login_task::render_dialog(ctx, &mut self.login_task, admin::is_elevated());
        self.keep_sensors_in_step();
        self.check_trophies();
        self.announce_tokens();
        self.run_trophy_toast(ctx);
        // once, after the first trophy is announced: where the Trophies tab is switched on
        if self.cfg.trophies_notice_due()
            && self.trophy_toast.is_none()
            && self.trophy_notice.is_empty()
            && self.quiet_moment(ctx)
            && ui::trophies::render_tab_notice(ctx)
        {
            self.cfg.trophies_tab_told = true;
            config::save(&self.cfg);
        }
        let mut open_shop = false;
        ui::shop::render_welcome(ctx, &mut self.shop_popup, self.coin_tex.as_ref(), &mut open_shop);
        if open_shop {
            self.show_options = true;
        }
        ui::void_popup::render(ctx, &mut self.void_popup);
        ui::void_popup::render_underworld(ctx, &mut self.underworld_popup, self.cfg.underworld_out());
        ui::poker::free_scene(ctx, &mut self.free_scene);

        // Handle close request from X button
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.cfg.close_to_tray && self.tray.is_some() && !self.should_quit {
                // hide instead of closing; the tray icon brings the window back
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                self.in_tray = true;
            } else {
                self.should_quit = true;
            }
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
                        // AC power detection: a confirmed change triggers the profile switch.
                        self.watch_power();
                        self.remember_last();

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
        // Minimized or in the tray: draw nothing, so no animation or game can keep the window
        // repainting (they ask for frames on their own). The polling above has already run.
        if minimized {
            return;
        }
        // The game takes the whole window, which stretches wide to fit it and shrinks back after
        self.step_window_stretch(ctx);
        if self.game.is_some() || self.cat_scene.is_some() || self.ally_scene.is_some() || self.game2.is_some() || self.game3.is_some() || self.poker.is_some() {
            self.run_game(ctx);
            return;
        }
        if let Some((size, pos)) = self.game_restore.take() {
            self.last_fit_height = size.y;
            self.stretch_window_to(ctx, size, pos);
        }
        let footer_height = egui::TopBottomPanel::bottom("footer")
            .frame(
                egui::Frame::new()
                    .fill(theme::surface())
                    .stroke(egui::Stroke::new(1.0, theme::border()))
                    .inner_margin(egui::Margin::symmetric(12, 2)),
            )
            .show(ctx, |ui| {
                let tokens = self.cfg.shop.visible().then(|| (self.cfg.shop.balance(), self.coin_tex.as_ref()));
                ui::footer::render_footer(ui, &mut self.show_options, &device_label, tokens);
            })
            .response
            .rect
            .height();

        let mut content_height = 0.0_f32;
        let central_response = egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::bg()).inner_margin(egui::Margin::same(12)))
            .show(ctx, |ui| {
                // the backdrop picture goes down first, so everything else sits on top of it
                let style = self.cfg.shown_backdrop_style();
                let strength =
                    if self.tab == Tab::Info { style.info_strength() } else { style.card_strength() };
                // Father Time stands at the top of the page instead (below), and takes the logo's place
                let underworld = self.cfg.underworld_active();
                let father = !underworld && self.cfg.backdrop && style.has_clock();
                let gold = self.cfg.gold_active();
                let void = self.cfg.void_active();
                let gold_time = (ui.input(|i| i.time) % 3600.0) as f32;
                let gold_motion = self.cfg.backdrop_animated && !minimized;
                // the Platinum look has its own banner (the one the user may have added gives way to it)
                let platinum_banner = theme::skin() == theme::Skin::Platinum && !gold && !void && !underworld && self.cfg.platinum_banner && self.platinum_banner.is_some();
                // the Underworld: lava over the whole panel, with the rock slabs lying on it
                let uw_time = if gold_motion { gold_time } else { 0.0 };
                ui::underworld::set_motion(gold_motion);
                if underworld {
                    if let Some(tex) = &self.underworld_tex {
                        ui::underworld::paint_lava(ui, tex, uw_time);
                    }
                    if gold_motion {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
                    }
                }
                // the art that belongs to a skin: the blossom branches, the winter forest, the red thread
                if self.cfg.backdrop && matches!(theme::skin(), theme::Skin::Sakura | theme::Skin::Frost | theme::Skin::Descent | theme::Skin::Platinum) {
                    let t = if self.cfg.backdrop_animated && (self.cfg.platinum_effects || theme::skin() != theme::Skin::Platinum) { gold_time } else { 0.0 };
                    match theme::skin() {
                        theme::Skin::Sakura => ui::backdrop::paint_blossom_branches(ui, t, strength),
                        theme::Skin::Frost => ui::backdrop::paint_pine_forest(ui, t, strength),
                        theme::Skin::Platinum => ui::backdrop::paint_platinum(ui, t, strength),
                        _ => ui::backdrop::paint_descent(ui, t, strength),
                    }
                    if gold_motion {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
                    }
                }
                if gold && self.cfg.backdrop && self.cfg.gold_sparkles {
                    ui::backdrop::paint_gold_rays(ui, if self.cfg.backdrop_animated { gold_time } else { 0.0 });
                }
                if self.cfg.backdrop && !underworld {
                    // the angel cats become the Soul Catsters while Platinum is the look
                    let soul = theme::skin() == theme::Skin::Platinum && !void && !gold;
                    if self.backdrop_loaded != Some(style) || self.backdrop_void != void || self.backdrop_soul != soul {
                        self.backdrop = style
                            .pngs_with(void, soul)
                            .iter()
                            .enumerate()
                            .filter_map(|(i, png)| {
                                let img = image::load_from_memory(png).ok()?.to_rgba8();
                                let size = [img.width() as usize, img.height() as usize];
                                Some(ctx.load_texture(
                                    format!("cpunity_backdrop_{}", i),
                                    egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
                                    egui::TextureOptions::LINEAR,
                                ))
                            })
                            .collect();
                        self.backdrop_loaded = Some(style);
                        self.backdrop_void = void;
                        self.backdrop_soul = soul;
                    }
                    if style == info::BackdropStyle::VoidStars || style == info::BackdropStyle::DeadKing {
                        ui::backdrop::paint_void_stars(ui, if self.cfg.backdrop_animated { gold_time } else { 0.0 });
                        if gold_motion {
                            ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
                        }
                    } else if style == info::BackdropStyle::Cat {
                        ui::cat_talk::paint_market(ui, if self.cfg.backdrop_animated { gold_time } else { 0.0 }, strength);
                        if gold_motion {
                            ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
                        }
                    } else if style.is_ambient() {
                        let t = if self.cfg.backdrop_animated { gold_time } else { 0.0 };
                        match style {
                            info::BackdropStyle::Embers => ui::backdrop::paint_embers(ui, t, strength),
                            info::BackdropStyle::DataRain => ui::backdrop::paint_data_rain(ui, t, strength),
                            info::BackdropStyle::Petals => ui::backdrop::paint_petals(ui, t, strength),
                            info::BackdropStyle::SynthGrid => ui::backdrop::paint_synth_grid(ui, t, strength),
                            info::BackdropStyle::Bubbles => ui::backdrop::paint_bubbles(ui, t, strength),
                            info::BackdropStyle::Scanlines => ui::backdrop::paint_scanlines(ui, t, strength),
                            info::BackdropStyle::Fireflies => ui::backdrop::paint_fireflies(ui, t, strength),
                            info::BackdropStyle::Snowfall => ui::backdrop::paint_snowfall(ui, t, strength),
                            info::BackdropStyle::Storm => ui::backdrop::paint_storm(ui, t, strength),
                            info::BackdropStyle::Constellations => ui::backdrop::paint_constellations(ui, t, strength),
                            info::BackdropStyle::Sunbeams => ui::backdrop::paint_sunbeams(ui, t, strength),
                            _ => ui::backdrop::paint_northern(ui, t, strength),
                        }
                        if gold_motion {
                            ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
                        }
                    } else if style == info::BackdropStyle::Dragons {
                        ui::backdrop::paint_dragons(
                            ui,
                            &self.backdrop,
                            strength,
                            !minimized,
                            self.cfg.backdrop_animated,
                        );
                    } else if style == info::BackdropStyle::AngelCats {
                        ui::backdrop::paint_angels(
                            ui,
                            &self.backdrop,
                            strength,
                            !minimized,
                            self.cfg.backdrop_animated,
                            void,
                            soul,
                        );
                    } else if !style.has_clock() {
                        ui::backdrop::paint(ui, &self.backdrop, strength);
                    }
                }
                if gold && self.cfg.backdrop && self.cfg.gold_sparkles {
                    ui::backdrop::paint_gold_sparkles(ui, if self.cfg.backdrop_animated { gold_time } else { 0.0 });
                }
                if self.cfg.backdrop && theme::skin() == theme::Skin::Platinum {
                    ui::backdrop::paint_platinum_sparkles(ui, if self.cfg.backdrop_animated { gold_time } else { 0.0 });
                    if gold_motion {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
                    }
                }
                if gold && gold_motion {
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
                }
                let mut start_game = false;
                let mut staff_taken = false;
                // the moon waits in the corner of the glitching stars from 7pm to 5am
                let moon_area = ui.clip_rect();
                let show_moon = self.cfg.backdrop
                    && void
                    && style == info::BackdropStyle::VoidStars
                    && !minimized
                    && moon::appears(
                        sensor_log::now().hour,
                        self.cfg.void_pretend_night,
                        self.cfg.void_staff_broken,
                        (moon_area.width(), moon_area.height()),
                    );
                let mut moon_events = None;
                let dead_king = !underworld && self.cfg.backdrop && style == info::BackdropStyle::DeadKing;
                let mut dead_level = 0.0_f32;
                // the street with the cat is only at the top of the Info and Changelog tabs; the city lies behind all of them
                let cat_back = !underworld && self.cfg.backdrop && style == info::BackdropStyle::Cat && matches!(self.tab, Tab::Info | Tab::Changelog);
                let mut cat_clicked = false;
                let mut alley_clicked = false;
                let scroll_out =
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        // (inside the scroll area, so that it is above what scrolls and gets the drag)
                        if show_moon {
                            moon_events = Some(ui::moon::show(ui, &mut self.moon, moon_area, self.cfg.backdrop_animated));
                        } else if self.moon.phase() != moon::Phase::Tucked {
                            self.moon = moon::Moon::new();
                        }
                        if dead_king {
                            dead_level = ui::dead_king::paint_dead_king(ui, &mut self.dead_king_laugh, !minimized, self.cfg.backdrop_animated);
                        }
                        if cat_back {
                            // the cat before the crown: a click on him opens the talk
                            (cat_clicked, alley_clicked) = ui::cat_talk::paint_cat_backdrop(ui, &mut self.cat_backdrop, !minimized, self.cfg.backdrop_animated, release::ally_shown() && self.cfg.games.ally_open() && (!self.cfg.games.ally().door_open() || self.cfg.games.ally().left_friend()));
                        }
                        if father {
                            // in the Gold Edition he sits smoking instead of waiting for his beard
                            let smoker = if gold { Some(&mut self.smoker) } else { None };
                            let events = ui::backdrop::paint_father(
                                ui,
                                &self.backdrop,
                                strength,
                                &mut self.father_click,
                                smoker,
                                !minimized,
                                self.cfg.backdrop_animated,
                                &self.gold_staff,
                                if void { self.void_father.as_ref() } else { None },
                            );
                            start_game = events.game1;
                            staff_taken = events.staff_taken;
                        }
                        // the title and the tabs: on the Underworld's rock, with the causeway of skeletons below
                        let mut head = |ui: &mut egui::Ui| {
                        // Header with device name and status messages
                        ui::header::render_header(
                            ui,
                            ctx,
                            self.loading,
                            &self.system_specs,
                            &self.device,
                            &self.message_manager,
                            self.detecting_device,
                            if void && self.void_logo.is_some() {
                                self.void_logo.as_ref()
                            } else if gold && self.logo_recolor {
                                self.gold_logo.as_ref()
                            } else {
                                self.logo.as_ref()
                            },
                            if (gold && self.cfg.gold_banner) || (void && self.cfg.void_banner) || platinum_banner { None } else { self.banner.as_ref() },
                            if gold || void || platinum_banner { 44.0 } else { 32.0 },
                        );
                        if void && self.cfg.void_banner {
                            ui.add_space(6.0);
                            ui::header::render_void_banner(ui, if self.cfg.backdrop_animated { gold_time } else { 0.0 });
                            if gold_motion {
                                ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
                            }
                        }
                        if platinum_banner {
                            if let Some(tex) = &self.platinum_banner {
                                ui.add_space(6.0);
                                let glint = if self.cfg.backdrop_animated && self.cfg.platinum_effects {
                                    gold::glint((ui.input(|i| i.time) % 3600.0) as f32)
                                } else {
                                    None
                                };
                                ui::header::render_platinum_banner(ui, tex, glint);
                                if gold_motion {
                                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
                                }
                            }
                        }
                        if gold && self.cfg.gold_banner {
                            if let Some(tex) = &self.gold_banner {
                                ui.add_space(6.0);
                                let glint = if self.cfg.backdrop_animated {
                                    gold::glint((ui.input(|i| i.time) % 3600.0) as f32)
                                } else {
                                    None
                                };
                                ui::header::render_gold_banner(ui, tex, glint);
                            }
                        }
                        ui.add_space(8.0);

                        // a tab switched off in Options is left out; with all of them off
                        // only Main is left and the row is not drawn at all
                        let tabs = [
                            (Tab::Main, "\u{1f3e0} Main"),
                            (Tab::Lighting, "\u{1f4a1} Lighting"),
                            (Tab::Sensors, "Sensors"),
                            (Tab::Files, "\u{1f4c1} Files"),
                            (Tab::Trophies, "\u{1f3c6} Trophies"),
                            (Tab::Help, "\u{2753} Help"),
                            (Tab::Changelog, "\u{1f4dc} Changelog"),
                            (Tab::Info, "\u{1f4ac} Info"),
                        ];
                        let shown: Vec<(Tab, &str)> = tabs
                            .into_iter()
                            .filter(|(tab, _)| match tab {
                                Tab::Trophies => self.cfg.trophies_tab_shown(),
                                _ => tab.config_key().map(|k| self.cfg.tab_shown(k)).unwrap_or(true),
                            })
                            .collect();
                        if !shown.iter().any(|(tab, _)| *tab == self.tab) {
                            self.tab = Tab::Main;
                        }
                        if shown.len() > 1 {
                            // wraps onto a second line when the tabs do not fit the window: a row that is
                            // wider than the window would make everything under it wider as well
                            ui.horizontal_wrapped(|ui| {
                                for (tab, label) in shown {
                                    let text = egui::RichText::new(label).size(15.0);
                                    if ui.selectable_label(self.tab == tab, text).clicked() {
                                        self.tab = tab;
                                    }
                                }
                            });
                        }
                        if !underworld {
                            ui.add_space(6.0);
                        }
                        };
                        if underworld {
                            ui::underworld::slab_frame(ui, head);
                            ui.add_space(4.0);
                            ui::underworld::causeway(ui, uw_time);
                            ui.add_space(theme::card_gap());
                        } else {
                            head(ui);
                        }

                        // a card's "What does this do?" link opens the Help tab at its function
                        if let Some(key) = ui::help::take_request(ui.ctx()) {
                            theme::set_open(&ui::help::open_key(&key), true);
                            self.cfg.hidden_tabs.retain(|k| k != "help");
                            self.tab = Tab::Help;
                            ui.ctx().request_repaint();
                        }
                        match self.tab {
                            Tab::Main => {
                                self.render_performance_section(ui);
                                ui.add_space(theme::card_gap());

                                self.render_fan_section(ui);
                                ui.add_space(theme::card_gap());

                                if self.render_cooling_pad_section(ui) {
                                    ui.add_space(theme::card_gap());
                                }

                                self.render_battery_section(ui);
                                ui.add_space(theme::card_gap());

                                self.render_synapse_section(ui);
                                ui.add_space(theme::card_gap());

                                self.render_power_profile_section(ui);
                            }
                            Tab::Lighting => {
                                self.render_lighting_section(ui);
                                ui.add_space(theme::card_gap());

                                if self.render_sync_section(ui) {
                                    ui.add_space(theme::card_gap());
                                }

                                if self.render_pad_lighting_section(ui) {
                                    ui.add_space(theme::card_gap());
                                }

                                self.render_presets_section(ui);
                            }
                            Tab::Sensors => {
                                self.render_sensors_section(ui);
                            }
                            // (these are not made of cards, so on the Underworld each sits on one slab)
                            Tab::Help => {
                                theme::slab_on_underworld(ui, |ui| {
                                    ui::help::render_help(ui);
                                });
                            }
                            Tab::Changelog => {
                                theme::slab_on_underworld(ui, |ui| {
                                    ui::changelog::render_changelog(ui);
                                });
                            }
                            Tab::Info => {
                                theme::slab_on_underworld(ui, |ui| {
                                    self.render_info_tab(ui);
                                });
                            }
                            Tab::Trophies => {
                                theme::slab_on_underworld(ui, |ui| {
                                    ui::trophies::render_page(ui, &self.cfg);
                                });
                            }
                            Tab::Files => {
                                theme::slab_on_underworld(ui, |ui| {
                                    let picking = self.file_pick.is_some();
                                    let action = ui::files::render_files(ui, &mut self.cfg, picking);
                                    self.apply_files_action(action);
                                });
                            }
                        }
                    });
                content_height = scroll_out.content_size.y;
                if underworld {
                    // the souls rise last, over the cards
                    if self.cfg.backdrop && style == info::BackdropStyle::SoulWhispers {
                        // Soul Whispers: they come to the cursor, and whisper (see `whispers.rs`)
                        let area = ui.clip_rect();
                        if gold_motion {
                            let spots = ui::underworld::gap_spots(area);
                            let dt = ui.input(|i| i.stable_dt);
                            let pointer = ui
                                .input(|i| i.pointer.hover_pos())
                                .filter(|p| area.contains(*p))
                                .map(|p| (p.x - area.left(), p.y - area.top()));
                            // (what was heard before, and how many whispers there have been, come from the
                            // save, and what is heard now goes back)
                            self.whispers.restore(self.cfg.whispers_heard, self.cfg.whispers_spoken);
                            self.whispers.step(dt, (area.width(), area.height()), &spots, pointer);
                            if self.whispers.heard != self.cfg.whispers_heard || self.whispers.spoken != self.cfg.whispers_spoken {
                                self.cfg.whispers_heard = self.whispers.heard;
                                self.cfg.whispers_spoken = self.whispers.spoken;
                                config::save(&self.cfg);
                            }
                            ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
                        }
                        ui::underworld::paint_whispers(ui, &self.whispers);
                    } else {
                        ui::underworld::paint_souls(ui, uw_time);
                    }
                }
                self.moon_level = moon_events.as_ref().map_or(0.0, |e| e.glitch).max(dead_level);
                // the sound of the portal and of him coming out of it: made while the moon is
                // being flicked, played on the hit, cut if the scene is skipped
                match self.moon.phase() {
                    moon::Phase::Held | moon::Phase::Drifting | moon::Phase::Falling => {
                        if let Some(a) = self.audio() {
                            a.prepare(audio::Scene::Portal);
                        }
                    }
                    moon::Phase::Struck if !self.portal_sounded => {
                        self.portal_sounded = true;
                        if let Some(a) = self.audio() {
                            a.play_scene(audio::Scene::Portal);
                        }
                    }
                    moon::Phase::Struck => {}
                    _ => self.portal_sounded = false,
                }
                if let Some(events) = moon_events {
                    self.shake_window(ctx, events.shake);
                    if events.skipped {
                        if let Some(a) = &self.audio {
                            a.stop_effects();
                        }
                    }
                    if events.finished {
                        // the moon struck the staff and the sands are out: Game 3 begins
                        self.cfg.void_staff_broken = true;
                        config::save(&self.cfg);
                        self.moon = moon::Moon::new();
                        self.portal_sounded = false;
                        if self.game3.is_none() {
                            self.start_game3();
                        }
                    }
                } else {
                    self.shake_window(ctx, None);
                }
                if staff_taken && self.game.is_none() && self.game2.is_none() {
                    // the staff is yours: Game 2 (he gets it back when it is over)
                    self.game_runs += 1;
                    let seed = self.game_runs.wrapping_mul(7919).wrapping_add((ui.input(|i| i.time) * 1000.0) as u32);
                    self.game2 = Some(game2::Game::with_cheats(seed, self.cfg.games.difficulty(1), self.cfg.games.active(1)));
                    ui.ctx().request_repaint();
                }
                if alley_clicked && release::ally_shown() && self.game.is_none() && self.cat_scene.is_none() && self.ally_scene.is_none() && self.cfg.games.ally_open() {
                    // the alley: warped by Time, and it has you
                    self.game_runs += 1;
                    self.cfg.games.ally_mut().enter();
                    self.ally_scene = Some(ui::ally::Scene::new(ui.input(|i| i.time)));
                    ui.ctx().request_repaint();
                }
                if release::story() && (start_game || cat_clicked) && self.game.is_none() && self.cat_scene.is_none() && self.ally_scene.is_none() {
                    let time = ui.input(|i| i.time);
                    if self.cfg.games.cat_helped() {
                        // you already said yes to him: straight into the game
                        self.game_runs += 1;
                        let seed = self.game_runs.wrapping_mul(7919).wrapping_add((time * 1000.0) as u32);
                        self.game = Some(game1::Game::start(seed, self.cfg.games.difficulty(0), self.cfg.games.active(0), game1::Part::from_checkpoint(self.cfg.games.game1_checkpoint())));
                    } else {
                        // the cat first (five clicks on the clock is a temporary way in)
                        self.game_runs += 1;
                        let seed = self.game_runs.wrapping_mul(7919).wrapping_add((time * 1000.0) as u32);
                        self.cat_scene = Some(ui::cat_talk::Scene::new(self.cfg.games.cat_declines(), seed, time));
                    }
                    ui.ctx().request_repaint();
                }
            });
        // Options window (accent color, always-on-top, startup, auto-switch modes)
        if self.show_options {
            let mut open = true;
            let modes = self.available_performance_modes.clone();
            let mut cfg = self.cfg.clone();
            let mut startup = self.startup_enabled;
            let mut res = ui::options::OptionsResult::default();
            if cfg.run_as_admin {
                self.login_task.refresh();
            }
            let login_task_present = self.login_task.present;
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
                        login_task_present,
                        self.coin_tex.as_ref(),
                    );
                });
            self.cfg = cfg;
            self.startup_enabled = startup;
            for id in std::mem::take(&mut res.trophies_given) {
                if let Some(t) = trophies::trophy(id) {
                    self.trophy_notice.push(t);
                }
            }
            if let Some(item) = res.shop_bought.and_then(shop::item) {
                self.message_manager.add_message(status_message(format!("Bought {}.", item.name)));
            }

            if res.accent_changed {
                theme::set_accent(egui::Color32::from_rgb(
                    self.cfg.accent[0],
                    self.cfg.accent[1],
                    self.cfg.accent[2],
                ));
                self.applied_accent = None; // re-apply egui selection color next frame
            }
            if res.tray_changed {
                self.update_tray(ctx);
            }
            if res.gold_preview {
                // show him: the Info tab, and he laughs
                self.cfg.hidden_tabs.retain(|k| k != "info");
                self.tab = Tab::Info;
                self.king_laugh = Some(ctx.input(|i| i.time));
                self.show_options = false;
            }
            if res.break_staff || res.take_life {
                // the staff is broken without the moon: straight to Game 3 (and, for the test switch,
                // straight to the end of it)
                self.show_options = false;
                self.moon = moon::Moon::new();
                self.portal_sounded = false;
                if self.game3.is_none() {
                    self.start_game3();
                }
                if res.take_life {
                    if let Some(run) = &mut self.game3 {
                        run.take_life();
                    }
                }
            }
            if res.game1_part2 || res.game2_start {
                // the test buttons: straight into the game, whatever else was on
                self.show_options = false;
                self.game = None;
                self.game2 = None;
                self.game_runs += 1;
                let seed = self.game_runs.wrapping_mul(7919).wrapping_add((ctx.input(|i| i.time) * 1000.0) as u32);
                if res.game1_part2 {
                    self.game = Some(game1::Game::start(seed, self.cfg.games.difficulty(0), self.cfg.games.active(0), game1::Part::Two));
                } else {
                    self.game2 = Some(game2::Game::with_cheats(seed, self.cfg.games.difficulty(1), self.cfg.games.active(1)));
                }
            }
            if res.ally_enter && self.ally_scene.is_none() {
                self.show_options = false;
                self.game = None;
                self.game2 = None;
                self.game_runs += 1;
                self.cfg.games.ally_mut().enter();
                self.ally_scene = Some(ui::ally::Scene::new(ctx.input(|i| i.time)));
            }
            if res.cat_talk {
                // the test button: the talk, whether or not you have helped him before
                self.show_options = false;
                self.game = None;
                self.game2 = None;
                self.game_runs += 1;
                let time = ctx.input(|i| i.time);
                let seed = self.game_runs.wrapping_mul(7919).wrapping_add((time * 1000.0) as u32);
                self.cat_scene = Some(ui::cat_talk::Scene::new(self.cfg.games.cat_declines(), seed, time));
            }
            if res.poker_sit || res.poker_test {
                self.start_poker(res.poker_test);
                if self.poker.is_some() {
                    self.show_options = false;
                }
            }
            if res.login_task_setup {
                self.login_task.error = None;
                self.login_task.dialog_open = true;
            }
            if res.login_task_remove && self.login_task.busy.is_none() {
                self.login_task.remove();
            }
            if res.admin_restart {
                config::save(&self.cfg);
                if admin::relaunch_as_admin() {
                    // the elevated copy takes over; this one closes
                    self.should_quit = true;
                    ctx.request_repaint();
                } else {
                    self.set_status_message("Administrator permission was not granted".to_string());
                }
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
            if res.sensor_source_changed && self.sensors.is_on() {
                // read from the newly chosen source: restart with fresh statistics
                self.sensors.stop();
                self.sensors.start(self.cfg.sensor_source_config());
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

        self.run_glitch(ctx, minimized);
        self.run_moon_glitch(ctx, minimized);

        // Fit the window height to the content (capped to the monitor) whenever the content
        // height changes, e.g. when the Custom boost rows appear. The user can still resize.
        if content_height > 0.0 && !minimized && self.fully_initialized && self.window_stretch.is_none() {
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
    if cfg.run_as_admin
        && !admin::is_elevated()
        && !admin::started_elevated()
        && admin::relaunch_as_admin()
    {
        // an elevated copy was started: this one is done (if the permission prompt is declined
        // or elevating is impossible, R-Helper just continues without administrator rights)
        return Ok(());
    }
    let keep_on_top = cfg.always_on_top;
    let start_hidden = cfg.start_minimized;
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
            .with_visible(!cfg.start_minimized)
            .with_always_on_top()
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            if start_hidden {
                // a window that starts hidden may not be asked to draw by anything else: nudge it
                // for a few seconds so the first frame runs and the tray icon appears
                let nudge = cc.egui_ctx.clone();
                std::thread::spawn(move || {
                    for _ in 0..20 {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        nudge.request_repaint();
                    }
                });
            }
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
