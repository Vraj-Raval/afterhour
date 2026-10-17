import type { ReactNode } from "react";
import type { MusicTrack, ArtworkRef } from "../library/types";
import { collectionDuration, totalDuration } from "../library/presentation";
import { AlbumArtwork } from "./AlbumArtwork";
import { PlayerIcon, type PlayerIconName } from "./PlayerIcon";

type Props = { kind: string; title: string; subtitle?: string; artwork?: ArtworkRef | null; icon?: PlayerIconName; tracks: readonly MusicTrack[]; onPlay?: () => void; disabled?: boolean; children?: ReactNode };
export function CollectionHeader({ kind, title, subtitle, artwork, icon, tracks, onPlay, disabled, children }: Props) {
  const duration = totalDuration(tracks);
  const year = kind === "Album" ? tracks.find(track => track.year)?.year : null;
  return <>
    <header className="collection-header">
      <div className={`collection-cover${kind === "Liked Songs" ? " liked-cover" : ""}`}>{icon ? <PlayerIcon name={icon} /> : <AlbumArtwork artwork={artwork ?? null} />}</div>
      <div className="collection-heading"><span className="eyebrow">{kind}</span><h1 title={title}>{title}</h1><p>{[subtitle, year, `${tracks.length} ${tracks.length === 1 ? "song" : "songs"}`, duration > 0 ? collectionDuration(duration) : null].filter(Boolean).join(" / ")}</p></div>
    </header>
    <div className="collection-toolbar">{onPlay && <button type="button" className="primary-play" aria-label={`Play ${title}`} disabled={disabled || !tracks.some(track => track.available)} onClick={onPlay}><PlayerIcon name="play" /><span>Play</span></button>}{children}</div>
  </>;
}
