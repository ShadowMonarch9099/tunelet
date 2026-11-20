# Tunelet

A tiny, always-on-top Spotify mini-player for Windows. It floats over whatever you're working in, never steals focus, and hides or appears with a keyboard shortcut — so you can play, skip and search songs without switching to the Spotify app.

> **Status: early development (v0.1).** Now playing, controls, album-adaptive colors and search all work. See the [roadmap](#roadmap).

## Features (planned for v1)

- **Floating pill** — album art, song, artist, play / pause / skip, always on top
- **Album-adaptive look** — the widget's colors shift to match the current cover
- **Doesn't get in your way** — clicking it keeps focus in your current app; hidden from the taskbar and Alt+Tab
- **Search anything** — expand the pill to search Spotify and play or queue songs, albums and playlists
- **Global hotkeys** — work from any app
- **Tray icon** — show/hide, settings, start with Windows, quit

## Hotkeys

| Shortcut | Action |
|---|---|
| `Ctrl+Alt+S` | Show / hide the widget |
| `Ctrl+Alt+F` | Show and jump to search |
| `Ctrl+Alt+P` | Play / pause |
| `Ctrl+Alt+←` / `→` | Previous / next |
| `Ctrl+Alt+L` | Like current song *(coming)* |
| `Esc` | Close search / hide the widget |
| `↑` `↓` `Enter` | In search: choose and play (`Shift+Enter` adds a song to the queue) |

## Install

Download the latest installer from the [Releases](../../releases) page and run it.

Windows may show **"Windows protected your PC"** because the app isn't code-signed yet. Click **More info → Run anyway**.

### Spotify setup (for search)

Play/pause/skip and now-playing work with no setup. Searching and picking songs needs **Spotify Premium** and a one-time login. Because Spotify limits how many people can use one developer app, each user connects their own (free, ~2 minutes):

1. Go to [developer.spotify.com/dashboard](https://developer.spotify.com/dashboard) and click **Create app** (any name and description).
2. Under **Redirect URIs**, add `http://127.0.0.1:43821/callback`, tick **Web API**, and save.
3. Open Tunelet's search (`Ctrl+Alt+F`), paste the app's **Client ID**, and click **Connect**. Approve the login in your browser.

No client secret is needed — Tunelet uses the PKCE login flow, and tokens are stored in Windows Credential Manager.

## Build from source

Prerequisites: [Node.js 20+](https://nodejs.org), [Rust](https://rustup.rs), and the [Tauri Windows prerequisites](https://tauri.app/start/prerequisites/) (Microsoft C++ Build Tools + WebView2).

```powershell
.\scripts\dev.ps1           # run in development (installs npm deps on first run)
.\scripts\dev.ps1 build     # build installers into src-tauri/target/release/bundle
```

The script finds Cargo and the Windows SDK's `rc.exe` even when they aren't on `PATH`. Works on x64 and ARM64 (Snapdragon) PCs. If PowerShell blocks the script, run it with `powershell -ExecutionPolicy Bypass -File .\scripts\dev.ps1`.

## Roadmap

- [x] Project setup, CI builds
- [x] Floating frameless window, always on top, no focus stealing, show/hide hotkey, tray icon
- [x] Now playing + play/pause/skip via Windows media controls
- [x] Album-adaptive colors
- [x] Spotify login (bring-your-own Client ID, PKCE)
- [x] Expanded mode: search, play, queue
- [ ] Like, playlists, seek, volume, device switch, settings
- [ ] v0.1.0 release with auto-updater

## Tech

[Tauri 2](https://tauri.app) (Rust) + [Svelte 5](https://svelte.dev) + TypeScript.

## License

[MIT](LICENSE)
