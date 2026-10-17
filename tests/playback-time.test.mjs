import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";

const module = { exports: {} };
const source = ts.transpileModule(readFileSync(new URL("../src/playback/time.ts", import.meta.url), "utf8"), { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText;
new Function("exports", source)(module.exports);
const { formatPlaybackTime: format, clampPosition: clamp } = module.exports;

test("playback time formats real seconds, short tracks, hours and unknown duration", () => {
  assert.equal(format(0), "0:00");
  assert.equal(format(0.25), "0:00");
  assert.equal(format(65.9), "1:05");
  assert.equal(format(3601), "60:01");
  assert.equal(format(-10), "0:00");
  assert.equal(format(null), "--:--");
  assert.equal(format(NaN), "--:--");
});

test("progress clamps to known finite boundaries including subsecond tracks", () => {
  assert.equal(clamp(-1, 60), 0);
  assert.equal(clamp(90, 60), 60);
  assert.equal(clamp(0.2, 0.3), 0.2);
  assert.equal(clamp(1, 0.3), 0.3);
  assert.equal(clamp(10, null), 0);
  assert.equal(clamp(Infinity, 60), 0);
  assert.equal(clamp(10, 0), 0);
});
