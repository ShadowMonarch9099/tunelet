use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WebviewWindow,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

mod colors;
mod media;

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
    use tauri::WebviewWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE, SW_SHOWNOACTIVATE,
        WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    fn hwnd(w: &WebviewWindow) -> Option<*mut core::ffi::c_void> {
        w.hwnd().ok().map(|h| h.0 as _)
    }

    /// `interactive = false`: clicks never activate the window (default).
    /// `interactive = true`: window can take keyboard focus (e.g. typing in search).
    pub fn set_interactive(w: &WebviewWindow, interactive: bool) {
        let Some(h) = hwnd(w) else { return };
        unsafe {
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
    let shortcuts = [toggle_shortcut, play_shortcut, next_shortcut, prev_shortcut];

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
                    }
                })
                .build(),
        )
        .setup(move |app| {
            app.manage(media::start(app.handle().clone()));

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
            let quit = MenuItem::with_id(app, "quit", "Quit Tunelet", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle, &quit])?;

            TrayIconBuilder::with_id("tray")
                .icon(app.default_window_icon().cloned().expect("app icon missing"))
                .tooltip("Tunelet")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => toggle_widget(app),
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
        .invoke_handler(tauri::generate_handler![hide_widget, set_interactive, media])
        .run(tauri::generate_context!())
        .expect("error while running Tunelet");
}
