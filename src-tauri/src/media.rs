//! Now-playing state and transport controls via Windows' media session API
//! (the same thing that powers the media card in the volume flyout).
//! Needs no Spotify login and works with any Spotify account.

use serde::Serialize;
use std::sync::mpsc;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    /// False when Spotify isn't running or has no active session.
    pub available: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    /// Album cover as a data: URL.
    pub art: Option<String>,
    /// Two colors picked from the cover, as #rrggbb.
    pub colors: Option<[String; 2]>,
}

/// Playback position, sent separately (and often) so the large now-playing
/// payload with the cover image only goes out when something changes.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeline {
    pub position_ms: u64,
    pub duration_ms: u64,
    pub playing: bool,
}

pub enum Cmd {
    TogglePlay,
    Next,
    Previous,
    /// Jump to a position in the current song (Windows media controls).
    Seek(u64),
    /// Re-send the current state (used when the UI loads).
    Refresh,
}

/// Handle to the media worker thread, kept in Tauri state.
pub struct Media(mpsc::Sender<Cmd>);

impl Media {
    pub fn send(&self, cmd: Cmd) {
        let _ = self.0.send(cmd);
    }
}

#[cfg(windows)]
pub fn start(app: tauri::AppHandle) -> Media {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("media".into())
        .spawn(move || win::run(app, rx))
        .expect("failed to start media thread");
    Media(tx)
}

#[cfg(not(windows))]
pub fn start(_app: tauri::AppHandle) -> Media {
    let (tx, _rx) = mpsc::channel();
    Media(tx)
}

#[cfg(windows)]
mod win {
    use super::{Cmd, NowPlaying, Timeline};
    use base64::Engine;
    use std::{
        sync::mpsc::{Receiver, RecvTimeoutError},
        time::Duration,
    };
    use tauri::{AppHandle, Emitter};
    use windows::{
        core::Result,
        Media::Control::{
            GlobalSystemMediaTransportControlsSession as Session,
            GlobalSystemMediaTransportControlsSessionManager as Manager,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
        },
        Storage::Streams::{DataReader, IRandomAccessStreamReference},
        Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED},
    };

    /// How often the session is checked for changes.
    const POLL: Duration = Duration::from_millis(600);
    /// Spotify often publishes the new title before the new cover, so keep
    /// re-reading the cover for a few polls after a track change.
    const ART_RETRY_POLLS: u8 = 5;

    struct Worker {
        manager: Option<Manager>,
        track_key: String,
        art: Option<String>,
        art_len: usize,
        colors: Option<[String; 2]>,
        art_polls_left: u8,
    }

    pub fn run(app: AppHandle, rx: Receiver<Cmd>) {
        unsafe {
            let _ = RoInitialize(RO_INIT_MULTITHREADED);
        }
        let mut w = Worker {
            manager: None,
            track_key: String::new(),
            art: None,
            art_len: 0,
            colors: None,
            art_polls_left: 0,
        };
        let mut last = NowPlaying::default();

        loop {
            let mut force = false;
            match rx.recv_timeout(POLL) {
                Ok(Cmd::Refresh) => force = true,
                Ok(cmd) => {
                    if let Some(s) = w.spotify_session() {
                        let _ = control(&s, cmd);
                        // Give Spotify a moment to update its session.
                        std::thread::sleep(Duration::from_millis(120));
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }

            let state = w.read().unwrap_or_default();
            if force || state != last {
                let _ = app.emit("now-playing", &state);
                last = state;
            }
            if let Some(t) = w.spotify_session().and_then(|s| timeline(&s).ok()) {
                let _ = app.emit("timeline", &t);
            }
        }
    }

    fn control(s: &Session, cmd: Cmd) -> Result<()> {
        match cmd {
            Cmd::TogglePlay => s.TryTogglePlayPauseAsync()?.join()?,
            Cmd::Next => s.TrySkipNextAsync()?.join()?,
            Cmd::Previous => s.TrySkipPreviousAsync()?.join()?,
            Cmd::Seek(ms) => s.TryChangePlaybackPositionAsync(ms as i64 * 10_000)?.join()?,
            Cmd::Refresh => true,
        };
        Ok(())
    }

    /// Current time in Windows' 100-ns ticks since 1601 (the DateTime epoch).
    fn now_ticks() -> i64 {
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        (unix.as_nanos() / 100) as i64 + 116_444_736_000_000_000
    }

    fn timeline(s: &Session) -> Result<Timeline> {
        let t = s.GetTimelineProperties()?;
        let playing = s.GetPlaybackInfo()?.PlaybackStatus()? == Status::Playing;
        let end = t.EndTime()?.Duration.max(0);
        let mut pos = t.Position()?.Duration;
        // Apps update the position only now and then; extrapolate while playing.
        if playing {
            let elapsed = now_ticks() - t.LastUpdatedTime()?.UniversalTime;
            if elapsed > 0 {
                pos += elapsed;
            }
        }
        let pos = pos.clamp(0, end.max(0));
        Ok(Timeline {
            position_ms: (pos / 10_000) as u64,
            duration_ms: (end / 10_000) as u64,
            playing,
        })
    }

    impl Worker {
        fn spotify_session(&mut self) -> Option<Session> {
            if self.manager.is_none() {
                self.manager = Manager::RequestAsync().ok()?.join().ok();
            }
            let sessions = self.manager.as_ref()?.GetSessions().ok()?;
            sessions.into_iter().find(|s| {
                s.SourceAppUserModelId()
                    .map(|id| id.to_string().to_lowercase().contains("spotify"))
                    .unwrap_or(false)
            })
        }

        fn read(&mut self) -> Result<NowPlaying> {
            let Some(s) = self.spotify_session() else {
                return Ok(NowPlaying::default());
            };
            let props = s.TryGetMediaPropertiesAsync()?.join()?;
            let title = props.Title()?.to_string();
            let artist = props.Artist()?.to_string();
            let album = props.AlbumTitle()?.to_string();
            let playing = s.GetPlaybackInfo()?.PlaybackStatus()? == Status::Playing;

            let key = format!("{title}\u{1}{artist}\u{1}{album}");
            if key != self.track_key {
                self.track_key = key;
                self.art_polls_left = ART_RETRY_POLLS;
            }
            if self.art_polls_left > 0 {
                self.art_polls_left -= 1;
                if let Ok(bytes) = props.Thumbnail().and_then(|t| read_bytes(&t)) {
                    if !bytes.is_empty() && bytes.len() != self.art_len {
                        self.art_len = bytes.len();
                        self.colors = crate::colors::extract(&bytes);
                        self.art = Some(format!(
                            "data:{};base64,{}",
                            sniff_mime(&bytes),
                            base64::engine::general_purpose::STANDARD.encode(&bytes)
                        ));
                    }
                }
            }

            Ok(NowPlaying {
                available: !title.is_empty(),
                title,
                artist,
                album,
                playing,
                art: self.art.clone(),
                colors: self.colors.clone(),
            })
        }
    }

    fn read_bytes(r: &IRandomAccessStreamReference) -> Result<Vec<u8>> {
        let stream = r.OpenReadAsync()?.join()?;
        let size = stream.Size()? as u32;
        let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
        reader.LoadAsync(size)?.join()?;
        let mut buf = vec![0u8; size as usize];
        reader.ReadBytes(&mut buf)?;
        Ok(buf)
    }

    fn sniff_mime(b: &[u8]) -> &'static str {
        if b.starts_with(&[0x89, b'P', b'N', b'G']) { "image/png" } else { "image/jpeg" }
    }
}
