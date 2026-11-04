import type { EventFilter, EventSource, EventsQueryVariables, Severity } from '$lib/hub/generated/graphql';
import type { EventPage, LogEvent } from './api';

export type Since = '1h' | '24h' | '7d' | 'all';

/** What the operator chose in the logs panel (LOG-009). */
export interface LogFilter {
  readonly since: Since;
  readonly severities: readonly Severity[];
  readonly sources: readonly EventSource[];
  readonly deviceId: string | null;
}

export const DEFAULT_LOG_FILTER: LogFilter = { since: '24h', severities: [], sources: [], deviceId: null };

/** Rows kept in the panel; the oldest drop first (LOG-010). */
export const LOG_CAP = 500;

const PAGE_SIZE = 100;
const HOUR_MS = 3_600_000;
const WINDOW_MS: Record<Exclude<Since, 'all'>, number> = {
  '1h': HOUR_MS,
  '24h': 24 * HOUR_MS,
  '7d': 168 * HOUR_MS,
};
const RANK: Record<Severity, number> = { INFO: 0, WARNING: 1, ERROR: 2 };

function sinceTime(since: Since, now: Date): Date | null {
  return since === 'all' ? null : new Date(now.getTime() - WINDOW_MS[since]);
}

/** The `events` variables for a filter; empty parts are left out (LOG-009). */
export function eventVariables(filter: LogFilter, now: Date, after?: string | null): EventsQueryVariables {
  const since = sinceTime(filter.since, now);
  const parts: EventFilter = {
    ...(filter.severities.length > 0 && { severity: [...filter.severities] }),
    ...(filter.sources.length > 0 && { source: [...filter.sources] }),
    ...(filter.deviceId !== null && { deviceId: filter.deviceId }),
    ...(since !== null && { since: since.toISOString() }),
  };
  return {
    filter: parts,
    first: PAGE_SIZE,
    ...(after !== undefined && after !== null && { after }),
  };
}

/** The lowest selected severity, or INFO when none is selected: the `eventAdded` argument. */
export function minSeverity(filter: LogFilter): Severity {
  return filter.severities.reduce<Severity>(
    (lowest, severity) => (RANK[severity] < RANK[lowest] ? severity : lowest),
    filter.severities[0] ?? 'INFO',
  );
}

/** Whether `event` belongs in the list the filter describes (LOG-009). */
export function matches(event: LogEvent, filter: LogFilter, now: Date): boolean {
  const since = sinceTime(filter.since, now);
  return (
    (filter.severities.length === 0 || filter.severities.includes(event.severity)) &&
    (filter.sources.length === 0 || filter.sources.includes(event.source)) &&
    (filter.deviceId === null || event.deviceId === filter.deviceId) &&
    (since === null || new Date(String(event.at)).getTime() >= since.getTime())
  );
}

/** The rows on screen, newest first. */
export interface EventLog {
  readonly rows: readonly LogEvent[];
  readonly endCursor: string | null;
  readonly hasNextPage: boolean;
}

function rowsOf(page: EventPage): LogEvent[] {
  return page.edges.map((edge) => edge.node);
}

/** A log made of the first page. */
export function fromPage(page: EventPage): EventLog {
  return {
    rows: rowsOf(page).slice(0, LOG_CAP),
    endCursor: page.pageInfo.endCursor ?? null,
    hasNextPage: page.pageInfo.hasNextPage,
  };
}

/** Add an older page below the rows; ids already shown are skipped (LOG-009). */
export function appendOlder(log: EventLog, page: EventPage): EventLog {
  const known = new Set(log.rows.map((row) => row.id));
  const older = rowsOf(page).filter((row) => !known.has(row.id));
  return {
    rows: [...log.rows, ...older].slice(0, LOG_CAP),
    endCursor: page.pageInfo.endCursor ?? null,
    hasNextPage: page.pageInfo.hasNextPage,
  };
}

/** Put a live event on top; a non-matching or known event leaves the same log (LOG-010). */
export function addLive(log: EventLog, event: LogEvent, filter: LogFilter, now: Date): EventLog {
  if (!matches(event, filter, now) || log.rows.some((row) => row.id === event.id)) return log;
  return { ...log, rows: [event, ...log.rows].slice(0, LOG_CAP) };
}
