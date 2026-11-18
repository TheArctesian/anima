import { expect, it } from 'vitest';
import { CONTAINERS, container } from './containers.fixture';
import { NO_PROJECT, lineTime, logText, projectGroups } from './rows';

it('HOST-013: the containers are in groups by Compose project, by name, and those with no project are last', () => {
  const groups = projectGroups(CONTAINERS);
  expect(groups.map((group) => group.project)).toEqual(['ai', 'tracks', NO_PROJECT]);
  expect(groups[1]?.rows.map((row) => row.name)).toEqual(['tracks-app-1', 'tracks-worker-1']);
  expect(groups.map((group) => group.summary)).toEqual(['1 of 1 running', '1 of 2 running', '1 of 1 running']);
});

it('HOST-001: a row gives the short identifier, the state as a word, the status text and the ports', () => {
  const [row] = projectGroups([container({ ports: ['0.0.0.0:3000->3000/tcp', '[::]:3000->3000/tcp'] })])[0]?.rows ?? [];
  expect(row).toMatchObject({ shortId: '4f5c1d2e3a4b', state: 'Running', tone: 'up', status: 'Up 3 hours (healthy)', ports: '0.0.0.0:3000->3000/tcp, [::]:3000->3000/tcp' });
});

it('HOST-001: a state that is not healthy running never reads as up', () => {
  const read = (change: Parameters<typeof container>[0]) => {
    const row = projectGroups([container(change)])[0]?.rows[0];
    return [row?.state, row?.tone];
  };
  expect(read({ health: 'UNHEALTHY' })).toEqual(['Running, unhealthy', 'down']);
  expect(read({ health: 'STARTING' })).toEqual(['Running, starting', 'warning']);
  expect(read({ state: 'RESTARTING', health: 'NONE' })).toEqual(['Restarting', 'warning']);
  expect(read({ state: 'EXITED', health: 'NONE' })).toEqual(['Exited', 'unknown']);
  expect(read({ state: 'DEAD', health: 'NONE' })).toEqual(['Dead', 'down']);
  expect(read({ state: 'UNKNOWN', health: 'NONE' })).toEqual(['Unknown', 'unknown']);
});

it('HOST-001: the text of a log is its lines, oldest first, with no time', () => {
  expect(logText([{ at: '2026-11-18T18:00:00+00:00', text: 'one' }, { at: null, text: 'two' }])).toBe('one\ntwo');
});

it('HOST-001: the time of a line is the time of the clock of the operator, and a line with no time has none', () => {
  expect(lineTime('2026-11-18T18:00:04+00:00')).toBe(new Date('2026-11-18T18:00:04Z').toLocaleTimeString([], { hour12: false }));
  expect(lineTime(null)).toBe('');
  expect(lineTime('not a time')).toBe('');
});
