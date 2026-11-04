import { describe, expect, it } from 'vitest';
import type { EventPage, LogEvent } from './api';
import {
  addLive,
  appendOlder,
  DEFAULT_LOG_FILTER,
  eventVariables,
  fromPage,
  LOG_CAP,
  matches,
  minSeverity,
  type EventLog,
  type LogFilter,
} from './log';

const NOW = new Date('2026-06-01T12:00:00.000Z');

function event(id: string, overrides: Partial<LogEvent> = {}): LogEvent {
  return {
    id,
    at: '2026-06-01T11:59:00.000Z',
    severity: 'INFO',
    source: 'CHECK',
    deviceId: 'd1',
    deviceName: 'nas',
    message: `message ${id}`,
    ...overrides,
  };
}

function page(ids: string[], hasNextPage = false, endCursor: string | null = null): EventPage {
  return { pageInfo: { hasNextPage, endCursor }, edges: ids.map((id) => ({ node: event(id) })) };
}

function logOf(ids: string[]): EventLog {
  return fromPage(page(ids, true, 'c1'));
}

function fullLog(): EventLog {
  return logOf(Array.from({ length: LOG_CAP }, (_, index) => `e${String(index)}`));
}

const filterWith = (changes: Partial<LogFilter>): LogFilter => ({ ...DEFAULT_LOG_FILTER, ...changes });

describe('eventVariables', () => {
  it('LOG-009 the default filter asks for the last 24 hours, 100 rows', () => {
    expect(eventVariables(DEFAULT_LOG_FILTER, NOW)).toEqual({
      filter: { since: '2026-05-31T12:00:00.000Z' },
      first: 100,
    });
  });

  it.each([
    ['1h', '2026-06-01T11:00:00.000Z'],
    ['7d', '2026-05-25T12:00:00.000Z'],
  ] as const)('LOG-009 since %s is computed from now', (since, expected) => {
    expect(eventVariables(filterWith({ since }), NOW).filter).toEqual({ since: expected });
  });

  it('LOG-009 since "all" leaves since out', () => {
    expect(eventVariables(filterWith({ since: 'all' }), NOW).filter).toEqual({});
  });

  it('LOG-009 severity, source and device each reach the filter', () => {
    const variables = eventVariables(
      filterWith({ since: 'all', severities: ['ERROR'], sources: ['CHECK', 'HUB'], deviceId: 'd9' }),
      NOW,
    );
    expect(variables.filter).toEqual({ severity: ['ERROR'], source: ['CHECK', 'HUB'], deviceId: 'd9' });
  });

  it('LOG-009 an older page sends the cursor', () => {
    expect(eventVariables(DEFAULT_LOG_FILTER, NOW, 'c7').after).toBe('c7');
    expect(eventVariables(DEFAULT_LOG_FILTER, NOW)).not.toHaveProperty('after');
  });
});

describe('minSeverity', () => {
  it('LOG-010 is INFO when none is selected', () => {
    expect(minSeverity(DEFAULT_LOG_FILTER)).toBe('INFO');
  });

  it('LOG-010 is the lowest selected severity', () => {
    expect(minSeverity(filterWith({ severities: ['ERROR', 'WARNING'] }))).toBe('WARNING');
    expect(minSeverity(filterWith({ severities: ['ERROR'] }))).toBe('ERROR');
  });
});

describe('matches', () => {
  const cases: [string, LogEvent, Partial<LogFilter>, boolean][] = [
    ['default filter takes a recent event', event('a'), {}, true],
    ['severity in the selection', event('a', { severity: 'ERROR' }), { severities: ['ERROR'] }, true],
    ['severity outside the selection', event('a'), { severities: ['ERROR'] }, false],
    ['source in the selection', event('a'), { sources: ['CHECK'] }, true],
    ['source outside the selection', event('a'), { sources: ['HUB'] }, false],
    ['the chosen device', event('a'), { deviceId: 'd1' }, true],
    ['another device', event('a'), { deviceId: 'd2' }, false],
    ['an event without a device under a device filter', event('a', { deviceId: null }), { deviceId: 'd1' }, false],
    ['older than the window', event('a', { at: '2026-05-30T00:00:00.000Z' }), {}, false],
    ['older than the window under "all"', event('a', { at: '2020-01-01T00:00:00.000Z' }), { since: 'all' }, true],
  ];
  it.each(cases)('LOG-009 %s', (_name, candidate, changes, expected) => {
    expect(matches(candidate, filterWith(changes), NOW)).toBe(expected);
  });
});

describe('event log', () => {
  it('LOG-009 the first page keeps the order and the cursor', () => {
    const log = fromPage(page(['b', 'a'], true, 'c1'));
    expect(log.rows.map((row) => row.id)).toEqual(['b', 'a']);
    expect(log).toMatchObject({ endCursor: 'c1', hasNextPage: true });
  });

  it('LOG-009 an older page goes below, without ids already shown', () => {
    const merged = appendOlder(logOf(['c', 'b']), page(['b', 'a'], false, 'c2'));
    expect(merged.rows.map((row) => row.id)).toEqual(['c', 'b', 'a']);
    expect(merged).toMatchObject({ endCursor: 'c2', hasNextPage: false });
  });

  it('LOG-010 a live event appears at the top', () => {
    const next = addLive(logOf(['b', 'a']), event('c'), DEFAULT_LOG_FILTER, NOW);
    expect(next.rows.map((row) => row.id)).toEqual(['c', 'b', 'a']);
  });

  it('LOG-009 an event with an unparseable time does not match a time window, but does under "all"', () => {
    const odd = event('a', { at: 'yesterday' });
    expect(matches(odd, DEFAULT_LOG_FILTER, NOW)).toBe(false);
    expect(matches(odd, filterWith({ since: 'all' }), NOW)).toBe(true);
  });

  it('LOG-009 a live event that does not match leaves the log as it was', () => {
    const log = logOf(['a']);
    expect(addLive(log, event('x', { severity: 'INFO' }), filterWith({ severities: ['ERROR'] }), NOW)).toBe(log);
  });

  it('LOG-010 a live event already shown leaves the log as it was', () => {
    const log = logOf(['a']);
    expect(addLive(log, event('a'), DEFAULT_LOG_FILTER, NOW)).toBe(log);
  });

  it('LOG-010 the list holds at 500 rows, drops the oldest and offers no older page', () => {
    const next = addLive(fullLog(), event('new'), DEFAULT_LOG_FILTER, NOW);
    expect(next.rows).toHaveLength(LOG_CAP);
    expect(next.rows[0]?.id).toBe('new');
    expect(next.rows.at(-1)?.id).toBe(`e${String(LOG_CAP - 2)}`);
    expect(next).toMatchObject({ hasNextPage: false, endCursor: 'c1' });
  });

  it('LOG-010 a live event below the cap keeps the cursor and the older page', () => {
    expect(addLive(logOf(['a']), event('b'), DEFAULT_LOG_FILTER, NOW)).toMatchObject({
      hasNextPage: true,
      endCursor: 'c1',
    });
  });

  it('LOG-010 an older page on a full list changes nothing and offers no older page', () => {
    const full = fullLog();
    const next = appendOlder(full, page(['x', 'y'], true, 'c9'));
    expect(next.rows).toEqual(full.rows);
    expect(next).toMatchObject({ hasNextPage: false, endCursor: 'c1' });
  });

  it('LOG-010 an older page that only partly fits fills the list to 500 and offers no older page', () => {
    const ids = Array.from({ length: LOG_CAP - 2 }, (_, index) => `e${String(index)}`);
    const older = Array.from({ length: 100 }, (_, index) => `o${String(index)}`);
    const next = appendOlder(logOf(ids), page(older, true, 'c9'));
    expect(next.rows).toHaveLength(LOG_CAP);
    expect(next.rows.slice(-2).map((row) => row.id)).toEqual(['o0', 'o1']);
    expect(next).toMatchObject({ hasNextPage: false, endCursor: 'c1' });
  });
});
