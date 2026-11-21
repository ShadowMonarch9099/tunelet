<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount, tick } from "svelte";
  import { app } from "../app.svelte";
  import { icon } from "../icons";
  import type { Item } from "../types";

  let { tab, onswitch }: { tab: "search" | "playlists"; onswitch: (t: "search" | "playlists") => void } = $props();

  let input = $state<HTMLInputElement | null>(null);
  let selected = $state(0);
  let clientId = $state(app.auth?.clientId ?? "");
  let copied = $state(false);

  const kindLabel = { track: "Song", album: "Album", playlist: "Playlist", liked: "Liked" };

  let filtered = $derived(
    (app.playlists ?? []).filter(
      (p) => !app.query.trim() || p.name.toLowerCase().includes(app.query.trim().toLowerCase()),
    ),
  );
  /** Songs of the open playlist, filtered by the search box. */
  let listTracks = $derived(
    (app.openList?.tracks ?? []).filter((t) => {
      const q = app.query.trim().toLowerCase();
      return !q || t.name.toLowerCase().includes(q) || t.subtitle.toLowerCase().includes(q);
    }),
  );
  let inList = $derived(tab === "playlists" && !!app.openList);
  let items = $derived(tab === "search" ? app.results : inList ? listTracks : filtered);

  function openList(it: Item) {
    app.query = "";
    selected = 0;
    app.openPlaylist(it);
    focus();
  }

  function back() {
    app.query = "";
    selected = 0;
    app.closePlaylist();
    focus();
  }

  /** Load the next page of songs when scrolled near the bottom. */
  function onScroll(e: Event) {
    const el = e.currentTarget as HTMLElement;
    if (inList && el.scrollTop + el.clientHeight > el.scrollHeight - 240) app.loadMoreTracks();
  }

  function activate(it: Item) {
    if (inList) app.playInList(it);
    else if (tab === "playlists") openList(it);
    else app.play(it);
  }

  onMount(() => {
    if (tab === "playlists" && app.auth?.loggedIn) app.loadPlaylists();
    focus();
  });

  export async function focus() {
    await tick();
    input?.focus();
    input?.select();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      selected = Math.min(selected + 1, items.length - 1);
      scrollSel();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selected = Math.max(selected - 1, 0);
      scrollSel();
    } else if (e.key === "Escape" && inList) {
      // Esc steps back out of a playlist before it closes the card.
      e.preventDefault();
      e.stopPropagation();
      back();
    } else if (e.key === "Backspace" && inList && !app.query) {
      e.preventDefault();
      back();
    } else if (e.key === "Tab") {
      e.preventDefault();
      onswitch(tab === "search" ? "playlists" : "search");
    } else if (e.key === "Enter" && items[selected]) {
      e.preventDefault();
      const it = items[selected];
      if (e.shiftKey && it.kind === "track") app.addToQueue(it);
      else activate(it);
    }
  }

  async function scrollSel() {
    await tick();
    document.querySelector(".result.selected")?.scrollIntoView({ block: "nearest" });
  }

  async function login() {
    try {
      await app.login(clientId);
      if (tab === "playlists") app.loadPlaylists();
      await invoke("set_interactive", { interactive: true }); // take focus back from the browser
      focus();
    } catch (e) {
      app.handleError(e);
    }
  }

  async function copyRedirect() {
    if (!app.auth) return;
    await navigator.clipboard.writeText(app.auth.redirectUri);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  function sub(it: Item) {
    return tab === "search" && it.kind !== "track" ? it.subtitle.replace(/^(Album|Playlist) · /, "") : it.subtitle;
  }
</script>

{#if !app.auth}
  <div class="center muted">Loading…</div>
{:else if !app.auth.loggedIn}
  <div class="setup scroll">
    <p class="muted">Connect Spotify once to search and browse playlists. Needs Premium; about 2 minutes.</p>
    <ol>
      <li>
        Open the
        <button class="link" onclick={() => openUrl("https://developer.spotify.com/dashboard")}>Spotify developer dashboard</button>
        and click <b>Create app</b>.
      </li>
      <li>
        Under <b>Redirect URIs</b> paste this and click <b>Add</b>:
        <span class="copy-row">
          <code>{app.auth.redirectUri}</code>
          <button class="chip" onclick={copyRedirect}>{copied ? "Copied" : "Copy"}</button>
        </span>
        Tick <b>Web API</b>, then save.
      </li>
      <li>Copy the <b>Client ID</b> and paste it here:</li>
    </ol>
    <input
      class="field"
      placeholder="Client ID (32 characters)"
      bind:value={clientId}
      spellcheck="false"
      onkeydown={(e) => e.key === "Enter" && login()}
    />
    <button class="primary" disabled={app.loggingIn || !clientId.trim()} onclick={login}>
      {app.loggingIn ? "Waiting for browser…" : "Connect"}
    </button>
    {#if app.error}<p class="error">{app.error}</p>{/if}
  </div>
{:else}
  <div class="search-bar">
    <svg viewBox="0 0 24 24"><path d={icon.search} /></svg>
    <input
      bind:this={input}
      bind:value={app.query}
      oninput={() => {
        selected = 0;
        app.onQueryInput(tab);
      }}
      onkeydown={onKey}
      placeholder={tab === "search" ? "Songs, albums, playlists" : inList ? "Filter songs" : "Filter your playlists"}
      spellcheck="false"
    />
    {#if app.searching || app.playlistsLoading || app.openList?.loading}<span class="spinner"></span>{/if}
  </div>

  {#if inList && app.openList}
    {@const l = app.openList}
    <div class="list-head">
      <button class="icon-btn" title="Back (Esc)" aria-label="Back" onclick={back}>
        <svg viewBox="0 0 24 24"><path d={icon.back} /></svg>
      </button>
      <div class="thumb small" class:liked-tile={l.item.kind === "liked"}>
        {#if l.item.kind === "liked"}
          <svg viewBox="0 0 24 24"><path d={icon.heart} /></svg>
        {:else if l.item.image}
          <img src={l.item.image} alt="" draggable="false" />
        {/if}
      </div>
      <div class="info">
        <div class="name">{l.item.name}</div>
        <div class="sub">{l.total ? `${l.total} songs` : l.item.subtitle}</div>
      </div>
      <button class="round-btn" title="Shuffle play" aria-label="Shuffle play" onclick={() => app.playList(l.item, true)}>
        <svg viewBox="0 0 24 24"><path d={icon.shuffle} /></svg>
      </button>
      <button class="round-btn accent" title="Play" aria-label="Play" onclick={() => app.playList(l.item, false)}>
        <svg viewBox="0 0 24 24"><path d={icon.play} /></svg>
      </button>
    </div>
  {/if}

  <div class="results scroll fade" onscroll={onScroll}>
    {#if app.error}
      <p class="error pad">{app.error}</p>
    {:else if tab === "search" && !app.query.trim()}
      <div class="center muted hint">
        Type to search<br />
        <span>↑↓ choose · Enter play · Shift+Enter queue<br />Tab playlists · Esc close</span>
      </div>
    {:else if tab === "search" && !app.searching && app.results.length === 0}
      <div class="center muted">No results</div>
    {:else if inList && app.openList?.error}
      <p class="note pad">{app.openList.error}</p>
    {:else if inList && !app.openList?.loading && listTracks.length === 0}
      <div class="center muted">{app.query.trim() ? "No matching songs" : "No songs"}</div>
    {:else if tab === "playlists" && !inList && app.playlists && filtered.length === 0}
      <div class="center muted">{app.query.trim() ? "No matching playlists" : "No playlists yet"}</div>
    {/if}

    {#each items as it, i (inList ? `${i}:${it.uri}` : it.uri)}
      <div
        class="result"
        class:selected={i === selected}
        style="--i: {Math.min(i, 12)}"
        role="button"
        tabindex="-1"
        onmouseenter={() => (selected = i)}
        onclick={() => activate(it)}
        onkeydown={() => {}}
      >
        <div class="thumb" class:liked-tile={it.kind === "liked"}>
          {#if it.kind === "liked"}
            <svg viewBox="0 0 24 24"><path d={icon.heart} /></svg>
          {:else if it.image}
            <img src={it.image} alt="" draggable="false" />
          {/if}
        </div>
        <div class="info">
          <div class="name">{it.name}</div>
          <div class="sub">
            {#if tab === "search" && it.kind !== "track"}<span class="badge">{kindLabel[it.kind]}</span>{/if}
            {sub(it)}
          </div>
        </div>
        {#if tab === "playlists" && !inList}
          <button
            class="row-play queue"
            title="Play"
            aria-label="Play {it.name}"
            onclick={(e) => {
              e.stopPropagation();
              app.playList(it, false);
            }}
          >
            <svg viewBox="0 0 24 24"><path d={icon.play} /></svg>
          </button>
        {:else if it.kind === "track"}
          <button
            class="chip queue"
            title="Add to queue (Shift+Enter)"
            onclick={(e) => {
              e.stopPropagation();
              app.addToQueue(it);
            }}>+ Queue</button>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .list-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 12px 6px 6px;
    animation: row-in 240ms ease both;
  }
  .list-head .info { flex: 1; min-width: 0; line-height: 1.3; }
  .thumb.small { width: 36px; height: 36px; border-radius: 6px; }
  .icon-btn,
  .round-btn,
  .row-play {
    display: grid;
    place-items: center;
    flex: none;
    padding: 0;
    border: none;
    border-radius: 50%;
    color: #fff;
    cursor: pointer;
    transition: background 150ms, transform 150ms, opacity 150ms;
  }
  .icon-btn { width: 28px; height: 28px; background: transparent; }
  .icon-btn:hover { background: rgba(255, 255, 255, 0.14); }
  .round-btn { width: 32px; height: 32px; background: rgba(255, 255, 255, 0.14); }
  .round-btn:hover { background: rgba(255, 255, 255, 0.24); }
  .round-btn.accent { background: #fff; color: #111; }
  .round-btn.accent:hover { transform: scale(1.06); }
  .icon-btn:active,
  .round-btn:active,
  .row-play:active { transform: scale(0.9); }
  .icon-btn svg,
  .round-btn svg { width: 16px; height: 16px; fill: currentColor; }
  .row-play { width: 28px; height: 28px; background: #fff; color: #111; }
  .row-play svg { width: 14px; height: 14px; fill: currentColor; }
  .note { font-size: 12px; opacity: 0.8; line-height: 1.45; margin: 6px 0; }
</style>
