import { useDeferredValue, useMemo, type ReactNode } from "react";
import type { MusicTrack } from "../library/types";
import type { Section } from "./AppSidebar";
import type { useMusicLibrary } from "../hooks/useMusicLibrary";
import type { usePlayback } from "../hooks/usePlayback";
import { TrackList } from "./TrackList";
import { CollectionHeader } from "./CollectionHeader";
import { PlayerIcon } from "./PlayerIcon";

type Props = { overview?: ReactNode; onNavigate?: (section: Section) => void; onAddToPlaylist?: (track: MusicTrack) => void; activeSection: Section; search: string; folderCount: number; isSelecting: boolean; library: ReturnType<typeof useMusicLibrary>; isLoading: boolean; loadError: string; onReload: () => Promise<void>; playback: ReturnType<typeof usePlayback> };
export function MainContent({ overview, onNavigate, onAddToPlaylist, activeSection, search, folderCount, isSelecting, library, isLoading, loadError, onReload, playback }: Props) {
  const liked = activeSection === "Liked Songs";
  const home = activeSection === "Home";
  const collection = useMemo(()=>library.tracks.filter(track=>!liked || track.favorite),[library.tracks,liked]);
  const query = useDeferredValue(search.trim().toLocaleLowerCase());
  const visibleTracks = useMemo(()=>{
    const matches=collection.filter(track=>`${track.title} ${track.artist} ${track.album} ${track.albumArtist??""}`.toLocaleLowerCase().includes(query));
    return home?matches.filter(track=>track.available!==false).slice(0,12):matches;
  },[collection,query,home]);
  const playFirst = ()=>{ const first=collection.find(track=>track.available); if(first) void playback.playTrack(first.id,{liked,trackIds:collection.map(track=>track.id)}); };
  return <main id="main-content" className="main-content" tabIndex={-1}><div className="home-content">
    {liked?<CollectionHeader kind="Liked Songs" title="Liked Songs" subtitle="Your saved favorites" icon="heart" tracks={collection} onPlay={playFirst} disabled={!playback.isReady || playback.isBusy} />:<header className="page-heading"><span className="eyebrow">{home?"Made for your hours":"Your local collection"}</span><h1>{home?"Your music. Your hours.":"Library"}</h1><p>{library.tracks.length} songs / {folderCount} music folders</p></header>}
    {overview}
    <section className="library-scan" aria-labelledby="library-heading">
      <div className="section-heading"><h2 id="library-heading">{liked?"Songs":home?"Ready to play":"All songs"}</h2>{home && onNavigate && <button type="button" className="text-button" onClick={()=>onNavigate("Library")}>Show all</button>}<span className="collection-count">{visibleTracks.length} {home?"available":"songs"}</span></div>
      {!liked && <div className="library-tools-bar"><button type="button" disabled={!folderCount || isSelecting || library.isScanning || library.isSavingFavorite || isLoading || !!loadError} onClick={()=>void library.scanMusic()}><PlayerIcon name="folder" />{library.isScanning?"Scanning...":library.hasScanned?"Scan again":"Scan Music"}</button>{library.isScanning && <button type="button" disabled={library.isCancelling} onClick={()=>void library.cancelScan()}>{library.isCancelling?"Cancelling...":"Cancel scan"}</button>}{library.summary && !library.isScanning && <details className="scan-details"><summary>Scan details</summary><p>{library.summary}</p></details>}<span>{library.foldersChanged?"Folders changed. Scan to update.":"Music on this device"}</span></div>}
      <div className="scan-status" role="status">{library.isScanning && <p>{library.progress.discovered} files discovered / {library.progress.skipped} skipped</p>}{isLoading && <p>Loading your saved library...</p>}</div>
      {library.error && <p className="scan-error" role="alert">{library.error}</p>}{loadError && <p className="scan-error" role="alert">{loadError} <button disabled={isLoading} onClick={()=>void onReload()}>Retry loading</button></p>}
      {liked && !collection.length && !isLoading && !loadError?<div className="library-empty"><PlayerIcon name="heart" /><div><h2>Keep your favorites close</h2><p>Like a song with the heart button. Find it here, every time.</p></div></div>:collection.length>0?<>
        {(home && !visibleTracks.length)?<div className="library-empty"><PlayerIcon name="folder" /><div><h2>Your music is saved</h2><p>Reconnect your music folders to listen. Open Library to see unavailable files.</p></div></div>:<TrackList playbackStatus={playback.status} tracks={visibleTracks} onQueue={playback.enqueue} onToggleFavorite={library.toggleFavorite} favoritesDisabled={library.isScanning || library.isSavingFavorite || isLoading} key={`${activeSection}:${query}`} onPlay={id=>playback.playTrack(id,{liked,trackIds:collection.map(track=>track.id)})} currentTrackId={playback.track?.id??null} isBusy={playback.isBusy || !playback.isReady} renderActions={onAddToPlaylist?track=><button type="button" className="track-playlist-action" onClick={event=>{event.stopPropagation();onAddToPlaylist(track);}}>Add to playlist</button>:undefined} />}
      </>:!isLoading && !loadError && <div className="library-empty"><PlayerIcon name="music" /><div><h2>{library.isScanning?"Getting to know your collection":library.hasScanned?"No readable music found":"A library that's entirely yours"}</h2><p>{library.hasScanned?"Check your folders or add another music folder and scan again.":folderCount?"Your music folder is ready. Scan Music to bring your collection in.":"Add a music folder in Your Library, then choose Scan Music."}</p><span>MP3, FLAC, WAV, M4A, AAC, OGG and OPUS</span></div></div>}
    </section>
  </div></main>;
}
