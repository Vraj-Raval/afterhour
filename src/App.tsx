import { useState } from "react";
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
  const musicFolders = useMusicFolders();
  const library = useMusicLibrary(musicFolders.folders, musicFolders.snapshot, musicFolders.updateFolders);
  const playback = usePlayback();
  const [activeSection, setActiveSection] = useState<Section>("Home");

  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">Skip to main content</a>
      <AppSidebar onSectionChange={setActiveSection} activeSection={activeSection} musicFolders={musicFolders} />
      <div className="workspace">
        <TopBar activeSection={activeSection} search={search} onSearchChange={setSearch} />
        <MainContent activeSection={activeSection} search={search} folderCount={musicFolders.folders.length} library={library} isSelecting={musicFolders.isSelecting} isLoading={musicFolders.isLoading || library.isRestoring} loadError={musicFolders.loadError} onReload={musicFolders.restoreLibrary} playback={playback} />
      </div>
      <PlayerBar playback={playback} favoriteTrack={library.tracks.find((track) => track.id === playback.track?.id)} onToggleFavorite={library.toggleFavorite} favoritesDisabled={library.isScanning || library.isSavingFavorite || musicFolders.isLoading || library.isRestoring} />
    </div>
  );
}

export default App;
