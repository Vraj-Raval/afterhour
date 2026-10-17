import { useState } from "react";
import { groupLibrary, type LibraryGroup } from "../library/navigation";
import { AlbumArtwork } from "./AlbumArtwork";
import { PlayerIcon } from "./PlayerIcon";
import { CollectionHeader } from "./CollectionHeader";
import { CollectionTracks, type CollectionTrackProps } from "./CollectionTracks";

export function GroupCard({group,onOpen,artist=false}:{group:LibraryGroup;onOpen:(id:string)=>void;artist?:boolean}){
  return <button type="button" className="group-card" onClick={()=>onOpen(group.id)} title={`${group.name}${group.subtitle?` / ${group.subtitle}`:""}`}><span className="group-card-art">{artist?<span className="artist-placeholder"><PlayerIcon name="artist" /></span>:<AlbumArtwork artwork={group.artwork} />}</span><strong>{group.name}</strong><span>{artist?"Artist":group.subtitle||"Album"}</span><small>{group.tracks.length} songs</small></button>;
}
export function LibraryBrowse({kind,groups,selectedId,onOpen,onOpenAlbum,...props}:CollectionTrackProps&{kind:"Albums"|"Artists";groups:LibraryGroup[];selectedId:string|null;onOpen:(id:string|null)=>void;onOpenAlbum?:(id:string)=>void}){
  const [page,setPage]=useState(0);const selected=groups.find(group=>group.id===selectedId);const pageCount=Math.ceil(groups.length/48);const currentPage=Math.min(page,Math.max(0,pageCount-1));
  function playSelected(){const first=selected?.tracks.find(track=>track.available);if(first&&selected)void props.playback.playTrack(first.id,{liked:false,selection:true,trackIds:selected.tracks.map(track=>track.id)});}
  const artistAlbums=selected&&kind==="Artists"?groupLibrary(selected.tracks,"Albums"):[];
  return <main id="main-content" className="main-content" tabIndex={-1}><div className="home-content">
    {selected?<><button type="button" className="back-to-collection text-button" onClick={()=>onOpen(null)}><PlayerIcon name="back" />All {kind.toLowerCase()}</button><CollectionHeader kind={kind==="Albums"?"Album":"Artist"} title={selected.name} subtitle={selected.subtitle} artwork={kind==="Albums"?selected.artwork:null} icon={kind==="Artists"?"artist":undefined} tracks={selected.tracks} onPlay={playSelected} disabled={!props.playback.isReady||props.playback.isBusy} /></>:<header className="page-heading"><span className="eyebrow">Your Library</span><h1>{kind}</h1><p>{groups.length} {kind.toLowerCase()} from your music folders</p></header>}
    {props.library.error&&<p className="scan-error" role="alert">{props.library.error}</p>}{props.isLoading&&<p className="collection-note" role="status">Loading your saved library...</p>}
    {selected?<><div className="section-heading"><h2>{kind==="Artists"?"Songs":"Track list"}</h2><span className="collection-count">{selected.tracks.length} songs</span></div><CollectionTracks key={selected.id} {...props} tracks={selected.tracks} />{kind==="Artists"&&artistAlbums.length>0&&onOpenAlbum&&<section className="artist-albums"><div className="section-heading"><h2>Albums in your library</h2></div><div className="group-grid">{artistAlbums.slice(0,6).map(group=><GroupCard key={group.id} group={group} onOpen={onOpenAlbum} />)}</div></section>}</>
      :groups.length?<><div className="group-grid">{groups.slice(currentPage*48,(currentPage+1)*48).map(group=><GroupCard key={group.id} group={group} artist={kind==="Artists"} onOpen={onOpen} />)}</div><div className="track-pagination" aria-label={`${kind} pages`}><span>{groups.length} {kind.toLowerCase()}</span><div><button disabled={currentPage===0} onClick={()=>setPage(currentPage-1)}>Previous</button><span role="status">{currentPage+1} / {pageCount}</span><button disabled={currentPage+1>=pageCount} onClick={()=>setPage(currentPage+1)}>Next</button></div></div></>
      :!props.isLoading&&<div className="library-empty"><PlayerIcon name={kind==="Albums"?"album":"artist"} /><div><h2>Your {kind.toLowerCase()} belong here</h2><p>Add a music folder in Your Library, then scan it from Library.</p></div></div>}
  </div></main>;
}
