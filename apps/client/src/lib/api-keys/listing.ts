// How one API key reads in the list: its scope, its dates and whether it has expired (INT-003, UI-003).
import type { ApiKey } from './api';

const SCOPE_LABELS: Record<ApiKey['scope'], string> = { READ: 'Read', WRITE: 'Write' };

export function scopeLabel(scope: ApiKey['scope']): string {
  return SCOPE_LABELS[scope];
}

/** Whether the key has stopped working at `now`. A date the client cannot read counts as not expired. */
export function isExpired(key: Pick<ApiKey, 'expiresAt'>, now: Date): boolean {
  return Date.parse(key.expiresAt) <= now.getTime();
}

/** A date for the operator, for example "1 Jun 2027"; a dash for a date the client cannot read. */
export function shownDate(iso: string, locale?: string): string {
  const at = new Date(iso);
  return Number.isNaN(at.getTime()) ? '—' : at.toLocaleDateString(locale, { dateStyle: 'medium' });
}

/** "Never" for a key no program has used yet. */
export function lastUsedText(key: Pick<ApiKey, 'lastUsedAt'>, locale?: string): string {
  return key.lastUsedAt === null ? 'Never' : shownDate(key.lastUsedAt, locale);
}
