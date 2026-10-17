export type Album = {
  id: string;
  title: string;
  artist: string;
  year: number;
  artwork: "amber" | "slate" | "clay" | "olive";
};

export type Track = {
  id: string;
  albumId: Album["id"];
  title: string;
  durationSeconds: number;
  playedLabel: string;
};

// Fictional preview records. No listening history or local files are read.
export const albums: readonly Album[] = [
  { id: "01", title: "The quiet between", artist: "Mira Sol", year: 2024, artwork: "amber" },
  { id: "02", title: "Blue hour", artist: "North Atlantic", year: 2023, artwork: "slate" },
  { id: "03", title: "Slow motion", artist: "June & the City", year: 2025, artwork: "clay" },
  { id: "04", title: "Still, here", artist: "Oren Vale", year: 2024, artwork: "olive" },
];

export const featuredAlbum = albums[0];

export const recentTracks: readonly Track[] = [
  { id: "t1", albumId: "01", title: "Light through the curtains", durationSeconds: 258, playedLabel: "Just now" },
  { id: "t2", albumId: "02", title: "Somewhere, a coastline", durationSeconds: 312, playedLabel: "12 min ago" },
  { id: "t3", albumId: "03", title: "Nothing in a hurry", durationSeconds: 226, playedLabel: "Yesterday" },
  { id: "t4", albumId: "04", title: "A room of our own", durationSeconds: 284, playedLabel: "Yesterday" },
];
