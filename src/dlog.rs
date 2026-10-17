// Minimal diagnostic log: %APPDATA%\CPUnity-R-Helper\log.txt (auto-reset above 256 KB).
use std::io::Write;

pub fn log(msg: &str) {
    let dir = crate::config::config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("log.txt");
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 262_144 {
            let _ = std::fs::remove_file(&path);
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            % 86_400;
        let _ =
            writeln!(f, "[{:02}:{:02}:{:02}Z] {}", secs / 3600, (secs / 60) % 60, secs % 60, msg);
    }
}
