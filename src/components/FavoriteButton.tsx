import type { MusicTrack } from "../library/types";
import { PlayerIcon } from "./PlayerIcon";

type Props = { track: MusicTrack; disabled: boolean; onToggle: (track: MusicTrack) => Promise<void> };
export function FavoriteButton({ track, disabled, onToggle }: Props) {
  const label = `${track.favorite ? "Remove" : "Add"} ${track.title} ${track.favorite ? "from" : "to"} Liked Songs`;
  return <button type="button" className="icon-button favorite-button" aria-pressed={track.favorite} aria-label={label} title={label} disabled={disabled}
    onClick={(event) => { event.stopPropagation(); void onToggle(track); }}><PlayerIcon name="heart" /></button>;
}
