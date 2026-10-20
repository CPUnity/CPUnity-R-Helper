//! Everything R-Helper stores on the PC, for the Files tab: where it is and what it holds.
use std::path::{Path, PathBuf};

/// One place on disk (a folder, or one file shown selected in File Explorer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub icon: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
    pub path: PathBuf,
    /// `path` is a file: Explorer opens its folder with the file selected.
    pub is_file: bool,
}

/// The places, most useful first. `exe` is this program's file (None when unknown).
pub fn places(config_dir: &Path, log_dir: &Path, exe: Option<&Path>) -> Vec<Place> {
    let mut list = vec![
        Place {
            icon: "\u{2699}",
            title: "Settings folder",
            detail: "Everything R-Helper keeps for you. Delete the folder to reset R-Helper to \
                     its defaults.",
            path: config_dir.to_path_buf(),
            is_file: false,
        },
        Place {
            icon: "\u{1f4c4}",
            title: "Settings file",
            detail: "config.txt: every setting, lighting preset, fan profile and which \
                     drop-downs are open.",
            path: config_dir.join("config.txt"),
            is_file: true,
        },
        Place {
            icon: "\u{1f4ca}",
            title: "Sensor logs",
            detail: "The CSV files written by Logging, one new file each time logging starts. \
                     Never deleted by R-Helper.",
            path: log_dir.to_path_buf(),
            is_file: false,
        },
        Place {
            icon: "\u{1f50d}",
            title: "Diagnostic log",
            detail: "log.txt: a small troubleshooting log (starts over above 256 KB). The HWiNFO \
                     and PawnIO listings are in the settings folder too.",
            path: config_dir.join("log.txt"),
            is_file: true,
        },
        Place {
            icon: "\u{1f4dc}",
            title: "Synapse standby script",
            detail: "synapse-standby.ps1: the script behind Razer Synapse Standby, written when \
                     you use it.",
            path: config_dir.join("synapse-standby.ps1"),
            is_file: true,
        },
    ];
    if let Some(dir) = exe.and_then(Path::parent) {
        list.push(Place {
            icon: "\u{1f5a5}",
            title: "Program folder",
            detail: "Where R-Helper runs from. A logo.png, banner.png or font.ttf placed here \
                     customizes the look; IntelMSR.bin and Nvidia.bin can sit here too.",
            path: dir.to_path_buf(),
            is_file: false,
        });
    }
    list
}

/// What is at a path the user typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStatus {
    Empty,
    Missing,
    File,
    Folder,
}

pub fn path_status(text: &str) -> PathStatus {
    let text = text.trim().trim_matches('"');
    if text.is_empty() {
        return PathStatus::Empty;
    }
    let path = Path::new(text);
    if path.is_file() {
        PathStatus::File
    } else if path.is_dir() {
        PathStatus::Folder
    } else {
        PathStatus::Missing
    }
}

/// What to hand to File Explorer for `place`: the file selected when it exists, otherwise the
/// folder it would be in.
pub fn explorer_args(place: &Place, exists: bool) -> Vec<String> {
    if place.is_file {
        if exists {
            return vec![format!("/select,{}", place.path.display())];
        }
        return vec![place.path.parent().unwrap_or(&place.path).display().to_string()];
    }
    vec![place.path.display().to_string()]
}

/// A web page the Files tab can open, with what to get there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    pub title: &'static str,
    pub url: &'static str,
    /// What to download once the page is open.
    pub get: &'static str,
}

/// Where the PawnIO driver and its modules come from, in the order to do them.
pub const PAWN_LINKS: [Link; 2] = [
    Link {
        title: "1. PawnIO driver",
        url: "https://pawnio.eu",
        get: "Download the PawnIO installer from the page and run it once (needs administrator \
              rights). Nothing else is installed.",
    },
    Link {
        title: "2. Modules: IntelMSR.bin and Nvidia.bin",
        url: "https://github.com/namazso/PawnIO.Modules/releases",
        get: "Open the newest release and, under Assets, download the module files. IntelMSR.bin \
              reads the CPU (temperature, PL1 / PL2, power); Nvidia.bin reads the GPU memory \
              temperature. If they come in a .zip, extract it, then point the box below at \
              IntelMSR.bin (Nvidia.bin is found next to it).",
    },
];

/// Opens a web page in the default browser. Only fixed addresses from this program are passed.
pub fn open_url(url: &str) {
    if !url.starts_with("https://") {
        return;
    }
    let _ =
        std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_links_are_https_and_say_what_to_get() {
        assert_eq!(PAWN_LINKS.len(), 2);
        for link in PAWN_LINKS {
            assert!(link.url.starts_with("https://") && !link.url.contains(' '));
            assert!(!link.title.is_empty() && !link.get.is_empty());
        }
        let modules = PAWN_LINKS[1];
        assert!(modules.url.ends_with("PawnIO.Modules/releases"));
        assert!(modules.title.contains("IntelMSR.bin") && modules.title.contains("Nvidia.bin"));
        // anything that is not https is ignored rather than handed to Windows
        open_url("file:///C:/Windows/System32/calc.exe");
    }

    #[test]
    fn every_place_is_listed_once_with_its_path() {
        let config = PathBuf::from("C:\\Users\\a\\AppData\\Roaming\\CPUnity-R-Helper");
        let logs = config.join("logs");
        let exe = PathBuf::from("C:\\Apps\\rhelper.exe");
        let list = places(&config, &logs, Some(&exe));
        assert_eq!(list.len(), 6);
        assert_eq!(list[0].path, config);
        assert!(list.iter().any(|p| p.path == logs && !p.is_file));
        assert!(list.iter().any(|p| p.path == config.join("config.txt") && p.is_file));
        let titles: std::collections::HashSet<_> = list.iter().map(|p| p.title).collect();
        assert_eq!(titles.len(), list.len(), "titles are unique");
        // without an exe path there is no program folder tile
        assert_eq!(places(&config, &logs, None).len(), 5);
    }

    #[test]
    fn typed_paths_are_classified() {
        let dir = std::env::temp_dir().join(format!("rh_places_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.csv");
        std::fs::write(&file, "x").unwrap();
        assert_eq!(path_status(""), PathStatus::Empty);
        assert_eq!(path_status("  "), PathStatus::Empty);
        assert_eq!(path_status(&file.to_string_lossy()), PathStatus::File);
        assert_eq!(path_status(&format!("\"{}\"", dir.display())), PathStatus::Folder);
        assert_eq!(path_status(&dir.join("nope.csv").to_string_lossy()), PathStatus::Missing);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn explorer_selects_existing_files_and_opens_folders() {
        let file = Place {
            icon: "",
            title: "f",
            detail: "",
            path: PathBuf::from("/data/cfg/config.txt"),
            is_file: true,
        };
        assert_eq!(explorer_args(&file, true), vec!["/select,/data/cfg/config.txt".to_string()]);
        assert_eq!(explorer_args(&file, false), vec!["/data/cfg".to_string()]);
        let dir = Place { is_file: false, path: PathBuf::from("/data/cfg"), ..file };
        assert_eq!(explorer_args(&dir, true), vec!["/data/cfg".to_string()]);
    }
}
