import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import ts from "typescript";
import { pushView } from "../src/library/presentation.ts";
import { groupLibrary, normalizeMetadata, searchTracks } from "../src/library/navigation.ts";
const require=createRequire(import.meta.url);
const track=(id,overrides={})=>({id,path:id,title:"Hang On To Your Love",artist:"Sade",album:"Diamond Life",albumArtist:null,artwork:null,available:true,favorite:false,discNumber:1,trackNumber:1,...overrides});
test("local search matches title artist and album with normalized whitespace and empty-query behavior",()=>{
 const tracks=[track("one"),track("two",{title:"Jezebel",album:"Promise"}),track("three",{artist:"Other",title:"Elsewhere",album:"Unknown"})];
 assert.deepEqual(searchTracks(tracks,"  JEZEBEL  ").map(t=>t.id),["two"]);
 assert.deepEqual(searchTracks(tracks,"sade").map(t=>t.id),["one","two"]);
 assert.deepEqual(searchTracks(tracks," diamond   life ").map(t=>t.id),["one"]);
 assert.deepEqual(searchTracks(tracks,"sade promise").map(t=>t.id),["two"]);
 assert.deepEqual(searchTracks(tracks," \t\n "),tracks);assert.deepEqual(searchTracks(tracks,"missing"),[]);
 assert.equal(normalizeMetadata(" \uff33\uff21\uff24\uff25  "),"sade");
});
test("albums normalize metadata, use album artist identity, and preserve disc/track order",()=>{
 const art={id:"real-art",mime:"image/png"};
 const tracks=[track("b",{album:"  Diamond   Life",artist:"SADE",trackNumber:2}),track("a",{album:"diamond life",artist:" sade ",artwork:art}),track("c",{artist:"Other",albumArtist:"Sade",discNumber:2,trackNumber:1}),track("d",{artist:"Different artist"})];
 const groups=groupLibrary(tracks,"Albums");assert.equal(groups.length,2);
 const sade=groups.find(g=>g.tracks.some(t=>t.id==="a"));assert.deepEqual(sade.tracks.map(t=>t.id),["a","b","c"]);assert.deepEqual(sade.artwork,art);
 assert.equal(sade.id,groupLibrary([...tracks].reverse(),"Albums").find(g=>g.tracks.some(t=>t.id==="a")).id);
 assert.deepEqual(tracks.map(t=>t.id),["b","a","c","d"],"Grouping does not mutate library order");
});
test("artists and missing metadata form real stable groups without fabricated entries",()=>{
 const groups=groupLibrary([track("a"),track("b",{artist:"  SADE  "}),track("c",{artist:" ",album:" "})],"Artists");
 assert.equal(groups.length,2);assert.equal(groups.find(g=>g.name==="Sade").tracks.length,2);assert.equal(groups.find(g=>g.name==="Unknown Artist").tracks.length,1);
 assert.equal(groupLibrary([track("c",{artist:"",album:""})],"Albums")[0].name,"Unknown Album");assert.deepEqual(groupLibrary([],"Albums"),[]);
});
function load(file,imports={}){
 const source=ts.transpileModule(readFileSync(new URL(`../src/components/${file}.tsx`,import.meta.url),"utf8"),{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX,target:ts.ScriptTarget.ES2020}}).outputText;
 const module={exports:{}};new Function("require","module","exports",source)(name=>name in imports?imports[name]:require(name),module,module.exports);return module.exports;
}
function all(node,type){if(!node||typeof node!=="object")return [];return [...(node.type===type?[node]:[]),...[node.props?.children].flat(Infinity).flatMap(child=>all(child,type))];}
test("collection rows pass the complete pre-pagination selection to the audio engine",()=>{
 const {CollectionTracks}=load("CollectionTracks",{"./TrackList":{TrackList:"tracks"}});const calls=[];
 const tracks=Array.from({length:70},(_,i)=>track(`song-${i}`));
 const list=CollectionTracks({tracks,library:{},playback:{playTrack:(...args)=>calls.push(args)},onAddToPlaylist:()=>{}});
 list.props.onPlay("song-51");assert.deepEqual(calls[0],["song-51",{liked:false,selection:true,trackIds:tracks.map(t=>t.id)}]);
});
test("search view filters the full saved library and clear returns control to navigation",()=>{
 const {SearchView}=load("SearchView",{react:{useDeferredValue:value=>value,useMemo:fn=>fn()},"../library/navigation":{normalizeMetadata,searchTracks},"./CollectionTracks":{CollectionTracks:"tracks"},"./PlayerIcon":{PlayerIcon:"icon"},"./CollectionHeader":{CollectionHeader:"header"},"./LibraryBrowse":{GroupCard:"card"},"../library/navigation":{groupLibrary,normalizeMetadata,searchTracks}});
 let cleared=false;const tracks=[track("one"),track("two",{title:"Jezebel",album:"Promise"})];
 const view=SearchView({query:"promise",onClear:()=>{cleared=true;},library:{tracks},playback:{}});
 assert.deepEqual(all(view,"tracks")[0].props.tracks.map(t=>t.id),["two"]);all(view,"button")[0].props.onClick();assert.equal(cleared,true);
 const empty=SearchView({query:"missing",onClear:()=>{},library:{tracks},playback:{}});assert.equal(all(empty,"tracks").length,0);assert.ok(all(empty,"h2").some(h=>h.props.children==="No songs found."));
});
test("album and artist browsing passes the selected group rather than the global library",()=>{
 const {LibraryBrowse}=load("LibraryBrowse",{react:{useState:initial=>[initial,()=>{}]},"./AlbumArtwork":{AlbumArtwork:"art"},"./CollectionTracks":{CollectionTracks:"tracks"},"./PlayerIcon":{PlayerIcon:"icon"},"./CollectionHeader":{CollectionHeader:"header"},"./LibraryBrowse":{GroupCard:"card"},"../library/navigation":{groupLibrary,normalizeMetadata,searchTracks}});
 const tracks=[track("one"),track("two",{album:"Promise"})];const groups=groupLibrary(tracks,"Albums");
 const selected=groups.find(g=>g.tracks.some(t=>t.id==="two"));const view=LibraryBrowse({kind:"Albums",groups,selectedId:selected.id,onOpen:()=>{},library:{tracks},playback:{}});
 assert.deepEqual(all(view,"tracks")[0].props.tracks,[tracks[1]]);
 const list=LibraryBrowse({kind:"Albums",groups,selectedId:null,onOpen:()=>{},library:{tracks},playback:{}});assert.equal(all(list,"tracks").length,0);
});


test("App search is global across navigation and browsing clears search without scanning",()=>{
 const slots=[];let cursor=0;const react={useState:initial=>{const i=cursor++;if(!(i in slots))slots[i]=initial;return [slots[i],value=>{slots[i]=typeof value==="function"?value(slots[i]):value;}];},useMemo:fn=>fn()};
 const tracks=[track("one"),track("two",{title:"Jezebel",album:"Promise"})];
 const source=ts.transpileModule(readFileSync(new URL("../src/App.tsx",import.meta.url),"utf8"),{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX,target:ts.ScriptTarget.ES2020}}).outputText;
 const imports={react,"./components/NowPlayingPanel":{NowPlayingPanel:"nowpanel"},"./library/presentation":{pushView},"./components/QueueView":{QueueView:"queue"},"./hooks/usePlaylists":{usePlaylists:()=>({playlists:[]})},"./components/PlaylistView":{PlaylistView:"playlists",PlaylistFeedback:"feedback"},"./components/PlaylistPicker":{PlaylistPicker:"picker"},"./components/SearchView":{SearchView:"search"},"./components/LibraryBrowse":{LibraryBrowse:"browse"},"./components/HomeOverview":{HomeOverview:"overview"},"./library/navigation":{groupLibrary,normalizeMetadata},"./components/AppSidebar":{AppSidebar:"sidebar"},"./components/TopBar":{TopBar:"top"},"./components/MainContent":{MainContent:"main"},"./components/PlayerBar":{PlayerBar:"player"},"./hooks/useMusicFolders":{useMusicFolders:()=>({folders:[],snapshot:null})},"./hooks/useMusicLibrary":{useMusicLibrary:()=>({tracks,scanMusic:()=>{throw Error("Search must not scan");}})},"./hooks/usePlayback":{usePlayback:()=>({track:null})},"./App.css":{}};
 const module={exports:{}};new Function("require","module","exports",source)(name=>name in imports?imports[name]:require(name),module,module.exports);
 function render(){cursor=0;return module.exports.default();}
 let view=render();assert.equal(all(view,"main")[0].props.overview.props.albums.length,2);
 all(view,"sidebar")[0].props.onSectionChange("Artists");view=render();assert.equal(all(view,"browse")[0].props.kind,"Artists");
 all(view,"top")[0].props.onSearchChange("Sade");view=render();assert.equal(all(view,"search")[0].props.library.tracks,tracks);assert.equal(all(view,"browse").length,0);
 all(view,"search")[0].props.onClear();view=render();assert.equal(all(view,"browse")[0].props.kind,"Artists");
 all(view,"sidebar")[0].props.onSectionChange("Queue");view=render();assert.equal(all(view,"queue").length,1);
 all(view,"top")[0].props.onSearchChange("Jezebel");view=render();assert.equal(all(view,"search").length,1);
 all(view,"sidebar")[0].props.onSectionChange("Library");view=render();assert.equal(all(view,"main")[0].props.activeSection,"Library");assert.equal(all(view,"search").length,0);
});
