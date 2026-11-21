use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WebviewWindow,
};
use tauri_plugin_global_shortcut::ShortcutState;

mod colors;
mod launcher;
mod lyrics;
mod media;
mod settings;
mod spotify;

use media::{Cmd, Media};
use settings::{Action, SettingsState};

const MAIN: &str = "main";


fn send_media(app: &AppHandle, cmd: Cmd) {
    if let Some(m) = app.try_state::<Media>() {
        m.send(cmd);
    }
}

/// Play/pause. If Spotify isn't running, start it hidden and press play once
/// it's up (when the "Start Spotify when needed" setting is on).
fn play_pause(app: &AppHandle) {
    let wake = app.try_state::<SettingsState>().is_some_and(|s| s.launch_spotify());
    if wake && !launcher::is_running_or_starting() {
        match launcher::launch_hidden() {
            Ok(()) => send_media(app, Cmd::PlayWhenReady),
            Err(e) => {
                let _ = app.emit("toast", e);
            }
        }
        return;
    }
    send_media(app, Cmd::TogglePlay);
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN)
}

/// Windows-only window tweaks: never steal focus from the app you're working
/// in, and stay out of the taskbar and Alt+Tab.
#[cfg(windows)]
mod native {
    use std::sync::atomic::{AtomicIsize, Ordering};
    use tauri::WebviewWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowLongPtrW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos,
        IsWindowVisible, ShowWindow, GWL_EXSTYLE, SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE, SW_SHOWNOACTIVATE,
        WS_EX_APPWINDOW,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    /// The window that had focus before Tunelet took it, so we can hand it back.
    static PREVIOUS: AtomicIsize = AtomicIsize::new(0);

    fn hwnd(w: &WebviewWindow) -> Option<*mut core::ffi::c_void> {
        w.hwnd().ok().map(|h| h.0 as _)
    }

    /// `interactive = false`: clicks never activate the window (default), and
    /// focus goes back to the app you were in.
    /// `interactive = true`: window can take keyboard focus (e.g. typing in search).
    pub fn set_interactive(w: &WebviewWindow, interactive: bool) {
        let Some(h) = hwnd(w) else { return };
        unsafe {
            let fg = GetForegroundWindow();
            if interactive {
                if !fg.is_null() && fg != h {
                    PREVIOUS.store(fg as isize, Ordering::Relaxed);
                }
            } else {
                let prev = PREVIOUS.swap(0, Ordering::Relaxed);
                // Only hand focus back if we still have it (the user may
                // already have clicked somewhere else).
                if prev != 0 && fg == h {
                    SetForegroundWindow(prev as _);
                }
            }
            let mut style = GetWindowLongPtrW(h, GWL_EXSTYLE);
            style |= WS_EX_TOOLWINDOW as isize;
            style &= !(WS_EX_APPWINDOW as isize);
            if interactive {
                style &= !(WS_EX_NOACTIVATE as isize);
            } else {
                style |= WS_EX_NOACTIVATE as isize;
            }
            SetWindowLongPtrW(h, GWL_EXSTYLE, style);
        }
    }

    pub fn show_no_activate(w: &WebviewWindow) {
        if let Some(h) = hwnd(w) {
            unsafe { ShowWindow(h, SW_SHOWNOACTIVATE) };
        }
    }

    // Show/hide/visibility all go through Win32 directly. Mixing a native
    // show with the window library's hide() breaks: the library keeps its own
    // visibility flag, still thinks the window is hidden, and ignores hide().
    pub fn hide(w: &WebviewWindow) {
        if let Some(h) = hwnd(w) {
            unsafe { ShowWindow(h, SW_HIDE) };
        }
    }

    pub fn is_visible(w: &WebviewWindow) -> bool {
        hwnd(w).map(|h| unsafe { IsWindowVisible(h) != 0 }).unwrap_or(false)
    }

    /// Move and resize in one step (physical pixels), so content never jumps.
    pub fn set_bounds(w: &WebviewWindow, x: i32, y: i32, width: i32, height: i32) {
        if let Some(h) = hwnd(w) {
            unsafe { SetWindowPos(h, core::ptr::null_mut(), x, y, width, height, SWP_NOZORDER | SWP_NOACTIVATE) };
        }
    }
}

#[cfg(not(windows))]
mod native {
    use tauri::WebviewWindow;
    pub fn set_interactive(_w: &WebviewWindow, _interactive: bool) {}
    pub fn show_no_activate(w: &WebviewWindow) {
        let _ = w.show();
    }
    pub fn hide(w: &WebviewWindow) {
        let _ = w.hide();
    }
    pub fn is_visible(w: &WebviewWindow) -> bool {
        w.is_visible().unwrap_or(false)
    }
    pub fn set_bounds(w: &WebviewWindow, x: i32, y: i32, width: i32, height: i32) {
        let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = w.set_size(tauri::PhysicalSize::new(width as u32, height as u32));
    }
}

fn show_widget(app: &AppHandle) {
    if let Some(w) = main_window(app) {
        native::show_no_activate(&w);
        let _ = w.set_always_on_top(true);
    }
}

fn toggle_widget(app: &AppHandle) {
    if let Some(w) = main_window(app) {
        if native::is_visible(&w) {
            native::hide(&w);
        } else {
            show_widget(app);
        }
    }
}

#[tauri::command]
fn hide_widget(app: AppHandle) {
    if let Some(w) = main_window(&app) {
        native::hide(&w);
    }
}

#[tauri::command]
fn set_interactive(app: AppHandle, interactive: bool) {
    if let Some(w) = main_window(&app) {
        native::set_interactive(&w, interactive);
        if interactive {
            let _ = w.set_focus();
        }
    }
}

/// Moves/resizes the widget window in logical (DPI-independent) pixels.
#[tauri::command]
fn set_bounds(app: AppHandle, x: f64, y: f64, width: f64, height: f64) {
    if let Some(w) = main_window(&app) {
        let s = w.scale_factor().unwrap_or(1.0);
        let p = |v: f64| (v * s).round() as i32;
        native::set_bounds(&w, p(x), p(y), p(width), p(height));
    }
}

#[derive(serde::Serialize)]
struct ScreenArea {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// The usable area (excluding the taskbar) of the monitor the widget is on, in logical pixels.
#[tauri::command]
fn screen_area(app: AppHandle) -> Option<ScreenArea> {
    let w = main_window(&app)?;
    let m = w.current_monitor().ok().flatten().or_else(|| w.primary_monitor().ok().flatten())?;
    let s = m.scale_factor();
    let a = m.work_area();
    Some(ScreenArea {
        x: a.position.x as f64 / s,
        y: a.position.y as f64 / s,
        width: a.size.width as f64 / s,
        height: a.size.height as f64 / s,
    })
}

/// Transport controls from the UI: "toggle", "next", "previous" or "refresh".
#[tauri::command]
fn media(app: AppHandle, action: String) {
    let cmd = match action.as_str() {
        "toggle" => return play_pause(&app),
        "next" => Cmd::Next,
        "previous" => Cmd::Previous,
        _ => Cmd::Refresh,
    };
    send_media(&app, cmd);
}

/// Whether the left mouse button is held. A window drag is run by Windows,
/// which swallows the button-up, so the UI polls this to know when it ended.
#[tauri::command]
fn mouse_down() -> bool {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
        (GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000) != 0
    }
    #[cfg(not(windows))]
    false
}

/// Seek via Windows media controls (works without a Spotify login).
#[tauri::command]
fn media_seek(app: AppHandle, position_ms: u64) {
    send_media(&app, Cmd::Seek(position_ms));
}


fn open_panel(app: &AppHandle, event: &str) {
    show_widget(app);
    let _ = app.emit(event, ());
}

// ---- hold-to-repeat for next/previous ------------------------------------------
// Windows registers hotkeys with "no repeat", so holding a shortcut fires it
// once. For skipping, holding should keep going: repeat until it's released.

use std::sync::atomic::{AtomicU64, Ordering};

/// Incremented on every press/release; a repeat loop stops once it changes.
static REPEAT_GEN: AtomicU64 = AtomicU64::new(0);
const REPEAT_DELAY: std::time::Duration = std::time::Duration::from_millis(450);
const REPEAT_EVERY: std::time::Duration = std::time::Duration::from_millis(350);

fn start_repeat(app: &AppHandle, action: Action) {
    let generation = REPEAT_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(REPEAT_DELAY);
        while REPEAT_GEN.load(Ordering::SeqCst) == generation {
            run_action(&app, action);
            std::thread::sleep(REPEAT_EVERY);
        }
    });
}

fn stop_repeat() {
    REPEAT_GEN.fetch_add(1, Ordering::SeqCst);
}

fn run_action(app: &AppHandle, action: Action) {
    match action {
        Action::Toggle => toggle_widget(app),
        Action::Search => open_panel(app, "open-search"),
        Action::PlayPause => play_pause(app),
        Action::Next => send_media(app, Cmd::Next),
        Action::Previous => send_media(app, Cmd::Previous),
        Action::Like => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let sp = app.state::<spotify::Spotify>();
                let result = match sp.toggle_like_current().await {
                    Ok(liked) => serde_json::json!({ "liked": liked }),
                    Err(e) => serde_json::json!({ "error": e }),
                };
                let _ = app.emit("like-result", result);
            });
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // A second launch just shows the running widget instead of opening another.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_widget(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    let Some(action) = app.try_state::<SettingsState>().and_then(|s| s.action_for(shortcut)) else {
                        return;
                    };
                    match event.state() {
                        ShortcutState::Pressed => {
                            run_action(app, action);
                            if matches!(action, Action::Next | Action::Previous) {
                                start_repeat(app, action);
                            }
                        }
                        ShortcutState::Released => stop_repeat(),
                    }
                })
                .build(),
        )
        .setup(|app| {
            app.manage(media::start(app.handle().clone()));
            app.manage(spotify::Spotify::new());
            app.manage(settings::PendingUpdate::default());
            settings::init(app.handle());

            if let Some(w) = main_window(app.handle()) {
                native::set_interactive(&w, false);
            }

            let toggle = MenuItem::with_id(app, "toggle", "Show / Hide", true, None::<&str>)?;
            let search = MenuItem::with_id(app, "search", "Search", true, None::<&str>)?;
            let prefs = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Tunelet", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle, &search, &prefs, &quit])?;

            TrayIconBuilder::with_id("tray")
                .icon(app.default_window_icon().cloned().expect("app icon missing"))
                .tooltip("Tunelet")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => toggle_widget(app),
                    "search" => open_panel(app, "open-search"),
                    "settings" => open_panel(app, "open-settings"),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_widget(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hide_widget,
            set_interactive,
            set_bounds,
            screen_area,
            media,
            media_seek,
            mouse_down,
            spotify::spotify_status,
            spotify::spotify_login,
            spotify::spotify_logout,
            spotify::spotify_search,
            spotify::spotify_play,
            spotify::spotify_shuffle,
            lyrics::lyrics,
            spotify::spotify_playlist_tracks,
            spotify::spotify_queue,
            spotify::spotify_player,
            spotify::spotify_set_liked,
            spotify::spotify_toggle_like,
            spotify::spotify_seek,
            spotify::spotify_volume,
            spotify::spotify_devices,
            spotify::spotify_transfer,
            spotify::spotify_playlists,
            settings::get_settings,
            settings::save_settings,
            settings::default_settings,
            settings::suspend_hotkeys,
            settings::set_autostart,
            settings::check_update,
            settings::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tunelet");
}