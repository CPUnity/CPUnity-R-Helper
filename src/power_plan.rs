//! The decisions behind "apply my AC / Battery profile", kept free of device access so they can be
//! tested. The code that talks to the laptop lives in main.rs.

/// What Windows says about the charger (`SYSTEM_POWER_STATUS::ACLineStatus`).
/// 0 = unplugged, 1 = plugged in, 255 = Windows does not know yet (common right after logon or
/// wake). "Does not know" must not count as "on battery": it used to, so a start-up could apply
/// the Battery profile, then flip to the AC profile a second later.
pub fn ac_flag(line_status: u8) -> Option<bool> {
    match line_status {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

/// Has the user picked a profile for this power source? "Default" means leave the laptop alone.
/// The same rule applies at launch and when the charger is plugged in or pulled out.
pub fn profile_is_explicit(
    on_ac: bool,
    ac_mode_set: bool,
    ac_overclock: bool,
    battery_mode_set: bool,
) -> bool {
    if on_ac {
        ac_mode_set || ac_overclock
    } else {
        battery_mode_set
    }
}

/// Sees the charger state once a second and reports a change only after two readings in a row
/// agree, so one odd reading (a dock waking, a flaky adapter) cannot flip the profile.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PowerWatch {
    pending: Option<bool>,
}

impl PowerWatch {
    /// `shown` is the state the app currently acts on, `reading` what Windows says now.
    /// Returns the new state when it has really changed.
    pub fn see(&mut self, shown: bool, reading: bool) -> Option<bool> {
        if reading == shown {
            self.pending = None;
            return None;
        }
        if self.pending == Some(reading) {
            self.pending = None;
            Some(reading)
        } else {
            self.pending = Some(reading);
            None
        }
    }
}

/// How many times to ask the EC whether the mode has taken, and how long to wait in between.
pub const MODE_WAIT_TRIES: u32 = 20;
pub const MODE_WAIT_MS: u64 = 100;

/// A boost name saved in the settings file (`Low`, `Medium`, ...), or nothing for a blank or
/// unknown one. The caller turns it into the device's own type.
pub fn clean_boost_name(name: &str) -> Option<String> {
    let t = name.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("default") || !t.chars().all(|c| c.is_ascii_alphabetic()) {
        None
    } else {
        Some(t.to_string())
    }
}

/// What the laptop was running last on one power source: the mode, and for Custom the CPU and GPU
/// boosts. Saved in the settings as `Mode|cpu|gpu` (for example `Custom|Overclock|High`).
#[derive(Debug, Clone, PartialEq)]
pub struct LastState {
    pub mode: String,
    pub cpu: Option<String>,
    pub gpu: Option<String>,
}

pub fn encode_last(mode: &str, cpu: Option<&str>, gpu: Option<&str>) -> String {
    format!("{}|{}|{}", mode, cpu.unwrap_or(""), gpu.unwrap_or(""))
}

pub fn decode_last(s: &str) -> Option<LastState> {
    let mut it = s.trim().split('|');
    let mode = it.next()?.trim();
    if mode.is_empty() || !mode.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let cpu = it.next().and_then(clean_boost_name);
    let gpu = it.next().and_then(clean_boost_name);
    if it.next().is_some() {
        return None;
    }
    Some(LastState { mode: mode.to_string(), cpu, gpu })
}

/// What to put on a power source: a mode, and the CPU and GPU boosts when it is Custom.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub mode: String,
    pub cpu: Option<String>,
    pub gpu: Option<String>,
}

/// The settings of one power source, as chosen in Options.
pub struct Pick<'a> {
    /// "Last Known Settings" is on, and what was saved (if anything yet).
    pub last_on: bool,
    pub last: Option<&'a str>,
    /// The mode chosen by hand (`None` is Default) and, on AC, the overclock switch.
    pub mode: Option<&'a str>,
    pub overclock: bool,
    /// The CPU and GPU chosen by hand for Custom (`None` is Default).
    pub cpu: Option<&'a str>,
    pub gpu: Option<&'a str>,
    /// The boosts last picked in Custom mode on the main page, used for a Default.
    pub fallback_cpu: Option<&'a str>,
    pub fallback_gpu: Option<&'a str>,
    /// The mode of a source that has no choice of its own.
    pub default_mode: &'a str,
}

/// The mode and boosts to put on a power source. Last Known Settings wins once something is saved;
/// otherwise the hand-picked mode, and for Custom its CPU and GPU.
pub fn resolve(p: &Pick) -> Target {
    if p.last_on {
        if let Some(l) = p.last.and_then(decode_last) {
            return Target { mode: l.mode, cpu: l.cpu, gpu: l.gpu };
        }
    }
    let mode = if p.overclock { "Custom" } else { p.mode.unwrap_or(p.default_mode) };
    if mode != "Custom" {
        return Target { mode: mode.to_string(), cpu: None, gpu: None };
    }
    let cpu = if p.overclock {
        Some("Overclock".to_string())
    } else {
        p.cpu.and_then(clean_boost_name).or_else(|| p.fallback_cpu.and_then(clean_boost_name))
    };
    let gpu = p.gpu.and_then(clean_boost_name).or_else(|| p.fallback_gpu.and_then(clean_boost_name));
    Target { mode: mode.to_string(), cpu, gpu }
}

/// Writes down what the laptop is running, but only once the same thing has been seen on two ticks
/// in a row and differs from what is saved, so a mode that is still settling is never saved.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Remember {
    pending: Option<String>,
}

impl Remember {
    pub fn see(&mut self, now: &str, saved: Option<&str>) -> Option<String> {
        if saved == Some(now) {
            self.pending = None;
            return None;
        }
        if self.pending.as_deref() == Some(now) {
            self.pending = None;
            Some(now.to_string())
        } else {
            self.pending = Some(now.to_string());
            None
        }
    }

    /// Forget what was being waited on (after a switch of power source).
    pub fn reset(&mut self) {
        self.pending = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_charger_state_is_not_battery() {
        assert_eq!(ac_flag(1), Some(true));
        assert_eq!(ac_flag(0), Some(false));
        assert_eq!(ac_flag(255), None, "Windows not knowing yet must not look like battery");
        assert_eq!(ac_flag(2), None);
    }

    #[test]
    fn default_profiles_leave_the_laptop_alone() {
        // nothing picked: neither on AC nor on battery
        assert!(!profile_is_explicit(true, false, false, false));
        assert!(!profile_is_explicit(false, false, false, false));
        // a mode picked for AC does not matter on battery, and the other way round
        assert!(profile_is_explicit(true, true, false, false));
        assert!(!profile_is_explicit(false, true, true, false));
        assert!(profile_is_explicit(false, false, false, true));
        assert!(!profile_is_explicit(true, false, false, true));
        // the overclock option alone counts on AC
        assert!(profile_is_explicit(true, false, true, false));
    }

    #[test]
    fn a_charger_change_needs_two_matching_readings() {
        let mut w = PowerWatch::default();
        assert_eq!(w.see(true, true), None);
        assert_eq!(w.see(true, false), None, "first odd reading only waits");
        assert_eq!(w.see(true, true), None, "and a flicker back cancels it");
        assert_eq!(w.see(true, false), None);
        assert_eq!(w.see(true, false), Some(false), "two in a row is real");
        assert_eq!(w.see(false, false), None);
        assert_eq!(w.see(false, true), None);
        assert_eq!(w.see(false, true), Some(true));
    }

    #[test]
    fn boost_names_are_cleaned() {
        assert_eq!(clean_boost_name("High"), Some("High".into()));
        assert_eq!(clean_boost_name(" Low "), Some("Low".into()));
        assert_eq!(clean_boost_name(""), None);
        assert_eq!(clean_boost_name("default"), None);
        assert_eq!(clean_boost_name("a=b"), None);
    }

    fn pick<'a>() -> Pick<'a> {
        Pick {
            last_on: false,
            last: None,
            mode: None,
            overclock: false,
            cpu: None,
            gpu: None,
            fallback_cpu: None,
            fallback_gpu: None,
            default_mode: "Performance",
        }
    }

    #[test]
    fn last_state_round_trips_and_rejects_junk() {
        let s = encode_last("Custom", Some("Overclock"), Some("High"));
        assert_eq!(s, "Custom|Overclock|High");
        let l = decode_last(&s).unwrap();
        assert_eq!((l.mode.as_str(), l.cpu.as_deref(), l.gpu.as_deref()), ("Custom", Some("Overclock"), Some("High")));
        let plain = decode_last(&encode_last("Balanced", None, None)).unwrap();
        assert_eq!((plain.mode.as_str(), plain.cpu, plain.gpu), ("Balanced", None, None));
        assert_eq!(decode_last(""), None);
        assert_eq!(decode_last("|a|b"), None);
        assert_eq!(decode_last("Custom|a|b|c"), None);
        assert_eq!(decode_last("Cus tom"), None);
    }

    #[test]
    fn last_known_wins_once_something_is_saved() {
        let mut p = pick();
        p.mode = Some("Silent");
        p.last_on = true;
        // nothing saved yet: the hand-picked mode is used
        assert_eq!(resolve(&p).mode, "Silent");
        p.last = Some("Custom|Boost|Medium");
        let t = resolve(&p);
        assert_eq!((t.mode.as_str(), t.cpu.as_deref(), t.gpu.as_deref()), ("Custom", Some("Boost"), Some("Medium")));
        // switched off: the hand-picked mode again
        p.last_on = false;
        assert_eq!(resolve(&p).mode, "Silent");
        // a saved line that is junk is ignored
        p.last_on = true;
        p.last = Some("???");
        assert_eq!(resolve(&p).mode, "Silent");
    }

    #[test]
    fn custom_takes_its_cpu_and_gpu_from_the_pick_then_the_last_custom() {
        let mut p = pick();
        p.mode = Some("Custom");
        p.fallback_cpu = Some("High");
        p.fallback_gpu = Some("Low");
        let t = resolve(&p);
        assert_eq!((t.cpu.as_deref(), t.gpu.as_deref()), (Some("High"), Some("Low")));
        p.cpu = Some("Boost");
        p.gpu = Some("High");
        let t = resolve(&p);
        assert_eq!((t.cpu.as_deref(), t.gpu.as_deref()), (Some("Boost"), Some("High")));
        // the overclock switch means Custom with the CPU on Overclock, and the GPU as picked
        p.mode = None;
        p.overclock = true;
        let t = resolve(&p);
        assert_eq!((t.mode.as_str(), t.cpu.as_deref(), t.gpu.as_deref()), ("Custom", Some("Overclock"), Some("High")));
        // other modes carry no boosts
        let mut q = pick();
        q.mode = Some("Balanced");
        q.cpu = Some("Boost");
        let t = resolve(&q);
        assert_eq!((t.mode.as_str(), t.cpu, t.gpu), ("Balanced", None, None));
        assert_eq!(resolve(&pick()).mode, "Performance");
    }

    #[test]
    fn what_is_running_is_saved_only_when_steady() {
        let mut r = Remember::default();
        assert_eq!(r.see("Custom|Low|Low", None), None);
        assert_eq!(r.see("Custom|Low|Low", None), Some("Custom|Low|Low".to_string()));
        // already saved: nothing to do
        assert_eq!(r.see("Custom|Low|Low", Some("Custom|Low|Low")), None);
        // a flicker to another value and back is not saved
        assert_eq!(r.see("Balanced||", Some("Custom|Low|Low")), None);
        assert_eq!(r.see("Custom|Low|Low", Some("Custom|Low|Low")), None);
        assert_eq!(r.see("Balanced||", Some("Custom|Low|Low")), None);
        assert_eq!(r.see("Balanced||", Some("Custom|Low|Low")), Some("Balanced||".to_string()));
        r.see("Silent||", None);
        r.reset();
        assert_eq!(r.see("Silent||", None), None, "a reset forgets what was pending");
    }
}
