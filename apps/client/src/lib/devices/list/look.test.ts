import { describe, expect, it } from 'vitest';
import type { Beat } from '$lib/checks/heartbeats';
import type { Status } from '$lib/hub/generated/graphql';
import { DEFAULT_QUERY } from '../query';
import { durationText, listItems, reasonOf, tallyOf, tallyQuery, type LookDevice } from './look';

const NOW = Date.parse('2026-11-13T12:00:00Z');
const ago = (minutes: number): string => new Date(NOW - minutes * 60_000).toISOString();

function check(status: Status, message: string | null = null, paused = false): LookDevice['checks'][number] {
  return { id: 'c1', name: 'HTTP :8123', status, paused, heartbeats: { edges: [{ node: { at: ago(0), status, responseTimeMs: null, message } }] } };
}

const look = (id: string, status: Status, checks: LookDevice['checks'] = []): LookDevice => ({ id, status, checks });

describe('D18: the list groups problems first only when it is sorted by status and not filtered by it', () => {
  const devices = [look('a', 'DOWN'), look('b', 'UNKNOWN'), look('c', 'MAINTENANCE'), look('d', 'UP'), look('e', 'UP')];

  it('UR-7: by status, each group opens with its title and count, in the hub order', () => {
    const items = listItems(devices, { sort: 'STATUS', statuses: [] });
    expect(items.map((item) => (item.kind === 'group' ? `${item.title} ${String(item.count)}` : item.key))).toEqual([
      'Needs a look 2',
      'a',
      'b',
      'In maintenance 1',
      'c',
      'Up 2',
      'd',
      'e',
    ]);
  });

  it.each([
    ['another order', { sort: 'NAME', statuses: [] }],
    ['a status filter', { sort: 'STATUS', statuses: ['UP'] }],
  ] as const)('UR-7: with %s the rows are the devices alone', (_, query) => {
    expect(listItems(devices, query).every((item) => item.kind === 'device')).toBe(true);
  });
});

describe('D18: the count line', () => {
  it('UR-7: counts every device and each status some device has, worst first', () => {
    expect(tallyOf([look('a', 'UP'), look('b', 'DOWN'), look('c', 'UP')])).toEqual({
      total: 3,
      parts: [
        { status: 'DOWN', words: 'down', count: 1 },
        { status: 'UP', words: 'up', count: 2 },
      ],
    });
  });

  it('UR-7: a count filters on its status; the same count again, or the total, shows every device', () => {
    const down = tallyQuery(DEFAULT_QUERY, 'DOWN');
    expect(down.statuses).toEqual(['DOWN']);
    expect(tallyQuery(down, 'DOWN').statuses).toEqual([]);
    expect(tallyQuery(down, 'UP').statuses).toEqual(['UP']);
    expect(tallyQuery(down, null).statuses).toEqual([]);
  });
});

describe('D18: why a device needs a look', () => {
  const beats = (statuses: readonly Status[]): Beat[] =>
    statuses.map((status, index) => ({ checkId: 'c1', status, at: ago(statuses.length - 1 - index) }));

  it("UR-7: the worst check's last message, and how long it has not been up", () => {
    const device = look('a', 'DOWN', [check('UP'), { ...check('DOWN', 'Connection refused'), id: 'c1' }]);
    expect(reasonOf(device, beats(['UP', 'UP', 'DOWN', 'DOWN', 'DOWN']), NOW)).toEqual({ text: 'Connection refused', since: '2 min' });
  });

  it('UR-7: without a message, the check and its status', () => {
    expect(reasonOf(look('a', 'DEGRADED', [check('DEGRADED')]), undefined, NOW)).toEqual({ text: 'HTTP :8123 is degraded', since: null });
  });

  it.each([
    ['maintenance', look('a', 'MAINTENANCE', [check('UP')]), 'In maintenance'],
    ['no checks', look('a', 'UNKNOWN'), 'No checks yet'],
    ['every check paused', look('a', 'UNKNOWN', [check('UNKNOWN', null, true)]), 'Every check is paused'],
  ] as const)('UR-7: %s says so', (_, device, text) => {
    expect(reasonOf(device, undefined, NOW)?.text).toBe(text);
  });

  it('UR-7: an up device has no reason', () => {
    expect(reasonOf(look('a', 'UP', [check('UP')]), undefined, NOW)).toBeNull();
  });

  it.each([
    [30_000, '<1 min'],
    [12 * 60_000, '12 min'],
    [3 * 3_600_000, '3 h'],
    [3 * 86_400_000, '3 days'],
  ])('UR-7: %i ms reads as %s', (ms, text) => {
    expect(durationText(ms)).toBe(text);
  });
});
