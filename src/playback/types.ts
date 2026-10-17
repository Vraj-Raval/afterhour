import type { MusicTrack } from "../library/types";

export type PlaybackSnapshot = {
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
export type PlaybackCollection = { trackIds: string[]; liked: boolean };

export type PlaybackAction =
  | ({ type: "playCollection"; trackId: string } & PlaybackCollection)
  | { type: "setVolume"; volume: number }
  | { type: "setMuted"; muted: boolean }
  | { type: "next" | "previous"; sourceId: number }
  | { type: "seek"; sourceId: number; seconds: number }
  | { type: "play"; trackId: string }
  | { type: "pause" | "resume" | "stop" | "status" };
