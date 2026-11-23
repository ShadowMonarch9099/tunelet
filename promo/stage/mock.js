// A stand-in for Tunelet's Rust backend, injected into the real Tunelet UI
// (served by Vite) so it runs in a plain browser for filming the promo.
// Implements the Tauri internals the UI uses (invoke, events, window info)
// with fictional songs, generated covers and synced lyrics.
(() => {
  if (location.port !== "1420") return; // only inside the Tunelet frame

  // ---------- sample music ----------
  const cover = (a, b, shape) =>
    "data:image/svg+xml;utf8," +
    encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 300">
      <defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${a}"/><stop offset="1" stop-color="${b}"/></linearGradient></defs>
      <rect width="300" height="300" fill="url(#g)"/>${shape}</svg>`);
  const SONGS = [
    { title: "Neon Rain", artist: "Kairo", album: "City Lights", colors: ["#7C3AED", "#0EA5E9"], durationMs: 214000,
      art: cover("#7C3AED", "#0EA5E9", `<circle cx="150" cy="150" r="80" fill="none" stroke="#fff" stroke-opacity=".7" stroke-width="14"/><circle cx="150" cy="150" r="30" fill="#fff" fill-opacity=".85"/>`) },
    { title: "Golden Hour", artist: "Mira Sol", album: "Sunday Tapes", colors: ["#F59E0B", "#EC4899"], durationMs: 198000,
      art: cover("#F59E0B", "#EC4899", `<circle cx="150" cy="190" r="70" fill="#FDE68A"/><rect y="190" width="300" height="110" fill="#BE185D" fill-opacity=".55"/>`) },
    { title: "Paper Skies", artist: "The Velvet Owls", album: "Paper Skies", colors: ["#10B981", "#1E3A8A"], durationMs: 231000,
      art: cover("#10B981", "#1E3A8A", `<path d="M40 210 L150 70 L260 210 Z" fill="#fff" fill-opacity=".8"/><path d="M90 230 L150 150 L210 230 Z" fill="#064E3B"/>`) },
    { title: "Static Hearts", artist: "Lumen", album: "Afterglow", colors: ["#EF4444", "#7C2D12"], durationMs: 187000,
      art: cover("#F43F5E", "#7C2D12", `<path d="M150 230 C60 170 70 90 120 90 C140 90 150 105 150 115 C150 105 160 90 180 90 C230 90 240 170 150 230 Z" fill="#fff" fill-opacity=".85"/>`) },
    { title: "Lo-fi Sunday", artist: "Nova Lane", album: "Slow Mornings", colors: ["#22D3EE", "#A855F7"], durationMs: 205000,
      art: cover("#22D3EE", "#A855F7", `<rect x="70" y="120" width="160" height="90" rx="20" fill="#fff" fill-opacity=".8"/><circle cx="115" cy="165" r="18" fill="#6D28D9"/><circle cx="185" cy="165" r="18" fill="#6D28D9"/>`) },
  ].map((s, i) => ({ ...s, uri: "spotify:track:demo" + i }));
  const LYRICS = [
    [0, "♪"], [3500, "City lights are calling out my name"], [7600, "Neon rain is falling down again"], [11800, "Headphones on, the world can fade away"],
    [16000, "I'm in the zone, I'm gonna stay"], [20200, "One more song, one more line"], [24400, "Everything is falling into time"],
    [28600, "Turn it up, don't look back"], [32800, "Neon rain on a midnight track"], [37000, "City lights are calling out my name"],
    [41200, "Neon rain is falling down again"], [45400, "♪"],
  ].map(([ms, text]) => ({ ms, text }));

  let cur = 0, playing = false, posMs = 52000, lastTick = performance.now(), liked = new Set([0]);
  const pos = () => Math.min(SONGS[cur].durationMs, posMs + (playing ? performance.now() - lastTick : 0));
  const setPos = (ms) => { posMs = ms; lastTick = performance.now(); };

  // ---------- Tauri internals ----------
  const callbacks = new Map();
  const listeners = {}; // event -> [callbackId]
  let nextId = 1;
  const emit = (event, payload) => (listeners[event] || []).forEach((id) => callbacks.get(id)?.({ event, id, payload }));

  const nowPlaying = () => {
    const s = SONGS[cur];
    return { available: true, title: s.title, artist: s.artist, album: s.album, playing, art: s.art, colors: s.colors };
  };
  const pushState = () => {
    emit("now-playing", nowPlaying());
    emit("timeline", { positionMs: pos(), durationMs: SONGS[cur].durationMs, playing });
  };
  setInterval(() => emit("timeline", { positionMs: pos(), durationMs: SONGS[cur].durationMs, playing }), 500);

  const parentMsg = (m) => window.parent.postMessage({ tunelet: true, ...m }, "*");
  let bounds = { x: 0, y: 0, w: 96, h: 96 };
  let interactive = false;
  const item = (s) => ({ kind: "track", uri: s.uri, name: s.title, subtitle: s.artist, image: s.art, owned: false });
  const settings = {
    hotkeys: { toggle: "Alt+KeyZ", search: "Alt+KeyX", playPause: "Alt+KeyS", next: "Alt+KeyD", previous: "Alt+KeyA", like: "Alt+KeyC" },
    opacity: 1, launchSpotify: true, startHidden: true,
  };
  const view = () => ({ settings, autostart: true, version: "0.7.2", hotkeyErrors: [] });
  const play = (i) => { cur = i; setPos(0); playing = true; pushState(); };

  const commands = {
    "plugin:event|listen": ({ event, handler }) => { (listeners[event] ||= []).push(handler); return handler; },
    "plugin:event|unlisten": () => null,
    "plugin:window|outer_position": () => ({ x: bounds.x, y: bounds.y }),
    "plugin:window|scale_factor": () => 1,
    "plugin:window|start_dragging": () => null,
    "plugin:opener|open_url": () => null,
    screen_area: () => ({ x: 0, y: 0, width: 1920, height: 1080 }),
    set_bounds: ({ x, y, width, height }) => { bounds = { x, y, w: width, h: height }; parentMsg({ type: "bounds", ...bounds }); },
    set_interactive: ({ interactive: v }) => { interactive = v; parentMsg({ type: "interactive", value: v }); },
    hide_widget: () => parentMsg({ type: "hide" }),
    mouse_down: () => false,
    media: ({ action }) => {
      if (action === "toggle") { setPos(pos()); playing = !playing; }
      if (action === "next") play((cur + 1) % SONGS.length);
      if (action === "previous") play((cur + SONGS.length - 1) % SONGS.length);
      pushState();
    },
    media_seek: ({ positionMs }) => { setPos(positionMs); pushState(); },
    get_settings: view, save_settings: ({ settings: s }) => (Object.assign(settings, s), view()), default_settings: () => settings,
    suspend_hotkeys: () => null, set_autostart: ({ enabled }) => enabled, check_update: () => null, install_update: () => null,
    spotify_status: () => ({ clientId: "demo", loggedIn: true, redirectUri: "http://127.0.0.1:43821/callback" }),
    spotify_player: () => ({ active: true, device: { id: "pc", name: "My Laptop", kind: "Computer", active: true, volume: 72, supportsVolume: true },
      trackUri: SONGS[cur].uri, liked: liked.has(cur), isPlaying: playing, shuffle: false, smartShuffle: false, contextUri: null }),
    spotify_devices: () => [{ id: "pc", name: "My Laptop", kind: "Computer", active: true, volume: 72, supportsVolume: true },
      { id: "ph", name: "Phone", kind: "Smartphone", active: false, volume: 50, supportsVolume: true }],
    spotify_search: ({ query }) => {
      const q = query.toLowerCase();
      const hits = SONGS.filter((s) => (s.title + " " + s.artist).toLowerCase().includes(q));
      return [...hits, ...SONGS.filter((s) => !hits.includes(s))].slice(0, 6).map(item);
    },
    spotify_play: ({ uri }) => { const i = SONGS.findIndex((s) => s.uri === uri); play(i < 0 ? 0 : i); },
    spotify_queue: () => null, spotify_seek: ({ positionMs }) => setPos(positionMs), spotify_volume: () => null, spotify_shuffle: () => null,
    spotify_set_liked: () => null, spotify_toggle_like: () => (liked.has(cur) ? liked.delete(cur) : liked.add(cur), liked.has(cur)),
    spotify_transfer: () => null,
    spotify_playlists: () => [
      { kind: "liked", uri: "spotify:user:demo:collection", name: "Liked Songs", subtitle: "Your liked songs", image: null, owned: true },
      ...SONGS.slice(0, 4).map((s, i) => ({ kind: "playlist", uri: "spotify:playlist:p" + i, name: ["Deep Focus", "Late Night Drive", "Sunday Chill", "Workout Mix"][i], subtitle: `${20 + i * 7} songs · You`, image: s.art, owned: true })),
    ],
    spotify_playlist_tracks: () => ({ tracks: SONGS.map(item), total: SONGS.length, next: null }),
    lyrics: () => ({ synced: cur === 0 ? LYRICS : LYRICS.map((l) => ({ ...l })), plain: null, instrumental: false }),
  };

  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
    transformCallback(cb) { const id = nextId++; callbacks.set(id, cb); return id; },
    unregisterCallback(id) { callbacks.delete(id); },
    convertFileSrc: (p) => p,
    async invoke(cmd, args = {}) {
      const f = commands[cmd];
      if (!f) { console.warn("mock: unhandled", cmd); return null; }
      return f(args);
    },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };

  // In bubble mode the real window never takes focus; mirror that by handing
  // focus straight back to the stage (the "app you were working in").
  window.addEventListener("contextmenu", () => { if (!interactive) parentMsg({ type: "refocus" }); }, true);

  // Controls for the filming script.
  window.__mock = {
    emit, play, pushState,
    hotkey(name) {
      if (name === "search") emit("open-search");
      if (name === "next") commands.media({ action: "next" });
      if (name === "playPause") commands.media({ action: "toggle" });
    },
    seek: (ms) => { setPos(ms); pushState(); },
    state: () => ({ cur, playing, pos: pos(), bounds, interactive }),
  };
  try { localStorage.setItem("tunelet.bob-center", JSON.stringify({ x: 1215, y: 620 })); } catch {}
})();
