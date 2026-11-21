<script lang="ts">
  // Lyrics for the current song (from LRCLIB). Synced lyrics highlight the
  // current line and keep it centred; click a line to jump there.
  import { tick } from "svelte";
  import { app } from "../app.svelte";
  import { icon } from "../icons";

  let box = $state<HTMLDivElement | null>(null);
  /** While the user scrolls by hand, stop auto-scrolling for a few seconds. */
  let userScrollUntil = 0;

  // Fetch whenever the song changes.
  $effect(() => {
    void app.title;
    void app.artist;
    app.loadLyrics();
  });

  let lines = $derived(app.lyrics.data?.synced ?? null);
  /** Index of the line being sung (shows a bit early so it's ready to read). */
  let current = $derived.by(() => {
    if (!lines) return -1;
    const t = app.positionMs + 250;
    let i = -1;
    for (let k = 0; k < lines.length; k++) {
      if (lines[k].ms <= t) i = k;
      else break;
    }
    return i;
  });

  $effect(() => {
    const i = current;
    if (i < 0 || !box || Date.now() < userScrollUntil) return;
    tick().then(() => {
      const el = box?.querySelector<HTMLElement>(`[data-i="${i}"]`);
      if (!el || !box) return;
      box.scrollTo({ top: el.offsetTop - box.clientHeight / 2 + el.clientHeight / 2, behavior: "smooth" });
    });
  });
</script>

<div
  class="lyrics scroll fade"
  bind:this={box}
  onwheel={() => (userScrollUntil = Date.now() + 4000)}
>
  {#if !app.available}
    <div class="center muted">Play a song to see its lyrics</div>
  {:else if app.lyrics.loading}
    <div class="center muted"><span class="spinner"></span></div>
  {:else if app.lyrics.error}
    <p class="error">{app.lyrics.error}</p>
  {:else if app.lyrics.data?.instrumental}
    <div class="center muted">♪ Instrumental ♪</div>
  {:else if lines}
    <div class="pad-top"></div>
    {#each lines as l, i (i)}
      <button
        class="line"
        class:now={i === current}
        class:past={i < current}
        data-i={i}
        title="Jump here"
        onclick={() => {
          userScrollUntil = 0;
          app.seek(l.ms);
        }}
      >
        {l.text || "♪"}
      </button>
    {/each}
    <div class="pad-bottom"></div>
  {:else if app.lyrics.data?.plain}
    <p class="plain">{app.lyrics.data.plain}</p>
  {:else}
    <div class="center muted empty">
      <svg viewBox="0 0 24 24"><path d={icon.lyrics} /></svg>
      No lyrics found for this song
    </div>
  {/if}
</div>
<p class="credit">Lyrics from LRCLIB</p>

<style>
  .lyrics { padding: 0 18px; display: flex; flex-direction: column; }
  .pad-top, .pad-bottom { height: 40%; flex: none; }
  .line {
    display: block;
    width: 100%;
    margin: 0;
    padding: 5px 0;
    border: none;
    background: none;
    color: #fff;
    font: inherit;
    font-size: 16px;
    font-weight: 700;
    line-height: 1.3;
    text-align: left;
    opacity: 0.38;
    cursor: pointer;
    transition: opacity 250ms, transform 250ms;
    transform-origin: left center;
  }
  .line:hover { opacity: 0.7; }
  .line.past { opacity: 0.55; }
  .line.now { opacity: 1; transform: scale(1.04); }
  .plain { white-space: pre-wrap; font-size: 14px; line-height: 1.55; margin: 8px 0 20px; opacity: 0.9; }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 8px; }
  .empty svg { width: 28px; height: 28px; fill: currentColor; opacity: 0.7; }
  .error { margin: 12px 0; }
  .credit { margin: 0; padding: 4px 18px 10px; font-size: 10px; opacity: 0.45; text-align: right; }
</style>
