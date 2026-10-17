import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";
import { musicFolderFromPath } from "../src/library/musicFolders.ts";

// Exercise async scan workflows without adding a DOM/testing dependency.
// Only React's state storage and the native IPC boundary are mocked.
function workflow(invoke, initialFolders = [{ key: "music", path: "D:/Music", name: "Music" }]) {
  const slots = [];
  let cursor = 0;
  let folders = initialFolders;
  let snapshot = null;
  let effects = [];
  const react = {
    useState(initial) {
      const index = cursor++;
      if (!(index in slots)) slots[index] = initial;
      return [slots[index], (next) => { slots[index] = typeof next === "function" ? next(slots[index]) : next; }];
    },
    useRef(initial) {
      const index = cursor++;
      if (!(index in slots)) slots[index] = { current: initial };
      return slots[index];
    },
    useEffect(effect, dependencies) {
      const index = cursor++;
      if (!(index in slots) || dependencies.some((value, i) => value !== slots[index][i])) {
        slots[index] = dependencies; effects.push(effect);
      }
    },
  };
  const source = ts.transpileModule(readFileSync(new URL("../src/hooks/useMusicLibrary.ts", import.meta.url), "utf8"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText;
  const module = { exports: {} };
  new Function("require", "module", "exports", source)((name) => {
    if (name === "react") return react;
    if (name === "@tauri-apps/api/core") return { invoke, Channel: class { onmessage = () => {}; } };
    if (name === "../library/musicFolders") return { musicFolderFromPath };
    throw new Error(`Unexpected import: ${name}`);
  }, module, module.exports);
  return { render(nextFolders = folders, nextSnapshot = snapshot) {
    folders = nextFolders; snapshot = nextSnapshot; cursor = 0; effects = [];
    const value = module.exports.useMusicLibrary(folders, snapshot); effects.forEach((effect) => effect()); return value;
  } };
}

const track = { id: "fixture", title: "Fixture", available: true };
const result = (overrides = {}) => ({ tracks: [track], discovered: 1, skipped: 0, warnings: 0, recoveredTags: 0, limited: false, cancelled: false, scannedPaths: ["D:/Music"], ...overrides });

test("duplicate scan clicks submit once and native progress is exposed", async () => {
  let complete;
  let calls = 0;
  const app = workflow((command, args) => {
    assert.equal(command, "scan_music"); assert.deepEqual(args.roots, ["D:/Music"]); calls++;
    args.onProgress.onmessage({ discovered: 4, skipped: 1 });
    return new Promise((resolve) => { complete = resolve; });
  });
  const library = app.render(); const pending = library.scanMusic();
  await library.scanMusic(); assert.equal(calls, 1);
  assert.equal(app.render().isScanning, true); assert.equal(app.render().progress.discovered, 4);
  complete(result()); await pending;
  assert.equal(app.render().isScanning, false); assert.deepEqual(app.render().tracks, [track]);
});

test("cancellation keeps the previous library and a later scan is allowed", async () => {
  let complete;
  let scans = 0;
  let cancellations = 0;
  const app = workflow((command) => {
    if (command === "cancel_music_scan") { cancellations++; return Promise.resolve(); }
    scans++;
    if (scans === 2) return new Promise((resolve) => { complete = resolve; });
    return Promise.resolve(result());
  });
  await app.render().scanMusic();
  const pending = app.render().scanMusic(); await app.render().cancelScan();
  assert.equal(cancellations, 1); assert.equal(app.render().isCancelling, true);
  complete(result({ tracks: [], cancelled: true })); await pending;
  assert.deepEqual(app.render().tracks, [track]); assert.match(app.render().summary, /cancelled/);
  await app.render().scanMusic(); assert.equal(scans, 3);
});

test("scan errors preserve tracks, report details, and release the submission guard", async () => {
  let calls = 0;
  const app = workflow(() => ++calls === 2 ? Promise.reject(new Error("Drive disconnected")) : Promise.resolve(result()));
  await app.render().scanMusic(); await app.render().scanMusic();
  assert.match(app.render().error, /Drive disconnected/); assert.equal(app.render().isScanning, false);
  assert.deepEqual(app.render().tracks, [track]);
  await app.render().scanMusic(); assert.equal(calls, 3); assert.equal(app.render().error, "");
});

test("empty results replace the snapshot and adding a folder marks the library stale", async () => {
  const app = workflow(() => Promise.resolve(result({ tracks: [], discovered: 0 })));
  await app.render().scanMusic(); assert.equal(app.render().hasScanned, true); assert.deepEqual(app.render().tracks, []);
  assert.equal(app.render([{ key: "new", path: "D:/New", name: "New" }]).foldersChanged, true);
});

test("no selected folders means no native scan request", async () => {
  const app = workflow(() => { throw new Error("Should not invoke"); }, []);
  await app.render().scanMusic(); assert.equal(app.render().isScanning, false);
});

test("startup restores saved metadata and unavailable files without scanning", () => {
  const app = workflow(() => { throw new Error("Startup must not scan"); });
  const snapshot = { tracks: [{ ...track, available: false }], scannedPaths: ["D:/Music"] };
  assert.equal(app.render(undefined, snapshot).isRestoring, true);
  const library = app.render();
  assert.deepEqual(library.tracks, snapshot.tracks); assert.equal(library.hasScanned, true);
  assert.equal(library.isRestoring, false);
  assert.match(library.summary, /1 saved tracks restored/); assert.match(library.summary, /1 unavailable/);
});


test("favorites wait for persistence, update shared state, and block duplicate saves and scans", async () => {
  let complete; const calls = [];
  const app = workflow((command, args) => {
    calls.push(command);
    if (command === "scan_music") return Promise.resolve(result());
    assert.deepEqual(args, { id: track.id, favorite: true });
    return new Promise(resolve => { complete = resolve; });
  });
  await app.render().scanMusic();
  const before = app.render();
  const pending = before.toggleFavorite({ ...track, favorite: false });
  await before.toggleFavorite(track); await before.scanMusic();
  assert.deepEqual(calls, ["scan_music", "set_track_favorite"]);
  assert.equal(app.render().isSavingFavorite, true);
  assert.equal(app.render().tracks[0].favorite, undefined);
  complete(true); await pending;
  assert.equal(app.render().tracks[0].favorite, true);
  assert.equal(app.render().isSavingFavorite, false);
});

test("favorite errors preserve state and allow unfavorite retry", async () => {
  let fail = true;
  const app = workflow((command, args) => {
    if (command === "scan_music") return Promise.resolve(result({ tracks: [{ ...track, favorite: true }] }));
    assert.equal(args.favorite, false);
    return fail ? Promise.reject(new Error("Database unavailable")) : Promise.resolve(false);
  });
  await app.render().scanMusic();
  await app.render().toggleFavorite(app.render().tracks[0]);
  assert.equal(app.render().tracks[0].favorite, true);
  assert.match(app.render().error, /Database unavailable/);
  fail = false;
  await app.render().toggleFavorite(app.render().tracks[0]);
  assert.equal(app.render().tracks[0].favorite, false);
  assert.equal(app.render().error, "");
});
