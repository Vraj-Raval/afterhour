import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";
import { musicFolderFromPath } from "../src/library/musicFolders.ts";

function workflow(invoke) {
  const slots = []; let cursor = 0; let effects = [];
  const react = {
    useState(initial) { const i = cursor++; if (!(i in slots)) slots[i] = initial; return [slots[i], (value) => { slots[i] = typeof value === "function" ? value(slots[i]) : value; }]; },
    useRef(initial) { const i = cursor++; if (!(i in slots)) slots[i] = { current: initial }; return slots[i]; },
    useEffect(effect) { const i = cursor++; if (!(i in slots)) { slots[i] = true; effects.push(effect); } },
  };
  const source = ts.transpileModule(readFileSync(new URL("../src/hooks/useMusicFolders.ts", import.meta.url), "utf8"), { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText;
  const module = { exports: {} };
  new Function("require", "module", "exports", source)((name) => {
    if (name === "react") return react;
    if (name === "@tauri-apps/api/core") return { invoke, isTauri: () => true };
    if (name === "../library/musicFolders") return { musicFolderFromPath };
    throw new Error(`Unexpected import: ${name}`);
  }, module, module.exports);
  return { render() { cursor = 0; effects = []; const value = module.exports.useMusicFolders(); effects.forEach((effect) => effect()); return value; } };
}
const folder = { ...musicFolderFromPath("D:/Music"), available: true };
const snapshot = { folders: [folder], tracks: [], scannedPaths: [] };
const flush = () => new Promise(setImmediate);

test("startup restores saved folders and prevents duplicate picker additions", async () => {
  const app = workflow((command) => Promise.resolve(command === "load_library" ? snapshot : folder.path));
  assert.equal(app.render().isLoading, true); await flush();
  assert.deepEqual(app.render().folders, [folder]); assert.equal(app.render().isLoading, false);
  await app.render().addMusicFolder();
  assert.equal(app.render().folders.length, 1); assert.match(app.render().message, /already selected/);
});

test("startup database errors remain visible and retry can restore the library", async () => {
  let calls = 0;
  const app = workflow(() => ++calls === 1 ? Promise.reject(new Error("Database locked")) : Promise.resolve(snapshot));
  app.render(); await flush(); assert.match(app.render().loadError, /Database locked/);
  assert.equal(app.render().isLoading, false); assert.equal(app.render().snapshot, null);
  await app.render().restoreLibrary(); assert.equal(app.render().loadError, ""); assert.deepEqual(app.render().folders, [folder]);
});

test("picker cancellation and failed folder saves preserve restored folders", async () => {
  let selections = 0;
  const app = workflow((command) => command === "load_library" ? Promise.resolve(snapshot) : ++selections === 1 ? Promise.resolve(null) : Promise.reject(new Error("Cannot save folder")));
  app.render(); await flush(); await app.render().addMusicFolder();
  assert.equal(app.render().error, ""); assert.deepEqual(app.render().folders, [folder]);
  await app.render().addMusicFolder(); assert.match(app.render().error, /Cannot save folder/); assert.deepEqual(app.render().folders, [folder]);
});

test("a returning scan snapshot cannot erase a newer folder selection", async () => {
  const app = workflow((command) => Promise.resolve(command === "load_library" ? snapshot : "D:/More"));
  app.render(); await flush(); await app.render().addMusicFolder();
  app.render().updateFolders([{ ...folder, available: false }]);
  assert.equal(app.render().folders.length, 2); assert.equal(app.render().folders[0].available, false);
  await app.render().addMusicFolder(); assert.equal(app.render().folders.length, 2);
});
