//! Administrator rights: the PawnIO driver (CPU temperature, power limits, GPU memory
//! temperature) and some sensor sources need them. "Always run as administrator" in Options
//! restarts R-Helper elevated (Windows asks for permission once per start).

/// Marks the elevated copy, so a copy that still reports "not elevated" (for example when the
/// check itself fails) never starts yet another one.
pub const ELEVATED_FLAG: &str = "--elevated";

/// Quotes one command line argument the way Windows programs parse it.
pub fn quote_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_string();
    }
    let mut out = String::from("\"");
    let mut backslashes = 0;
    for c in arg.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                // backslashes in front of a quote are doubled, the quote itself is escaped
                out.push_str(&"\\".repeat(backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.push_str(&"\\".repeat(backslashes));
                out.push(c);
                backslashes = 0;
            }
        }
    }
    // backslashes in front of the closing quote are doubled too
    out.push_str(&"\\".repeat(backslashes * 2));
    out.push('"');
    out
}

/// The arguments for the elevated copy: the current ones plus the flag.
pub fn elevated_arguments(args: &[String]) -> String {
    let mut all: Vec<String> =
        args.iter().filter(|a| a.as_str() != ELEVATED_FLAG).map(|a| quote_arg(a)).collect();
    all.push(ELEVATED_FLAG.to_string());
    all.join(" ")
}

/// This copy was started by `relaunch_as_admin`.
pub fn started_elevated() -> bool {
    std::env::args().any(|a| a == ELEVATED_FLAG)
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    #[link(name = "advapi32")]
    extern "system" {
        fn OpenProcessToken(process: *mut c_void, access: u32, token: *mut *mut c_void) -> i32;
        fn GetTokenInformation(
            token: *mut c_void,
            class: u32,
            info: *mut c_void,
            length: u32,
            returned: *mut u32,
        ) -> i32;
    }

    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            window: *mut c_void,
            verb: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show: i32,
        ) -> isize;
    }

    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_ELEVATION: u32 = 20;
    const SW_SHOWNORMAL: i32 = 1;

    fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
        text.encode_wide().chain(std::iter::once(0)).collect()
    }

    pub fn is_elevated() -> bool {
        unsafe {
            let mut token = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return false;
            }
            let mut elevation = 0u32;
            let mut returned = 0u32;
            let ok = GetTokenInformation(
                token,
                TOKEN_ELEVATION,
                &mut elevation as *mut u32 as *mut c_void,
                std::mem::size_of::<u32>() as u32,
                &mut returned,
            ) != 0;
            CloseHandle(token);
            ok && elevation != 0
        }
    }

    /// Starts this program again with the "runas" verb. `false` when Windows did not start it
    /// (permission refused or no way to elevate).
    pub fn relaunch_as_admin(parameters: &str) -> bool {
        let Ok(exe) = std::env::current_exe() else {
            return false;
        };
        let verb = wide(std::ffi::OsStr::new("runas"));
        let file = wide(exe.as_os_str());
        let parameters = wide(std::ffi::OsStr::new(parameters));
        let directory = exe.parent().map(|d| wide(d.as_os_str()));
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                file.as_ptr(),
                parameters.as_ptr(),
                directory.as_ref().map_or(std::ptr::null(), |d| d.as_ptr()),
                SW_SHOWNORMAL,
            )
        };
        // values above 32 mean success
        result > 32
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn is_elevated() -> bool {
        false
    }

    pub fn relaunch_as_admin(_parameters: &str) -> bool {
        false
    }
}

/// The program runs with administrator rights.
pub fn is_elevated() -> bool {
    // cannot change while the program runs
    static ELEVATED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ELEVATED.get_or_init(imp::is_elevated)
}

/// Starts an elevated copy of this program with the same arguments. The caller quits when this
/// returns `true`.
pub fn relaunch_as_admin() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();
    imp::relaunch_as_admin(&elevated_arguments(&args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_quoted_for_windows() {
        assert_eq!(quote_arg("plain"), "plain");
        assert_eq!(quote_arg("two words"), "\"two words\"");
        assert_eq!(quote_arg(""), "\"\"");
        assert_eq!(quote_arg("say \"hi\""), "\"say \\\"hi\\\"\"");
        // a trailing backslash must not swallow the closing quote
        assert_eq!(quote_arg("C:\\my dir\\"), "\"C:\\my dir\\\\\"");
        assert_eq!(quote_arg("C:\\dir\\file"), "C:\\dir\\file");
    }

    #[test]
    fn elevated_copy_gets_the_flag_once() {
        assert_eq!(elevated_arguments(&[]), ELEVATED_FLAG);
        let args = vec!["a b".to_string(), ELEVATED_FLAG.to_string(), "c".to_string()];
        assert_eq!(elevated_arguments(&args), format!("\"a b\" c {}", ELEVATED_FLAG));
    }
}
