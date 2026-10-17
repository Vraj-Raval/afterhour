import type { usePlayback } from "../hooks/usePlayback";
import type { MusicTrack } from "../library/types";
import { AlbumArtwork } from "./AlbumArtwork";
import { FavoriteButton } from "./FavoriteButton";
import { PlayerIcon } from "./PlayerIcon";

export function NowPlayingPanel({ playback, favoriteTrack, onToggleFavorite, favoritesDisabled, onClose, onOpenQueue }: { playback: ReturnType<typeof usePlayback>; favoriteTrack?: MusicTrack; onToggleFavorite: (track: MusicTrack) => Promise<void>; favoritesDisabled: boolean; onClose: () => void; onOpenQueue: () => void }) {
  const track = playback.track;
  const queue = playback.queue ?? [];
  return <aside className="now-playing-panel" aria-label="Now Playing">
    <div className="panel-heading"><h2>Now Playing</h2><button type="button" className="icon-button" aria-label="Hide Now Playing" title="Hide Now Playing" onClick={onClose}><PlayerIcon name="close" /></button></div>
    <div className="now-panel-scroll">{track ? <>
      <div className="now-panel-art"><AlbumArtwork artwork={track.artwork} /></div>
      <div className="now-panel-title"><div><h2 title={track.title}>{track.title}</h2><p title={track.artist}>{track.artist}</p></div>{favoriteTrack && <FavoriteButton track={favoriteTrack} disabled={favoritesDisabled} onToggle={onToggleFavorite} />}</div>
      <section className="track-info"><h3>About this track</h3><dl><div><dt>Album</dt><dd title={track.album}>{track.album}</dd></div><div><dt>Format</dt><dd>{track.format}</dd></div>{track.year && <div><dt>Year</dt><dd>{track.year}</dd></div>}<div><dt>Status</dt><dd className="playback-state">{playback.status}</dd></div></dl></section>
    </> : <div className="now-panel-idle"><div className="idle-record"><PlayerIcon name="album" /></div><h3>Your next listen starts here</h3><p>Choose a song from your library. Its artwork and details will appear here.</p></div>}
      <section className="now-panel-queue"><div className="section-heading"><h3>Next in queue</h3><button type="button" className="text-button" onClick={onOpenQueue}>Open queue</button></div>
        {queue.length ? <ol>{queue.slice(0,4).map(entry => <li key={entry.id}><AlbumArtwork artwork={entry.track.artwork} compact /><div><strong title={entry.track.title}>{entry.track.title}</strong><span title={entry.track.artist}>{entry.track.artist}</span></div></li>)}</ol> : <p>No queued songs. Playback follows your current collection.</p>}
      </section>
    </div>
  </aside>;
}
