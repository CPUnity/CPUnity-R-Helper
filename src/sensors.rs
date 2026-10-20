//! Opt-in hardware readings (temperatures, power, clocks) for the Sensors tab.
//!
//! Nothing is read until the user switches the readings on: `Handle::start` spawns the sampling
//! thread and opens the data sources, `Handle::stop` makes the thread close them again and exit.
//! No driver is installed and no administrator rights are needed:
//!
//! * GPU: NVIDIA's NVML library (`nvml.dll`, installed with the graphics driver) gives the GPU
//!   temperature, power and load, and the memory temperature where the driver reports it. The
//!   clock comes from NVAPI (`nvapi64.dll`, the same library Afterburner uses) when it answers,
//!   which follows the GPU's real speed, and from NVML otherwise.
//! * CPU and system: Windows performance counters (PDH) give the CPU clock and load, the CPU
//!   package power where Windows exposes the "Energy Meter" counter, and the ACPI thermal zones.
//!
//! The CPU core / package temperature and the PL1 / PL2 power limits are model specific registers,
//! which need a kernel driver, so they are not available this way. A program that already has
//! such a driver can supply them instead: the "Sensor source" option (see `sensor_ext`) reads
//! HWiNFO, LibreHardwareMonitor or a CSV log (ThrottleStop, XTU) and lays its values over the
//! built-in ones.
use crate::log_plan::{Plan, Runner, Start, Why};
use crate::sensor_ext::{merge, External, SourceConfig, SourceState, SOURCE_COUNT};
use crate::sensor_history::History;
use crate::sensor_log::{log_dir, Logger};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

/// Time between two readings unless the user changed it ("factory" setting).
pub const DEFAULT_INTERVAL_MS: u64 = 1000;
/// Shortest and longest time between two readings that can be chosen.
pub const MIN_INTERVAL_MS: u64 = 250;
pub const MAX_INTERVAL_MS: u64 = 10_000;

/// `ms` limited to what can be chosen.
pub fn clamp_interval(ms: u64) -> u64 {
    ms.clamp(MIN_INTERVAL_MS, MAX_INTERVAL_MS)
}
/// How often a waiting sampler checks whether it should stop.
const STOP_POLL: Duration = Duration::from_millis(50);

/// One value over time: the latest reading, the average of all readings since the start (or the
/// last reset) and the highest one.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Stat {
    pub now: Option<f32>,
    pub avg: Option<f32>,
    pub max: Option<f32>,
    sum: f64,
    count: u64,
}

impl Stat {
    /// Adds a reading; `None` (not available this time) only clears the latest value.
    pub fn push(&mut self, value: Option<f32>) {
        self.now = value;
        if let Some(v) = value.filter(|v| v.is_finite()) {
            self.sum += v as f64;
            self.count += 1;
            self.avg = Some((self.sum / self.count as f64) as f32);
            self.max = Some(self.max.map_or(v, |m| m.max(v)));
        }
    }

    pub fn reset(&mut self) {
        *self = Stat::default();
    }

    /// True once at least one real reading has arrived.
    pub fn seen(&self) -> bool {
        self.count > 0
    }
}

/// One round of raw readings. `None` = this source does not provide the value.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sample {
    pub cpu_temp: Option<f32>,
    pub core_temp: Option<f32>,
    pub pl1: Option<f32>,
    pub pl2: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub gpu_mem_temp: Option<f32>,
    pub gpu_power: Option<f32>,
    pub gpu_clock: Option<f32>,
    pub gpu_load: Option<f32>,
    pub cpu_zone_temp: Option<f32>,
    pub cpu_power: Option<f32>,
    pub cpu_clock: Option<f32>,
    /// The fastest core, and the average of the performance and of the efficiency cores (read from
    /// each core's own register, so only with the PawnIO driver).
    pub cpu_clock_top: Option<f32>,
    pub cpu_clock_p: Option<f32>,
    pub cpu_clock_e: Option<f32>,
    pub cpu_load: Option<f32>,
}

/// All values with their running statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct Readings {
    pub cpu_temp: Stat,
    pub core_temp: Stat,
    pub pl1: Stat,
    pub pl2: Stat,
    pub gpu_temp: Stat,
    pub gpu_mem_temp: Stat,
    pub gpu_power: Stat,
    pub gpu_clock: Stat,
    pub gpu_load: Stat,
    pub cpu_zone_temp: Stat,
    pub cpu_power: Stat,
    pub cpu_clock: Stat,
    pub cpu_clock_top: Stat,
    pub cpu_clock_p: Stat,
    pub cpu_clock_e: Stat,
    pub cpu_load: Stat,
    /// Readings taken since the start (or the last reset).
    pub samples: u64,
    /// How each source is doing (index = source number; see `sensor_ext`).
    pub sources: [SourceState; SOURCE_COUNT],
}

impl Readings {
    pub fn add(&mut self, s: &Sample) {
        self.cpu_temp.push(s.cpu_temp);
        self.core_temp.push(s.core_temp);
        self.pl1.push(s.pl1);
        self.pl2.push(s.pl2);
        self.gpu_temp.push(s.gpu_temp);
        self.gpu_mem_temp.push(s.gpu_mem_temp);
        self.gpu_power.push(s.gpu_power);
        self.gpu_clock.push(s.gpu_clock);
        self.gpu_load.push(s.gpu_load);
        self.cpu_zone_temp.push(s.cpu_zone_temp);
        self.cpu_power.push(s.cpu_power);
        self.cpu_clock.push(s.cpu_clock);
        self.cpu_clock_top.push(s.cpu_clock_top);
        self.cpu_clock_p.push(s.cpu_clock_p);
        self.cpu_clock_e.push(s.cpu_clock_e);
        self.cpu_load.push(s.cpu_load);
        self.samples += 1;
    }

    /// Restarts the statistics; the link state is kept.
    pub fn reset(&mut self) {
        *self = Readings { sources: self.sources, ..Readings::default() };
    }
}

/// The CSV log, as far as the window needs to know.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogInfo {
    /// File being written right now.
    pub path: Option<PathBuf>,
    /// The file could not be created or written.
    pub failed: bool,
    /// Clock time (unix seconds) the file being written was started.
    pub since_unix: Option<i64>,
    /// Readings written to it so far.
    pub rows: u64,
    /// The time limit it was started with (0 = none).
    pub limit_secs: u64,
    /// Switched on, but waiting for the program to start.
    pub waiting: bool,
    /// The last file that was finished, and why.
    pub last: Option<(PathBuf, Why)>,
}

#[derive(Default)]
struct Shared {
    readings: Mutex<Readings>,
    /// The last hour of readings, for the graphs.
    history: Mutex<History>,
    /// Write the readings to a CSV file (only happens while sampling).
    logging: AtomicBool,
    log: Mutex<LogInfo>,
    /// What the user typed as the log's name (the date is added in front).
    log_name: Mutex<String>,
    /// When the log starts and stops.
    plan: Mutex<Plan>,
    /// Milliseconds between two readings (0 until set = the default).
    interval_ms: AtomicU64,
    /// The graphs are switched off: nothing is recorded for them.
    history_off: AtomicBool,
    /// Sampling is wanted.
    on: AtomicBool,
    /// Incremented by every `start`, so a sampler that is still winding down notices that a newer
    /// one took over and leaves.
    generation: AtomicU64,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct Handle {
    shared: Arc<Shared>,
}

impl Handle {
    /// Creates the handle. No thread runs and nothing is opened until `start`.
    pub fn new() -> Self {
        Self { shared: Arc::new(Shared::default()) }
    }

    pub fn is_on(&self) -> bool {
        self.shared.on.load(Ordering::SeqCst)
    }

    /// Starts reading from the built-in sources plus the external one in `source` (a no-op while
    /// it already is). Statistics start from zero.
    pub fn start(&self, source: SourceConfig) {
        if self.shared.on.swap(true, Ordering::SeqCst) {
            return;
        }
        *lock(&self.shared.readings) = Readings::default();
        lock(&self.shared.history).clear();
        let generation = self.shared.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let shared = Arc::clone(&self.shared);
        let spawned = thread::Builder::new()
            .name("sensors".into())
            .spawn(move || run(shared, generation, source));
        if spawned.is_err() {
            self.shared.on.store(false, Ordering::SeqCst);
        }
    }

    /// Stops reading; the sampler closes its data sources and exits. The last values stay visible.
    pub fn stop(&self) {
        self.shared.on.store(false, Ordering::SeqCst);
    }

    /// Restarts the averages and maximums (the graphs keep their history).
    pub fn reset(&self) {
        lock(&self.shared.readings).reset();
    }

    /// Switches the CSV log on or off. Lines are only written while the readings run.
    pub fn set_logging(&self, on: bool) {
        self.shared.logging.store(on, Ordering::SeqCst);
        if on {
            lock(&self.shared.log).failed = false;
        }
    }

    /// Whether the logging switch is on (a log with a time limit switches it off when it is done).
    pub fn logging(&self) -> bool {
        self.shared.logging.load(Ordering::SeqCst)
    }

    /// When logs start and stop (takes effect right away).
    pub fn set_log_plan(&self, plan: Plan) {
        *lock(&self.shared.plan) = plan;
    }

    /// The name for the next log file (applies when logging starts).
    pub fn set_log_name(&self, name: &str) {
        *lock(&self.shared.log_name) = name.to_string();
    }

    pub fn log_info(&self) -> LogInfo {
        lock(&self.shared.log).clone()
    }

    /// Sets the time between two readings (takes effect right away, also while running).
    pub fn set_interval(&self, ms: u64) {
        self.shared.interval_ms.store(clamp_interval(ms), Ordering::SeqCst);
    }

    /// Switches recording for the graphs on or off. Off also drops what was recorded.
    pub fn set_history(&self, on: bool) {
        self.shared.history_off.store(!on, Ordering::SeqCst);
        if !on {
            lock(&self.shared.history).clear();
        }
    }

    /// Runs `f` on the recorded history without copying it.
    pub fn with_history<R>(&self, f: impl FnOnce(&History) -> R) -> R {
        f(&lock(&self.shared.history))
    }

    pub fn snapshot(&self) -> Readings {
        *lock(&self.shared.readings)
    }
}

/// The time between two readings right now.
fn interval(shared: &Shared) -> Duration {
    match shared.interval_ms.load(Ordering::SeqCst) {
        0 => Duration::from_millis(DEFAULT_INTERVAL_MS),
        ms => Duration::from_millis(clamp_interval(ms)),
    }
}

fn run(shared: Arc<Shared>, generation: u64, source: SourceConfig) {
    let wanted = |shared: &Shared| {
        shared.on.load(Ordering::SeqCst) && shared.generation.load(Ordering::SeqCst) == generation
    };
    // Opened here, closed when this function returns: nothing stays open while the switch is off.
    let mut sources = Sources::open();
    let mut external = External::new(source.clone());
    let started = Instant::now();
    let mut log = LogState::default();
    while wanted(&shared) {
        let builtin = sources.sample();
        let sample = merge(&builtin, &external.sample(), &source);
        if !wanted(&shared) {
            break;
        }
        let mut readings = lock(&shared.readings);
        readings.add(&sample);
        readings.sources = external.states();
        drop(readings);
        if !shared.history_off.load(Ordering::SeqCst) {
            lock(&shared.history).push(started.elapsed().as_secs_f64(), &sample);
        }
        let remaining = write_log(&shared, &mut log, &sample, started.elapsed().as_secs_f64());
        // the last reading of a log with a time limit is taken when the time is up, not a whole
        // interval later
        let wait = match remaining {
            Some(left) => interval(&shared).min(Duration::from_secs_f64(left)),
            None => interval(&shared),
        };
        let mut waited = Duration::ZERO;
        while waited < wait && wanted(&shared) {
            thread::sleep(STOP_POLL.min(wait - waited));
            waited += STOP_POLL;
        }
    }
    if log.logger.is_some() {
        let mut info = lock(&shared.log);
        info.path = None;
        info.since_unix = None;
    }
}

/// What the sampler keeps for the CSV log.
#[derive(Default)]
struct LogState {
    logger: Option<Logger>,
    runner: Runner,
    watch: Watch,
    rows: u64,
}

/// Looks for the program that starts the log, not more than once every few seconds.
#[derive(Default)]
struct Watch {
    program: String,
    checked: Option<Instant>,
}

impl Watch {
    const EVERY: Duration = Duration::from_secs(3);

    /// The answer when it was looked up this time, `None` when the last one still holds.
    fn poll(&mut self, program: &str) -> Option<bool> {
        let due =
            self.program != program || self.checked.is_none_or(|t| t.elapsed() >= Self::EVERY);
        if !due {
            return None;
        }
        self.program = program.to_string();
        self.checked = Some(Instant::now());
        crate::log_plan::is_running(program)
    }
}

/// Keeps the CSV log in line with the switch and the plan and appends this round. Returns the
/// seconds left until the time limit, when a log with one is open.
fn write_log(shared: &Shared, state: &mut LogState, sample: &Sample, now: f64) -> Option<f64> {
    let plan = lock(&shared.plan).clone();
    let armed = shared.logging.load(Ordering::SeqCst);
    let program =
        if armed && plan.start == Start::Program { state.watch.poll(&plan.program) } else { None };
    let step = state.runner.before(&plan, armed, now, program);
    if let Some(why) = step.close {
        finish(shared, state, why);
    }
    if step.open {
        let typed = lock(&shared.log_name).clone();
        let name = crate::log_plan::file_label(&plan, &typed, crate::sensor_log::now());
        match Logger::create(&log_dir(), &name, plan.stamp) {
            Ok(created) => {
                let mut info = lock(&shared.log);
                info.failed = false;
                info.path = Some(created.path().to_path_buf());
                info.since_unix = Some(crate::sensor_log::unix_now());
                info.limit_secs = plan.limit_secs;
                info.rows = 0;
                drop(info);
                state.rows = 0;
                state.logger = Some(created);
            }
            Err(_) => {
                state.runner.abort();
                lock(&shared.log).failed = true;
            }
        }
    }
    if let Some(open) = state.logger.as_mut() {
        if open.write(sample).is_err() {
            state.logger = None;
            state.runner.abort();
            let mut info = lock(&shared.log);
            info.path = None;
            info.since_unix = None;
            info.failed = true;
        } else {
            state.rows += 1;
            lock(&shared.log).rows = state.rows;
        }
    }
    if let Some(why) = state.runner.after(&plan, now) {
        finish(shared, state, why);
        // a log that started at once is over: the switch goes off with it
        if plan.start == Start::Now {
            shared.logging.store(false, Ordering::SeqCst);
        }
    }
    lock(&shared.log).waiting = armed && plan.start == Start::Program && !state.runner.recording();
    state.runner.remaining(&plan, now)
}

/// Closes the file and says why.
fn finish(shared: &Shared, state: &mut LogState, why: Why) {
    state.logger = None;
    let mut info = lock(&shared.log);
    if let Some(path) = info.path.take() {
        info.last = Some((path, why));
    }
    info.since_unix = None;
}

#[cfg(not(windows))]
struct Sources;

#[cfg(not(windows))]
impl Sources {
    fn open() -> Self {
        Sources
    }

    fn sample(&mut self) -> Sample {
        Sample::default()
    }
}

#[cfg(windows)]
use win::Sources;

#[cfg(windows)]
mod win {
    use super::Sample;
    use std::ffi::c_void;
    use std::os::raw::{c_char, c_int, c_uint};

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryA(name: *const c_char) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
        fn FreeLibrary(module: *mut c_void) -> c_int;
    }

    #[link(name = "pdh")]
    extern "system" {
        fn PdhOpenQueryW(source: *const u16, user: usize, query: *mut *mut c_void) -> u32;
        fn PdhAddEnglishCounterW(
            query: *mut c_void,
            path: *const u16,
            user: usize,
            counter: *mut *mut c_void,
        ) -> u32;
        fn PdhCollectQueryData(query: *mut c_void) -> u32;
        fn PdhGetFormattedCounterValue(
            counter: *mut c_void,
            format: u32,
            counter_type: *mut u32,
            value: *mut FmtValue,
        ) -> u32;
        fn PdhGetFormattedCounterArrayW(
            counter: *mut c_void,
            format: u32,
            buffer_size: *mut u32,
            item_count: *mut u32,
            items: *mut c_void,
        ) -> u32;
        fn PdhCloseQuery(query: *mut c_void) -> u32;
    }

    pub struct Sources {
        nvml: Option<Nvml>,
        nvapi: Option<Nvapi>,
        pdh: Option<Pdh>,
    }

    impl Sources {
        pub fn open() -> Self {
            Sources { nvml: Nvml::open(), nvapi: Nvapi::open(), pdh: Pdh::open() }
        }

        pub fn sample(&mut self) -> Sample {
            let mut s = Sample::default();
            if let Some(nvml) = &self.nvml {
                nvml.read(&mut s);
            }
            // NVAPI's clock is the one that follows the GPU's real speed (it is what Afterburner and
            // HWiNFO read); NVML's can sit on one value, so NVAPI's replaces it when it answers
            if let Some(clock) = self.nvapi.as_ref().and_then(|n| n.graphics_clock()) {
                s.gpu_clock = Some(clock);
            }
            if let Some(pdh) = &self.pdh {
                pdh.read(&mut s);
            }
            s
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    // ---------------------------------------------------------------- NVAPI

    /// `NV_GPU_CLOCK_FREQUENCIES_V2` (264 bytes): a version, the kind of clock asked for (0 = the
    /// current one) and 32 domains of (flags, frequency in kHz).
    #[repr(C)]
    struct NvapiClocks {
        version: c_uint,
        clock_type: c_uint,
        domains: [[c_uint; 2]; 32],
    }

    struct Nvapi {
        gpu: *mut c_void,
        get_clocks: unsafe extern "C" fn(*mut c_void, *mut NvapiClocks) -> c_int,
    }

    impl Nvapi {
        fn open() -> Option<Nvapi> {
            unsafe {
                let lib = LoadLibraryA(b"nvapi64.dll\0".as_ptr() as *const c_char);
                if lib.is_null() {
                    return None;
                }
                // everything in NVAPI is reached through this one export, by the id of the function
                let query: unsafe extern "C" fn(c_uint) -> *mut c_void =
                    symbol(lib, b"nvapi_QueryInterface\0")?;
                let function = |id: c_uint| -> Option<*mut c_void> {
                    let p = query(id);
                    (!p.is_null()).then_some(p)
                };
                let initialize: unsafe extern "C" fn() -> c_int =
                    std::mem::transmute_copy(&function(0x0150_E828)?);
                let enumerate: unsafe extern "C" fn(*mut [*mut c_void; 64], *mut c_uint) -> c_int =
                    std::mem::transmute_copy(&function(0xE5AC_921F)?);
                let get_clocks: unsafe extern "C" fn(*mut c_void, *mut NvapiClocks) -> c_int =
                    std::mem::transmute_copy(&function(0xDCB6_16C3)?);
                if initialize() != 0 {
                    return None;
                }
                let mut handles = [std::ptr::null_mut::<c_void>(); 64];
                let mut count: c_uint = 0;
                if enumerate(&mut handles, &mut count) != 0 || count == 0 || handles[0].is_null() {
                    return None;
                }
                let found = Nvapi { gpu: handles[0], get_clocks };
                // only kept if it answers (the struct version is tried newest first)
                found.graphics_clock().map(|_| found)
            }
        }

        /// The graphics clock in MHz.
        fn graphics_clock(&self) -> Option<f32> {
            for version in [3u32, 2, 1] {
                let mut clocks = NvapiClocks {
                    version: 264 | (version << 16),
                    clock_type: 0,
                    domains: [[0; 2]; 32],
                };
                // SAFETY: the struct is the size NVAPI expects (264 bytes) and outlives the call
                if unsafe { (self.get_clocks)(self.gpu, &mut clocks) } == 0 {
                    let [flags, khz] = clocks.domains[0]; // domain 0 is the graphics clock
                    return (flags & 1 == 1 && khz > 0).then(|| khz as f32 / 1000.0);
                }
            }
            None
        }
    }

    // ---------------------------------------------------------------- NVML

    type NvmlDevice = *mut c_void;

    #[repr(C)]
    struct NvmlUtilization {
        gpu: c_uint,
        memory: c_uint,
    }

    /// `nvmlFieldValue_t` (40 bytes) with spare room, so a bigger struct can never overrun it.
    #[repr(C)]
    struct NvmlFieldValue {
        field_id: c_uint,
        scope_id: c_uint,
        timestamp: i64,
        latency_usec: i64,
        value_type: c_int,
        nvml_return: c_int,
        value: [u8; 8],
        _spare: [u8; 24],
    }

    /// `NVML_FI_DEV_MEMORY_TEMP`
    const NVML_FI_DEV_MEMORY_TEMP: c_uint = 82;

    struct Nvml {
        lib: *mut c_void,
        device: NvmlDevice,
        shutdown: unsafe extern "C" fn() -> c_int,
        get_temperature: unsafe extern "C" fn(NvmlDevice, c_uint, *mut c_uint) -> c_int,
        get_power: unsafe extern "C" fn(NvmlDevice, *mut c_uint) -> c_int,
        get_clock: unsafe extern "C" fn(NvmlDevice, c_uint, *mut c_uint) -> c_int,
        get_utilization: unsafe extern "C" fn(NvmlDevice, *mut NvmlUtilization) -> c_int,
        get_fields: Option<unsafe extern "C" fn(NvmlDevice, c_int, *mut NvmlFieldValue) -> c_int>,
    }

    /// Looks up an exported function; `name` must end with a NUL byte.
    unsafe fn symbol<T: Copy>(lib: *mut c_void, name: &[u8]) -> Option<T> {
        let p = GetProcAddress(lib, name.as_ptr() as *const c_char);
        if p.is_null() {
            None
        } else {
            Some(std::mem::transmute_copy::<*mut c_void, T>(&p))
        }
    }

    impl Nvml {
        fn open() -> Option<Nvml> {
            unsafe {
                let mut lib = LoadLibraryA(b"nvml.dll\0".as_ptr() as *const c_char);
                if lib.is_null() {
                    lib = LoadLibraryA(
                        b"C:\\Program Files\\NVIDIA Corporation\\NVSMI\\nvml.dll\0".as_ptr()
                            as *const c_char,
                    );
                }
                if lib.is_null() {
                    return None;
                }
                let opened = Nvml::bind(lib);
                if opened.is_none() {
                    FreeLibrary(lib);
                }
                opened
            }
        }

        unsafe fn bind(lib: *mut c_void) -> Option<Nvml> {
            let init: unsafe extern "C" fn() -> c_int = symbol(lib, b"nvmlInit_v2\0")?;
            let shutdown: unsafe extern "C" fn() -> c_int = symbol(lib, b"nvmlShutdown\0")?;
            if init() != 0 {
                return None;
            }
            let found = (|| {
                let handle_by_index: unsafe extern "C" fn(c_uint, *mut NvmlDevice) -> c_int =
                    symbol(lib, b"nvmlDeviceGetHandleByIndex_v2\0")?;
                let mut device: NvmlDevice = std::ptr::null_mut();
                if handle_by_index(0, &mut device) != 0 || device.is_null() {
                    return None;
                }
                Some(Nvml {
                    lib,
                    device,
                    shutdown,
                    get_temperature: symbol(lib, b"nvmlDeviceGetTemperature\0")?,
                    get_power: symbol(lib, b"nvmlDeviceGetPowerUsage\0")?,
                    get_clock: symbol(lib, b"nvmlDeviceGetClockInfo\0")?,
                    get_utilization: symbol(lib, b"nvmlDeviceGetUtilizationRates\0")?,
                    get_fields: symbol(lib, b"nvmlDeviceGetFieldValues\0"),
                })
            })();
            if found.is_none() {
                shutdown();
            }
            found
        }

        fn read(&self, s: &mut Sample) {
            unsafe {
                let mut v: c_uint = 0;
                if (self.get_temperature)(self.device, 0, &mut v) == 0 {
                    s.gpu_temp = Some(v as f32);
                }
                if (self.get_power)(self.device, &mut v) == 0 {
                    s.gpu_power = Some(v as f32 / 1000.0); // milliwatts
                }
                if (self.get_clock)(self.device, 0, &mut v) == 0 {
                    s.gpu_clock = Some(v as f32); // graphics clock, MHz
                }
                let mut util = NvmlUtilization { gpu: 0, memory: 0 };
                if (self.get_utilization)(self.device, &mut util) == 0 {
                    s.gpu_load = Some(util.gpu as f32);
                }
                if let Some(get_fields) = self.get_fields {
                    let mut field = NvmlFieldValue {
                        field_id: NVML_FI_DEV_MEMORY_TEMP,
                        scope_id: 0,
                        timestamp: 0,
                        latency_usec: 0,
                        value_type: 0,
                        nvml_return: -1,
                        value: [0; 8],
                        _spare: [0; 24],
                    };
                    if get_fields(self.device, 1, &mut field) == 0 && field.nvml_return == 0 {
                        s.gpu_mem_temp = field_number(&field).filter(|t| *t > 0.0 && *t < 200.0);
                    }
                }
            }
        }
    }

    /// The value of a field as a number, whatever type NVML used for it.
    fn field_number(field: &NvmlFieldValue) -> Option<f32> {
        let b = field.value;
        let four = [b[0], b[1], b[2], b[3]];
        match field.value_type {
            0 => Some(f64::from_ne_bytes(b) as f32),        // double
            1 | 2 => Some(u32::from_ne_bytes(four) as f32), // unsigned int / long (Windows)
            3 => Some(u64::from_ne_bytes(b) as f32),        // unsigned long long
            4 => Some(i64::from_ne_bytes(b) as f32),        // signed long long
            5 => Some(i32::from_ne_bytes(four) as f32),     // signed int
            _ => None,
        }
    }

    impl Drop for Nvml {
        fn drop(&mut self) {
            unsafe {
                (self.shutdown)();
                FreeLibrary(self.lib);
            }
        }
    }

    // ----------------------------------------------------------------- PDH

    /// `PDH_FMT_COUNTERVALUE`
    #[repr(C)]
    struct FmtValue {
        status: u32,
        _pad: u32,
        value: f64,
    }

    /// `PDH_FMT_COUNTERVALUE_ITEM_W`
    #[repr(C)]
    struct FmtItem {
        name: *mut u16,
        value: FmtValue,
    }

    const PDH_FMT_DOUBLE: u32 = 0x0000_0200;
    const PDH_FMT_NOCAP100: u32 = 0x0000_8000;
    const PDH_MORE_DATA: u32 = 0x8000_07D2;

    struct Pdh {
        query: *mut c_void,
        cpu_power: Option<*mut c_void>,
        zones: Option<*mut c_void>,
        cpu_clock: Option<*mut c_void>,
        cpu_perf: Option<*mut c_void>,
        cpu_load: Option<*mut c_void>,
    }

    impl Pdh {
        fn open() -> Option<Pdh> {
            unsafe {
                let mut query: *mut c_void = std::ptr::null_mut();
                if PdhOpenQueryW(std::ptr::null(), 0, &mut query) != 0 || query.is_null() {
                    return None;
                }
                let add = |path: &str| -> Option<*mut c_void> {
                    let mut counter: *mut c_void = std::ptr::null_mut();
                    let path = wide(path);
                    (PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) == 0
                        && !counter.is_null())
                    .then_some(counter)
                };
                let pdh = Pdh {
                    query,
                    cpu_power: add("\\Energy Meter(RAPL_Package0_PKG)\\Power"),
                    zones: add("\\Thermal Zone Information(*)\\Temperature"),
                    cpu_clock: add("\\Processor Information(_Total)\\Processor Frequency"),
                    cpu_perf: add("\\Processor Information(_Total)\\% Processor Performance"),
                    cpu_load: add("\\Processor Information(_Total)\\% Processor Utility"),
                };
                // The first collection only sets the baseline for the rate counters.
                PdhCollectQueryData(query);
                Some(pdh)
            }
        }

        fn value(counter: *mut c_void) -> Option<f64> {
            unsafe {
                let mut v = FmtValue { status: 0, _pad: 0, value: 0.0 };
                let r = PdhGetFormattedCounterValue(
                    counter,
                    PDH_FMT_DOUBLE | PDH_FMT_NOCAP100,
                    std::ptr::null_mut(),
                    &mut v,
                );
                (r == 0 && (v.status == 0 || v.status == 1) && v.value.is_finite())
                    .then_some(v.value)
            }
        }

        /// The highest value over all instances of a wildcard counter.
        fn highest(counter: *mut c_void) -> Option<f64> {
            unsafe {
                let format = PDH_FMT_DOUBLE | PDH_FMT_NOCAP100;
                let (mut size, mut count) = (0u32, 0u32);
                let r = PdhGetFormattedCounterArrayW(
                    counter,
                    format,
                    &mut size,
                    &mut count,
                    std::ptr::null_mut(),
                );
                if r != PDH_MORE_DATA || size == 0 {
                    return None;
                }
                // u64 elements keep the buffer 8-byte aligned for the items
                let mut buf = vec![0u64; size as usize / 8 + 2];
                let r = PdhGetFormattedCounterArrayW(
                    counter,
                    format,
                    &mut size,
                    &mut count,
                    buf.as_mut_ptr() as *mut c_void,
                );
                if r != 0 {
                    return None;
                }
                let items = buf.as_ptr() as *const FmtItem;
                let mut best: Option<f64> = None;
                for i in 0..count as usize {
                    let item = &*items.add(i);
                    let v = item.value.value;
                    if (item.value.status == 0 || item.value.status == 1) && v.is_finite() {
                        best = Some(best.map_or(v, |b: f64| b.max(v)));
                    }
                }
                best
            }
        }

        fn read(&self, s: &mut Sample) {
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return;
                }
            }
            if let Some(v) = self.cpu_power.and_then(Pdh::value) {
                // Documented in milliwatts; a plain-watt value (small) is taken as it is.
                s.cpu_power = Some(if v > 1000.0 { (v / 1000.0) as f32 } else { v as f32 });
            }
            if let Some(kelvin) = self.zones.and_then(Pdh::highest) {
                if (250.0..400.0).contains(&kelvin) {
                    s.cpu_zone_temp = Some((kelvin - 273.15) as f32);
                }
            }
            // "Processor Frequency" is the nominal clock; Task Manager's live speed is that times
            // "% Processor Performance" / 100.
            let nominal = self.cpu_clock.and_then(Pdh::value);
            let performance = self.cpu_perf.and_then(Pdh::value);
            s.cpu_clock = match (nominal, performance) {
                (Some(mhz), Some(pct)) if pct > 0.0 => Some((mhz * pct / 100.0) as f32),
                (Some(mhz), _) => Some(mhz as f32),
                _ => None,
            }
            .filter(|mhz| *mhz > 100.0 && *mhz < 9000.0);
            s.cpu_load = self.cpu_load.and_then(Pdh::value).map(|v| v as f32);
        }
    }

    impl Drop for Pdh {
        fn drop(&mut self) {
            unsafe {
                PdhCloseQuery(self.query);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_tracks_now_average_and_maximum() {
        let mut s = Stat::default();
        assert_eq!((s.now, s.avg, s.max), (None, None, None));
        s.push(Some(40.0));
        s.push(Some(60.0));
        s.push(Some(50.0));
        assert_eq!(s.now, Some(50.0));
        assert_eq!(s.avg, Some(50.0));
        assert_eq!(s.max, Some(60.0));
        // a missing reading clears "now" but keeps the history
        s.push(None);
        assert_eq!((s.now, s.avg, s.max), (None, Some(50.0), Some(60.0)));
        // garbage is ignored
        s.push(Some(f32::NAN));
        assert_eq!(s.avg, Some(50.0));
        s.reset();
        assert_eq!(s, Stat::default());
    }

    #[test]
    fn readings_count_samples_and_reset() {
        let mut r = Readings::default();
        let sample = Sample { gpu_temp: Some(70.0), cpu_power: Some(30.0), ..Sample::default() };
        r.add(&sample);
        r.add(&Sample { gpu_temp: Some(80.0), ..Sample::default() });
        assert_eq!(r.samples, 2);
        assert_eq!(r.gpu_temp.avg, Some(75.0));
        assert_eq!(r.gpu_temp.max, Some(80.0));
        assert_eq!(r.cpu_power.now, None);
        assert_eq!(r.cpu_power.max, Some(30.0));
        assert_eq!(r.gpu_mem_temp.max, None);
        r.reset();
        assert_eq!(r.samples, 0);
    }

    #[test]
    fn reset_keeps_the_source_states() {
        use crate::sensor_ext::{Link, HWINFO};
        let mut r = Readings::default();
        r.sources[HWINFO] = SourceState { link: Link::Connected, detail: "" };
        r.add(&Sample { cpu_temp: Some(70.0), ..Sample::default() });
        r.reset();
        assert_eq!(r.samples, 0);
        assert_eq!(r.sources[HWINFO].link, Link::Connected);
        assert!(!r.cpu_temp.seen());
    }

    #[test]
    fn history_fills_while_running_and_logging_follows_the_switch() {
        let handle = Handle::new();
        assert_eq!(handle.log_info(), LogInfo::default());
        assert!(handle.with_history(|h| h.is_empty()));
        handle.set_logging(true);
        handle.start(SourceConfig::default());
        std::thread::sleep(Duration::from_millis(300));
        assert!(handle.with_history(|h| h.len()) >= 1, "a point per round");
        let info = handle.log_info();
        let path = info.path.clone().expect("log file is open while logging");
        assert!(path.exists());
        handle.set_logging(false);
        std::thread::sleep(Duration::from_millis(1300));
        assert_eq!(handle.log_info().path, None, "closed when logging is switched off");
        handle.stop();
        std::thread::sleep(Duration::from_millis(200));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn interval_is_limited_and_changes_while_running() {
        assert_eq!(clamp_interval(0), MIN_INTERVAL_MS);
        assert_eq!(clamp_interval(1000), 1000);
        assert_eq!(clamp_interval(1_000_000), MAX_INTERVAL_MS);
        let handle = Handle::new();
        assert_eq!(interval(&handle.shared), Duration::from_millis(DEFAULT_INTERVAL_MS));
        handle.set_interval(250);
        handle.start(SourceConfig::default());
        std::thread::sleep(Duration::from_millis(1300));
        let fast = handle.snapshot().samples;
        assert!(fast >= 4, "250 ms rounds, got {}", fast);
        // slowing down applies to the running sampler
        handle.set_interval(10_000);
        std::thread::sleep(Duration::from_millis(600));
        let before = handle.snapshot().samples;
        std::thread::sleep(Duration::from_millis(1200));
        assert!(handle.snapshot().samples <= before + 1);
        handle.stop();
        std::thread::sleep(Duration::from_millis(200));
    }

    #[test]
    fn graphs_can_be_switched_off() {
        let handle = Handle::new();
        handle.set_interval(250);
        handle.start(SourceConfig::default());
        std::thread::sleep(Duration::from_millis(700));
        assert!(handle.with_history(|h| h.len()) >= 1);
        handle.set_history(false);
        assert!(handle.with_history(|h| h.is_empty()), "dropped when switched off");
        std::thread::sleep(Duration::from_millis(700));
        assert!(handle.with_history(|h| h.is_empty()), "nothing recorded while off");
        handle.set_history(true);
        std::thread::sleep(Duration::from_millis(700));
        assert!(handle.with_history(|h| h.len()) >= 1, "recording again");
        handle.stop();
        std::thread::sleep(Duration::from_millis(200));
    }

    #[test]
    fn nothing_runs_until_started_and_stop_ends_it() {
        let handle = Handle::new();
        assert!(!handle.is_on());
        assert_eq!(handle.snapshot().samples, 0);
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(handle.snapshot().samples, 0, "no reading before start");

        handle.start(SourceConfig::default());
        assert!(handle.is_on());
        std::thread::sleep(Duration::from_millis(250));
        assert!(handle.snapshot().samples >= 1, "first reading is taken right away");
        handle.stop();
        assert!(!handle.is_on());
        std::thread::sleep(Duration::from_millis(200));
        let after_stop = handle.snapshot().samples;
        std::thread::sleep(Duration::from_millis(1300));
        assert_eq!(handle.snapshot().samples, after_stop, "no reading after stop");

        // starting again begins with fresh statistics
        handle.start(SourceConfig::default());
        std::thread::sleep(Duration::from_millis(250));
        assert!(handle.snapshot().samples <= 2);
        handle.stop();
    }
}
