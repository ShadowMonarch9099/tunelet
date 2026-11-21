export type NowPlaying = {
  available: boolean;
  title: string;
  artist: string;
  album: string;
  playing: boolean;
  art: string | null;
  colors: [string, string] | null;
};

export type AuthStatus = { clientId: string | null; loggedIn: boolean; redirectUri: string };

export type Item = {
  kind: "track" | "album" | "playlist" | "liked";
  uri: string;
  name: string;
  subtitle: string;
  image: string | null;
  /** Playlists: whether you own it (Spotify only lists songs of your own playlists). */
  owned: boolean;
};

export type TrackPage = { tracks: Item[]; total: number; next: number | null };

export type Timeline = { positionMs: number; durationMs: number; playing: boolean };

export type Device = {
  id: string;
  name: string;
  kind: string;
  active: boolean;
  volume: number | null;
  supportsVolume: boolean;
};

export type PlayerState = {
  active: boolean;
  device: Device | null;
  trackUri: string | null;
  liked: boolean | null;
  isPlaying: boolean;
  shuffle: boolean;
  smartShuffle: boolean;
  contextUri: string | null;
};

export type Action = "toggle" | "search" | "playPause" | "next" | "previous" | "like";
export type Settings = { hotkeys: Record<Action, string>; opacity: number; launchSpotify: boolean };
export type HotkeyError = { action: Action; combo: string; reason: "taken" | "invalid" | "duplicate" };
export type SettingsView = { settings: Settings; autostart: boolean; version: string; hotkeyErrors: HotkeyError[] };

export type UpdateState =
  | { state: "idle" | "checking" | "none" | "installing" }
  | { state: "available"; version: string }
  | { state: "error"; msg: string };

/** Which card is open next to the bob. */
export type Card = "search" | "playlists" | "player" | "settings";
