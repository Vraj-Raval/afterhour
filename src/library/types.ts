export type ArtworkRef = { id: string; mime: string };
export type MusicTrack = {
  id: string;
  path: string;
  title: string;
  artist: string;
  album: string;
  albumArtist: string | null;
  durationSeconds: number | null;
  trackNumber: number | null;
  discNumber: number | null;
  year: number | null;
  format: string;
  artwork: ArtworkRef | null;
  available: boolean;
  favorite: boolean;
};
export type ScanProgress = { discovered: number; skipped: number };
export type ScanResult = ScanProgress & {
  tracks: MusicTrack[];
  warnings: number;
  recoveredTags: number;
  cancelled: boolean;
  limited: boolean;
  scannedPaths: string[];
  folders: import("./musicFolders").MusicFolder[];
};

export type LibrarySnapshot = {
  folders: import("./musicFolders").MusicFolder[];
  tracks: MusicTrack[];
  scannedPaths: string[];
};
