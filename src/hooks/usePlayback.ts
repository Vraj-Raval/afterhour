import { useCallback, useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import type { PlaybackAction, PlaybackSnapshot, PlaybackCollection } from "../playback/types";

const idle: PlaybackSnapshot = { revision: 0, volume: 1, muted: false, preferencesError: null, sourceId: 0, positionSeconds: 0, durationSeconds: null, canSeek: false, status: "stopped", track: null, error: null };

export function usePlayback() {
  const [snapshot, setSnapshot] = useState<PlaybackSnapshot>(idle);
  const [isBusy, setIsBusy] = useState(false);
  const [isReady, setIsReady] = useState(false);
  const [requestError, setRequestError] = useState("");
  const pending = useRef(false);
  const [isVolumeBusy, setIsVolumeBusy] = useState(false);
  const volumeRequest = useRef<PlaybackAction | null>(null);
  const volumeRunning = useRef(false);
  const volumeTimer = useRef<number | null>(null);
  const ready = useRef(false);
  const disposedVolume = useRef(false);
  ready.current = isReady;

  function accept(next: PlaybackSnapshot) {
    // A delayed status response must not undo a newer play/pause response.
    setSnapshot((current) => next.revision > current.revision ? next : current);
    setIsReady(true);
  }

  useEffect(() => {
    disposedVolume.current = false;
    if (!isTauri()) return;
    let disposed = false;
    let polling = false;
    async function refresh() {
      if (polling || pending.current) return;
      polling = true;
      try {
        const next = await invoke<PlaybackSnapshot>("playback_control", { action: { type: "status" } });
        if (!disposed) { accept(next); setRequestError(""); }
      } catch (cause) {
        if (!disposed) { setIsReady(false); setRequestError(`Cannot read playback state. ${String(cause)}`); }
      } finally { polling = false; }
    }
    void refresh();
    // Native measurements only; no independently advancing frontend clock.
    const interval = window.setInterval(() => void refresh(), 250);
    return () => { disposed = true; disposedVolume.current = true; window.clearInterval(interval); if (volumeTimer.current !== null) window.clearTimeout(volumeTimer.current); };
  }, []);

  const control = useCallback(async (action: PlaybackAction) => {
    if (pending.current || !isReady) return;
    pending.current = true; setIsBusy(true); setRequestError("");
    try { accept(await invoke<PlaybackSnapshot>("playback_control", { action })); }
    catch (cause) { setIsReady(false); setRequestError(`Playback request failed. ${cause instanceof Error ? cause.message : String(cause)}`); }
    finally { pending.current = false; setIsBusy(false); }
  }, [isReady]);

  // Keep only the latest slider request, with at most one volume RPC in flight.
  function scheduleVolume() {
    if (volumeTimer.current !== null || volumeRunning.current) return;
    volumeTimer.current = window.setTimeout(() => { volumeTimer.current = null; void flushVolume(); }, 100);
  }
  async function flushVolume() {
    if (disposedVolume.current) return;
    if (pending.current) { scheduleVolume(); return; }
    if (!ready.current) { volumeRequest.current = null; setIsVolumeBusy(false); return; }
    const action = volumeRequest.current;
    if (!action) return;
    volumeRequest.current = null; volumeRunning.current = true;
    try {
      const next = await invoke<PlaybackSnapshot>("playback_control", { action });
      if (!disposedVolume.current) { accept(next); setRequestError(""); }
    } catch (cause) {
      if (!disposedVolume.current) { setIsReady(false); setRequestError(`Volume request failed. ${String(cause)}`); }
      volumeRequest.current = null;
    } finally {
      volumeRunning.current = false;
      if (!disposedVolume.current) {
        if (volumeRequest.current) scheduleVolume();
        else setIsVolumeBusy(false);
      }
    }
  }
  function setVolume(volume: number) {
    if (!isReady || !Number.isFinite(volume)) return;
    volumeRequest.current = { type: "setVolume", volume: Math.max(0, Math.min(1, volume)) };
    setIsVolumeBusy(true); scheduleVolume();
  }
  function setMuted(muted: boolean) {
    if (!isReady) return;
    volumeRequest.current = { type: "setMuted", muted };
    setIsVolumeBusy(true); scheduleVolume();
  }

  const playTrack = useCallback((trackId: string, collection?: PlaybackCollection) => control(collection ? { type: "playCollection", trackId, ...collection } : { type: "play", trackId }), [control]);

  function togglePlayback() {
    if (snapshot.status === "playing") return control({ type: "pause" });
    if (snapshot.status === "paused") return control({ type: "resume" });
    if (snapshot.track) return control({ type: "play", trackId: snapshot.track.id });
    return Promise.resolve();
  }

  return { ...snapshot, error: requestError || snapshot.error, isBusy, isReady,
    playTrack, togglePlayback, setVolume, setMuted, isVolumeBusy,
    seek: (seconds: number, sourceId: number) => control({ type: "seek", seconds, sourceId }),
    nextTrack: () => control({ type: "next", sourceId: snapshot.sourceId }),
    previousTrack: () => control({ type: "previous", sourceId: snapshot.sourceId }),
    stopPlayback: () => control({ type: "stop" }) };
}
