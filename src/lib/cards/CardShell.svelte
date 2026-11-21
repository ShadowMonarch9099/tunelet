<script lang="ts">
  import type { Snippet } from "svelte";
  import { app } from "../app.svelte";
  import { icon } from "../icons";
  import type { Card } from "../types";

  let {
    card,
    shown,
    origin,
    onswitch,
    onclose,
    ondrag,
    children,
  }: {
    card: Card;
    shown: boolean;
    /** CSS transform-origin: the corner nearest the bob, so the card grows out of it. */
    origin: string;
    onswitch: (c: Card) => void;
    onclose: () => void;
    /** Drag the whole widget by the card's header. */
    ondrag: (e: PointerEvent) => void;
    children: Snippet;
  } = $props();

  const NAV: { id: Card; label: string; path: string }[] = [
    { id: "search", label: "Search", path: icon.search },
    { id: "playlists", label: "Playlists", path: icon.playlist },
    { id: "player", label: "Now playing", path: icon.nowPlaying },
    { id: "lyrics", label: "Lyrics", path: icon.lyrics },
    { id: "settings", label: "Settings", path: icon.settings },
  ];
  const TITLE: Record<Card, string> = {
    search: "Search",
    playlists: "Your playlists",
    player: "Now playing",
    lyrics: "Lyrics",
    settings: "Settings",
  };
</script>

<section class="card" class:shown style="transform-origin: {origin}">
  <header
    role="presentation"
    onpointerdown={(e) => {
      // The header's empty space and title are a drag handle; its buttons aren't.
      if (e.button === 0 && e.buttons & 1 && !(e.target as HTMLElement).closest("button")) {
        (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
        ondrag(e);
      }
    }}
  >
    <h2>
      {#key card}<span class="title-anim">{TITLE[card]}</span>{/key}
    </h2>
    <nav>
      {#each NAV as n (n.id)}
        <button class:on={card === n.id} title={n.label} aria-label={n.label} onclick={() => onswitch(n.id)}>
          <svg viewBox="0 0 24 24"><path d={n.path} /></svg>
          {#if n.id === "settings" && app.update.state === "available"}<span class="dot"></span>{/if}
        </button>
      {/each}
      <button class="x" title="Close (Esc)" aria-label="Close" onclick={onclose}>
        <svg viewBox="0 0 24 24"><path d={icon.close} /></svg>
      </button>
    </nav>
  </header>

  <div class="body">
    {#key card}
      <div class="body-anim">{@render children()}</div>
    {/key}
  </div>

  {#if app.toast}<div class="toast">{app.toast}</div>{/if}
</section>

<style>
  .card {
    position: absolute;
    width: 320px;
    height: 428px;
    display: flex;
    flex-direction: column;
    border-radius: 22px;
    color: #fff;
    background:
      linear-gradient(rgba(10, 10, 14, 0.42), rgba(10, 10, 14, 0.42)),
      linear-gradient(150deg, var(--c1), var(--c2));
    border: 1px solid rgba(255, 255, 255, 0.14);
    box-shadow: 0 14px 40px rgba(0, 0, 0, 0.45), inset 0 1px 0 rgba(255, 255, 255, 0.1);
    overflow: hidden;
    opacity: 0;
    transform: scale(0.6);
    filter: blur(4px);
    pointer-events: none;
    transition:
      transform 200ms cubic-bezier(.5, 0, .75, 0),
      opacity 160ms ease,
      filter 160ms ease;
  }
  .card.shown {
    opacity: 1;
    transform: scale(1);
    filter: none;
    pointer-events: auto;
    transition:
      transform 420ms cubic-bezier(.3, 1.3, .5, 1),
      opacity 220ms ease,
      filter 260ms ease;
  }

  header {
    cursor: grab;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 10px 6px 16px;
  }
  h2 { margin: 0; font-size: 15px; font-weight: 700; letter-spacing: 0.01em; }
  .title-anim { display: inline-block; animation: fade-up 260ms ease; }

  nav { display: flex; gap: 2px; }
  nav button {
    position: relative;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: #fff;
    opacity: 0.6;
    cursor: pointer;
    transition: opacity 150ms, background 150ms, transform 150ms;
  }
  nav button:hover { opacity: 1; background: rgba(255, 255, 255, 0.12); }
  nav button:active { transform: scale(0.9); }
  nav button.on { opacity: 1; background: rgba(255, 255, 255, 0.2); }
  nav button svg { width: 16px; height: 16px; fill: currentColor; }
  nav .x { margin-left: 4px; }
  .dot { position: absolute; top: 3px; right: 3px; width: 7px; height: 7px; border-radius: 50%; background: #1ed760; }

  .body { flex: 1; min-height: 0; display: flex; flex-direction: column; }
  .body-anim { flex: 1; min-height: 0; display: flex; flex-direction: column; animation: fade-up 280ms ease; }
  @keyframes fade-up {
    from { opacity: 0; transform: translateY(6px); }
    to { opacity: 1; transform: none; }
  }

  .toast {
    position: absolute;
    left: 50%;
    bottom: 14px;
    transform: translateX(-50%);
    max-width: 86%;
    padding: 7px 14px;
    border-radius: 16px;
    background: rgba(0, 0, 0, 0.8);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    animation: fade-up 180ms ease;
    pointer-events: none;
  }
</style>
