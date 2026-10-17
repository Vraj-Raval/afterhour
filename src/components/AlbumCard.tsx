import type { Album } from "../data/home";
import { AlbumArtwork } from "./AlbumArtwork";

export function AlbumCard({ album }: { album: Album }) {
  return (
    <article className="album-card" tabIndex={0} aria-labelledby={`album-title-${album.id}`}>
      <AlbumArtwork album={album} />
      <h3 id={`album-title-${album.id}`} title={album.title}>{album.title}</h3>
      <p className="album-artist" title={album.artist}>{album.artist}</p>
      <span className="album-year">{album.year}</span>
    </article>
  );
}
