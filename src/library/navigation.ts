import type { ArtworkRef, MusicTrack } from "./types";

export function cleanMetadata(value: string | null | undefined, fallback = "") {
  return value?.normalize("NFKC").trim().replace(/\s+/gu, " ") || fallback;
}
export function normalizeMetadata(value: string | null | undefined) {
  return cleanMetadata(value).toLowerCase();
}
export function searchTracks(tracks: readonly MusicTrack[], query: string): MusicTrack[] {
  const terms = normalizeMetadata(query).split(" ").filter(Boolean);
  if (!terms.length) return [...tracks];
  return tracks.filter(track => {
    const text = normalizeMetadata(`${track.title} ${track.artist} ${track.album} ${track.albumArtist ?? ""}`);
    return terms.every(term => text.includes(term));
  });
}
export type LibraryGroup = { id: string; name: string; subtitle: string; tracks: MusicTrack[]; artwork: ArtworkRef | null };
const compareText = (a: string, b: string) => a < b ? -1 : a > b ? 1 : 0;
function compareTracks(a: MusicTrack, b: MusicTrack) {
  return (a.discNumber ?? 1) - (b.discNumber ?? 1) || (a.trackNumber ?? Number.MAX_SAFE_INTEGER) - (b.trackNumber ?? Number.MAX_SAFE_INTEGER) || compareText(normalizeMetadata(a.title), normalizeMetadata(b.title)) || compareText(a.id, b.id);
}
export function groupLibrary(tracks: readonly MusicTrack[], kind: "Albums" | "Artists"): LibraryGroup[] {
  const groups = new Map<string, LibraryGroup>();
  for (const track of tracks) {
    const artist = cleanMetadata(track.artist, "Unknown Artist");
    const album = cleanMetadata(track.album, "Unknown Album");
    const albumArtist = cleanMetadata(track.albumArtist, artist);
    const id = kind === "Albums" ? JSON.stringify([normalizeMetadata(album), normalizeMetadata(albumArtist)]) : normalizeMetadata(artist);
    let group = groups.get(id);
    if (!group) {
      group = { id, name: kind === "Albums" ? album : artist, subtitle: kind === "Albums" ? albumArtist : "", tracks: [], artwork: null };
      groups.set(id, group);
    }
    group.tracks.push(track);
    group.artwork ??= track.artwork;
  }
  for (const group of groups.values()) {
    group.tracks.sort((a, b) => (kind === "Artists" ? compareText(normalizeMetadata(a.album), normalizeMetadata(b.album)) : 0) || compareTracks(a, b));
  }
  return [...groups.values()].sort((a,b) => compareText(normalizeMetadata(a.name), normalizeMetadata(b.name)) || compareText(a.id,b.id));
}
