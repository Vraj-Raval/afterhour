export type Playlist = { id: string; name: string; createdAt: number; updatedAt: number; trackIds: string[] };
export type PlaylistAction =
  | { type: "list" }
  | { type: "create"; name: string }
  | { type: "rename"; playlistId: string; name: string }
  | { type: "delete"; playlistId: string }
  | { type: "add" | "remove"; playlistId: string; trackId: string };
