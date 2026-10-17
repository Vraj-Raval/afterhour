type PlayerIconName = "previous" | "next" | "play" | "pause" | "stop" | "shuffle" | "repeat" | "volume" | "muted" | "heart";

const paths: Record<PlayerIconName, string> = {
  previous: "M6 5v14 M19 5 8 12l11 7Z",
  next: "M18 5v14 M5 5l11 7-11 7Z",
  play: "m8 5 11 7-11 7Z",
  pause: "M8 5v14 M16 5v14",
  stop: "M6 6h12v12H6Z",
  shuffle: "M3 6h3c5 0 7 12 12 12h3 M17 14l4 4-4 4 M3 18h3c2 0 3-2 4-4 M14 8c1-1 2-2 4-2h3 M17 2l4 4-4 4",
  repeat: "m17 2 4 4-4 4 M21 6H7a4 4 0 0 0-4 4 M7 22l-4-4 4-4 M3 18h14a4 4 0 0 0 4-4",
  volume: "M4 9h4l5-4v14l-5-4H4Z M16 8a6 6 0 0 1 0 8 M19 5a10 10 0 0 1 0 14",
  muted: "M4 9h4l5-4v14l-5-4H4Z M17 9l5 6 M22 9l-5 6",
  heart: "M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.7l-1.1-1.1a5.5 5.5 0 0 0-7.8 7.8L12 21l8.8-8.6a5.5 5.5 0 0 0 0-7.8Z",
};

export function PlayerIcon({ name }: { name: PlayerIconName }) {
  return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]} /></svg>;
}
