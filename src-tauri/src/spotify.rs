//! Spotify Web API: login (PKCE, bring-your-own Client ID), search, play, queue.
//!
//! Each user registers their own free Spotify developer app and pastes its
//! Client ID, because Spotify only lets a handful of users share one app in
//! development mode. PKCE means no client secret is ever needed or stored.
//! The Client ID and refresh token live in Windows Credential Manager.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{distributions::Alphanumeric, Rng};
use reqwest::{Method, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tauri::{async_runtime::Mutex, AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

pub const REDIRECT_PORT: u16 = 43821;
pub const REDIRECT_URI: &str = "http://127.0.0.1:43821/callback";
const SCOPES: &str = "user-read-playback-state user-modify-playback-state \
                      user-library-read user-library-modify playlist-read-private";
const API: &str = "https://api.spotify.com/v1";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const KEYRING_SERVICE: &str = "Tunelet";
const KEYRING_USER: &str = "spotify";

type Res<T> = Result<T, String>;

pub struct Spotify {
    http: reqwest::Client,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    client_id: Option<String>,
    refresh: Option<String>,
    access: Option<(String, Instant)>,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    client_id: String,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct TokenResp {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    client_id: Option<String>,
    logged_in: bool,
    redirect_uri: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    kind: &'static str,
    uri: String,
    name: String,
    subtitle: String,
    image: Option<String>,
}

fn keyring_entry() -> Option<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()
}

fn load_saved() -> Option<Saved> {
    serde_json::from_str(&keyring_entry()?.get_password().ok()?).ok()
}

fn persist(inner: &Inner) {
    let Some(entry) = keyring_entry() else { return };
    match &inner.client_id {
        Some(id) => {
            let saved = Saved { client_id: id.clone(), refresh_token: inner.refresh.clone() };
            if let Ok(s) = serde_json::to_string(&saved) {
                let _ = entry.set_password(&s);
            }
        }
        None => {
            let _ = entry.delete_credential();
        }
    }
}

fn net_err(e: reqwest::Error) -> String {
    format!("Couldn't reach Spotify ({e}). Check your internet connection.")
}

fn random_string(len: usize) -> String {
    rand::thread_rng().sample_iter(&Alphanumeric).take(len).map(char::from).collect()
}

impl Spotify {
    pub fn new() -> Self {
        let saved = load_saved();
        Self {
            http: reqwest::Client::new(),
            inner: Mutex::new(Inner {
                client_id: saved.as_ref().map(|s| s.client_id.clone()),
                refresh: saved.and_then(|s| s.refresh_token),
                access: None,
            }),
        }
    }

    fn store_tokens(inner: &mut Inner, t: TokenResp) -> String {
        if let Some(r) = t.refresh_token {
            inner.refresh = Some(r);
        }
        let expires = Instant::now() + Duration::from_secs(t.expires_in);
        inner.access = Some((t.access_token.clone(), expires));
        persist(inner);
        t.access_token
    }

    /// A valid access token, refreshing it if it's about to expire.
    async fn token(&self) -> Res<String> {
        let mut g = self.inner.lock().await;
        if let Some((t, exp)) = &g.access {
            if Instant::now() + Duration::from_secs(60) < *exp {
                return Ok(t.clone());
            }
        }
        let (Some(cid), Some(rt)) = (g.client_id.clone(), g.refresh.clone()) else {
            return Err("not_logged_in".into());
        };
        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&[("grant_type", "refresh_token"), ("refresh_token", &rt), ("client_id", &cid)])
            .send()
            .await
            .map_err(net_err)?;
        if !resp.status().is_success() {
            if resp.status() == StatusCode::BAD_REQUEST {
                // Refresh token revoked or expired: require a fresh login.
                g.refresh = None;
                g.access = None;
                persist(&g);
                return Err("not_logged_in".into());
            }
            return Err(format!("Spotify login service error ({}).", resp.status()));
        }
        let t: TokenResp = resp.json().await.map_err(net_err)?;
        Ok(Self::store_tokens(&mut g, t))
    }

    async fn api(&self, method: Method, path: &str, query: &[(&str, &str)], body: Option<Value>) -> Res<Response> {
        let token = self.token().await?;
        let mut req = self
            .http
            .request(method, format!("{API}{path}"))
            .bearer_auth(token)
            .query(query);
        req = match body {
            Some(b) => req.json(&b),
            None => req.header(reqwest::header::CONTENT_LENGTH, 0),
        };
        let resp = req.send().await.map_err(net_err)?;
        if resp.status() == StatusCode::UNAUTHORIZED {
            self.inner.lock().await.access = None;
        }
        Ok(resp)
    }

    async fn check(resp: Response) -> Res<Response> {
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        let body = resp.text().await.unwrap_or_default();
        let reason = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| v["error"]["reason"].as_str().map(String::from))
            .unwrap_or_default();
        Err(match status {
            StatusCode::UNAUTHORIZED => "not_logged_in".into(),
            StatusCode::FORBIDDEN if reason == "PREMIUM_REQUIRED" => {
                "Spotify Premium is required to control playback.".into()
            }
            StatusCode::FORBIDDEN => {
                "Spotify refused that. If you just made your developer app, make sure your account is added under User Management.".into()
            }
            StatusCode::NOT_FOUND => "No Spotify device found. Open the Spotify app first.".into(),
            StatusCode::TOO_MANY_REQUESTS => "Spotify is rate-limiting requests. Try again in a moment.".into(),
            _ => format!("Spotify error {status}."),
        })
    }

    async fn first_device(&self) -> Res<String> {
        let resp = Self::check(self.api(Method::GET, "/me/player/devices", &[], None).await?).await?;
        let v: Value = resp.json().await.map_err(net_err)?;
        let devices = v["devices"].as_array().cloned().unwrap_or_default();
        devices
            .iter()
            .filter(|d| !d["is_restricted"].as_bool().unwrap_or(false))
            .max_by_key(|d| d["type"].as_str() == Some("Computer"))
            .and_then(|d| d["id"].as_str().map(String::from))
            .ok_or_else(|| "No Spotify device found. Open the Spotify app first.".into())
    }

    async fn login(&self, app: &AppHandle, client_id: String) -> Res<()> {
        let client_id = client_id.trim().to_string();
        if client_id.len() != 32 || !client_id.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err("That doesn't look like a Client ID. It should be 32 letters and numbers.".into());
        }

        let verifier = random_string(64);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let state = random_string(16);

        let listener = TcpListener::bind(("127.0.0.1", REDIRECT_PORT))
            .await
            .map_err(|_| format!("Port {REDIRECT_PORT} is in use. Close whatever is using it and try again."))?;

        let url = reqwest::Url::parse_with_params(
            "https://accounts.spotify.com/authorize",
            &[
                ("client_id", client_id.as_str()),
                ("response_type", "code"),
                ("redirect_uri", REDIRECT_URI),
                ("code_challenge_method", "S256"),
                ("code_challenge", challenge.as_str()),
                ("scope", SCOPES),
                ("state", state.as_str()),
            ],
        )
        .map_err(|e| e.to_string())?;
        app.opener()
            .open_url(url.as_str(), None::<&str>)
            .map_err(|e| format!("Couldn't open your browser: {e}"))?;

        let code = tokio::time::timeout(Duration::from_secs(180), wait_for_code(&listener, &state))
            .await
            .map_err(|_| "Login timed out. Try again.".to_string())??;
        drop(listener);

        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code.as_str()),
                ("redirect_uri", REDIRECT_URI),
                ("client_id", client_id.as_str()),
                ("code_verifier", verifier.as_str()),
            ])
            .send()
            .await
            .map_err(net_err)?;
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Spotify refused the login: {body}"));
        }
        let t: TokenResp = resp.json().await.map_err(net_err)?;

        let mut g = self.inner.lock().await;
        g.client_id = Some(client_id);
        Self::store_tokens(&mut g, t);
        Ok(())
    }
}

async fn wait_for_code(listener: &TcpListener, state: &str) -> Res<String> {
    loop {
        let (mut sock, _) = listener.accept().await.map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; 8192];
        let n = sock.read(&mut buf).await.unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]);
        let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();

        if !path.starts_with("/callback") {
            let _ = sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
            continue;
        }

        let q: HashMap<String, String> = reqwest::Url::parse(&format!("http://127.0.0.1{path}"))
            .map(|u| u.query_pairs().into_owned().collect())
            .unwrap_or_default();
        let ok = q.get("state").map(String::as_str) == Some(state) && q.contains_key("code");

        let page = callback_page(ok);
        let _ = sock
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
                    page.len()
                )
                .as_bytes(),
            )
            .await;

        if ok {
            return Ok(q["code"].clone());
        }
        if let Some(err) = q.get("error") {
            return Err(format!("Spotify login was cancelled ({err})."));
        }
    }
}

fn callback_page(ok: bool) -> String {
    let (title, msg) = if ok {
        ("Connected to Tunelet", "You can close this tab and go back to what you were doing.")
    } else {
        ("Login didn't finish", "Go back to Tunelet and try again.")
    };
    format!(
        "<!doctype html><meta charset=utf-8><title>{title}</title>\
         <body style=\"font-family:Segoe UI,system-ui,sans-serif;background:#121212;color:#fff;\
         display:grid;place-items:center;height:100vh;margin:0\">\
         <div style=\"text-align:center\"><h2>{title}</h2><p style=\"opacity:.7\">{msg}</p></div>"
    )
}

fn smallest_image(images: &Value) -> Option<String> {
    images.as_array()?.last()?["url"].as_str().map(String::from)
}

fn artist_names(v: &Value) -> String {
    v["artists"]
        .as_array()
        .map(|a| a.iter().filter_map(|x| x["name"].as_str()).collect::<Vec<_>>().join(", "))
        .unwrap_or_default()
}

fn parse_search(v: &Value) -> Vec<Item> {
    let mut out = Vec::new();
    let list = |key: &str| v[key]["items"].as_array().cloned().unwrap_or_default();

    for t in list("tracks").iter().filter(|t| !t.is_null()) {
        out.push(Item {
            kind: "track",
            uri: t["uri"].as_str().unwrap_or_default().into(),
            name: t["name"].as_str().unwrap_or_default().into(),
            subtitle: artist_names(t),
            image: smallest_image(&t["album"]["images"]),
        });
    }
    for a in list("albums").iter().filter(|a| !a.is_null()).take(3) {
        out.push(Item {
            kind: "album",
            uri: a["uri"].as_str().unwrap_or_default().into(),
            name: a["name"].as_str().unwrap_or_default().into(),
            subtitle: format!("Album · {}", artist_names(a)),
            image: smallest_image(&a["images"]),
        });
    }
    for p in list("playlists").iter().filter(|p| !p.is_null()).take(3) {
        out.push(Item {
            kind: "playlist",
            uri: p["uri"].as_str().unwrap_or_default().into(),
            name: p["name"].as_str().unwrap_or_default().into(),
            subtitle: format!("Playlist · {}", p["owner"]["display_name"].as_str().unwrap_or("Spotify")),
            image: smallest_image(&p["images"]),
        });
    }
    out.retain(|i| !i.uri.is_empty());
    out
}

// ---- player, library, devices ----------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    id: String,
    name: String,
    kind: String,
    active: bool,
    volume: Option<u64>,
    supports_volume: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    /// False when no device is playing or paused anywhere.
    active: bool,
    device: Option<Device>,
    track_uri: Option<String>,
    liked: Option<bool>,
    is_playing: bool,
}

fn parse_device(d: &Value) -> Option<Device> {
    Some(Device {
        id: d["id"].as_str()?.into(),
        name: d["name"].as_str().unwrap_or("Unknown device").into(),
        kind: d["type"].as_str().unwrap_or_default().into(),
        active: d["is_active"].as_bool().unwrap_or(false),
        volume: d["volume_percent"].as_u64(),
        supports_volume: d["supports_volume"].as_bool().unwrap_or(true),
    })
}

impl Spotify {
    async fn is_liked(&self, uri: &str) -> Res<bool> {
        let resp = self.api(Method::GET, "/me/library/contains", &[("uris", uri)], None).await?;
        let v: Value = Self::check(resp).await?.json().await.map_err(net_err)?;
        Ok(v.get(0).and_then(Value::as_bool).unwrap_or(false))
    }

    async fn set_liked(&self, uri: &str, liked: bool) -> Res<()> {
        let method = if liked { Method::PUT } else { Method::DELETE };
        Self::check(self.api(method, "/me/library", &[("uris", uri)], None).await?).await?;
        Ok(())
    }

    async fn player(&self) -> Res<PlayerState> {
        let resp = self.api(Method::GET, "/me/player", &[], None).await?;
        if resp.status() == StatusCode::NO_CONTENT {
            return Ok(PlayerState { active: false, device: None, track_uri: None, liked: None, is_playing: false });
        }
        let v: Value = Self::check(resp).await?.json().await.map_err(net_err)?;
        let track_uri = v["item"]["uri"].as_str().map(String::from);
        let liked = match track_uri.as_deref() {
            Some(uri) if uri.starts_with("spotify:track:") => self.is_liked(uri).await.ok(),
            _ => None,
        };
        Ok(PlayerState {
            active: true,
            device: parse_device(&v["device"]),
            track_uri,
            liked,
            is_playing: v["is_playing"].as_bool().unwrap_or(false),
        })
    }

    /// Likes the current song, or unlikes it if it's already liked.
    /// Returns the new liked state.
    pub async fn toggle_like_current(&self) -> Res<bool> {
        let p = self.player().await?;
        let uri = p
            .track_uri
            .filter(|u| u.starts_with("spotify:track:"))
            .ok_or("Nothing to like right now.")?;
        let liked = !self.is_liked(&uri).await?;
        self.set_liked(&uri, liked).await?;
        Ok(liked)
    }

    async fn user_id(&self) -> Res<String> {
        let v: Value = Self::check(self.api(Method::GET, "/me", &[], None).await?)
            .await?
            .json()
            .await
            .map_err(net_err)?;
        v["id"].as_str().map(String::from).ok_or_else(|| "Couldn't read your Spotify profile.".into())
    }
}

// ---- Tauri commands -------------------------------------------------------

#[tauri::command]
pub async fn spotify_status(sp: State<'_, Spotify>) -> Res<AuthStatus> {
    let g = sp.inner.lock().await;
    Ok(AuthStatus {
        client_id: g.client_id.clone(),
        logged_in: g.refresh.is_some(),
        redirect_uri: REDIRECT_URI,
    })
}

#[tauri::command]
pub async fn spotify_login(app: AppHandle, sp: State<'_, Spotify>, client_id: String) -> Res<()> {
    sp.login(&app, client_id).await
}

#[tauri::command]
pub async fn spotify_logout(sp: State<'_, Spotify>) -> Res<()> {
    let mut g = sp.inner.lock().await;
    g.refresh = None;
    g.access = None;
    persist(&g);
    Ok(())
}

#[tauri::command]
pub async fn spotify_search(sp: State<'_, Spotify>, query: String) -> Res<Vec<Item>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let resp = sp
        .api(Method::GET, "/search", &[("q", q), ("type", "track,album,playlist"), ("limit", "6")], None)
        .await?;
    let v: Value = Spotify::check(resp).await?.json().await.map_err(net_err)?;
    Ok(parse_search(&v))
}

#[tauri::command]
pub async fn spotify_play(sp: State<'_, Spotify>, uri: String) -> Res<()> {
    let body = if uri.starts_with("spotify:track:") {
        json!({ "uris": [uri] })
    } else {
        json!({ "context_uri": uri })
    };
    let resp = sp.api(Method::PUT, "/me/player/play", &[], Some(body.clone())).await?;
    if resp.status() == StatusCode::NOT_FOUND {
        // Nothing active yet: wake the Spotify app on this PC and play there.
        let device = sp.first_device().await?;
        let resp = sp.api(Method::PUT, "/me/player/play", &[("device_id", &device)], Some(body)).await?;
        Spotify::check(resp).await?;
        return Ok(());
    }
    Spotify::check(resp).await?;
    Ok(())
}

#[tauri::command]
pub async fn spotify_queue(sp: State<'_, Spotify>, uri: String) -> Res<()> {
    let resp = sp.api(Method::POST, "/me/player/queue", &[("uri", &uri)], None).await?;
    Spotify::check(resp).await?;
    Ok(())
}

#[tauri::command]
pub async fn spotify_player(sp: State<'_, Spotify>) -> Res<PlayerState> {
    sp.player().await
}

#[tauri::command]
pub async fn spotify_set_liked(sp: State<'_, Spotify>, uri: String, liked: bool) -> Res<()> {
    sp.set_liked(&uri, liked).await
}

#[tauri::command]
pub async fn spotify_toggle_like(sp: State<'_, Spotify>) -> Res<bool> {
    sp.toggle_like_current().await
}

#[tauri::command]
pub async fn spotify_seek(sp: State<'_, Spotify>, position_ms: u64) -> Res<()> {
    let pos = position_ms.to_string();
    Spotify::check(sp.api(Method::PUT, "/me/player/seek", &[("position_ms", &pos)], None).await?).await?;
    Ok(())
}

#[tauri::command]
pub async fn spotify_volume(sp: State<'_, Spotify>, percent: u8) -> Res<()> {
    let pct = percent.min(100).to_string();
    Spotify::check(sp.api(Method::PUT, "/me/player/volume", &[("volume_percent", &pct)], None).await?).await?;
    Ok(())
}

#[tauri::command]
pub async fn spotify_devices(sp: State<'_, Spotify>) -> Res<Vec<Device>> {
    let resp = Spotify::check(sp.api(Method::GET, "/me/player/devices", &[], None).await?).await?;
    let v: Value = resp.json().await.map_err(net_err)?;
    Ok(v["devices"].as_array().map(|a| a.iter().filter_map(parse_device).collect()).unwrap_or_default())
}

/// Moves playback to another device (phone, speaker, PC…), keeping it playing.
#[tauri::command]
pub async fn spotify_transfer(sp: State<'_, Spotify>, device_id: String) -> Res<()> {
    let body = json!({ "device_ids": [device_id], "play": true });
    Spotify::check(sp.api(Method::PUT, "/me/player", &[], Some(body)).await?).await?;
    Ok(())
}

/// "Liked Songs" first, then the user's playlists.
#[tauri::command]
pub async fn spotify_playlists(sp: State<'_, Spotify>) -> Res<Vec<Item>> {
    let mut out = Vec::new();
    if let Ok(id) = sp.user_id().await {
        out.push(Item {
            kind: "liked",
            uri: format!("spotify:user:{id}:collection"),
            name: "Liked Songs".into(),
            subtitle: "Your liked songs".into(),
            image: None,
        });
    }
    let resp = Spotify::check(sp.api(Method::GET, "/me/playlists", &[("limit", "50")], None).await?).await?;
    let v: Value = resp.json().await.map_err(net_err)?;
    for p in v["items"].as_array().cloned().unwrap_or_default().iter().filter(|p| !p.is_null()) {
        // Feb 2026 API renamed `tracks` to `items`; accept either.
        let total = p["items"]["total"].as_u64().or_else(|| p["tracks"]["total"].as_u64());
        let owner = p["owner"]["display_name"].as_str().unwrap_or("Spotify");
        out.push(Item {
            kind: "playlist",
            uri: p["uri"].as_str().unwrap_or_default().into(),
            name: p["name"].as_str().unwrap_or_default().into(),
            subtitle: match total {
                Some(n) => format!("{n} songs · {owner}"),
                None => owner.to_string(),
            },
            image: smallest_image(&p["images"]),
        });
    }
    out.retain(|i| !i.uri.is_empty());
    Ok(out)
}
