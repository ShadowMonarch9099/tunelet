//! Song lyrics from LRCLIB (https://lrclib.net), a free, open lyrics database.
//! Spotify's own lyrics aren't available to apps. Synced lyrics come with a
//! timestamp per line, so the card can follow along with the song.

use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    ms: u64,
    text: String,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    /// Timed lines, when LRCLIB has synced lyrics.
    synced: Option<Vec<Line>>,
    /// Plain text, when only unsynced lyrics exist.
    plain: Option<String>,
    instrumental: bool,
}

/// Parses LRC lines like "[01:23.45] text".
fn parse_lrc(lrc: &str) -> Vec<Line> {
    let mut out = Vec::new();
    for raw in lrc.lines() {
        let mut rest = raw.trim();
        let mut stamps = Vec::new();
        while let Some(end) = rest.strip_prefix('[').and_then(|r| r.find(']')) {
            let tag = &rest[1..=end];
            rest = rest[end + 2..].trim_start();
            let Some((m, s)) = tag.split_once(':') else { continue };
            if let (Ok(m), Ok(s)) = (m.parse::<u64>(), s.parse::<f64>()) {
                stamps.push(m * 60_000 + (s * 1000.0) as u64);
            }
        }
        for ms in stamps {
            out.push(Line { ms, text: rest.to_string() });
        }
    }
    out.sort_by_key(|l| l.ms);
    out
}

fn from_record(v: &Value) -> Lyrics {
    let synced = v["syncedLyrics"].as_str().map(parse_lrc).filter(|l| !l.is_empty());
    Lyrics {
        plain: if synced.is_none() { v["plainLyrics"].as_str().map(String::from) } else { None },
        synced,
        instrumental: v["instrumental"].as_bool().unwrap_or(false),
    }
}

/// Looks up lyrics for a song. `duration_ms` helps pick the right version.
#[tauri::command]
pub async fn lyrics(title: String, artist: String, album: String, duration_ms: u64) -> Result<Lyrics, String> {
    let c = reqwest::Client::builder()
        .connect_timeout(crate::settings::CONNECT_TIMEOUT)
        .timeout(Duration::from_secs(15))
        .user_agent("Tunelet (https://github.com/ShadowMonarch9099/tunelet)")
        .build()
        .map_err(|e| e.to_string())?;
    let artist1 = artist.split([',', '&']).next().unwrap_or(&artist).trim().to_string();
    let dur = (duration_ms / 1000).to_string();

    // Exact match first (title + artist + album + duration)…
    let resp = c
        .get("https://lrclib.net/api/get")
        .query(&[("track_name", &title), ("artist_name", &artist1), ("album_name", &album), ("duration", &dur)])
        .send()
        .await
        .map_err(|e| format!("Couldn't reach the lyrics service ({e})."))?;
    if resp.status().is_success() {
        if let Ok(v) = resp.json::<Value>().await {
            return Ok(from_record(&v));
        }
    }

    // …then a search, preferring synced lyrics with the closest length.
    let results: Value = c
        .get("https://lrclib.net/api/search")
        .query(&[("track_name", &title), ("artist_name", &artist1)])
        .send()
        .await
        .map_err(|e| format!("Couldn't reach the lyrics service ({e})."))?
        .json()
        .await
        .unwrap_or_default();
    let want = duration_ms as f64 / 1000.0;
    let best = results.as_array().and_then(|list| {
        list.iter().min_by(|a, b| {
            let score = |r: &Value| {
                let off = (r["duration"].as_f64().unwrap_or(0.0) - want).abs();
                off + if r["syncedLyrics"].is_string() { 0.0 } else { 30.0 }
            };
            score(a).total_cmp(&score(b))
        })
    });
    Ok(best.map(from_record).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lrc() {
        let l = parse_lrc("[00:12.50] Hello\n[01:02.00][01:30.25] Chorus\nno stamp\n[00:00.00]");
        let got: Vec<_> = l.iter().map(|l| (l.ms, l.text.as_str())).collect();
        assert_eq!(got, vec![(0, ""), (12_500, "Hello"), (62_000, "Chorus"), (90_250, "Chorus")]);
    }
}
