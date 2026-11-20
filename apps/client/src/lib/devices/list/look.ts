import type { Beat } from '$lib/checks/heartbeats';
import type { Status } from '$lib/hub/generated/graphql';
import type { CardDevice } from '../api';
import type { ListQuery } from '../query';

/** What the list knows of a device to group, count and explain it: its status, maintenance flag and checks. */
export type LookDevice = Pick<CardDevice, 'id' | 'status' | 'checks'>;

/** The groups of a list sorted by status (UR-7, D18), in the hub's STATUS order. */
export type GroupKey = 'look' | 'maintenance' | 'up';

const GROUP_TITLES: Record<GroupKey, string> = { look: 'Needs a look', maintenance: 'In maintenance', up: 'Up' };

export function groupOf(status: Status): GroupKey {
  if (status === 'UP') return 'up';
  return status === 'MAINTENANCE' ? 'maintenance' : 'look';
}

export type ListEntry<T> =
  | { readonly kind: 'group'; readonly key: string; readonly title: string; readonly count: number }
  | { readonly kind: 'device'; readonly key: string; readonly device: T };

/**
 * The list as rows: sorted by status and not filtered by it, each group opens with a quiet title and its count, so
 * the problems read first without a second list (D18). Any other order is the devices alone.
 */
export function listItems<T extends LookDevice>(devices: readonly T[], query: Pick<ListQuery, 'sort' | 'statuses'>): readonly ListEntry<T>[] {
  const rows = devices.map((device) => ({ kind: 'device' as const, key: device.id, device }));
  if (!grouped(query)) return rows;
  const counts = new Map<GroupKey, number>();
  for (const device of devices) counts.set(groupOf(device.status), (counts.get(groupOf(device.status)) ?? 0) + 1);
  const items: ListEntry<T>[] = [];
  let open: GroupKey | null = null;
  for (const row of rows) {
    const group = groupOf(row.device.status);
    if (group !== open) {
      items.push({ kind: 'group', key: `group:${group}`, title: GROUP_TITLES[group], count: counts.get(group) ?? 0 });
      open = group;
    }
    items.push(row);
  }
  return items;
}

/** Whether the list has group titles: sorted by status and not filtered by it (D18). */
export function grouped({ sort, statuses }: Pick<ListQuery, 'sort' | 'statuses'>): boolean {
  return sort === 'STATUS' && statuses.length === 0;
}

/** The count line's statuses, worst first, each with the words after its number. */
const TALLY: readonly (readonly [Status, string])[] = [
  ['DOWN', 'down'],
  ['DEGRADED', 'degraded'],
  ['UNKNOWN', 'unknown'],
  ['MAINTENANCE', 'in maintenance'],
  ['UP', 'up'],
];

export interface Tally {
  readonly total: number;
  /** Only the statuses some device has. */
  readonly parts: readonly { readonly status: Status; readonly count: number; readonly words: string }[];
}

/** "12 devices · 1 down · 2 degraded · 8 up" (D18). */
export function tallyOf(devices: readonly Pick<LookDevice, 'status'>[]): Tally {
  const parts = TALLY.map(([status, words]) => ({ status, words, count: devices.filter((device) => device.status === status).length }));
  return { total: devices.length, parts: parts.filter((part) => part.count > 0) };
}

/** A count filters the list on its status; the same count again, or the total, clears it. */
export function tallyQuery(query: ListQuery, status: Status | null): ListQuery {
  const only = query.statuses.length === 1 ? query.statuses[0] : undefined;
  return { ...query, statuses: status === null || status === only ? [] : [status] };
}

export interface Reason {
  /** What is wrong, in a few words. */
  readonly text: string;
  /** How long it has been so, "12 min", or "over 12 min" when no kept beat is up; `null` when the beats do not say. */
  readonly since: string | null;
}

const WORDS: Record<Status, string> = { UP: 'up', DOWN: 'down', DEGRADED: 'degraded', MAINTENANCE: 'in maintenance', UNKNOWN: 'unknown' };

/**
 * Why a device is not up (D18): the message of its worst check's last heartbeat, or the check's name and status.
 * `beats` are the device's live beats, oldest first, for how long it has been so.
 */
export function reasonOf(device: LookDevice, beats: readonly Beat[] | undefined, now: number): Reason | null {
  if (device.status === 'UP') return null;
  if (device.status === 'MAINTENANCE') return { text: 'In maintenance', since: null };
  if (device.checks.length === 0) return { text: 'No checks yet', since: null };
  const running = device.checks.filter((check) => !check.paused);
  if (running.length === 0) return { text: 'All checks paused', since: null };
  const worst = running.find((check) => check.status === device.status) ?? running.find((check) => check.status !== 'UP');
  if (worst === undefined) return { text: `Status is ${WORDS[device.status]}`, since: null };
  const last = worst.heartbeats.edges.at(0)?.node;
  const text = last?.message?.trim() || `${worst.name} is ${WORDS[worst.status]}`;
  return { text, since: sinceOf(beats?.filter((beat) => beat.checkId === worst.id), now) };
}

/**
 * How long the check has been other than up: from the first beat of the last run that is not up. Only the newest
 * beats are kept; a run that fills them all began before the oldest.
 */
function sinceOf(beats: readonly Beat[] | undefined, now: number): string | null {
  if (beats === undefined || beats.length === 0) return null;
  let start: string | null = null;
  let sawUp = false;
  for (let index = beats.length - 1; index >= 0; index -= 1) {
    const beat = beats[index];
    if (beat === undefined) break;
    if (beat.status === 'UP') {
      sawUp = true;
      break;
    }
    start = beat.at;
  }
  const from = start === null ? Number.NaN : Date.parse(start);
  if (Number.isNaN(from)) return null;
  // The kept beats do not reach the start of the run, so their oldest is only the least it can be.
  return sawUp ? durationText(now - from) : `over ${durationText(now - from)}`;
}

/** "<1 min", "12 min", "3 h", "2 days". */
export function durationText(ms: number): string {
  const minutes = Math.floor(Math.max(0, ms) / 60_000);
  if (minutes < 1) return '<1 min';
  if (minutes < 90) return `${String(minutes)} min`;
  const hours = Math.round(minutes / 60);
  if (hours < 36) return `${String(hours)} h`;
  const days = Math.round(hours / 24);
  return `${String(days)} ${days === 1 ? 'day' : 'days'}`;
}

/** A device that no check reaches gives no reading that is of now: its usage is a dash, not old values. */
export function hasLiveUsage(status: Status): boolean {
  return status !== 'DOWN' && status !== 'UNKNOWN';
}

/** The order of a list that has no groups: by name, every status. */
export const UNGROUPED: Pick<ListQuery, 'sort' | 'statuses'> = { sort: 'NAME', statuses: [] };
