import type { ThreadRow } from './thread';

/**
 * The state of a thread as the session file and the clock give it (THR-055 to THR-057). It is a reading of a file, not a
 * report of the program: a file does not say that a tool call waits for an approval, only that it has no result yet.
 */
export type ThreadState =
  /** A tool call has had no result for a minute: it runs long, or it waits for an approval. */
  | 'may_wait'
  /** The agent ended its turn and the person did not open the thread since. */
  | 'your_turn'
  /** The file changed in the last 2 minutes and the turn is open. */
  | 'working'
  /** The turn ended and the person saw it, or it ended more than a day ago. */
  | 'done'
  | 'stopped'
  /** An open turn in a file that did not change for 2 minutes or more. */
  | 'idle';

/** A file that changed in this time has an agent at work, when its turn is open. */
export const WORKING_MS = 2 * 60_000;
/** A tool call with no result for this long may wait for an approval. */
export const QUIET_TOOL_MS = 60_000;
/** A thread that waits is in the group "Needs you" for this long; after it, it is in "Earlier". */
export const NEEDS_YOU_MS = 24 * 60 * 60_000;

/** When the person last opened each thread, by its ID. */
export type Seen = ReadonlyMap<string, Date>;

function seenSince(row: ThreadRow, seen: Seen): boolean {
  const at = seen.get(row.id);
  return at !== undefined && row.last !== null && at.getTime() >= row.last.getTime();
}

/** The state of `row` at `now`, with what the person opened. */
export function threadState(row: ThreadRow, now: Date, seen: Seen): ThreadState {
  const age = row.last === null ? Number.POSITIVE_INFINITY : now.getTime() - row.last.getTime();
  const fresh = age < NEEDS_YOU_MS && !seenSince(row, seen);
  switch (row.tail) {
    case 'turn_ended':
      return fresh ? 'your_turn' : 'done';
    case 'stopped':
      return 'stopped';
    case 'open_tool':
      if (age < QUIET_TOOL_MS) return 'working';
      return fresh ? 'may_wait' : 'idle';
    case 'open':
      return age < WORKING_MS ? 'working' : 'idle';
    case 'nothing':
      return 'idle';
  }
}

/** Whether a state is one that waits for the person. */
export function needsYou(state: ThreadState): boolean {
  return state === 'may_wait' || state === 'your_turn';
}

/** The words and the icon of a state; the mark is never the colour alone (UI-003). */
export const STATE_MARKS: Readonly<Record<ThreadState, { readonly words: string; readonly icon: 'carbon:notification' | 'carbon:activity' | 'carbon:checkmark' | 'carbon:pause' | 'carbon:circle-dash'; readonly detail: string }>> = {
  may_wait: { words: 'May wait for you', icon: 'carbon:notification', detail: 'A tool call has had no result for a minute. It runs long, or it waits for your approval. The session file cannot tell which.' },
  your_turn: { words: 'Your turn', icon: 'carbon:notification', detail: 'The agent ended its turn. It waits for your next prompt.' },
  working: { words: 'Working', icon: 'carbon:activity', detail: 'The session file changed in the last 2 minutes.' },
  done: { words: 'Done', icon: 'carbon:checkmark', detail: 'The agent ended its turn.' },
  stopped: { words: 'Stopped', icon: 'carbon:pause', detail: 'The turn was stopped.' },
  idle: { words: 'Idle', icon: 'carbon:circle-dash', detail: 'The turn is open and the session file did not change for 2 minutes or more.' },
};

export type GroupKey = 'needs' | 'earlier' | 'working';

/** The key of the entry of the group "Earlier", which the list can fold. */
export const EARLIER_KEY = 'group:earlier';

const GROUP_TITLES: Readonly<Record<GroupKey, string>> = { needs: 'Needs you', earlier: 'Earlier', working: 'Working' };

function groupOf(state: ThreadState): GroupKey {
  if (needsYou(state)) return 'needs';
  return state === 'working' ? 'working' : 'earlier';
}

/** A row of the list with its state. */
export interface StatedRow {
  readonly row: ThreadRow;
  readonly state: ThreadState;
}

/** One entry of the list, in the form that the held order of a live list takes (`$lib/devices/list/held`). */
export type ThreadEntry = { readonly kind: 'group'; readonly key: string; readonly title: string; readonly count: number } | { readonly kind: 'device'; readonly key: string; readonly device: StatedRow };

const newestFirst = (a: StatedRow, b: StatedRow): number => (b.row.last?.getTime() ?? 0) - (a.row.last?.getTime() ?? 0);

/** Whether the group "Earlier" shows its threads, and the thread that is open. */
export interface Fold {
  readonly earlierOpen: boolean;
  readonly chosen: string | null;
}

/**
 * The list in its order (THR-056): the threads that need the person at the top, a tool call that may wait for an
 * approval before a turn that ended, the newest first; then the threads that are done, stopped or idle, by their last
 * change; the threads that work at the bottom. Each group opens with its title and its count; an empty group is left out.
 * The group "Earlier" is folded unless `fold.earlierOpen`: then only its title, its count and the thread that is open (`fold.chosen`) are in the list.
 */
export function orderedThreads(rows: readonly ThreadRow[], now: Date, seen: Seen, fold: Fold = { earlierOpen: true, chosen: null }): readonly ThreadEntry[] {
  const stated = rows.map((row) => ({ row, state: threadState(row, now, seen) }));
  const entries: ThreadEntry[] = [];
  for (const group of ['needs', 'earlier', 'working'] as const) {
    const members = stated.filter((each) => groupOf(each.state) === group).sort(newestFirst);
    if (group === 'needs') members.sort((a, b) => Number(b.state === 'may_wait') - Number(a.state === 'may_wait'));
    if (members.length === 0) continue;
    entries.push({ kind: 'group', key: `group:${group}`, title: GROUP_TITLES[group], count: members.length });
    // The thread that is open stays in the list when its group is folded, so that it does not go from under the person.
    const shown = group === 'earlier' && !fold.earlierOpen ? members.filter((member) => member.row.id === fold.chosen) : members;
    for (const member of shown) entries.push({ kind: 'device', key: member.row.id, device: member });
  }
  return entries;
}

/** How many threads need the person: the count on the item Threads of the bar (THR-057). */
export function needsYouCount(rows: readonly ThreadRow[], now: Date, seen: Seen): number {
  return rows.filter((row) => needsYou(threadState(row, now, seen))).length;
}

const SEEN_KEY = 'anima.threads.seen';
/** The most threads whose last opening is kept. */
const SEEN_LIMIT = 500;

/** What the person opened, as kept on this computer; an unreadable value is nothing. */
export function storedSeen(storage: Pick<Storage, 'getItem'>): Seen {
  try {
    const parsed: unknown = JSON.parse(storage.getItem(SEEN_KEY) ?? '{}');
    if (typeof parsed !== 'object' || parsed === null) return new Map();
    const pairs = Object.entries(parsed).flatMap(([id, at]) => {
      const time = typeof at === 'string' ? new Date(at) : null;
      return time !== null && !Number.isNaN(time.getTime()) ? [[id, time] as const] : [];
    });
    return new Map(pairs);
  } catch {
    return new Map();
  }
}

/** `seen` with the thread `id` opened at `now`, kept in `storage` when there is one; only the newest openings are kept. */
export function markSeen(storage: Pick<Storage, 'setItem'> | null, seen: Seen, id: string, now: Date): Seen {
  const next = [...seen.entries(), [id, now] as const].sort((a, b) => b[1].getTime() - a[1].getTime());
  const kept = new Map(next.reverse().slice(-SEEN_LIMIT));
  try {
    storage?.setItem(SEEN_KEY, JSON.stringify(Object.fromEntries([...kept].map(([key, at]) => [key, at.toISOString()]))));
  } catch {
    // A full or closed store only means that the mark is not kept after a restart.
  }
  return kept;
}
