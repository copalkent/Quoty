mod desktop;
mod quota;
mod settings;
mod spotify;
mod window_ctl;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WebviewWindow, WindowEvent, Wry};

use quota::model::Snapshot;
use quota::QuotaService;
use settings::{Preferences, PreferencesPatch, SavedPosition};

pub struct AppState {
    prefs: Mutex<Preferences>,
    quota: QuotaService,
    always_on_top_item: Mutex<Option<CheckMenuItem<Wry>>>,
    position_dirty: AtomicBool,
    _tray: Mutex<Option<TrayIcon<Wry>>>,
}

impl AppState {
    fn new(prefs: Preferences, tray: TrayIcon<Wry>, top_item: CheckMenuItem<Wry>) -> Self {
        Self {
            prefs: Mutex::new(prefs),
            quota: QuotaService::new(),
            always_on_top_item: Mutex::new(Some(top_item)),
            position_dirty: AtomicBool::new(false),
            _tray: Mutex::new(Some(tray)),
        }
    }

    fn preferences(&self) -> Preferences {
        self.prefs
            .lock()
            .map(|prefs| prefs.clone())
            .unwrap_or_default()
    }

    fn set_preferences(&self, next: Preferences) {
        if let Ok(mut guard) = self.prefs.lock() {
            *guard = next;
        }
    }

    fn mark_moved(&self, position: &tauri::PhysicalPosition<i32>) {
        if let Ok(mut guard) = self.prefs.lock() {
            guard.position = Some(SavedPosition {
                x: position.x as f64,
                y: position.y as f64,
            });
        }
        self.position_dirty.store(true, Ordering::SeqCst);
    }
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
async fn quota_snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    Ok(state.quota.snapshot().await)
}

#[tauri::command]
fn get_preferences(state: State<'_, AppState>) -> Preferences {
    state.preferences()
}

#[tauri::command]
fn update_preferences(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: PreferencesPatch,
) -> Result<Preferences, String> {
    let mut next = state.preferences();
    let previous = next.clone();
    next.apply(patch);

    if previous.start_with_windows != next.start_with_windows {
        use tauri_plugin_autostart::ManagerExt;
        let manager = app.autolaunch();
        let result = if next.start_with_windows {
            manager.enable()
        } else {
            manager.disable()
        };
        if result.is_err() {
            return Err("Windows didn't accept the startup setting.".into());
        }
    }

    if previous.always_on_top != next.always_on_top {
        if let Some(window) = main_window(&app) {
            window_ctl::set_always_on_top(&window, next.always_on_top)?;
        }
    }

    if let Some(item) = state
        .always_on_top_item
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
    {
        let _ = item.set_checked(next.always_on_top);
    }

    settings::save(&app, &next)?;
    state.set_preferences(next.clone());
    let _ = app.emit("preferences-changed", &next);
    Ok(next)
}

#[tauri::command]
fn set_expanded(app: AppHandle, state: State<'_, AppState>, expanded: bool) -> Result<(), String> {
    eprintln!("[quoty] set_expanded({expanded})");
    let window = main_window(&app).ok_or("The widget window is missing.")?;
    window_ctl::apply_size(&window, expanded)?;

    let mut next = state.preferences();
    next.expanded = expanded;
    if let Ok(position) = window.outer_position() {
        next.position = Some(SavedPosition {
            x: position.x as f64,
            y: position.y as f64,
        });
    }
    settings::save(&app, &next)?;
    state.set_preferences(next);
    Ok(())
}

#[tauri::command]
fn begin_drag(window: WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|error| error.to_string())
}

#[tauri::command]
fn persist_position(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let window = main_window(&app).ok_or("The widget window is missing.")?;
    let position = window.outer_position().map_err(|error| error.to_string())?;

    let mut next = state.preferences();
    next.position = Some(SavedPosition {
        x: position.x as f64,
        y: position.y as f64,
    });
    settings::save(&app, &next)?;
    state.set_preferences(next);
    state.position_dirty.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn set_window_size(app: AppHandle, width: f64, height: f64) -> Result<(), String> {
    let window = main_window(&app).ok_or("The widget window is missing.")?;
    window
        .set_size(tauri::LogicalSize::new(
            width.clamp(320.0, 850.0),
            height.clamp(200.0, 900.0),
        ))
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn debug_trace(message: String) {
    eprintln!("[quoty] trace: {message}");
}

#[tauri::command]
fn ensure_on_screen(app: AppHandle) {
    if let Some(window) = main_window(&app) {
        window_ctl::ensure_visible(&window);
    }
}

#[tauri::command]
fn hide_widget(app: AppHandle) {
    if let Some(window) = main_window(&app) {
        window_ctl::hide(&window);
    }
}

#[tauri::command]
fn quit_app(app: AppHandle, state: State<'_, AppState>) {
    let _ = settings::save(&app, &state.preferences());
    app.exit(0);
}

#[tauri::command]
fn spotify_status() -> spotify::SpotifyStatus {
    spotify::current_status()
}

#[tauri::command]
fn spotify_play_pause() {
    spotify::play_pause();
}

#[tauri::command]
fn spotify_next() {
    spotify::next_track();
}

#[tauri::command]
fn spotify_prev() {
    spotify::prev_track();
}

#[tauri::command]
fn spotify_launch() {
    spotify::launch_spotify();
}

// ---------------------------------------------------------------------------
// Tray
// ---------------------------------------------------------------------------

fn build_tray(app: &AppHandle, prefs: &Preferences) -> tauri::Result<(TrayIcon<Wry>, CheckMenuItem<Wry>)> {
    let settings = MenuItemBuilder::with_id("settings", "Settings...").build(app)?;
    let show = MenuItemBuilder::with_id("show", "Show widget").build(app)?;
    let refresh = MenuItemBuilder::with_id("refresh", "Refresh quotas").build(app)?;
    let hide = MenuItemBuilder::with_id("hide", "Hide widget").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit Quoty").build(app)?;
    let always_on_top = CheckMenuItemBuilder::with_id("always-on-top", "Always on top")
        .checked(prefs.always_on_top)
        .build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&settings)
        .item(&always_on_top)
        .item(&refresh)
        .separator()
        .item(&show)
        .item(&hide)
        .separator()
        .item(&quit)
        .build()?;

    let mut builder = TrayIconBuilder::with_id("quoty-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Antigravity Quota HUD");

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    let tray = builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            "settings" => {
                if let Some(window) = main_window(app) {
                    window_ctl::show_without_focus(&window);
                }
                let _ = app.emit("open-settings", ());
            }
            "show" => {
                if let Some(window) = main_window(app) {
                    window_ctl::show_without_focus(&window);
                }
            }
            "hide" => {
                if let Some(window) = main_window(app) {
                    window_ctl::hide(&window);
                }
            }
            "refresh" => {
                let _ = app.emit("refresh-quota", ());
                if let Some(window) = main_window(app) {
                    window_ctl::show_without_focus(&window);
                }
            }
            "always-on-top" => {
                let Some(state) = app.try_state::<AppState>() else {
                    return;
                };
                let mut next = state.preferences();
                next.always_on_top = !next.always_on_top;
                if let Some(window) = main_window(app) {
                    let _ = window_ctl::set_always_on_top(&window, next.always_on_top);
                }
                if let Some(item) = state
                    .always_on_top_item
                    .lock()
                    .ok()
                    .and_then(|guard| guard.clone())
                {
                    let _ = item.set_checked(next.always_on_top);
                }
                let _ = settings::save(app, &next);
                state.set_preferences(next.clone());
                let _ = app.emit("preferences-changed", &next);
            }
            "quit" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let _ = settings::save(app, &state.preferences());
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = main_window(app) {
                    window_ctl::show_without_focus(&window);
                }
                let _ = app.emit("open-settings", ());
            }
        })
        .build(app)?;

    Ok((tray, always_on_top))
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

fn start_position_watcher(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(150));

        // Fallback for when the WinEvent hook is unavailable or the desktop
        // state changes without a foreground event.
        desktop::sync();

        let Some(state) = app.try_state::<AppState>() else {
            continue;
        };
        if !state.position_dirty.swap(false, Ordering::SeqCst) {
            continue;
        }
        let _ = settings::save(&app, &state.preferences());
    });
}

static ALREADY_RUNNING_DIALOG_OPEN: AtomicBool = AtomicBool::new(false);

/// Tells the user that the widget they just tried to launch is already running.
///
/// The single-instance callback runs on the running app's message loop, so the
/// dialog is created on a worker thread to keep the widget and tray responsive.
fn show_already_running_message() {
    #[cfg(windows)]
    {
        if ALREADY_RUNNING_DIALOG_OPEN.swap(true, Ordering::SeqCst) {
            return;
        }

        std::thread::spawn(|| {
            use windows::Win32::UI::WindowsAndMessaging::{
                MessageBoxW, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST,
            };

            // SAFETY: the dialog is a standalone top-level window; no borrowed
            // handles or pointers outlive this call.
            unsafe {
                let _ = MessageBoxW(
                    None,
                    windows::core::w!("Quoty is already running."),
                    windows::core::w!("Quoty"),
                    MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND | MB_TOPMOST,
                );
            }

            ALREADY_RUNNING_DIALOG_OPEN.store(false, Ordering::SeqCst);
        });
    }

    #[cfg(not(windows))]
    println!("Quoty is already running.");
}

pub fn run() {
    let app = tauri::Builder::default()
        // Register this first so a second launch exits before the app's own
        // setup can create another tray icon.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = main_window(app) {
                window_ctl::ensure_visible(&window);
                window_ctl::show_without_focus(&window);
            }
            show_already_running_message();
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            quota_snapshot,
            get_preferences,
            update_preferences,
            set_expanded,
            begin_drag,
            persist_position,
            ensure_on_screen,
            debug_trace,
            hide_widget,
            quit_app,
            spotify_status,
            spotify_play_pause,
            spotify_next,
            spotify_prev,
            spotify_launch,
            set_window_size
        ])
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::Moved(position) => {
                if let Some(state) = window.app_handle().try_state::<AppState>() {
                    state.mark_moved(position);
                }
            }
            _ => {}
        })
        .setup(|app| {
            use tauri_plugin_autostart::ManagerExt;

            let handle = app.handle().clone();
            let prefs = settings::load(&handle);

            // Keep the Windows startup entry in sync with the saved preference.
            if let Ok(enabled) = handle.autolaunch().is_enabled() {
                if enabled != prefs.start_with_windows {
                    let _ = if prefs.start_with_windows {
                        handle.autolaunch().enable()
                    } else {
                        handle.autolaunch().disable()
                    };
                }
            }

            let window = handle
                .get_webview_window("main")
                .ok_or("main window is missing")?;

            desktop::attach(&window, prefs.always_on_top);
            window_ctl::set_always_on_top(&window, prefs.always_on_top)?;

            let saved = prefs
                .position
                .map(|position| (position.x as i32, position.y as i32));
            eprintln!("[quoty] startup prefs expanded={} saved={saved:?}", prefs.expanded);
            window_ctl::restore_position(&window, saved, prefs.expanded)?;
            eprintln!(
                "[quoty] after restore pos={:?} size={:?}",
                window.outer_position(),
                window.outer_size()
            );

            let (tray, top_item) = build_tray(&handle, &prefs)?;
            app.manage(AppState::new(prefs, tray, top_item));

            window_ctl::show_without_focus(&window);
            start_position_watcher(handle);

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build Quoty");

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = app_handle.try_state::<AppState>() {
                let _ = settings::save(app_handle, &state.preferences());
            }
        }
    });
}
