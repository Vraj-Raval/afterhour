import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import ts from "typescript";
const require=createRequire(import.meta.url);
function load(file,imports={}){
 const source=ts.transpileModule(readFileSync(new URL(`../src/components/${file}.tsx`,import.meta.url),"utf8"),{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX,target:ts.ScriptTarget.ES2020}}).outputText;
 const module={exports:{}};new Function("require","module","exports",source)(name=>name==="./PlayerIcon"?{PlayerIcon:"icon"}:name in imports?imports[name]:require(name),module,module.exports);return module.exports;
}
function all(node,type){if(!node||typeof node!=="object")return [];return [...(node.type===type?[node]:[]),...[node.props?.children].flat(Infinity).flatMap(child=>all(child,type))];}
const track={id:"song",title:"Evening",artist:"Sade",available:true,artwork:null};
test("queue view uses occurrence IDs for moves/removal and leaves current song separate",()=>{
 const {QueueView}=load("QueueView",{react:{useMemo:fn=>fn()},"./AlbumArtwork":{AlbumArtwork:"art"}});const calls=[];
 const playback={track,queue:[{id:7,track},{id:9,track}],isReady:true,status:"playing",moveQueued:(...args)=>calls.push(args),removeQueued:id=>calls.push([id]),clearQueue:()=>calls.push(["clear"])};
 const view=QueueView({playback,tracks:[track]});const rows=all(view,"li");assert.deepEqual(rows.map(row=>row.key),["7","9"]);
 const buttons=all(rows[0],"button");assert.equal(buttons[0].props.disabled,true);buttons[1].props.onClick();buttons[2].props.onClick();
 assert.deepEqual(calls,[[7,1],[7]]);assert.equal(all(view,"h2")[0].props.children,"Now playing");
});
test("track queue controls stop row propagation and only enqueue",()=>{
 const {TrackList}=load("TrackList",{react:{memo:fn=>fn,useState:initial=>[initial,()=>{}]},"./AlbumArtwork":{AlbumArtwork:"art"},"./FavoriteButton":{FavoriteButton:"favorite"}});
 const calls=[];const view=TrackList({tracks:[track],onPlay:()=>{throw Error("Unexpected playback");},onQueue:(...args)=>calls.push(args),favoritesDisabled:false,isBusy:false});
 assert.equal(all(view,"details").length,1);assert.equal(all(view,"summary")[0].props["aria-label"],"Actions for Evening");
 const buttons=all(view,"button").filter(b=>b.props["aria-label"]?.includes("next")||b.props["aria-label"]?.includes("queue"));
 let stopped=0;for(const button of buttons)button.props.onClick({stopPropagation(){stopped++;}});
 assert.equal(stopped,2);assert.deepEqual(calls,[[track.id,true],[track.id]]);
});
