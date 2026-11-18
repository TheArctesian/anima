import type { AgentSource, AgentUsageDimension, AgentUsageFilter } from '$lib/hub/generated/graphql';
import type { UsageRequest } from './api';

/** The page state that lives in the address, so Back, Forward and a copied link restore the page. */
export interface UsageQuery {
  /** How many days, ending today (UTC). */
  readonly days: Days;
  /** The dimensions the rows are summed by; never empty. */
  readonly group: readonly AgentUsageDimension[];
  /** A device ID, or null for every device. */
  readonly device: string | null;
  readonly source: AgentSource | null;
}

export const DAY_CHOICES = [7, 30, 90] as const;
export type Days = (typeof DAY_CHOICES)[number];

/** The dimensions in the order of the table's key columns, which is also the order they have in the address. */
export const GROUP_CHOICES: readonly { readonly value: AgentUsageDimension; readonly label: string }[] = [
  { value: 'DAY', label: 'Day' },
  { value: 'DEVICE', label: 'Device' },
  { value: 'SOURCE', label: 'Program' },
  { value: 'MODEL', label: 'Model' },
  { value: 'REPOSITORY', label: 'Repository' },
];

export const SOURCE_CHOICES: readonly { readonly value: AgentSource; readonly label: string }[] = [
  { value: 'CLAUDE_CODE', label: 'Claude Code' },
  { value: 'CODEX', label: 'Codex' },
];

export const DEFAULT_QUERY: UsageQuery = { days: 30, group: ['DEVICE', 'SOURCE'], device: null, source: null };

const DAY_MS = 86_400_000;

/** The groups of `values` that the page knows, in table order, with none repeated. */
function knownGroups(values: readonly string[]): AgentUsageDimension[] {
  return GROUP_CHOICES.map((choice) => choice.value).filter((value) => values.includes(value));
}

/** The query that `params` describe; the address is typed by hand, so unknown values fall back to the defaults. */
export function parseUsageQuery(params: URLSearchParams): UsageQuery {
  const days = DAY_CHOICES.find((choice) => String(choice) === params.get('days')) ?? DEFAULT_QUERY.days;
  const group = knownGroups(params.getAll('by'));
  const source = SOURCE_CHOICES.find((choice) => choice.value === params.get('source'))?.value ?? null;
  return { days, group: group.length === 0 ? DEFAULT_QUERY.group : group, device: params.get('device') || null, source };
}

/** The search parameters for `query`: defaults are left out and the order is fixed (days, by, device, source). */
export function writeUsageQuery(query: UsageQuery): URLSearchParams {
  const params = new URLSearchParams();
  if (query.days !== DEFAULT_QUERY.days) params.set('days', String(query.days));
  if (query.group.join() !== DEFAULT_QUERY.group.join()) for (const value of query.group) params.append('by', value);
  if (query.device !== null) params.set('device', query.device);
  if (query.source !== null) params.set('source', query.source);
  return params;
}

/** `group` with `value` added or removed; the last one stays, because rows need at least one key. */
export function withGroup(group: readonly AgentUsageDimension[], value: AgentUsageDimension): AgentUsageDimension[] {
  const next = knownGroups(group.includes(value) ? group.filter((each) => each !== value) : [...group, value]);
  return next.length === 0 ? [...group] : next;
}

/** The UTC day, `YYYY-MM-DD`, of `moment`. */
function utcDay(moment: number): string {
  return new Date(moment).toISOString().slice(0, 10);
}

/** What the hub is asked for `query` when the time is `now`: the days end today in UTC (AIS-020). */
export function usageRequest(query: UsageQuery, now: Date): UsageRequest {
  const filter: AgentUsageFilter = {
    firstDay: utcDay(now.getTime() - (query.days - 1) * DAY_MS),
    lastDay: utcDay(now.getTime()),
    ...(query.device === null ? {} : { deviceId: query.device }),
    ...(query.source === null ? {} : { source: query.source }),
  };
  return { filter, by: query.group };
}
