# Tunelet download site

A single static page (no build step): `index.html`, `styles.css`, `main.js` and `assets/`.
It reads the release list from the GitHub API, so new releases appear on their own.

## Deploy on Vercel

1. In Vercel, **Add New → Project** and import the `tunelet` GitHub repo.
2. Set **Root Directory** to `site` (important: the repo root is the SvelteKit app).
3. Framework Preset: **Other**. Leave Build and Install commands empty.
4. Deploy. `vercel.json` adds caching and security headers, and `404.html` is the not-found page.

From the command line instead: `cd site` then `npx vercel` (and `npx vercel --prod` to go live).

## Run locally

```powershell
npx serve site
```

## Change things

- Promo video: replace `assets/promo.mp4` and its cover `assets/poster.jpg`.
- Fallback release (used only if GitHub can't be reached): `FALLBACK` at the top of `main.js`.
- Share a link to a specific version: `https://your-site/?version=v0.7.1`.
