export function formatPlaybackTime(seconds: number | null): string {
  if (seconds === null || !Number.isFinite(seconds)) return "--:--";
  const whole = Math.floor(Math.max(0, seconds));
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, "0")}`;
}

export function clampPosition(seconds: number, duration: number | null): number {
  if (!Number.isFinite(seconds) || duration === null || !Number.isFinite(duration) || duration <= 0) return 0;
  return Math.max(0, Math.min(seconds, duration));
}
