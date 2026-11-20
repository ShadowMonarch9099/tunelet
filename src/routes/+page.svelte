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

  const win = getCurrentWindow();

  onMount(() => {
    const offNow = listen<NowPlaying>("now-playing", (e) => (np = e.payload));
    const offSearch = listen("open-search", () => expand());
    invoke("media", { action: "refresh" });
    loadAuth();
    return () => {
      offNow.then((f) => f());
      offSearch.then((f) => f());
    };
  });

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
    if (auth?.loggedIn) {
      searchInput?.focus();
      searchInput?.select();
    }
  }

  async function collapse() {
    if (!expanded) return;
    expanded = false;
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
    if (e.key === "Escape") {
      if (expanded) collapse();
      else hide();
    }
  }

  function onWindowBlur() {
    // Clicking into another app closes search, unless we're mid-login
    // (the browser takes focus during login).
    if (expanded && auth?.loggedIn && !loggingIn) collapse();
  }

  const kindLabel = { track: "Song", album: "Album", playlist: "Playlist" };
</script>

<svelte:window onkeydown={onKey} onblur={onWindowBlur} />

<div class="shell" style="--c1: {c1}; --c2: {c2}">
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
      <button aria-label="Previous" title="Previous (Ctrl+Alt+←)" onclick={() => media("previous")}>
        <svg viewBox="0 0 24 24"><path d="M6 6h2v12H6zM9.5 12 18 6v12z" /></svg>
      </button>
      <button class="play" aria-label={playing ? "Pause" : "Play"} title="Play / Pause (Ctrl+Alt+P)" onclick={() => media("toggle")}>
        {#if playing}
          <svg viewBox="0 0 24 24"><path d="M7 5h4v14H7zM13 5h4v14h-4z" /></svg>
        {:else}
          <svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z" /></svg>
        {/if}
      </button>
      <button aria-label="Next" title="Next (Ctrl+Alt+→)" onclick={() => media("next")}>
        <svg viewBox="0 0 24 24"><path d="M16 6h2v12h-2zM6 18l8.5-6L6 6z" /></svg>
      </button>
      <button
        class:active={expanded}
        aria-label="Search"
        title="Search (Ctrl+Alt+F)"
        onclick={() => (expanded ? collapse() : expand())}
      >
        <svg viewBox="0 0 24 24"><path d="M10 4a6 6 0 0 1 4.8 9.6l4.8 4.8-1.4 1.4-4.8-4.8A6 6 0 1 1 10 4zm0 2a4 4 0 1 0 0 8 4 4 0 0 0 0-8z" /></svg>
      </button>
    </div>

    <button class="close" aria-label="Hide" title="Hide (Ctrl+Alt+S)" onclick={hide}>×</button>
  </main>

  {#if expanded}
    <section class="panel surface">
      {#if !auth}
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

        <footer>
          <span class="muted">Connected to Spotify</span>
          <button class="link small" onclick={logout}>Log out</button>
        </footer>
      {/if}

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
