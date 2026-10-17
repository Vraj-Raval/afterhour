import { useMemo, useState } from "react";
import type { Playlist } from "../library/playlists";
import type { MusicTrack, ArtworkRef } from "../library/types";
import type { LibraryGroup } from "../library/navigation";
import { normalizeMetadata } from "../library/navigation";
import { LibraryFolders } from "./LibraryFolders";
import { AlbumArtwork } from "./AlbumArtwork";
import { PlayerIcon, type PlayerIconName } from "./PlayerIcon";
import type { useMusicFolders } from "../hooks/useMusicFolders";

export type Section = "Home" | "Library" | "Albums" | "Artists" | "Liked Songs" | "Playlists" | "Queue";
type Props = { playlists: Playlist[]; tracks: MusicTrack[]; albums: LibraryGroup[]; artists: LibraryGroup[]; selectedPlaylistId: string | null; selectedGroupId: string | null; onOpenPlaylist: (id: string | null) => void; onOpenGroup: (kind: "Albums" | "Artists", id: string) => void; onSectionChange: (section: Section) => void; activeSection: Section; musicFolders: ReturnType<typeof useMusicFolders>; compact: boolean; onToggleCompact: () => void };
type Item = { id: string; name: string; detail: string; kind: Section; artwork: ArtworkRef | null; icon?: PlayerIconName; open: () => void; active: boolean };
export function AppSidebar({ activeSection, musicFolders, onSectionChange, playlists, selectedPlaylistId, onOpenPlaylist, tracks, albums, artists, selectedGroupId, onOpenGroup, compact, onToggleCompact }: Props) {
  const [filter, setFilter] = useState("All");
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState("library");
  const [grid, setGrid] = useState(false);
  const [limit, setLimit] = useState(60);
  const byId = useMemo(() => new Map(tracks.map(track => [track.id,track])), [tracks]);
  const items: Item[] = [
    { id:"library", name:"All songs", detail:`${tracks.length} songs on this device`, kind:"Library", artwork:null, icon:"music", open:()=>onSectionChange("Library"), active:activeSection==="Library" },
    { id:"liked", name:"Liked Songs", detail:`${tracks.filter(track=>track.favorite).length} songs`, kind:"Liked Songs", artwork:null, icon:"heart", open:()=>onSectionChange("Liked Songs"), active:activeSection==="Liked Songs" },
    ...playlists.map(playlist=>({ id:playlist.id, name:playlist.name, detail:`Playlist / ${playlist.trackIds.length} songs`, kind:"Playlists" as const, artwork:playlist.trackIds.map(id=>byId.get(id)?.artwork).find(Boolean) ?? null, icon:"list" as const, open:()=>onOpenPlaylist(playlist.id), active:activeSection==="Playlists" && selectedPlaylistId===playlist.id })),
    ...albums.map(group=>({ id:group.id, name:group.name, detail:`Album / ${group.subtitle}`, kind:"Albums" as const, artwork:group.artwork, open:()=>onOpenGroup("Albums",group.id), active:activeSection==="Albums" && selectedGroupId===group.id })),
    ...artists.map(group=>({ id:group.id, name:group.name, detail:"Artist", kind:"Artists" as const, artwork:null, icon:"artist" as const, open:()=>onOpenGroup("Artists",group.id), active:activeSection==="Artists" && selectedGroupId===group.id }))
  ];
  const visible = items.filter(item=>(filter==="All" || item.kind===filter) && normalizeMetadata(`${item.name} ${item.detail}`).includes(normalizeMetadata(query)));
  if(sort==="alphabetical") visible.sort((a,b)=>a.name.localeCompare(b.name));
  return <aside className={`sidebar${compact?" sidebar-compact":""}`} aria-label="Your Library">
    <div className="panel-heading"><button type="button" className="library-heading" title={compact?"Expand Your Library":"Collapse Your Library"} aria-label={compact?"Expand Your Library":"Collapse Your Library"} aria-expanded={!compact} onClick={onToggleCompact}><PlayerIcon name="library" /><h2>Your Library</h2></button><button type="button" className="icon-button create-playlist" title="Create playlist" aria-label="Create playlist" onClick={()=>onOpenPlaylist(null)}><PlayerIcon name="plus" /></button></div>
    <div className="sidebar-expanded"><div className="library-filters" aria-label="Filter Your Library">{["All","Playlists","Artists","Albums","Liked Songs"].map(value=><button type="button" key={value} aria-pressed={filter===value} onClick={()=>{setFilter(value);setLimit(60);if(value!=="All")onSectionChange(value as Section);}}>{value}</button>)}</div>
      <div className="library-tools"><label className="library-search"><PlayerIcon name="search" /><input type="search" aria-label="Search Your Library collections" placeholder="Search Your Library" value={query} onChange={event=>{setQuery(event.currentTarget.value);setLimit(60);}} /></label><select aria-label="Sort Your Library" value={sort} onChange={event=>{setSort(event.currentTarget.value);setLimit(60);}}><option value="library">Library order</option><option value="alphabetical">A - Z</option></select><button type="button" className="icon-button" aria-label={grid?"Show library as list":"Show library as grid"} title={grid?"List view":"Grid view"} aria-pressed={grid} onClick={()=>setGrid(!grid)}><PlayerIcon name={grid?"list":"grid"} /></button></div>
      <nav className="library-browse-links" aria-label="Collection navigation">{(["Home","Library","Queue"] as const).map(section=><button type="button" key={section} aria-current={activeSection===section?"page":undefined} onClick={()=>onSectionChange(section)}>{section}</button>)}</nav>
    </div>
    <div className="sidebar-scroll"><nav className={`library-items${grid?" library-grid":""}`} aria-label="Saved collections">{visible.slice(0,limit).map(item=><button type="button" key={`${item.kind}:${item.id}`} className={`library-item${item.active?" is-active":""}${item.kind==="Liked Songs"?" liked-item":""}`} aria-current={item.active?"page":undefined} title={`${item.name} / ${item.detail}`} onClick={item.open}>
      <span className="library-item-art">{item.artwork?<AlbumArtwork artwork={item.artwork} compact />:<PlayerIcon name={item.icon??"album"} />}</span><span className="library-item-copy"><strong>{item.name}</strong><span>{item.detail}</span></span>
    </button>)}</nav>{!visible.length && <p className="sidebar-empty">No collections found. Try another filter or add music.</p>}{visible.length>limit && <button type="button" className="library-load-more" onClick={()=>setLimit(limit+60)}>Show more ({visible.length-limit})</button>}
      <div className="sidebar-expanded"><LibraryFolders {...musicFolders} /></div>
    </div>
    {compact && <button type="button" className="icon-button compact-add-folder" aria-label="Add Music Folder" title="Add Music Folder" disabled={musicFolders.isSelecting || musicFolders.isLoading || !!musicFolders.loadError} onClick={()=>void musicFolders.addMusicFolder()}><PlayerIcon name="folder" /></button>}
  </aside>;
}
