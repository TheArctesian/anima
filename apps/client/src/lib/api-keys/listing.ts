// How one API key reads in the list: its scope, its dates and whether it has expired (INT-003, UI-003).
import type { ApiKey } from './api';

const SCOPE_LABELS: Record<ApiKey['scope'], string> = { READ: 'Read', WRITE: 'Write' };

export function scopeLabel(scope: ApiKey['scope']): string {
  return SCOPE_LABELS[scope];
}

/** Who uses the key, in words (LOG-015). */
const MADE_FOR_LABELS: Record<ApiKey['madeFor'], string> = { USER: 'A script I run', AGENT: 'An AI agent' };

export function madeForLabel(madeFor: ApiKey['madeFor']): string {
  return MADE_FOR_LABELS[madeFor];
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

/**
 * A key of typical length that never reaches the screen as text: the loading skeleton and the space the list keeps
 * are drawn from it, so they have a real row's geometry at every width (MOT-007).
 */
export const PLACEHOLDER_KEY: ApiKey = {
  id: 'placeholder',
  name: 'placeholder-key',
  scope: 'WRITE',
  madeFor: 'AGENT',
  prefix: 'anima_AAAAAA',
  expiresAt: '2099-12-31T00:00:00Z',
  createdAt: '2099-12-31T00:00:00Z',
  lastUsedAt: '2099-12-31T00:00:00Z',
};

/**
 * Where focus goes after the key at `index` was revoked: the id of the key that took its place, else the key before
 * it, else `null` for "Create API key". `keys` is the list without the revoked key.
 */
export function focusAfterRevoke(keys: readonly Pick<ApiKey, 'id'>[], index: number): string | null {
  return (keys[index] ?? keys[index - 1])?.id ?? null;
}
