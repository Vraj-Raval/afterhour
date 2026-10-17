import type { useMusicLibrary } from "../hooks/useMusicLibrary";
import type { usePlayback } from "../hooks/usePlayback";
import type { MusicTrack } from "../library/types";
import { TrackList } from "./TrackList";

export type CollectionTrackProps = { library: ReturnType<typeof useMusicLibrary>; playback: ReturnType<typeof usePlayback>; onAddToPlaylist: (track: MusicTrack) => void; isLoading: boolean };
export function CollectionTracks({ tracks, library, playback, onAddToPlaylist, isLoading }: CollectionTrackProps & { tracks: MusicTrack[] }) {
  return <TrackList playbackStatus={playback.status} tracks={tracks} onQueue={playback.enqueue} currentTrackId={playback.track?.id ?? null} isBusy={!playback.isReady || playback.isBusy} onToggleFavorite={library.toggleFavorite} favoritesDisabled={isLoading || library.isScanning || library.isSavingFavorite || library.isRestoring}
    onPlay={id => playback.playTrack(id, { liked: false, selection: true, trackIds: tracks.map(track => track.id) })}
    renderActions={track => <button type="button" className="track-playlist-action" aria-label={`Add ${track.title} to playlist`} onClick={event => { event.stopPropagation(); onAddToPlaylist(track); }}>+ Playlist</button>} />;
}
