<script lang="ts" module>
  export type RadialId = "playback" | "like" | "search" | "playlists" | "player" | "settings";
  export type RadialSpot = { id: RadialId; dx: number; dy: number };
</script>

<script lang="ts">
  // Options around the bob. Rendered at the bob's center; each option flies
  // out to (dx, dy) with a staggered spring, and flies back in when closing.
  import { app } from "./app.svelte";
  import { icon } from "./icons";
  import type { Card } from "./types";

  let {
    spots,
    shown,
    activeCard = null,
    onpick,
  }: {
    spots: RadialSpot[];
    shown: boolean;
    activeCard?: Card | null;
    onpick: (id: RadialId) => void;
  } = $props();

  const LABEL: Record<RadialId, string> = {
    playback: "",
    like: "Like",
    search: "Search",
    playlists: "Playlists",
    player: "Now playing",
    settings: "Settings",
  };
  const ICON: Record<Exclude<RadialId, "playback">, string> = {
    like: icon.heartOutline,
    search: icon.search,
    playlists: icon.playlist,
    player: icon.nowPlaying,
    settings: icon.settings,
  };

  // Labels sit on the outer side of each bubble so they never cover the bob.
  function labelSide(s: RadialSpot) {
    if (Math.abs(s.dx) < 20) return s.dy < 0 ? "top" : "bottom";
    return s.dx > 0 ? "right" : "left";
  }
</script>

{#each spots as s, i (s.id)}
  <div
    class="spot"
    class:shown
    class:capsule={s.id === "playback"}
    style="--dx: {s.dx}px; --dy: {s.dy}px; --i: {i}; --n: {spots.length}"
  >
    {#if s.id === "playback"}
      <div class="capsule-inner">
        <button aria-label="Previous" title="Previous ({app.hk('previous')})" onclick={() => app.media("previous")}>
          <svg viewBox="0 0 24 24"><path d={icon.prev} /></svg>
        </button>
        <button
          class="main"
          aria-label={app.playing ? "Pause" : "Play"}
          title="Play / pause ({app.hk('playPause')})"
          onclick={() => app.media("toggle")}
        >
          <svg viewBox="0 0 24 24"><path d={app.playing ? icon.pause : icon.play} /></svg>
        </button>
        <button aria-label="Next" title="Next ({app.hk('next')})" onclick={() => app.media("next")}>
          <svg viewBox="0 0 24 24"><path d={icon.next} /></svg>
        </button>
      </div>
    {:else}
      <button
        class="bubble"
        class:active={activeCard === s.id}
        class:liked={s.id === "like" && app.liked === true}
        aria-label={LABEL[s.id]}
        disabled={s.id === "like" && app.likeBusy}
        onclick={() => onpick(s.id)}
      >
        <svg viewBox="0 0 24 24">
          <path d={s.id === "like" && app.liked ? icon.heart : ICON[s.id]} />
        </svg>
        <span class="label {labelSide(s)}">
          {s.id === "like" && app.liked ? "Liked" : LABEL[s.id]}
        </span>
        {#if s.id === "settings" && app.update.state === "available"}<span class="dot"></span>{/if}
      </button>
    {/if}
  </div>
{/each}

<style>
  .spot {
    position: absolute;
    left: 0;
    top: 0;
    opacity: 0;
    pointer-events: none;
    transform: translate(-50%, -50%) translate(0, 0) scale(0.2) rotate(-90deg);
    /* closing: quick, reverse order */
    transition:
      transform 220ms cubic-bezier(.5, 0, .75, 0) calc((var(--n) - var(--i)) * 18ms),
      opacity 180ms ease calc((var(--n) - var(--i)) * 18ms);
  }
  .spot.shown {
    opacity: 1;
    pointer-events: auto;
    transform: translate(-50%, -50%) translate(var(--dx), var(--dy)) scale(1) rotate(0deg);
    /* opening: springy, staggered clockwise */
    transition:
      transform 520ms cubic-bezier(.34, 1.56, .64, 1) calc(80ms + var(--i) * 45ms),
      opacity 220ms ease calc(80ms + var(--i) * 45ms);
  }

  button {
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    color: #fff;
    cursor: pointer;
    font: inherit;
  }
  svg { fill: currentColor; }

  .bubble,
  .capsule-inner {
    background:
      linear-gradient(rgba(12, 12, 16, 0.45), rgba(12, 12, 16, 0.45)),
      linear-gradient(135deg, var(--c1), var(--c2));
    border: 1px solid rgba(255, 255, 255, 0.16);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.12);
  }

  .bubble {
    position: relative;
    width: 46px;
    height: 46px;
    border-radius: 50%;
    transition: transform 200ms cubic-bezier(.3, 1.5, .5, 1), background 200ms;
  }
  .bubble svg { width: 20px; height: 20px; transition: transform 200ms; }
  .bubble:hover { transform: scale(1.12); }
  .bubble:active { transform: scale(0.92); }
  .bubble.active { box-shadow: 0 0 0 2px #fff, 0 6px 18px rgba(0, 0, 0, 0.4); }
  .bubble.liked { color: #1ed760; }
  .bubble.liked svg { animation: pop 350ms ease; }
  .bubble:disabled { cursor: default; }
  @keyframes pop { 50% { transform: scale(1.35); } }

  .label {
    position: absolute;
    white-space: nowrap;
    font-size: 11px;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 9px;
    background: rgba(0, 0, 0, 0.75);
    opacity: 0;
    pointer-events: none;
    transition: opacity 150ms, transform 150ms;
  }
  .label.right { left: calc(100% + 6px); transform: translateX(-4px); }
  .label.left { right: calc(100% + 6px); transform: translateX(4px); }
  .label.top { bottom: calc(100% + 6px); transform: translateY(4px); }
  .label.bottom { top: calc(100% + 6px); transform: translateY(-4px); }
  .bubble:hover .label { opacity: 1; transform: none; }

  .dot {
    position: absolute;
    top: 2px;
    right: 2px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #1ed760;
    box-shadow: 0 0 0 2px rgba(0, 0, 0, 0.5);
  }

  .capsule-inner {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 46px;
    padding: 0 6px;
    border-radius: 23px;
  }
  .capsule-inner button {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: transparent;
    transition: background 150ms, transform 150ms;
  }
  .capsule-inner button svg { width: 18px; height: 18px; }
  .capsule-inner button:hover { background: rgba(255, 255, 255, 0.16); }
  .capsule-inner button:active { transform: scale(0.9); }
  .capsule-inner .main { width: 38px; height: 38px; background: rgba(255, 255, 255, 0.95); color: #111; }
  .capsule-inner .main:hover { background: #fff; }
  .capsule-inner .main svg { width: 20px; height: 20px; }
</style>
