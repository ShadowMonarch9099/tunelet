//! Starts the Spotify app in the background when Tunelet needs it, so you
//! never have to open Spotify yourself. Windows: the installer version
//! (%APPDATA%\Spotify) or the Microsoft Store version, minimized to the tray.
//! macOS: launched hidden with `open`.

#[cfg(windows)]
use std::path::PathBuf;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

/// A launch counts as "in progress" for this long, so pressing play again
/// while Spotify is still starting doesn't start a second copy.
const STARTUP_GRACE: Duration = Duration::from_secs(20);

static LAST_LAUNCH: Mutex<Option<Instant>> = Mutex::new(None);

/// True while Spotify is running, or was just launched and is still starting.
pub fn is_running_or_starting() -> bool {
    is_running() || recently_launched()
}

fn recently_launched() -> bool {
    LAST_LAUNCH
        .lock()
        .ok()
        .and_then(|g| *g)
        .is_some_and(|t| t.elapsed() < STARTUP_GRACE)
}

#[cfg(windows)]
pub fn is_running() -> bool {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
        },
    };
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = false;
        let mut ok = Process32FirstW(snap, &mut entry) != 0;
        while ok {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..len]).eq_ignore_ascii_case("spotify.exe") {
                found = true;
                break;
            }
            ok = Process32NextW(snap, &mut entry) != 0;
        }
        CloseHandle(snap);
        found
    }
}

#[cfg(target_os = "macos")]
pub fn is_running() -> bool {
    std::process::Command::new("pgrep")
        .args(["-x", "Spotify"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn is_running() -> bool {
    true
}

#[cfg(windows)]
/// Where Spotify.exe can live. The WindowsApps one is the Store version's
/// "app execution alias", which accepts command-line flags like a normal exe.
fn candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        v.push(PathBuf::from(appdata).join(r"Spotify\Spotify.exe"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        v.push(PathBuf::from(local).join(r"Microsoft\WindowsApps\Spotify.exe"));
    }
    v
}

/// Starts Spotify minimized to the tray (the same flag Spotify's own
/// "open at login, minimized" option uses). Does nothing if it's running.
#[cfg(windows)]
pub fn launch_hidden() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const DETACHED_PROCESS: u32 = 0x0000_0008;

    if is_running_or_starting() {
        return Ok(());
    }
    for exe in candidates() {
        // The Store alias is a zero-byte link, so check that it exists, not its size.
        if std::fs::symlink_metadata(&exe).is_err() {
            continue;
        }
        let started = Command::new(&exe)
            .args(["--minimized"])
            .creation_flags(DETACHED_PROCESS)
            .spawn();
        if started.is_ok() {
            if let Ok(mut g) = LAST_LAUNCH.lock() {
                *g = Some(Instant::now());
            }
            return Ok(());
        }
    }
    Err("Couldn't find the Spotify app. Install it from spotify.com or the Microsoft Store.".into())
}

/// Starts Spotify in the background (`-g`) and hidden (`-j`).
#[cfg(target_os = "macos")]
pub fn launch_hidden() -> Result<(), String> {
    if is_running_or_starting() {
        return Ok(());
    }
    let ok = std::process::Command::new("open")
        .args(["-g", "-j", "-a", "Spotify"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        return Err("Couldn't find the Spotify app. Install it from spotify.com.".into());
    }
    if let Ok(mut g) = LAST_LAUNCH.lock() {
        *g = Some(Instant::now());
    }
    Ok(())
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn launch_hidden() -> Result<(), String> {
    Ok(())
}
