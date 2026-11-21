import type { SessionRow } from '../api';
import { shortDay } from './shape';

// What the list of sessions draws (AIS-025, AIS-026). Pure: the same rows and time give the same words.

const MINUTE_MS = 60_000;

/** The last use of a session in words, by the time of `now`: "12 minutes ago", "3 hours ago", "Yesterday", "18 Nov". */
export function lastUseText(lastUseAt: string, now: number): string {
  const at = Date.parse(lastUseAt);
  const minutes = Math.max(0, Math.floor((now - at) / MINUTE_MS));
  if (minutes < 1) return 'Just now';
  if (minutes < 60) return `${String(minutes)} ${minutes === 1 ? 'minute' : 'minutes'} ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${String(hours)} ${hours === 1 ? 'hour' : 'hours'} ago`;
  if (hours < 48) return 'Yesterday';
  return shortDay(lastUseAt.slice(0, 10));
}

/** How long a session ran from its start to its last use: "45 min", "2 h 10 min". */
export function spanText(startedAt: string, lastUseAt: string): string {
  const minutes = Math.max(0, Math.round((Date.parse(lastUseAt) - Date.parse(startedAt)) / MINUTE_MS));
  if (minutes < 60) return `${String(minutes)} min`;
  const rest = minutes % 60;
  return rest === 0 ? `${String(Math.floor(minutes / 60))} h` : `${String(Math.floor(minutes / 60))} h ${String(rest)} min`;
}

/** A session is recent when its last use is at most 30 minutes before now: two count periods of the daemon (AIS-026). */
const RECENT_MS = 30 * MINUTE_MS;

/** Whether `row` was used in the 30 minutes before `now`. Worked out here, so a page left open does not keep a stale mark. */
export function isRecent(row: Pick<SessionRow, 'lastUseAt'>, now: number): boolean {
  return now - Date.parse(row.lastUseAt) <= RECENT_MS;
}

/** The repository without `host` (`github.com/`, from `sharedHost`) when it starts with it. */
export function repositoryName(repository: string | null, host: string): string {
  if (repository === null) return 'No repository';
  return host !== '' && repository.startsWith(host) ? repository.slice(host.length) : repository;
}
