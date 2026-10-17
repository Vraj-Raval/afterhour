import { useState } from "react";
import type { usePlaylists } from "../hooks/usePlaylists";
import type { useMusicLibrary } from "../hooks/useMusicLibrary";
import type { usePlayback } from "../hooks/usePlayback";
import type { Playlist } from "../library/playlists";
import type { MusicTrack } from "../library/types";
import { CollectionHeader } from "./CollectionHeader";
import { AlbumArtwork } from "./AlbumArtwork";
import { PlayerIcon } from "./PlayerIcon";
import { TrackList } from "./TrackList";

type Manager = ReturnType<typeof usePlaylists>;
export function PlaylistFeedback({ manager }: { manager: Manager }) {
  return <>{manager.error && <p className="scan-error" role="alert">{manager.error}</p>}{manager.message && <p className="collection-note" role="status">{manager.message}</p>}</>;
}
function PlaylistManagement({ playlist, manager, onDeleted }: { playlist: Playlist; manager: Manager; onDeleted: () => void }) {
  const [name, setName] = useState(playlist.name);
  const [confirming, setConfirming] = useState(false);
  return <details className="playlist-management"><summary><PlayerIcon name="more" />Edit playlist</summary><div>
    <form className="playlist-form" onSubmit={async (event) => { event.preventDefault(); await manager.rename(playlist.id, name); }}>
      <label>Playlist name<input value={name} onChange={event => setName(event.currentTarget.value)} required /></label>
      <button disabled={manager.isBusy || !name.trim() || name.trim() === playlist.name}>Save name</button>
    </form>
    {confirming ? <div role="group" aria-label="Confirm playlist deletion"><p>Delete "{playlist.name}"? Songs will remain in your library and Liked Songs.</p><button disabled={manager.isBusy} onClick={async () => { if (await manager.deletePlaylist(playlist.id)) onDeleted(); }}>Delete playlist</button><button disabled={manager.isBusy} onClick={() => setConfirming(false)}>Cancel</button></div>
      : <button disabled={manager.isBusy} onClick={() => setConfirming(true)}>Delete playlist...</button>}
  </div></details>;
}
export function PlaylistView({ manager, selectedId, onOpen, library, playback, search, onAddToPlaylist, isLoading, loadError, onReload }: {
  isLoading: boolean; loadError: string; onReload: () => Promise<void>;
  manager: Manager; selectedId: string | null; onOpen: (id: string | null) => void;
  library: ReturnType<typeof useMusicLibrary>; playback: ReturnType<typeof usePlayback>; search: string; onAddToPlaylist: (track: MusicTrack) => void;
}) {
  const [name, setName] = useState("");
  const playlist = manager.playlists.find(p => p.id === selectedId);
  const byId = new Map(library.tracks.map(track => [track.id, track]));
  const tracks = (playlist?.trackIds ?? []).flatMap(id => { const track = byId.get(id); return track ? [track] : []; });
  const query = search.trim().toLocaleLowerCase();
  const visible = tracks.filter(track => `${track.title} ${track.artist} ${track.album}`.toLocaleLowerCase().includes(query));
  return <main id="main-content" className="main-content" tabIndex={-1}><div className="home-content">
    {playlist?<CollectionHeader kind="Playlist" title={playlist.name} subtitle="Your Library" artwork={tracks.find(track=>track.artwork)?.artwork??null} tracks={tracks} onPlay={()=>{const first=tracks.find(track=>track.available);if(first)void playback.playTrack(first.id,{liked:false,playlistId:playlist.id,trackIds:playlist.trackIds});}} disabled={!playback.isReady||playback.isBusy} />:<header className="page-heading"><span className="eyebrow">Your Library</span><h1>Playlists</h1><p>Your songs, in your own order.</p></header>}
    <PlaylistFeedback manager={manager} />
    {library.error && <p className="scan-error" role="alert">{library.error}</p>}
    {loadError && <p className="scan-error" role="alert">{loadError} <button disabled={isLoading} onClick={() => void onReload()}>Retry loading library</button></p>}
    {isLoading && <p role="status">Loading your saved library...</p>}
    {!manager.isLoaded && !manager.error && <p role="status">Loading playlists...</p>}
    {manager.error && <button disabled={manager.isBusy} onClick={() => void manager.reload()}>Reload playlists</button>}
    {playlist ? <>
      <button className="text-button" onClick={() => onOpen(null)}>All playlists</button>
      <PlaylistManagement key={playlist.id} playlist={playlist} manager={manager} onDeleted={() => onOpen(null)} />
      <div className="section-heading"><h2>{tracks.length} songs</h2><span className="eyebrow">In the order you added them</span></div>
      {tracks.length ? <TrackList playbackStatus={playback.status} onQueue={playback.enqueue} key={`${playlist.id}:${query}`} tracks={visible} onPlay={id => playback.playTrack(id, { liked: false, playlistId: playlist.id, trackIds: playlist.trackIds })} currentTrackId={playback.track?.id ?? null} isBusy={playback.isBusy || !playback.isReady} onToggleFavorite={library.toggleFavorite} favoritesDisabled={library.isScanning || library.isSavingFavorite || library.isRestoring || isLoading}
        renderActions={track => <div className="playlist-row-actions"><button className="track-playlist-action" onClick={event => { event.stopPropagation(); onAddToPlaylist(track); }}>+ Playlist</button><button className="track-playlist-action" aria-label={`Remove ${track.title} from ${playlist.name}`} disabled={manager.isBusy} onClick={event => { event.stopPropagation(); void manager.remove(playlist.id, track.id); }}>Remove</button></div>} />
        : !isLoading && !loadError && <div className="library-empty"><div><h2>A little room for your favorites.</h2><p>Add songs from your library or Liked Songs using the + Playlist button.</p></div></div>}
    </> : <>
      <form className="playlist-form" onSubmit={async event => { event.preventDefault(); const created = await manager.create(name); if (created) { setName(""); onOpen(created.id); } }}>
        <label>New playlist name<input placeholder="Late night favorites" value={name} onChange={event => setName(event.currentTarget.value)} required /></label><button disabled={manager.isBusy || !manager.isLoaded || !name.trim()}>Create playlist</button>
      </form>
      {manager.isLoaded && !manager.playlists.length ? <div className="library-empty"><div><h2>A place for every mood.</h2><p>Create your first playlist, then add songs from your collection.</p></div></div> : <div className="group-grid playlist-cards">{manager.playlists.map(p => <button className="group-card" key={p.id} onClick={() => onOpen(p.id)}><span className="group-card-art"><AlbumArtwork artwork={p.trackIds.map(id=>byId.get(id)?.artwork).find(Boolean)??null} /></span><strong>{p.name}</strong><span>Playlist / {p.trackIds.length} songs</span></button>)}</div>}
    </>}
  </div></main>;
}
