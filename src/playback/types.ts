import type { MusicTrack } from "../library/types";

export type QueueEntry = { id: number; track: MusicTrack };

export type PlaybackSnapshot = {
  queue: QueueEntry[];
  queueRevision: number;
  revision: number;
  volume: number;
  muted: boolean;
  preferencesError: string | null;
  sourceId: number;
  positionSeconds: number;
  durationSeconds: number | null;
  canSeek: boolean;
  status: "stopped" | "playing" | "paused";
  track: MusicTrack | null;
  error: string | null;
};
export type PlaybackCollection = { trackIds: string[]; liked: boolean; playlistId?: string; selection?: boolean };

export type PlaybackAction =
  | { type: "playSelection"; trackId: string; trackIds: string[] }
  | { type: "enqueue"; trackId: string; playNext: boolean }
  | { type: "queueRemove"; entryId: number; queueRevision: number }
  | { type: "queueMove"; entryId: number; direction: -1 | 1; queueRevision: number }
  | { type: "queueClear"; queueRevision: number }
  | { type: "playPlaylist"; trackId: string; playlistId: string; trackIds: string[] }
  | ({ type: "playCollection"; trackId: string } & PlaybackCollection)
  | { type: "setVolume"; volume: number }
  | { type: "setMuted"; muted: boolean }
  | { type: "next" | "previous"; sourceId: number }
  | { type: "seek"; sourceId: number; seconds: number }
  | { type: "play"; trackId: string }
  | { type: "pause" | "resume" | "stop" | "status" };
