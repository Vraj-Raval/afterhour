import { useState } from "react";
import type { usePlayback } from "../hooks/usePlayback";
import { clampPosition, formatPlaybackTime } from "../playback/time";

export function PlaybackProgress({ playback }: { playback: ReturnType<typeof usePlayback> }) {
  const [draft, setDraft] = useState<number | null>(null);
  const duration = playback.durationSeconds;
  const disabled = !playback.isReady || playback.isBusy || !playback.canSeek || playback.status === "stopped" || !duration;
  const position = clampPosition(playback.positionSeconds, duration);
  function commit() {
    if (draft !== null && !disabled) void playback.seek(clampPosition(draft, duration), playback.sourceId);
    setDraft(null);
  }
  return <div className="timeline">
    <span>{playback.track ? formatPlaybackTime(playback.positionSeconds) : "--:--"}</span>
    <input className="player-range" type="range" min={0} max={duration ?? 1} step="any"
      value={draft ?? position} disabled={disabled} aria-label="Playback position"
      aria-valuetext={`${formatPlaybackTime(draft ?? position)} of ${formatPlaybackTime(duration)}`}
      onPointerDown={(event) => event.currentTarget.setPointerCapture(event.pointerId)}
      onChange={(event) => setDraft(Number(event.target.value))}
      onPointerUp={commit} onKeyUp={commit} onBlur={commit} onPointerCancel={() => setDraft(null)} />
    <span>{formatPlaybackTime(duration)}</span>
  </div>;
}
