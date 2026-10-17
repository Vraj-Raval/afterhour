import { PlayerIcon } from "./PlayerIcon";
import type { Section } from "./AppSidebar";
import { useDeferredValue, useMemo } from "react";
import type { useMusicLibrary } from "../hooks/useMusicLibrary";
import { TrackList } from "./TrackList";
import type { usePlayback } from "../hooks/usePlayback";

type MainContentProps = { activeSection: Section; search: string; folderCount: number; isSelecting: boolean; library: ReturnType<typeof useMusicLibrary>; isLoading: boolean; loadError: string; onReload: () => Promise<void>; playback: ReturnType<typeof usePlayback> };

export function MainContent({ activeSection, search, folderCount, isSelecting, library, isLoading, loadError, onReload, playback }: MainContentProps) {
  const liked = activeSection === "Liked Songs";
  const collection = useMemo(() => library.tracks.filter((track) => !liked || track.favorite), [library.tracks, liked]);
  const query = useDeferredValue(search.trim().toLocaleLowerCase());
  const visibleTracks = useMemo(() => collection.filter((track) =>
    `${track.title} ${track.artist} ${track.album} ${track.albumArtist ?? ""}`.toLocaleLowerCase().includes(query)), [collection, query]);

  return (
    <main id="main-content" className="main-content" tabIndex={-1}>
      <div className="home-content">
        <section className="library-welcome" aria-labelledby="home-title">
          <span className="eyebrow">Your listening room</span>
          <h1 id="home-title">{liked ? <>Your favorites.<br />Always within reach.</> : <>Make room<br />for your music.</>}</h1>
          <p>A home for the records you return to.<br />Unhurried, uninterrupted, entirely yours.</p>
        </section>
        <section className="library-scan" aria-labelledby="library-heading">
          <div className="section-heading"><h2 id="library-heading">{liked ? "Liked Songs" : "Your library"}</h2><span className="eyebrow">{collection.length} tracks / {folderCount} folders</span></div>
          <div className="scan-actions">
            <button type="button" disabled={!folderCount || isSelecting || library.isScanning || library.isSavingFavorite || isLoading || !!loadError} onClick={() => void library.scanMusic()}>{library.isScanning ? "Scanning music..." : library.hasScanned ? "Scan Again" : "Scan Music"}</button>
            {loadError && <button type="button" disabled={isLoading} onClick={() => void onReload()}>Retry Loading Library</button>}
            {library.isScanning && <button type="button" disabled={library.isCancelling} onClick={() => void library.cancelScan()}>{library.isCancelling ? "Cancelling..." : "Cancel Scan"}</button>}
          </div>
          <div className="scan-status" role="status">
            {library.isScanning ? <p>{library.isCancelling ? "Stopping after the current file." : "Reading selected folders and audio tags."} {library.progress.discovered} files discovered; {library.progress.skipped} skipped.</p> : library.summary && <p>{library.summary}</p>}
            {isLoading && <p>Loading your saved folders and library...</p>}
            {library.foldersChanged && !library.isScanning && <p>Folders have changed. Scan again to update your library.</p>}
          </div>
          {library.error && <p className="scan-error" role="alert">{library.error}</p>}
          {loadError && <p className="scan-error" role="alert">{loadError}</p>}
          {liked && !collection.length && !isLoading && !loadError ? <div className="library-empty"><div className="empty-library-mark" aria-hidden="true"><PlayerIcon name="heart" /></div><div><h2>Your favorites belong here.</h2><p>Tap the heart beside any song in your library to keep it close. Your Liked Songs are saved on this device.</p></div></div> : collection.length > 0 ? <>
            <p className="collection-note">{visibleTracks.length} matching tracks. Select an available track to listen.</p>
            <TrackList onToggleFavorite={library.toggleFavorite} favoritesDisabled={library.isScanning || library.isSavingFavorite || isLoading} key={`${activeSection}:${query}`} tracks={visibleTracks} onPlay={(id) => playback.playTrack(id, { liked, trackIds: collection.map((track) => track.id) })} currentTrackId={playback.track?.id ?? null} isBusy={playback.isBusy || !playback.isReady} />
          </> : !isLoading && !loadError && <div className="library-empty" aria-busy={library.isScanning}>
            <div className="empty-library-mark" aria-hidden="true"><svg viewBox="0 0 48 48" fill="none" stroke="currentColor" strokeWidth="1.25"><rect x="8" y="8" width="32" height="32" rx="2" /><circle cx="24" cy="24" r="11" /><circle cx="24" cy="24" r="2" /></svg></div>
            <div>
              <h2>{library.isScanning ? "Getting to know your collection." : library.hasScanned ? "No readable music found." : "Your library starts here."}</h2>
              <p>{library.hasScanned
                ? "No supported, readable audio files were found in the last scan. Try another folder or check the scan summary."
                : folderCount ? `${folderCount} music folder${folderCount === 1 ? " is" : "s are"} selected. Click Scan Music to read audio files and metadata.` : "Choose a local music folder using Add Music Folder in the sidebar, then scan your collection."}</p>
              <p className="library-next-step">MP3, FLAC, WAV, M4A, AAC, OGG and OPUS. Your files are read only; your library is saved on this device.</p>
            </div>
          </div>}
        </section>
        <footer className="collection-footer"><span className="eyebrow">No rush. No noise.</span><p>Just you and the music.</p></footer>
      </div>
    </main>
  );
}
