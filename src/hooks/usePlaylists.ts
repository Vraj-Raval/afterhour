import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Playlist, PlaylistAction } from "../library/playlists";

export function usePlaylists() {
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [isBusy, setIsBusy] = useState(false);
  const [isLoaded, setIsLoaded] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const pending = useRef(false);
  async function run(action: PlaylistAction, message = "") {
    if (pending.current) return null;
    if ((action.type === "create" || action.type === "rename") && !action.name.trim()) {
      setError("Enter a playlist name."); return null;
    }
    pending.current = true; setIsBusy(true); setError(""); setMessage("");
    try {
      const result = await invoke<Playlist[]>("playlist_control", { action });
      setPlaylists(result); setIsLoaded(true); setMessage(message);
      return result;
    } catch (cause) {
      setError(`Could not update playlists. Please try again. ${cause instanceof Error ? cause.message : String(cause)}`);
      return null;
    } finally { pending.current = false; setIsBusy(false); }
  }
  useEffect(() => { void run({ type: "list" }); }, []);
  async function create(name: string) {
    const existing = new Set(playlists.map(p => p.id));
    const result = await run({ type: "create", name: name.trim() }, "Playlist created.");
    return result?.find(p => !existing.has(p.id)) ?? null;
  }
  return { playlists, isBusy, isLoaded, error, message, create,
    reload: () => run({ type: "list" }),
    rename: (playlistId: string, name: string) => run({ type: "rename", playlistId, name: name.trim() }, "Playlist renamed."),
    deletePlaylist: (playlistId: string) => run({ type: "delete", playlistId }, "Playlist deleted."),
    add: (playlistId: string, trackId: string) => run({ type: "add", playlistId, trackId }, "Song saved to playlist."),
    remove: (playlistId: string, trackId: string) => run({ type: "remove", playlistId, trackId }, "Song removed from playlist.") };
}
