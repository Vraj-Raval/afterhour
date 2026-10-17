import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import ts from "typescript";
const require = createRequire(import.meta.url);
function component(file, imports = {}) {
  const source = ts.transpileModule(readFileSync(new URL(`../src/components/${file}.tsx`, import.meta.url), "utf8"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020, jsx: ts.JsxEmit.ReactJSX },
  }).outputText;
  const module = { exports: {} };
  new Function("require", "module", "exports", source)(name => name in imports ? imports[name] : require(name), module, module.exports);
  return module.exports;
}
function find(node, type) {
  if (!node || typeof node !== "object") return undefined;
  if (node.type === type) return node;
  return [node.props?.children].flat(Infinity).map(child => find(child, type)).find(Boolean);
}
const track = { id: "one", title: "Evening", artist: "Artist", album: "Album", favorite: true };
test("heart exposes liked state and stops row propagation", async () => {
  const { FavoriteButton } = component("FavoriteButton", { "./PlayerIcon": { PlayerIcon: "icon" } });
  let stopped = false; let toggled;
  const button = FavoriteButton({ track, disabled: false, onToggle: async value => { toggled = value; } });
  assert.equal(button.props["aria-pressed"], true);
  assert.match(button.props["aria-label"], /Remove Evening from Liked Songs/);
  button.props.onClick({ stopPropagation() { stopped = true; } });
  assert.equal(stopped, true); assert.equal(toggled, track);
});
test("Liked Songs filters actual view rows and reuses the playback callback", () => {
  const { MainContent } = component("MainContent", {
    react: { useMemo: fn => fn(), useDeferredValue: value => value },
    "./CollectionHeader": { CollectionHeader:"header" }, "./TrackList": { TrackList: "tracks" }, "./PlayerIcon": { PlayerIcon: "icon" },
  });
  const tracks = [track, { ...track, id: "two", favorite: false }];
  const calls = [];
  const playTrack = (...args) => { calls.push(args); };
  const props = { activeSection: "Liked Songs", search: "", folderCount: 1, library: { tracks }, playback: { playTrack } };
  const list = find(MainContent(props), "tracks");
  assert.deepEqual(list.props.tracks, [track]); list.props.onPlay(track.id);
  assert.deepEqual(calls.pop(), [track.id, { liked: true, trackIds: [track.id] }]);
  const home = find(MainContent({ ...props, activeSection: "Home" }), "tracks");
  assert.equal(home.props.tracks.length, 2);
  home.props.onPlay(track.id);
  assert.deepEqual(calls.pop(), [track.id, { liked: false, trackIds: [track.id, "two"] }]);
  const added = { ...track, id: "three", title: "Jezebel" };
  const refreshed = { ...props, search: "Evening", library: { tracks: [...tracks, added] } };
  const filtered = find(MainContent(refreshed), "tracks");
  assert.deepEqual(filtered.props.tracks, [track]);
  filtered.props.onPlay(track.id);
  assert.deepEqual(calls.pop()[1], { liked: true, trackIds: [track.id, "three"] });
  assert.equal(find(MainContent({ ...props, library: { tracks: [{ ...track, favorite: false }] } }), "tracks"), undefined);
});
