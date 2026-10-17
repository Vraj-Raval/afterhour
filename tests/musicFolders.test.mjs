import assert from "node:assert/strict";
import test from "node:test";
import { musicFolderFromPath } from "../src/library/musicFolders.ts";

test("keeps the selected path and derives a readable folder name", () => {
  const path = String.raw`D:\Audio\Evening records`;
  assert.deepEqual(musicFolderFromPath(path), {
    path, name: "Evening records", key: String.raw`d:\audio\evening records`,
  });
});

test("Windows path case, slash direction, and trailing separators do not create duplicates", () => {
  const paths = ["D:/Audio/Records", "d:\\audio\\records\\", "D:\\AUDIO\\RECORDS"];
  assert.equal(new Set(paths.map((path) => musicFolderFromPath(path).key)).size, 1);
});

test("supports drive roots, UNC paths, and Unicode folder names", () => {
  assert.equal(musicFolderFromPath("D:\\").name, "D:");
  assert.equal(musicFolderFromPath("\\\\server\\share\\Music\\").name, "Music");
  assert.equal(musicFolderFromPath("D:\\音楽").name, "音楽");
});

test("keeps different folders distinct", () => {
  assert.notEqual(musicFolderFromPath("D:/Music").key, musicFolderFromPath("D:/Music live").key);
});
