<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  type NowPlaying = {
    available: boolean;
    title: string;
    artist: string;
    album: string;
    playing: boolean;
    art: string | null;
    colors: [string, string] | null;
  };

  const IDLE: [string, string] = ["#3a2a5c", "#12324a"];

  let np = $state<NowPlaying | null>(null);
  let title = $derived(np?.available ? np.title : "Nothing playing");
  let artist = $derived(np?.available ? np.artist : "Open Spotify to start");
  let playing = $derived(np?.available ? np.playing : false);
  let art = $derived(np?.available ? np.art : null);
  let c1 = $derived(np?.available && np.colors ? np.colors[0] : IDLE[0]);
  let c2 = $derived(np?.available && np.colors ? np.colors[1] : IDLE[1]);

  onMount(() => {
    const unlisten = listen<NowPlaying>("now-playing", (e) => (np = e.payload));
    invoke("media", { action: "refresh" });
    return () => {
      unlisten.then((f) => f());
    };
  });

  function media(action: "toggle" | "next" | "previous") {
    // Flip the icon right away; the real state arrives a moment later.
    if (action === "toggle" && np?.available) np = { ...np, playing: !np.playing };
    invoke("media", { action });
  }

  function hide() {
    invoke("hide_widget");
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") hide();
  }
</script>

<svelte:window onkeydown={onKey} />

<main class="pill" style="--c1: {c1}; --c2: {c2}" data-tauri-drag-region>
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
  </div>

  <button class="close" aria-label="Hide" title="Hide (Ctrl+Alt+S)" onclick={hide}>×</button>
</main>

<style>
  @property --c1 { syntax: "<color>"; inherits: true; initial-value: #222; }
  @property --c2 { syntax: "<color>"; inherits: true; initial-value: #111; }

  :global(*) { box-sizing: border-box; }

  .pill {
    position: relative;
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px 8px 8px;
    border-radius: 18px;
    color: #fff;
    font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
    background:
      linear-gradient(rgba(10, 10, 14, 0.35), rgba(10, 10, 14, 0.35)),
      linear-gradient(120deg, var(--c1), var(--c2));
    border: 1px solid rgba(255, 255, 255, 0.12);
    transition: --c1 600ms ease, --c2 600ms ease;
    user-select: none;
    overflow: hidden;
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
  .title, .artist { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .title { font-size: 13px; font-weight: 600; }
  .artist { font-size: 11.5px; opacity: 0.75; }

  .controls { display: flex; align-items: center; gap: 2px; }

  button {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: inherit;
    cursor: pointer;
    transition: background 120ms, transform 120ms;
  }
  button:hover { background: rgba(255, 255, 255, 0.14); }
  button:active { transform: scale(0.92); }
  button svg { width: 18px; height: 18px; fill: currentColor; }

  .play { width: 34px; height: 34px; background: rgba(255, 255, 255, 0.92); color: #111; }
  .play:hover { background: #fff; }

  .close {
    position: absolute;
    top: 3px;
    right: 5px;
    width: 16px;
    height: 16px;
    font-size: 13px;
    line-height: 1;
    opacity: 0;
    transition: opacity 150ms;
  }
  .pill:hover .close { opacity: 0.7; }
</style>
