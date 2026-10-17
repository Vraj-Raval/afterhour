import type { LibraryGroup } from "../library/navigation";
import type { MusicTrack } from "../library/types";
import type { Playlist } from "../library/playlists";
import type { Section } from "./AppSidebar";
import { GroupCard } from "./LibraryBrowse";
import { AlbumArtwork } from "./AlbumArtwork";
import { PlayerIcon } from "./PlayerIcon";

export function HomeOverview({ tracks, albums, artists, playlistCount, playlists = [], onNavigate, onOpenAlbum, onOpenPlaylist }: { tracks: readonly MusicTrack[]; albums: LibraryGroup[]; artists: LibraryGroup[]; playlistCount: number; playlists?: Playlist[]; onNavigate: (section: Section) => void; onOpenAlbum: (id: string) => void; onOpenPlaylist?: (id: string) => void }) {
  const byId = new Map(tracks.map(track=>[track.id,track]));
  return <section className="home-overview" aria-label="Your collection overview">
    <div className="home-shortcuts"><button type="button" className="home-shortcut" onClick={()=>onNavigate("Liked Songs")}><span className="shortcut-art liked-cover"><PlayerIcon name="heart" /></span><span><strong>Liked Songs</strong><small>{tracks.filter(track=>track.favorite).length} songs</small></span></button><button type="button" className="home-shortcut" onClick={()=>onNavigate("Library")}><span className="shortcut-art"><PlayerIcon name="library" /></span><span><strong>All your music</strong><small>{tracks.length} songs</small></span></button>{playlists.slice(0,4).map(playlist=><button type="button" className="home-shortcut" key={playlist.id} onClick={()=>onOpenPlaylist?.(playlist.id)}><AlbumArtwork artwork={playlist.trackIds.map(id=>byId.get(id)?.artwork).find(Boolean)??null} compact /><span><strong>{playlist.name}</strong><small>Playlist / {playlist.trackIds.length} songs</small></span></button>)}</div>
    <nav className="browse-tabs" aria-label="Browse your collection">{([ ["Albums",albums.length], ["Artists",artists.length], ["Playlists",playlistCount] ] as const).map(([section,count])=><button type="button" key={section} onClick={()=>onNavigate(section)}>{section}<span>{count}</span></button>)}</nav>
    {albums.length>0 && <><div className="section-heading"><h2>Albums in your library</h2><button type="button" className="text-button" onClick={()=>onNavigate("Albums")}>Show all</button></div><div className="group-grid home-albums">{albums.slice(0,6).map(group=><GroupCard key={group.id} group={group} onOpen={onOpenAlbum} />)}</div></>}
  </section>;
}
