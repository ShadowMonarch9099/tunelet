<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount, tick } from "svelte";

  type NowPlaying = {
    available: boolean;
    title: string;
    artist: string;
    album: string;
    playing: boolean;
    art: string | null;
    colors: [string, string] | null;
  };
  type AuthStatus = { clientId: string | null; loggedIn: boolean; redirectUri: string };
  type Item = {
    kind: "track" | "album" | "playlist";
    uri: string;
    name: string;
    subtitle: string;
    image: string | null;
  };

  type Action = "toggle" | "search" | "playPause" | "next" | "previous";
  type Settings = { hotkeys: Record<Action, string>; opacity: number };
  type HotkeyError = { action: Action; combo: string; reason: "taken" | "invalid" | "duplicate" };
  type SettingsView = { settings: Settings; autostart: boolean; version: string; hotkeyErrors: HotkeyError[] };
  type UpdateState =
    | { state: "idle" | "checking" | "none" | "installing" }
    | { state: "available"; version: string }
    | { state: "error"; msg: string };

  const ACTIONS: Action[] = ["toggle", "search", "playPause", "next", "previous"];
  const LABELS: Record<Action, string> = {
    toggle: "Show / hide",
    search: "Search",
    playPause: "Play / pause",
    next: "Next song",
    previous: "Previous song",
  };
  const HOTKEY_PROBLEM = {
    taken: "Used by another app. Pick another.",
    invalid: "That key isn't supported.",
    duplicate: "Already used for another action.",
  };

  const IDLE: [string, string] = ["#3a2a5c", "#12324a"];
  const WIDTH = 320;
  const PILL_H = 64;
  const EXPANDED_H = 460;

  // ---- now playing ---------------------------------------------------------
  let np = $state<NowPlaying | null>(null);
  let title = $derived(np?.available ? np.title : "Nothing playing");
  let artist = $derived(np?.available ? np.artist : "Open Spotify to start");
  let playing = $derived(np?.available ? np.playing : false);
  let art = $derived(np?.available ? np.art : null);
  let c1 = $derived(np?.available && np.colors ? np.colors[0] : IDLE[0]);
  let c2 = $derived(np?.available && np.colors ? np.colors[1] : IDLE[1]);

  // ---- expanded panel ------------------------------------------------------
  let expanded = $state(false);
  let auth = $state<AuthStatus | null>(null);
  let clientIdInput = $state("");
  let loggingIn = $state(false);
  let copied = $state(false);

  let query = $state("");
  let results = $state<Item[]>([]);
  let selected = $state(0);
  let searching = $state(false);
  let error = $state("");
  let toast = $state("");
  let searchInput = $state<HTMLInputElement | null>(null);
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let searchSeq = 0;
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  // ---- settings ------------------------------------------------------------
  let view = $state<"main" | "settings">("main");
  let sv = $state<SettingsView | null>(null);
  let recording = $state<Action | null>(null);
  let recordHint = $state("");
  let update = $state<UpdateState>({ state: "idle" });
  let opacity = $derived(sv?.settings.opacity ?? 1);

  const win = getCurrentWindow();

  onMount(() => {
    const offs = [
      listen<NowPlaying>("now-playing", (e) => (np = e.payload)),
      listen("open-search", () => {
        view = "main";
        expand();
      }),
      listen("open-settings", () => openSettings()),
    ];
    invoke("media", { action: "refresh" });
    loadAuth();
    loadSettings();
    checkUpdate(true);
    return () => offs.forEach((o) => o.then((f) => f()));
  });

  async function loadSettings() {
    sv = await invoke<SettingsView>("get_settings");
  }

  async function saveSettings() {
    if (!sv) return;
    sv = await invoke<SettingsView>("save_settings", { settings: $state.snapshot(sv.settings) });
  }

  function hk(a: Action) {
    return pretty(sv?.settings.hotkeys[a] ?? "");
  }

  function pretty(combo: string) {
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

  async function openSettings() {
    view = "settings";
    await loadSettings();
    await expand();
  }

  async function startRecording(a: Action) {
    recording = a;
    recordHint = "";
    await invoke("suspend_hotkeys", { suspend: true });
  }

  async function stopRecording(newCombo?: string) {
    const a = recording;
    recording = null;
    recordHint = "";
    if (a && sv && newCombo !== undefined) {
      sv.settings.hotkeys[a] = newCombo;
      await saveSettings(); // re-registers all hotkeys
    } else {
      await invoke("suspend_hotkeys", { suspend: false });
    }
  }

  function onRecordKey(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return;
    if (e.key === "Escape") return void stopRecording();
    if ((e.key === "Backspace" || e.key === "Delete") && !e.ctrlKey && !e.altKey) return void stopRecording("");
    const mods: string[] = [];
    if (e.ctrlKey) mods.push("Ctrl");
    if (e.altKey) mods.push("Alt");
    if (e.shiftKey) mods.push("Shift");
    if (e.metaKey) mods.push("Super");
    if (mods.length === 0 || (mods.length === 1 && mods[0] === "Shift")) {
      recordHint = "Include Ctrl, Alt or Win so it doesn't clash with typing.";
      return;
    }
    stopRecording([...mods, e.code].join("+"));
  }

  async function resetHotkeys() {
    if (!sv) return;
    const d = await invoke<Settings>("default_settings");
    sv.settings.hotkeys = d.hotkeys;
    await saveSettings();
  }

  async function toggleAutostart(enabled: boolean) {
    if (!sv) return;
    try {
      sv.autostart = await invoke<boolean>("set_autostart", { enabled });
    } catch (e) {
      error = String(e);
      await loadSettings();
    }
  }

  async function checkUpdate(silent = false) {
    if (!silent) update = { state: "checking" };
    try {
      const v = await invoke<string | null>("check_update");
      update = v ? { state: "available", version: v } : { state: silent ? "idle" : "none" };
    } catch (e) {
      update = silent ? { state: "idle" } : { state: "error", msg: "Couldn't check for updates." };
    }
  }

  async function installUpdate() {
    update = { state: "installing" };
    try {
      await invoke("install_update"); // restarts the app when done
    } catch (e) {
      update = { state: "error", msg: String(e) };
    }
  }

  async function loadAuth() {
    auth = await invoke<AuthStatus>("spotify_status");
    if (auth.clientId && !clientIdInput) clientIdInput = auth.clientId;
  }

  async function expand() {
    if (!expanded) {
      await win.setSize(new LogicalSize(WIDTH, EXPANDED_H));
      expanded = true;
    }
    await invoke("set_interactive", { interactive: true });
    await tick();
    if (auth?.loggedIn && view === "main") {
      searchInput?.focus();
      searchInput?.select();
    }
  }

  async function collapse() {
    if (!expanded) return;
    if (recording) await stopRecording();
    expanded = false;
    view = "main";
    error = "";
    await tick();
    await win.setSize(new LogicalSize(WIDTH, PILL_H));
    await invoke("set_interactive", { interactive: false });
  }

  function showToast(msg: string) {
    toast = msg;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ""), 1800);
  }

  function handleError(e: unknown) {
    const msg = String(e);
    if (msg === "not_logged_in") {
      error = "";
      loadAuth();
    } else {
      error = msg;
    }
  }

  // ---- login ---------------------------------------------------------------
  async function login() {
    error = "";
    loggingIn = true;
    try {
      await invoke("spotify_login", { clientId: clientIdInput });
      await loadAuth();
      await expand(); // take focus back from the browser and focus search
    } catch (e) {
      handleError(e);
    } finally {
      loggingIn = false;
    }
  }

  async function logout() {
    await invoke("spotify_logout");
    results = [];
    query = "";
    await loadAuth();
  }

  async function copyRedirect() {
    if (!auth) return;
    await navigator.clipboard.writeText(auth.redirectUri);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  // ---- search --------------------------------------------------------------
  function onQueryInput() {
    clearTimeout(searchTimer);
    error = "";
    if (!query.trim()) {
      results = [];
      searching = false;
      return;
    }
    searching = true;
    searchTimer = setTimeout(runSearch, 280);
  }

  async function runSearch() {
    const seq = ++searchSeq;
    try {
      const r = await invoke<Item[]>("spotify_search", { query });
      if (seq !== searchSeq) return; // a newer search is on its way
      results = r;
      selected = 0;
    } catch (e) {
      if (seq === searchSeq) handleError(e);
    } finally {
      if (seq === searchSeq) searching = false;
    }
  }

  async function play(item: Item) {
    try {
      await invoke("spotify_play", { uri: item.uri });
      showToast(`Playing ${item.name}`);
      setTimeout(collapse, 350);
    } catch (e) {
      handleError(e);
    }
  }

  async function addToQueue(item: Item) {
    try {
      await invoke("spotify_queue", { uri: item.uri });
      showToast("Added to queue");
    } catch (e) {
      handleError(e);
    }
  }

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selected = Math.min(selected + 1, results.length - 1);
      scrollSelected();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selected = Math.max(selected - 1, 0);
      scrollSelected();
    } else if (e.key === "Enter" && results[selected]) {
      e.preventDefault();
      const item = results[selected];
      if (e.shiftKey && item.kind === "track") addToQueue(item);
      else play(item);
    }
  }

  async function scrollSelected() {
    await tick();
    document.querySelector(".result.selected")?.scrollIntoView({ block: "nearest" });
  }

  // ---- misc ----------------------------------------------------------------
  function media(action: "toggle" | "next" | "previous") {
    // Flip the icon right away; the real state arrives a moment later.
    if (action === "toggle" && np?.available) np = { ...np, playing: !np.playing };
    invoke("media", { action });
  }

  function hide() {
    collapse();
    invoke("hide_widget");
  }

  function onKey(e: KeyboardEvent) {
    if (recording) return onRecordKey(e);
    if (e.key === "Escape") {
      if (expanded) collapse();
      else hide();
    }
  }

  function onWindowBlur() {
    // Clicking into another app closes search, unless we're mid-login
    // (the browser takes focus during login).
    if (recording) stopRecording();
    if (expanded && auth?.loggedIn && !loggingIn && view === "main") collapse();
  }

  const kindLabel = { track: "Song", album: "Album", playlist: "Playlist" };
</script>

<svelte:window onkeydown={onKey} onblur={onWindowBlur} />

<div class="shell" style="--c1: {c1}; --c2: {c2}; opacity: {opacity}">
  <main class="pill surface" data-tauri-drag-region>
    <div class="art" data-tauri-drag-region>
      {#if art}
        {#key art}
          <img src={art} alt="" draggable="false" data-tauri-drag-region />
        {/key}
      {/if}
    </div>

    <div class="meta" data-tauri-drag-region title={np?.available ? `${np.title} — ${np.artist}` : ""}>
      <div class="title" data-tauri-drag-region>{title}</div>
      <div class="artist" data-tauri-drag-region>{artist}</div>
    </div>

    <div class="controls">
      <button aria-label="Previous" title="Previous ({hk('previous')})" onclick={() => media("previous")}>
        <svg viewBox="0 0 24 24"><path d="M6 6h2v12H6zM9.5 12 18 6v12z" /></svg>
      </button>
      <button class="play" aria-label={playing ? "Pause" : "Play"} title="Play / Pause ({hk('playPause')})" onclick={() => media("toggle")}>
        {#if playing}
          <svg viewBox="0 0 24 24"><path d="M7 5h4v14H7zM13 5h4v14h-4z" /></svg>
        {:else}
          <svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg>
        {/if}
      </button>
      <button aria-label="Next" title="Next ({hk('next')})" onclick={() => media("next")}>
        <svg viewBox="0 0 24 24"><path d="M16 6h2v12h-2zM6 18l8.5-6L6 6z" /></svg>
      </button>
      <button
        class:active={expanded}
        aria-label="Search"
        title="Search ({hk('search')})"
        onclick={() => {
          if (expanded) collapse();
          else {
            view = "main";
            expand();
          }
        }}
      >
        <svg viewBox="0 0 24 24"><path d="M10 4a6 6 0 0 1 4.8 9.6l4.8 4.8-1.4 1.4-4.8-4.8A6 6 0 1 1 10 4zm0 2a4 4 0 1 0 0 8 4 4 0 0 0 0-8z" /></svg>
      </button>
    </div>

    <button class="close" aria-label="Hide" title="Hide ({hk('toggle')})" onclick={hide}>×</button>
  </main>

  {#if expanded}
    <section class="panel surface">
      {#if view === "settings" && sv}
        <div class="settings">
          <h3>Hotkeys</h3>
          {#each ACTIONS as a (a)}
            {@const problem = sv.hotkeyErrors.find((x) => x.action === a)}
            <div class="row">
              <span>{LABELS[a]}</span>
              <button
                class="key"
                class:recording={recording === a}
                class:bad={!!problem}
                onclick={() => (recording === a ? stopRecording() : startRecording(a))}
              >
                {recording === a ? "Press keys…" : pretty(sv.settings.hotkeys[a])}
              </button>
            </div>
            {#if problem && recording !== a}<div class="row-note error">{HOTKEY_PROBLEM[problem.reason]}</div>{/if}
            {#if recording === a && recordHint}<div class="row-note">{recordHint}</div>{/if}
          {/each}
          <div class="row-note">
            Click a shortcut, then press the new keys. Backspace clears it, Esc cancels.
            <button class="link small" onclick={resetHotkeys}>Reset to defaults</button>
          </div>

          <h3>Appearance</h3>
          <div class="row">
            <span>Opacity</span>
            <span class="slider">
              <input
                type="range"
                min="40"
                max="100"
                step="5"
                value={Math.round(opacity * 100)}
                oninput={(e) => sv && (sv.settings.opacity = +e.currentTarget.value / 100)}
                onchange={saveSettings}
              />
              <span class="pct">{Math.round(opacity * 100)}%</span>
            </span>
          </div>

          <h3>General</h3>
          <label class="row">
            <span>Start with Windows</span>
            <input
              type="checkbox"
              class="switch"
              checked={sv.autostart}
              onchange={(e) => toggleAutostart(e.currentTarget.checked)}
            />
          </label>
          <div class="row">
            <span>Spotify</span>
            {#if auth?.loggedIn}
              <button class="link small" onclick={logout}>Log out</button>
            {:else}
              <span class="muted small-text">Not connected</span>
            {/if}
          </div>

          <h3>About</h3>
          <div class="row">
            <span>Tunelet v{sv.version}</span>
            {#if update.state === "available"}
              <button class="chip accent" onclick={installUpdate}>Update to v{update.version}</button>
            {:else if update.state === "installing"}
              <span class="muted small-text">Installing…</span>
            {:else if update.state === "checking"}
              <span class="muted small-text">Checking…</span>
            {:else if update.state === "none"}
              <span class="muted small-text">Up to date</span>
            {:else}
              <button class="chip" onclick={() => checkUpdate()}>Check for updates</button>
            {/if}
          </div>
          {#if update.state === "error"}<div class="row-note error">{update.msg}</div>{/if}
          {#if error}<div class="row-note error">{error}</div>{/if}
          <button class="link small gh" onclick={() => openUrl("https://github.com/ShadowMonarch9099/tunelet")}>
            Tunelet on GitHub
          </button>
        </div>
      {:else if !auth}
        <div class="center muted">Loading…</div>
      {:else if !auth.loggedIn}
        <div class="setup">
          <h2>Connect Spotify</h2>
          <p class="muted">One-time setup, about 2 minutes. Needs Spotify Premium.</p>
          <ol>
            <li>
              Open the
              <button class="link" onclick={() => openUrl("https://developer.spotify.com/dashboard")}>Spotify developer dashboard</button>
              and click <b>Create app</b>. Any name is fine.
            </li>
            <li>
              Under <b>Redirect URIs</b>, add:
              <span class="copy-row">
                <code>{auth.redirectUri}</code>
                <button class="chip" onclick={copyRedirect}>{copied ? "Copied" : "Copy"}</button>
              </span>
              and tick <b>Web API</b>, then save.
            </li>
            <li>Copy the app's <b>Client ID</b> and paste it here:</li>
          </ol>
          <input
            class="field"
            placeholder="Client ID (32 characters)"
            bind:value={clientIdInput}
            spellcheck="false"
            onkeydown={(e) => e.key === "Enter" && login()}
          />
          <button class="primary" disabled={loggingIn || !clientIdInput.trim()} onclick={login}>
            {loggingIn ? "Waiting for browser…" : "Connect"}
          </button>
          {#if error}<p class="error">{error}</p>{/if}
        </div>
      {:else}
        <div class="search-bar">
          <svg viewBox="0 0 24 24"><path d="M10 4a6 6 0 0 1 4.8 9.6l4.8 4.8-1.4 1.4-4.8-4.8A6 6 0 1 1 10 4zm0 2a4 4 0 1 0 0 8 4 4 0 0 0 0-8z" /></svg>
          <input
            bind:this={searchInput}
            bind:value={query}
            oninput={onQueryInput}
            onkeydown={onSearchKey}
            placeholder="Search songs, albums, playlists"
            spellcheck="false"
          />
          {#if searching}<span class="spinner"></span>{/if}
        </div>

        <div class="results">
          {#if error}
            <p class="error pad">{error}</p>
          {:else if !query.trim()}
            <div class="center muted hint">
              Type to search<br />
              <span>↑↓ choose · Enter play · Shift+Enter queue · Esc close</span>
            </div>
          {:else if !searching && results.length === 0}
            <div class="center muted">No results</div>
          {/if}

          {#each results as item, i (item.uri)}
            <div
              class="result"
              class:selected={i === selected}
              role="button"
              tabindex="-1"
              onmouseenter={() => (selected = i)}
              onclick={() => play(item)}
              onkeydown={() => {}}
            >
              <div class="thumb">
                {#if item.image}<img src={item.image} alt="" draggable="false" />{/if}
              </div>
              <div class="info">
                <div class="name">{item.name}</div>
                <div class="sub">
                  {#if item.kind !== "track"}<span class="badge">{kindLabel[item.kind]}</span>{/if}
                  {item.kind === "track" ? item.subtitle : item.subtitle.replace(/^(Album|Playlist) · /, "")}
                </div>
              </div>
              {#if item.kind === "track"}
                <button
                  class="chip queue"
                  title="Add to queue (Shift+Enter)"
                  onclick={(e) => {
                    e.stopPropagation();
                    addToQueue(item);
                  }}>+ Queue</button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      <footer>
        <span class="muted">
          {#if view === "settings"}Settings{:else if auth?.loggedIn}Connected to Spotify{:else}Spotify not connected{/if}
        </span>
        <button
          class="chip footer-btn"
          onclick={async () => {
            if (recording) await stopRecording();
            if (view === "settings") {
              view = "main";
              await tick();
              searchInput?.focus();
            } else {
              await openSettings();
            }
          }}
        >
          {#if view === "settings"}
            ← Back
          {:else}
            Settings{#if update.state === "available"}<span class="dot"></span>{/if}
          {/if}
        </button>
      </footer>

      {#if toast}<div class="toast">{toast}</div>{/if}
    </section>
  {/if}
</div>

<style>
  @property --c1 { syntax: "<color>"; inherits: true; initial-value: #222; }
  @property --c2 { syntax: "<color>"; inherits: true; initial-value: #111; }

  :global(*) { box-sizing: border-box; }

  .shell {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 6px;
    color: #fff;
    font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
    transition: --c1 600ms ease, --c2 600ms ease;
  }

  .surface {
    border-radius: 18px;
    background:
      linear-gradient(rgba(10, 10, 14, 0.35), rgba(10, 10, 14, 0.35)),
      linear-gradient(120deg, var(--c1), var(--c2));
    border: 1px solid rgba(255, 255, 255, 0.12);
    overflow: hidden;
  }

  /* ---- pill ---- */
  .pill {
    position: relative;
    flex: none;
    height: 64px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 8px;
    user-select: none;
    cursor: grab;
  }

  .art {
    flex: none;
    width: 48px;
    height: 48px;
    border-radius: 10px;
    background: linear-gradient(135deg, var(--c2), var(--c1));
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
    overflow: hidden;
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    animation: art-in 450ms ease;
  }
  @keyframes art-in {
    from { opacity: 0; transform: scale(0.85) rotateY(60deg); }
    to { opacity: 1; transform: none; }
  }

  .meta { flex: 1; min-width: 0; line-height: 1.25; }
  .title, .artist, .name, .sub { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .title { font-size: 13px; font-weight: 600; }
  .artist { font-size: 11.5px; opacity: 0.75; }

  .controls { display: flex; align-items: center; gap: 0; }

  button {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
    transition: background 120ms, transform 120ms, opacity 120ms;
  }
  button:hover { background: rgba(255, 255, 255, 0.14); }
  button:active { transform: scale(0.92); }
  button svg { width: 18px; height: 18px; fill: currentColor; }
  button.active { background: rgba(255, 255, 255, 0.2); }

  .play { width: 34px; height: 34px; margin: 0 2px; background: rgba(255, 255, 255, 0.92); color: #111; }
  .play:hover { background: #fff; }

  .close {
    position: absolute;
    top: 2px;
    right: 4px;
    width: 16px;
    height: 16px;
    font-size: 13px;
    line-height: 1;
    opacity: 0;
  }
  .pill:hover .close { opacity: 0.7; }

  /* ---- panel ---- */
  .panel {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    animation: panel-in 180ms ease;
  }
  @keyframes panel-in {
    from { opacity: 0; transform: translateY(-6px); }
    to { opacity: 1; transform: none; }
  }

  .muted { opacity: 0.65; }
  .center { margin: auto; text-align: center; font-size: 13px; }
  .hint span { font-size: 11px; opacity: 0.8; }
  .pad { padding: 0 14px; }
  .error { color: #ffb4b4; font-size: 12px; margin: 8px 0 0; }

  .search-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 10px;
    padding: 0 12px;
    height: 38px;
    border-radius: 12px;
    background: rgba(0, 0, 0, 0.28);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }
  .search-bar:focus-within { border-color: rgba(255, 255, 255, 0.35); }
  .search-bar svg { width: 16px; height: 16px; fill: currentColor; opacity: 0.7; flex: none; }
  .search-bar input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    color: inherit;
    font: inherit;
    font-size: 13.5px;
  }
  .search-bar input::placeholder { color: rgba(255, 255, 255, 0.5); }

  .spinner {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid rgba(255, 255, 255, 0.25);
    border-top-color: #fff;
    animation: spin 700ms linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .results {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 6px 6px;
    display: flex;
    flex-direction: column;
  }
  .results::-webkit-scrollbar { width: 6px; }
  .results::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.2); border-radius: 3px; }

  .result {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px;
    border-radius: 10px;
    cursor: pointer;
  }
  .result.selected { background: rgba(255, 255, 255, 0.13); }

  .thumb {
    flex: none;
    width: 40px;
    height: 40px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.3);
    overflow: hidden;
  }
  .thumb img { width: 100%; height: 100%; object-fit: cover; display: block; }

  .info { flex: 1; min-width: 0; line-height: 1.3; }
  .name { font-size: 13px; font-weight: 500; }
  .sub { font-size: 11.5px; opacity: 0.7; }
  .badge {
    display: inline-block;
    font-size: 9.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0 5px;
    margin-right: 4px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.18);
  }

  .chip {
    width: auto;
    height: 24px;
    padding: 0 9px;
    border-radius: 12px;
    font-size: 11px;
    background: rgba(255, 255, 255, 0.14);
    white-space: nowrap;
  }
  .chip:hover { background: rgba(255, 255, 255, 0.25); }
  .queue { opacity: 0; }
  .result.selected .queue { opacity: 1; }

  footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 14px 8px;
    font-size: 11px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
  }

  .link {
    display: inline;
    width: auto;
    height: auto;
    border-radius: 0;
    color: #fff;
    text-decoration: underline;
    text-underline-offset: 2px;
    font-weight: 600;
  }
  .link:hover { background: none; opacity: 0.85; }
  .small { font-size: 11px; font-weight: 400; opacity: 0.8; }

  .setup { padding: 14px 16px; overflow-y: auto; font-size: 12.5px; line-height: 1.45; }
  .setup h2 { margin: 0 0 2px; font-size: 16px; }
  .setup p { margin: 0 0 8px; }
  .setup ol { margin: 0 0 10px; padding-left: 18px; }
  .setup li { margin-bottom: 8px; }
  .copy-row { display: flex; align-items: center; gap: 6px; margin: 4px 0; }
  code {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: "Cascadia Mono", Consolas, monospace;
    font-size: 11px;
    padding: 3px 6px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.3);
    user-select: all;
  }
  .field {
    width: 100%;
    height: 34px;
    padding: 0 10px;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: rgba(0, 0, 0, 0.28);
    color: inherit;
    font-family: "Cascadia Mono", Consolas, monospace;
    font-size: 12px;
    outline: none;
  }
  .field:focus { border-color: rgba(255, 255, 255, 0.4); }
  .primary {
    width: 100%;
    height: 34px;
    margin-top: 8px;
    border-radius: 10px;
    background: #fff;
    color: #111;
    font-weight: 600;
    font-size: 13px;
  }
  .primary:hover { background: #fff; opacity: 0.9; }
  .primary:disabled { opacity: 0.5; cursor: default; }

  /* ---- settings ---- */
  .settings { flex: 1; min-height: 0; overflow-y: auto; padding: 6px 14px 10px; font-size: 12.5px; }
  .settings::-webkit-scrollbar { width: 6px; }
  .settings::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.2); border-radius: 3px; }
  .settings h3 {
    margin: 12px 0 4px;
    font-size: 10.5px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.6;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    min-height: 32px;
  }
  .row-note { font-size: 11px; opacity: 0.75; margin: -2px 0 4px; line-height: 1.4; }
  .row-note.error { opacity: 1; margin-top: 0; }
  .key {
    width: auto;
    min-width: 96px;
    height: 26px;
    padding: 0 10px;
    border-radius: 8px;
    font-family: "Cascadia Mono", Consolas, monospace;
    font-size: 11px;
    background: rgba(0, 0, 0, 0.28);
    border: 1px solid rgba(255, 255, 255, 0.15);
  }
  .key.recording { border-color: #fff; background: rgba(255, 255, 255, 0.18); animation: pulse 1s ease infinite; }
  .key.bad { border-color: #ff9a9a; }
  @keyframes pulse { 50% { border-color: rgba(255, 255, 255, 0.4); } }
  .slider { display: flex; align-items: center; gap: 8px; }
  .slider input { width: 120px; accent-color: #fff; }
  .pct { width: 34px; text-align: right; font-size: 11px; opacity: 0.8; }
  .small-text { font-size: 11px; }
  .gh { margin-top: 10px; }
  .accent { background: #fff; color: #111; font-weight: 600; }
  .accent:hover { background: #fff; opacity: 0.9; }

  .switch {
    appearance: none;
    width: 34px;
    height: 20px;
    margin: 0;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.2);
    position: relative;
    cursor: pointer;
    transition: background 150ms;
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #fff;
    transition: transform 150ms;
  }
  .switch:checked { background: #1ed760; }
  .switch:checked::after { transform: translateX(14px); }

  .footer-btn { position: relative; height: 22px; }
  .dot {
    position: absolute;
    top: -2px;
    right: -2px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #1ed760;
  }

  .toast {
    position: absolute;
    left: 50%;
    bottom: 40px;
    transform: translateX(-50%);
    max-width: 86%;
    padding: 6px 12px;
    border-radius: 14px;
    background: rgba(0, 0, 0, 0.75);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    animation: panel-in 150ms ease;
  }
</style>
