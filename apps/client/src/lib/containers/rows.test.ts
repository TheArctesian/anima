import { expect, it } from 'vitest';
import { CONTAINERS, UPDATES, container } from './containers.fixture';
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

it('HOST-019: a group has the one state of its project as the hub made it, and the containers with no project have none', () => {
  const groups = projectGroups(CONTAINERS);
  expect(groups.map((group) => [group.project, group.state, group.tone])).toEqual([
    ['ai', 'Unhealthy', 'down'],
    ['tracks', 'Partly running', 'warning'],
    [NO_PROJECT, '', 'unknown'],
  ]);
  const stopped = projectGroups([container({ state: 'EXITED', composeProjectState: 'STOPPED' })]);
  expect([stopped[0]?.state, stopped[0]?.tone]).toEqual(['Stopped', 'unknown']);
});

it('HOST-021: a row has no mark before the operator asked, and then the answer of the hub as words', () => {
  expect(projectGroups(CONTAINERS).flatMap((group) => group.rows.map((row) => row.update))).toEqual([null, null, null, null]);
  const marks = Object.fromEntries(projectGroups(CONTAINERS, UPDATES).flatMap((group) => group.rows.map((row) => [row.name, row.update])));
  expect(marks).toEqual({
    'tracks-app-1': { newer: true, words: 'Newer image' },
    'tracks-worker-1': { newer: false, words: 'Up to date' },
    recorder: { newer: false, words: 'Built here, or the tag moved' },
    // The hub gave no answer for this container.
    ollama: null,
  });
});
