<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onDestroy, onMount } from "svelte";
  import { app, prettyCombo } from "../app.svelte";
  import type { Action } from "../types";

  const ACTIONS: Action[] = ["toggle", "search", "playPause", "next", "previous", "like"];
  const LABELS: Record<Action, string> = {
    toggle: "Show / hide",
    search: "Search",
    playPause: "Play / pause",
    next: "Next song",
    previous: "Previous song",
    like: "Like / unlike song",
  };
  const PROBLEM = {
    taken: "Used by another app. Pick another.",
    invalid: "That key isn't supported.",
    duplicate: "Already used for another action.",
  };

  let hint = $state("");

  onMount(() => app.loadSettings());
  onDestroy(() => {
    if (app.recording) stop();
  });

  async function start(a: Action) {
    app.recording = a;
    hint = "";
    await invoke("suspend_hotkeys", { suspend: true });
  }

  async function stop(combo?: string) {
    const a = app.recording;
    app.recording = null;
    hint = "";
    if (a && combo !== undefined) await app.setHotkey(a, combo); // re-registers all hotkeys
    else await invoke("suspend_hotkeys", { suspend: false });
  }

  function onkey(e: KeyboardEvent) {
    if (!app.recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return;
    if (e.key === "Escape") return void stop();
    if ((e.key === "Backspace" || e.key === "Delete") && !e.ctrlKey && !e.altKey) return void stop("");
    const mods: string[] = [];
    if (e.ctrlKey) mods.push("Ctrl");
    if (e.altKey) mods.push("Alt");
    if (e.shiftKey) mods.push("Shift");
    if (e.metaKey) mods.push("Super");
    if (mods.length === 0 || (mods.length === 1 && mods[0] === "Shift")) {
      hint = "Include Ctrl, Alt or Win so it doesn't clash with typing.";
      return;
    }
    stop([...mods, e.code].join("+"));
  }
</script>

<svelte:window onkeydowncapture={onkey} />

{#if app.sv}
  {@const sv = app.sv}
  <div class="settings scroll fade">
    <h3>Hotkeys</h3>
    {#each ACTIONS as a (a)}
      {@const problem = sv.hotkeyErrors.find((x) => x.action === a)}
      <div class="row">
        <span>{LABELS[a]}</span>
        <button
          class="key"
          class:recording={app.recording === a}
          class:bad={!!problem}
          onclick={() => (app.recording === a ? stop() : start(a))}
        >
          {app.recording === a ? "Press keys…" : prettyCombo(sv.settings.hotkeys[a])}
        </button>
      </div>
      {#if problem && app.recording !== a}<div class="row-note error">{PROBLEM[problem.reason]}</div>{/if}
      {#if app.recording === a && hint}<div class="row-note">{hint}</div>{/if}
    {/each}
    <div class="row-note after-keys">
      Click a shortcut, then press the new keys. Backspace clears, Esc cancels.
      <button class="link small" onclick={() => app.resetHotkeys()}>Reset to defaults</button>
    </div>

    <h3>Mouse</h3>
    <div class="mouse">
      <span>Click the bubble</span><span>Open the menu</span>
      <span>Right-click</span><span>Play / pause</span>
      <span>Drag</span><span>Move it</span>
    </div>

    <h3>Appearance</h3>
    <div class="row">
      <span>Opacity</span>
      <span class="slider">
        <input
          class="range"
          style="--v: {((app.opacity * 100 - 40) / 60) * 100}%"
          type="range"
          min="40"
          max="100"
          step="5"
          value={Math.round(app.opacity * 100)}
          oninput={(e) => (sv.settings.opacity = +e.currentTarget.value / 100)}
          onchange={() => app.saveSettings()}
        />
        <span class="pct">{Math.round(app.opacity * 100)}%</span>
      </span>
    </div>

    <h3>General</h3>
    <label class="row">
      <span>Start with Windows</span>
      <input type="checkbox" class="switch" checked={sv.autostart} onchange={(e) => app.toggleAutostart(e.currentTarget.checked)} />
    </label>
    <label class="row" title="When Spotify isn't running, pressing play starts it minimized to the tray, so you never have to open it.">
      <span>Start Spotify when needed</span>
      <input
        type="checkbox"
        class="switch"
        checked={sv.settings.launchSpotify}
        onchange={(e) => {
          sv.settings.launchSpotify = e.currentTarget.checked;
          app.saveSettings();
        }}
      />
    </label>
    <div class="row">
      <span>Spotify</span>
      {#if app.auth?.loggedIn}
        <button class="link small" onclick={() => app.logout()}>Log out</button>
      {:else}
        <span class="muted small-text">Not connected</span>
      {/if}
    </div>

    <h3>About</h3>
    <div class="row">
      <span>Tunelet v{sv.version}</span>
      {#if app.update.state === "available"}
        <button class="chip accent" onclick={() => app.installUpdate()}>Update to v{app.update.version}</button>
      {:else if app.update.state === "installing"}
        <span class="muted small-text">Installing…</span>
      {:else if app.update.state === "checking"}
        <span class="muted small-text">Checking…</span>
      {:else if app.update.state === "none"}
        <span class="muted small-text">Up to date</span>
      {:else}
        <button class="chip" onclick={() => app.checkUpdate()}>Check for updates</button>
      {/if}
    </div>
    {#if app.update.state === "error"}<div class="row-note error">{app.update.msg}</div>{/if}
    {#if app.error}<div class="row-note error">{app.error}</div>{/if}
    <button class="link small gh" onclick={() => openUrl("https://github.com/ShadowMonarch9099/tunelet")}>Tunelet on GitHub</button>
  </div>
{:else}
  <div class="center muted">Loading…</div>
{/if}

<style>
  .settings { padding: 0 16px 14px; font-size: 12.5px; }
  h3 { margin: 12px 0 4px; font-size: 10.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; opacity: 0.6; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 10px; min-height: 32px; }
  .row-note { font-size: 11px; opacity: 0.75; margin: -2px 0 4px; line-height: 1.4; }
  .row-note.after-keys { margin-top: 6px; }
  .mouse { display: grid; grid-template-columns: auto 1fr; gap: 6px 12px; font-size: 12px; }
  .mouse span:nth-child(odd) { opacity: 0.7; }
  .mouse span:nth-child(even) { text-align: right; }
  .row-note.error { opacity: 1; margin-top: 0; }
  .key {
    min-width: 96px;
    height: 26px;
    padding: 0 10px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: rgba(0, 0, 0, 0.28);
    color: #fff;
    font: inherit;
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.02em;
    cursor: pointer;
    transition: background 150ms, border-color 150ms;
  }
  .key:hover { background: rgba(255, 255, 255, 0.12); }
  .key.recording { border-color: #fff; background: rgba(255, 255, 255, 0.18); animation: pulse 1s ease infinite; }
  .key.bad { border-color: #ff9a9a; }
  @keyframes pulse { 50% { border-color: rgba(255, 255, 255, 0.4); } }
  .slider { display: flex; align-items: center; gap: 8px; }
  .slider .range { width: 120px; }
  .small-text { font-size: 11px; }
  .gh { margin-top: 10px; }
</style>
