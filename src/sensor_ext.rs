//! External sensor sources for the Sensors tab: programs that already read the CPU's model
//! specific registers (so R-Helper does not need a kernel driver) and share the numbers.
//!
//! * HWiNFO: its "Shared Memory Support" feed (HWiNFO > Sensors > Settings).
//! * LibreHardwareMonitor: its remote web server (Options > Remote Web Server), `/data.json`.
//! * Log file: the last line of a CSV log, as written by ThrottleStop, Intel XTU or HWiNFO. Those
//!   programs have no live interface, so this is the best-effort route: columns are recognized by
//!   their names.
//!
//! * PawnIO: the signed PawnIO driver, if installed, lets R-Helper read the CPU's own registers
//!   (package temperature, PL1 / PL2, package energy) directly, without any other program running.
//!
//! Any number of sources can be switched on at once. For each reading the user picks which source
//! it comes from, or leaves it on "Auto" (the first source that has it: PawnIO, HWiNFO,
//! LibreHardwareMonitor, log file, then the built-in readings). `merge` does the choosing.
use crate::sensors::Sample;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Source numbers (also the order of `SOURCE_NAMES`).
pub const BUILT_IN: usize = 0;
pub const HWINFO: usize = 1;
pub const LHM: usize = 2;
pub const LOG: usize = 3;
pub const PAWN: usize = 4;
pub const SOURCE_COUNT: usize = 5;

/// Names shown in Options.
pub const SOURCE_NAMES: [&str; SOURCE_COUNT] =
    ["Built-in", "HWiNFO", "LibreHardwareMonitor", "Log file", "PawnIO"];

/// Longer description of each source, for the toggles in Options.
pub const SOURCE_TITLES: [&str; SOURCE_COUNT] = [
    "Built-in (Windows + NVIDIA)",
    "HWiNFO (shared memory)",
    "LibreHardwareMonitor (web server)",
    "Log file (ThrottleStop / XTU / HWiNFO CSV)",
    "PawnIO driver (direct CPU registers)",
];

/// The order "Auto" tries the sources in.
const AUTO_ORDER: [usize; SOURCE_COUNT] = [PAWN, HWINFO, LHM, LOG, BUILT_IN];

/// "Pick" value meaning "choose for me".
pub const AUTO: u8 = 255;

pub const DEFAULT_LHM_ADDR: &str = "127.0.0.1:8085";

/// The readings whose source can be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    CpuTemp,
    CoreTemp,
    Pl1,
    Pl2,
    CpuPower,
    GpuTemp,
    GpuMemTemp,
    GpuPower,
}

pub const READING_COUNT: usize = 8;

impl Reading {
    pub const ALL: [Reading; READING_COUNT] = [
        Reading::CpuTemp,
        Reading::CoreTemp,
        Reading::Pl1,
        Reading::Pl2,
        Reading::CpuPower,
        Reading::GpuTemp,
        Reading::GpuMemTemp,
        Reading::GpuPower,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            Reading::CpuTemp => "CPU package temperature",
            Reading::CoreTemp => "CPU core temperature (hottest core)",
            Reading::Pl1 => "CPU PL1 (long term limit)",
            Reading::Pl2 => "CPU PL2 (short term limit)",
            Reading::CpuPower => "CPU package power",
            Reading::GpuTemp => "GPU temperature",
            Reading::GpuMemTemp => "GPU memory junction",
            Reading::GpuPower => "GPU power",
        }
    }

    /// Name in `config.txt`.
    pub fn key(self) -> &'static str {
        match self {
            Reading::CpuTemp => "cpu_temp",
            Reading::CoreTemp => "core_temp",
            Reading::Pl1 => "pl1",
            Reading::Pl2 => "pl2",
            Reading::CpuPower => "cpu_power",
            Reading::GpuTemp => "gpu_temp",
            Reading::GpuMemTemp => "gpu_mem_temp",
            Reading::GpuPower => "gpu_power",
        }
    }

    pub fn from_key(key: &str) -> Option<Reading> {
        Reading::ALL.into_iter().find(|r| r.key() == key)
    }

    /// Whether `source` is able to provide this reading at all.
    pub fn provided_by(self, source: usize) -> bool {
        match source {
            BUILT_IN => matches!(
                self,
                Reading::CpuPower | Reading::GpuTemp | Reading::GpuMemTemp | Reading::GpuPower
            ),
            LHM => !matches!(self, Reading::Pl1 | Reading::Pl2),
            PAWN => matches!(
                self,
                Reading::CpuTemp
                    | Reading::CoreTemp
                    | Reading::Pl1
                    | Reading::Pl2
                    | Reading::CpuPower
                    | Reading::GpuMemTemp
            ),
            HWINFO | LOG => true,
            _ => false,
        }
    }
}

fn value_of(sample: &Sample, reading: Reading) -> Option<f32> {
    match reading {
        Reading::CpuTemp => sample.cpu_temp,
        Reading::CoreTemp => sample.core_temp,
        Reading::Pl1 => sample.pl1,
        Reading::Pl2 => sample.pl2,
        Reading::CpuPower => sample.cpu_power,
        Reading::GpuTemp => sample.gpu_temp,
        Reading::GpuMemTemp => sample.gpu_mem_temp,
        Reading::GpuPower => sample.gpu_power,
    }
}

fn set_value(sample: &mut Sample, reading: Reading, value: Option<f32>) {
    match reading {
        Reading::CpuTemp => sample.cpu_temp = value,
        Reading::CoreTemp => sample.core_temp = value,
        Reading::Pl1 => sample.pl1 = value,
        Reading::Pl2 => sample.pl2 = value,
        Reading::CpuPower => sample.cpu_power = value,
        Reading::GpuTemp => sample.gpu_temp = value,
        Reading::GpuMemTemp => sample.gpu_mem_temp = value,
        Reading::GpuPower => sample.gpu_power = value,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceConfig {
    /// Which sources are switched on (the built-in one always is).
    pub enabled: [bool; SOURCE_COUNT],
    /// Per reading (index = `Reading::index`): `AUTO` or a source number.
    pub picks: [u8; READING_COUNT],
    /// `host:port` of the LibreHardwareMonitor web server.
    pub lhm_addr: String,
    /// Path of the CSV log file.
    pub log_path: String,
    /// Path of PawnIO's `IntelMSR.bin` module (empty = look in the usual places).
    pub module_path: String,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            enabled: [false; SOURCE_COUNT],
            picks: [AUTO; READING_COUNT],
            lhm_addr: DEFAULT_LHM_ADDR.to_string(),
            log_path: String::new(),
            module_path: String::new(),
        }
    }
}

impl SourceConfig {
    pub fn is_on(&self, source: usize) -> bool {
        source == BUILT_IN || self.enabled.get(source).copied().unwrap_or(false)
    }

    /// The source chosen for `reading`; `None` = Auto. A pick that is no longer usable (the
    /// source was switched off, or cannot provide the reading) counts as Auto.
    pub fn pick(&self, reading: Reading) -> Option<usize> {
        let source = usize::from(self.picks[reading.index()]);
        (source < SOURCE_COUNT && self.is_on(source) && reading.provided_by(source))
            .then_some(source)
    }
}

/// Chooses each reading's value from the sources' samples. `external[BUILT_IN]` is ignored: the
/// built-in sample is passed on its own.
pub fn merge(
    builtin: &Sample,
    external: &[Option<Sample>; SOURCE_COUNT],
    cfg: &SourceConfig,
) -> Sample {
    let mut out = *builtin;
    for reading in Reading::ALL {
        let from = |source: usize| -> Option<f32> {
            if source == BUILT_IN {
                value_of(builtin, reading)
            } else {
                external[source].as_ref().and_then(|s| value_of(s, reading))
            }
        };
        let value = match cfg.pick(reading) {
            // an explicit choice is honoured even when that source has nothing right now
            Some(source) => from(source),
            None => AUTO_ORDER
                .iter()
                .filter(|source| cfg.is_on(**source))
                .find_map(|source| from(*source)),
        };
        set_value(&mut out, reading, value);
    }
    // readings only one place provides (they are not selectable): from whichever source has them
    for sample in external.iter().flatten() {
        out.cpu_clock_top = out.cpu_clock_top.or(sample.cpu_clock_top);
        out.cpu_clock_p = out.cpu_clock_p.or(sample.cpu_clock_p);
        out.cpu_clock_e = out.cpu_clock_e.or(sample.cpu_clock_e);
    }
    out
}

/// Whether a source could be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Link {
    /// Not switched on (or nothing read yet).
    #[default]
    None,
    Connected,
    NotFound,
}

/// State of one source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceState {
    pub link: Link,
    /// Why it gave nothing (empty while all is well).
    pub detail: &'static str,
}

// ------------------------------------------------------------------ matching

#[derive(Default)]
struct Slot {
    value: Option<f32>,
    score: u8,
}

impl Slot {
    /// Keeps the value with the highest score (the first one on a tie).
    fn offer(&mut self, value: f64, score: u8) {
        if value.is_finite() && (self.value.is_none() || score > self.score) {
            self.value = Some(value as f32);
            self.score = score;
        }
    }
}

#[derive(Default)]
struct Picker {
    cpu_temp: Slot,
    core_temp: Slot,
    /// The hottest of the individual core readings ("Core 0", "Core 1", ...).
    per_core: Option<f32>,
    cpu_power: Slot,
    pl1: Slot,
    pl2: Slot,
    gpu_temp: Slot,
    gpu_mem_temp: Slot,
    gpu_power: Slot,
}

impl Picker {
    fn into_sample(self) -> Sample {
        Sample {
            cpu_temp: self.cpu_temp.value,
            // "Core Max" if the source has one, otherwise the hottest single core
            core_temp: self.core_temp.value.or(self.per_core),
            cpu_power: self.cpu_power.value,
            pl1: self.pl1.value,
            pl2: self.pl2.value,
            gpu_temp: self.gpu_temp.value,
            gpu_mem_temp: self.gpu_mem_temp.value,
            gpu_power: self.gpu_power.value,
            ..Sample::default()
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Temp,
    Power,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hw {
    Cpu,
    Gpu,
    Unknown,
}

/// Files one reading into the right slot. `label` must be lower case. The rules follow the labels
/// HWiNFO and LibreHardwareMonitor use.
fn classify(p: &mut Picker, label: &str, kind: Kind, hw: Hw, value: f64) {
    // power limits are recognized by name whatever type the source gives them
    if label.contains("pl1") && label.contains("limit") {
        p.pl1.offer(value, if label.contains("static") { 3 } else { 2 });
        return;
    }
    if label.contains("pl2") && label.contains("limit") {
        p.pl2.offer(value, if label.contains("static") { 3 } else { 2 });
        return;
    }
    let gpu = hw == Hw::Gpu || label.starts_with("gpu");
    match kind {
        Kind::Temp if label.contains("memory junction") => p.gpu_mem_temp.offer(value, 2),
        Kind::Temp if gpu => {
            let plain = !label.contains("hot")
                && !label.contains("junction")
                && !label.contains("memory")
                && !label.contains("limit");
            if plain && (label.contains("gpu temperature") || label == "gpu core") {
                p.gpu_temp.offer(value, 2);
            } else if plain && label.contains("gpu") {
                p.gpu_temp.offer(value, 1);
            }
        }
        Kind::Temp => {
            let unwanted = ["distance", "limit", "tjmax", "throttl", "vid"];
            if label == "cpu package" || (label.contains("cpu package") && !label.contains("power"))
            {
                p.cpu_temp.offer(value, 3);
            } else if label == "core max" {
                p.cpu_temp.offer(value, 2);
                p.core_temp.offer(value, 3);
            } else if label == "core average" {
                p.cpu_temp.offer(value, 1);
            } else if (label.starts_with("core ") || label.starts_with("cpu core"))
                && label.chars().any(|c| c.is_ascii_digit())
                && !label.contains("max")
                && !label.contains("average")
                && !unwanted.iter().any(|w| label.contains(w))
                && value.is_finite()
            {
                // an individual core: the hottest one stands for "core temperature"
                let value = value as f32;
                p.per_core = Some(p.per_core.map_or(value, |old| old.max(value)));
            }
        }
        Kind::Power if gpu => {
            if label == "gpu power" || label.contains("gpu card power") || label == "gpu package" {
                p.gpu_power.offer(value, 2);
            }
        }
        Kind::Power => {
            if label == "cpu package power" || label == "cpu package" {
                p.cpu_power.offer(value, 3);
            }
        }
        Kind::Other => {}
    }
}

// -------------------------------------------------------------------- HWiNFO

const HWINFO_SIGNATURE: u32 = 0x5369_5748; // "HWiS"
const HWINFO_HEADER_LEN: usize = 44;
/// `HWiNFO_SENSORS_READING_ELEMENT`: type, sensor index, id, two 128 byte labels, a 16 byte unit
/// and four doubles (all packed).
const HWINFO_READING_MIN: usize = 316;

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?))
}

fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// How many bytes of the shared memory hold the header, the sensors and the readings.
fn hwinfo_total_len(header: &[u8]) -> Option<usize> {
    if u32_at(header, 0)? != HWINFO_SIGNATURE {
        return None;
    }
    let part = |off: usize, size: usize, num: usize| off.checked_add(size.checked_mul(num)?);
    let sensors = part(
        u32_at(header, 20)? as usize,
        u32_at(header, 24)? as usize,
        u32_at(header, 28)? as usize,
    )?;
    let readings = part(
        u32_at(header, 32)? as usize,
        u32_at(header, 36)? as usize,
        u32_at(header, 40)? as usize,
    )?;
    Some(sensors.max(readings).max(HWINFO_HEADER_LEN))
}

/// Reads the values out of a copy of HWiNFO's shared memory.
pub fn hwinfo_parse(buf: &[u8]) -> Option<Sample> {
    hwinfo_total_len(buf)?;
    let sensor_off = u32_at(buf, 20)? as usize;
    let sensor_size = u32_at(buf, 24)? as usize;
    let sensor_num = u32_at(buf, 28)? as usize;
    let reading_off = u32_at(buf, 32)? as usize;
    let reading_size = u32_at(buf, 36)? as usize;
    let reading_num = u32_at(buf, 40)? as usize;
    if reading_size < HWINFO_READING_MIN || sensor_size < 8 + 128 {
        return None;
    }

    // sensor index -> (lower case) name, to tell the CPU's readings from the GPU's
    let mut sensor_hw = Vec::with_capacity(sensor_num.min(512));
    for i in 0..sensor_num.min(512) {
        let at = sensor_off + i * sensor_size;
        let name = buf.get(at + 8..at + 8 + 128).map(c_string).unwrap_or_default().to_lowercase();
        sensor_hw.push(if name.starts_with("gpu") {
            Hw::Gpu
        } else if name.starts_with("cpu") {
            Hw::Cpu
        } else {
            Hw::Unknown
        });
    }

    let mut picker = Picker::default();
    for i in 0..reading_num.min(4096) {
        let at = reading_off + i * reading_size;
        let Some(element) = buf.get(at..at + reading_size) else {
            break;
        };
        let (Some(kind), Some(sensor), Some(value)) =
            (u32_at(element, 0), u32_at(element, 4), f64_at(element, 284))
        else {
            continue;
        };
        let label = c_string(&element[12..140]).to_lowercase();
        let kind = match kind {
            1 => Kind::Temp,
            5 => Kind::Power,
            _ => Kind::Other,
        };
        let hw = sensor_hw.get(sensor as usize).copied().unwrap_or(Hw::Unknown);
        classify(&mut picker, &label, kind, hw, value);
    }
    Some(picker.into_sample())
}

const HWINFO_NOT_FOUND: &str = "HWiNFO's shared memory was not found: HWiNFO must be running \
     with its Sensors window open and Shared Memory Support switched on";
const HWINFO_DENIED: &str = "HWiNFO's shared memory exists but Windows denied access: start \
     R-Helper as administrator (HWiNFO is probably running as administrator)";
const HWINFO_INACTIVE: &str = "HWiNFO's shared memory is there but not active (HWiNFO may have \
     stopped sharing, e.g. a time limit of the free version): restart HWiNFO's sensors";

/// A copy of HWiNFO's shared memory, if HWiNFO is running with Shared Memory Support on.
#[cfg(windows)]
fn hwinfo_read() -> Result<Vec<u8>, &'static str> {
    use std::ffi::c_void;
    use std::os::raw::c_char;

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenFileMappingA(access: u32, inherit: i32, name: *const c_char) -> *mut c_void;
        fn MapViewOfFile(
            mapping: *mut c_void,
            access: u32,
            offset_high: u32,
            offset_low: u32,
            bytes: usize,
        ) -> *mut c_void;
        fn UnmapViewOfFile(base: *const c_void) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
        fn GetLastError() -> u32;
    }
    const FILE_MAP_READ: u32 = 4;
    const ERROR_ACCESS_DENIED: u32 = 5;
    const MAX_LEN: usize = 16 * 1024 * 1024;

    unsafe {
        let mapping = OpenFileMappingA(
            FILE_MAP_READ,
            0,
            b"Global\\HWiNFO_SENS_SM2\0".as_ptr() as *const c_char,
        );
        if mapping.is_null() {
            return Err(if GetLastError() == ERROR_ACCESS_DENIED {
                HWINFO_DENIED
            } else {
                HWINFO_NOT_FOUND
            });
        }
        let view = MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, 0);
        let copy = if view.is_null() {
            Err(HWINFO_DENIED)
        } else {
            let header = std::slice::from_raw_parts(view as *const u8, HWINFO_HEADER_LEN);
            let total = hwinfo_total_len(header).filter(|n| *n <= MAX_LEN);
            let copy = total.map(|n| std::slice::from_raw_parts(view as *const u8, n).to_vec());
            UnmapViewOfFile(view);
            copy.ok_or(HWINFO_INACTIVE)
        };
        CloseHandle(mapping);
        copy
    }
}

#[cfg(not(windows))]
fn hwinfo_read() -> Result<Vec<u8>, &'static str> {
    Err(HWINFO_NOT_FOUND)
}

/// Every HWiNFO sensor and reading (label, type, value) as text, to see what a machine offers.
pub fn hwinfo_dump(buf: &[u8]) -> String {
    let mut out = String::new();
    let (Some(sensor_off), Some(sensor_size), Some(sensor_num)) =
        (u32_at(buf, 20), u32_at(buf, 24), u32_at(buf, 28))
    else {
        return out;
    };
    let (Some(reading_off), Some(reading_size), Some(reading_num)) =
        (u32_at(buf, 32), u32_at(buf, 36), u32_at(buf, 40))
    else {
        return out;
    };
    out.push_str("# HWiNFO shared memory as seen by R-Helper\n[sensors]\n");
    for i in 0..(sensor_num as usize).min(512) {
        let at = sensor_off as usize + i * sensor_size as usize;
        if let Some(name) = buf.get(at + 8..at + 8 + 128) {
            out.push_str(&format!("{}: {}\n", i, c_string(name)));
        }
    }
    out.push_str("[readings] sensor | type | label | value\n");
    for i in 0..(reading_num as usize).min(4096) {
        let at = reading_off as usize + i * reading_size as usize;
        let Some(element) = buf.get(at..at + reading_size as usize) else {
            break;
        };
        if element.len() < HWINFO_READING_MIN {
            break;
        }
        if let (Some(kind), Some(sensor), Some(value)) =
            (u32_at(element, 0), u32_at(element, 4), f64_at(element, 284))
        {
            out.push_str(&format!(
                "{} | {} | {} | {}\n",
                sensor,
                kind,
                c_string(&element[12..140]),
                value
            ));
        }
    }
    out
}

// ------------------------------------------------------- LibreHardwareMonitor

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

struct JsonReader<'a> {
    s: &'a [u8],
    i: usize,
    depth: usize,
}

impl<'a> JsonReader<'a> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> Option<()> {
        self.ws();
        (self.s.get(self.i) == Some(&byte)).then(|| self.i += 1)
    }

    fn value(&mut self) -> Option<Json> {
        self.ws();
        self.depth += 1;
        if self.depth > 64 {
            return None;
        }
        let out = match *self.s.get(self.i)? {
            b'{' => {
                self.i += 1;
                let mut fields = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                } else {
                    loop {
                        self.ws();
                        let Json::Str(key) = self.value()? else {
                            return None;
                        };
                        self.eat(b':')?;
                        fields.push((key, self.value()?));
                        self.ws();
                        match self.s.get(self.i)? {
                            b',' => self.i += 1,
                            b'}' => {
                                self.i += 1;
                                break;
                            }
                            _ => return None,
                        }
                    }
                }
                Json::Obj(fields)
            }
            b'[' => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                } else {
                    loop {
                        items.push(self.value()?);
                        self.ws();
                        match self.s.get(self.i)? {
                            b',' => self.i += 1,
                            b']' => {
                                self.i += 1;
                                break;
                            }
                            _ => return None,
                        }
                    }
                }
                Json::Arr(items)
            }
            b'"' => {
                self.i += 1;
                let mut bytes = Vec::new();
                loop {
                    let b = *self.s.get(self.i)?;
                    self.i += 1;
                    match b {
                        b'"' => break,
                        b'\\' => {
                            let e = *self.s.get(self.i)?;
                            self.i += 1;
                            match e {
                                b'n' => bytes.push(b'\n'),
                                b't' => bytes.push(b'\t'),
                                b'r' => bytes.push(b'\r'),
                                b'u' => {
                                    let hex = self.s.get(self.i..self.i + 4)?;
                                    self.i += 4;
                                    let code =
                                        u32::from_str_radix(std::str::from_utf8(hex).ok()?, 16)
                                            .ok()?;
                                    let ch = char::from_u32(code).unwrap_or('?');
                                    let mut tmp = [0u8; 4];
                                    bytes.extend_from_slice(ch.encode_utf8(&mut tmp).as_bytes());
                                }
                                other => bytes.push(other),
                            }
                        }
                        other => bytes.push(other),
                    }
                }
                Json::Str(String::from_utf8_lossy(&bytes).into_owned())
            }
            b't' if self.s[self.i..].starts_with(b"true") => {
                self.i += 4;
                Json::Bool(true)
            }
            b'f' if self.s[self.i..].starts_with(b"false") => {
                self.i += 5;
                Json::Bool(false)
            }
            b'n' if self.s[self.i..].starts_with(b"null") => {
                self.i += 4;
                Json::Null
            }
            _ => {
                let start = self.i;
                while self.i < self.s.len()
                    && matches!(self.s[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                {
                    self.i += 1;
                }
                Json::Num(std::str::from_utf8(&self.s[start..self.i]).ok()?.parse().ok()?)
            }
        };
        self.depth -= 1;
        Some(out)
    }
}

fn field<'a>(obj: &'a [(String, Json)], name: &str) -> Option<&'a Json> {
    obj.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

/// The first number in a text such as "45,5 °C" (a decimal comma is accepted).
fn number_in(text: &str) -> Option<f64> {
    let start = text.find(|c: char| c.is_ascii_digit() || c == '-')?;
    let token: String = text[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-'))
        .collect();
    token.replace(',', ".").parse().ok()
}

fn lhm_walk(node: &Json, ancestors: &mut Vec<String>, picker: &mut Picker) {
    let Json::Obj(fields) = node else {
        return;
    };
    let text = match field(fields, "Text") {
        Some(Json::Str(t)) => t.clone(),
        _ => String::new(),
    };
    let children = match field(fields, "Children") {
        Some(Json::Arr(c)) => c.as_slice(),
        _ => &[],
    };
    if children.is_empty() {
        if let Some(Json::Str(value)) = field(fields, "Value") {
            let (Some(number), false) = (number_in(value), text.is_empty()) else {
                return;
            };
            let path = ancestors.join("/").to_lowercase();
            let kind = if path.contains("temperatures") {
                Kind::Temp
            } else if path.contains("powers") {
                Kind::Power
            } else {
                Kind::Other
            };
            const GPU_WORDS: [&str; 6] = ["nvidia", "geforce", "radeon", "rtx", "gtx", "arc "];
            const CPU_WORDS: [&str; 6] = ["intel", "amd", "ryzen", "core i", "xeon", "cpu"];
            let hw = if GPU_WORDS.iter().any(|w| path.contains(w)) {
                Hw::Gpu
            } else if CPU_WORDS.iter().any(|w| path.contains(w)) {
                Hw::Cpu
            } else {
                Hw::Unknown
            };
            classify(picker, &text.to_lowercase(), kind, hw, number);
        }
        return;
    }
    ancestors.push(text);
    for child in children {
        lhm_walk(child, ancestors, picker);
    }
    ancestors.pop();
}

/// Reads the values out of LibreHardwareMonitor's `data.json`.
pub fn lhm_parse(json: &str) -> Option<Sample> {
    let mut reader = JsonReader { s: json.as_bytes(), i: 0, depth: 0 };
    let root = reader.value()?;
    let mut picker = Picker::default();
    lhm_walk(&root, &mut Vec::new(), &mut picker);
    Some(picker.into_sample())
}

fn lhm_read(addr: &str) -> Option<String> {
    let target = addr.to_socket_addrs().ok()?.next()?;
    let mut stream = TcpStream::connect_timeout(&target, Duration::from_millis(400)).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(800))).ok()?;
    stream.set_write_timeout(Some(Duration::from_millis(400))).ok()?;
    // HTTP/1.0 so the reply is neither chunked nor kept alive
    let request = format!("GET /data.json HTTP/1.0\r\nHost: {}\r\nConnection: close\r\n\r\n", addr);
    stream.write_all(request.as_bytes()).ok()?;
    let mut reply = Vec::new();
    stream.take(8 * 1024 * 1024).read_to_end(&mut reply).ok()?;
    let text = String::from_utf8_lossy(&reply).into_owned();
    let (head, body) = text.split_once("\r\n\r\n")?;
    head.lines().next()?.contains(" 200").then(|| body.to_string())
}

// ------------------------------------------------------------------ log file

fn delimiter(header: &str) -> char {
    [',', ';', '\t'].into_iter().max_by_key(|d| header.matches(*d).count()).unwrap_or(',')
}

/// Reads the values of one CSV log line, with the columns named by `header`.
pub fn csv_parse(header: &str, line: &str) -> Option<Sample> {
    let delim = delimiter(header);
    let names: Vec<String> =
        header.split(delim).map(|c| c.trim().trim_matches('"').to_lowercase()).collect();
    let values: Vec<&str> = line.split(delim).map(|c| c.trim().trim_matches('"')).collect();
    if names.len() < 2 || values.len() < 2 {
        return None;
    }
    let mut picker = Picker::default();
    for (name, value) in names.iter().zip(values.iter()) {
        let Some(number) = number_in(value) else {
            continue;
        };
        let gpu = name.contains("gpu");
        let kind = if name.contains("temp") {
            Kind::Temp
        } else if name.contains("power") || name.contains("watt") || name.contains("(w)") {
            Kind::Power
        } else {
            Kind::Other
        };
        let cpu_ish = ["cpu", "pkg", "package", "core"].iter().any(|w| name.contains(w));
        let limit = name.contains("limit") || name.contains("pl1") || name.contains("pl2");
        // power limits: PL1 is the long term limit, PL2 the short term one
        if kind == Kind::Power && !gpu && name.contains("long") && limit {
            picker.pl1.offer(number, 2);
        } else if kind == Kind::Power && !gpu && name.contains("short") && limit {
            picker.pl2.offer(number, 2);
        } else if name.contains("pl1") || name.contains("pl2") {
            classify(&mut picker, &format!("{} limit", name), kind, Hw::Cpu, number);
        } else if gpu && kind == Kind::Temp && (name.contains("junction") || name.contains("mem")) {
            picker.gpu_mem_temp.offer(number, 2);
        } else if gpu && kind == Kind::Temp && !name.contains("hot") {
            picker.gpu_temp.offer(number, 1);
        } else if gpu && kind == Kind::Power {
            picker.gpu_power.offer(number, 1);
        } else if kind == Kind::Temp && cpu_ish && !gpu {
            let package = name.contains("package") || name.contains("pkg");
            picker.cpu_temp.offer(number, if package { 3 } else { 1 });
            if !package && name.contains("core") {
                picker.core_temp.offer(number, if name.contains("max") { 2 } else { 1 });
            }
        } else if kind == Kind::Power && cpu_ish && !gpu && !limit {
            picker.cpu_power.offer(number, 2);
        }
    }
    Some(picker.into_sample())
}

/// The header (first line) and the latest (last) line of a CSV log file.
fn log_read(path: &str) -> Option<(String, String)> {
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let mut head = vec![0u8; 8192.min(len as usize)];
    file.read_exact(&mut head).ok()?;
    let head = String::from_utf8_lossy(&head).into_owned();
    let header = head.lines().next()?.to_string();

    file.seek(SeekFrom::Start(len.saturating_sub(8192))).ok()?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail).ok()?;
    let tail = String::from_utf8_lossy(&tail).into_owned();
    let last = tail.lines().rev().find(|l| !l.trim().is_empty())?.to_string();
    (last != header).then_some((header, last))
}

// ------------------------------------------------------------------- PawnIO

/// Intel model specific registers read through PawnIO's `IntelMSR` module.
const MSR_TEMPERATURE_TARGET: u32 = 0x1A2;
const MSR_PACKAGE_THERM_STATUS: u32 = 0x1B1;
/// `IA32_THERM_STATUS`: the temperature of the core the reading is made on.
const MSR_THERM_STATUS: u32 = 0x19C;
/// `IA32_PERF_STATUS`: bits 15..8 are the core's current ratio (times the 100 MHz bus clock).
const MSR_PERF_STATUS: u32 = 0x198;
const MSR_RAPL_POWER_UNIT: u32 = 0x606;
const MSR_PKG_POWER_LIMIT: u32 = 0x610;
const MSR_PKG_ENERGY_STATUS: u32 = 0x611;

/// Watts per count of the power limit fields and joules per count of the energy counter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaplUnits {
    pub power: f64,
    pub energy: f64,
}

/// `MSR_RAPL_POWER_UNIT`: power unit = 1 / 2^bits 3..0, energy unit = 1 / 2^bits 12..8.
pub fn rapl_units(raw: u64) -> RaplUnits {
    RaplUnits {
        power: 1.0 / f64::from(1u32 << (raw & 0xF)),
        energy: 1.0 / f64::from(1u32 << ((raw >> 8) & 0x1F).min(31)),
    }
}

/// `MSR_PKG_POWER_LIMIT`: PL1 = bits 14..0, PL2 = bits 46..32 (both in power units).
pub fn power_limits(raw: u64, units: RaplUnits) -> (Option<f32>, Option<f32>) {
    let watts = |field: u64| (field != 0).then(|| (field as f64 * units.power) as f32);
    (watts(raw & 0x7FFF), watts((raw >> 32) & 0x7FFF))
}

/// Package temperature: TjMax (`MSR_TEMPERATURE_TARGET` bits 23..16, 100 if the register is
/// empty) minus the digital readout (`IA32_PACKAGE_THERM_STATUS` bits 22..16, valid bit 31).
pub fn package_temp(target: Option<u64>, status: u64) -> Option<f32> {
    if status >> 31 & 1 == 0 {
        return None;
    }
    let tjmax = match target.map(|t| (t >> 16) & 0xFF) {
        Some(t) if t > 0 => t,
        _ => 100,
    };
    let readout = (status >> 16) & 0x7F;
    Some(tjmax.saturating_sub(readout) as f32)
}

/// The hottest core out of one `IA32_THERM_STATUS` value per logical processor (same layout as
/// the package register; cores whose reading is not valid are skipped).
pub fn core_temp(target: Option<u64>, statuses: &[u64]) -> Option<f32> {
    statuses
        .iter()
        .filter_map(|status| package_temp(target, *status))
        .fold(None, |best: Option<f32>, t| Some(best.map_or(t, |b| b.max(t))))
}

/// What is read from one logical processor for its clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CpuClockRead {
    /// `IA32_PERF_STATUS`.
    pub status: u64,
    /// The kind of core, from CPUID leaf 0x1A bits 31..24: 0x40 performance, 0x20 efficiency,
    /// 0 when the processor does not say (not a hybrid one).
    pub kind: u8,
}

/// The clock of the fastest core and the average of the performance and of the efficiency cores.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CoreClocks {
    pub fastest: Option<f32>,
    pub performance: Option<f32>,
    pub efficiency: Option<f32>,
}

/// The bus clock the ratios are counted in, MHz.
const BUS_MHZ: f32 = 100.0;

/// Clocks of the cores out of one reading per logical processor (the threads of one core give
/// the same value, so averaging over threads is averaging over cores).
pub fn core_clocks(reads: &[CpuClockRead]) -> CoreClocks {
    let mut fastest: Option<f32> = None;
    let (mut perf, mut eff) = ((0.0f32, 0u32), (0.0f32, 0u32));
    for read in reads {
        let ratio = ((read.status >> 8) & 0xFF) as f32;
        // a ratio of 0 (or something absurd) is a register that did not answer
        if !(4.0..=80.0).contains(&ratio) {
            continue;
        }
        let mhz = ratio * BUS_MHZ;
        fastest = Some(fastest.map_or(mhz, |f| f.max(mhz)));
        match read.kind {
            0x40 => perf = (perf.0 + mhz, perf.1 + 1),
            0x20 => eff = (eff.0 + mhz, eff.1 + 1),
            _ => {}
        }
    }
    let average = |(sum, count): (f32, u32)| (count > 0).then(|| sum / count as f32);
    CoreClocks { fastest, performance: average(perf), efficiency: average(eff) }
}

/// Average power between two reads of the 32 bit energy counter (which wraps around).
pub fn energy_power(prev: u32, now: u32, seconds: f64, units: RaplUnits) -> Option<f32> {
    if seconds < 0.2 {
        return None;
    }
    let counts = now.wrapping_sub(prev);
    Some((f64::from(counts) * units.energy / seconds) as f32)
}

/// What is remembered between two rounds of register reads.
#[derive(Default)]
struct MsrState {
    units: Option<RaplUnits>,
    target: Option<u64>,
    energy: Option<(u32, Instant)>,
}

/// One round: reads the registers through `read` (None = register not readable).
fn msr_sample(
    read: &dyn Fn(u32) -> Option<u64>,
    state: &mut MsrState,
    clock: &dyn Fn() -> Instant,
) -> Option<Sample> {
    if state.units.is_none() {
        state.units = read(MSR_RAPL_POWER_UNIT).map(rapl_units);
    }
    if state.target.is_none() {
        state.target = read(MSR_TEMPERATURE_TARGET);
    }
    let mut sample = Sample::default();
    if let Some(status) = read(MSR_PACKAGE_THERM_STATUS) {
        sample.cpu_temp = package_temp(state.target, status);
    }
    if let Some(units) = state.units {
        if let Some(raw) = read(MSR_PKG_POWER_LIMIT) {
            (sample.pl1, sample.pl2) = power_limits(raw, units);
        }
        if let Some(raw) = read(MSR_PKG_ENERGY_STATUS) {
            // the time is taken the moment the counter was read (not before the other registers), so
            // that a slow register read does not skew the watts of a short interval
            let now = clock();
            let counter = raw as u32;
            if let Some((before, when)) = state.energy {
                let seconds = now.saturating_duration_since(when).as_secs_f64();
                // keep the older reference until enough time has passed for a stable average
                if seconds >= 0.2 {
                    sample.cpu_power = energy_power(before, counter, seconds, units);
                    state.energy = Some((counter, now));
                }
            } else {
                state.energy = Some((counter, now));
            }
        }
    }
    let any =
        [sample.cpu_temp, sample.pl1, sample.pl2, sample.cpu_power].iter().any(Option::is_some);
    // the first round has no energy reference yet; temperature or limits still count
    any.then_some(sample)
}

/// Places where a PawnIO module file (`IntelMSR.bin`, `Nvidia.bin`) may be, best guess first.
/// `configured` is the path the user gave for IntelMSR.bin (a file or its folder); the other
/// modules are looked for next to it.
pub fn module_candidates(
    configured: &str,
    exe_dir: Option<PathBuf>,
    program_files: Option<PathBuf>,
    name: &str,
) -> Vec<PathBuf> {
    let mut list = Vec::new();
    let configured = configured.trim().trim_matches('"');
    if !configured.is_empty() {
        let path = PathBuf::from(configured);
        if path.extension().is_none() {
            // a folder
            list.push(path.join(name));
        } else if path.file_name().map_or(false, |n| n.eq_ignore_ascii_case(name)) {
            list.push(path);
        } else if let Some(dir) = path.parent() {
            // the path names another module: this one is in the same folder
            list.push(dir.join(name));
        }
    }
    if let Some(dir) = exe_dir {
        list.push(dir.join(name));
        list.push(dir.join("modules").join(name));
    }
    if let Some(dir) = program_files {
        let pawn = dir.join("PawnIO");
        list.push(pawn.join(name));
        list.push(pawn.join("modules").join(name));
    }
    list
}

/// Where the module file `name` is found right now (None = not found), searched the same way
/// the PawnIO source does.
pub fn find_module(configured: &str, name: &str) -> Option<PathBuf> {
    let program_files = std::env::var_os("ProgramFiles").map(PathBuf::from);
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
    module_candidates(configured, exe_dir, program_files, name).into_iter().find(|p| p.is_file())
}

/// The folder of the installed PawnIO driver (where `PawnIOLib.dll` is), if there is one.
pub fn pawnio_driver_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("ProgramFiles").map(PathBuf::from)?.join("PawnIO");
    dir.join("PawnIOLib.dll").is_file().then_some(dir)
}

/// Number of values the NVIDIA module's memory temperature call fills in.
const NVIDIA_MEMORY_OUT: usize = 50;
/// Number of values its thermal register call fills in (only written to the diagnostics file).
const NVIDIA_THERMAL_OUT: usize = 6;

/// The memory temperature out of what `ioctl_read_memory_temperatures` returned: the hottest
/// value that is a plausible temperature in degrees Celsius (zeros and counters are skipped).
pub fn nvidia_mem_temp(out: &[u64]) -> Option<f32> {
    out.iter().copied().filter(|v| (20..=125).contains(v)).max().map(|v| v as f32)
}

/// `bus`, `device`, `function` out of a PCI bus id such as `00000000:01:00.0`.
pub fn parse_bus_id(text: &str) -> Option<(u8, u8, u8)> {
    let mut parts = text.trim().trim_end_matches('\0').split(':');
    let _domain = parts.next()?;
    let bus = u8::from_str_radix(parts.next()?, 16).ok()?;
    let (device, function) = parts.next()?.split_once('.')?;
    Some((bus, u8::from_str_radix(device, 16).ok()?, function.parse().ok()?))
}

#[cfg(windows)]
mod pawn_win {
    use super::module_candidates;
    use std::ffi::{c_void, CString};
    use std::os::raw::c_char;
    use std::path::PathBuf;

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryA(name: *const c_char) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
        fn FreeLibrary(module: *mut c_void) -> i32;
        fn GetCurrentThread() -> *mut c_void;
        fn GetCurrentProcess() -> *mut c_void;
        fn SetThreadAffinityMask(thread: *mut c_void, mask: usize) -> usize;
        fn GetProcessAffinityMask(
            process: *mut c_void,
            process_mask: *mut usize,
            system_mask: *mut usize,
        ) -> i32;
    }

    type OpenFn = unsafe extern "system" fn(*mut *mut c_void) -> i32;
    type LoadFn = unsafe extern "system" fn(*mut c_void, *const u8, usize) -> i32;
    type ExecuteFn = unsafe extern "system" fn(
        *mut c_void,
        *const c_char,
        *const u64,
        usize,
        *mut u64,
        usize,
        *mut usize,
    ) -> i32;
    type CloseFn = unsafe extern "system" fn(*mut c_void) -> i32;

    /// An open PawnIO handle with one module (`IntelMSR`, `Nvidia`) loaded.
    pub struct Device {
        lib: *mut c_void,
        handle: *mut c_void,
        execute: ExecuteFn,
        close: CloseFn,
    }

    // The handle is only ever used from the sampler thread that created it.
    unsafe impl Send for Device {}

    unsafe fn symbol<T: Copy>(lib: *mut c_void, name: &[u8]) -> Option<T> {
        let address = GetProcAddress(lib, name.as_ptr() as *const c_char);
        (!address.is_null()).then(|| std::mem::transmute_copy::<*mut c_void, T>(&address))
    }

    impl Device {
        /// Opens the driver and loads the module file `name` (searched as `module_candidates`).
        pub fn open(configured: &str, name: &str) -> Option<Device> {
            let program_files = std::env::var_os("ProgramFiles").map(PathBuf::from);
            let exe_dir =
                std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
            let blob = module_candidates(configured, exe_dir, program_files.clone(), name)
                .into_iter()
                .find_map(|path| std::fs::read(path).ok().filter(|b| !b.is_empty()))?;

            unsafe {
                let mut lib = std::ptr::null_mut();
                if let Some(dir) = &program_files {
                    let dll = dir.join("PawnIO").join("PawnIOLib.dll");
                    if let Ok(c) = CString::new(dll.to_string_lossy().as_bytes()) {
                        lib = LoadLibraryA(c.as_ptr());
                    }
                }
                if lib.is_null() {
                    lib = LoadLibraryA(b"PawnIOLib.dll\0".as_ptr() as *const c_char);
                }
                if lib.is_null() {
                    return None;
                }
                let bound = (|| {
                    Some((
                        symbol::<OpenFn>(lib, b"pawnio_open\0")?,
                        symbol::<LoadFn>(lib, b"pawnio_load\0")?,
                        symbol::<ExecuteFn>(lib, b"pawnio_execute\0")?,
                        symbol::<CloseFn>(lib, b"pawnio_close\0")?,
                    ))
                })();
                let Some((open, load, execute, close)) = bound else {
                    FreeLibrary(lib);
                    return None;
                };
                let mut handle = std::ptr::null_mut();
                if open(&mut handle) < 0 || handle.is_null() {
                    FreeLibrary(lib);
                    return None;
                }
                if load(handle, blob.as_ptr(), blob.len()) < 0 {
                    close(handle);
                    FreeLibrary(lib);
                    return None;
                }
                Some(Device { lib, handle, execute, close })
            }
        }

        /// Calls `name` with `input`; `Ok` holds the values it returned, `Err` the status.
        pub fn call(&self, name: &str, input: &[u64], out_len: usize) -> Result<Vec<u64>, i32> {
            let Ok(name) = CString::new(name) else {
                return Err(-1);
            };
            let mut output = vec![0u64; out_len];
            let mut returned = 0usize;
            let status = unsafe {
                (self.execute)(
                    self.handle,
                    name.as_ptr(),
                    input.as_ptr(),
                    input.len(),
                    output.as_mut_ptr(),
                    output.len(),
                    &mut returned,
                )
            };
            if status < 0 {
                return Err(status);
            }
            output.truncate(returned.min(out_len));
            Ok(output)
        }

        /// Reads `msr` once on every logical processor this program may run on (the register
        /// belongs to the core the calling thread is on, so the thread visits each one in turn
        /// and then goes back to where it was).
        pub fn read_each_cpu(&self, msr: u32) -> Vec<u64> {
            self.each_cpu(|device| device.read_msr(msr))
        }

        /// The clock registers and the kind of core of every logical processor.
        pub fn read_cpu_clocks(&self) -> Vec<super::CpuClockRead> {
            self.each_cpu(|device| {
                Some(super::CpuClockRead {
                    status: device.read_msr(super::MSR_PERF_STATUS)?,
                    kind: core_kind(),
                })
            })
        }

        /// Runs `read` once on every logical processor this program may run on (the registers
        /// belong to the core the calling thread is on, so the thread visits each one in turn
        /// and then goes back to where it was). Answers that are `None` are left out.
        fn each_cpu<T>(&self, read: impl Fn(&Device) -> Option<T>) -> Vec<T> {
            let mut values = Vec::new();
            unsafe {
                let (mut process, mut system) = (0usize, 0usize);
                if GetProcessAffinityMask(GetCurrentProcess(), &mut process, &mut system) == 0
                    || process == 0
                {
                    return values;
                }
                let thread = GetCurrentThread();
                let mut original = 0usize;
                for cpu in 0..usize::BITS as usize {
                    let bit = 1usize << cpu;
                    if process & bit == 0 {
                        continue;
                    }
                    let previous = SetThreadAffinityMask(thread, bit);
                    if previous == 0 {
                        continue;
                    }
                    if original == 0 {
                        original = previous;
                    }
                    if let Some(value) = read(self) {
                        values.push(value);
                    }
                }
                if original != 0 {
                    SetThreadAffinityMask(thread, original);
                }
            }
            values
        }

        pub fn read_msr(&self, msr: u32) -> Option<u64> {
            self.call("ioctl_read_msr", &[u64::from(msr)], 1).ok().and_then(|v| v.first().copied())
        }
    }

    /// The kind of the core the calling thread is on (CPUID leaf 0x1A exists on hybrid processors).
    #[cfg(target_arch = "x86_64")]
    #[allow(unused_unsafe)]
    fn core_kind() -> u8 {
        use std::arch::x86_64::{__cpuid, __cpuid_count};
        unsafe {
            if __cpuid(0).eax < 0x1A {
                return 0;
            }
            (__cpuid_count(0x1A, 0).eax >> 24) as u8
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn core_kind() -> u8 {
        0
    }

    impl Drop for Device {
        fn drop(&mut self) {
            unsafe {
                (self.close)(self.handle);
                FreeLibrary(self.lib);
            }
        }
    }

    #[repr(C)]
    struct NvmlPciInfo {
        bus_id_legacy: [u8; 16],
        domain: u32,
        bus: u32,
        device: u32,
        pci_device_id: u32,
        pci_sub_system_id: u32,
        bus_id: [u8; 32],
    }

    /// PCI location (bus, device, function) of the first NVIDIA GPU, from the NVIDIA driver.
    pub fn nvidia_location() -> Option<(u8, u8, u8)> {
        type InitFn = unsafe extern "system" fn() -> i32;
        type IndexFn = unsafe extern "system" fn(u32, *mut *mut c_void) -> i32;
        type PciFn = unsafe extern "system" fn(*mut c_void, *mut NvmlPciInfo) -> i32;
        type ShutdownFn = unsafe extern "system" fn() -> i32;
        unsafe {
            let lib = LoadLibraryA(b"nvml.dll\0".as_ptr() as *const c_char);
            if lib.is_null() {
                return None;
            }
            let found = (|| {
                let init: InitFn = symbol(lib, b"nvmlInit_v2\0")?;
                let index: IndexFn = symbol(lib, b"nvmlDeviceGetHandleByIndex_v2\0")?;
                let pci: PciFn = symbol(lib, b"nvmlDeviceGetPciInfo_v3\0")?;
                let shutdown: ShutdownFn = symbol(lib, b"nvmlShutdown\0")?;
                if init() != 0 {
                    return None;
                }
                let mut handle = std::ptr::null_mut();
                let mut info = std::mem::zeroed::<NvmlPciInfo>();
                let ok = index(0, &mut handle) == 0 && pci(handle, &mut info) == 0;
                shutdown();
                if !ok {
                    return None;
                }
                let text = String::from_utf8_lossy(&info.bus_id).into_owned();
                // the bus id string ("00000000:01:00.0") also carries the function number
                super::parse_bus_id(&text).or(Some((info.bus as u8, info.device as u8, 0)))
            })();
            FreeLibrary(lib);
            found
        }
    }
}

#[cfg(windows)]
use pawn_win::{nvidia_location, Device as PawnDevice};

#[cfg(not(windows))]
struct PawnDevice;

#[cfg(not(windows))]
impl PawnDevice {
    fn open(_configured: &str, _name: &str) -> Option<PawnDevice> {
        None
    }

    fn call(&self, _name: &str, _input: &[u64], _out_len: usize) -> Result<Vec<u64>, i32> {
        Err(-1)
    }

    fn read_msr(&self, _msr: u32) -> Option<u64> {
        None
    }

    fn read_each_cpu(&self, _msr: u32) -> Vec<u64> {
        Vec::new()
    }

    fn read_cpu_clocks(&self) -> Vec<CpuClockRead> {
        Vec::new()
    }
}

#[cfg(not(windows))]
fn nvidia_location() -> Option<(u8, u8, u8)> {
    None
}

/// Where the NVIDIA module's raw answers are written, to check how they are read.
fn nvidia_dump_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA").map(PathBuf::from)?;
    Some(base.join("CPUnity-R-Helper").join("pawnio_nvidia.txt"))
}

/// PawnIO's `Nvidia` module: the GPU's memory temperature.
struct NvidiaMemory {
    device: PawnDevice,
    location: (u8, u8, u8),
}

impl NvidiaMemory {
    fn open(configured: &str) -> Option<NvidiaMemory> {
        let location = nvidia_location()?;
        let device = PawnDevice::open(configured, "Nvidia.bin")?;
        let this = NvidiaMemory { device, location };
        this.dump();
        Some(this)
    }

    fn input(&self) -> [u64; 3] {
        [u64::from(self.location.0), u64::from(self.location.1), u64::from(self.location.2)]
    }

    fn read(&self) -> Option<f32> {
        let out =
            self.device.call("ioctl_read_memory_temperatures", &self.input(), NVIDIA_MEMORY_OUT);
        nvidia_mem_temp(&out.ok()?)
    }

    /// Writes what both calls return once, with the location they were given.
    fn dump(&self) {
        let Some(path) = nvidia_dump_path() else {
            return;
        };
        let describe =
            |name: &str, out_len: usize| match self.device.call(name, &self.input(), out_len) {
                Ok(values) => format!("{}: {:?}\n", name, values),
                Err(status) => format!("{}: failed, status {:#x}\n", name, status),
            };
        let text = format!(
            "# PawnIO Nvidia module, as called by R-Helper\nGPU PCI location (bus, device, function): {:?}\n{}{}",
            self.location,
            describe("ioctl_read_memory_temperatures", NVIDIA_MEMORY_OUT),
            describe("ioctl_read_thermal_registers", NVIDIA_THERMAL_OUT),
        );
        let _ = std::fs::create_dir_all(path.parent().unwrap_or(&path));
        let _ = std::fs::write(path, text);
    }
}

/// The PawnIO driver, opened on the first sample and closed again with the sampler.
#[derive(Default)]
struct Pawn {
    device: Option<PawnDevice>,
    nvidia: Option<NvidiaMemory>,
    /// The NVIDIA module was looked for already (it is not retried every round).
    nvidia_tried: bool,
    state: MsrState,
    /// Rounds since the last failed attempt to open the driver (it is retried every few rounds).
    since_failure: u32,
}

impl Pawn {
    fn sample(&mut self, module_path: &str) -> Option<Sample> {
        if self.device.is_none() {
            if self.since_failure > 0 && self.since_failure < 5 {
                self.since_failure += 1;
                return None;
            }
            self.device = PawnDevice::open(module_path, "IntelMSR.bin");
            if self.device.is_none() {
                self.since_failure = 1;
                return None;
            }
            self.state = MsrState::default();
        }
        if !self.nvidia_tried {
            // optional: needs Nvidia.bin and an NVIDIA GPU; without them nothing changes
            self.nvidia_tried = true;
            self.nvidia = NvidiaMemory::open(module_path);
        }
        let device = self.device.as_ref()?;
        let msr = msr_sample(&|msr| device.read_msr(msr), &mut self.state, &Instant::now);
        let memory = self.nvidia.as_ref().and_then(NvidiaMemory::read);
        let cores = core_temp(self.state.target, &device.read_each_cpu(MSR_THERM_STATUS));
        let clocks = core_clocks(&device.read_cpu_clocks());
        if msr.is_none() && memory.is_none() && cores.is_none() {
            // nothing readable (wrong module, no rights): close and try again later
            self.device = None;
            self.nvidia = None;
            self.nvidia_tried = false;
            self.since_failure = 1;
            return None;
        }
        let mut sample = msr.unwrap_or_default();
        sample.gpu_mem_temp = memory;
        sample.core_temp = cores;
        sample.cpu_clock_top = clocks.fastest;
        sample.cpu_clock_p = clocks.performance;
        sample.cpu_clock_e = clocks.efficiency;
        Some(sample)
    }
}

// ---------------------------------------------------------------------- source

/// Reads every switched-on external source.
pub struct External {
    cfg: SourceConfig,
    pawn: Pawn,
    states: [SourceState; SOURCE_COUNT],
    /// The HWiNFO listing was written already.
    dumped: bool,
}

fn any_value(s: &Sample) -> bool {
    Reading::ALL.into_iter().any(|r| value_of(s, r).is_some())
}

/// Where the HWiNFO listing is written.
fn dump_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA").map(PathBuf::from)?;
    Some(base.join("CPUnity-R-Helper").join("hwinfo_sensors.txt"))
}

impl External {
    pub fn new(cfg: SourceConfig) -> Self {
        Self {
            cfg,
            pawn: Pawn::default(),
            states: [SourceState::default(); SOURCE_COUNT],
            dumped: false,
        }
    }

    /// How each source is doing (`Link::None` for the ones that are off).
    pub fn states(&self) -> [SourceState; SOURCE_COUNT] {
        self.states
    }

    /// Reads one source: its values, and what to say when it gave nothing.
    fn read(&mut self, source: usize) -> (Option<Sample>, &'static str) {
        match source {
            HWINFO => match hwinfo_read() {
                Ok(buf) => {
                    if !self.dumped {
                        // once per start: what HWiNFO offers, for finding the right labels
                        self.dumped = true;
                        if let Some(path) = dump_path() {
                            let _ = std::fs::create_dir_all(path.parent().unwrap_or(&path));
                            let _ = std::fs::write(path, hwinfo_dump(&buf));
                        }
                    }
                    (hwinfo_parse(&buf), HWINFO_INACTIVE)
                }
                Err(why) => (None, why),
            },
            LHM => (
                lhm_read(self.cfg.lhm_addr.trim()).and_then(|json| lhm_parse(&json)),
                "the web server did not answer (is Remote Web Server on, and is the address right?)",
            ),
            LOG => (
                log_read(self.cfg.log_path.trim())
                    .and_then(|(header, line)| csv_parse(&header, &line)),
                "the log file was not found, or has no data line yet",
            ),
            PAWN => (
                self.pawn.sample(&self.cfg.module_path),
                "not reachable: PawnIO installed, IntelMSR.bin (and Nvidia.bin for memory temperature) found, running as administrator?",
            ),
            _ => (None, ""),
        }
    }

    /// One round: the samples of the switched-on sources (index = source number).
    pub fn sample(&mut self) -> [Option<Sample>; SOURCE_COUNT] {
        let mut out: [Option<Sample>; SOURCE_COUNT] = [None; SOURCE_COUNT];
        for source in 1..SOURCE_COUNT {
            if !self.cfg.is_on(source) {
                self.states[source] = SourceState::default();
                continue;
            }
            let (sample, missing) = self.read(source);
            self.states[source] = match &sample {
                None => SourceState { link: Link::NotFound, detail: missing },
                Some(s) if !any_value(s) => SourceState {
                    link: Link::Connected,
                    detail: "connected, but none of the values R-Helper looks for were found",
                },
                Some(_) => SourceState { link: Link::Connected, detail: "" },
            };
            out[source] = sample;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(buf: &mut Vec<u8>, at: usize, bytes: &[u8]) {
        if buf.len() < at + bytes.len() {
            buf.resize(at + bytes.len(), 0);
        }
        buf[at..at + bytes.len()].copy_from_slice(bytes);
    }

    /// A shared memory image like HWiNFO's: header, sensors, readings.
    fn hwinfo_image(sensors: &[&str], readings: &[(u32, u32, &str, f64)]) -> Vec<u8> {
        let (sensor_size, reading_size) = (264usize, 316usize);
        let sensor_off = HWINFO_HEADER_LEN;
        let reading_off = sensor_off + sensors.len() * sensor_size;
        let mut buf = vec![0u8; reading_off + readings.len() * reading_size];
        put(&mut buf, 0, &HWINFO_SIGNATURE.to_le_bytes());
        for (at, v) in [
            (20, sensor_off as u32),
            (24, sensor_size as u32),
            (28, sensors.len() as u32),
            (32, reading_off as u32),
            (36, reading_size as u32),
            (40, readings.len() as u32),
        ] {
            put(&mut buf, at, &v.to_le_bytes());
        }
        for (i, name) in sensors.iter().enumerate() {
            put(&mut buf, sensor_off + i * sensor_size + 8, name.as_bytes());
        }
        for (i, (kind, sensor, label, value)) in readings.iter().enumerate() {
            let at = reading_off + i * reading_size;
            put(&mut buf, at, &kind.to_le_bytes());
            put(&mut buf, at + 4, &sensor.to_le_bytes());
            put(&mut buf, at + 12, label.as_bytes());
            put(&mut buf, at + 284, &value.to_le_bytes());
        }
        buf
    }

    #[test]
    fn hwinfo_values_are_found_by_label() {
        let image = hwinfo_image(
            &["CPU [#0]: Intel Core i9-13950HX: DTS", "GPU [#0]: NVIDIA GeForce RTX 4080: Sensors"],
            &[
                (1, 0, "Core Max", 88.0),
                (1, 0, "Core 3 Distance to TjMAX", 12.0),
                (1, 0, "CPU Package", 81.0),
                (5, 0, "CPU Package Power", 55.5),
                (5, 0, "PL1 Power Limit (Dynamic)", 90.0),
                (5, 0, "PL1 Power Limit (Static)", 80.0),
                (5, 0, "PL2 Power Limit (Static)", 157.0),
                (1, 1, "GPU Temperature", 61.0),
                (1, 1, "GPU Hot Spot Temperature", 70.0),
                (1, 1, "GPU Memory Junction Temperature", 66.0),
                (5, 1, "GPU Power", 120.0),
                (6, 0, "Core 0 Clock", 5200.0),
            ],
        );
        let s = hwinfo_parse(&image).unwrap();
        assert_eq!(s.cpu_temp, Some(81.0), "CPU Package beats Core Max");
        assert_eq!(s.core_temp, Some(88.0), "Core Max is the core temperature");
        assert_eq!(s.cpu_power, Some(55.5));
        assert_eq!(s.pl1, Some(80.0), "the static limit beats the dynamic one");
        assert_eq!(s.pl2, Some(157.0));
        assert_eq!(s.gpu_temp, Some(61.0));
        assert_eq!(s.gpu_mem_temp, Some(66.0));
        assert_eq!(s.gpu_power, Some(120.0));
    }

    #[test]
    fn the_hottest_single_core_stands_in_for_core_max() {
        let image = hwinfo_image(
            &["CPU [#0]: Intel Core i9: DTS"],
            &[
                (1, 0, "Core 0", 70.0),
                (1, 0, "Core 1", 77.0),
                (1, 0, "Core 2", 74.0),
                (1, 0, "Core 1 Distance to TjMAX", 23.0),
                (1, 0, "Core Average", 60.0),
            ],
        );
        let s = hwinfo_parse(&image).unwrap();
        assert_eq!(s.core_temp, Some(77.0), "distance and average readings are not cores");
    }

    #[test]
    fn core_temperature_is_the_hottest_valid_core() {
        let target = Some(100u64 << 16);
        let status = |readout: u64, valid: bool| (u64::from(valid) << 31) | (readout << 16);
        // readouts are degrees below TjMax: 30 -> 70 C, 20 -> 80 C; an invalid one is skipped
        assert_eq!(
            core_temp(target, &[status(30, true), status(20, true), status(5, false)]),
            Some(80.0)
        );
        assert_eq!(core_temp(target, &[status(5, false)]), None);
        assert_eq!(core_temp(target, &[]), None);
    }

    fn sample_with(f: impl FnOnce(&mut Sample)) -> Sample {
        let mut s = Sample::default();
        f(&mut s);
        s
    }

    #[test]
    fn auto_takes_the_first_source_that_has_the_value() {
        let builtin = sample_with(|s| {
            s.gpu_temp = Some(60.0);
            s.cpu_power = Some(10.0);
            s.gpu_clock = Some(2000.0);
        });
        let mut external: [Option<Sample>; SOURCE_COUNT] = [None; SOURCE_COUNT];
        external[HWINFO] = Some(sample_with(|s| {
            s.cpu_temp = Some(80.0);
            s.cpu_power = Some(44.0);
            s.gpu_temp = Some(61.0);
        }));
        external[PAWN] = Some(sample_with(|s| {
            s.cpu_temp = Some(81.0);
            s.pl1 = Some(80.0);
        }));
        let mut cfg = SourceConfig::default();
        cfg.enabled[HWINFO] = true;
        cfg.enabled[PAWN] = true;
        let out = merge(&builtin, &external, &cfg);
        assert_eq!(out.cpu_temp, Some(81.0), "PawnIO comes first");
        assert_eq!(out.pl1, Some(80.0));
        assert_eq!(out.cpu_power, Some(44.0), "HWiNFO before the built-in value");
        assert_eq!(out.gpu_temp, Some(61.0));
        assert_eq!(out.pl2, None);
        assert_eq!(out.gpu_clock, Some(2000.0), "readings without a choice stay as they are");
        // a source that is switched off is never used, even when it has a sample
        cfg.enabled[PAWN] = false;
        assert_eq!(merge(&builtin, &external, &cfg).cpu_temp, Some(80.0));
    }

    #[test]
    fn an_explicit_pick_is_honoured() {
        let builtin = sample_with(|s| s.gpu_temp = Some(60.0));
        let mut external: [Option<Sample>; SOURCE_COUNT] = [None; SOURCE_COUNT];
        external[HWINFO] = Some(sample_with(|s| {
            s.gpu_temp = Some(61.0);
            s.gpu_mem_temp = Some(66.0);
        }));
        external[LHM] = Some(sample_with(|s| s.gpu_temp = Some(62.0)));
        let mut cfg = SourceConfig::default();
        cfg.enabled[HWINFO] = true;
        cfg.enabled[LHM] = true;
        cfg.picks[Reading::GpuTemp.index()] = LHM as u8;
        assert_eq!(merge(&builtin, &external, &cfg).gpu_temp, Some(62.0));
        cfg.picks[Reading::GpuTemp.index()] = BUILT_IN as u8;
        assert_eq!(merge(&builtin, &external, &cfg).gpu_temp, Some(60.0));
        // the picked source has nothing: the reading stays empty instead of falling back
        cfg.picks[Reading::GpuMemTemp.index()] = LHM as u8;
        assert_eq!(merge(&builtin, &external, &cfg).gpu_mem_temp, None);
        // a pick of a source that is switched off (or cannot provide it) counts as Auto
        cfg.picks[Reading::GpuMemTemp.index()] = PAWN as u8;
        assert_eq!(cfg.pick(Reading::GpuMemTemp), None);
        assert_eq!(merge(&builtin, &external, &cfg).gpu_mem_temp, Some(66.0));
        cfg.picks[Reading::Pl1.index()] = LHM as u8;
        assert_eq!(cfg.pick(Reading::Pl1), None, "LibreHardwareMonitor has no PL1");
    }

    #[test]
    fn reading_names_round_trip_and_every_reading_has_a_source() {
        for reading in Reading::ALL {
            assert_eq!(Reading::from_key(reading.key()), Some(reading));
            assert_eq!(Reading::ALL[reading.index()], reading);
            assert!((0..SOURCE_COUNT).any(|s| reading.provided_by(s)));
            assert!(!reading.label().is_empty());
        }
        assert_eq!(Reading::from_key("nope"), None);
    }

    #[test]
    fn switched_off_sources_are_not_read() {
        let mut ext = External::new(SourceConfig::default());
        let out = ext.sample();
        assert!(out.iter().all(Option::is_none));
        assert!(ext.states().iter().all(|s| s.link == Link::None));
        // PawnIO on, but no driver here
        let mut cfg = only(PAWN);
        cfg.module_path = "Z:\\does\\not\\exist".into();
        let mut ext = External::new(cfg);
        assert!(ext.sample()[PAWN].is_none());
        assert_eq!(ext.states()[PAWN].link, Link::NotFound);
        assert!(
            ext.states()[PAWN].detail.contains("PawnIO") || !ext.states()[PAWN].detail.is_empty()
        );
    }

    #[test]
    fn hwinfo_listing_names_every_reading() {
        let image = hwinfo_image(
            &["CPU [#0]: Intel Core i9"],
            &[(1, 0, "CPU Package", 81.0), (5, 0, "CPU Package Power", 55.5)],
        );
        let text = hwinfo_dump(&image);
        assert!(text.contains("0: CPU [#0]: Intel Core i9"));
        assert!(text.contains("0 | 1 | CPU Package | 81"));
        assert!(text.contains("0 | 5 | CPU Package Power | 55.5"));
        assert_eq!(hwinfo_dump(&[]), "");
    }

    #[test]
    fn hwinfo_rejects_other_data() {
        assert!(hwinfo_parse(&[]).is_none());
        assert!(hwinfo_parse(&[0u8; 100]).is_none());
        let mut dead = hwinfo_image(&[], &[]);
        put(&mut dead, 0, b"DEAD");
        assert!(hwinfo_parse(&dead).is_none());
        // a truncated image must not panic
        let image = hwinfo_image(&["CPU"], &[(1, 0, "CPU Package", 50.0)]);
        let _ = hwinfo_parse(&image[..image.len() - 20]);
        assert_eq!(hwinfo_total_len(&image), Some(image.len()));
    }

    const LHM_JSON: &str = r#"{"id":0,"Text":"Sensor","Children":[{"id":1,"Text":"LAPTOP","Children":[
      {"id":2,"Text":"Intel Core i9-13950HX","Children":[
        {"id":3,"Text":"Temperatures","Children":[
          {"id":4,"Text":"Core Max","Value":"85,0 °C","Children":[]},
          {"id":5,"Text":"CPU Package","Value":"82.0 °C","Children":[]}]},
        {"id":6,"Text":"Powers","Children":[
          {"id":7,"Text":"CPU Package","Value":"54.3 W","Children":[]}]}]},
      {"id":8,"Text":"NVIDIA GeForce RTX 4080 Laptop GPU","Children":[
        {"id":9,"Text":"Temperatures","Children":[
          {"id":10,"Text":"GPU Core","Value":"60.0 °C","Children":[]},
          {"id":11,"Text":"GPU Hot Spot","Value":"69.0 °C","Children":[]},
          {"id":12,"Text":"GPU Memory Junction","Value":"64.0 °C","Children":[]}]},
        {"id":13,"Text":"Powers","Children":[
          {"id":14,"Text":"GPU Package","Value":"110.5 W","Children":[]}]}]}]}]}"#;

    #[test]
    fn librehardwaremonitor_json_is_read() {
        let s = lhm_parse(LHM_JSON).unwrap();
        assert_eq!(s.cpu_temp, Some(82.0));
        assert_eq!(s.cpu_power, Some(54.3));
        assert_eq!(s.gpu_temp, Some(60.0));
        assert_eq!(s.gpu_mem_temp, Some(64.0));
        assert_eq!(s.gpu_power, Some(110.5));
        assert_eq!(s.pl1, None);
        assert!(lhm_parse("not json").is_none());
        assert!(lhm_parse("{\"Text\": ").is_none());
        // a decimal comma is accepted
        assert_eq!(number_in("85,5 \u{b0}C"), Some(85.5));
        assert_eq!(number_in("-3.5 V"), Some(-3.5));
        assert_eq!(number_in("no digits"), None);
    }

    #[test]
    fn csv_columns_are_recognized_by_name() {
        let header = "Date,Time,CPU Package Temp,CPU Package Power,PL1 Power Limit,PL2 Power Limit,GPU Temperature,GPU Memory Junction Temp,GPU Power";
        let line = "2026-10-17,04:30:00,79.0,48.5,80,157,62,67,95.5";
        let s = csv_parse(header, line).unwrap();
        assert_eq!(s.cpu_temp, Some(79.0));
        assert_eq!(s.cpu_power, Some(48.5));
        assert_eq!(s.pl1, Some(80.0));
        assert_eq!(s.pl2, Some(157.0));
        assert_eq!(s.gpu_temp, Some(62.0));
        assert_eq!(s.gpu_mem_temp, Some(67.0));
        assert_eq!(s.gpu_power, Some(95.5));
        // semicolons and quotes
        let s = csv_parse("\"Time\";\"CPU Temp\"", "\"04:30\";\"71\"").unwrap();
        assert_eq!(s.cpu_temp, Some(71.0));
        assert!(csv_parse("onlyone", "1").is_none());
    }

    fn only(source: usize) -> SourceConfig {
        let mut cfg = SourceConfig::default();
        cfg.enabled[source] = true;
        cfg
    }

    #[test]
    fn log_file_uses_header_and_last_line() {
        let dir = std::env::temp_dir().join(format!("rh_sensor_log_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("log.csv");
        std::fs::write(&path, "Time,CPU Temp\n04:30:00,70\n04:30:01,72\n\n").unwrap();
        let (header, line) = log_read(path.to_str().unwrap()).unwrap();
        assert_eq!(header, "Time,CPU Temp");
        assert_eq!(line, "04:30:01,72");
        let mut ext = External::new(SourceConfig {
            log_path: path.to_str().unwrap().to_string(),
            ..only(LOG)
        });
        let out = ext.sample();
        assert_eq!(ext.states()[LOG].link, Link::Connected);
        assert_eq!(out[LOG].unwrap().cpu_temp, Some(72.0));
        // a header-only file has nothing to read
        std::fs::write(&path, "Time,CPU Temp\n").unwrap();
        assert!(log_read(path.to_str().unwrap()).is_none());
        let mut missing = External::new(SourceConfig {
            log_path: dir.join("nope.csv").to_str().unwrap().to_string(),
            ..only(LOG)
        });
        missing.sample();
        assert_eq!(missing.states()[LOG].link, Link::NotFound);
        assert!(missing.states()[LOG].detail.contains("log file"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn core_clocks_from_the_ratio_of_each_processor() {
        let read = |ratio: u64, kind: u8| CpuClockRead { status: ratio << 8 | 0x1234_0000, kind };
        // two threads of one performance core, two efficiency cores
        let clocks = core_clocks(&[read(54, 0x40), read(54, 0x40), read(30, 0x20), read(34, 0x20)]);
        assert_eq!(clocks.fastest, Some(5400.0));
        assert_eq!(clocks.performance, Some(5400.0));
        assert_eq!(clocks.efficiency, Some(3200.0));
        // not a hybrid processor: only the fastest one is known
        let plain = core_clocks(&[read(40, 0), read(44, 0)]);
        assert_eq!(
            (plain.fastest, plain.performance, plain.efficiency),
            (Some(4400.0), None, None)
        );
        // registers that did not answer are left out
        let odd = core_clocks(&[read(0, 0x40), read(200, 0x40), read(8, 0x20)]);
        assert_eq!(
            (odd.fastest, odd.performance, odd.efficiency),
            (Some(800.0), None, Some(800.0))
        );
        assert_eq!(core_clocks(&[]), CoreClocks::default());
    }

    #[test]
    fn single_source_readings_are_taken_from_any_source() {
        let mut builtin = Sample::default();
        builtin.cpu_clock_p = Some(4000.0);
        let mut pawn = Sample::default();
        pawn.cpu_clock_top = Some(5400.0);
        pawn.cpu_clock_e = Some(3100.0);
        let mut external: [Option<Sample>; SOURCE_COUNT] = Default::default();
        external[PAWN] = Some(pawn);
        let out = merge(&builtin, &external, &SourceConfig::default());
        assert_eq!(out.cpu_clock_p, Some(4000.0));
        assert_eq!(out.cpu_clock_top, Some(5400.0));
        assert_eq!(out.cpu_clock_e, Some(3100.0));
    }

    #[test]
    fn rapl_fields_are_decoded() {
        // typical Intel value 0x000A0E03: power unit 1/8 W, energy unit 1/2^14 J
        let units = rapl_units(0x000A_0E03);
        assert_eq!(units.power, 0.125);
        assert_eq!(units.energy, 1.0 / 16384.0);
        // PL1 = 640 counts (80 W), PL2 = 1256 counts (157 W)
        let raw = (1256u64 << 32) | 640;
        assert_eq!(power_limits(raw, units), (Some(80.0), Some(157.0)));
        assert_eq!(power_limits(0, units), (None, None));
        // enable and clamp bits above the 15 bit fields are ignored
        assert_eq!(power_limits(raw | (1 << 15) | (1 << 47), units), (Some(80.0), Some(157.0)));
    }

    #[test]
    fn package_temperature_is_tjmax_minus_readout() {
        let target = Some(100u64 << 16);
        let status = (1u64 << 31) | (25u64 << 16);
        assert_eq!(package_temp(target, status), Some(75.0));
        assert_eq!(package_temp(None, status), Some(75.0), "TjMax defaults to 100");
        assert_eq!(package_temp(Some(0), status), Some(75.0));
        assert_eq!(package_temp(target, 25u64 << 16), None, "invalid readout");
        assert_eq!(package_temp(Some(90u64 << 16), (1u64 << 31) | (95u64 << 16)), Some(0.0));
    }

    #[test]
    fn energy_counter_gives_watts_and_survives_wrap() {
        let units = RaplUnits { power: 0.125, energy: 1.0 / 16384.0 };
        // 16384 * 50 counts in one second = 50 W
        assert_eq!(energy_power(1000, 1000 + 16384 * 50, 1.0, units), Some(50.0));
        // the counter wrapped around 2^32
        assert_eq!(energy_power(u32::MAX - 100, 16384 * 10 - 101, 1.0, units), Some(10.0));
        assert_eq!(energy_power(0, 100, 0.05, units), None, "too short an interval");
    }

    #[test]
    fn register_rounds_combine_temperature_limits_and_power() {
        use std::cell::Cell;
        let energy = Cell::new(0u64);
        let read = |msr: u32| -> Option<u64> {
            match msr {
                MSR_RAPL_POWER_UNIT => Some(0x000A_0E03),
                MSR_TEMPERATURE_TARGET => Some(100u64 << 16),
                MSR_PACKAGE_THERM_STATUS => Some((1u64 << 31) | (22u64 << 16)),
                MSR_PKG_POWER_LIMIT => Some((1256u64 << 32) | 640),
                MSR_PKG_ENERGY_STATUS => Some(energy.get()),
                _ => None,
            }
        };
        let mut state = MsrState::default();
        let t0 = Instant::now();
        let first = msr_sample(&read, &mut state, &|| t0).unwrap();
        assert_eq!(first.cpu_temp, Some(78.0));
        assert_eq!((first.pl1, first.pl2), (Some(80.0), Some(157.0)));
        assert_eq!(first.cpu_power, None, "needs two energy readings");
        energy.set(16384 * 40);
        let second = msr_sample(&read, &mut state, &|| t0 + Duration::from_secs(1)).unwrap();
        assert_eq!(second.cpu_power, Some(40.0));
        // a second read right after the first does not produce a noisy value
        let third = msr_sample(&read, &mut state, &|| t0 + Duration::from_millis(1050)).unwrap();
        assert_eq!(third.cpu_power, None);
        // nothing readable at all means no sample
        assert!(msr_sample(&|_| None, &mut MsrState::default(), &|| t0).is_none());
    }

    #[test]
    fn nvidia_memory_temperature_is_the_hottest_plausible_value() {
        assert_eq!(nvidia_mem_temp(&[]), None);
        assert_eq!(nvidia_mem_temp(&[0, 0, 0]), None);
        assert_eq!(nvidia_mem_temp(&[4, 62, 66, 64, 0, 0]), Some(66.0));
        // raw register words and huge numbers are not temperatures
        assert_eq!(nvidia_mem_temp(&[0x8000_0000, 255, 5_000_000]), None);
        assert_eq!(nvidia_mem_temp(&[1, 2, 3]), None);
    }

    #[test]
    fn modules_are_found_next_to_the_configured_one() {
        let dir = std::env::temp_dir().join(format!("rh_modules_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("IntelMSR.bin"), "x").unwrap();
        let configured = dir.join("IntelMSR.bin").to_string_lossy().into_owned();
        assert_eq!(find_module(&configured, "IntelMSR.bin"), Some(dir.join("IntelMSR.bin")));
        assert_eq!(find_module(&configured, "Nvidia.bin"), None, "not there yet");
        std::fs::write(dir.join("Nvidia.bin"), "x").unwrap();
        assert_eq!(find_module(&configured, "Nvidia.bin"), Some(dir.join("Nvidia.bin")));
        // the folder alone works too
        assert_eq!(find_module(&dir.to_string_lossy(), "Nvidia.bin"), Some(dir.join("Nvidia.bin")));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pci_bus_ids_are_read() {
        assert_eq!(parse_bus_id("00000000:01:00.0"), Some((1, 0, 0)));
        assert_eq!(parse_bus_id("0000:2A:1F.3\0\0"), Some((0x2A, 0x1F, 3)));
        assert_eq!(parse_bus_id("garbage"), None);
        assert_eq!(parse_bus_id(""), None);
    }

    #[test]
    fn module_is_searched_in_the_usual_places() {
        let list = module_candidates(
            "D:\\mods",
            Some(PathBuf::from("C:\\app")),
            Some(PathBuf::from("C:\\Program Files")),
            "IntelMSR.bin",
        );
        assert_eq!(list[0], PathBuf::from("D:\\mods").join("IntelMSR.bin"));
        assert!(list.contains(&PathBuf::from("C:\\app").join("IntelMSR.bin")));
        assert!(list
            .iter()
            .any(|p| p.ends_with("PawnIO/IntelMSR.bin") || p.ends_with("PawnIO\\IntelMSR.bin")));
        // (built with join so the test also holds where `\\` is not a separator)
        let dir = PathBuf::from("D:\\x");
        let intel = dir.join("IntelMSR.bin");
        let file = module_candidates(&intel.to_string_lossy(), None, None, "IntelMSR.bin");
        assert_eq!(file, vec![intel.clone()]);
        assert!(module_candidates("", None, None, "IntelMSR.bin").is_empty());
        // another module is looked for next to the configured file
        let other = module_candidates(&intel.to_string_lossy(), None, None, "Nvidia.bin");
        assert_eq!(other, vec![dir.join("Nvidia.bin")]);
    }
}
