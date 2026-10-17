import test from "node:test";
import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
import {createRequire} from "node:module";
import ts from "typescript";
import {pushView,totalDuration,collectionDuration} from "../src/library/presentation.ts";
import {normalizeMetadata,groupLibrary} from "../src/library/navigation.ts";
const require=createRequire(import.meta.url);
function load(file,imports={}){
 const source=ts.transpileModule(readFileSync(new URL(`../src/components/${file}.tsx`,import.meta.url),"utf8"),{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX,target:ts.ScriptTarget.ES2020}}).outputText;
 const module={exports:{}};new Function("require","module","exports",source)(name=>name in imports?imports[name]:require(name),module,module.exports);return module.exports;
}
function all(node,type){if(!node||typeof node!=="object")return [];return [...(node.type===type?[node]:[]),...[node.props?.children].flat(Infinity).flatMap(child=>all(child,type))];}
const track={id:"a",title:"Evening",artist:"Sade",album:"Promise",available:true,favorite:true,durationSeconds:120,artwork:null};
test("view history retains album and playlist identities and discards only the forward branch",()=>{
 const home={section:"Home",groupId:null,playlistId:null};
 let history={entries:[home],index:0};
 history=pushView(history,{section:"Albums",groupId:"sade/promise",playlistId:null});
 history=pushView(history,{section:"Playlists",groupId:null,playlistId:"custom"});
 assert.equal(history.index,2);assert.equal(history.entries[1].groupId,"sade/promise");
 const backed={...history,index:1};assert.equal(pushView(backed,backed.entries[1]),backed);
 const branched=pushView(backed,{section:"Queue",groupId:null,playlistId:null});
 assert.deepEqual(branched.entries.map(v=>v.section),["Home","Albums","Queue"]);assert.equal(history.entries[2].playlistId,"custom");
});
test("collection duration includes only known finite audio durations",()=>{
 assert.equal(totalDuration([track,{...track,durationSeconds:null},{...track,durationSeconds:NaN},{...track,durationSeconds:-1}]),120);
 assert.equal(collectionDuration(3660),"1 hr 1 min");
});
test("album header Play uses the same full selection as its track list",()=>{
 const {LibraryBrowse}=load("LibraryBrowse",{react:{useState:initial=>[initial,()=>{}]},"../library/navigation":{groupLibrary},"./PlayerIcon":{PlayerIcon:"icon"},"./AlbumArtwork":{AlbumArtwork:"art"},"./CollectionHeader":{CollectionHeader:"header"},"./CollectionTracks":{CollectionTracks:"tracks"}});
 const tracks=[{...track,available:false}, {...track,id:"b"}];const calls=[];
 const view=LibraryBrowse({kind:"Albums",groups:[{id:"album",name:"Promise",subtitle:"Sade",tracks,artwork:null}],selectedId:"album",onOpen:()=>{},library:{tracks},playback:{isReady:true,playTrack:(...args)=>calls.push(args)}});
 all(view,"header")[0].props.onPlay();assert.deepEqual(calls,[["b",{liked:false,selection:true,trackIds:["a","b"]}]]);
});
test("Now Playing uses current local metadata and queue and preserves favorite actions",()=>{
 const {NowPlayingPanel}=load("NowPlayingPanel",{"./AlbumArtwork":{AlbumArtwork:"art"},"./FavoriteButton":{FavoriteButton:"favorite"},"./PlayerIcon":{PlayerIcon:"icon"}});
 const calls=[];const onToggle=async()=>{};
 const view=NowPlayingPanel({playback:{track,status:"paused",queue:[{id:1,track:{...track,id:"b",title:"Jezebel"}}]},favoriteTrack:track,onToggleFavorite:onToggle,favoritesDisabled:false,onClose:()=>calls.push("close"),onOpenQueue:()=>calls.push("queue")});
 assert.equal(all(view,"favorite")[0].props.onToggle,onToggle);assert.equal(all(view,"li").length,1);
 all(view,"button").find(b=>b.props["aria-label"]==="Hide Now Playing").props.onClick();all(view,"button").find(b=>b.props.children==="Open queue").props.onClick();assert.deepEqual(calls,["close","queue"]);
 const idle=NowPlayingPanel({playback:{track:null,queue:[]},onClose:()=>{},onOpenQueue:()=>{}});assert.equal(all(idle,"favorite").length,0);assert.equal(all(idle,"li").length,0);
});
test("sidebar collection filters and search use actual library items and existing navigation",()=>{
 const slots=[];let cursor=0;const react={useMemo:fn=>fn(),useState:initial=>{const i=cursor++;if(!(i in slots))slots[i]=initial;return [slots[i],value=>{slots[i]=typeof value==="function"?value(slots[i]):value;}];}};
 const {AppSidebar}=load("AppSidebar",{react,"../library/navigation":{normalizeMetadata},"./LibraryFolders":{LibraryFolders:"folders"},"./AlbumArtwork":{AlbumArtwork:"art"},"./PlayerIcon":{PlayerIcon:"icon"}});
 const calls=[];const props={tracks:[track],albums:[],artists:[{id:"sade",name:"Sade",tracks:[track]},{id:"other",name:"Other",tracks:[]}],playlists:[],activeSection:"Home",musicFolders:{},onOpenGroup:(...args)=>calls.push(args),onSectionChange:section=>calls.push([section])};
 function render(){cursor=0;return AppSidebar(props);}
 let view=render();all(view,"button").find(b=>b.props.children==="Artists").props.onClick();assert.deepEqual(calls.pop(),["Artists"]);
 view=render();assert.equal(all(view,"button").filter(b=>b.props.className?.includes("library-item")).length,2);
 all(view,"input")[0].props.onChange({currentTarget:{value:"  SADE  "}});view=render();const items=all(view,"button").filter(b=>b.props.className?.includes("library-item"));assert.equal(items.length,1);items[0].props.onClick();assert.deepEqual(calls.pop(),["Artists","sade"]);
});
test("bottom player exposes only supported transport controls and preserves callbacks",()=>{
 const {PlayerBar}=load("PlayerBar",{"./FavoriteButton":{FavoriteButton:"favorite"},"./PlayerIcon":{PlayerIcon:"icon"},"./AlbumArtwork":{AlbumArtwork:"art"},"./PlaybackVolume":{PlaybackVolume:"volume"},"./PlaybackProgress":{PlaybackProgress:"progress"}});
 const calls=[];const playback={track,isReady:true,status:"paused",queue:[],previousTrack:()=>calls.push("previous"),nextTrack:()=>calls.push("next"),togglePlayback:()=>calls.push("play")};
 const view=PlayerBar({playback,onOpenQueue:()=>calls.push("queue")});const buttons=all(view,"button");
 assert.equal(buttons.some(b=>["Shuffle","Repeat"].includes(b.props["aria-label"])),false);
 for(const label of ["Previous track","Play","Next track"])buttons.find(b=>b.props["aria-label"]===label).props.onClick();assert.deepEqual(calls,["previous","play","next"]);
});
