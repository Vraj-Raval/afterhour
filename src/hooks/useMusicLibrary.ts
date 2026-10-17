import { useEffect, useRef, useState } from "react";
import { Channel, invoke } from "@tauri-apps/api/core";
import type { MusicFolder } from "../library/musicFolders";
import type { MusicTrack, ScanProgress, ScanResult } from "../library/types";
import type { LibrarySnapshot } from "../library/types";
import { musicFolderFromPath } from "../library/musicFolders";

export function useMusicLibrary(folders: readonly MusicFolder[], snapshot: LibrarySnapshot | null = null, onFoldersRestored?: (folders: MusicFolder[]) => void) {
  const [tracks, setTracks] = useState<MusicTrack[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [isCancelling, setIsCancelling] = useState(false);
  const [progress, setProgress] = useState<ScanProgress>({ discovered: 0, skipped: 0 });
  const [summary, setSummary] = useState("");
  const [error, setError] = useState("");
  const [lastRoots, setLastRoots] = useState<string | null>(null);
  const [restoredSnapshot, setRestoredSnapshot] = useState<LibrarySnapshot | null>(null);
  const running = useRef(false);
  const savingFavorite = useRef(false);
  const [isSavingFavorite, setIsSavingFavorite] = useState(false);
  const signature = folders.map((folder) => folder.key).sort().join("\0");

  useEffect(() => {
    if (!snapshot) return;
    setTracks(snapshot.tracks);
    setLastRoots(snapshot.scannedPaths.length || snapshot.tracks.length ? snapshot.scannedPaths.map((path) => musicFolderFromPath(path).key).sort().join("\0") : null);
    setSummary(snapshot.tracks.length ? `${snapshot.tracks.length} saved tracks restored. ${snapshot.tracks.filter((track) => !track.available).length} unavailable files.` : "");
    setRestoredSnapshot(snapshot);
  }, [snapshot]);

  async function scanMusic() {
    if (running.current || savingFavorite.current || !folders.length) return;
    running.current = true;
    setIsScanning(true); setIsCancelling(false); setError(""); setSummary("");
    setProgress({ discovered: 0, skipped: 0 });
    const channel = new Channel<ScanProgress>();
    channel.onmessage = (next) => { if (running.current) setProgress(next); };
    try {
      const result = await invoke<ScanResult>("scan_music", { roots: folders.map((folder) => folder.path), onProgress: channel });
      if (result.cancelled) {
        setSummary("Scan cancelled. Your previous library has been kept.");
      } else {
        setTracks(result.tracks); setLastRoots(result.scannedPaths.map((path) => musicFolderFromPath(path).key).sort().join("\0"));
        onFoldersRestored?.(result.folders);
        setSummary(`${result.discovered} supported files discovered. ${result.tracks.length} tracks loaded (${result.tracks.filter((track) => !track.available).length} unavailable); ${result.skipped} unreadable or malformed files skipped. ${result.warnings} folder or file access warnings; ${result.recoveredTags} files recovered without tags.${result.limited ? " Scan stopped at the 100,000-file per-scan limit." : ""}`);
      }
      setProgress({ discovered: result.discovered, skipped: result.skipped });
    } catch (cause) {
      const detail = cause instanceof Error ? cause.message : String(cause);
      setError(`Could not scan your music folders. Check that they are connected and accessible, then try again. ${detail}`);
    } finally {
      running.current = false; channel.onmessage = () => {};
      setIsScanning(false); setIsCancelling(false);
    }
  }

  async function toggleFavorite(track: MusicTrack) {
    if (running.current || savingFavorite.current || snapshot !== restoredSnapshot) return;
    savingFavorite.current = true;
    setIsSavingFavorite(true);
    setError("");
    try {
      const favorite = await invoke<boolean>("set_track_favorite", { id: track.id, favorite: !track.favorite });
      setTracks((current) => current.map((item) => item.id === track.id ? { ...item, favorite } : item));
    } catch (cause) {
      const detail = cause instanceof Error ? cause.message : String(cause);
      setError(`Could not save your favorite. Please try again. ${detail}`);
    } finally {
      savingFavorite.current = false;
      setIsSavingFavorite(false);
    }
  }

  async function cancelScan() {
    if (!running.current || isCancelling) return;
    setIsCancelling(true);
    try { await invoke("cancel_music_scan"); }
    catch { setError("Could not cancel the scan. It may still be running."); setIsCancelling(false); }
  }

  return { toggleFavorite, isSavingFavorite, tracks, isScanning, isCancelling, progress, summary, error, hasScanned: lastRoots !== null, isRestoring: snapshot !== restoredSnapshot,
    foldersChanged: lastRoots !== null && lastRoots !== signature, scanMusic, cancelScan };
}
