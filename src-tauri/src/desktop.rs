//! Keeps the widget glued to the desktop.
//!
//! Windows' "Show desktop" (Win+D) does not minimize normal windows. Instead it
//! raises the desktop host window (`Progman`/`WorkerW`) above every other
//! non-topmost window, which covers the widget. Rainmeter solves this by
//! keeping a tiny helper window pinned to the bottom of the z-order and
//! watching whether the desktop host has moved above it. When that happens the
//! skins are temporarily promoted above the raised desktop.
//!
//! This module implements the same trick:
//!
//! * The widget is marked as a tool window so the shell never treats it as an
//!   application window, and it is excluded from Aero Peek.
//! * A hidden marker window is kept at the bottom of the z-order. The desktop
//!   host being above the marker means "show desktop" is active.
//! * While show desktop is active the widget is briefly made topmost and then
//!   normal again, which lifts it above the raised desktop without leaving it
//!   in the topmost band (so the taskbar and other topmost windows still win).
//! * When show desktop ends the widget is placed right below the marker again,
//!   i.e. at the bottom of the normal band, above the desktop.
//!
//! The "Always on top" preference keeps its existing meaning: when enabled we
//! leave the topmost window alone and never demote it.

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::mem::size_of;
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
    use std::sync::{Mutex, Once};
    use std::time::Duration;

    use tauri::WebviewWindow;
    use windows::core::{w, BOOL};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_EXCLUDED_FROM_PEEK};
    use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DispatchMessageW, FindWindowExW, GetMessageW, GetShellWindow, GetWindow,
        GetWindowLongPtrW, IsWindow, IsWindowVisible, SetWindowLongPtrW, SetWindowPos,
        TranslateMessage, EVENT_SYSTEM_FOREGROUND, GWL_EXSTYLE, GW_HWNDNEXT, GW_HWNDPREV,
        HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST, MSG, SET_WINDOW_POS_FLAGS, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSENDCHANGING, SWP_NOSIZE, WINEVENT_OUTOFCONTEXT,
        WINEVENT_SKIPOWNPROCESS, WS_DISABLED, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        WS_POPUP,
    };

    /// Marker and widget handles are stored as `isize` so they can live in
    /// atomics. `0` means "not attached yet".
    static WIDGET: AtomicIsize = AtomicIsize::new(0);
    static MARKER: AtomicIsize = AtomicIsize::new(0);
    static TOPMOST: AtomicBool = AtomicBool::new(false);
    static SHOW_DESKTOP: AtomicBool = AtomicBool::new(false);
    static SYNC_LOCK: Mutex<()> = Mutex::new(());
    static HOOK_STARTED: Once = Once::new();

    const ZPOS_FLAGS: SET_WINDOW_POS_FLAGS = SET_WINDOW_POS_FLAGS(
        SWP_NOMOVE.0 | SWP_NOSIZE.0 | SWP_NOACTIVATE.0 | SWP_NOOWNERZORDER.0 | SWP_NOSENDCHANGING.0,
    );

    fn widget_handle() -> Option<HWND> {
        let value = WIDGET.load(Ordering::SeqCst);
        (value != 0).then_some(HWND(value as *mut c_void))
    }

    fn marker_handle() -> Option<HWND> {
        let value = MARKER.load(Ordering::SeqCst);
        (value != 0).then_some(HWND(value as *mut c_void))
    }

    fn set_z_order(window: HWND, after: HWND) {
        // SAFETY: `window` is a live top-level window owned by this process and
        // `after` is either a z-order sentinel or another live window.
        unsafe {
            let _ = SetWindowPos(window, Some(after), 0, 0, 0, 0, ZPOS_FLAGS);
        }
    }

    /// Adds `WS_EX_TOOLWINDOW` so the shell does not fold the widget into the
    /// Show Desktop/minimize pass and so it stays out of Alt+Tab.
    unsafe fn make_tool_window(window: HWND) {
        let style = GetWindowLongPtrW(window, GWL_EXSTYLE);
        let style = (style as u32 | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0;
        SetWindowLongPtrW(window, GWL_EXSTYLE, style as isize);
    }

    /// Aero Peek (hovering the Show Desktop button) hides every window that is
    /// not topmost. Excluding the widget keeps it visible while peeking.
    unsafe fn exclude_from_peek(window: HWND) {
        let value = BOOL(1);
        let _ = DwmSetWindowAttribute(
            window,
            DWMWA_EXCLUDED_FROM_PEEK,
            &value as *const BOOL as *const c_void,
            size_of::<BOOL>() as u32,
        );
    }

    /// Creates the hidden helper window that marks the bottom of the z-order.
    unsafe fn ensure_marker() -> Option<HWND> {
        if let Some(marker) = marker_handle() {
            if IsWindow(Some(marker)).as_bool() {
                return Some(marker);
            }
        }

        let marker = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("STATIC"),
            w!("Quoty desktop marker"),
            WS_POPUP | WS_DISABLED,
            -32000,
            -32000,
            1,
            1,
            None,
            None,
            None,
            None,
        )
        .ok()?;

        MARKER.store(marker.0 as isize, Ordering::SeqCst);
        set_z_order(marker, HWND_BOTTOM);
        Some(marker)
    }

    /// Returns the window that owns the desktop icons. On current Windows 11
    /// builds that is `Progman`; older builds use a `WorkerW` sibling.
    unsafe fn desktop_host() -> Option<HWND> {
        let shell = GetShellWindow();
        if shell.0.is_null() {
            return None;
        }

        if FindWindowExW(Some(shell), None, w!("SHELLDLL_DefView"), w!("")).is_ok() {
            return Some(shell);
        }

        let mut previous = None;
        loop {
            let Ok(worker) = FindWindowExW(None, previous, w!("WorkerW"), w!("")) else {
                break;
            };
            previous = Some(worker);

            if IsWindowVisible(worker).as_bool()
                && FindWindowExW(Some(worker), None, w!("SHELLDLL_DefView"), w!("")).is_ok()
            {
                return Some(worker);
            }
        }

        Some(shell)
    }

    /// True when `marker` sits below `host` in the z-order, which means the
    /// shell has raised the desktop above our bottom-pinned helper.
    unsafe fn marker_below(host: HWND, marker: HWND) -> bool {
        let Ok(mut current) = GetWindow(host, GW_HWNDNEXT) else {
            return false;
        };

        for _ in 0..4096 {
            if current == marker {
                return true;
            }
            let Ok(next) = GetWindow(current, GW_HWNDNEXT) else {
                return false;
            };
            current = next;
        }

        false
    }

    /// True when `window` is above `reference` in the z-order.
    unsafe fn is_above(window: HWND, reference: HWND) -> bool {
        let Ok(mut current) = GetWindow(reference, GW_HWNDPREV) else {
            return false;
        };

        for _ in 0..4096 {
            if current == window {
                return true;
            }
            let Ok(previous) = GetWindow(current, GW_HWNDPREV) else {
                return false;
            };
            current = previous;
        }

        false
    }

    /// Lifts the widget above the raised desktop. Going through the topmost
    /// band is required: `HWND_TOP` alone cannot rise above the desktop host
    /// once the shell has raised it.
    fn raise_above_desktop(window: HWND) {
        set_z_order(window, HWND_TOPMOST);
        set_z_order(window, HWND_NOTOPMOST);
    }

    /// Puts the widget back at the bottom of the normal band, directly below
    /// the marker (and therefore directly above the desktop).
    fn settle_to_desktop(window: HWND, marker: HWND) {
        let anchor = if unsafe { IsWindow(Some(marker)).as_bool() } {
            marker
        } else {
            HWND_BOTTOM
        };
        set_z_order(window, anchor);
    }

    fn is_desktop_active(host: HWND, marker: HWND) -> bool {
        // SAFETY: both handles are checked for validity by the caller.
        unsafe { IsWindowVisible(host).as_bool() && marker_below(host, marker) }
    }

    /// Re-evaluates the show-desktop state and adjusts the widget's z-order
    /// when it changes. Cheap and safe to call from any thread.
    pub fn sync() {
        let _guard = SYNC_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let Some(window) = widget_handle() else {
            return;
        };
        let Some(marker) = marker_handle() else {
            return;
        };
        if !unsafe { IsWindow(Some(window)).as_bool() } {
            return;
        }
        if !unsafe { IsWindow(Some(marker)).as_bool() } {
            return;
        }

        let Some(host) = (unsafe { desktop_host() }) else {
            return;
        };

        let active = is_desktop_active(host, marker);
        let was_active = SHOW_DESKTOP.swap(active, Ordering::SeqCst);

        // The user explicitly asked for a topmost widget; the shell cannot
        // cover it, and demoting it would break that preference.
        if TOPMOST.load(Ordering::SeqCst) {
            return;
        }

        let covered_by_desktop = !unsafe { is_above(window, host) };

        if active {
            // The desktop can be raised again while it is being interacted
            // with (for example after opening and closing its context menu),
            // so keep the widget above it for as long as Show Desktop is
            // active instead of only reacting to the initial transition.
            if covered_by_desktop {
                raise_above_desktop(window);
            }
        } else if was_active || covered_by_desktop {
            settle_to_desktop(window, marker);
        }
    }

    /// Updates the cached preference and, when it is turned off during show
    /// desktop, makes sure the widget does not end up behind the raised desktop.
    pub fn set_always_on_top(enabled: bool) {
        TOPMOST.store(enabled, Ordering::SeqCst);
        if enabled || !SHOW_DESKTOP.load(Ordering::SeqCst) {
            return;
        }

        let _guard = SYNC_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(window) = widget_handle() {
            if unsafe { IsWindow(Some(window)).as_bool() } {
                raise_above_desktop(window);
            }
        }
    }

    unsafe extern "system" fn win_event_proc(
        _hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        _id_object: i32,
        _id_child: i32,
        _event_thread: u32,
        _event_time: u32,
    ) {
        if event != EVENT_SYSTEM_FOREGROUND {
            return;
        }

        sync();

        // When the shell becomes foreground it may raise the desktop a few
        // milliseconds later. Retry briefly so the widget does not flicker.
        let shell = GetShellWindow();
        if hwnd == shell {
            for _ in 0..5 {
                std::thread::sleep(Duration::from_millis(4));
                sync();
            }
        }
    }

    fn start_hook_thread() {
        HOOK_STARTED.call_once(|| {
            let _ = std::thread::Builder::new()
                .name("quoty-desktop-hook".into())
                .spawn(|| {
                    // SAFETY: the hook outlives the message loop and is
                    // removed before the thread exits.
                    unsafe {
                        let hook = SetWinEventHook(
                            EVENT_SYSTEM_FOREGROUND,
                            EVENT_SYSTEM_FOREGROUND,
                            None,
                            Some(win_event_proc),
                            0,
                            0,
                            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                        );
                        if hook.0.is_null() {
                            return;
                        }

                        let mut message = MSG::default();
                        loop {
                            let result = GetMessageW(&mut message, None, 0, 0);
                            if result.0 <= 0 {
                                break;
                            }
                            let _ = TranslateMessage(&message);
                            let _ = DispatchMessageW(&message);
                        }

                        let _ = UnhookWinEvent(hook);
                    }
                });
        });
    }

    pub fn attach(window: &WebviewWindow, always_on_top: bool) {
        let Ok(hwnd) = window.hwnd() else {
            return;
        };

        let window = HWND(hwnd.0);
        WIDGET.store(window.0 as isize, Ordering::SeqCst);
        TOPMOST.store(always_on_top, Ordering::SeqCst);

        // SAFETY: `window` is the live widget window and the marker is created
        // on this thread, which runs the Tauri message loop.
        unsafe {
            make_tool_window(window);
            exclude_from_peek(window);
            ensure_marker();
        }

        start_hook_thread();
        sync();
    }
}

#[cfg(windows)]
pub use imp::{attach, set_always_on_top, sync};

#[cfg(not(windows))]
pub fn attach(_window: &tauri::WebviewWindow, _always_on_top: bool) {}

#[cfg(not(windows))]
pub fn set_always_on_top(_enabled: bool) {}

#[cfg(not(windows))]
pub fn sync() {}
