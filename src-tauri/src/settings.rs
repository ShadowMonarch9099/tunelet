//! User settings (hotkeys, opacity), saved as JSON in the app config folder,
//! plus start-with-Windows (stored by the autostart plugin in the registry).

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Toggle,
    Search,
    PlayPause,
    Next,
    Previous,
    Like,
}

const DEFAULT_HOTKEYS: [(Action, &str); 6] = [
    (Action::Toggle, "Ctrl+Alt+KeyS"),
    (Action::Search, "Ctrl+Alt+KeyF"),
    (Action::PlayPause, "Ctrl+Alt+KeyP"),
    (Action::Next, "Ctrl+Alt+ArrowRight"),
    (Action::Previous, "Ctrl+Alt+ArrowLeft"),
    (Action::Like, "Ctrl+Alt+KeyL"),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Action -> shortcut such as "Ctrl+Alt+KeyS". Empty string = no shortcut.
    pub hotkeys: HashMap<Action, String>,
    /// Widget opacity, 0.4 to 1.0.
    pub opacity: f64,
    /// Start the Spotify app (minimized to the tray) when you press play
    /// and it isn't running.
    pub launch_spotify: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkeys: DEFAULT_HOTKEYS.iter().map(|(a, k)| (*a, k.to_string())).collect(),
            opacity: 1.0,
            launch_spotify: true,
        }
    }
}

#[derive(Default)]
pub struct SettingsState {
    settings: Mutex<Settings>,
    /// Registered shortcut id -> action, used by the global shortcut handler.
    bindings: Mutex<HashMap<u32, Action>>,
}

impl SettingsState {
    pub fn launch_spotify(&self) -> bool {
        self.settings.lock().map(|s| s.launch_spotify).unwrap_or(true)
    }

    pub fn action_for(&self, shortcut: &Shortcut) -> Option<Action> {
        self.bindings.lock().ok()?.get(&shortcut.id()).copied()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyError {
    action: Action,
    combo: String,
    reason: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    settings: Settings,
    autostart: bool,
    version: String,
    hotkey_errors: Vec<HotkeyError>,
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join("settings.json"))
}

fn load(app: &AppHandle) -> Settings {
    let mut s: Settings = path(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    // Fill in any actions added since the file was written.
    for (a, k) in DEFAULT_HOTKEYS {
        s.hotkeys.entry(a).or_insert_with(|| k.to_string());
    }
    s.opacity = s.opacity.clamp(0.4, 1.0);
    s
}

fn save(app: &AppHandle, s: &Settings) -> Result<(), String> {
    let p = path(app).ok_or("No settings folder")?;
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(p, serde_json::to_string_pretty(s).unwrap_or_default()).map_err(|e| e.to_string())
}

/// Registers every hotkey in `s`, replacing whatever was registered before.
fn apply_hotkeys(app: &AppHandle, s: &Settings) -> Vec<HotkeyError> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut map = HashMap::new();
    let mut errors = Vec::new();

    for (action, combo) in &s.hotkeys {
        if combo.trim().is_empty() {
            continue;
        }
        let err = |reason| HotkeyError { action: *action, combo: combo.clone(), reason };
        match combo.parse::<Shortcut>() {
            Ok(sc) if map.contains_key(&sc.id()) => errors.push(err("duplicate")),
            Ok(sc) => match gs.register(sc) {
                Ok(()) => {
                    map.insert(sc.id(), *action);
                }
                Err(_) => errors.push(err("taken")),
            },
            Err(_) => errors.push(err("invalid")),
        }
    }

    if let Some(state) = app.try_state::<SettingsState>() {
        if let Ok(mut b) = state.bindings.lock() {
            *b = map;
        }
    }
    for e in &errors {
        eprintln!("hotkey {:?} ({}) not registered: {}", e.action, e.combo, e.reason);
    }
    errors
}

/// Loads settings and registers hotkeys. Call once from `setup`.
pub fn init(app: &AppHandle) {
    let s = load(app);
    app.manage(SettingsState { settings: Mutex::new(s.clone()), ..Default::default() });
    apply_hotkeys(app, &s);
}

fn view(app: &AppHandle, s: Settings, hotkey_errors: Vec<HotkeyError>) -> SettingsView {
    SettingsView {
        settings: s,
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        version: app.package_info().version.to_string(),
        hotkey_errors,
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle, state: State<'_, SettingsState>) -> SettingsView {
    let s = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    view(&app, s, vec![])
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, SettingsState>, mut settings: Settings) -> Result<SettingsView, String> {
    settings.opacity = settings.opacity.clamp(0.4, 1.0);
    let errors = apply_hotkeys(&app, &settings);
    save(&app, &settings)?;
    if let Ok(mut s) = state.settings.lock() {
        *s = settings.clone();
    }
    Ok(view(&app, settings, errors))
}

#[tauri::command]
pub fn default_settings() -> Settings {
    Settings::default()
}

/// While recording a new shortcut, global hotkeys are switched off so the
/// keys reach the widget instead of triggering actions.
#[tauri::command]
pub fn suspend_hotkeys(app: AppHandle, state: State<'_, SettingsState>, suspend: bool) {
    if suspend {
        let _ = app.global_shortcut().unregister_all();
    } else {
        let s = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
        apply_hotkeys(&app, &s);
    }
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let al = app.autolaunch();
    if enabled { al.enable() } else { al.disable() }.map_err(|e| e.to_string())?;
    al.is_enabled().map_err(|e| e.to_string())
}

// ---- updates ----------------------------------------------------------------

use tauri_plugin_updater::UpdaterExt;

/// Returns the newer version number if an update is available.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<String>, String> {
    let update = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?;
    Ok(update.map(|u| u.version))
}

/// Downloads and installs the update, then restarts Tunelet.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        return Err("Already up to date.".into());
    };
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("Update failed: {e}"))?;
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_and_recorded_hotkeys_parse() {
        for (_, combo) in DEFAULT_HOTKEYS {
            assert!(combo.parse::<Shortcut>().is_ok(), "{combo} should parse");
        }
        // Formats the settings screen produces from KeyboardEvent.code
        for combo in ["Ctrl+Shift+Digit1", "Alt+Super+Space", "Ctrl+Alt+Slash", "Ctrl+Alt+F5", "Ctrl+Alt+BracketLeft"] {
            assert!(combo.parse::<Shortcut>().is_ok(), "{combo} should parse");
        }
    }
}
