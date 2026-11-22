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
  import LyricsCard from "$lib/cards/LyricsCard.svelte";
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
  // Near an edge or corner the options fill rings around the bob, inner ring
  // first, so they never overlap or leave the screen (they stack outward).
  const EDGE_RINGS = [108, 170, 232];
  const OPTION_GAP = 8; // min space between options
  // Angle gaps between options (degrees). The capsule is wide, so the
  // options next to it get a bigger gap.
  const RING_STEPS = [62, 48, 48, 48, 48, 48]; // full ring, from the capsule at the top
  const STORE_KEY = "tunelet.bob-center";

  const win = getCurrentWindow();
  const IS_MAC = navigator.userAgent.includes("Macintosh");

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
    const ids: RadialId[] = ["playback", "like", "search", "playlists", "player", "lyrics", "settings"];
    const need = R_CAPSULE + 70;
    const roomy =
      center.x - a.x >= need &&
      a.x + a.width - center.x >= need &&
      center.y - a.y >= need &&
      a.y + a.height - center.y >= need;
    if (!roomy) return spotsNearEdge(a, ids);

    // Full ring: capsule at the top, others clockwise.
    let angle = -90;
    const angles = [angle, ...RING_STEPS.map((s) => (angle += s))];
    return ids.map((id, i) => {
      const R = id === "playback" ? R_CAPSULE : R_FULL;
      const t = (angles[i] * Math.PI) / 180;
      return { id, dx: Math.round(R * Math.cos(t)), dy: Math.round(R * Math.sin(t)) };
    });
  }

  /**
   * Near an edge or in a corner. The play capsule sits straight above the bob
   * (below if there's no room above), nudged sideways to stay on screen. The
   * other options fill rings around the bob in order, evenly spaced across the
   * part of each ring that's free (on screen, clear of the bob and capsule);
   * whatever doesn't fit goes on the next ring out. Nothing overlaps.
   */
  function spotsNearEdge(a: Area, ids: RadialId[]): RadialSpot[] {
    const R_BUBBLE = 22;
    const CAP = { w: 57, h: 22 };
    const deg = (d: number) => (d * Math.PI) / 180;
    const inside = (x: number, y: number, w: number, h: number) =>
      x - w >= a.x + 4 && x + w <= a.x + a.width - 4 && y - h >= a.y + 4 && y + h <= a.y + a.height - 4;
    const clear = (x: number, y: number, w: number, h: number, o: { x: number; y: number; w: number; h: number }) =>
      Math.abs(o.x - x) >= o.w + w + OPTION_GAP || Math.abs(o.y - y) >= o.h + h + OPTION_GAP;

    // Capsule: above, else below, nudged horizontally onto the screen.
    const capX = clamp(center.x, a.x + CAP.w + 4, a.x + a.width - CAP.w - 4);
    const above = center.y - R_CAPSULE;
    const capY = above - CAP.h >= a.y + 4 ? above : center.y + R_CAPSULE;
    const capsule = { x: capX, y: capY, w: CAP.w, h: CAP.h };
    const bob = { x: center.x, y: center.y, w: BOB_VISUAL_HALF, h: BOB_VISUAL_HALF };
    const spots: RadialSpot[] = [{ id: "playback", dx: Math.round(capX - center.x), dy: Math.round(capY - center.y) }];

    // Bubbles, ring by ring, in a half circle facing the middle of the screen.
    const toward = (Math.atan2(a.y + a.height / 2 - center.y, a.x + a.width / 2 - center.x) * 180) / Math.PI;
    let rest = ids.filter((id) => id !== "playback");
    for (const R of EDGE_RINGS) {
      if (!rest.length) break;
      const at = (d: number) => ({ x: center.x + R * Math.cos(deg(d)), y: center.y + R * Math.sin(deg(d)) });
      const ok = (d: number) => {
        const { x, y } = at(d);
        return inside(x, y, R_BUBBLE, R_BUBBLE) && clear(x, y, R_BUBBLE, R_BUBBLE, bob) && clear(x, y, R_BUBBLE, R_BUBBLE, capsule);
      };
      // Longest run of free angles (1° steps) on this ring.
      let best = { s: 0, e: -1 };
      let runStart: number | null = null;
      for (let d = toward - 120; d <= toward + 121; d++) {
        if (d <= toward + 120 && ok(d)) {
          runStart ??= d;
        } else if (runStart !== null) {
          if (d - 1 - runStart > best.e - best.s) best = { s: runStart, e: d - 1 };
          runStart = null;
        }
      }
      if (best.e < best.s) continue;
      // Spacing: at least a bubble and a gap apart; comfortably apart when there's room.
      const minStep = (2 * Math.asin((2 * R_BUBBLE + OPTION_GAP) / 2 / R) * 180) / Math.PI;
      const niceStep = (2 * Math.asin(31 / R) * 180) / Math.PI;
      const fit = Math.floor((best.e - best.s) / minStep) + 1;
      const n = Math.min(fit, rest.length);
      const step = n > 1 ? Math.min((best.e - best.s) / (n - 1), niceStep) : 0;
      const first = (best.s + best.e) / 2 - (step * (n - 1)) / 2;
      rest.slice(0, n).forEach((id, i) => {
        const { x, y } = at(first + step * i);
        spots.push({ id, dx: Math.round(x - center.x), dy: Math.round(y - center.y) });
      });
      rest = rest.slice(n);
    }
    // Out of rings (tiny screen): line the rest up toward the middle.
    rest.forEach((id, i) => {
      const R = EDGE_RINGS[EDGE_RINGS.length - 1] + 56 * (i + 1);
      spots.push({ id, dx: Math.round(R * Math.cos(deg(toward))), dy: Math.round(R * Math.sin(deg(toward))) });
    });
    // Keep the original option order (the radial animates in this order).
    return ids.map((id) => spots.find((sp) => sp.id === id)!);
  }

  const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), hi);

  /** Keeps the whole bob on screen. */
  function clampCenter(p: { x: number; y: number }, a: Area) {
    return {
      x: clamp(p.x, a.x + BOB_HALF, a.x + a.width - BOB_HALF),
      y: clamp(p.y, a.y + BOB_HALF, a.y + a.height - BOB_HALF),
    };
  }

  /** The window's current bounds (logical px), as last set by us. */
  let bounds: Rect = { x: 0, y: 0, w: 0, h: 0 };

  async function applyBounds(r: Rect) {
    bounds = r;
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

  /**
   * Moves the whole widget (bob plus any open menu or card) with the mouse.
   * When the drag ends, re-anchors on the bob's new spot and re-lays out the
   * menu/card so it opens toward the free side of the screen again.
   */
  let dragging = false;
  async function dragWindow(e: PointerEvent) {
    if (IS_MAC) return dragOnMac(e);
    if (dragging) return;
    // Only ever start a window drag while the button is really held: starting
    // one after it's been released leaves Windows in a drag with no button,
    // where the widget sticks to the cursor until the next click.
    if (!(await invoke<boolean>("mouse_down"))) return;
    dragging = true;
    try {
      await win.startDragging();
      // Windows runs the drag itself; wait for the button to come up.
      const until = Date.now() + 60_000;
      while (Date.now() < until && (await invoke<boolean>("mouse_down"))) await wait(60);
      const pos = (await win.outerPosition()).toLogical(await win.scaleFactor());
      // Dropped partly off screen? Snap the bob back fully onto it.
      const dropped = { x: pos.x + bobAt.x, y: pos.y + bobAt.y };
      center = clampCenter(dropped, await screenArea());
      saveCenter();
      if (mode === "radial") serial(layoutRadial);
      else if (mode === "card") serial(layoutCard);
      else if (center.x !== dropped.x || center.y !== dropped.y) serial(layoutBob);
    } finally {
      dragging = false;
    }
  }

  /**
   * macOS can't start a system window drag from here (it has to happen inside
   * the original mouse event), so move the window ourselves: follow the
   * pointer until the button is released.
   */
  function dragOnMac(e: PointerEvent) {
    if (dragging) return;
    dragging = true;
    const start = { x: e.screenX, y: e.screenY };
    const from = { x: bounds.x, y: bounds.y };
    let target = from;
    let sending = false;
    const send = async () => {
      if (sending) return;
      sending = true;
      try {
        while (bounds.x !== target.x || bounds.y !== target.y) {
          const t = target;
          bounds = { ...bounds, x: t.x, y: t.y };
          await invoke("set_bounds", { x: t.x, y: t.y, width: bounds.w, height: bounds.h });
        }
      } finally {
        sending = false;
      }
    };
    const move = (ev: PointerEvent) => {
      target = { x: Math.round(from.x + ev.screenX - start.x), y: Math.round(from.y + ev.screenY - start.y) };
      send();
    };
    const up = async () => {
      window.removeEventListener("pointermove", move, true);
      window.removeEventListener("pointerup", up, true);
      window.removeEventListener("pointercancel", up, true);
      while (sending) await wait(16);
      const dropped = { x: bounds.x + bobAt.x, y: bounds.y + bobAt.y };
      center = clampCenter(dropped, await screenArea());
      saveCenter();
      dragging = false;
      if (mode === "radial") serial(layoutRadial);
      else if (mode === "card") serial(layoutCard);
      else serial(layoutBob);
    };
    window.addEventListener("pointermove", move, true);
    window.addEventListener("pointerup", up, true);
    window.addEventListener("pointercancel", up, true);
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
    if (import.meta.env.DEV) {
      // Dev-only peek at the layout state, for debugging through the web view's devtools.
      (window as any).__tunelet = {
        state: () => ({ mode, card, dragging, radialShown, cardShown, center, bobAt, focused: document.hasFocus(), vis: document.visibilityState }),
        open: (c: Card) => openCard(c),
        moveTo: async (x: number, y: number) => {
          await closeAll();
          center = clampCenter({ x, y }, await screenArea());
          await serial(layoutBob);
          return center;
        },
        radial: () => openRadial(),
        app,
        queueIdle: () => Promise.race([queue.then(() => "idle"), wait(500).then(() => "busy")]),
      };
    }

    const offs = [
      listen<NowPlaying>("now-playing", (e) => (app.np = e.payload)),
      listen<Timeline>("timeline", (e) => {
        app.tl = { ...e.payload, at: performance.now() };
        app.clock = app.tl.at;
      }),
      listen<{ liked?: boolean; error?: string }>("like-result", (e) => app.likeResult(e.payload)),
      listen("open-search", () => openCard("search")),
      listen<boolean>("spotify-waking", (e) => (app.waking = e.payload)),
      listen<string>("toast", (e) => app.showToast(e.payload)),
      listen("open-settings", () => openCard("settings")),
      win.onMoved(async ({ payload }) => {
        // Only user drags in bob mode move the anchor; our own resizes don't.
        if (mode !== "bob" || dragging || Date.now() < ignoreMovesUntil) return;
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
    center = clampCenter(saved, a);
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
      <CardShell {card} shown={cardShown} origin={cardAt.origin} onswitch={(c) => openCard(c)} onclose={closeAll} ondrag={dragWindow}>
        {#if card === "search" || card === "playlists"}
          <SearchCard tab={card} onswitch={(t) => openCard(t)} />
        {:else if card === "lyrics"}
          <LyricsCard />
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
      <Bob bind:this={bobRef} open={mode !== "bob"} onmenu={toggleMenu} ondrag={dragWindow} />
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
