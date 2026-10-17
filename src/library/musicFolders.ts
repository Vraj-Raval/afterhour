export type MusicFolder = { path: string; name: string; key: string; available?: boolean };

// Compare Windows picker paths without touching the filesystem. This does not
// resolve junctions or symlinks; that belongs to future library scanning.
export function musicFolderFromPath(path: string): MusicFolder {
  const normalized = path.replace(/\//g, "\\").replace(/\\+$/, "");
  return {
    path,
    name: normalized.split("\\").pop() || path,
    key: normalized.toLowerCase(),
  };
}
