import type { useMusicFolders } from "../hooks/useMusicFolders";
import { PlayerIcon } from "./PlayerIcon";
export function LibraryFolders({ folders, isSelecting, message, error, addMusicFolder, isLoading, loadError }: ReturnType<typeof useMusicFolders>) {
  return <section className="library-status" aria-label="Local music folders"><details><summary><PlayerIcon name="folder" /><span>Music folders</span><span className="folder-count">{folders.length}</span></summary>
    <button type="button" className="add-music-folder" disabled={isSelecting || isLoading || !!loadError} aria-busy={isSelecting} onClick={()=>void addMusicFolder()}><PlayerIcon name="plus" />{isSelecting?"Choosing folder...":"Add Music Folder"}</button>
    {folders.length>0 && <ul className="music-folders">{folders.map(folder=><li key={folder.key}><span className="folder-name" title={folder.path}>{folder.name}</span><span className="folder-path" title={folder.path}>{folder.path}</span>{folder.available===false && <span className="folder-unavailable">Unavailable. Reconnect this folder.</span>}</li>)}</ul>}
    <p className="folder-help">Use Scan Music in Library to update your collection.</p></details>{message && <p className="folder-message" role="status">{message}</p>}{error && <p className="folder-error" role="alert">{error}</p>}</section>;
}
