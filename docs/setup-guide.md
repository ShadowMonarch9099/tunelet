# Tunelet setup guide (start to finish)

About 5 minutes from zero to a working widget.

## What you need

- A **Windows 10 or 11** PC
- The **Spotify desktop app**, installed and logged in
- **Spotify Premium**, for search, playlists, likes, shuffle, volume and switching devices. Play/pause, skipping, lyrics and now-playing work on any account.

---

## Part 1: Download and install Tunelet

1. Open **https://github.com/ShadowMonarch9099/tunelet/releases/latest**
2. Scroll down to **Assets** and download the right installer:
   - **Most PCs (Intel / AMD):** `Tunelet_x.y.z_x64-setup.exe`
   - **Snapdragon / ARM laptops:** `Tunelet_x.y.z_arm64-setup.exe`

   Not sure which you have? Open **Settings → System → About** and look at **System type**: "x64-based processor" means x64, "ARM-based processor" means arm64.
3. Run the downloaded file.
   - If Windows says **"Windows protected your PC"**, click **More info → Run anyway**. Tunelet isn't signed with a paid certificate, which is why Windows warns.
   - It installs just for you, with no admin password needed.
4. A round **bubble** appears on your screen, with a Tunelet icon in the system tray (bottom right).

Play a song in Spotify: the bubble shows the album art and spins. **Right-click** it to play/pause. The basics already work.

---

## Part 2: Create your Spotify Client ID

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

1. Open Tunelet's search: press **`Ctrl+Alt+F`**, or click the bubble and then 🔍.
2. Paste your **Client ID** into the box and click **Connect**.
3. Your browser opens a Spotify page. Click **Agree**.
4. The page says **"Connected to Tunelet"**. Close the tab.

Done. Tunelet remembers the login, so you only do this once.

---

## Part 4: Make it yours (optional)

Open **Settings**: click the bubble, then ⚙.

- **Start with Windows:** turn it on so Tunelet is always there.
- **Hotkeys:** click one and press new keys. Avoid plain `Ctrl+letter` combos like `Ctrl+S` or `Ctrl+C`, because they'd stop working as Save/Copy in every app.
- **Opacity:** make the widget see-through.
- **Start Spotify when needed:** with this on, pressing play when Spotify is closed starts it in the background.

---

## How to use it

**Mouse**

| Do this | What happens |
|---|---|
| Click the bubble | Opens the menu around it |
| Right-click the bubble | Play / pause |
| Drag the bubble or a card's title | Moves the widget (it remembers the spot) |

**Menu buttons:** ⏮ ▶ ⏭ · ❤ like · 🔍 search · playlists · now playing · 🎤 lyrics · ⚙ settings

**Default hotkeys** (work from any app)

| Keys | Action |
|---|---|
| `Ctrl+Alt+S` | Show / hide Tunelet |
| `Ctrl+Alt+F` | Search |
| `Ctrl+Alt+P` | Play / pause |
| `Ctrl+Alt+→` / `←` | Next / previous (hold to keep skipping) |
| `Ctrl+Alt+L` | Like / unlike the song |
| `Esc` | Close the menu or card |

**In search:** `↑` `↓` to choose, `Enter` to play, `Shift+Enter` to add to the queue, `Tab` to switch to your playlists. Click a playlist to see its songs; click a song to play from there.

---

## Updates

You never need to reinstall. When a new version is out, a **green dot** appears on ⚙ Settings. Open Settings and click **Update**, and Tunelet updates and restarts itself.

---

## Troubleshooting

| Problem | Fix |
|---|---|
| Browser shows **"INVALID_CLIENT: Invalid redirect URI"** | The redirect address in your Spotify app is missing or mistyped. Dashboard → your app → **Settings → Edit**, set it to exactly `http://127.0.0.1:43821/callback`, **Save**, then click **Connect** again. |
| **"Spotify Premium is required"** | Spotify only lets apps control playback on Premium accounts. Play/pause/skip from the bubble still work. |
| **"Spotify refused that…"** | You approved with a different Spotify account than the one that made the Client ID. Use the same account, or add the other account under **User Management** in your app's settings on the dashboard. |
| **"No Spotify device found"** | Open the Spotify app (or turn on **Start Spotify when needed** in Settings) and try again. |
| Bubble shows **"Nothing playing"** | Start a song in the Spotify desktop app. Tunelet follows the Spotify app on this PC. |
| A hotkey doesn't work | Another app is using it. Settings shows a warning next to it; pick different keys. |
| Lost the widget | Press `Ctrl+Alt+S`, or click the Tunelet icon in the system tray. |

---

*macOS: a Mac version is being tested. It will appear on the same download page as `Tunelet_x.y.z_universal.dmg` once it's ready.*
