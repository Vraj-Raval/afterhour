import { PlayerIcon } from "./PlayerIcon";
import { useMemo } from "react";
import type { usePlayback } from "../hooks/usePlayback";
import type { MusicTrack } from "../library/types";
import { AlbumArtwork } from "./AlbumArtwork";

export function QueueView({ playback, tracks }: { playback: ReturnType<typeof usePlayback>; tracks: readonly MusicTrack[] }) {
  const byId = useMemo(() => new Map(tracks.map(track => [track.id, track])), [tracks]);
  const queue = playback.queue ?? [];
  const busy = playback.isBusy || !playback.isReady;
  return <main id="main-content" className="main-content" tabIndex={-1}><div className="home-content">
    <header className="page-heading"><span className="eyebrow">Your listening order</span><h1>Queue</h1><p>Queued songs play first. Then your collection continues.</p></header>
    {playback.error && <p className="scan-error" role="alert">{playback.error}</p>}
    <section aria-labelledby="queue-current"><div className="section-heading"><h2 id="queue-current">Now playing</h2></div>
      {playback.track ? <div className="queue-current"><AlbumArtwork artwork={playback.track.artwork} compact /><div><strong>{playback.track.title}</strong><p>{playback.track.artist} / {playback.status}</p></div></div> : <p className="collection-note">Nothing playing. Add a song and choose Start queue.</p>}
    </section>
    <section aria-labelledby="queue-heading"><div className="section-heading"><h2 id="queue-heading">Upcoming <span className="eyebrow">{queue.length} queued</span></h2><button type="button" disabled={busy || !queue.length} onClick={() => void playback.clearQueue()}>Clear upcoming</button></div>
      {(!playback.track || playback.status === "stopped") && queue.length > 0 && <button className="primary-play" disabled={busy} onClick={() => void playback.nextTrack()}><PlayerIcon name="play" />Start queue</button>}
      {queue.length ? <ol className="queue-list">{queue.map((entry, index) => {
        const track = byId.get(entry.track.id) ?? entry.track;
        return <li key={entry.id}><AlbumArtwork artwork={track.artwork} compact /><div className="queue-metadata"><strong>{track.title}</strong><p>{track.artist}{!track.available ? " / Unavailable" : ""}</p></div><div className="playlist-row-actions">
          <button type="button" className="icon-button" title="Move up" disabled={busy || index === 0} aria-label={`Move ${track.title} up`} onClick={() => void playback.moveQueued(entry.id, -1)}><PlayerIcon name="up" /></button>
          <button type="button" className="icon-button" title="Move down" disabled={busy || index === queue.length - 1} aria-label={`Move ${track.title} down`} onClick={() => void playback.moveQueued(entry.id, 1)}><PlayerIcon name="down" /></button>
          <button type="button" className="icon-button" title="Remove from queue" disabled={busy} aria-label={`Remove ${track.title} from queue`} onClick={() => void playback.removeQueued(entry.id)}><PlayerIcon name="close" /></button>
        </div></li>;
      })}</ol> : <div className="library-empty"><div><h2>Room for your next song.</h2><p>Use Play next or + Queue beside a song. When Up Next is empty, playback follows the collection you started.</p></div></div>}
      <p className="collection-note">Up Next lasts for this session. Clearing it leaves the current song playing. An unavailable queued song stays here so you can remove it or reconnect its folder.</p>
    </section>
  </div></main>;
}
