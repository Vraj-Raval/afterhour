export type PlayerIconName = "previous" | "next" | "play" | "pause" | "stop" | "shuffle" | "repeat" | "volume" | "muted" | "heart" | "home" | "library" | "search" | "back" | "forward" | "plus" | "close" | "list" | "grid" | "queue" | "panel" | "album" | "artist" | "music" | "folder" | "more" | "up" | "down";

const paths: Record<PlayerIconName, string> = {
  home: "m3 10 9-7 9 7v11h-6v-7H9v7H3Z",
  library: "M4 3v18 M9 3v18 M14 3l6 18 M3 4h3 M8 4h3",
  search: "M21 21l-6-6 M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0Z",
  back: "m14 5-7 7 7 7", forward: "m10 5 7 7-7 7",
  plus: "M12 4v16 M4 12h16", close: "m5 5 14 14 M19 5 5 19",
  list: "M8 5h13 M8 12h13 M8 19h13 M3 5h1 M3 12h1 M3 19h1",
  grid: "M3 3h7v7H3Z M14 3h7v7h-7Z M3 14h7v7H3Z M14 14h7v7h-7Z",
  queue: "M3 5h18 M3 11h18 M3 17h9 m4-2 6 4-6 4Z",
  panel: "M3 3h18v18H3Z M15 3v18", album: "M3 3h18v18H3Z M17 12a5 5 0 1 1-10 0 5 5 0 0 1 10 0Z M12 12h.01",
  artist: "M16 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z M4 21v-2a8 8 0 0 1 16 0v2",
  music: "M9 18V5l12-2v13 M9 9l12-2 M9 18a3 3 0 1 1-3-3h3 M21 16a3 3 0 1 1-3-3h3",
  folder: "M3 5h7l2 3h9v13H3Z", more: "M4 12h.01 M12 12h.01 M20 12h.01",
  up: "m6 14 6-6 6 6", down: "m6 10 6 6 6-6",
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
