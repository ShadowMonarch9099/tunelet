# Tunelet setup guide (start to finish)

About 5 minutes from zero to a working widget, on **Windows** or **Mac**.

## What you need

- A **Windows 10 or 11** PC, or a **Mac with macOS 11 (Big Sur) or newer** (Apple Silicon or Intel)
- The **Spotify desktop app**, installed and logged in
- **Spotify Premium**, for search, playlists, likes, shuffle, volume and switching devices. Play/pause, skipping, lyrics and now-playing work on any account.

---

## Part 1: Download and install Tunelet

Open **https://github.com/ShadowMonarch9099/tunelet/releases/latest** and scroll down to **Assets**.

### Windows

1. Download the right installer:
   - **Most PCs (Intel / AMD):** `Tunelet_x.y.z_x64-setup.exe`
   - **Snapdragon / ARM laptops:** `Tunelet_x.y.z_arm64-setup.exe`

   Not sure which you have? Open **Settings → System → About** and look at **System type**: "x64-based processor" means x64, "ARM-based processor" means arm64.
2. Run the downloaded file.
   - If Windows says **"Windows protected your PC"**, click **More info → Run anyway**. Tunelet isn't signed with a paid certificate, which is why Windows warns.
   - It installs just for you, with no admin password needed.
3. A round **bubble** appears on your screen, with a Tunelet icon in the system tray (bottom right).

### Mac

> The Mac version is in testing and not on the download page yet. Until it is, get `Tunelet_..._universal.dmg` from whoever shared Tunelet with you; the steps are the same.

1. Download `Tunelet_x.y.z_universal.dmg`. It's one file for every Mac, Apple Silicon (M1–M4) and Intel.
2. Open the `.dmg` and drag **Tunelet** into **Applications**.
3. Open **Tunelet** from Applications. The first time, macOS blocks it because it isn't signed with a paid Apple developer account:
   - **macOS 15 (Sequoia) or newer:** click **Done**, open **System Settings → Privacy & Security**, scroll down, click **Open Anyway** next to Tunelet, and confirm with your password.
   - **macOS 14 or older:** right-click Tunelet in Applications → **Open** → **Open**.
   - If macOS says the app **"is damaged and can't be opened"**, open **Terminal**, run `xattr -cr /Applications/Tunelet.app`, and open it again.

   You only do this once.
4. A round **bubble** appears on your screen, with a Tunelet icon in the **menu bar** (top right). There's no Dock icon; that's on purpose.
5. Play a song in Spotify. macOS asks **"Tunelet wants access to control Spotify"**. Click **OK**. Tunelet needs this to show what's playing and to play, pause and skip.

### Both

Play a song in Spotify: the bubble shows the album art and spins. **Right-click** it to play/pause (on a Mac trackpad: two-finger click, or Control-click). The basics already work.

---

## Part 2: Create your Spotify Client ID

*Same on Windows and Mac.*

Tunelet needs a free "Client ID" from Spotify to search and control your account. Everyone makes their own; it's free and takes about 2 minutes.

1. Go to **https://developer.spotify.com/dashboard** and log in with your **normal Spotify account**.
   - The first time, accept the developer terms and verify your email if Spotify asks.
2. Click **Create app**.
3. Fill in the form:

   | Field | What to enter |
   |---|---|
   | **App name** | Anything, e.g. `Tunelet` |
   | **App description** | Anything, e.g. `My mini player` |
   | **Website** | Leave empty |
   | **Redirect URIs** | `http://127.0.0.1:43821/callback`, then click **Add** |

   - Put the address in the **Redirect URIs** box, not in App name, and click **Add**. It should appear in a list under the box.
   - Type it exactly: `127.0.0.1` (not `localhost`), port `43821`, ending in `/callback`.
4. Under **Which API/SDKs are you planning to use?**, tick **Web API**.
5. Tick the terms checkbox and click **Save**.
6. On your new app's page, open **Settings** and copy the **Client ID** (32 letters and numbers).
   - You don't need the **Client secret**. Tunelet never uses it; don't share it.

---

## Part 3: Connect Tunelet to Spotify

1. Open Tunelet's search:
   - **Windows:** press **`Ctrl+Alt+F`**
   - **Mac:** press **`⌃⌥F`** (Control + Option + F)
   - or, on either: click the bubble, then 🔍
2. Paste your **Client ID** into the box and click **Connect**.
3. Your browser opens a Spotify page. Click **Agree**.
4. The page says **"Connected to Tunelet"**. Close the tab.

Done. Tunelet remembers the login (in Windows Credential Manager or the Mac Keychain), so you only do this once.

---

## Part 4: Make it yours (optional)

Open **Settings**: click the bubble, then ⚙.

- **Start with Windows** (Windows) / **Open at login** (Mac): turn it on so Tunelet is always there.
- **Hotkeys:** click one and press new keys.
  - Windows: avoid plain `Ctrl+letter` combos like `Ctrl+S` or `Ctrl+C`; they'd stop working as Save/Copy in every app.
  - Mac: likewise, avoid plain `⌘+letter` combos like `⌘S` or `⌘C`.
- **Opacity:** make the widget see-through.
- **Start Spotify when needed:** with this on, pressing play when Spotify is closed starts it in the background.

---

## How to use it

**Mouse / trackpad**

| Do this | What happens |
|---|---|
| Click the bubble | Opens the menu around it |
| Right-click the bubble (Mac: two-finger click or Control-click) | Play / pause |
| Drag the bubble or a card's title | Moves the widget (it remembers the spot) |

**Menu buttons:** ⏮ ▶ ⏭ · ❤ like · 🔍 search · playlists · now playing · 🎤 lyrics · ⚙ settings

**Default hotkeys** (work from any app)

| Windows | Mac | Action |
|---|---|---|
| `Ctrl+Alt+S` | `⌃⌥S` | Show / hide Tunelet |
| `Ctrl+Alt+F` | `⌃⌥F` | Search |
| `Ctrl+Alt+P` | `⌃⌥P` | Play / pause |
| `Ctrl+Alt+→` / `←` | `⌃⌥→` / `←` | Next / previous (hold to keep skipping) |
| `Ctrl+Alt+L` | `⌃⌥L` | Like / unlike the song |
| `Esc` | `Esc` | Close the menu or card |

(On a Mac, ⌃ is **Control** and ⌥ is **Option**.)

**In search:** `↑` `↓` to choose, `Enter` (Mac: `Return`) to play, `Shift+Enter` to add to the queue, `Tab` to switch to your playlists. Click a playlist to see its songs; click a song to play from there.

---

## Updates

You never need to reinstall, on Windows or Mac. When a new version is out, a **green dot** appears on ⚙ Settings. Open Settings and click **Update**, and Tunelet updates and restarts itself.

---

## Troubleshooting

**Both**

| Problem | Fix |
|---|---|
| Browser shows **"INVALID_CLIENT: Invalid redirect URI"** | The redirect address in your Spotify app is missing or mistyped. Dashboard → your app → **Settings → Edit**, set it to exactly `http://127.0.0.1:43821/callback`, **Save**, then click **Connect** again. |
| **"Spotify Premium is required"** | Spotify only lets apps control playback on Premium accounts. Play/pause/skip from the bubble still work. |
| **"Spotify refused that…"** | You approved with a different Spotify account than the one that made the Client ID. Use the same account, or add the other account under **User Management** in your app's settings on the dashboard. |
| **"No Spotify device found"** | Open the Spotify app (or turn on **Start Spotify when needed** in Settings) and try again. |
| Bubble shows **"Nothing playing"** | Start a song in the Spotify desktop app. Tunelet follows the Spotify app on this computer. |
| A hotkey doesn't work | Another app is using it. Settings shows a warning next to it; pick different keys. |

**Windows**

| Problem | Fix |
|---|---|
| Lost the widget | Press `Ctrl+Alt+S`, or click the Tunelet icon in the system tray. |

**Mac**

| Problem | Fix |
|---|---|
| "Tunelet can't be opened" / "Apple could not verify…" | See Part 1, step 3: **Privacy & Security → Open Anyway** (macOS 15+) or right-click → **Open** (older). |
| "Tunelet is damaged and can't be opened" | In Terminal: `xattr -cr /Applications/Tunelet.app`, then open it again. |
| Bubble stays on **"Nothing playing"** while Spotify plays | You may have clicked **Don't Allow** on the "control Spotify" prompt. Open **System Settings → Privacy & Security → Automation → Tunelet** and turn on **Spotify**, then restart Tunelet. |
| Lost the widget | Press `⌃⌥S`, or click the Tunelet icon in the menu bar. |
