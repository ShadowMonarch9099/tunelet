# Testing Tunelet on a Mac

Thanks for testing! This is the first macOS build, so some things may not work yet. Please note what happens at each step (a screenshot or a short screen recording helps a lot).

You need: a Mac with macOS 11 or newer and the **Spotify desktop app** installed. Spotify Premium is only needed for part 3.

## 1. Install

1. Open `Tunelet_..._universal.dmg` and drag **Tunelet** into **Applications**.
2. Open Tunelet from Applications. macOS will block it the first time, because the app isn't signed with a paid Apple developer account:
   - **macOS 15 (Sequoia) or newer:** click **Done**, then open **System Settings → Privacy & Security**, scroll down, and click **Open Anyway** next to Tunelet. Confirm with your password.
   - **Older macOS:** right-click Tunelet in Applications → **Open** → **Open**.
   - If it says the app **"is damaged"**, open **Terminal** and run: `xattr -cr /Applications/Tunelet.app`, then open it again.
3. A round bubble should appear in the middle of the screen. There should be **no Dock icon**, but there should be a Tunelet icon in the **menu bar** (top right).

## 2. Basics (no Spotify login needed)

Play a song in the Spotify app, then check each of these. Write down ✅ or ❌ with a note:

| # | Try this | Expected |
|---|---|---|
| 1 | Look at the bubble | macOS asks "Tunelet wants to control Spotify". Click **OK**. The bubble then shows the album art, spinning while it plays, with a progress ring |
| 2 | Right-click the bubble (or two-finger click) | Play / pause |
| 3 | Click the bubble | A ring of buttons springs out around it |
| 4 | Use ⏮ ▶ ⏭ in the ring | Previous / play-pause / next |
| 5 | Click into another app (e.g. Notes), type something, then click the bubble | Your typing focus stays in Notes |
| 6 | Drag the bubble around (closed, with the ring open, and by a card's title) | It follows the mouse smoothly and stays where you drop it |
| 7 | Switch to another desktop (Space) or a full-screen app | The bubble is still there |
| 8 | Press **⌃⌥S** (Control + Option + S) | Hides / shows the bubble |
| 9 | Press **⌃⌥P**, **⌃⌥→**, **⌃⌥←** | Play/pause, next, previous |
| 10 | Quit Spotify, then right-click the bubble | Spotify starts in the background (no window) and starts playing |
| 11 | Open the ring → 🎤 Lyrics | Lyrics appear and follow the song |
| 12 | Settings (⚙) → **Open at login** on, restart the Mac | Tunelet starts by itself |

## 3. Spotify features (needs Spotify Premium)

1. Follow the "Connect Spotify" steps in Tunelet's search card (create a Client ID at developer.spotify.com, as described there).
2. Then check: search and play a song, open a playlist and play from a song, like a song (❤), shuffle, volume slider, and **Play on** (switching devices).

## 4. Send back

- Your macOS version (Apple menu → About This Mac) and whether it's an Apple Silicon (M1/M2/M3/M4) or Intel Mac.
- The ✅ / ❌ list with notes, and screenshots of anything that looks wrong.
