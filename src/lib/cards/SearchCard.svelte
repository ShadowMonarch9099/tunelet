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
  let items = $derived(tab === "search" ? app.results : filtered);

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
    } else if (e.key === "Tab") {
      e.preventDefault();
      onswitch(tab === "search" ? "playlists" : "search");
    } else if (e.key === "Enter" && items[selected]) {
      e.preventDefault();
      const it = items[selected];
      if (e.shiftKey && it.kind === "track") app.addToQueue(it);
      else app.play(it);
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
      placeholder={tab === "search" ? "Songs, albums, playlists" : "Filter your playlists"}
      spellcheck="false"
    />
    {#if app.searching || app.playlistsLoading}<span class="spinner"></span>{/if}
  </div>

  <div class="results scroll">
    {#if app.error}
      <p class="error pad">{app.error}</p>
    {:else if tab === "search" && !app.query.trim()}
      <div class="center muted hint">
        Type to search<br />
        <span>↑↓ choose · Enter play · Shift+Enter queue<br />Tab playlists · Esc close</span>
      </div>
    {:else if tab === "search" && !app.searching && app.results.length === 0}
      <div class="center muted">No results</div>
    {:else if tab === "playlists" && app.playlists && filtered.length === 0}
      <div class="center muted">{app.query.trim() ? "No matching playlists" : "No playlists yet"}</div>
    {/if}

    {#each items as it, i (it.uri)}
      <div
        class="result"
        class:selected={i === selected}
        style="--i: {Math.min(i, 12)}"
        role="button"
        tabindex="-1"
        onmouseenter={() => (selected = i)}
        onclick={() => app.play(it)}
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
        {#if it.kind === "track"}
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
