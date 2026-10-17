import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import ts from "typescript";
const require = createRequire(import.meta.url);
const flush = () => new Promise(setImmediate);
function load(file, imports) {
  const source = ts.transpileModule(readFileSync(new URL(`../src/${file}`,import.meta.url),"utf8"), { compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2020,jsx:ts.JsxEmit.ReactJSX} }).outputText;
  const module={exports:{}};
  new Function("require","module","exports",source)(name => name in imports ? imports[name] : require(name),module,module.exports);
  return module.exports;
}
function workflow(invoke) {
  const slots=[]; let cursor=0; let effects=[];
  const react={
    useState(initial) { const i=cursor++; if (!(i in slots)) slots[i]=initial; return [slots[i],value=>{slots[i]=typeof value === "function"?value(slots[i]):value;}]; },
    useRef(initial) {const i=cursor++; if (!(i in slots)) slots[i]={current:initial}; return slots[i];},
    useEffect(effect) { const i=cursor++; if (!(i in slots)) {slots[i]=true;effects.push(effect);} }
  };
  const {usePlaylists}=load("hooks/usePlaylists.ts",{react,"@tauri-apps/api/core":{invoke:(command,args)=>{assert.equal(command,"playlist_control");return invoke(args.action);}}});
  return {render(){cursor=0;effects=[];const value=usePlaylists();effects.forEach(fn=>fn());return value;}};
}
const playlist={id:"custom",name:"Night",createdAt:1,updatedAt:1,trackIds:[]};
test("playlists restore, trim names, prevent rapid creation, and publish confirmed membership",async()=>{
  let complete; const calls=[];
  const app=workflow(action=>{
    calls.push(action);
    if(action.type==="list") return Promise.resolve([]);
    if(action.type==="create") return new Promise(resolve=>{complete=resolve;});
    return Promise.resolve([{...playlist,trackIds:["song"]}]);
  });
  app.render();await flush();assert.equal(app.render().isLoaded,true);
  await app.render().create(" ");assert.match(app.render().error,/name/);assert.equal(calls.length,1);
  const before=app.render(); const pending=before.create("  Night  ");await before.create("  Night  ");
  assert.deepEqual(calls.at(-1),{type:"create",name:"Night"});assert.equal(calls.length,2);
  assert.equal(app.render().isBusy,true);assert.deepEqual(app.render().playlists,[]);
  complete([playlist]);assert.equal((await pending).id,"custom");
  await app.render().add("custom","song");
  assert.deepEqual(calls.at(-1),{type:"add",playlistId:"custom",trackId:"song"});
  assert.deepEqual(app.render().playlists[0].trackIds,["song"]);assert.match(app.render().message,/saved/);
});
test("playlist failures keep confirmed data and release the guard for retry",async()=>{
  let fail=true;const calls=[];
  const app=workflow(action=>{calls.push(action);if(action.type==="list")return Promise.resolve([playlist]);return fail?Promise.reject(new Error("SQLite unavailable")):Promise.resolve([{...playlist,name:action.name}]);});
  app.render();await flush();await app.render().rename("custom","  Changed  ");
  assert.equal(app.render().playlists[0].name,"Night");assert.match(app.render().error,/SQLite unavailable/);assert.equal(app.render().isBusy,false);
  fail=false;await app.render().rename("custom","  Changed  ");
  assert.deepEqual(calls.at(-1),{type:"rename",playlistId:"custom",name:"Changed"});assert.equal(app.render().playlists[0].name,"Changed");
});
function find(node,type){if(!node||typeof node!=="object")return; if(node.type===type)return node;return [node.props?.children].flat(Infinity).map(child=>find(child,type)).find(Boolean);}
test("playlist view resolves current library metadata in playlist order and passes full playback context",()=>{
  const {PlaylistView}=load("components/PlaylistView.tsx",{react:{useState:initial=>[initial,()=>{}]},"./CollectionHeader":{CollectionHeader:"header"},"./PlayerIcon":{PlayerIcon:"icon"},"./AlbumArtwork":{AlbumArtwork:"art"},"./TrackList":{TrackList:"tracks"}});
  const a={id:"a",title:"Evening",artist:"Sade",album:"Album"};const b={...a,id:"b",title:"Jezebel"};
  const calls=[];const removed=[];
  const props={manager:{playlists:[{...playlist,trackIds:["b","a"]}],remove:(...args)=>removed.push(args)},selectedId:"custom",library:{tracks:[a,b]},playback:{playTrack:(...args)=>calls.push(args)},search:"Evening",onAddToPlaylist:()=>{}};
  const list=find(PlaylistView(props),"tracks");assert.deepEqual(list.props.tracks,[a]);list.props.onPlay("a");
  assert.deepEqual(calls.pop(),["a",{liked:false,playlistId:"custom",trackIds:["b","a"]}]);
  assert.deepEqual(find(PlaylistView({...props,search:""}),"tracks").props.tracks,[b,a]);
  const removeButton=list.props.renderActions(a).props.children[1];let stopped=false;
  removeButton.props.onClick({stopPropagation(){stopped=true;}});assert.equal(stopped,true);assert.deepEqual(removed,[["custom","a"]]);
});
