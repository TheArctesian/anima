import { describe, expect, it } from 'vitest';
import type { CardDevice } from '$lib/devices/api';
import { BAR_LENGTH, addBeat, barSummary, beatOf, cellsOf, latestBeats, shiftedByNewBeat, uptimeText, type Beat } from './heartbeats';

const START = Date.parse('2026-11-04T10:00:00Z');
const at = (minute: number): string => new Date(START + minute * 60_000).toISOString();
const beat = (minute: number, checkId = 'c1', status: Beat['status'] = 'UP'): Beat => ({ checkId, at: at(minute), status });
const edge = (minute: number, status: Beat['status'] = 'UP') => ({
  node: { at: at(minute), status, responseTimeMs: null },
});
const check = (id: string, minutes: readonly number[]): CardDevice['checks'][number] => ({
  id,
  heartbeats: { edges: minutes.map((minute) => edge(minute)) },
});
const minutesOf = (beats: readonly Beat[]) => beats.map((b) => (Date.parse(b.at) - START) / 60_000);

describe('DEV-002: latestBeats merges the checks of a device', () => {
  it('orders every check together by time, newest last, whatever order the server used', () => {
    const beats = latestBeats([check('a', [5, 1, 3]), check('b', [4, 2])]);
    expect(minutesOf(beats)).toEqual([1, 2, 3, 4, 5]);
    expect(beats.map((b) => b.checkId)).toEqual(['a', 'b', 'a', 'b', 'a']);
  });

  it('keeps the newest 50 of 120 beats', () => {
    const first = Array.from({ length: 60 }, (_, i) => i * 2);
    const second = Array.from({ length: 60 }, (_, i) => i * 2 + 1);
    const beats = latestBeats([check('a', first), check('b', second)]);
    expect(beats).toHaveLength(BAR_LENGTH);
    expect(minutesOf(beats)).toEqual(Array.from({ length: 50 }, (_, i) => 70 + i));
  });

  it('is empty for a device without checks or beats', () => {
    expect(latestBeats([])).toEqual([]);
    expect(latestBeats([check('a', [])])).toEqual([]);
  });

  it('counts a beat once when the same check and time arrive twice, in any spelling of the time', () => {
    const twice = check('a', [1]);
    const again = { id: 'a', heartbeats: { edges: [{ node: { ...edge(1).node, at: at(1).replace('Z', '+00:00') } }] } };
    expect(latestBeats([twice, again])).toHaveLength(1);
  });

  it('keeps two checks that beat at the same instant', () => {
    expect(latestBeats([check('a', [1]), check('b', [1])])).toHaveLength(2);
  });
});

describe('DEV-002: latestBeats on a large device list', () => {
  const time = (beat: Beat) => (Number.isNaN(Date.parse(beat.at)) ? Number.NEGATIVE_INFINITY : Date.parse(beat.at));
  /** The definition: a beat is its check and its instant; the first of each stays; the newest 50 in time order. */
  function byDefinition(checks: readonly CardDevice['checks'][number][]): readonly Beat[] {
    const all = checks.flatMap((c) => c.heartbeats.edges.map(({ node }): Beat => ({ checkId: c.id, ...node })));
    const unique = all.filter((b, i) => all.findIndex((o) => o.checkId === b.checkId && time(o) === time(b)) === i);
    return unique.toSorted((a, b) => time(a) - time(b)).slice(-BAR_LENGTH);
  }

  it('gives the same beats as the definition for 10 checks of 50 beats with repeats, other spellings and shuffled order', () => {
    let seed = 11;
    const next = (limit: number) => (seed = (seed * 1_664_525 + 1_013_904_223) % 4_294_967_296) % limit;
    const checks = Array.from({ length: 10 }, (_, index) => {
      const minutes = Array.from({ length: 50 }, () => next(80));
      const edges = minutes.map((minute, i) => ({
        node: { at: i % 7 === 0 ? at(minute).replace('Z', '+00:00') : at(minute), status: (i % 3 === 0 ? 'DOWN' : 'UP') as Beat['status'], responseTimeMs: minute },
      }));
      return { id: `c${String(index % 8)}`, heartbeats: { edges } };
    });
    expect(latestBeats(checks)).toEqual(byDefinition(checks));
  });

  it('merges 20000 beats without the quadratic cost (the pairwise version takes many seconds)', () => {
    const checks = Array.from({ length: 40 }, (_, index) => check(`c${String(index)}`, Array.from({ length: 500 }, (_, minute) => minute)));
    const started = performance.now();
    expect(latestBeats(checks)).toHaveLength(BAR_LENGTH);
    expect(performance.now() - started).toBeLessThan(2000);
  });
});

describe('DEV-002: a time that does not parse counts as the oldest', () => {
  const bad = (checkId = 'c1'): Beat => ({ checkId, at: 'not a time', status: 'DOWN' });

  it('sorts first in latestBeats', () => {
    const broken = { id: 'a', heartbeats: { edges: [edge(2), { node: { at: 'not a time', status: 'DOWN' as const, responseTimeMs: null } }, edge(1)] } };
    const beats = latestBeats([broken]);
    expect(beats.map((b) => b.status)).toEqual(['DOWN', 'UP', 'UP']);
    expect(beats[0]?.at).toBe('not a time');
  });

  it('is ignored by addBeat on a full bar, which returns the same list', () => {
    const full = Array.from({ length: BAR_LENGTH }, (_, i) => beat(i));
    expect(addBeat(full, bad())).toBe(full);
  });

  it('goes first in a bar that has room', () => {
    expect(addBeat([beat(1)], bad()).map((b) => b.status)).toEqual(['DOWN', 'UP']);
  });

  it('collapses two of one check into one, but keeps those of two checks', () => {
    expect(addBeat([bad()], bad())).toHaveLength(1);
    expect(addBeat([bad('a')], bad('b'))).toHaveLength(2);
  });
});

describe('DEV-002: addBeat adds a live heartbeat to the bar', () => {
  it('appends a newer beat at the end', () => {
    expect(minutesOf(addBeat([beat(1), beat(2)], beat(3)))).toEqual([1, 2, 3]);
  });

  it('puts an older beat in its place in time', () => {
    expect(minutesOf(addBeat([beat(1), beat(3)], beat(2)))).toEqual([1, 2, 3]);
  });

  it('drops the oldest beat when the bar is full', () => {
    const full = Array.from({ length: BAR_LENGTH }, (_, i) => beat(i));
    const next = addBeat(full, beat(BAR_LENGTH));
    expect(next).toHaveLength(BAR_LENGTH);
    expect(minutesOf(next).at(0)).toBe(1);
    expect(minutesOf(next).at(-1)).toBe(BAR_LENGTH);
  });

  it('ignores a duplicate of the same check and time, returning the same list', () => {
    const beats = [beat(1), beat(2)];
    expect(addBeat(beats, beat(2, 'c1', 'DOWN'))).toBe(beats);
  });

  it('ignores a beat older than a full bar, returning the same list', () => {
    const full = Array.from({ length: BAR_LENGTH }, (_, i) => beat(i + 10));
    expect(addBeat(full, beat(1))).toBe(full);
  });

  it('adds a beat of another check at an equal time after the existing one', () => {
    expect(addBeat([beat(1, 'a')], beat(1, 'b')).map((b) => b.checkId)).toEqual(['a', 'b']);
  });

  it('turns a pushed heartbeat into a bar beat', () => {
    const added = { checkId: 'c9', deviceId: 'd1', heartbeat: { at: at(1), status: 'DOWN' as const, responseTimeMs: 12 } };
    expect(beatOf(added)).toEqual({ checkId: 'c9', at: at(1), status: 'DOWN', responseTimeMs: 12 });
  });
});

describe('DEV-002: the bar reads its beats', () => {
  it('pads a short bar with empty cells on the left', () => {
    const cells = cellsOf([beat(1, 'a', 'DOWN'), beat(2)]);
    expect(cells).toHaveLength(BAR_LENGTH);
    expect(cells.slice(-2)).toEqual(['DOWN', 'UP']);
    expect(cells.slice(0, -2).every((cell) => cell === null)).toBe(true);
  });

  it.each([
    [[], 'No heartbeats yet'],
    [[beat(1)], '1 heartbeat, latest up'],
    [[beat(1), beat(2, 'c1', 'DEGRADED')], '2 heartbeats, latest degraded'],
  ])('summarises %j as "%s"', (beats, text) => {
    expect(barSummary(beats)).toBe(text);
  });

  it('moves only when the newest beat changes', () => {
    expect(shiftedByNewBeat([beat(1)], [beat(1), beat(2)])).toBe(true);
    expect(shiftedByNewBeat([], [beat(1)])).toBe(false);
    expect(shiftedByNewBeat([beat(1), beat(3)], [beat(1), beat(2), beat(3)])).toBe(false);
    expect(shiftedByNewBeat([beat(1)], [])).toBe(false);
  });
});

describe('DEV-002: uptimeText', () => {
  it.each([
    [0.9995, '99.95 %'],
    [1, '100.00 %'],
    [0, '0.00 %'],
    [0.999999, '99.99 %'],
    [0.5, '50.00 %'],
    [null, '—'],
    [Number.NaN, '—'],
    [1.2, '100.00 %'],
    [-1, '0.00 %'],
  ])('shows %s as %s', (ratio, text) => {
    expect(uptimeText(ratio)).toBe(text);
  });
});
