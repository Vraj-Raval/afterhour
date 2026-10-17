import { useMemo, useState } from "react";
import { NowPlayingPanel } from "./components/NowPlayingPanel";
import { pushView, type ViewHistory } from "./library/presentation";
import { QueueView } from "./components/QueueView";
import { usePlaylists } from "./hooks/usePlaylists";
import { PlaylistView, PlaylistFeedback } from "./components/PlaylistView";
import { PlaylistPicker } from "./components/PlaylistPicker";
import { SearchView } from "./components/SearchView";
import { LibraryBrowse } from "./components/LibraryBrowse";
import { HomeOverview } from "./components/HomeOverview";
import { groupLibrary, normalizeMetadata } from "./library/navigation";
import type { MusicTrack } from "./library/types";
import { AppSidebar, type Section } from "./components/AppSidebar";
import { TopBar } from "./components/TopBar";
import { MainContent } from "./components/MainContent";
import { PlayerBar } from "./components/PlayerBar";
import { useMusicFolders } from "./hooks/useMusicFolders";
import { useMusicLibrary } from "./hooks/useMusicLibrary";
import { usePlayback } from "./hooks/usePlayback";
import "./App.css";

function App() {
  const [search, setSearch] = useState("");
  const [history, setHistory] = useState<ViewHistory>({ entries: [{ section: "Home", groupId: null, playlistId: null }], index: 0 });
  const { section: activeSection, groupId: selectedGroupId, playlistId: selectedPlaylistId } = history.entries[history.index];
  const [compactLibrary, setCompactLibrary] = useState(false);
  const [nowPlayingOpen, setNowPlayingOpen] = useState(true);
  const [addingTrack, setAddingTrack] = useState<MusicTrack | null>(null);
  const musicFolders = useMusicFolders();
  const library = useMusicLibrary(musicFolders.folders, musicFolders.snapshot, musicFolders.updateFolders);
  const playback = usePlayback();
  const playlists = usePlaylists();
  const albums = useMemo(() => groupLibrary(library.tracks, "Albums"), [library.tracks]);
  const artists = useMemo(() => groupLibrary(library.tracks, "Artists"), [library.tracks]);
  const loading = musicFolders.isLoading || library.isRestoring;
  function navigate(section: Section) { setHistory(current => pushView(current, { section, groupId: null, playlistId: null })); setSearch(""); }
  function openPlaylist(id: string | null) { setHistory(current => pushView(current, { section: "Playlists", playlistId: id, groupId: null })); setSearch(""); }
  function openGroup(kind: "Albums" | "Artists", id: string | null) { setHistory(current => pushView(current, { section: kind, groupId: id, playlistId: null })); setSearch(""); }
  function openAlbum(id: string) { openGroup("Albums",id); }
  function moveHistory(direction: -1 | 1) { if(search.trim()) { setSearch(""); return; } setHistory(current => ({ ...current, index: Math.max(0, Math.min(current.entries.length-1,current.index+direction)) })); }
  const favoriteTrack = library.tracks.find(track=>track.id===playback.track?.id);
  const favoritesDisabled = library.isScanning || library.isSavingFavorite || loading;
  const collectionProps = { library, playback, isLoading: loading, onAddToPlaylist: setAddingTrack };
  return <div className={`app-shell${compactLibrary?" is-library-compact":""}${nowPlayingOpen?" has-now-playing":""}`}>
    <a className="skip-link" href="#main-content">Skip to main content</a>
    <TopBar activeSection={activeSection} search={search} onSearchChange={setSearch} onHome={()=>navigate("Home")} onBack={()=>moveHistory(-1)} onForward={()=>moveHistory(1)} canGoBack={history.index>0 || !!search.trim()} canGoForward={history.index<history.entries.length-1} onOpenQueue={()=>navigate("Queue")} nowPlayingOpen={nowPlayingOpen} onToggleNowPlaying={()=>setNowPlayingOpen(!nowPlayingOpen)} />
    <AppSidebar compact={compactLibrary} onToggleCompact={()=>setCompactLibrary(!compactLibrary)} tracks={library.tracks} albums={albums} artists={artists} selectedGroupId={selectedGroupId} onOpenGroup={openGroup} playlists={playlists.playlists} selectedPlaylistId={selectedPlaylistId} onOpenPlaylist={openPlaylist} onSectionChange={navigate} activeSection={activeSection} musicFolders={musicFolders} />
    <div className="workspace">
      <div className="playlist-global-feedback"><PlaylistFeedback manager={playlists} />{musicFolders.loadError && activeSection !== "Playlists" && <p className="scan-error" role="alert">{musicFolders.loadError} <button disabled={loading} onClick={() => void musicFolders.restoreLibrary()}>Retry loading library</button></p>}</div>
      {normalizeMetadata(search) ? <SearchView {...collectionProps} query={search} onClear={() => setSearch("")} albums={albums} artists={artists} onOpenGroup={openGroup} />
        : activeSection === "Queue" ? <QueueView playback={playback} tracks={library.tracks} />
        : activeSection === "Playlists" ? <PlaylistView {...collectionProps} loadError={musicFolders.loadError} onReload={musicFolders.restoreLibrary} manager={playlists} selectedId={selectedPlaylistId} onOpen={openPlaylist} search="" />
        : activeSection === "Albums" || activeSection === "Artists" ? <LibraryBrowse onOpenAlbum={openAlbum} key={activeSection} {...collectionProps} kind={activeSection} groups={activeSection === "Albums" ? albums : artists} selectedId={selectedGroupId} onOpen={id=>openGroup(activeSection,id)} />
        : <MainContent {...collectionProps} onNavigate={navigate} overview={activeSection === "Home" ? <HomeOverview tracks={library.tracks} albums={albums} artists={artists} playlistCount={playlists.playlists.length} playlists={playlists.playlists} onOpenPlaylist={openPlaylist} onNavigate={navigate} onOpenAlbum={openAlbum} /> : undefined} activeSection={activeSection} search="" folderCount={musicFolders.folders.length} isSelecting={musicFolders.isSelecting} loadError={musicFolders.loadError} onReload={musicFolders.restoreLibrary} />}
    </div>
    {nowPlayingOpen && <NowPlayingPanel playback={playback} favoriteTrack={favoriteTrack} onToggleFavorite={library.toggleFavorite} favoritesDisabled={favoritesDisabled} onClose={()=>setNowPlayingOpen(false)} onOpenQueue={()=>navigate("Queue")} />}
    {addingTrack && <PlaylistPicker key={addingTrack.id} track={addingTrack} manager={playlists} onClose={() => setAddingTrack(null)} />}
    <PlayerBar onOpenQueue={() => navigate("Queue")} playback={playback} favoriteTrack={favoriteTrack} onToggleFavorite={library.toggleFavorite} favoritesDisabled={favoritesDisabled} />
  </div>;
}
export default App;
