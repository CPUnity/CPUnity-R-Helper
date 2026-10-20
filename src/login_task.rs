//! Starting R-Helper at login with administrator rights without the Windows permission prompt.
//!
//! Windows lets a Task Scheduler task that is set to "Run with highest privileges" start at
//! login without asking. Creating such a task needs administrator rights once, and only happens
//! when the user confirms in the dialog (`ui::login_task`), which shows the exact commands.
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, TryRecvError};

/// Name of the scheduled task.
pub const TASK_NAME: &str = "CPUnity R-Helper";

/// `text` as a PowerShell single-quoted string.
pub fn ps_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// The PowerShell commands that create (or replace) the task for `exe`.
pub fn setup_script(exe: &Path) -> String {
    format!(
        "$ErrorActionPreference = 'Stop'\n\
         $exe = {exe}\n\
         $user = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name\n\
         $action = New-ScheduledTaskAction -Execute $exe -WorkingDirectory (Split-Path $exe)\n\
         $trigger = New-ScheduledTaskTrigger -AtLogOn -User $user\n\
         $principal = New-ScheduledTaskPrincipal -UserId $user -RunLevel Highest -LogonType Interactive\n\
         $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero)\n\
         Register-ScheduledTask -TaskName {name} -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force | Out-Null\n",
        exe = ps_quote(&exe.to_string_lossy()),
        name = ps_quote(TASK_NAME),
    )
}

/// The PowerShell command that deletes the task.
pub fn remove_script() -> String {
    format!(
        "$ErrorActionPreference = 'Stop'\nUnregister-ScheduledTask -TaskName {} -Confirm:$false\n",
        ps_quote(TASK_NAME)
    )
}

#[cfg(windows)]
mod imp {
    use super::TASK_NAME;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub fn exists() -> bool {
        Command::new("schtasks")
            .args(["/Query", "/TN", TASK_NAME])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Runs a PowerShell script (from a temporary file) and reports the first line of its error.
    pub fn run_script(script: &str) -> Result<(), String> {
        let path = std::env::temp_dir().join(format!("cpunity-rhelper-{}.ps1", std::process::id()));
        std::fs::write(&path, script)
            .map_err(|e| format!("could not write a temporary file: {e}"))?;
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&path)
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        let _ = std::fs::remove_file(&path);
        let output = output.map_err(|e| format!("could not start PowerShell: {e}"))?;
        if output.status.success() {
            return Ok(());
        }
        let text = String::from_utf8_lossy(&output.stderr).into_owned()
            + &String::from_utf8_lossy(&output.stdout);
        let first = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("it failed");
        Err(first.to_string())
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn exists() -> bool {
        false
    }

    pub fn run_script(_script: &str) -> Result<(), String> {
        Err("only available on Windows".to_string())
    }
}

/// The task exists.
pub fn exists() -> bool {
    imp::exists()
}

/// What a background job is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    Create,
    Remove,
}

/// State of the Options section and its confirmation dialog.
#[derive(Default)]
pub struct State {
    /// Whether the task exists (`None` = not looked at yet).
    pub present: Option<bool>,
    /// The confirmation dialog is open.
    pub dialog_open: bool,
    /// A job is running in the background.
    pub busy: Option<(Job, Receiver<Result<(), String>>)>,
    /// Why the last job failed.
    pub error: Option<String>,
}

impl State {
    /// Looks the task up once (and again after it was cleared).
    pub fn refresh(&mut self) {
        if self.present.is_none() {
            self.present = Some(exists());
        }
    }

    fn start(&mut self, job: Job, script: String) {
        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            let _ = sender.send(imp::run_script(&script));
        });
        self.error = None;
        self.busy = Some((job, receiver));
    }

    /// Creates the task for `exe` in the background.
    pub fn create(&mut self, exe: &Path) {
        self.start(Job::Create, setup_script(exe));
    }

    /// Deletes the task in the background.
    pub fn remove(&mut self) {
        self.start(Job::Remove, remove_script());
    }

    /// Collects the result of a finished job; returns the job when it worked. A failure is kept
    /// in `error`.
    pub fn poll(&mut self) -> Option<Job> {
        let (job, receiver) = self.busy.as_ref()?;
        let job = *job;
        match receiver.try_recv() {
            Ok(Ok(())) => {
                self.busy = None;
                self.present = Some(job == Job::Create);
                self.dialog_open = false;
                Some(job)
            }
            Ok(Err(message)) => {
                self.busy = None;
                self.present = None;
                self.error = Some(message);
                None
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.busy = None;
                self.error = Some("the job ended unexpectedly".to_string());
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn quoting_survives_apostrophes() {
        assert_eq!(ps_quote("plain"), "'plain'");
        assert_eq!(ps_quote("it's"), "'it''s'");
    }

    #[test]
    fn script_registers_a_highest_privilege_logon_task() {
        let script = setup_script(&PathBuf::from("C:\\Users\\Jo's PC\\rhelper.exe"));
        assert!(script.contains("$exe = 'C:\\Users\\Jo''s PC\\rhelper.exe'"));
        assert!(script.contains("-AtLogOn"));
        assert!(script.contains("-RunLevel Highest"));
        assert!(script.contains("-AllowStartIfOnBatteries -DontStopIfGoingOnBatteries"));
        assert!(script.contains("-TaskName 'CPUnity R-Helper'"));
        assert!(script.starts_with("$ErrorActionPreference = 'Stop'"));
        assert!(remove_script().contains("Unregister-ScheduledTask -TaskName 'CPUnity R-Helper'"));
    }

    #[test]
    fn finished_jobs_update_the_state() {
        let mut state = State::default();
        assert_eq!(state.poll(), None);
        let (sender, receiver) = channel();
        state.busy = Some((Job::Create, receiver));
        state.dialog_open = true;
        assert_eq!(state.poll(), None, "still running");
        sender.send(Ok(())).unwrap();
        assert_eq!(state.poll(), Some(Job::Create));
        assert_eq!(state.present, Some(true));
        assert!(!state.dialog_open && state.busy.is_none());

        let (sender, receiver) = channel();
        state.busy = Some((Job::Remove, receiver));
        sender.send(Err("Access is denied".into())).unwrap();
        assert_eq!(state.poll(), None);
        assert_eq!(state.error.as_deref(), Some("Access is denied"));
        assert_eq!(state.present, None, "looked up again after a failure");
    }
}
