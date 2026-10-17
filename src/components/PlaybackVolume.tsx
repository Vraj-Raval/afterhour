import { useState, type CSSProperties } from "react";
import type { usePlayback } from "../hooks/usePlayback";
import { PlayerIcon } from "./PlayerIcon";

export function PlaybackVolume({ playback }: { playback: ReturnType<typeof usePlayback> }) {
  const [draft, setDraft] = useState<number | null>(null);
  const applied = playback.muted ? 0 : Math.round(playback.volume * 100);
  const requested = playback.isVolumeBusy ? draft ?? applied : applied;
  const disabled = !playback.isReady || playback.isBusy;
  return <>
    <button type="button" className="icon-button" aria-label={playback.muted ? "Unmute" : "Mute"}
      title={playback.muted ? `Unmute to ${Math.round(playback.volume * 100)}%` : "Mute Afterhours"}
      aria-pressed={playback.muted} disabled={disabled || playback.isVolumeBusy}
      onClick={() => { setDraft(null); playback.setMuted(!playback.muted); }}>
      <PlayerIcon name={playback.muted || applied === 0 ? "muted" : "volume"} />
    </button>
    <input className="player-range" style={{ "--range-fill": `${requested}%` } as CSSProperties} type="range" min={0} max={100} step={1}
      value={requested} disabled={disabled} aria-label="Afterhours volume" aria-busy={playback.isVolumeBusy}
      aria-valuetext={playback.isVolumeBusy ? `${requested}% requested, ${applied}% applied` : `${applied}%`}
      onChange={(event) => { const value = Number(event.target.value); setDraft(value); playback.setVolume(value / 100); }} />
    <span className="volume-value" title="Applied Afterhours volume">{applied}%</span>
  </>;
}
