import type { MusicTrack } from "./types";
export function totalDuration(tracks: readonly MusicTrack[]) {
  return tracks.reduce((total, track) => total + (Number.isFinite(track.durationSeconds) && (track.durationSeconds ?? 0) > 0 ? track.durationSeconds! : 0), 0);
}
export function collectionDuration(seconds: number) {
  const minutes = Math.floor(seconds / 60);
  return minutes >= 60 ? `${Math.floor(minutes / 60)} hr ${minutes % 60} min` : `${minutes} min`;
}
export type ViewLocation = { section: "Home" | "Library" | "Albums" | "Artists" | "Liked Songs" | "Playlists" | "Queue"; groupId: string | null; playlistId: string | null };
export type ViewHistory = { entries: ViewLocation[]; index: number };
export function pushView(history: ViewHistory, view: ViewLocation): ViewHistory {
  const current = history.entries[history.index];
  if (current.section === view.section && current.groupId === view.groupId && current.playlistId === view.playlistId) return history;
  return { entries: [...history.entries.slice(0, history.index + 1), view], index: history.index + 1 };
}
