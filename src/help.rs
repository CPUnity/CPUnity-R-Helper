//! The text of the Help tab: what each function does, one topic per function. Anything that
//! needs more than a line of explanation goes here instead of in small print under the controls
//! (the card keeps a "What does this do?" link that opens its topic).

pub struct Topic {
    /// Stable name, used by the cards' links and the remembered open / closed state.
    pub key: &'static str,
    /// The function's name as the tab shows it.
    pub title: &'static str,
    /// One paragraph each.
    pub text: &'static [&'static str],
}

pub const TOPICS: &[Topic] = &[
    Topic {
        key: "sensor_logging",
        title: "Logging",
        text: &[
            "Writes every reading (once per refresh interval) to a CSV file you can open in Excel, any graphing \
             tool, or the Log viewer under the switch. Each log is a new file, named with today's date in front. \
             It only writes while Read sensors is on.",
            "Start: Right away begins the moment you switch logging on. When a program starts waits for the program \
             you name (type its .exe name, or pick one that is running now) and begins a new file every time it starts. \
             If the program is already running when you switch logging on, it begins at once.",
            "Stop stops the log by itself after an exact time you set (hours, minutes and seconds), and with a program \
             also when the program closes. A log that was started right away switches logging off when its time is up. \
             A log that follows a program is one file per run, and waits for the program to close and start again \
             before it makes the next one.",
            "With no time limit and Stop when the program closes switched off, a log goes on until you switch it off. \
             That is allowed to be endless: at one reading a second it adds about 8 MB a day.",
            "Logging that waits for a program is remembered when you close R-Helper, so the next start picks it up \
             again. A log that starts right away is never remembered: after a start nothing is written unless you \
             switch it on.",
            "A file you are logging to right now can be opened in the Log viewer while it is still being written.",
        ],
    },
    Topic {
        key: "sensors",
        title: "Sensors",
        text: &[
            "Read sensors takes the readings on the Sensors tab every refresh interval. Now is the latest reading. \
             Average and Maximum count from the moment you switched Read sensors on (or pressed Reset average / max).",
            "\"n/a\" means the chosen source does not provide that reading on this laptop; \"--\" is a reading that is \
             just missing right now. Package temperature, PL1 / PL2 and usually package power need a program with a \
             kernel driver: pick HWiNFO, LibreHardwareMonitor or a log file under Options > Sensor sources.",
            "Hottest thermal zone is the laptop's ACPI zone, not the CPU core temperature. GPU memory junction is shown \
             only when the NVIDIA driver (or the external source) reports it.",
            "Clocks: CPU clock (all threads) is Windows' own figure, the same as Task Manager's speed. Fastest core, \
             P-cores average and E-cores average are read from each core's own register through the PawnIO driver (they \
             show n/a without it), as the core's current speed in 100 MHz steps. The GPU clock \
             comes from the NVIDIA driver (NVAPI, the library Afterburner reads, or NVML if that does not answer).",
            "Comparing with another program (HWiNFO, say): averages agree closely, but Maximum can differ a lot. \
             Package power is worked out from the CPU's energy counter over each refresh interval, so a short interval \
             sees brief bursts that a slower program averages away. For a fair comparison, give both programs the same \
             interval, reset both at the same moment, and compare the same stretch of time.",
        ],
    },
    Topic {
    key: "power_profile",
    title: "AC / battery profile",
    text: &[
        "Chooses what the laptop does when R-Helper starts and whenever you plug it in or unplug it. \
         Each power source (AC power, battery) has its own profile.",
        "Default leaves the laptop's mode alone, both at launch and when you plug or unplug it \
         (unless the other power source has a profile of yours).",
        "Custom lets you pick the CPU and GPU level for that power source. Default there means the \
         levels you last picked on the main page.",
        "Last Known Settings puts back exactly what the laptop was running last on that power source: \
         the mode and, for Custom, the CPU and GPU level. It replaces the choices above for that \
         source. Switch it off to set a profile by hand again.",
        "Apply current power profile now runs the profile of the power source you are on right away, \
         without waiting for a plug or unplug.",
    ],
    },
];

pub fn topic(key: &str) -> Option<&'static Topic> {
    TOPICS.iter().find(|t| t.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_topic_has_a_unique_key_a_title_and_text() {
        for (i, t) in TOPICS.iter().enumerate() {
            assert!(!t.key.is_empty() && !t.title.is_empty() && !t.text.is_empty(), "{}", t.key);
            assert!(t.text.iter().all(|p| !p.trim().is_empty()));
            assert!(TOPICS[i + 1..].iter().all(|o| o.key != t.key), "{} appears twice", t.key);
            assert!(!t.key.contains(char::is_whitespace));
        }
        assert!(topic("power_profile").is_some());
        assert!(topic("nope").is_none());
    }
}
