import type { PauseReason, SourceAccount, SourceHealth } from './api';

/** One health state in words (SRC-009): a short name and one sentence on what it means and what to do. */
export interface HealthWords {
  readonly name: string;
  readonly meaning: string;
}

const PAUSE_MEANING: Record<PauseReason, string> = {
  OPERATOR: 'You paused it. Nothing is read until you resume.',
  EXTENSION_OFF: 'The extension that uses it is off. It reads again when that extension is on.',
};

const MEANING: Record<Exclude<SourceHealth, 'PAUSED'>, HealthWords> = {
  OK: { name: 'OK', meaning: 'It reads on schedule.' },
  STARTING: { name: 'Starting', meaning: 'The first sync is running.' },
  STALLED: { name: 'Stalled', meaning: 'Syncs keep failing or have stopped. The hub tries again.' },
  NEVER_SYNCED: { name: 'Never synced', meaning: 'No sync has finished yet.' },
  NEEDS_RECONNECT: { name: 'Needs reconnect', meaning: 'The source refused the key. Disconnect, then connect again with a working key.' },
};

/** The words for the account's health; a paused account also says who paused it. */
export function healthWords(account: Pick<SourceAccount, 'health' | 'pausedBy'>): HealthWords {
  if (account.health !== 'PAUSED') return MEANING[account.health];
  return { name: 'Paused', meaning: PAUSE_MEANING[account.pausedBy ?? 'OPERATOR'] };
}

function plural(count: number, unit: string): string {
  return `${String(count)} ${unit}${count === 1 ? '' : 's'}`;
}

/** How long ago `iso` was, for example "4 min ago"; "at an unknown time" for a time the client cannot read. */
export function agoText(iso: string, now: Date): string {
  const seconds = Math.floor((now.getTime() - Date.parse(iso)) / 1000);
  if (Number.isNaN(seconds)) return 'at an unknown time';
  if (seconds < 90) return `${String(Math.max(0, seconds))} s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 90) return `${String(minutes)} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 48) return `${String(hours)} h ago`;
  return `${plural(Math.round(hours / 24), 'day')} ago`;
}

/** The last successful sync (ITG-002), for example "Synced 4 min ago". */
export function lastSyncText(lastSyncAt: string | null, now: Date): string {
  return lastSyncAt === null ? 'Never synced' : `Synced ${agoText(lastSyncAt, now)}`;
}
