import type { AgentSource } from '$lib/hub/generated/graphql';
import type { AgentUsageView, UsageSums } from '../api';
import type { Count } from '../query';

// What the page draws from the hub's sums. Pure: the same view and choices give the same shapes.

const DAY_MS = 86_400_000;

/** All the tokens of a group: input, output and both cache counts. */
export function totalTokens(sums: UsageSums): number {
  return sums.inputTokens + sums.outputTokens + sums.cacheReadTokens + sums.cacheWriteTokens;
}

/** What `count` counts in a group; a cost with no known price counts as none. */
export function countOf(sums: UsageSums, count: Count): number {
  if (count === 'tokens') return totalTokens(sums);
  return count === 'sessions' ? sums.sessions : (sums.estimatedCostUsd ?? 0);
}

/** One program and model: the row of the table that limits the chart. */
export interface Picked {
  readonly source: AgentSource | null;
  readonly model: string | null;
}

export interface DayBar {
  /** `YYYY-MM-DD`, UTC. */
  readonly day: string;
  readonly value: number;
  /** The share of the tallest day, 0 to 1. */
  readonly height: number;
}

/** One bar for each UTC day of the view, in order, a day with no use included; only the rows of `picked` when one is given. */
export function dayBars(view: Pick<AgentUsageView, 'firstDay' | 'lastDay' | 'days'>, count: Count, picked: Picked | null): DayBar[] {
  const sums = new Map<string, number>();
  for (const row of view.days) {
    if (picked !== null && (row.source !== picked.source || row.model !== picked.model)) continue;
    sums.set(row.day, (sums.get(row.day) ?? 0) + countOf(row, count));
  }
  const first = Date.parse(`${view.firstDay}T00:00:00Z`);
  const last = Date.parse(`${view.lastDay}T00:00:00Z`);
  if (Number.isNaN(first) || Number.isNaN(last) || last < first) return [];
  const days: string[] = [];
  // A view is at most 366 days (the hub's limit); the bound keeps a wrong answer from a long loop.
  for (let at = first; at <= last && days.length < 400; at += DAY_MS) days.push(new Date(at).toISOString().slice(0, 10));
  const tallest = Math.max(0, ...days.map((day) => sums.get(day) ?? 0));
  return days.map((day) => {
    const value = sums.get(day) ?? 0;
    return { day, value, height: tallest === 0 ? 0 : value / tallest };
  });
}

export interface RepositoryLine {
  /** Null for the use that has no repository. */
  readonly repository: string | null;
  readonly sessions: number;
  readonly estimatedCostUsd: number | null;
  /** The newest UTC day with use, when the hub gave it. */
  readonly lastDay: string | null;
}

/**
 * The repositories, newest use first; with the same newest day, or none known, the larger estimate first; the use with
 * no repository is last (AIS-020).
 */
export function repositoryLines(view: Pick<AgentUsageView, 'repositories' | 'newest'>): RepositoryLine[] {
  const newest = new Map<string | null, string>();
  for (const { repository, day } of view.newest) {
    const known = newest.get(repository);
    if (known === undefined || day > known) newest.set(repository, day);
  }
  const lines = view.repositories.map((row) => ({
    repository: row.repository,
    sessions: row.sessions,
    estimatedCostUsd: row.estimatedCostUsd,
    lastDay: newest.get(row.repository) ?? null,
  }));
  return lines.sort((one, other) => {
    if ((one.repository === null) !== (other.repository === null)) return one.repository === null ? 1 : -1;
    if (one.lastDay !== other.lastDay) return (other.lastDay ?? '') < (one.lastDay ?? '') ? -1 : 1;
    return (other.estimatedCostUsd ?? 0) - (one.estimatedCostUsd ?? 0);
  });
}

/** "last use today", "last use yesterday", "last use 3 days ago", from two UTC days; nothing for a day that is not known. */
export function lastUseWords(lastDay: string | null, today: string): string {
  if (lastDay === null) return '';
  const days = Math.round((Date.parse(`${today}T00:00:00Z`) - Date.parse(`${lastDay}T00:00:00Z`)) / DAY_MS);
  if (Number.isNaN(days) || days < 0) return '';
  if (days === 0) return 'last use today';
  return days === 1 ? 'last use yesterday' : `last use ${String(days)} days ago`;
}

/** The share of `value` in `whole`, 0 to 1; none of nothing. */
export function share(value: number, whole: number): number {
  return whole <= 0 ? 0 : Math.min(1, Math.max(0, value / whole));
}

export interface TokenPart {
  readonly label: 'Input' | 'Output' | 'Cache read' | 'Cache write';
  readonly tokens: number;
}

/** The four kinds of tokens of a group, in the order of the bar. */
export function tokenParts(sums: UsageSums): TokenPart[] {
  return [
    { label: 'Input', tokens: sums.inputTokens },
    { label: 'Output', tokens: sums.outputTokens },
    { label: 'Cache read', tokens: sums.cacheReadTokens },
    { label: 'Cache write', tokens: sums.cacheWriteTokens },
  ];
}

/** `2026-11-19` as `19 Nov`: the ends of the chart. */
export function shortDay(day: string): string {
  const at = new Date(`${day}T00:00:00Z`);
  if (Number.isNaN(at.getTime())) return day;
  return `${String(at.getUTCDate())} ${at.toLocaleString('en', { month: 'short', timeZone: 'UTC' })}`;
}

/** The host that each named repository starts with, with its `/` (`github.com/`); empty when they do not share one. */
export function sharedHost(lines: readonly Pick<RepositoryLine, 'repository'>[]): string {
  const names = lines.flatMap((line) => (line.repository === null ? [] : [line.repository]));
  const first = names[0];
  const end = first?.indexOf('/') ?? -1;
  if (first === undefined || end <= 0) return '';
  const host = first.slice(0, end + 1);
  return names.every((name) => name.startsWith(host) && name.length > host.length) ? host : '';
}
