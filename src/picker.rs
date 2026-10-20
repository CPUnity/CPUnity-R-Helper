//! "Browse..." for the Files tab: Windows' own open-file dialog, shown through PowerShell so no
//! extra library is needed. It runs in a background thread; the result arrives on a channel.
use std::sync::mpsc::{channel, Receiver};

/// What is being picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The CSV log of ThrottleStop / Intel XTU / HWiNFO.
    LogFile,
    /// PawnIO's IntelMSR.bin (Nvidia.bin is looked for next to it).
    ModuleFile,
    /// A sensor log to look at in the Log viewer.
    ViewLog,
    /// An mp3 to add to Game 3's songs.
    Song,
}

impl Target {
    pub fn title(self) -> &'static str {
        match self {
            Target::LogFile => "Choose the log file (CSV)",
            Target::ViewLog => "Choose a sensor log to view",
            Target::ModuleFile => "Choose IntelMSR.bin (Nvidia.bin is looked for next to it)",
            Target::Song => "Choose an mp3 to add",
        }
    }

    pub fn filter(self) -> &'static str {
        match self {
            Target::LogFile => {
                "Log files (*.csv;*.txt;*.log)|*.csv;*.txt;*.log|All files (*.*)|*.*"
            }
            Target::ModuleFile => "PawnIO modules (*.bin)|*.bin|All files (*.*)|*.*",
            Target::ViewLog => "Sensor logs (*.csv)|*.csv|All files (*.*)|*.*",
            Target::Song => "mp3 files (*.mp3)|*.mp3|All files (*.*)|*.*",
        }
    }
}

/// The PowerShell script that shows the dialog and prints the chosen path.
pub fn script(target: Target, initial: &str) -> String {
    format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8\n\
         Add-Type -AssemblyName System.Windows.Forms\n\
         $owner = New-Object System.Windows.Forms.Form -Property @{{ TopMost = $true }}\n\
         $dialog = New-Object System.Windows.Forms.OpenFileDialog\n\
         $dialog.Title = {title}\n\
         $dialog.Filter = {filter}\n\
         $start = {initial}\n\
         try {{\n\
         \x20   if ($start -and (Test-Path -LiteralPath $start -PathType Container)) {{ $dialog.InitialDirectory = $start }}\n\
         \x20   elseif ($start -and (Test-Path -LiteralPath (Split-Path -Parent $start))) {{ $dialog.InitialDirectory = Split-Path -Parent $start }}\n\
         }} catch {{ }}\n\
         if ($dialog.ShowDialog($owner) -eq [System.Windows.Forms.DialogResult]::OK) {{ [Console]::Out.Write($dialog.FileName) }}\n",
        title = crate::login_task::ps_quote(target.title()),
        filter = crate::login_task::ps_quote(target.filter()),
        initial = crate::login_task::ps_quote(initial.trim().trim_matches('"')),
    )
}

#[cfg(windows)]
fn run(script: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    let path =
        std::env::temp_dir().join(format!("cpunity-rhelper-pick-{}.ps1", std::process::id()));
    // a BOM, so PowerShell 5.1 reads non-ASCII paths correctly
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(script.as_bytes());
    std::fs::write(&path, bytes).ok()?;
    let output = std::process::Command::new("powershell.exe")
        .args(["-STA", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&path)
        .creation_flags(0x0800_0000)
        .output();
    let _ = std::fs::remove_file(&path);
    clean(&String::from_utf8_lossy(&output.ok()?.stdout))
}

#[cfg(not(windows))]
fn run(_script: &str) -> Option<String> {
    None
}

/// The chosen path out of the script's output (None = the dialog was cancelled).
pub fn clean(output: &str) -> Option<String> {
    let text = output.trim_matches(|c: char| c == '\u{feff}' || c.is_whitespace());
    (!text.is_empty()).then(|| text.to_string())
}

/// Shows the dialog in the background; the receiver gets `Some(path)` or `None` (cancelled).
pub fn spawn(target: Target, initial: &str) -> Receiver<Option<String>> {
    let script = script(target, initial);
    let (sender, receiver) = channel();
    std::thread::spawn(move || {
        let _ = sender.send(run(&script));
    });
    receiver
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_quotes_the_start_path() {
        let text = script(Target::LogFile, "\"C:\\Jo's logs\\t.csv\"");
        assert!(text.contains("$start = 'C:\\Jo''s logs\\t.csv'"));
        assert!(text.contains("OpenFileDialog"));
        assert!(text.contains("*.csv"));
        assert!(script(Target::ModuleFile, "").contains("$start = ''"));
        assert!(script(Target::ModuleFile, "").contains("*.bin"));
    }

    #[test]
    fn output_is_trimmed_and_empty_means_cancelled() {
        assert_eq!(clean("\u{feff}C:\\x\\a.csv\r\n"), Some("C:\\x\\a.csv".to_string()));
        assert_eq!(clean(""), None);
        assert_eq!(clean("  \r\n"), None);
    }
}
