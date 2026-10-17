import { LibraryFolders } from "./LibraryFolders";
import type { useMusicFolders } from "../hooks/useMusicFolders";

export type Section = "Home" | "Liked Songs";

type AppSidebarProps = { onSectionChange: (section: Section) => void; activeSection: Section; musicFolders: ReturnType<typeof useMusicFolders> };

export function AppSidebar({ activeSection, musicFolders, onSectionChange }: AppSidebarProps) {
  return (
    <aside className="sidebar" aria-label="Application sidebar">
      <div className="brand">afterhours<span>YOUR MUSIC. YOUR HOURS.</span></div>
      <nav aria-label="Main navigation" className="sidebar-nav">
        <span className="eyebrow">Collection</span>
        {(["Home", "Liked Songs"] as const).map((section) => <button key={section} type="button" className={`nav-item${activeSection === section ? " is-active" : ""}`} aria-current={activeSection === section ? "page" : undefined} onClick={() => onSectionChange(section)}>
          {section}
        </button>)}
        <button type="button" className="nav-item" disabled title="Available in a future update">Albums <span className="soon">Soon</span></button>
        <button type="button" className="nav-item" disabled title="Available in a future update">Artists <span className="soon">Soon</span></button>
      </nav>
      <section className="playlists" aria-labelledby="playlists-heading">
        <h2 id="playlists-heading" className="eyebrow">Playlists</h2>
        <p className="sidebar-note">A place for every mood.</p>
        <button type="button" className="playlist-create" disabled title="Playlist creation is not available yet">+ Create playlist</button>
      </section>
      <LibraryFolders {...musicFolders} />
    </aside>
  );
}
