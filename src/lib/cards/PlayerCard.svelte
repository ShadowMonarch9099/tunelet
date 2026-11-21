<script lang="ts">
  import { onMount } from "svelte";
  import { app, fmtTime } from "../app.svelte";
  import { icon } from "../icons";

  let bar = $state<HTMLDivElement | null>(null);
  let lastVolume = 50;

  /** Scroll over the volume row to change it in 2% steps (non-passive so the card doesn't scroll). */
  function wheelVolume(node: HTMLElement) {
    const onWheel = (e: WheelEvent) => {
      if (app.volume === null) return;
      e.preventDefault();
      app.stepVolume(e.deltaY < 0 ? 2 : -2);
    };
    node.addEventListener("wheel", onWheel, { passive: false });
    return { destroy: () => node.removeEventListener("wheel", onWheel) };
  }
  let dragFrac = $state<number | null>(null);
  let shownFrac = $derived(dragFrac ?? app.progress);

  onMount(() => {
    app.refreshPlayer();
    if (app.auth?.loggedIn) app.loadDevices();
  });

  function frac(e: PointerEvent) {
    const r = bar!.getBoundingClientRect();
    return Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
  }
  function down(e: PointerEvent) {
    if (!app.durationMs) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    dragFrac = frac(e);
  }
  function move(e: PointerEvent) {
    if (dragFrac !== null) dragFrac = frac(e);
  }
  function up(e: PointerEvent) {
    if (dragFrac === null) return;
    const ms = Math.round(frac(e) * app.durationMs);
    dragFrac = null;
    app.seek(ms);
  }

  function deviceIcon(kind: string) {
    if (kind === "Smartphone") return icon.phone;
    if (kind === "Computer") return icon.device;
    return icon.speaker;
  }
</script>

<div class="now scroll">
  <div class="cover-wrap">
    {#if app.art}
      {#key app.art}
        <img class="cover" class:spinning={app.playing} src={app.art} alt="" draggable="false" />
      {/key}
    {:else}
      <div class="cover empty"><svg viewBox="0 0 24 24"><path d={icon.note} /></svg></div>
    {/if}
  </div>

  {#key app.title}
    <div class="titles">
      <div class="t">{app.title}</div>
      <div class="a">{app.artist}</div>
    </div>
  {/key}

  <div
    class="seek"
    class:dragging={dragFrac !== null}
    class:disabled={!app.durationMs}
    bind:this={bar}
    role="slider"
    tabindex="-1"
    aria-label="Seek"
    aria-valuemin={0}
    aria-valuemax={app.durationMs}
    aria-valuenow={app.positionMs}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
  >
    <div class="track"><div class="fill" style="width: {shownFrac * 100}%"></div></div>
    <div class="knob" style="left: {shownFrac * 100}%"></div>
  </div>
  <div class="times">
    <span>{fmtTime(dragFrac !== null ? dragFrac * app.durationMs : app.positionMs)}</span>
    <span>{app.durationMs ? fmtTime(app.durationMs) : "–:–"}</span>
  </div>

  <div class="transport">
    <button
      class="side like"
      class:liked={app.liked === true}
      aria-label="Like"
      title="Like ({app.hk('like')})"
      disabled={!app.available || app.likeBusy}
      onclick={() => app.toggleLike()}
    >
      <svg viewBox="0 0 24 24"><path d={app.liked ? icon.heart : icon.heartOutline} /></svg>
    </button>
    <button aria-label="Previous" onclick={() => app.media("previous")}>
      <svg viewBox="0 0 24 24"><path d={icon.prev} /></svg>
    </button>
    <button class="big" aria-label={app.playing ? "Pause" : "Play"} onclick={() => app.media("toggle")}>
      <svg viewBox="0 0 24 24"><path d={app.playing ? icon.pause : icon.play} /></svg>
    </button>
    <button aria-label="Next" onclick={() => app.media("next")}>
      <svg viewBox="0 0 24 24"><path d={icon.next} /></svg>
    </button>
    <button
      class="side shuffle"
      class:on={app.shuffle || app.smartShuffle}
      aria-label="Shuffle"
      aria-pressed={app.shuffle || app.smartShuffle}
      title={app.smartShuffle
        ? "Smart Shuffle is on. Click to turn shuffle off"
        : app.shuffle
          ? "Shuffle is on. Click to turn it off"
          : "Shuffle"}
      disabled={!app.auth?.loggedIn}
      onclick={() => app.toggleShuffle()}
    >
      <svg viewBox="0 0 24 24"><path d={icon.shuffle} /></svg>
      {#if app.smartShuffle}<span class="sparkle" aria-hidden="true">✦</span>{/if}
    </button>
  </div>

  {#if app.auth?.loggedIn}
    <div class="vol" use:wheelVolume>
      <button
        class="mute"
        aria-label="Mute"
        title="Mute / unmute"
        disabled={app.volume === null}
        onclick={() => {
          if (app.volume === null) return;
          if (app.volume > 0) {
            lastVolume = app.volume;
            app.setVolume(0);
          } else {
            app.setVolume(lastVolume || 50);
          }
        }}
      >
        <svg viewBox="0 0 24 24"><path d={app.volume === 0 ? icon.muted : icon.volume} /></svg>
      </button>
      <input
        class="range"
        type="range"
        min="0"
        max="100"
        step="1"
        style="--v: {app.volume ?? 50}%"
        value={app.volume ?? 50}
        disabled={app.volume === null || app.player?.device?.supportsVolume === false}
        title={app.volume === null ? "Play something to change volume" : "Drag, scroll or use arrow keys"}
        oninput={(e) => app.setVolume(+e.currentTarget.value)}
      />
      <span class="pct">{app.volume ?? "–"}%</span>
    </div>

    <div class="section-title">Play on</div>
    <div class="devices">
      {#if app.devices === null}
        <div class="muted small">Loading devices…</div>
      {:else}
        {#each app.devices as d (d.id)}
          <button class="device" class:current={d.active} onclick={() => app.transfer(d)}>
            <svg viewBox="0 0 24 24"><path d={deviceIcon(d.kind)} /></svg>
            <span class="device-name">{d.name}</span>
            {#if d.active}<span class="eq"><i></i><i></i><i></i></span>{/if}
          </button>
        {:else}
          <div class="muted small">No devices found. Open Spotify on a phone, speaker or PC.</div>
        {/each}
      {/if}
    </div>
  {:else}
    <p class="muted small center-text">Connect Spotify (in Search) to like songs, change volume and switch devices.</p>
  {/if}
  {#if app.error}<p class="error">{app.error}</p>{/if}
</div>

<style>
  .now { padding: 4px 18px 16px; display: flex; flex-direction: column; align-items: stretch; }

  .cover-wrap { display: grid; place-items: center; margin: 2px 0 10px; }
  .cover {
    width: 112px;
    height: 112px;
    border-radius: 50%;
    object-fit: cover;
    box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.15), 0 10px 30px rgba(0, 0, 0, 0.5), 0 0 30px color-mix(in srgb, var(--c1) 60%, transparent);
    animation: vinyl 14s linear infinite, pop-in 450ms cubic-bezier(.3, 1.3, .5, 1);
    animation-play-state: paused, running;
  }
  .cover.spinning { animation-play-state: running, running; }
  .cover.empty { display: grid; place-items: center; background: radial-gradient(circle at 30% 30%, var(--c1), var(--c2)); }
  .cover.empty svg { width: 40px; height: 40px; fill: rgba(255, 255, 255, 0.8); }
  @keyframes vinyl { to { transform: rotate(360deg); } }
  @keyframes pop-in { from { transform: scale(0.8); opacity: 0; } }

  .titles { text-align: center; animation: fade 300ms ease; }
  .t, .a { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .t { font-size: 15px; font-weight: 700; }
  .a { font-size: 12.5px; opacity: 0.7; margin-top: 1px; }
  @keyframes fade { from { opacity: 0; transform: translateY(4px); } }

  .seek { position: relative; height: 18px; margin-top: 8px; cursor: pointer; touch-action: none; }
  .seek.disabled { opacity: 0.4; cursor: default; }
  .track { position: absolute; left: 0; right: 0; top: 7px; height: 4px; border-radius: 2px; background: rgba(255, 255, 255, 0.18); overflow: hidden; }
  .fill { height: 100%; background: #fff; border-radius: 2px; }
  .knob {
    position: absolute;
    top: 3px;
    width: 12px;
    height: 12px;
    margin-left: -6px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
    transform: scale(0);
    transition: transform 150ms ease;
  }
  .seek:hover .knob, .seek.dragging .knob { transform: scale(1); }
  .times { display: flex; justify-content: space-between; font-size: 10.5px; opacity: 0.65; font-variant-numeric: tabular-nums; }

  .transport { display: flex; align-items: center; justify-content: center; gap: 10px; margin: 4px 0 8px; }
  .transport button {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: #fff;
    cursor: pointer;
    transition: background 150ms, transform 150ms;
  }
  .transport button:hover { background: rgba(255, 255, 255, 0.14); }
  .transport button:active { transform: scale(0.9); }
  .transport button:disabled { opacity: 0.4; cursor: default; }
  .transport svg { width: 20px; height: 20px; fill: currentColor; }
  .transport .big { width: 48px; height: 48px; background: #fff; color: #111; }
  .transport .big:hover { background: #fff; transform: scale(1.06); }
  .transport .big svg { width: 24px; height: 24px; }
  .side { width: 36px; }
  .like.liked { color: #1ed760; }
  .shuffle { position: relative; }
  .shuffle svg { width: 18px; height: 18px; }
  .shuffle.on { color: #1ed760; }
  /* Small dot under an active toggle, like Spotify's own. */
  .shuffle.on::after {
    content: "";
    position: absolute;
    bottom: 3px;
    left: 50%;
    width: 4px;
    height: 4px;
    margin-left: -2px;
    border-radius: 50%;
    background: currentColor;
  }
  .sparkle { position: absolute; top: 3px; right: 3px; font-size: 9px; line-height: 1; }
  .like.liked svg { animation: pop 350ms ease; }
  @keyframes pop { 50% { transform: scale(1.35); } }

  .vol { display: flex; align-items: center; gap: 6px; font-size: 11px; }
  .mute {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: #fff;
    cursor: pointer;
    flex: none;
    transition: background 150ms;
  }
  .mute:hover { background: rgba(255, 255, 255, 0.12); }
  .mute:disabled { opacity: 0.4; cursor: default; }
  .mute svg { width: 16px; height: 16px; fill: currentColor; }

  .vol .range { flex: 1; }

  .section-title { font-size: 10.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; opacity: 0.6; margin: 12px 0 4px; }
  .devices { display: flex; flex-direction: column; gap: 2px; }
  .device {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 10px;
    border: none;
    border-radius: 10px;
    background: transparent;
    color: #fff;
    font: inherit;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
    transition: background 150ms;
  }
  .device:hover { background: rgba(255, 255, 255, 0.1); }
  .device.current { background: rgba(255, 255, 255, 0.16); color: #1ed760; }
  .device svg { width: 16px; height: 16px; fill: currentColor; flex: none; }
  .device-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .eq { display: flex; align-items: flex-end; gap: 2px; height: 12px; }
  .eq i { width: 3px; background: #1ed760; border-radius: 1px; animation: eq 900ms ease-in-out infinite; }
  .eq i:nth-child(2) { animation-delay: -300ms; }
  .eq i:nth-child(3) { animation-delay: -600ms; }
  @keyframes eq { 0%, 100% { height: 3px; } 50% { height: 12px; } }

  .small { font-size: 11.5px; }
  .center-text { text-align: center; margin-top: 8px; }
</style>
