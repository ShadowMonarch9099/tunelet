// Shared app state and actions used by the bob, the radial menu and the cards.

import { invoke } from "@tauri-apps/api/core";
import type {
  Action,
  AuthStatus,
  Device,
  Item,
  NowPlaying,
  PlayerState,
  Settings,
  SettingsView,
  Timeline,
  UpdateState,
} from "./types";

const IDLE: [string, string] = ["#3a2a5c", "#12324a"];

class App {
  // ---- now playing (Windows media controls, no login needed) ---------------
  np = $state<NowPlaying | null>(null);
  tl = $state<(Timeline & { at: number }) | null>(null);
  clock = $state(performance.now());

  available = $derived(!!this.np?.available);
  title = $derived(this.np?.available ? this.np.title : "Nothing playing");
  artist = $derived(this.np?.available ? this.np.artist : "Open Spotify to start");
  playing = $derived(this.np?.available ? this.np.playing : false);
  art = $derived(this.np?.available ? this.np.art : null);
  colors = $derived<[string, string]>(this.np?.available && this.np.colors ? this.np.colors : IDLE);

  durationMs = $derived(this.tl?.durationMs ?? 0);
  positionMs = $derived.by(() => {
    const tl = this.tl;
    if (!tl) return 0;
    const p = tl.playing ? tl.positionMs + (this.clock - tl.at) : tl.positionMs;
    return Math.min(p, tl.durationMs);
  });
  progress = $derived(this.durationMs > 0 ? this.positionMs / this.durationMs : 0);

  // ---- Spotify account / player (Web API) -----------------------------------
  auth = $state<AuthStatus | null>(null);
  player = $state<PlayerState | null>(null);
  liked = $state<boolean | null>(null);
  likeBusy = $state(false);
  volume = $state<number | null>(null);
  #volumeTimer: ReturnType<typeof setTimeout> | undefined;
  devices = $state<Device[] | null>(null);

  // ---- search / playlists ----------------------------------------------------
  query = $state("");
  results = $state<Item[]>([]);
  searching = $state(false);
  playlists = $state<Item[] | null>(null);
  playlistsLoading = $state(false);
  #searchTimer: ReturnType<typeof setTimeout> | undefined;
  #searchSeq = 0;

  // ---- settings ----------------------------------------------------------------
  sv = $state<SettingsView | null>(null);
  /** Which hotkey is being re-recorded in Settings (global hotkeys are paused meanwhile). */
  recording = $state<Action | null>(null);
  update = $state<UpdateState>({ state: "idle" });
  opacity = $derived(this.sv?.settings.opacity ?? 1);

  // ---- feedback ----------------------------------------------------------------
  error = $state("");
  toast = $state("");
  #toastTimer: ReturnType<typeof setTimeout> | undefined;
  /** Bumped on each like/unlike so the bob can play its heart animation. */
  likePulse = $state(0);

  /** Set by the page: closes whatever is open and returns to the bob. */
  onRequestClose: () => void = () => {};

  // ---- feedback helpers ------------------------------------------------------
  showToast(msg: string) {
    this.toast = msg;
    clearTimeout(this.#toastTimer);
    this.#toastTimer = setTimeout(() => (this.toast = ""), 1900);
  }

  handleError(e: unknown) {
    const msg = String(e);
    if (msg === "not_logged_in") {
      this.error = "";
      this.loadAuth();
    } else {
      this.error = msg;
    }
  }

  // ---- playback ----------------------------------------------------------------
  media(action: "toggle" | "next" | "previous") {
    // Flip the icon right away; the real state arrives a moment later.
    if (action === "toggle" && this.np?.available) {
      this.np = { ...this.np, playing: !this.np.playing };
      if (this.tl) this.tl = { ...this.tl, positionMs: this.positionMs, playing: !this.tl.playing, at: performance.now() };
    }
    invoke("media", { action });
  }

  async seek(ms: number) {
    if (this.tl) this.tl = { ...this.tl, positionMs: ms, at: performance.now() };
    try {
      if (this.auth?.loggedIn) await invoke("spotify_seek", { positionMs: ms });
      else await invoke("media_seek", { positionMs: ms });
    } catch {
      await invoke("media_seek", { positionMs: ms });
    }
  }

  async refreshPlayer() {
    if (!this.auth?.loggedIn) return;
    try {
      const p = await invoke<PlayerState>("spotify_player");
      this.player = p;
      this.liked = p.liked;
      if (p.device?.volume != null && this.#volumeTimer === undefined) this.volume = p.device.volume;
    } catch (e) {
      if (String(e) === "not_logged_in") this.loadAuth();
    }
  }

  setVolume(v: number) {
    this.volume = v;
    clearTimeout(this.#volumeTimer);
    this.#volumeTimer = setTimeout(async () => {
      this.#volumeTimer = undefined;
      try {
        await invoke("spotify_volume", { percent: v });
      } catch (e) {
        this.handleError(e);
      }
    }, 150);
  }

  async loadDevices() {
    try {
      this.devices = await invoke<Device[]>("spotify_devices");
    } catch (e) {
      this.handleError(e);
    }
  }

  async transfer(d: Device) {
    if (d.active) return;
    try {
      await invoke("spotify_transfer", { deviceId: d.id });
      this.showToast(`Playing on ${d.name}`);
      setTimeout(() => {
        this.refreshPlayer();
        this.loadDevices();
      }, 900);
    } catch (e) {
      this.handleError(e);
    }
  }

  async toggleLike() {
    if (this.likeBusy) return;
    if (!this.auth?.loggedIn) return this.showToast("Connect Spotify to like songs");
    this.likeBusy = true;
    try {
      if (this.player?.trackUri?.startsWith("spotify:track:") && this.liked !== null) {
        const next = !this.liked;
        this.liked = next;
        await invoke("spotify_set_liked", { uri: this.player.trackUri, liked: next });
      } else {
        this.liked = await invoke<boolean>("spotify_toggle_like");
      }
      this.likePulse++;
      this.showToast(this.liked ? "Added to Liked Songs" : "Removed from Liked Songs");
    } catch (e) {
      await this.refreshPlayer();
      this.showToast(String(e) === "not_logged_in" ? "Connect Spotify to like songs" : String(e));
    } finally {
      this.likeBusy = false;
    }
  }

  /** Result of the like hotkey, which runs in Rust. */
  likeResult(r: { liked?: boolean; error?: string }) {
    if (r.error) {
      this.showToast(r.error === "not_logged_in" ? "Connect Spotify to like songs" : r.error);
      return;
    }
    this.liked = !!r.liked;
    this.likePulse++;
  }

  // ---- search / playlists ------------------------------------------------------
  onQueryInput(tab: "search" | "playlists") {
    clearTimeout(this.#searchTimer);
    this.error = "";
    if (tab === "playlists") return; // filtered locally
    if (!this.query.trim()) {
      this.results = [];
      this.searching = false;
      return;
    }
    this.searching = true;
    this.#searchTimer = setTimeout(() => this.#runSearch(), 280);
  }

  async #runSearch() {
    const seq = ++this.#searchSeq;
    try {
      const r = await invoke<Item[]>("spotify_search", { query: this.query });
      if (seq !== this.#searchSeq) return; // a newer search is on its way
      this.results = r;
    } catch (e) {
      if (seq === this.#searchSeq) this.handleError(e);
    } finally {
      if (seq === this.#searchSeq) this.searching = false;
    }
  }

  async loadPlaylists(force = false) {
    if (this.playlists && !force) return;
    this.playlistsLoading = true;
    try {
      this.playlists = await invoke<Item[]>("spotify_playlists");
    } catch (e) {
      this.handleError(e);
    } finally {
      this.playlistsLoading = false;
    }
  }

  async play(item: Item) {
    try {
      await invoke("spotify_play", { uri: item.uri });
      this.showToast(`Playing ${item.name}`);
      setTimeout(() => this.onRequestClose(), 450);
    } catch (e) {
      this.handleError(e);
    }
  }

  async addToQueue(item: Item) {
    try {
      await invoke("spotify_queue", { uri: item.uri });
      this.showToast("Added to queue");
    } catch (e) {
      this.handleError(e);
    }
  }

  // ---- account -------------------------------------------------------------------
  async loadAuth() {
    this.auth = await invoke<AuthStatus>("spotify_status");
  }

  /** True while the browser is open for Spotify login (so losing focus doesn't close the card). */
  loggingIn = $state(false);

  async login(clientId: string) {
    this.error = "";
    this.loggingIn = true;
    try {
      await invoke("spotify_login", { clientId });
      await this.loadAuth();
      this.refreshPlayer();
    } finally {
      this.loggingIn = false;
    }
  }

  async logout() {
    await invoke("spotify_logout");
    this.results = [];
    this.query = "";
    this.playlists = null;
    this.player = null;
    this.liked = null;
    await this.loadAuth();
  }

  // ---- settings ------------------------------------------------------------------
  async loadSettings() {
    this.sv = await invoke<SettingsView>("get_settings");
  }

  async saveSettings() {
    if (!this.sv) return;
    this.sv = await invoke<SettingsView>("save_settings", { settings: $state.snapshot(this.sv.settings) });
  }

  async resetHotkeys() {
    if (!this.sv) return;
    const d = await invoke<Settings>("default_settings");
    this.sv.settings.hotkeys = d.hotkeys;
    await this.saveSettings();
  }

  async setHotkey(a: Action, combo: string) {
    if (!this.sv) return;
    this.sv.settings.hotkeys[a] = combo;
    await this.saveSettings();
  }

  async toggleAutostart(enabled: boolean) {
    if (!this.sv) return;
    try {
      this.sv.autostart = await invoke<boolean>("set_autostart", { enabled });
    } catch (e) {
      this.error = String(e);
      await this.loadSettings();
    }
  }

  async checkUpdate(silent = false) {
    if (!silent) this.update = { state: "checking" };
    try {
      const v = await invoke<string | null>("check_update");
      this.update = v ? { state: "available", version: v } : { state: silent ? "idle" : "none" };
    } catch {
      this.update = silent ? { state: "idle" } : { state: "error", msg: "Couldn't check for updates." };
    }
  }

  async installUpdate() {
    this.update = { state: "installing" };
    try {
      await invoke("install_update"); // restarts the app when done
    } catch (e) {
      this.update = { state: "error", msg: String(e) };
    }
  }

  hk(a: Action) {
    return prettyCombo(this.sv?.settings.hotkeys[a] ?? "");
  }
}

export function prettyCombo(combo: string) {
  if (!combo) return "None";
  const names: Record<string, string> = {
    ArrowLeft: "←", ArrowRight: "→", ArrowUp: "↑", ArrowDown: "↓", Space: "Space",
    Backquote: "`", Minus: "-", Equal: "=", BracketLeft: "[", BracketRight: "]",
    Backslash: "\\", Semicolon: ";", Quote: "'", Comma: ",", Period: ".", Slash: "/",
  };
  return combo
    .split("+")
    .map((p) => names[p] ?? p.replace(/^Key/, "").replace(/^Digit/, "").replace(/^Numpad/, "Num "))
    .join(" + ");
}

export function fmtTime(ms: number) {
  const s = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

export const app = new App();
