import { expect, it } from 'vitest';
import { category, enumWords, grouped, machineTest, markWords, matches, protocolWords, reviewFact, rowLine, tierCounts, tierFact, unreviewed } from './entry';

it('HOST-047: an entry with no test on a machine says so, and a tested one gives the day', () => {
  expect(machineTest({ lastMachineTest: null })).toBe('Not tested on a machine');
  expect(machineTest({ lastMachineTest: '2026-11-20' })).toBe('Passed on a machine on 2026-11-20');
});

it('R-229: a network port mark names its port and protocols', () => {
  expect(markWords({ marks: [], networkPorts: [] })).toBe('None');
  expect(markWords({ marks: ['NETWORK_PORT'], networkPorts: [{ port: 53, protocols: ['tcp', 'udp'] }] })).toBe('Network port 53 (tcp, udp)');
  expect(markWords({ marks: ['ENGINE_SOCKET'], networkPorts: [] })).toBe('Engine socket');
  expect(enumWords('DATABASE')).toBe('Database');
});

const entry = (name: string, tags: string[], tier: 'TESTED' | 'GENERATED') => ({ name, tags, tier });

it('H39: the category is the first tag that does not name the publisher', () => {
  expect(category({ tags: ['linuxserver', 'media servers', 'music'] })).toBe('Media servers');
  expect(category({ tags: ['database'] })).toBe('Database');
  expect(category({ tags: ['linuxserver'] })).toBe('Other');
});

it('H39: the search matches a part of the name or of a tag, in any case', () => {
  const radarr = entry('radarr', ['linuxserver', 'media management'], 'GENERATED');
  expect(matches(radarr, '  RAD ')).toBe(true);
  expect(matches(radarr, 'management')).toBe(true);
  expect(matches(radarr, 'sonarr')).toBe(false);
  expect(matches(radarr, '')).toBe(true);
});

it('OD-HOST-50: the list keeps one tier on demand, in groups by category, each in the order of the names', () => {
  const entries = [
    entry('sonarr', ['linuxserver', 'media management'], 'GENERATED'),
    entry('PostgreSQL', ['database'], 'TESTED'),
    entry('radarr', ['linuxserver', 'media management'], 'GENERATED'),
    entry('Gitea', ['git'], 'TESTED'),
  ];
  const names = (groups: ReturnType<typeof grouped<(typeof entries)[number]>>) => groups.map((group) => [group.category, group.entries.map((each) => each.name)]);
  expect(names(grouped(entries, '', 'ALL'))).toEqual([['Database', ['PostgreSQL']], ['Git', ['Gitea']], ['Media management', ['radarr', 'sonarr']]]);
  expect(names(grouped(entries, '', 'GENERATED'))).toEqual([['Media management', ['radarr', 'sonarr']]]);
  expect(names(grouped(entries, 'son', 'ALL'))).toEqual([['Media management', ['sonarr']]]);
  expect(tierCounts(entries)).toEqual({ ALL: 4, TESTED: 2, GENERATED: 2 });
});

it('H39: a row names its streams and optional parts, and the tier says where a generated entry comes from', () => {
  const part = { kind: 'DEVICE', path: '/dev/dri', on: false, needsMachineSetting: true } as const;
  expect(rowLine({ version: '1.0', kind: 'APPLICATION', streams: [], optionalParts: [] })).toBe('1.0');
  expect(rowLine({ version: '18.7', kind: 'DATABASE', streams: [{ port: 1, protocols: ['tcp'] }, { port: 2, protocols: ['udp'] }], optionalParts: [part] })).toBe('18.7 · Database · 2 streams · 1 optional part');
  expect(protocolWords(['tcp', 'udp'])).toBe('TCP and UDP');
  expect(tierFact({ origin: { publisher: 'LinuxServer.io', read: '2026-11-17' }, lastMachineTest: null })).toBe('From LinuxServer.io, data of 2026-11-17. Anima did not test it.');
  expect(tierFact({ origin: null, lastMachineTest: null })).toBe('Written and read by the Anima project. Not tested on a machine.');
});

it('H39: a generated entry says who read it and when, or plainly that no one did', () => {
  const origin = { publisher: 'LinuxServer.io', read: '2026-11-17' };
  const read = { origin, reviewedBy: 'a person', reviewedOn: '2026-11-21' };
  expect(reviewFact(read)).toBe('Reviewed by a person on 2026-11-21');
  expect(unreviewed(read)).toBe(false);
  const unread = { origin, reviewedBy: null, reviewedOn: null };
  expect(reviewFact(unread)).toBe('Not reviewed');
  expect(unreviewed(unread)).toBe(true);
  const written = { origin: null, reviewedBy: null, reviewedOn: null };
  expect(reviewFact(written)).toBeNull();
  expect(unreviewed(written)).toBe(false);
});
