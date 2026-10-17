import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";

function workflow(invoke) {
  const slots = []; let cursor = 0; let effects = []; let refresh;
  const timers = new Map(); let timerId = 0;
  const react = {
    useState(initial) { const i = cursor++; if (!(i in slots)) slots[i] = initial; return [slots[i], (value) => { slots[i] = typeof value === "function" ? value(slots[i]) : value; }]; },
    useCallback(fn, deps) { const i = cursor++; if (!(i in slots) || deps.some((d,j) => d !== slots[i].deps[j])) slots[i] = { fn, deps }; return slots[i].fn; },
    useRef(initial) { const i = cursor++; if (!(i in slots)) slots[i] = { current: initial }; return slots[i]; },
    useEffect(effect) { const i = cursor++; if (!(i in slots)) { slots[i] = true; effects.push(effect); } },
  };
  globalThis.window = { setTimeout(callback) { timers.set(++timerId, callback); return timerId; }, clearTimeout(id) { timers.delete(id); }, setInterval(callback) { refresh = callback; return 1; }, clearInterval() {} };
  const source = ts.transpileModule(readFileSync(new URL("../src/hooks/usePlayback.ts", import.meta.url), "utf8"), { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText;
  const module = { exports: {} };
  new Function("require", "module", "exports", source)((name) => {
    if (name === "react") return react;
    if (name === "@tauri-apps/api/core") return { invoke: (command,args) => { assert.equal(command,"playback_control"); return invoke(args.action); }, isTauri: () => true };
    throw new Error(`Unexpected import: ${name}`);
  }, module, module.exports);
  return {
    render() { cursor = 0; effects = []; const value = module.exports.usePlayback(); effects.forEach((effect) => effect()); return value; },
    poll() { refresh(); },
    volumeTick() { const entry = timers.entries().next().value; if (entry) { timers.delete(entry[0]); entry[1](); } },
  };
}
const track = { id: "saved-track", title: "Actual library track" };
const snapshot = (revision = 0, status = "stopped", current = null, error = null) => ({ volume: 1, muted: false, preferencesError: null, revision, status, track: current, error, sourceId: current ? 1 : 0, positionSeconds: 0, durationSeconds: current ? 60 : null, canSeek: !!current });
const flush = () => new Promise(setImmediate);

test("startup only requests status and never auto-plays", async () => {
  const actions = [];
  const app = workflow((action) => { actions.push(action.type); return Promise.resolve(snapshot()); });
  assert.equal(app.render().isReady,false); await flush();
  assert.equal(app.render().isReady,true); assert.equal(app.render().status,"stopped"); assert.deepEqual(actions,["status"]);
});

test("play waits for native confirmation and duplicate requests submit once", async () => {
  let complete; let plays = 0;
  const app = workflow((action) => action.type === "status" ? Promise.resolve(snapshot()) : (plays++, new Promise((resolve) => { complete = resolve; })));
  app.render(); await flush(); const state = app.render();
  const pending = state.playTrack(track.id); await state.playTrack(track.id);
  assert.equal(plays,1); assert.equal(app.render().status,"stopped"); assert.equal(app.render().track,null); assert.equal(app.render().isBusy,true);
  complete(snapshot(1,"playing",track)); await pending;
  assert.equal(app.render().status,"playing"); assert.equal(app.render().track.id,track.id); assert.equal(app.render().isBusy,false);
});

test("pause and resume use actual native commands rather than restarting the file", async () => {
  const actions = [];
  const app = workflow((action) => {
    actions.push(action.type);
    const status = action.type === "pause" ? "paused" : action.type === "stop" || action.type === "status" ? "stopped" : "playing";
    return Promise.resolve(snapshot(actions.length,status,action.type === "status" ? null : track));
  });
  app.render(); await flush(); await app.render().playTrack(track.id);
  await app.render().togglePlayback(); assert.equal(app.render().status,"paused");
  await app.render().togglePlayback(); assert.equal(app.render().status,"playing");
  await app.render().stopPlayback(); assert.equal(app.render().status,"stopped");
  assert.deepEqual(actions,["status","play","pause","resume","stop"]);
  await app.render().togglePlayback(); assert.equal(actions.at(-1),"play");
});

test("native decoder errors preserve the correct previous track and status", async () => {
  const app = workflow((action) => Promise.resolve(action.type === "status" ? snapshot(1,"playing",track) : snapshot(2,"playing",track,"Cannot decode selected file")));
  app.render(); await flush(); await app.render().playTrack("bad-track");
  assert.equal(app.render().track.id,track.id); assert.equal(app.render().status,"playing"); assert.match(app.render().error,/Cannot decode/);
});

test("IPC failures never optimistically claim playback started", async () => {
  const app = workflow((action) => action.type === "status" ? Promise.resolve(snapshot()) : Promise.reject(new Error("Worker unavailable")));
  app.render(); await flush(); await app.render().playTrack(track.id);
  assert.equal(app.render().status,"stopped"); assert.equal(app.render().track,null); assert.match(app.render().error,/Worker unavailable/); assert.equal(app.render().isBusy,false);
  assert.equal(app.render().isReady,false);
});

test("a delayed status response cannot undo a newer play result", async () => {
  let reads = 0; let completeStatus;
  const app = workflow((action) => {
    if (action.type !== "status") return Promise.resolve(snapshot(5,"playing",track));
    return ++reads === 1 ? Promise.resolve(snapshot()) : new Promise((resolve) => { completeStatus = resolve; });
  });
  app.render(); await flush(); app.poll(); await app.render().playTrack(track.id);
  completeStatus(snapshot(1)); await flush(); assert.equal(app.render().status,"playing"); assert.equal(app.render().revision,5);
});

test("device errors are reflected without frontend advancement", async () => {
  let reads = 0; const actions = [];
  const app = workflow((action) => {
    actions.push(action.type); reads++;
    return Promise.resolve(reads === 1 ? snapshot(1,"playing",track) : snapshot(2,"stopped",track,"Output device disconnected"));
  });
  app.render(); await flush(); app.poll(); await flush();
  assert.equal(app.render().status,"stopped"); assert.match(app.render().error,/disconnected/); assert.deepEqual(actions,["status","status"]);
});


test("progress follows native snapshots and paused seeking waits for native confirmation", async () => {
  let completeSeek;
  let native = { ...snapshot(1, "paused", track), positionSeconds: 12 };
  const actions = [];
  const app = workflow((action) => {
    actions.push(action);
    if (action.type === "seek") return new Promise(resolve => { completeSeek = resolve; });
    return Promise.resolve(native);
  });
  app.render(); await flush();
  assert.equal(app.render().positionSeconds, 12);
  app.poll(); await flush();
  assert.equal(app.render().positionSeconds, 12);
  const seeking = app.render().seek(30, 1);
  assert.equal(app.render().positionSeconds, 12);
  assert.deepEqual(actions.at(-1), { type: "seek", seconds: 30, sourceId: 1 });
  native = { ...native, revision: 2, positionSeconds: 30 };
  completeSeek(native); await seeking;
  assert.equal(app.render().status, "paused");
  assert.equal(app.render().positionSeconds, 30);
  native = { ...snapshot(3, "playing", { ...track, id: "second" }), sourceId: 2 };
  app.poll(); await flush();
  assert.equal(app.render().sourceId, 2);
  assert.equal(app.render().positionSeconds, 0);
});

test("failed seeks display native errors without inventing a new position", async () => {
  const paused = { ...snapshot(1, "paused", track), positionSeconds: 12 };
  const app = workflow(action => Promise.resolve(action.type === "status" ? paused : {
    ...paused, revision: 2, canSeek: false, error: "Seeking unsupported",
  }));
  app.render(); await flush(); await app.render().seek(30, 1);
  assert.equal(app.render().positionSeconds, 12);
  assert.equal(app.render().status, "paused");
  assert.equal(app.render().canSeek, false);
  assert.match(app.render().error, /unsupported/);
});


test("next and previous wait for native acknowledgement and reject overlapping clicks", async () => {
  const actions = []; let complete;
  const second = { ...track, id: "second", title: "Second track" };
  const app = workflow(action => {
    actions.push(action);
    if (action.type === "status") return Promise.resolve(snapshot(1, "playing", track));
    if (action.type === "next") return new Promise(resolve => { complete = resolve; });
    return Promise.resolve({ ...snapshot(3, "playing", track), sourceId: 3 });
  });
  app.render(); await flush();
  const before = app.render();
  const request = before.nextTrack();
  await before.nextTrack(); await before.previousTrack();
  assert.equal(actions.filter(a => a.type === "next").length, 1);
  assert.equal(actions.filter(a => a.type === "previous").length, 0);
  assert.equal(app.render().track.id, track.id);
  assert.equal(app.render().isBusy, true);
  assert.deepEqual(actions.at(-1), { type: "next", sourceId: 1 });
  complete({ ...snapshot(2, "playing", second), sourceId: 2 }); await request;
  assert.equal(app.render().track.id, "second");
  assert.equal(app.render().positionSeconds, 0);
  await app.render().previousTrack();
  assert.deepEqual(actions.at(-1), { type: "previous", sourceId: 2 });
  assert.equal(app.render().track.id, track.id);
});

test("native automatic advancement updates the current track without frontend play commands", async () => {
  let reads = 0; const actions = [];
  const next = { ...track, id: "next", title: "Next track", artist: "Next artist" };
  const app = workflow(action => {
    actions.push(action.type);
    return Promise.resolve(++reads === 1 ? snapshot(1, "playing", track) : {
      ...snapshot(2, "playing", next), sourceId: 2, durationSeconds: 120,
    });
  });
  app.render(); await flush(); app.poll(); await flush();
  assert.equal(app.render().track.id, "next");
  assert.equal(app.render().durationSeconds, 120);
  assert.equal(app.render().positionSeconds, 0);
  assert.equal(app.render().status, "playing");
  assert.deepEqual(actions, ["status", "status"]);
});

test("navigation failures preserve the native current track and release the request guard", async () => {
  const app = workflow(action => Promise.resolve(action.type === "status" ? snapshot(1, "playing", track) : {
    ...snapshot(2, "playing", track, "Cannot open next file"),
  }));
  app.render(); await flush(); await app.render().nextTrack();
  assert.equal(app.render().track.id, track.id);
  assert.equal(app.render().status, "playing");
  assert.equal(app.render().isBusy, false);
  assert.match(app.render().error, /Cannot open/);
});


test("volume slider coalesces bursts and keeps only the latest request while native gain is pending", async () => {
  const actions = []; const completions = [];
  const paused = { ...snapshot(1, "paused", track), volume: 0.6, positionSeconds: 20 };
  const app = workflow(action => {
    actions.push(action);
    if (action.type === "status") return Promise.resolve(paused);
    return new Promise(resolve => completions.push(resolve));
  });
  app.render(); await flush();
  const playback = app.render();
  playback.setVolume(0.2); playback.setVolume(0.3); playback.setVolume(0.4);
  assert.equal(actions.length, 1);
  assert.equal(app.render().isVolumeBusy, true);
  assert.equal(app.render().volume, 0.6);
  app.volumeTick();
  assert.deepEqual(actions.at(-1), { type: "setVolume", volume: 0.4 });
  app.render().setVolume(0.7); app.render().setVolume(0.8);
  app.volumeTick(); assert.equal(actions.length, 2);
  completions[0]({ ...paused, revision: 2, volume: 0.4 }); await flush();
  app.volumeTick();
  assert.deepEqual(actions.at(-1), { type: "setVolume", volume: 0.8 });
  completions[1]({ ...paused, revision: 3, volume: 0.8 }); await flush();
  const result = app.render();
  assert.equal(result.volume, 0.8); assert.equal(result.isVolumeBusy, false);
  assert.equal(result.positionSeconds, 20); assert.equal(result.status, "paused");
  assert.equal(result.sourceId, 1); assert.equal(result.isBusy, false);
});

test("restored mute and preferred volume are native state, and volume clamps before submission", async () => {
  const actions = []; let state = { ...snapshot(1), volume: 0.42, muted: true };
  const app = workflow(action => {
    actions.push(action);
    if (action.type === "setMuted") state = { ...state, revision: 2, muted: action.muted };
    if (action.type === "setVolume") state = { ...state, revision: state.revision + 1, volume: action.volume, muted: false };
    return Promise.resolve(state);
  });
  app.render(); await flush();
  assert.equal(app.render().muted, true); assert.equal(app.render().volume, 0.42);
  assert.deepEqual(actions.map(a => a.type), ["status"]);
  app.render().setMuted(false); app.volumeTick(); await flush();
  assert.equal(app.render().volume, 0.42); assert.equal(app.render().muted, false);
  app.render().setVolume(Infinity); app.volumeTick(); assert.equal(actions.length, 2);
  app.render().setVolume(10); app.volumeTick(); await flush(); assert.equal(app.render().volume, 1);
  app.render().setVolume(-1); app.volumeTick(); await flush(); assert.equal(app.render().volume, 0);
});

test("volume RPC failure restores acknowledged volume and polling allows recovery", async () => {
  const saved = { ...snapshot(1, "playing", track), volume: 0.5 };
  let fail = true;
  const app = workflow(action => action.type === "status" ? Promise.resolve(saved) : fail
    ? Promise.reject(new Error("Worker unavailable")) : Promise.resolve({ ...saved, revision: 2, volume: action.volume }));
  app.render(); await flush(); app.render().setVolume(0.2); app.volumeTick(); await flush();
  assert.equal(app.render().volume, 0.5); assert.equal(app.render().isVolumeBusy, false);
  assert.equal(app.render().isReady, false); assert.match(app.render().error, /Volume request failed/);
  app.render().setVolume(0.3); app.volumeTick(); // Unavailable engine cannot receive a slider request.
  app.poll(); await flush(); assert.equal(app.render().isReady, true);
  fail = false; app.render().setVolume(0.3); app.volumeTick(); await flush();
  assert.equal(app.render().volume, 0.3); assert.equal(app.render().error, null);
});

test("volume changes wait for an outstanding track transition rather than dropping the final slider value", async () => {
  let completePlay; const actions = [];
  const app = workflow(action => {
    actions.push(action);
    if (action.type === "status") return Promise.resolve(snapshot(1, "playing", track));
    if (action.type === "play") return new Promise(resolve => { completePlay = resolve; });
    return Promise.resolve({ ...snapshot(3, "playing", { ...track, id: "second" }), sourceId: 2, volume: action.volume });
  });
  app.render(); await flush();
  const request = app.render().playTrack("second");
  app.render().setVolume(0.25); app.volumeTick();
  assert.equal(actions.length, 2);
  completePlay({ ...snapshot(2, "playing", { ...track, id: "second" }), sourceId: 2 }); await request;
  app.volumeTick(); await flush();
  assert.equal(app.render().volume, 0.25); assert.equal(app.render().track.id, "second");
});


test("collection starts send the complete context, and navigation keeps source guards", async () => {
  const actions = [];
  const app = workflow(action => {
    actions.push(action);
    return Promise.resolve(snapshot(actions.length, action.type === "status" ? "stopped" : "playing", action.type === "status" ? null : track));
  });
  app.render(); await flush();
  await app.render().playTrack(track.id, { liked: true, trackIds: [track.id, "second"] });
  assert.deepEqual(actions.at(-1), { type: "playCollection", trackId: track.id, liked: true, trackIds: [track.id, "second"] });
  await app.render().nextTrack();
  assert.deepEqual(actions.at(-1), { type: "next", sourceId: 1 });
  await app.render().playTrack(track.id, { liked: false, trackIds: [track.id, "unliked", "second"] });
  assert.equal(actions.at(-1).liked, false);
});
