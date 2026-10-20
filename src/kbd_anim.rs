//! Background worker for software-animated keyboard lighting (Breathing with two halves or a
//! gradient, and the Fire, Tidal and Starlight effects, which the keyboard firmware cannot do).
//!
//! The worker opens its own handle to the laptop's HID device and repaints the whole keyboard
//! as fast as the keyboard accepts frames, scaling the colors up and down. Every HID exchange is serialized in
//! `librazer::device::Device::send`, so it cannot get in the way of the UI thread's polling.
//!
//! The animation only runs while the user has picked such a Breathing layout. Choosing any other
//! keyboard effect calls `Handle::stop`, which also waits for the frame that is being sent, so no
//! stale frame can land after the new effect.
use librazer::command::{self, KeyboardColors};
use librazer::descriptor::Descriptor;
use librazer::fire::Fire;
use librazer::device::Device;
use librazer::keymap::{KeyGrid, KeyLayout};
use librazer::dipp::Dipp;
use librazer::rainbow::Rainbow;
use librazer::spectrum::Spectrum;
use librazer::starlight::Starlight;
use librazer::tidal::Tidal;
use librazer::wave::Wave;
use librazer::wheel::Wheel;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

/// Pause between two frames. Sending a frame (seven HID exchanges) already takes long enough to
/// leave the CPU alone, so this is only a short breather for the bus.
const FRAME_GAP: Duration = Duration::from_millis(1);
/// How often an idle worker looks for something to do.
const IDLE_POLL: Duration = Duration::from_millis(100);
/// Consecutive failed frames after which the animation gives up.
const MAX_FAILS: u8 = 8;
/// Consecutive failed quick frames after which the worker switches to the slower, patient exchange
/// (and later reopens the device).
const QUICK_FAILS: u8 = 3;

/// What the worker draws.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    /// Breathing with halves, a gradient or a custom speed.
    Breathing,
    /// The flame (the Fire effect).
    Fire,
    /// The waves of the Tidal effect; `inward` = they come in instead of jetting out.
    Tidal { inward: bool },
    /// Custom Keys with keys that move (the layout is in the job).
    Keys,
    /// Stars twinkling on the keys (the job's colors: the first, and the second if there is one).
    Starlight,
    /// The whole keyboard going round the rainbow (the Spectrum effect).
    Spectrum,
    /// The whole keyboard going from the job's first color to its second and back.
    Dipp,
    /// The color wheel across the keys, scrolling along.
    Rainbow,
    /// Wave Right / Wave Left in picked colors (the job's first color, and its second if there is one).
    Wave { left: bool },
    /// A hand sweeping round the middle of the keyboard (the job's first color, the tail turning
    /// into its second); `ccw` = counter-clockwise.
    /// `rainbow`: Razer's own Wheel (the turning rainbow); off, the two colors you picked, going from
    /// one to the other through the hues between.
    Wheel { ccw: bool, rainbow: bool },
}

struct Job {
    descriptor: Descriptor,
    colors: KeyboardColors,
    /// A percent of the effect's standard speed (see `command::STANDARD_SPEED`).
    speed: u8,
    kind: Kind,
    /// Kind::Keys: the layout and the column shift to draw it with.
    keys: Option<(KeyLayout, i8)>,
}

#[derive(Default)]
struct Shared {
    /// What to animate right now (`None` = idle).
    job: Mutex<Option<Job>>,
    /// Held while one frame is being sent, so `stop` can wait for it.
    frame: Mutex<()>,
    /// The reason the animation gave up, for the UI's status line (taken once).
    error: Mutex<Option<String>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct Handle {
    shared: Arc<Shared>,
}

impl Handle {
    pub fn spawn() -> Self {
        let shared = Arc::new(Shared::default());
        let worker_shared = Arc::clone(&shared);
        let _ = thread::Builder::new().name("keyboard-animation".into()).spawn(move || {
            run(worker_shared);
        });
        Self { shared }
    }

    /// Starts (or changes) the animation. `descriptor` identifies the laptop's HID device and
    /// `speed` is a percent of the standard speed.
    pub fn start(&self, descriptor: Descriptor, colors: KeyboardColors, speed: u8) {
        *lock(&self.shared.error) = None;
        *lock(&self.shared.job) = Some(Job { descriptor, colors, speed, kind: Kind::Breathing, keys: None });
    }

    /// Starts the Fire effect, or changes its colors while it burns (`hot` and `cold` as in
    /// Chroma's Fire: the cold color is the dim cells at the top, the hot color the bright ones).
    pub fn start_fire(&self, descriptor: Descriptor, hot: [u8; 3], cold: [u8; 3], speed: u8) {
        *lock(&self.shared.error) = None;
        let colors = KeyboardColors { primary: hot, secondary: Some(cold), split: Default::default() };
        *lock(&self.shared.job) = Some(Job { descriptor, colors, speed, kind: Kind::Fire, keys: None });
    }

    /// Starts the Tidal effect, or changes its colors or direction while it runs (`a` is the
    /// color that comes first in each key's cycle, `b` the one after it).
    pub fn start_tidal(
        &self,
        descriptor: Descriptor,
        a: [u8; 3],
        b: [u8; 3],
        inward: bool,
        speed: u8,
    ) {
        *lock(&self.shared.error) = None;
        let colors = KeyboardColors { primary: a, secondary: Some(b), split: Default::default() };
        *lock(&self.shared.job) =
            Some(Job { descriptor, colors, speed, kind: Kind::Tidal { inward }, keys: None });
    }

    /// Starts the Starlight effect, or changes its colors while the stars twinkle (`b` is the
    /// second color, if it is switched on).
    pub fn start_starlight(
        &self,
        descriptor: Descriptor,
        a: [u8; 3],
        b: Option<[u8; 3]>,
        speed: u8,
    ) {
        *lock(&self.shared.error) = None;
        let colors = KeyboardColors { primary: a, secondary: b, split: Default::default() };
        *lock(&self.shared.job) =
            Some(Job { descriptor, colors, speed, kind: Kind::Starlight, keys: None });
    }

    /// Starts the Spectrum effect (the keyboard's color goes round the rainbow like Chroma's).
    pub fn start_spectrum(&self, descriptor: Descriptor, speed: u8) {
        *lock(&self.shared.error) = None;
        let colors = KeyboardColors { primary: [0, 0, 0], secondary: None, split: Default::default() };
        *lock(&self.shared.job) =
            Some(Job { descriptor, colors, speed, kind: Kind::Spectrum, keys: None });
    }

    /// Starts the Rainbow effect, or changes its speed while it scrolls.
    pub fn start_rainbow(&self, descriptor: Descriptor, speed: u8) {
        *lock(&self.shared.error) = None;
        let colors = KeyboardColors { primary: [0, 0, 0], secondary: None, split: Default::default() };
        *lock(&self.shared.job) =
            Some(Job { descriptor, colors, speed, kind: Kind::Rainbow, keys: None });
    }

    /// Starts Color Dipp, or changes its two colors while it runs (it keeps going from where it
    /// is).
    pub fn start_dipp(&self, descriptor: Descriptor, a: [u8; 3], b: [u8; 3], speed: u8) {
        *lock(&self.shared.error) = None;
        let colors = KeyboardColors { primary: a, secondary: Some(b), split: Default::default() };
        *lock(&self.shared.job) =
            Some(Job { descriptor, colors, speed, kind: Kind::Dipp, keys: None });
    }

    /// Starts the Wave in colors of your own (`b` is the second color, if there is one), or changes
    /// its colors, direction or speed while the bands travel.
    pub fn start_wave(&self, descriptor: Descriptor, a: [u8; 3], b: Option<[u8; 3]>, left: bool, speed: u8) {
        *lock(&self.shared.error) = None;
        let colors = KeyboardColors { primary: a, secondary: b, split: Default::default() };
        *lock(&self.shared.job) =
            Some(Job { descriptor, colors, speed, kind: Kind::Wave { left }, keys: None });
    }

    /// Starts the Wheel, or changes its look, colors, direction or speed while it goes round.
    /// `rainbow` is Razer's own (no colors); off, the wheel goes from `a` to `b` and back.
    pub fn start_wheel(&self, descriptor: Descriptor, a: [u8; 3], b: [u8; 3], ccw: bool, rainbow: bool, speed: u8) {
        *lock(&self.shared.error) = None;
        let colors = KeyboardColors { primary: a, secondary: Some(b), split: Default::default() };
        *lock(&self.shared.job) =
            Some(Job { descriptor, colors, speed, kind: Kind::Wheel { ccw, rainbow }, keys: None });
    }

    /// Starts Custom Keys with keys that move, or changes the layout while it runs. `speed` is an
    /// percent of the standard speed (how fast Breathing, Spectrum and Flash go).
    pub fn start_keys(&self, descriptor: Descriptor, layout: KeyLayout, shift: i8, speed: u8) {
        *lock(&self.shared.error) = None;
        let colors = KeyboardColors { primary: [0, 0, 0], secondary: None, split: Default::default() };
        *lock(&self.shared.job) =
            Some(Job { descriptor, colors, speed, kind: Kind::Keys, keys: Some((layout, shift)) });
    }

    /// Stops the animation and returns once the frame in flight (if any) has been sent.
    pub fn stop(&self) {
        *lock(&self.shared.job) = None;
        drop(lock(&self.shared.frame));
    }

    pub fn is_running(&self) -> bool {
        lock(&self.shared.job).is_some()
    }

    /// The reason the animation stopped by itself, if it did since the last call.
    pub fn take_error(&self) -> Option<String> {
        lock(&self.shared.error).take()
    }
}

fn run(shared: Arc<Shared>) {
    let mut device: Option<Device> = None;
    let mut running: Option<(KeyboardColors, u8, Kind, Option<(KeyLayout, i8)>)> = None;
    let mut keying = false;
    let mut keys_started = Instant::now();
    // The flame or the tide is already going (a new color then does not start it over).
    let mut burning = false;
    let mut tiding = false;
    let mut flame = Fire::new(0);
    let mut tide = Tidal::new();
    let mut sky = Starlight::new(1);
    let mut starry = false;
    let mut rainbow = Spectrum::new();
    let mut rainbowing = false;
    let mut dipper = Dipp::new();
    let mut dipping = false;
    let mut arch = Rainbow::new();
    let mut arching = false;
    let mut waver = Wave::new();
    let mut waving = false;
    let mut wheeler = Wheel::new();
    let mut wheeling = false;
    let mut last_frame = Instant::now();
    // Quick exchanges first; if the keyboard keeps refusing them, fall back to the patient ones.
    let mut quick = true;
    let mut started = Instant::now();
    let mut last_sent: Option<KeyGrid> = None;
    let mut fails: u8 = 0;

    loop {
        // Only the worker is left: the app is shutting down.
        if Arc::strong_count(&shared) == 1 {
            return;
        }

        let snapshot = lock(&shared.job)
            .as_ref()
            .map(|job| (job.descriptor.clone(), job.colors, job.speed, job.kind, job.keys.clone()));
        let Some((descriptor, colors, speed, kind, keys)) = snapshot else {
            running = None;
            burning = false;
            tiding = false;
            keying = false;
            starry = false;
            rainbowing = false;
            dipping = false;
            arching = false;
            waving = false;
            wheeling = false;
            thread::sleep(IDLE_POLL);
            continue;
        };

        let fire = kind == Kind::Fire;
        let tidal = matches!(kind, Kind::Tidal { .. });
        let keyed = kind == Kind::Keys;
        if running != Some((colors, speed, kind, keys.clone())) {
            // New colors, layout or speed: restart the fade from dark. A fire that is already
            // burning only changes color (it keeps flickering); one that was not is lit. The
            // same goes for the tide.
            let relight = fire && !burning;
            let retide = tidal && !tiding;
            // (editing the keys while they move does not start their motion over)
            let rekey = keyed && !keying;
            let restar = kind == Kind::Starlight && !starry;
            let respectrum = kind == Kind::Spectrum && !rainbowing;
            let redip = kind == Kind::Dipp && !dipping;
            let rearch = kind == Kind::Rainbow && !arching;
            let rewave = matches!(kind, Kind::Wave { .. }) && !waving;
            let rewheel = matches!(kind, Kind::Wheel { .. }) && !wheeling;
            running = Some((colors, speed, kind, keys.clone()));
            burning = fire;
            tiding = tidal;
            keying = keyed;
            starry = kind == Kind::Starlight;
            rainbowing = kind == Kind::Spectrum;
            dipping = kind == Kind::Dipp;
            arching = kind == Kind::Rainbow;
            waving = matches!(kind, Kind::Wave { .. });
            wheeling = matches!(kind, Kind::Wheel { .. });
            started = Instant::now();
            if restar {
                last_frame = started;
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(1, |d| d.as_nanos() as u64);
                sky = Starlight::new(seed);
            }
            if rearch {
                last_frame = started;
                arch = Rainbow::new();
            }
            if rewave {
                last_frame = started;
                waver = Wave::new();
            }
            if rewheel {
                last_frame = started;
                wheeler = Wheel::new();
            }
            if redip {
                last_frame = started;
                dipper = Dipp::new();
            }
            if respectrum {
                last_frame = started;
                rainbow = Spectrum::new();
            }
            if rekey {
                keys_started = started;
                last_frame = started;
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(1, |d| d.as_nanos() as u64);
                flame = Fire::new(seed);
            }
            if retide {
                last_frame = started;
                tide = Tidal::new();
            }
            if relight {
                last_frame = started;
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(1, |d| d.as_nanos() as u64);
                flame = Fire::new(seed);
            }
            last_sent = None;
            fails = 0;
            quick = true;
        }

        let frame_guard = lock(&shared.frame);
        // `stop` may have run between reading the job and getting the frame lock.
        if lock(&shared.job).is_none() {
            drop(frame_guard);
            continue;
        }

        if device.is_none() {
            device = Device::new(descriptor).ok();
        }
        // (how much faster or slower than standard the effect runs; Breathing and the moving keys
        // take their speed through the period instead)
        let pace = if matches!(kind, Kind::Breathing | Kind::Keys) {
            1.0
        } else {
            command::speed_factor(speed)
        };
        let grid = if let Kind::Wave { left } = kind {
            let now = Instant::now();
            waver.advance(now.duration_since(last_frame).as_secs_f32() * pace);
            last_frame = now;
            waver.grid(colors.primary, colors.secondary, left)
        } else if let Kind::Wheel { ccw, rainbow } = kind {
            let now = Instant::now();
            wheeler.advance(now.duration_since(last_frame).as_secs_f32() * pace);
            last_frame = now;
            if rainbow {
                wheeler.rainbow_grid(ccw)
            } else {
                wheeler.dipp_grid(colors.primary, colors.secondary.unwrap_or(colors.primary), ccw)
            }
        } else if kind == Kind::Rainbow {
            let now = Instant::now();
            arch.advance(now.duration_since(last_frame).as_secs_f32() * pace);
            last_frame = now;
            arch.grid()
        } else if kind == Kind::Dipp {
            let now = Instant::now();
            dipper.advance(now.duration_since(last_frame).as_secs_f32() * pace);
            last_frame = now;
            dipper.grid(colors.primary, colors.secondary.unwrap_or(colors.primary))
        } else if kind == Kind::Spectrum {
            let now = Instant::now();
            rainbow.advance(now.duration_since(last_frame).as_secs_f32() * pace);
            last_frame = now;
            rainbow.grid()
        } else if kind == Kind::Starlight {
            let now = Instant::now();
            sky.advance(now.duration_since(last_frame).as_secs_f32() * pace);
            last_frame = now;
            sky.grid(colors.primary, colors.secondary)
        } else if let (Kind::Keys, Some((layout, shift))) = (kind, keys.as_ref()) {
            let now = Instant::now();
            flame.advance(now.duration_since(last_frame).as_secs_f32() * pace);
            last_frame = now;
            layout.grid_at(*shift, keys_started.elapsed().as_secs_f32(), speed, &flame)
        } else if let Kind::Tidal { inward } = kind {
            let now = Instant::now();
            tide.advance(now.duration_since(last_frame).as_secs_f32() * pace);
            last_frame = now;
            tide.grid(colors.primary, colors.secondary.unwrap_or(colors.primary), inward)
        } else if fire {
            let now = Instant::now();
            flame.advance(now.duration_since(last_frame).as_secs_f32() * pace);
            last_frame = now;
            flame.grid(colors.primary, colors.secondary.unwrap_or(colors.primary))
        } else {
            let t = started.elapsed().as_secs_f32();
            let period = command::breath_period(speed);
            command::breathing_grid(
                &command::breath_cycle_colors(&colors, command::breath_cycle(t, period)),
                command::breath_level_with(t, period),
            )
        };
        let sent = match device.as_ref() {
            Some(_) if last_sent.as_ref() == Some(&grid) => true, // nothing changed visibly
            Some(dev) if quick => command::set_keyboard_grid_quick(dev, &grid).is_ok(),
            Some(dev) => command::set_keyboard_grid(dev, &grid).is_ok(),
            None => false,
        };
        if sent {
            last_sent = Some(grid);
            fails = 0;
        } else {
            fails += 1;
            if quick && fails >= QUICK_FAILS {
                quick = false; // the keyboard wants the slower exchange
                fails = 0;
            } else if !quick && fails >= QUICK_FAILS {
                device = None; // reopen next time
            }
            if !quick && fails >= MAX_FAILS {
                *lock(&shared.job) = None;
                *lock(&shared.error) =
                    Some("Keyboard animation stopped: the keyboard did not accept frames".into());
            }
        }
        drop(frame_guard);

        thread::sleep(FRAME_GAP);
    }
}
