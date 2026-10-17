import { useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { musicFolderFromPath, type MusicFolder } from "../library/musicFolders";
import type { LibrarySnapshot } from "../library/types";

export function useMusicFolders() {
  const [folders, setFolders] = useState<MusicFolder[]>([]);
  const [isSelecting, setIsSelecting] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [snapshot, setSnapshot] = useState<LibrarySnapshot | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState("");
  const restoring = useRef(false);
  const selecting = useRef(false);
  const selectedKeys = useRef(new Set<string>());

  function updateFolders(saved: MusicFolder[]) {
    // A picker may finish while a scan snapshot is returning; keep newer selections.
    const keys = new Set(saved.map((folder) => folder.key));
    setFolders((current) => [...saved, ...current.filter((folder) => !keys.has(folder.key))]);
    saved.forEach((folder) => selectedKeys.current.add(folder.key));
  }

  async function restoreLibrary() {
    if (restoring.current) return;
    restoring.current = true;
    setIsLoading(true); setLoadError("");
    try {
      if (!isTauri()) {
        setLoadError("Open the Afterhours desktop app to load your saved library.");
        return;
      }
      const saved = await invoke<LibrarySnapshot>("load_library");
      updateFolders(saved.folders);
      setSnapshot(saved);
    } catch (cause) {
      setLoadError(`Could not load your saved library. Your database has not been reset. ${cause instanceof Error ? cause.message : String(cause)}`);
    } finally { restoring.current = false; setIsLoading(false); }
  }

  useEffect(() => { void restoreLibrary(); }, []);

  async function addMusicFolder() {
    if (selecting.current || restoring.current || loadError) return;
    setError("");
    setMessage("");
    if (!isTauri()) {
      setError("Open Afterhours in the desktop app to select a music folder. The browser preview cannot open the native picker.");
      return;
    }
    selecting.current = true;
    setIsSelecting(true);
    try {
      // Native wrapper records the picker selection before authorizing scanning.
      const path = await invoke<string | null>("select_music_folder");
      if (path === null) return;
      const folder = musicFolderFromPath(path);
      if (selectedKeys.current.has(folder.key)) {
        setMessage(`${folder.name} is already selected.`);
        return;
      }
      selectedKeys.current.add(folder.key);
      setFolders((current) => [...current, folder]);
      setMessage(`${folder.name} saved. Scan Music to update your library.`);
    } catch (cause) {
      const detail = cause instanceof Error ? cause.message : typeof cause === "string" ? cause : "";
      setError(`Could not open the music folder picker. Please try again.${detail ? ` Details: ${detail}` : " If this continues, restart the desktop app."}`);
    } finally {
      selecting.current = false;
      setIsSelecting(false);
    }
  }

  return { folders, isSelecting, message, error, addMusicFolder, snapshot, isLoading, loadError, restoreLibrary, updateFolders };
}
