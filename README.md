<p align="center"><img src="src-tauri/icons/128x128@2x.png" width="96" alt="Tunelet icon" /></p>

# Tunelet

A tiny, always-on-top Spotify mini-player for Windows: a round bubble with the album art that floats over whatever you're working in. Click it for a menu that fans out around it, right-click to play/pause, and search, like and skip songs without switching to the Spotify app.

## Features

- **Floating bubble** — album art that spins like a vinyl while playing, with a progress ring; drag it anywhere
- **Radial menu** — click the bubble and the options spring out around it: play/skip, like, search, playlists, now playing, settings
- **Album-adaptive look** — the glow and every card shift to the current cover's colors
- **Doesn't get in your way** — clicking it keeps focus in your current app; hidden from the taskbar and Alt+Tab
- **No need to open Spotify** — press play and Tunelet starts the Spotify app minimized to the tray, then plays (turn off in Settings)
- **Search anything** — play or queue songs, albums and playlists from a card that opens beside the bubble
- **Your playlists** — Liked Songs and all your playlists, one keypress away; open one to see its songs and play from any of them
- **Like songs** — heart button or `Ctrl+Alt+L` from any app
- **Now playing** — seek, shuffle, Spotify volume, and move playback to your phone or speakers
- **Lyrics** — synced lyrics that follow the song (click a line to jump there), from [LRCLIB](https://lrclib.net)
- **Global hotkeys** — work from any app, and you can change them
- **Settings** — hotkeys, opacity, start with Windows
- **Updates itself** — new versions download in the background and install from inside the app
- **Tray icon** — show/hide, search, settings, quit

## Install

**New here? Follow the [step-by-step setup guide](docs/setup-guide.md)** for Windows and Mac: download, Spotify Client ID, and connecting, start to finish.

Download the latest installer from the [Releases](../../releases/latest) page:

- **Most PCs (Intel / AMD):** `Tunelet_x.y.z_x64-setup.exe`
- **Snapdragon / ARM PCs:** `Tunelet_x.y.z_arm64-setup.exe`

Windows may show **"Windows protected your PC"** because the app isn't code-signed. Click **More info → Run anyway**. No admin rights needed.

After installing, open **Settings** (tray icon → Settings) and turn on **Start with Windows**.

### Spotify setup (for search)

Play/pause/skip and now-playing work with no setup. Searching and picking songs needs **Spotify Premium** and a one-time login. Because Spotify limits how many people can use one developer app, each user connects their own (free, ~2 minutes):

1. Go to [developer.spotify.com/dashboard](https://developer.spotify.com/dashboard) and click **Create app** (any name and description).
2. Under **Redirect URIs**, paste `http://127.0.0.1:43821/callback` and click **Add**. Tick **Web API**, and save.
3. Copy the app's **Client ID** (not the secret). Open Tunelet's search (`Ctrl+Alt+F`), paste it, and click **Connect**. Approve the login in your browser.

No client secret is needed — Tunelet uses the PKCE login flow, and tokens are stored in Windows Credential Manager.

## Mouse

| | |
|---|---|
| Click the bubble | Open / close the menu |
| Right-click | Play / pause (starts Spotify in the background if it's closed) |
| Drag | Move it (it remembers where) |
| Hover | Shows play/pause and the song name |

## Hotkeys

Defaults — change any of them in Settings.

| Shortcut | Action |
|---|---|
| `Ctrl+Alt+S` | Show / hide the widget |
| `Ctrl+Alt+F` | Show and jump to search |
| `Ctrl+Alt+P` | Play / pause |
| `Ctrl+Alt+←` / `→` | Previous / next (hold to keep skipping) |
| `Ctrl+Alt+L` | Like / unlike the current song |
| `Esc` | Close the menu or card / hide the widget |
| `↑` `↓` `Enter` | In search: choose and play (`Shift+Enter` adds a song to the queue) |
| `Tab` | In search: switch between Search and Your playlists |

## Build from source

Prerequisites: [Node.js 20+](https://nodejs.org), [Rust](https://rustup.rs), and the [Tauri Windows prerequisites](https://tauri.app/start/prerequisites/) (Microsoft C++ Build Tools + WebView2).

```powershell
.\scripts\dev.ps1           # run in development (installs npm deps on first run)
.\scripts\dev.ps1 build     # build installers into src-tauri/target/release/bundle
```

The script finds Cargo and the Windows SDK's `rc.exe` even when they aren't on `PATH`. Works on x64 and ARM64 (Snapdragon) PCs. If PowerShell blocks the script, run it with `powershell -ExecutionPolicy Bypass -File .\scripts\dev.ps1`.

Release builds sign update files with the key in `%USERPROFILE%\.tauri\tunelet.key` (or the `TAURI_SIGNING_PRIVATE_KEY` environment variable). Pushing a `v*` tag builds x64 and ARM64 installers on GitHub Actions and drafts a release.

## Roadmap

- [x] Floating frameless window, always on top, no focus stealing, tray icon
- [x] Now playing + play/pause/skip via Windows media controls
- [x] Album-adaptive colors
- [x] Spotify login (bring-your-own Client ID, PKCE)
- [x] Search, play, queue
- [x] Settings: custom hotkeys, opacity, start with Windows
- [x] x64 + ARM64 installers, in-app updates
- [x] Like button, your playlists, seek, volume, device switch
- [x] Round bubble with radial menu and side cards

## Tech

[Tauri 2](https://tauri.app) (Rust) + [Svelte 5](https://svelte.dev) + TypeScript.

## License

[MIT](LICENSE)
