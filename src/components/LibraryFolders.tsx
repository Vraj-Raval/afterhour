import type { useMusicFolders } from "../hooks/useMusicFolders";

export function LibraryFolders({ folders, isSelecting, message, error, addMusicFolder, isLoading, loadError }: ReturnType<typeof useMusicFolders>) {
  return (
    <section className="library-status" aria-labelledby="local-library-heading">
      <h2 id="local-library-heading" className="eyebrow">Local library</h2>
      <p className="folder-count">{isLoading ? "Loading folders..." : `${folders.length} folder${folders.length === 1 ? "" : "s"} saved`}</p>
      <button type="button" className="add-music-folder" disabled={isSelecting || isLoading || !!loadError} aria-busy={isSelecting} onClick={() => void addMusicFolder()}>{isSelecting ? "Choosing folder..." : "Add Music Folder"}</button>
      {folders.length > 0 && <ul className="music-folders">{folders.map((folder) => <li key={folder.key}>
        <span className="folder-name" title={folder.path}>{folder.name}</span>
        <span className="folder-path">{folder.path}</span>
        {folder.available === false && <span className="folder-unavailable">Unavailable. Reconnect the folder, then reload or scan again.</span>}
      </li>)}</ul>}
      <p className="folder-help">Saved on this device. Use Scan Music to update your library.</p>
      <p className="folder-message" role="status">{message}</p>
      {error && <p className="folder-error" role="alert">{error}</p>}
    </section>
  );
}
