use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WebviewWindow,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

mod colors;
mod media;
mod spotify;

use media::{Cmd, Media};

const MAIN: &str = "main";

fn hotkey(code: Code) -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), code)
}

fn send_media(app: &AppHandle, cmd: Cmd) {
    if let Some(m) = app.try_state::<Media>() {
        m.send(cmd);
    }
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
        GetForegroundWindow, GetWindowLongPtrW, SetForegroundWindow, SetWindowLongPtrW, ShowWindow,
        GWL_EXSTYLE, SW_SHOWNOACTIVATE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
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
}

#[cfg(not(windows))]
mod native {
    use tauri::WebviewWindow;
    pub fn set_interactive(_w: &WebviewWindow, _interactive: bool) {}
    pub fn show_no_activate(w: &WebviewWindow) {
        let _ = w.show();
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
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            show_widget(app);
        }
    }
}

#[tauri::command]
fn hide_widget(app: AppHandle) {
    if let Some(w) = main_window(&app) {
        let _ = w.hide();
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

/// Transport controls from the UI: "toggle", "next", "previous" or "refresh".
#[tauri::command]
fn media(app: AppHandle, action: String) {
    let cmd = match action.as_str() {
        "toggle" => Cmd::TogglePlay,
        "next" => Cmd::Next,
        "previous" => Cmd::Previous,
        _ => Cmd::Refresh,
    };
    send_media(&app, cmd);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let toggle_shortcut = hotkey(Code::KeyS);
    let play_shortcut = hotkey(Code::KeyP);
    let next_shortcut = hotkey(Code::ArrowRight);
    let prev_shortcut = hotkey(Code::ArrowLeft);
    let search_shortcut = hotkey(Code::KeyF);
    let shortcuts = [toggle_shortcut, play_shortcut, next_shortcut, prev_shortcut, search_shortcut];

    tauri::Builder::default()
        // A second launch just shows the running widget instead of opening another.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_widget(app);
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(tauri_plugin_window_state::StateFlags::POSITION)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    if shortcut == &toggle_shortcut {
                        toggle_widget(app);
                    } else if shortcut == &play_shortcut {
                        send_media(app, Cmd::TogglePlay);
                    } else if shortcut == &next_shortcut {
                        send_media(app, Cmd::Next);
                    } else if shortcut == &prev_shortcut {
                        send_media(app, Cmd::Previous);
                    } else if shortcut == &search_shortcut {
                        show_widget(app);
                        let _ = app.emit("open-search", ());
                    }
                })
                .build(),
        )
        .setup(move |app| {
            app.manage(media::start(app.handle().clone()));
            app.manage(spotify::Spotify::new());

            for s in shortcuts {
                // Another app may already own a combo; keep going with the rest.
                if let Err(e) = app.global_shortcut().register(s) {
                    eprintln!("could not register hotkey {s:?}: {e}");
                }
            }

            if let Some(w) = main_window(app.handle()) {
                native::set_interactive(&w, false);
            }

            let toggle = MenuItem::with_id(app, "toggle", "Show / Hide   (Ctrl+Alt+S)", true, None::<&str>)?;
            let search = MenuItem::with_id(app, "search", "Search   (Ctrl+Alt+F)", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Tunelet", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle, &search, &quit])?;

            TrayIconBuilder::with_id("tray")
                .icon(app.default_window_icon().cloned().expect("app icon missing"))
                .tooltip("Tunelet")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => toggle_widget(app),
                    "search" => {
                        show_widget(app);
                        let _ = app.emit("open-search", ());
                    }
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
            media,
            spotify::spotify_status,
            spotify::spotify_login,
            spotify::spotify_logout,
            spotify::spotify_search,
            spotify::spotify_play,
            spotify::spotify_queue,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tunelet");
}
