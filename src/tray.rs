//! Notification-area (system tray) icon for "Close button hides to the tray".
//!
//! The icon lives on its own thread with a hidden message window, so it keeps working while the
//! main window is hidden. Left click (or double click) shows the window again; right click opens
//! a small menu with "Show" and "Quit". The icon exists only while the option is switched on.
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// What the tray thread tells the main window.
#[derive(Default)]
pub struct Signals {
    quit: AtomicBool,
    shown: AtomicBool,
}

pub struct Tray {
    signals: Arc<Signals>,
    #[cfg(windows)]
    hwnd: isize,
}

impl Tray {
    /// "Quit" was chosen in the tray menu.
    pub fn quit_requested(&self) -> bool {
        self.signals.quit.load(Ordering::SeqCst)
    }

    /// The window was brought back from the tray since the last call.
    pub fn take_shown(&self) -> bool {
        self.signals.shown.swap(false, Ordering::SeqCst)
    }
}

#[cfg(not(windows))]
impl Tray {
    pub fn start(_ctx: egui::Context) -> Option<Tray> {
        None
    }
}

#[cfg(windows)]
mod win {
    use super::{egui, Arc, Ordering, Signals, Tray};
    use std::ffi::c_void;
    use std::sync::Mutex;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> isize;
        fn GetCurrentProcessId() -> u32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn RegisterClassW(class: *const WndClassW) -> u16;
        fn CreateWindowExW(
            ex_style: u32,
            class: *const u16,
            title: *const u16,
            style: u32,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            parent: isize,
            menu: isize,
            instance: isize,
            param: *const c_void,
        ) -> isize;
        fn DefWindowProcW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
        fn GetMessageW(msg: *mut Msg, hwnd: isize, min: u32, max: u32) -> i32;
        fn TranslateMessage(msg: *const Msg) -> i32;
        fn DispatchMessageW(msg: *const Msg) -> isize;
        fn PostMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> i32;
        fn PostQuitMessage(code: i32);
        fn DestroyWindow(hwnd: isize) -> i32;
        fn LoadIconW(instance: isize, name: *const u16) -> isize;
        fn RegisterWindowMessageW(name: *const u16) -> u32;
        fn CreatePopupMenu() -> isize;
        fn AppendMenuW(menu: isize, flags: u32, id: usize, text: *const u16) -> i32;
        fn TrackPopupMenu(
            menu: isize,
            flags: u32,
            x: i32,
            y: i32,
            reserved: i32,
            hwnd: isize,
            rect: *const c_void,
        ) -> i32;
        fn DestroyMenu(menu: isize) -> i32;
        fn GetCursorPos(point: *mut Point) -> i32;
        fn SetForegroundWindow(hwnd: isize) -> i32;
        fn ShowWindow(hwnd: isize, command: i32) -> i32;
        fn GetWindow(hwnd: isize, command: u32) -> isize;
        fn GetWindowTextW(hwnd: isize, text: *mut u16, max: i32) -> i32;
        fn GetWindowThreadProcessId(hwnd: isize, process: *mut u32) -> u32;
        fn EnumWindows(
            callback: unsafe extern "system" fn(isize, isize) -> i32,
            param: isize,
        ) -> i32;
    }

    #[link(name = "shell32")]
    extern "system" {
        fn Shell_NotifyIconW(message: u32, data: *const NotifyIconDataW) -> i32;
    }

    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    struct Msg {
        hwnd: isize,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        pt: Point,
    }

    #[repr(C)]
    struct WndClassW {
        style: u32,
        wnd_proc: unsafe extern "system" fn(isize, u32, usize, isize) -> isize,
        class_extra: i32,
        window_extra: i32,
        instance: isize,
        icon: isize,
        cursor: isize,
        background: isize,
        menu_name: *const u16,
        class_name: *const u16,
    }

    /// `NOTIFYICONDATAW`.
    #[repr(C)]
    struct NotifyIconDataW {
        size: u32,
        hwnd: isize,
        id: u32,
        flags: u32,
        callback_message: u32,
        icon: isize,
        tip: [u16; 128],
        state: u32,
        state_mask: u32,
        info: [u16; 256],
        timeout_or_version: u32,
        info_title: [u16; 64],
        info_flags: u32,
        guid: [u32; 4],
        balloon_icon: isize,
    }

    const NIM_ADD: u32 = 0;
    const NIM_DELETE: u32 = 2;
    const NIF_MESSAGE: u32 = 1;
    const NIF_ICON: u32 = 2;
    const NIF_TIP: u32 = 4;
    const WM_DESTROY: u32 = 0x0002;
    const WM_CLOSE: u32 = 0x0010;
    const WM_COMMAND: u32 = 0x0111;
    const WM_NULL: u32 = 0x0000;
    const WM_LBUTTONUP: u32 = 0x0202;
    const WM_LBUTTONDBLCLK: u32 = 0x0203;
    const WM_RBUTTONUP: u32 = 0x0205;
    const WM_TRAY: u32 = 0x0400 + 1;
    const HWND_MESSAGE: isize = -3;
    const TPM_RIGHTBUTTON: u32 = 0x0002;
    const TPM_RETURNCMD: u32 = 0x0100;
    const SW_RESTORE: i32 = 9;
    const GW_OWNER: u32 = 4;
    const MENU_SHOW: usize = 1;
    const MENU_QUIT: usize = 2;
    /// `IDI_APPICON` in razer-gui.rc.
    const APP_ICON_ID: usize = 101;
    /// `IDI_APPLICATION`.
    const DEFAULT_ICON_ID: usize = 32512;
    const TOOLTIP: &str = "CPUnity:R-Helper";

    struct Shared {
        signals: Arc<Signals>,
        ctx: egui::Context,
        /// Registered "TaskbarCreated" message: the icon is added again when Explorer restarts.
        taskbar_created: u32,
        icon: isize,
    }

    static SHARED: Mutex<Option<Shared>> = Mutex::new(None);

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn icon_data(hwnd: isize, icon: isize) -> NotifyIconDataW {
        let mut data = NotifyIconDataW {
            size: std::mem::size_of::<NotifyIconDataW>() as u32,
            hwnd,
            id: 1,
            flags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            callback_message: WM_TRAY,
            icon,
            tip: [0; 128],
            state: 0,
            state_mask: 0,
            info: [0; 256],
            timeout_or_version: 0,
            info_title: [0; 64],
            info_flags: 0,
            guid: [0; 4],
            balloon_icon: 0,
        };
        for (slot, unit) in data.tip.iter_mut().zip(TOOLTIP.encode_utf16().take(127)) {
            *slot = unit;
        }
        data
    }

    /// Forgets the shared state, unless a newer tray has replaced it already.
    fn clear_shared(mine: &Arc<Signals>) {
        let mut guard = SHARED.lock().unwrap_or_else(|e| e.into_inner());
        if guard.as_ref().map_or(false, |s| Arc::ptr_eq(&s.signals, mine)) {
            *guard = None;
        }
    }

    fn with_shared<R>(f: impl FnOnce(&Shared) -> R) -> Option<R> {
        let guard = SHARED.lock().unwrap_or_else(|e| e.into_inner());
        guard.as_ref().map(f)
    }

    unsafe extern "system" fn collect_window(hwnd: isize, found: isize) -> i32 {
        let mut process = 0u32;
        GetWindowThreadProcessId(hwnd, &mut process);
        if process == GetCurrentProcessId() && GetWindow(hwnd, GW_OWNER) == 0 {
            // the event loop has invisible helper windows too; ours is the one with the title
            let mut title = [0u16; 64];
            let len = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32).max(0) as usize;
            if String::from_utf16_lossy(&title[..len]) == TOOLTIP {
                *(found as *mut isize) = hwnd;
                return 0; // stop
            }
        }
        1
    }

    /// The main window: the top-level window of this process titled like the app.
    fn main_window() -> isize {
        let mut found: isize = 0;
        unsafe {
            EnumWindows(collect_window, &mut found as *mut isize as isize);
        }
        found
    }

    /// Shows the main window again. The window is brought back with Windows calls, not through
    /// egui: a hidden window may not run its update loop, so egui commands would just sit there.
    fn show_window() {
        let window = main_window();
        if window != 0 {
            unsafe {
                ShowWindow(window, SW_RESTORE);
                SetForegroundWindow(window);
            }
        }
        with_shared(|s| {
            s.signals.shown.store(true, Ordering::SeqCst);
            s.ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            s.ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            s.ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            s.ctx.request_repaint();
        });
    }

    /// Quits R-Helper. The main window is asked to close; if its update loop does not react (it
    /// is hidden), the process ends anyway after a moment. Nothing needs saving at exit: settings
    /// are written when they change.
    fn quit_app(tray: isize) {
        let icon = with_shared(|s| {
            s.signals.quit.store(true, Ordering::SeqCst);
            s.ctx.request_repaint();
            s.icon
        });
        let window = main_window();
        if window != 0 {
            unsafe {
                PostMessageW(window, WM_CLOSE, 0, 0);
            }
        }
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            if let Some(icon) = icon {
                unsafe {
                    Shell_NotifyIconW(NIM_DELETE, &icon_data(tray, icon));
                }
            }
            std::process::exit(0);
        });
    }

    unsafe fn show_menu(hwnd: isize) {
        let menu = CreatePopupMenu();
        if menu == 0 {
            return;
        }
        AppendMenuW(menu, 0, MENU_SHOW, wide("Show R-Helper").as_ptr());
        AppendMenuW(menu, 0, MENU_QUIT, wide("Quit").as_ptr());
        let mut at = Point { x: 0, y: 0 };
        GetCursorPos(&mut at);
        // required so the menu closes again when the user clicks elsewhere
        SetForegroundWindow(hwnd);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_RETURNCMD,
            at.x,
            at.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);
        match chosen as usize {
            MENU_SHOW => show_window(),
            MENU_QUIT => quit_app(hwnd),
            _ => {}
        }
    }

    unsafe extern "system" fn window_proc(
        hwnd: isize,
        msg: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        match msg {
            WM_TRAY => {
                match (lparam as u32) & 0xFFFF {
                    WM_LBUTTONUP | WM_LBUTTONDBLCLK => show_window(),
                    WM_RBUTTONUP => show_menu(hwnd),
                    _ => {}
                }
                0
            }
            WM_COMMAND => {
                match wparam & 0xFFFF {
                    MENU_SHOW => show_window(),
                    MENU_QUIT => quit_app(hwnd),
                    _ => {}
                }
                0
            }
            WM_DESTROY => {
                if let Some(icon) = with_shared(|s| s.icon) {
                    Shell_NotifyIconW(NIM_DELETE, &icon_data(hwnd, icon));
                }
                PostQuitMessage(0);
                0
            }
            _ => {
                let restarted = with_shared(|s| s.taskbar_created != 0 && s.taskbar_created == msg);
                if restarted == Some(true) {
                    if let Some(icon) = with_shared(|s| s.icon) {
                        Shell_NotifyIconW(NIM_ADD, &icon_data(hwnd, icon));
                    }
                    return 0;
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
    }

    impl Tray {
        /// Adds the icon. `None` if Windows refused (the close button then quits as usual).
        pub fn start(ctx: egui::Context) -> Option<Tray> {
            let signals = Arc::new(Signals::default());
            let (sent, received) = std::sync::mpsc::channel::<isize>();
            let thread_signals = Arc::clone(&signals);
            std::thread::Builder::new()
                .name("tray".into())
                .spawn(move || unsafe {
                    let instance = GetModuleHandleW(std::ptr::null());
                    let mut icon = LoadIconW(instance, APP_ICON_ID as *const u16);
                    if icon == 0 {
                        icon = LoadIconW(0, DEFAULT_ICON_ID as *const u16);
                    }
                    let taskbar_created = RegisterWindowMessageW(wide("TaskbarCreated").as_ptr());
                    let mine = Arc::clone(&thread_signals);
                    *SHARED.lock().unwrap_or_else(|e| e.into_inner()) =
                        Some(Shared { signals: thread_signals, ctx, taskbar_created, icon });

                    let class_name = wide("CPUnityRHelperTray");
                    let class = WndClassW {
                        style: 0,
                        wnd_proc: window_proc,
                        class_extra: 0,
                        window_extra: 0,
                        instance,
                        icon: 0,
                        cursor: 0,
                        background: 0,
                        menu_name: std::ptr::null(),
                        class_name: class_name.as_ptr(),
                    };
                    RegisterClassW(&class); // already registered is fine
                    let hwnd = CreateWindowExW(
                        0,
                        class_name.as_ptr(),
                        class_name.as_ptr(),
                        0,
                        0,
                        0,
                        0,
                        0,
                        HWND_MESSAGE,
                        0,
                        instance,
                        std::ptr::null(),
                    );
                    if hwnd == 0 || Shell_NotifyIconW(NIM_ADD, &icon_data(hwnd, icon)) == 0 {
                        if hwnd != 0 {
                            DestroyWindow(hwnd);
                        }
                        clear_shared(&mine);
                        let _ = sent.send(0);
                        return;
                    }
                    let _ = sent.send(hwnd);
                    let mut msg = Msg {
                        hwnd: 0,
                        message: 0,
                        wparam: 0,
                        lparam: 0,
                        time: 0,
                        pt: Point { x: 0, y: 0 },
                    };
                    while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                    clear_shared(&mine);
                })
                .ok()?;
            let hwnd = received.recv_timeout(std::time::Duration::from_secs(3)).ok()?;
            (hwnd != 0).then_some(Tray { signals, hwnd })
        }
    }

    impl Drop for Tray {
        /// Removes the icon and ends the tray thread.
        fn drop(&mut self) {
            unsafe {
                PostMessageW(self.hwnd, WM_CLOSE, 0, 0);
            }
        }
    }
}
