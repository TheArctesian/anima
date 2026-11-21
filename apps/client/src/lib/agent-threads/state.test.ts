import { describe, expect, it } from 'vitest';
import { markSeen, needsYouCount, orderedThreads, storedSeen, threadState, type Seen } from './state';
import { rowsFrom, type Tail, type ThreadRow } from './thread';

const NOW = new Date('2026-11-20T15:00:00Z');
const NONE: Seen = new Map();

function row(id: string, tail: Tail, secondsAgo: number): ThreadRow {
  const made = rowsFrom([{ id, program: 'claude_code', title: id, folder: '/w/a', branch: null, model: null, started: null, last: new Date(NOW.getTime() - secondsAgo * 1000).toISOString(), tail }]);
  if (made === null || made[0] === undefined) throw new Error('not a row');
  return made[0];
}

describe('the state of a thread from its session file', () => {
  it('THR-055: a turn that ended is your turn until the thread is opened, and for a day', () => {
    const ended = row('a', 'turn_ended', 600);
    expect(threadState(ended, NOW, NONE)).toBe('your_turn');
    expect(threadState(ended, NOW, new Map([['a', NOW]]))).toBe('done');
    expect(threadState(ended, NOW, new Map([['a', new Date(NOW.getTime() - 3_600_000)]]))).toBe('your_turn');
    expect(threadState(row('a', 'turn_ended', 25 * 3600), NOW, NONE)).toBe('done');
  });

  it('THR-055: an open tool call works for a minute, then may wait for the person; the file cannot tell more', () => {
    expect(threadState(row('a', 'open_tool', 20), NOW, NONE)).toBe('working');
    expect(threadState(row('a', 'open_tool', 90), NOW, NONE)).toBe('may_wait');
    expect(threadState(row('a', 'open_tool', 2 * 24 * 3600), NOW, NONE)).toBe('idle');
  });

  it('THR-055: an open turn works while its file changes, and is idle after 2 minutes; a stop is a stop', () => {
    expect(threadState(row('a', 'open', 30), NOW, NONE)).toBe('working');
    expect(threadState(row('a', 'open', 300), NOW, NONE)).toBe('idle');
    expect(threadState(row('a', 'stopped', 30), NOW, NONE)).toBe('stopped');
    expect(threadState(row('a', 'nothing', 30), NOW, NONE)).toBe('idle');
  });

  it('THR-056: the threads that need you are at the top, a possible approval first; the threads that work are at the bottom', () => {
    const rows = [row('work', 'open', 10), row('old', 'turn_ended', 3 * 24 * 3600), row('turn-new', 'turn_ended', 60), row('ask', 'open_tool', 900), row('turn-old', 'turn_ended', 7200), row('stop', 'stopped', 30)];
    const entries = orderedThreads(rows, NOW, NONE).map((entry) => (entry.kind === 'group' ? `${entry.title} ${String(entry.count)}` : entry.key));
    expect(entries).toEqual(['Needs you 3', 'ask', 'turn-new', 'turn-old', 'Earlier 2', 'stop', 'old', 'Working 1', 'work']);
    expect(needsYouCount(rows, NOW, NONE)).toBe(3);
  });

  it('THR-056: the folded group Earlier keeps its title and its count, and none of its threads', () => {
    const rows = [row('work', 'open', 10), row('old', 'turn_ended', 3 * 24 * 3600), row('turn', 'turn_ended', 60)];
    const entries = orderedThreads(rows, NOW, NONE, { earlierOpen: false, chosen: null }).map((entry) => (entry.kind === 'group' ? `${entry.title} ${String(entry.count)}` : entry.key));
    expect(entries).toEqual(['Needs you 1', 'turn', 'Earlier 1', 'Working 1', 'work']);
  });

  it('THR-056: a group with no thread is left out', () => {
    expect(orderedThreads([row('work', 'open', 10)], NOW, NONE).map((entry) => entry.key)).toEqual(['group:working', 'work']);
  });

  it('THR-055: what the person opened is kept on this computer, and a value that cannot be read is nothing', () => {
    const kept = new Map<string, string>();
    const storage = { getItem: (key: string) => kept.get(key) ?? null, setItem: (key: string, value: string) => void kept.set(key, value) };
    const seen = markSeen(storage, markSeen(storage, NONE, 'a', new Date(NOW.getTime() - 1000)), 'a', NOW);
    expect([...seen]).toEqual([['a', NOW]]);
    expect(storedSeen(storage).get('a')).toEqual(NOW);
    kept.set('anima.threads.seen', '{not json');
    expect(storedSeen(storage).size).toBe(0);
  });
});
