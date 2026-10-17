import { useEffect, useRef, useState } from "react";
import type { usePlaylists } from "../hooks/usePlaylists";
import type { MusicTrack } from "../library/types";
import { PlaylistFeedback } from "./PlaylistView";

export function PlaylistPicker({ track, manager, onClose }: { track: MusicTrack; manager: ReturnType<typeof usePlaylists>; onClose: () => void }) {
  const [name, setName] = useState("");
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => { dialog.current?.showModal(); return () => dialog.current?.close(); }, []);
  return <dialog ref={dialog} className="playlist-dialog" aria-labelledby="playlist-picker-title" onCancel={event => { event.preventDefault(); if (!manager.isBusy) onClose(); }}>
    <h2 id="playlist-picker-title">Add to playlist</h2><p className="collection-note">{track.title} / {track.artist}</p>
    <PlaylistFeedback manager={manager} />
    {manager.error && <button disabled={manager.isBusy} onClick={() => void manager.reload()}>Reload playlists</button>}
    <div className="playlist-choices">{manager.playlists.map(playlist => {
      const added = playlist.trackIds.includes(track.id);
      return <button key={playlist.id} disabled={manager.isBusy || added} onClick={async () => { if (await manager.add(playlist.id, track.id)) onClose(); }}><span>{playlist.name}</span><span>{added ? "Added" : "+ Add"}</span></button>;
    })}</div>
    {!manager.playlists.length && <p className="collection-note">Create a playlist for this song.</p>}
    <form className="playlist-form" onSubmit={async event => { event.preventDefault(); const created = await manager.create(name); if (created) { setName(""); if (await manager.add(created.id, track.id)) onClose(); } }}>
      <label>New playlist name<input value={name} onChange={event => setName(event.currentTarget.value)} required /></label><button disabled={manager.isBusy || !manager.isLoaded || !name.trim()}>Create &amp; add</button>
    </form><button disabled={manager.isBusy} onClick={onClose}>Close</button>
  </dialog>;
}
