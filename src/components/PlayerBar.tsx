import { FavoriteButton } from "./FavoriteButton";
import type { MusicTrack } from "../library/types";
import { PlayerIcon } from "./PlayerIcon";
import { AlbumArtwork } from "./AlbumArtwork";
import { PlaybackVolume } from "./PlaybackVolume";
import { PlaybackProgress } from "./PlaybackProgress";
import type { usePlayback } from "../hooks/usePlayback";

export function PlayerBar({ playback, favoriteTrack, onToggleFavorite, favoritesDisabled }: { playback: ReturnType<typeof usePlayback>; favoriteTrack?: MusicTrack; onToggleFavorite: (track: MusicTrack) => Promise<void>; favoritesDisabled: boolean }) {
  const track = playback.track;
  const playing = playback.isReady && playback.status === "playing";
  const disabled = !track || !playback.isReady || playback.isBusy;
  const statusLabel = !playback.isReady && track ? "Playback status unavailable" : playback.isBusy ? "Updating playback..." : track
    ? playing ? "Playing" : playback.status === "paused" ? "Paused" : "Stopped"
    : "Choose a library track to listen.";

  return (
    <footer className="player-bar" aria-label="Music player" aria-describedby="player-state-note">
      <div className="now-playing">
        {track ? <AlbumArtwork artwork={track.artwork} compact /> : <div className="player-art-placeholder" aria-hidden="true"><PlayerIcon name="volume" /></div>}
        <div className="player-metadata">
          <strong className="truncate" title={track?.title}>{track?.title ?? "Nothing playing"}</strong>
          <p className="truncate" title={track?.artist}>{track?.artist ?? "No track loaded"}</p>
        </div>
        {favoriteTrack && <FavoriteButton track={favoriteTrack} onToggle={onToggleFavorite} disabled={favoritesDisabled} />}
      </div>
      <div className="player-transport">
        <div className="transport-buttons" role="group" aria-label="Playback controls">
          <button type="button" className="icon-button" aria-label="Shuffle" title="Shuffle is not available yet" disabled><PlayerIcon name="shuffle" /></button>
          <button type="button" className="icon-button" aria-label="Previous track" title="Previous track, or restart after three seconds" disabled={disabled} onClick={() => void playback.previousTrack()}><PlayerIcon name="previous" /></button>
          <button type="button" className="play-button" aria-label={playing ? "Pause" : "Play"} title={!track ? "Select a library track first" : playing ? "Pause playback" : "Play or resume this track"} disabled={disabled} aria-busy={playback.isBusy} onClick={() => void playback.togglePlayback()}><PlayerIcon name={playing ? "pause" : "play"} /></button>
          <button type="button" className="icon-button" aria-label="Next track" title="Next track" disabled={disabled} onClick={() => void playback.nextTrack()}><PlayerIcon name="next" /></button>
          <button type="button" className="icon-button" aria-label="Repeat" title="Repeat is not available yet" disabled><PlayerIcon name="repeat" /></button>
        </div>
        <p id="player-state-note" className="player-idle-status" role="status">{statusLabel}</p>
        <PlaybackProgress key={playback.sourceId} playback={playback} />
      </div>
      <div className="volume-control" role="group" aria-label="Stop and Afterhours volume">
        <button type="button" className="icon-button" aria-label="Stop playback" title="Stop playback and release the current file" disabled={disabled || playback.status === "stopped"} onClick={() => void playback.stopPlayback()}><PlayerIcon name="stop" /></button>
        <PlaybackVolume playback={playback} />
      </div>
      {playback.preferencesError && <p className="playback-error" role="status">{playback.preferencesError}</p>}
      {playback.error && <p className="playback-error" role="alert">{playback.error}</p>}
    </footer>
  );
}
