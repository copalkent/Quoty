import { invoke } from "@tauri-apps/api/core";
import type { Preferences, PreferencesPatch, QuotaSnapshot } from "./quota";

export function fetchSnapshot(): Promise<QuotaSnapshot> {
  return invoke<QuotaSnapshot>("quota_snapshot");
}

export function fetchPreferences(): Promise<Preferences> {
  return invoke<Preferences>("get_preferences");
}

export function savePreferences(patch: PreferencesPatch): Promise<Preferences> {
  return invoke<Preferences>("update_preferences", { patch });
}

export function setWindowExpanded(expanded: boolean): Promise<void> {
  return invoke<void>("set_expanded", { expanded });
}

export function beginDrag(): Promise<void> {
  return invoke<void>("begin_drag");
}

export function setWindowSize(width: number, height: number): Promise<void> {
  return invoke<void>("set_window_size", { width, height });
}

export function persistPosition(): Promise<void> {
  return invoke<void>("persist_position");
}

export function ensureOnScreen(): Promise<void> {
  return invoke<void>("ensure_on_screen");
}

export function hideWidget(): Promise<void> {
  return invoke<void>("hide_widget");
}

export function quitWidget(): Promise<void> {
  return invoke<void>("quit_app");
}

export interface SpotifyStatus {
  isRunning: boolean;
  isPlaying: boolean;
  artist: string;
  title: string;
}

export function fetchSpotifyStatus(): Promise<SpotifyStatus> {
  return invoke<SpotifyStatus>("spotify_status");
}

export function spotifyPlayPause(): Promise<void> {
  return invoke<void>("spotify_play_pause");
}

export function spotifyNext(): Promise<void> {
  return invoke<void>("spotify_next");
}

export function spotifyPrev(): Promise<void> {
  return invoke<void>("spotify_prev");
}

export function spotifyLaunch(): Promise<void> {
  return invoke<void>("spotify_launch");
}

