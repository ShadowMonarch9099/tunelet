<script lang="ts">
  // The round widget: album art (spins like a vinyl while playing), a glow in
  // the album's colors, and a progress ring. Left-click opens the menu,
  // right-click plays/pauses, dragging moves the widget.
  import { app } from "./app.svelte";
  import { icon } from "./icons";

  let {
    open = false,
    onmenu,
    ondrag,
  }: { open?: boolean; onmenu: () => void; ondrag: (e: PointerEvent) => void } = $props();

  const SIZE = 52;
  const RING_R = 30;
  const RING_C = 2 * Math.PI * RING_R;

  let el = $state<HTMLDivElement | null>(null);
  let down: { x: number; y: number } | null = null;
  let pressed = $state(false);

  /** Full spin with a little bounce — played when the menu opens/closes. */
  export function spin(direction: 1 | -1 = 1) {
    el?.animate(
      [
        { transform: "rotate(0deg) scale(1)" },
        { transform: `rotate(${direction * 200}deg) scale(1.12)`, offset: 0.55 },
        { transform: `rotate(${direction * 360}deg) scale(1)` },
      ],
      { duration: 720, easing: "cubic-bezier(.2,.75,.25,1)" },
    );
  }

  function onpointerdown(e: PointerEvent) {
    if (e.button !== 0) return;
    down = { x: e.screenX, y: e.screenY };
    pressed = true;
    // Keep receiving moves and the release even when a quick flick leaves the
    // bubble; otherwise the press gets "stuck" and a later hover starts a drag.
    el?.setPointerCapture(e.pointerId);
  }

  function onpointermove(e: PointerEvent) {
    if (!down) return;
    // Button no longer held (release was missed somehow): forget the press.
    if (!(e.buttons & 1)) return reset(e);
    if (Math.hypot(e.screenX - down.x, e.screenY - down.y) > 5) {
      // Keep the pointer captured on macOS, where the drag is done by hand
      // and needs the moves; reset() only clears the "pressed" look.
      down = null;
      pressed = false;
      ondrag(e);
    }
  }

  function onpointerup(e: PointerEvent) {
    if (e.button !== 0) return;
    const click = !!down;
    reset(e);
    if (click) onmenu();
  }

  function reset(e?: PointerEvent) {
    down = null;
    pressed = false;
    if (e && el?.hasPointerCapture(e.pointerId)) el.releasePointerCapture(e.pointerId);
  }

  function oncontextmenu(e: MouseEvent) {
    e.preventDefault();
    // Works even when Spotify is closed: Tunelet starts it hidden and plays.
    app.media("toggle");
    el?.animate(
      [{ transform: "scale(1)" }, { transform: "scale(.88) rotate(-12deg)" }, { transform: "scale(1)" }],
      { duration: 320, easing: "cubic-bezier(.3,1.4,.5,1)" },
    );
  }

  let ringOffset = $derived(RING_C * (1 - app.progress));
</script>

<div
  class="bob-wrap"
  class:open
  style="--c1: {app.colors[0]}; --c2: {app.colors[1]}"
  title={open ? undefined : app.available
    ? `${app.title} — ${app.artist}\nClick: menu · Right-click: ${app.playing ? "pause" : "play"}`
    : app.waking
      ? "Starting Spotify…"
      : `Tunelet — click for menu${app.canWake ? " · Right-click: start Spotify and play" : ""}`}
>
  <svg class="ring" viewBox="0 0 68 68" aria-hidden="true">
    <circle cx="34" cy="34" r={RING_R} class="ring-bg" />
    {#if app.waking}
      <circle cx="34" cy="34" r={RING_R} class="ring-wake" stroke-dasharray="{RING_C * 0.22} {RING_C}" />
    {/if}
    {#if app.durationMs > 0}
      <circle
        cx="34"
        cy="34"
        r={RING_R}
        class="ring-fg"
        stroke-dasharray={RING_C}
        stroke-dashoffset={ringOffset}
      />
    {/if}
  </svg>

  <div
    bind:this={el}
    class="bob"
    class:pressed
    style="width: {SIZE}px; height: {SIZE}px"
    role="button"
    tabindex="-1"
    aria-label="Open menu"
    {onpointerdown}
    {onpointermove}
    {onpointerup}
    onpointercancel={() => reset()}
    onlostpointercapture={() => reset()}
    {oncontextmenu}
  >
    {#if app.art}
      {#key app.art}
        <img class="art" class:spinning={app.playing} src={app.art} alt="" draggable="false" />
      {/key}
    {:else}
      <div class="placeholder">
        <svg viewBox="0 0 24 24"><path d={icon.note} /></svg>
      </div>
    {/if}
    <div class="hole"></div>

    {#if !open && (app.available || app.canWake)}
      <div class="hover">
        <svg viewBox="0 0 24 24"><path d={app.playing ? icon.pause : icon.play} /></svg>
      </div>
    {/if}

    {#if app.likePulse > 0}
      {#key app.likePulse}
        <div class="like-burst" class:unliked={!app.liked}>
          <svg viewBox="0 0 24 24"><path d={app.liked ? icon.heart : icon.heartOutline} /></svg>
        </div>
      {/key}
    {/if}
  </div>
</div>

<style>
  .bob-wrap {
    position: relative;
    width: 68px;
    height: 68px;
    display: grid;
    place-items: center;
    transition: --c1 700ms ease, --c2 700ms ease;
  }

  .ring {
    position: absolute;
    inset: 0;
    transform: rotate(-90deg);
    pointer-events: none;
  }
  .ring circle { fill: none; stroke-width: 2.5; }
  .ring-bg { stroke: rgba(255, 255, 255, 0.14); }
  /* A short arc chasing round the ring while Spotify starts up. */
  .ring-wake {
    stroke: rgba(255, 255, 255, 0.85);
    stroke-linecap: round;
    transform-origin: 34px 34px;
    animation: wake 1.1s linear infinite;
  }
  @keyframes wake { to { transform: rotate(360deg); } }
  .ring-fg {
    stroke: rgba(255, 255, 255, 0.9);
    stroke-linecap: round;
    transition: stroke-dashoffset 260ms linear;
  }

  .bob {
    position: relative;
    border-radius: 50%;
    overflow: hidden;
    cursor: pointer;
    background: radial-gradient(circle at 30% 30%, var(--c1), var(--c2));
    box-shadow:
      0 0 0 1.5px rgba(255, 255, 255, 0.18),
      0 0 14px 2px color-mix(in srgb, var(--c1) 70%, transparent),
      0 6px 16px rgba(0, 0, 0, 0.45);
    transition: transform 220ms cubic-bezier(.3, 1.4, .5, 1), box-shadow 300ms ease;
    touch-action: none;
  }
  .bob:hover { transform: scale(1.06); }
  .bob.pressed { transform: scale(0.94); }
  .open .bob {
    box-shadow:
      0 0 0 2px rgba(255, 255, 255, 0.35),
      0 0 20px 5px color-mix(in srgb, var(--c1) 80%, transparent),
      0 6px 16px rgba(0, 0, 0, 0.45);
  }

  .art {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    animation: vinyl 14s linear infinite, art-in 500ms ease;
    animation-play-state: paused, running;
  }
  .art.spinning { animation-play-state: running, running; }
  @keyframes vinyl { to { transform: rotate(360deg); } }
  @keyframes art-in {
    from { opacity: 0; filter: blur(6px); }
    to { opacity: 1; filter: none; }
  }

  .placeholder {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
  }
  .placeholder svg { width: 22px; height: 22px; fill: rgba(255, 255, 255, 0.85); }

  /* little spindle hole, vinyl style */
  .hole {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 6px;
    height: 6px;
    margin: -3px 0 0 -3px;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.55);
    box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.25);
    pointer-events: none;
  }

  .hover {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.42);
    opacity: 0;
    transition: opacity 180ms ease;
    pointer-events: none;
  }
  .hover svg {
    width: 22px;
    height: 22px;
    fill: #fff;
    transform: scale(0.7);
    transition: transform 220ms cubic-bezier(.3, 1.5, .5, 1);
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.5));
  }
  .bob:hover .hover { opacity: 1; }
  .bob:hover .hover svg { transform: scale(1); }

  .like-burst {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    pointer-events: none;
    animation: burst 900ms ease forwards;
  }
  .like-burst svg { width: 26px; height: 26px; fill: #1ed760; filter: drop-shadow(0 0 6px rgba(0, 0, 0, 0.6)); }
  .like-burst.unliked svg { fill: #fff; }
  @keyframes burst {
    0% { opacity: 0; transform: scale(0.3); }
    25% { opacity: 1; transform: scale(1.25); }
    60% { opacity: 1; transform: scale(1); }
    100% { opacity: 0; transform: scale(1.1); }
  }
</style>
