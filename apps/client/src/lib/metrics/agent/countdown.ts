/** Whole seconds until `expiresAt`, never below zero; an unreadable time counts as expired. `now` is a parameter (MET-012). */
export function secondsLeft(expiresAt: string, now: Date): number {
  const end = Date.parse(expiresAt);
  if (Number.isNaN(end)) return 0;
  return Math.max(0, Math.ceil((end - now.getTime()) / 1000));
}

/** `14:05` for 845 seconds. */
export function clockText(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  return `${String(minutes)}:${String(seconds % 60).padStart(2, '0')}`;
}

/** MET-012: a code works for 15 minutes. */
const CODE_LIFE_MS = 15 * 60 * 1000;

/** The time to clear a code shown at `shownAt`: the hub's `expiresAt` or 15 minutes later, whichever comes first. An unreadable time is `shownAt`. */
export function clearTime(expiresAt: string, shownAt: Date): string {
  const hub = Date.parse(expiresAt);
  const capped = Number.isNaN(hub) ? shownAt.getTime() : Math.min(hub, shownAt.getTime() + CODE_LIFE_MS);
  return new Date(capped).toISOString();
}
