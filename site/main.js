/* Tunelet download page */
(() => {
  "use strict";

  const REPO = "ShadowMonarch9099/tunelet";
  const RELEASES_API = `https://api.github.com/repos/${REPO}/releases?per_page=30`;
  const CACHE_KEY = "tunelet-releases-v1";
  const CACHE_MS = 10 * 60 * 1000;

  // Used when the GitHub API can't be reached (offline, rate-limited).
  // Files follow the release naming, so only the newest known tag is needed.
  const FALLBACK = [{ tag: "v0.7.2", date: "2026-11-23", sizes: { x64: 2447720, arm64: 2200042, mac: 6654487 } }];

  const NOTE_COLOURS = ["#ffc93c", "#ffc93c", "#fff4e0", "#ff6a3d"];
  const NOTE_SHAPES = ["n-eighth", "n-beam", "n-quarter", "n-half", "n-eighth"];

  const $ = (s, el = document) => el.querySelector(s);
  const $$ = (s, el = document) => [...el.querySelectorAll(s)];
  const rand = (a, b) => a + Math.random() * (b - a);
  const pick = (arr) => arr[(Math.random() * arr.length) | 0];

  const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

  const state = { releases: [], current: null, arch: "x64", os: "other" };

  const ICONS = {
    windows: '<svg viewBox="0 0 24 24"><path fill="currentColor" d="M3 5.1 10.4 4v7.1H3zm8.3-1.2L21 2.5v8.6h-9.7zM3 12.9h7.4V20L3 18.9zm8.3 0H21v8.6l-9.7-1.4z"/></svg>',
    mac: '<svg viewBox="0 0 24 24"><path fill="currentColor" d="M16.4 12.6c0-2.6 2.1-3.8 2.2-3.9-1.2-1.8-3.1-2-3.7-2-1.6-.2-3.1.9-3.9.9-.8 0-2-.9-3.4-.9-1.7 0-3.3 1-4.2 2.6-1.8 3.1-.5 7.7 1.3 10.2.9 1.2 1.9 2.6 3.2 2.6 1.3-.1 1.8-.8 3.3-.8s2 .8 3.4.8c1.4 0 2.3-1.3 3.1-2.5 1-1.4 1.4-2.8 1.4-2.9 0 0-2.7-1-2.7-4.1zM13.9 5c.7-.9 1.2-2 1-3.2-1 0-2.3.7-3 1.6-.7.8-1.2 2-1.1 3.1 1.2.1 2.3-.6 3.1-1.5z"/></svg>',
    down: '<svg viewBox="0 0 24 24"><path d="M12 4v13m0 0-5-5m5 5 5-5M5 20.5h14" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/></svg>',
  };

  /* ------------------------------------------------------------------ */
  /* Platform detection                                                  */
  /* ------------------------------------------------------------------ */
  async function detectPlatform() {
    const ua = navigator.userAgent;
    const uad = navigator.userAgentData;
    const plat = ((uad && uad.platform) || navigator.platform || "").toLowerCase();
    let os = "other";
    let arch = "x64";

    if (/iphone|ipad|ipod|android/i.test(ua)) os = "mobile";
    else if (plat.includes("win") || /windows/i.test(ua)) os = "windows";
    else if (plat.includes("mac") || /macintosh|mac os x/i.test(ua)) {
      os = navigator.maxTouchPoints > 1 ? "mobile" : "mac"; // iPadOS reports as Mac
    }

    if (os === "windows") {
      if (uad && uad.getHighEntropyValues) {
        try {
          const h = await uad.getHighEntropyValues(["architecture"]);
          if (h.architecture === "arm") arch = "arm64";
        } catch (_) { /* keep x64 */ }
      } else if (/arm64|aarch64/i.test(ua)) {
        arch = "arm64";
      }
    }
    return { os, arch };
  }

  /* ------------------------------------------------------------------ */
  /* Releases                                                            */
  /* ------------------------------------------------------------------ */
  function normalise(r) {
    const find = (re) => {
      const a = (r.assets || []).find((x) => re.test(x.name));
      return a ? { name: a.name, url: a.browser_download_url, size: a.size } : null;
    };
    return {
      tag: r.tag_name,
      date: r.published_at,
      prerelease: r.prerelease,
      files: {
        x64: find(/_x64-setup\.exe$/i) || find(/_x64.*\.msi$/i),
        arm64: find(/_arm64-setup\.exe$/i),
        mac: find(/universal\.dmg$/i) || find(/\.dmg$/i),
      },
    };
  }

  function fallbackReleases() {
    return FALLBACK.map(({ tag, date, sizes }) => {
      const v = tag.replace(/^v/, "");
      const file = (name, size) => ({ name, size, url: `https://github.com/${REPO}/releases/download/${tag}/${name}` });
      return {
        tag, date, prerelease: false,
        files: {
          x64: file(`Tunelet_${v}_x64-setup.exe`, sizes.x64),
          arm64: file(`Tunelet_${v}_arm64-setup.exe`, sizes.arm64),
          mac: file(`Tunelet_${v}_universal.dmg`, sizes.mac),
        },
      };
    });
  }

  async function loadReleases() {
    try {
      const cached = JSON.parse(sessionStorage.getItem(CACHE_KEY) || "null");
      if (cached && Date.now() - cached.t < CACHE_MS && cached.data.length) return cached.data;
    } catch (_) { /* ignore */ }

    try {
      const ctrl = new AbortController();
      const timer = setTimeout(() => ctrl.abort(), 6000);
      const res = await fetch(RELEASES_API, { signal: ctrl.signal, headers: { Accept: "application/vnd.github+json" } });
      clearTimeout(timer);
      if (!res.ok) throw new Error(res.status);
      const data = (await res.json())
        .filter((r) => !r.draft)
        .map(normalise)
        .filter((r) => r.files.x64 || r.files.arm64 || r.files.mac);
      if (!data.length) throw new Error("no releases");
      try { sessionStorage.setItem(CACHE_KEY, JSON.stringify({ t: Date.now(), data })); } catch (_) { /* ignore */ }
      return data;
    } catch (_) {
      return fallbackReleases();
    }
  }

  const fmtSize = (b) => (b ? `${(b / 1048576).toFixed(1)} MB` : "");
  const fmtDate = (d) => {
    try { return new Date(d).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" }); }
    catch (_) { return ""; }
  };

  /* ------------------------------------------------------------------ */
  /* Rendering                                                           */
  /* ------------------------------------------------------------------ */
  function fillVersions() {
    const sel = $("#versionSelect");
    sel.innerHTML = "";
    state.releases.forEach((r, i) => {
      const o = document.createElement("option");
      o.value = r.tag;
      o.textContent = `${r.tag} · ${r.prerelease ? "pre-release" : i === 0 ? "latest" : fmtDate(r.date)}`;
      sel.appendChild(o);
    });
    sel.value = state.current.tag;
  }

  function winFile() {
    const f = state.current.files;
    return f[state.arch] || f.x64 || f.arm64;
  }

  function setDownload(btn, file) {
    if (file) {
      btn.href = file.url;
      btn.classList.remove("disabled");
      btn.removeAttribute("aria-disabled");
      btn.dataset.name = file.name;
    } else {
      btn.href = `https://github.com/${REPO}/releases/tag/${state.current.tag}`;
      btn.classList.add("disabled");
      btn.setAttribute("aria-disabled", "true");
      delete btn.dataset.name;
    }
  }

  function render() {
    const r = state.current;
    const f = r.files;

    $$(".seg button").forEach((b) => {
      const has = !!f[b.dataset.arch];
      b.disabled = !has;
      b.title = has ? "" : `Not available in ${r.tag}`;
    });
    if (!f[state.arch]) state.arch = f.x64 ? "x64" : "arm64";
    updateSeg(false);

    const wf = winFile();
    setDownload($('[data-dl="windows"]'), wf);
    $('[data-file="windows"]').textContent = wf ? `${wf.name} · ${fmtSize(wf.size)}` : `Not available in ${r.tag}`;

    setDownload($('[data-dl="mac"]'), f.mac);
    $('[data-file="mac"]').textContent = f.mac ? `${f.mac.name} · ${fmtSize(f.mac.size)}` : `Not available in ${r.tag}`;

    renderHero();
  }

  function renderHero() {
    const btn = $("#heroDownload");
    const r = state.current;
    let file = null;

    if (state.os === "windows") {
      file = winFile();
      $("#heroLabel").textContent = "Download for Windows";
      $("#heroIcon").innerHTML = ICONS.windows;
      const archName = file && /arm64/i.test(file.name) ? "ARM" : "Intel / AMD";
      $("#heroMeta").innerHTML = `${r.tag} · ${archName} · ${fmtSize(file && file.size)} · <a href="#download">Other downloads</a>`;
    } else if (state.os === "mac") {
      file = r.files.mac;
      $("#heroLabel").textContent = "Download for Mac";
      $("#heroIcon").innerHTML = ICONS.mac;
      $("#heroMeta").innerHTML = `${r.tag} · Apple Silicon &amp; Intel · ${fmtSize(file && file.size)} · <a href="#download">Other downloads</a>`;
    } else {
      $("#heroLabel").textContent = "Get Tunelet";
      $("#heroIcon").innerHTML = ICONS.down;
      $("#heroMeta").innerHTML = state.os === "mobile"
        ? "Tunelet is a desktop app. Open this page on your Windows PC or Mac."
        : 'Free for Windows &amp; Mac · <a href="#download">All downloads</a>';
    }

    if (file) {
      btn.href = file.url;
      btn.dataset.name = file.name;
    } else {
      btn.href = "#download";
      delete btn.dataset.name;
    }
  }

  function updateSeg(animate = true) {
    const idx = state.arch === "arm64" ? 1 : 0;
    $$(".seg button").forEach((b, i) => b.setAttribute("aria-checked", String(i === idx)));
    const pill = $(".seg-pill");
    if (window.gsap && animate && !reduceMotion) gsap.to(pill, { xPercent: idx * 100, duration: 0.5, ease: "back.out(1.4)" });
    else pill.style.transform = `translateX(${idx * 100}%)`;
  }

  function markDetected() {
    if (state.os !== "windows" && state.os !== "mac") return;
    const card = $(`#card-${state.os}`);
    card.classList.add("is-detected");
    $(".sticker", card).hidden = false;
  }

  /* ------------------------------------------------------------------ */
  /* Interactions                                                        */
  /* ------------------------------------------------------------------ */
  function bindUI() {
    $("#versionSelect").addEventListener("change", (e) => {
      state.current = state.releases.find((r) => r.tag === e.target.value) || state.releases[0];
      const url = new URL(location.href);
      if (state.current === state.releases[0]) url.searchParams.delete("version");
      else url.searchParams.set("version", state.current.tag);
      history.replaceState(null, "", url);
      render();
      if (window.gsap && !reduceMotion) {
        gsap.fromTo(".file", { autoAlpha: 0, y: 6 }, { autoAlpha: 0.7, y: 0, duration: 0.4, stagger: 0.06, ease: "power2.out", clearProps: "opacity,visibility,transform" });
      }
    });

    $$(".seg button").forEach((b) =>
      b.addEventListener("click", () => {
        if (b.disabled || state.arch === b.dataset.arch) return;
        state.arch = b.dataset.arch;
        updateSeg();
        render();
      })
    );
    $(".seg").addEventListener("keydown", (e) => {
      if (!["ArrowLeft", "ArrowRight"].includes(e.key)) return;
      const next = $$(".seg button").find((b) => !b.disabled && b.dataset.arch !== state.arch);
      if (next) { next.click(); next.focus(); }
    });

    // Download clicks: the browser follows the link (GitHub serves it as a
    // download, so the page stays put); we just celebrate.
    document.addEventListener("click", (e) => {
      const a = e.target.closest("a.btn");
      if (a && a.dataset.name) onDownload(a);
    });

    // In-page links (#watch, #download, back to top) glide with GSAP; plain jump without it
    document.addEventListener("click", (e) => {
      const a = e.target.closest('a[href^="#"]');
      if (!a) return;
      const target = a.getAttribute("href") === "#top" ? 0 : $(a.getAttribute("href"));
      if (target === null) return;
      e.preventDefault();
      if (window.gsap && window.ScrollToPlugin && !reduceMotion) {
        gsap.to(window, { scrollTo: { y: target, autoKill: true }, duration: 1.1, ease: "power3.inOut" });
      } else {
        target === 0 ? scrollTo(0, 0) : target.scrollIntoView();
      }
      history.replaceState(null, "", a.getAttribute("href") === "#top" ? location.pathname + location.search : a.getAttribute("href"));
    });

    const video = $("#promo");
    const playBtn = $("#playBtn");
    playBtn.addEventListener("click", () => {
      playBtn.classList.add("hidden");
      video.controls = true;
      video.play().catch(() => {});
    });
    video.addEventListener("play", () => { playBtn.classList.add("hidden"); video.controls = true; });
    video.addEventListener("ended", () => playBtn.classList.remove("hidden"));
  }

  let toastTimer;
  function onDownload(btn) {
    const toast = $("#toast");
    toast.innerHTML = `<span class="eqmini"><i></i><i></i><i></i><i></i></span> Downloading <b>${btn.dataset.name}</b>`;
    clearTimeout(toastTimer);

    const label = $(".btn-label", btn);
    const original = label.dataset.original || label.textContent;
    label.dataset.original = original;
    label.textContent = "Download started";
    setTimeout(() => (label.textContent = original), 2600);

    if (!window.gsap || reduceMotion) {
      toast.style.opacity = 1;
      toastTimer = setTimeout(() => (toast.style.opacity = 0), 3200);
      return;
    }

    burstFrom(btn.getBoundingClientRect(), 12, document.body, 40);
    gsap.killTweensOf(toast);
    gsap.fromTo(toast, { autoAlpha: 0, y: 20 }, { autoAlpha: 1, y: 0, duration: 0.45, ease: "back.out(1.6)" });
    gsap.fromTo($$(".eqmini i", toast), { scaleY: 0.25 }, {
      scaleY: 1, duration: 0.35, ease: "sine.inOut", repeat: 9, yoyo: true,
      stagger: { each: 0.09, from: "random" },
    });
    toastTimer = setTimeout(() => gsap.to(toast, { autoAlpha: 0, y: 20, duration: 0.4, ease: "power2.in" }), 3400);
  }

  /* ------------------------------------------------------------------ */
  /* Notes                                                               */
  /* ------------------------------------------------------------------ */
  function makeNote(size, colour) {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("class", "note");
    svg.setAttribute("aria-hidden", "true");
    svg.style.width = svg.style.height = `${size}px`;
    svg.style.color = colour || pick(NOTE_COLOURS);
    const use = document.createElementNS("http://www.w3.org/2000/svg", "use");
    use.setAttribute("href", `#${pick(NOTE_SHAPES)}`);
    svg.appendChild(use);
    return svg;
  }

  // One note leaves a point and drifts up, swaying, until it fades out.
  function floatNote(layer, x0, y0, opts = {}) {
    const size = opts.size || rand(18, 44);
    const n = makeNote(size, opts.colour);
    if (opts.fixed) Object.assign(n.style, { position: "fixed", zIndex: opts.z || 40, pointerEvents: "none" });
    layer.appendChild(n);

    const dur = opts.dur || rand(4.5, 7.5);
    const rise = opts.rise || rand(innerHeight * 0.55, innerHeight * 0.95);
    const drift = opts.drift != null ? opts.drift : rand(-200, 80);
    gsap.set(n, { x: x0 - size / 2, y: y0 - size / 2, scale: 0.2, rotation: rand(-30, 30), opacity: 0 });

    const tl = gsap.timeline({ onComplete: () => n.remove() });
    tl.to(n, { opacity: rand(0.75, 1), duration: 0.35, ease: "power1.out" }, 0)
      .to(n, { scale: 1, duration: 0.9, ease: "back.out(2.2)" }, 0)
      .to(n, { y: `-=${rise}`, duration: dur, ease: "power1.out" }, 0)
      .to(n, { x: `+=${drift}`, duration: dur, ease: "sine.inOut" }, 0)
      // a little side-to-side wobble, like it's bobbing on the music
      .to(n, { rotation: `+=${rand(-50, 50)}`, duration: dur / 3, ease: "sine.inOut", repeat: 2, yoyo: true }, 0)
      .to(n, { opacity: 0, duration: dur * 0.35, ease: "power1.in" }, dur * 0.65);
    return tl;
  }

  function burstFrom(rect, count, layer, z) {
    const cx = rect.left + rect.width / 2;
    const cy = rect.top + rect.height / 2;
    for (let i = 0; i < count; i++) {
      floatNote(layer, cx + rand(-rect.width / 3, rect.width / 3), cy, {
        fixed: true, z, size: rand(16, 30), dur: rand(1.4, 2.2), rise: rand(90, 220), drift: rand(-120, 120),
      });
    }
  }

  /* ------------------------------------------------------------------ */
  /* Record player: its spin drives the notes                            */
  /* ------------------------------------------------------------------ */
  function startPlayer() {
    const layer = $("#notes");
    const player = $("#player");
    const record = $("#record");
    const ARM_REST = 0;   // parked, off the record
    const ARM_PLAY = 24;  // needle in the groove
    const ARM_HOVER = 30;

    gsap.set("#tonearm", { rotation: ARM_REST });
    const spin = gsap.to(record, { rotation: 360, duration: 2.4, ease: "none", repeat: -1, paused: true });

    // Emit from a random spot on the record; the faster it spins, the more notes.
    const emit = () => {
      const r = record.getBoundingClientRect();
      if (r.bottom < 0 || r.top > innerHeight) return;
      const a = rand(-Math.PI, 0); // top half of the disc
      const d = rand(0.2, 0.46) * r.width;
      floatNote(layer, r.left + r.width / 2 + Math.cos(a) * d, r.top + r.height / 2 + Math.sin(a) * d);
    };

    let inView = true;
    let acc = 0;
    const GAP = 430; // ms between notes at normal speed
    gsap.ticker.add((_t, dt) => {
      if (!inView || spin.paused()) return;
      acc += Math.min(dt, 100) * Math.abs(spin.timeScale());
      while (acc > GAP) { acc -= GAP; emit(); }
    });

    const speed = (v) => gsap.to(spin, { timeScale: v, duration: 0.8, ease: "power2.out", overwrite: true });
    const hoverable = matchMedia("(hover: hover)").matches;
    if (hoverable) {
      player.addEventListener("pointerenter", () => { speed(2.6); gsap.to("#tonearm", { rotation: ARM_HOVER, duration: 0.6, ease: "power2.out" }); });
      player.addEventListener("pointerleave", () => { speed(1); gsap.to("#tonearm", { rotation: ARM_PLAY, duration: 0.8, ease: "power2.out" }); });
    }

    // Scratch: a quick back-and-forth and a handful of notes at once
    const scratch = () => {
      gsap.timeline()
        .to(spin, { timeScale: -4, duration: 0.14, ease: "power1.out" })
        .to(spin, { timeScale: 4, duration: 0.18, ease: "power1.inOut" })
        .to(spin, { timeScale: hoverable && player.matches(":hover") ? 2.6 : 1, duration: 0.7, ease: "power2.out" });
      const r = record.getBoundingClientRect();
      for (let i = 0; i < 9; i++) {
        floatNote(layer, r.left + r.width / 2 + rand(-r.width / 3, r.width / 3), r.top + r.height / 2 + rand(-r.height / 4, 0), {
          dur: rand(2.5, 4), rise: rand(200, 420), drift: rand(-180, 180),
        });
      }
    };
    player.addEventListener("click", scratch);
    player.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") { e.preventDefault(); scratch(); }
    });

    // Only spin and emit while the record player is actually on screen
    let started = false;
    if ("IntersectionObserver" in window) {
      new IntersectionObserver(([entry]) => {
        inView = entry.isIntersecting;
        if (!started) return;
        inView ? spin.resume() : spin.pause();
      }).observe(player);
    }

    // Needle drop, then the record winds up to speed
    return gsap.timeline()
      .to("#tonearm", { rotation: ARM_PLAY, duration: 1.1, ease: "power3.inOut" })
      .add(() => { started = true; spin.play(); gsap.fromTo(spin, { timeScale: 0.05 }, { timeScale: 1, duration: 1.4, ease: "power2.in" }); });
  }

  /* ------------------------------------------------------------------ */
  /* Ticker                                                              */
  /* ------------------------------------------------------------------ */
  function startTicker() {
    const track = $("#ticker");
    const row = $(".ticker-row", track);
    const copy = row.cloneNode(true);
    copy.setAttribute("aria-hidden", "true");
    track.appendChild(copy);
    if (reduceMotion) return;
    const loop = gsap.to(track, { xPercent: -50, duration: 32, ease: "none", repeat: -1 });
    const t = $(".ticker");
    t.addEventListener("pointerenter", () => gsap.to(loop, { timeScale: 0.25, duration: 0.6 }));
    t.addEventListener("pointerleave", () => gsap.to(loop, { timeScale: 1, duration: 0.6 }));
  }

  /* ------------------------------------------------------------------ */
  /* Page animation                                                      */
  /* ------------------------------------------------------------------ */
  function animate() {
    const html = document.documentElement;
    if (!window.gsap) { html.classList.remove("js"); return; }
    if (window.ScrollTrigger) gsap.registerPlugin(ScrollTrigger);
    if (window.ScrollToPlugin) gsap.registerPlugin(ScrollToPlugin);

    startTicker();
    if (reduceMotion) { html.classList.remove("js"); return; }

    // Logo equalizer
    gsap.to(".eq1", { transformOrigin: "50% 50%", scaleY: 0.45, duration: 0.42, ease: "sine.inOut", repeat: -1, yoyo: true });
    gsap.to(".eq2", { transformOrigin: "50% 50%", scaleY: 0.55, duration: 0.55, ease: "sine.inOut", repeat: -1, yoyo: true, delay: 0.15 });
    gsap.to(".eq3", { transformOrigin: "50% 50%", scaleY: 1.7, duration: 0.36, ease: "sine.inOut", repeat: -1, yoyo: true, delay: 0.3 });

    // LED blinks to the beat
    gsap.to(".led", { opacity: 0.35, duration: 0.45, ease: "steps(1)", repeat: -1, yoyo: true });

    const tl = gsap.timeline({ defaults: { ease: "power4.out" } });
    tl.from(".nav", { y: -20, autoAlpha: 0, duration: 0.8 }, 0)
      .fromTo(".headline .word", { yPercent: 105, autoAlpha: 1 }, { yPercent: 0, duration: 1.1, stagger: 0.1 }, 0.1)
      .fromTo(".sub", { y: 18, autoAlpha: 0 }, { y: 0, autoAlpha: 1, duration: 0.9 }, 0.6)
      .fromTo(".cta", { y: 18, autoAlpha: 0 }, { y: 0, autoAlpha: 1, duration: 0.9 }, 0.75)
      .fromTo("#player", { autoAlpha: 0, y: 60, rotation: 6 }, { autoAlpha: 1, y: 0, rotation: 0, duration: 1.3, ease: "expo.out" }, 0.25)
      .add(startPlayer(), 0.9);

    if (window.ScrollTrigger) {
      ScrollTrigger.batch(".reveal", {
        start: "top 88%",
        onEnter: (els) => gsap.fromTo(els, { y: 40, autoAlpha: 0 }, { y: 0, autoAlpha: 1, duration: 1, stagger: 0.12, ease: "power3.out", overwrite: true }),
      });
      // The video tilts flat as it scrolls in, like a sleeve being laid down
      gsap.fromTo(".video-frame", { rotation: -2.5, scale: 0.94 }, {
        rotation: 0, scale: 1, ease: "none",
        scrollTrigger: { trigger: ".video-frame", start: "top bottom", end: "top 35%", scrub: 0.6 },
      });
      // Big footer letters rise in once, one after another
      gsap.fromTo(".foot-word span", { yPercent: 60 }, {
        yPercent: 0, duration: 0.9, stagger: 0.05, ease: "power3.out",
        scrollTrigger: { trigger: ".foot", start: "top 85%" },
      });
    } else {
      gsap.set(".reveal", { autoAlpha: 1 });
    }
  }

  /* ------------------------------------------------------------------ */
  /* Boot                                                                */
  /* ------------------------------------------------------------------ */
  async function boot() {
    const [platform, releases] = await Promise.all([detectPlatform(), loadReleases()]);
    state.os = platform.os;
    state.arch = platform.arch;
    state.releases = releases;

    const wanted = new URLSearchParams(location.search).get("version");
    const norm = (t) => (t || "").replace(/^v/, "");
    state.current = releases.find((r) => wanted && norm(r.tag) === norm(wanted)) || releases[0];

    fillVersions();
    render();
    markDetected();
  }

  function init() {
    bindUI();
    animate();
    boot();
  }

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", init);
  else init();
})();
