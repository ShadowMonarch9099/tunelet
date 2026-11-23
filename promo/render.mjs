// Renders video.html to MP4: headless Edge steps the timeline frame by frame,
// then ffmpeg encodes the frames.
//
//   node render.mjs                 full video -> out/tunelet-promo.mp4
//   node render.mjs --stills 1,8,12 PNGs of those seconds -> out/still-<t>.png
//   node render.mjs --from 7 --to 17 --preview   a quick low-res clip of a section
import puppeteer from "puppeteer-core";
import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, existsSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const args = process.argv.slice(2);
const arg = (k, d) => {
  const i = args.indexOf("--" + k);
  return i < 0 ? d : args[i + 1] ?? true;
};
const EDGE = [
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  "C:/Program Files/Microsoft/Edge/Application/msedge.exe",
].find(existsSync);
const FFMPEG =
  process.env.FFMPEG ||
  resolve(process.env.LOCALAPPDATA, "Microsoft/WinGet/Packages/Gyan.FFmpeg_Microsoft.Winget.Source_8wekyb3d8bbwe/ffmpeg-9.1-full_build/bin/ffmpeg.exe");

const preview = args.includes("--preview");
const browser = await puppeteer.launch({
  executablePath: EDGE,
  headless: true,
  args: ["--allow-file-access-from-files", "--disable-web-security", "--hide-scrollbars"],
});
const page = await browser.newPage();
await page.setViewport({ width: 1920, height: 1080, deviceScaleFactor: preview ? 0.5 : 1 });
await page.goto(pathToFileURL(resolve("video.html")).href, { waitUntil: "load" });
await page.evaluate(() => document.fonts.ready);
const { DURATION, FPS } = await page.evaluate(() => ({ DURATION: window.DURATION, FPS: window.FPS }));
mkdirSync("out", { recursive: true });

const stills = arg("stills", null);
if (stills) {
  for (const s of String(stills).split(",")) {
    await page.evaluate((t) => window.seek(t), Number(s));
    await page.screenshot({ path: `out/still-${s}.png` });
    console.log("still", s);
  }
  await browser.close();
  process.exit(0);
}

const from = Number(arg("from", 0));
const to = Math.min(Number(arg("to", DURATION)), DURATION);
rmSync("frames", { recursive: true, force: true });
mkdirSync("frames", { recursive: true });
const total = Math.round((to - from) * FPS);
const t0 = Date.now();
for (let f = 0; f < total; f++) {
  await page.evaluate((t) => window.seek(t), from + f / FPS);
  await page.screenshot({ path: `frames/${String(f).padStart(5, "0")}.jpg`, type: "jpeg", quality: 92 });
  if (f % 60 === 0) console.log(`frame ${f}/${total}  (${((Date.now() - t0) / 1000).toFixed(0)}s)`);
}
await browser.close();

const out = preview ? `out/preview-${from}-${to}.mp4` : "out/tunelet-promo.mp4";
execFileSync(FFMPEG, [
  "-y", "-hide_banner", "-loglevel", "error",
  "-framerate", String(FPS), "-i", "frames/%05d.jpg",
  "-c:v", "libx264", "-preset", preview ? "veryfast" : "slow", "-crf", preview ? "26" : "17",
  "-pix_fmt", "yuv420p", "-movflags", "+faststart",
  out,
]);
console.log(`done: ${out}  (${total} frames, ${((Date.now() - t0) / 1000).toFixed(0)}s)`);
