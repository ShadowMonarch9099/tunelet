// Films the montage clips off-screen: the real Tunelet UI (served by Vite on
// :1420, with stage/mock.js standing in for the Rust side) inside
// stage/stage.html, in headless Edge. Input is synthetic and stays inside that
// browser. Each clip -> footage/<name>/0001.jpg … at 30 fps.
//
//   (in the repo root) npm run dev        # serves the Tunelet UI
//   node capture.mjs [clipName ...]       # all clips, or just some
import puppeteer from "puppeteer-core";
import { mkdirSync, rmSync, writeFileSync, readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const EDGE = ["C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe", "C:/Program Files/Microsoft/Edge/Application/msedge.exe"].find(existsSync);
const FPS = 30;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const browser = await puppeteer.launch({
  executablePath: EDGE,
  headless: true,
  args: ["--allow-file-access-from-files", "--disable-web-security", "--hide-scrollbars", "--autoplay-policy=no-user-gesture-required"],
});
const page = await browser.newPage();
await page.setViewport({ width: 1920, height: 1080, deviceScaleFactor: 1 });
await page.evaluateOnNewDocument(readFileSync("stage/mock.js", "utf8"));
page.on("console", (m) => { if (m.type() === "warning" || m.type() === "error") console.log("[page]", m.text()); });
await page.goto(pathToFileURL(resolve("stage/stage.html")).href, { waitUntil: "load" });

// Wait for the Tunelet frame to boot.
let app;
for (let i = 0; i < 100 && !app; i++) {
  app = page.frames().find((f) => f.url().startsWith("http://localhost:1420"));
  if (app && !(await app.evaluate(() => !!window.__tunelet).catch(() => false))) app = null;
  if (!app) await sleep(200);
}
if (!app) throw new Error("Tunelet UI didn't load. Is `npm run dev` running?");
await sleep(1500);

const T = (fn, ...a) => app.evaluate(fn, ...a);
const bob = async () => {
  const st = await T(() => ({ b: window.__mock.state().bounds, at: { x: window.__tunelet.state().bobAt.x, y: window.__tunelet.state().bobAt.y } }));
  const p = { x: st.b.x + st.at.x, y: st.b.y + st.at.y };
  if (!Number.isFinite(p.x) || !Number.isFinite(p.y)) throw new Error("bad bob position " + JSON.stringify(st));
  return p;
};
const reset = async () => {
  await page.mouse.move(60, 1040); // no hover effects left over from the last clip
  await T(() => window.__tunelet.moveTo(1215, 620));
  await page.evaluate(() => { window.hideKeys(); document.getElementById("tunelet").style.opacity = "1"; });
  await sleep(700);
};

// ---- screencast recording ----
const cdp = await page.createCDPSession();
let frames = null;
cdp.on("Page.screencastFrame", async ({ data, metadata, sessionId }) => {
  if (frames) frames.push({ t: metadata.timestamp, buf: Buffer.from(data, "base64") });
  await cdp.send("Page.screencastFrameAck", { sessionId }).catch(() => {});
});
async function record(name, seconds, action) {
  frames = [];
  await cdp.send("Page.startScreencast", { format: "jpeg", quality: 92, maxWidth: 1920, maxHeight: 1080, everyNthFrame: 1 });
  await sleep(150);
  const t0 = Date.now() / 1000;
  const run = action();
  await sleep(seconds * 1000);
  await run;
  await cdp.send("Page.stopScreencast");
  const got = frames;
  frames = null;
  // Resample to a steady 30 fps (screencast only sends frames when something changes).
  const dir = `footage/${name}`;
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const n = Math.round(seconds * FPS);
  let j = 0;
  for (let f = 0; f < n; f++) {
    const t = t0 + f / FPS;
    while (j + 1 < got.length && got[j + 1].t <= t) j++;
    writeFileSync(`${dir}/${String(f + 1).padStart(4, "0")}.jpg`, got[j].buf);
  }
  console.log(`${name}: ${n} frames from ${got.length} screencast frames`);
}

const CLIPS = {
  // Typing keeps going while a right-click on the bubble starts the music.
  async focus() {
    await reset();
    await T(() => { window.__mock.seek(48000); });
    await page.evaluate(() => window.focusCode());
    const b = await bob();
    await record("focus", 3.6, async () => {
      await page.keyboard.type("\n  session.start();", { delay: 55 });
      await page.mouse.click(b.x, b.y, { button: "right" });
      await sleep(250);
      await page.keyboard.type("  // still in flow", { delay: 55 });
    });
  },
  // One click: the menu springs out.
  async menu() {
    await reset();
    const b = await bob();
    await record("menu", 2.6, async () => {
      await sleep(300);
      await page.mouse.click(b.x, b.y);
    });
  },
  // Hotkey -> search -> type -> Enter plays it.
  async search() {
    await reset();
    await record("search", 5.2, async () => {
      await page.evaluate(() => window.showKeys(["Alt", "X"]));
      await T(() => window.__mock.hotkey("search"));
      await sleep(500);
      await page.evaluate(() => window.hideKeys());
      await sleep(300);
      await page.keyboard.type("golden", { delay: 110 });
      await sleep(700);
      await page.keyboard.press("Enter");
    });
  },
  // Skip songs: the whole card takes on the new cover's colours.
  async colors() {
    await reset();
    await T(() => window.__mock.play(4));
    await T(() => window.__tunelet.open("player"));
    await sleep(900);
    await record("colors", 3.4, async () => {
      await sleep(300);
      await page.evaluate(() => window.showKeys(["Alt", "D"]));
      await T(() => window.__mock.hotkey("next"));
      await sleep(1300);
      await page.evaluate(() => window.showKeys(["Alt", "D"]));
      await T(() => window.__mock.hotkey("next"));
      await sleep(600);
      await page.evaluate(() => window.hideKeys());
    });
  },
  // Lyrics follow the song.
  async lyrics() {
    await reset();
    await T(() => { window.__mock.play(0); window.__mock.seek(9800); });
    await T(() => window.__tunelet.open("lyrics"));
    await sleep(1000);
    await record("lyrics", 3.2, async () => {});
  },
  // One key hides it, the same key brings it back.
  async hide() {
    await reset();
    await record("hide", 3.4, async () => {
      await sleep(300);
      await page.evaluate(() => window.showKeys(["Alt", "Z"]));
      await sleep(250);
      await page.evaluate(() => window.toggleWidget());
      await sleep(800);
      await page.evaluate(() => window.showKeys(["Alt", "Z"]));
      await sleep(250);
      await page.evaluate(() => window.toggleWidget());
      await sleep(500);
      await page.evaluate(() => window.hideKeys());
    });
  },
};

const wanted = process.argv.slice(2);
for (const [name, fn] of Object.entries(CLIPS)) {
  if (wanted.length && !wanted.includes(name)) continue;
  await fn();
}
await browser.close();
