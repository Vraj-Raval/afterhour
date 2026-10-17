import { FavoriteButton } from "./FavoriteButton";
import { memo, useState } from "react";
import type { MusicTrack } from "../library/types";
import { AlbumArtwork } from "./AlbumArtwork";

const PAGE_SIZE = 50;
function formatDuration(seconds: number | null) {
  if (seconds === null) return "—";
  return `${Math.floor(seconds / 60)}:${String(Math.floor(seconds) % 60).padStart(2, "0")}`;
}

type TrackListProps = { onToggleFavorite: (track: MusicTrack) => Promise<void>; favoritesDisabled: boolean; tracks: readonly MusicTrack[]; onPlay: (id: string) => Promise<void>; currentTrackId: string | null; isBusy: boolean };

export const TrackList = memo(function TrackList({ tracks, onPlay, currentTrackId, isBusy, onToggleFavorite, favoritesDisabled }: TrackListProps) {
  const [page, setPage] = useState(0);
  const pageCount = Math.ceil(tracks.length / PAGE_SIZE);
  const currentPage = Math.min(page, Math.max(0, pageCount - 1));
  const visibleTracks = tracks.slice(currentPage * PAGE_SIZE, (currentPage + 1) * PAGE_SIZE);

  if (!tracks.length) return <p className="empty-results">No tracks match your search. Try another title or artist.</p>;

  return <>
    <div className="track-table-wrap" tabIndex={0} role="region" aria-label="Library tracks, scroll horizontally for all columns">
      <table className="track-table">
        <caption className="sr-only">Scanned local music. Activate a track title to play it.</caption>
        <colgroup><col className="number-column" /><col className="title-column" /><col className="artist-column" /><col className="album-column" /><col className="favorite-column" /><col className="duration-column" /></colgroup>
        <thead><tr><th scope="col"><span aria-hidden="true">#</span><span className="sr-only">Track number</span></th><th scope="col">Title</th><th scope="col">Artist</th><th scope="col">Album</th><th scope="col"><span className="sr-only">Favorite</span></th><th scope="col" className="track-duration">Duration</th></tr></thead>
        <tbody>{visibleTracks.map((track) => <tr key={track.id} className={currentTrackId === track.id ? "is-selected" : undefined} onClick={() => { if (track.available && !isBusy) void onPlay(track.id); }}>
          <td className="track-number">{track.trackNumber ?? "—"}</td>
          <th scope="row"><div className="scanned-track-title"><AlbumArtwork artwork={track.artwork} compact /><div className="scanned-track-metadata">
            <button type="button" className="track-select" aria-current={currentTrackId === track.id ? "true" : undefined} disabled={!track.available || isBusy} aria-label={`Play ${track.title} by ${track.artist}`} title={`${track.title}\n${track.path}`} onClick={(event) => { event.stopPropagation(); void onPlay(track.id); }}><span className="truncate">{track.title}</span></button>
            <span className={`track-file-detail truncate${track.available ? "" : " track-unavailable"}`} title={track.path}>{[!track.available ? "Unavailable" : null, track.format, track.year, track.discNumber ? `Disc ${track.discNumber}` : null].filter(Boolean).join(" / ")}</span>
          </div></div></th>
          <td><span className="truncate" title={track.artist}>{track.artist}</span></td>
          <td><span className="truncate" title={`${track.album}${track.albumArtist ? ` / Album artist: ${track.albumArtist}` : ""}`}>{track.album}</span></td>
          <td><FavoriteButton track={track} disabled={favoritesDisabled} onToggle={onToggleFavorite} /></td>
          <td className="track-duration">{formatDuration(track.durationSeconds)}</td>
        </tr>)}</tbody>
      </table>
    </div>
    <div className="track-pagination" aria-label="Track pages">
      <button type="button" disabled={currentPage === 0} onClick={() => setPage(currentPage - 1)}>Previous</button>
      <span role="status">Page {currentPage + 1} of {pageCount}</span>
      <button type="button" disabled={currentPage + 1 >= pageCount} onClick={() => setPage(currentPage + 1)}>Next</button>
    </div>
  </>;
});
