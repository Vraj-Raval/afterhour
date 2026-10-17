import { useDeferredValue, useMemo } from "react";
import { GroupCard } from "./LibraryBrowse";
import { normalizeMetadata, searchTracks, type LibraryGroup } from "../library/navigation";
import { CollectionTracks, type CollectionTrackProps } from "./CollectionTracks";

export function SearchView({ query, onClear, albums = [], artists = [], onOpenGroup, ...props }: CollectionTrackProps & { query: string; onClear: () => void; albums?: LibraryGroup[]; artists?: LibraryGroup[]; onOpenGroup?: (kind:"Albums"|"Artists",id:string)=>void }) {
  const deferred = useDeferredValue(query);
  const results = useMemo(() => searchTracks(props.library.tracks, deferred), [props.library.tracks, deferred]);
  const terms = normalizeMetadata(deferred).split(" ").filter(Boolean);
  const matchingGroups = (groups:LibraryGroup[])=>groups.filter(group=>terms.every(term=>normalizeMetadata(`${group.name} ${group.subtitle}`).includes(term)));
  const matchingArtists = matchingGroups(artists); const matchingAlbums = matchingGroups(albums);
  const updating = normalizeMetadata(query) !== normalizeMetadata(deferred);
  return <main id="main-content" className="main-content" tabIndex={-1}><div className="home-content">
    <header className="page-heading"><span className="eyebrow">Your local collection</span><h1>Search results</h1><p title={query}>Results for &quot;{query.trim()}&quot;</p></header>
    <div className="section-heading"><h2>Songs</h2><button type="button" onClick={onClear}>Clear search</button></div>
    <p className="collection-note" role="status">{props.isLoading ? "Loading your library..." : updating ? "Updating results..." : `${results.length} matching songs`}</p>
    {props.library.error && <p className="scan-error" role="alert">{props.library.error}</p>}
    <div aria-busy={updating} inert={updating || undefined}>
      {results.length ? <CollectionTracks key={normalizeMetadata(deferred)} {...props} tracks={results} /> : !props.isLoading && <div className="library-empty"><div><h2>No songs found.</h2><p>Try another title, artist, or album, or clear your search to browse the collection.</p></div></div>}
    </div>
    {onOpenGroup && <>{matchingArtists.length>0 && <section><div className="section-heading"><h2>Artists</h2><span className="collection-count">{matchingArtists.length} found</span></div><div className="group-grid">{matchingArtists.slice(0,6).map(group=><GroupCard artist key={group.id} group={group} onOpen={id=>onOpenGroup("Artists",id)} />)}</div></section>}{matchingAlbums.length>0 && <section><div className="section-heading"><h2>Albums</h2><span className="collection-count">{matchingAlbums.length} found</span></div><div className="group-grid">{matchingAlbums.slice(0,6).map(group=><GroupCard key={group.id} group={group} onOpen={id=>onOpenGroup("Albums",id)} />)}</div></section>}</>}
    <p className="collection-note">Search playback keeps the result order. Songs in Up Next play first.</p>
  </div></main>;
}
