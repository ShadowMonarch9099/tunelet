<script lang="ts">
  import "../app.css";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, tick, untrack } from "svelte";
  import { app } from "$lib/app.svelte";
  import Bob from "$lib/Bob.svelte";
  import Radial, { type RadialId, type RadialSpot } from "$lib/Radial.svelte";
  import CardShell from "$lib/cards/CardShell.svelte";
  import SearchCard from "$lib/cards/SearchCard.svelte";
  import PlayerCard from "$lib/cards/PlayerCard.svelte";
  import SettingsCard from "$lib/cards/SettingsCard.svelte";
  import type { Card, NowPlaying, Timeline } from "$lib/types";

  type Mode = "bob" | "radial" | "card";
  type Rect = { x: number; y: number; w: number; h: number };
  type Area = { x: number; y: number; width: number; height: number };

  // Sizes in logical pixels.
  const BOB_HALF = 40; // half of the window in bob mode (68px bob + glow room)
  const BOB_VISUAL_HALF = 34; // half of the bob + ring itself
  const CARD_W = 320;
  const CARD_H = 428;
  const R_FULL = 78; // radius of the option ring
  const R_CAPSULE = 82; // the wider playback capsule sits a touch further out
  const R_ARC = 90; // radius when fanned out in an arc near a screen edge
  // Angle gaps between options (degrees). The capsule is wide, so the
  // options next to it get a bigger gap.
  const RING_STEPS = [68, 56, 56, 56, 56]; // full ring, from the capsule at the top
  const ARC_STEPS = [54, 42, 42, 42, 42]; // arc near an edge
  const STORE_KEY = "tunelet.bob-center";

  const win = getCurrentWindow();

  let mode = $state<Mode>("bob");
  let card = $state<Card>("search");
  let radialShown = $state(false);
  let cardShown = $state(false);
  let bobAt = $state({ x: BOB_HALF, y: BOB_HALF }); // bob center inside the window
  let spots = $state<RadialSpot[]>([]);
  let cardAt = $state({ x: 0, y: 0, origin: "top left" });
  let bobRef = $state<ReturnType<typeof Bob> | null>(null);

  /** Bob center on screen (logical px) — the anchor everything is laid out around. */
  let center = { x: 0, y: 0 };
  let ignoreMovesUntil = 0;
  let queue: Promise<unknown> = Promise.resolve();

  const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
  /**
   * Waits for the next painted frame so CSS transitions start from the right
   * state. Windows stops painting hidden/covered windows, so never wait more
   * than a moment for it.
   */
  const frame = () =>
    Promise.race([new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))), wait(60)]);
  /**
   * Runs transitions one after another so rapid clicks can't tangle them.
   * A step that somehow hangs is abandoned after 3s so it can't block every
   * later transition (which would leave the widget stuck "open").
   */
  function serial(fn: () => Promise<void>) {
    const step = () => Promise.race([fn(), wait(3000)]);
    queue = queue.then(step, step);
    return queue;
  }

  async function screenArea(): Promise<Area> {
    return (await invoke<Area | null>("screen_area")) ?? { x: 0, y: 0, width: 1920, height: 1080 };
  }

  // ---- layout ------------------------------------------------------------------
  function union(rects: Rect[], margin: { x: number; y: number }): Rect {
    const x1 = Math.min(...rects.map((r) => r.x)) - margin.x;
    const y1 = Math.min(...rects.map((r) => r.y)) - margin.y;
    const x2 = Math.max(...rects.map((r) => r.x + r.w)) + margin.x;
    const y2 = Math.max(...rects.map((r) => r.y + r.h)) + margin.y;
    return { x: Math.floor(x1), y: Math.floor(y1), w: Math.ceil(x2 - x1), h: Math.ceil(y2 - y1) };
  }

  const bobRect = (): Rect => ({ x: center.x - BOB_HALF, y: center.y - BOB_HALF, w: BOB_HALF * 2, h: BOB_HALF * 2 });

  function computeSpots(a: Area): RadialSpot[] {
    const ids: RadialId[] = ["playback", "like", "search", "playlists", "player", "settings"];
    const need = R_CAPSULE + 70;
    const roomy =
      center.x - a.x >= need &&
      a.x + a.width - center.x >= need &&
      center.y - a.y >= need &&
      a.y + a.height - center.y >= need;

    const steps = roomy ? RING_STEPS : ARC_STEPS;
    const span = steps.reduce((s, v) => s + v, 0);
    // Full ring: capsule at the top, others clockwise. Near an edge: an arc
    // that faces the middle of the screen.
    const toward = (Math.atan2(a.y + a.height / 2 - center.y, a.x + a.width / 2 - center.x) * 180) / Math.PI;
    let angle = roomy ? -90 : toward - span / 2;
    const angles = [angle, ...steps.map((s) => (angle += s))];

    return ids.map((id, i) => {
      const R = id === "playback" ? (roomy ? R_CAPSULE : R_ARC + 4) : roomy ? R_FULL : R_ARC;
      const t = (angles[i] * Math.PI) / 180;
      return { id, dx: Math.round(R * Math.cos(t)), dy: Math.round(R * Math.sin(t)) };
    });
  }

  async function applyBounds(r: Rect) {
    bobAt = { x: center.x - r.x, y: center.y - r.y };
    await tick();
    ignoreMovesUntil = Date.now() + 500;
    await invoke("set_bounds", { x: r.x, y: r.y, width: r.w, height: r.h });
  }

  async function layoutBob() {
    await applyBounds(bobRect());
  }

  async function layoutRadial() {
    const a = await screenArea();
    spots = computeSpots(a);
    const rects = spots.map((s) => {
      const hw = s.id === "playback" ? 56 : 22;
      return { x: center.x + s.dx - hw, y: center.y + s.dy - 22, w: hw * 2, h: 44 };
    });
    // Extra room on the sides for hover labels.
    await applyBounds(union([bobRect(), ...rects], { x: 84, y: 34 }));
  }

  async function layoutCard() {
    const a = await screenArea();
    const right = center.x < a.x + a.width / 2;
    const down = center.y < a.y + a.height / 2;
    let x = right ? center.x + BOB_VISUAL_HALF + 8 : center.x - BOB_VISUAL_HALF - 8 - CARD_W;
    let y = down ? center.y - BOB_VISUAL_HALF : center.y + BOB_VISUAL_HALF - CARD_H;
    x = Math.min(Math.max(x, a.x + 8), a.x + a.width - CARD_W - 8);
    y = Math.min(Math.max(y, a.y + 8), a.y + a.height - CARD_H - 8);
    const cardRect = { x, y, w: CARD_W, h: CARD_H };
    const r = union([bobRect(), cardRect], { x: 16, y: 16 });
    cardAt = {
      x: x - r.x,
      y: y - r.y,
      // Grow out of the bob.
      origin: `${Math.round(center.x - x)}px ${Math.round(center.y - y)}px`,
    };
    await applyBounds(r);
  }

  // ---- mode transitions ----------------------------------------------------------
  function openRadial() {
    return serial(async () => {
      if (mode === "radial") return;
      if (mode === "card") {
        cardShown = false;
        await wait(200);
      }
      await layoutRadial();
      mode = "radial";
      await invoke("set_interactive", { interactive: true });
      bobRef?.spin(1);
      await frame();
      radialShown = true;
      app.refreshPlayer();
    });
  }

  function openCard(c: Card) {
    return serial(async () => {
      card = c;
      app.error = "";
      if (mode === "card") return; // just switch the card's contents
      if (mode === "radial") {
        radialShown = false;
        await wait(230);
      } else {
        bobRef?.spin(1);
      }
      await layoutCard();
      mode = "card";
      await invoke("set_interactive", { interactive: true });
      await frame();
      cardShown = true;
    });
  }

  function closeAll() {
    return serial(async () => {
      if (mode === "bob") return;
      if (app.recording) return;
      bobRef?.spin(-1);
      if (mode === "radial") {
        radialShown = false;
        await wait(280);
      } else {
        cardShown = false;
        await wait(210);
      }
      mode = "bob";
      app.devices = null;
      await layoutBob();
      await invoke("set_interactive", { interactive: false });
    });
  }

  function toggleMenu() {
    if (mode === "bob") openRadial();
    else closeAll();
  }

  function pick(id: RadialId) {
    if (id === "like") app.toggleLike();
    else if (id !== "playback") openCard(id);
  }

  // ---- startup / events ---------------------------------------------------------------
  onMount(() => {
    app.onRequestClose = () => closeAll();

    const offs = [
      listen<NowPlaying>("now-playing", (e) => (app.np = e.payload)),
      listen<Timeline>("timeline", (e) => {
        app.tl = { ...e.payload, at: performance.now() };
        app.clock = app.tl.at;
      }),
      listen<{ liked?: boolean; error?: string }>("like-result", (e) => app.likeResult(e.payload)),
      listen("open-search", () => openCard("search")),
      listen("open-settings", () => openCard("settings")),
      win.onMoved(async ({ payload }) => {
        // Only user drags in bob mode move the anchor; our own resizes don't.
        if (mode !== "bob" || Date.now() < ignoreMovesUntil) return;
        const p = payload.toLogical(await win.scaleFactor());
        center = { x: p.x + BOB_HALF, y: p.y + BOB_HALF };
        saveCenter();
      }),
    ];

    const ticker = setInterval(() => (app.clock = performance.now()), 250);
    const poll = setInterval(() => {
      if (mode === "card" && card === "player") app.refreshPlayer();
    }, 5000);

    (async () => {
      await restoreCenter();
      await layoutBob();
    })();
    invoke("media", { action: "refresh" });
    app.loadAuth();
    app.loadSettings();
    app.checkUpdate(true);

    return () => {
      offs.forEach((o) => o.then((f) => f()));
      clearInterval(ticker);
      clearInterval(poll);
    };
  });

  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  function saveCenter() {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      try {
        localStorage.setItem(STORE_KEY, JSON.stringify(center));
      } catch {}
    }, 300);
  }

  async function restoreCenter() {
    const a = await screenArea();
    let saved: { x: number; y: number } | null = null;
    try {
      saved = JSON.parse(localStorage.getItem(STORE_KEY) ?? "null");
    } catch {}
    if (!saved) {
      const p = (await win.outerPosition()).toLogical(await win.scaleFactor());
      saved = { x: p.x + BOB_HALF, y: p.y + BOB_HALF };
    }
    // Keep it on screen (e.g. after unplugging a monitor).
    center = {
      x: Math.min(Math.max(saved.x, a.x + BOB_HALF), a.x + a.width - BOB_HALF),
      y: Math.min(Math.max(saved.y, a.y + BOB_HALF), a.y + a.height - BOB_HALF),
    };
  }

  // When the song changes, ask Spotify whether it's liked.
  let trackKey = $derived(app.available ? `${app.title}\u0001${app.artist}` : "");
  $effect(() => {
    const key = trackKey;
    const loggedIn = app.auth?.loggedIn;
    untrack(() => {
      app.liked = null;
      if (key && loggedIn) setTimeout(() => app.refreshPlayer(), 900);
    });
  });

  function onKey(e: KeyboardEvent) {
    if (app.recording) return; // Settings is recording a shortcut
    if (e.key === "Escape") {
      if (mode !== "bob") closeAll();
      else invoke("hide_widget");
    }
  }

  function onBlur() {
    // Clicking into another app closes the menu/card — except while the
    // browser has focus during Spotify login, or a shortcut is being recorded.
    if (app.loggingIn || app.recording) return;
    if (mode !== "bob") closeAll();
  }

  function onStageDown(e: PointerEvent) {
    // Clicking the empty space around the bob/menu closes it.
    if (e.target === e.currentTarget && mode !== "bob") closeAll();
  }
</script>

<svelte:window onkeydown={onKey} onblur={onBlur} oncontextmenu={(e) => e.preventDefault()} />

<div
  class="stage"
  style="--c1: {app.colors[0]}; --c2: {app.colors[1]}; opacity: {app.opacity}"
  role="presentation"
  onpointerdown={onStageDown}
>
  {#if mode === "card"}
    <div class="card-pos" style="left: {cardAt.x}px; top: {cardAt.y}px">
      <CardShell {card} shown={cardShown} origin={cardAt.origin} onswitch={(c) => openCard(c)} onclose={closeAll}>
        {#if card === "search" || card === "playlists"}
          <SearchCard tab={card} onswitch={(t) => openCard(t)} />
        {:else if card === "player"}
          <PlayerCard />
        {:else}
          <SettingsCard />
        {/if}
      </CardShell>
    </div>
  {/if}

  <div class="anchor" style="left: {bobAt.x}px; top: {bobAt.y}px">
    {#if mode === "radial"}
      <Radial {spots} shown={radialShown} onpick={pick} />
    {/if}
    <div class="bob-pos">
      <Bob bind:this={bobRef} open={mode !== "bob"} draggable={mode === "bob"} onmenu={toggleMenu} />
    </div>
  </div>

  {#if mode === "radial" && app.toast}
    <div class="radial-toast" style="left: {bobAt.x}px; top: {bobAt.y + R_FULL + 30}px">{app.toast}</div>
  {/if}
</div>

<style>
  .stage {
    position: fixed;
    inset: 0;
    transition: --c1 700ms ease, --c2 700ms ease;
  }
  .anchor { position: absolute; width: 0; height: 0; }
  .bob-pos { position: absolute; left: -34px; top: -34px; }
  .card-pos { position: absolute; }

  .radial-toast {
    position: absolute;
    transform: translateX(-50%);
    padding: 5px 12px;
    border-radius: 14px;
    background: rgba(0, 0, 0, 0.8);
    font-size: 11.5px;
    white-space: nowrap;
    pointer-events: none;
    animation: toast-in 200ms ease;
  }
  @keyframes toast-in { from { opacity: 0; transform: translate(-50%, 4px); } }
</style>
