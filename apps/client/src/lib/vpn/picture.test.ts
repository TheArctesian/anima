import { expect, it } from 'vitest';
import { keyText, keyTone, pathLabel, readAgo, sortedNodes, stateLabel } from './picture';
import { node, NOW } from './vpn.fixture';

const at = (days: number): string => new Date(NOW.getTime() + days * 86_400_000).toISOString();

it('VPN-009: every path has its words, and a relayed path names its region', () => {
  expect(pathLabel('DIRECT', null)).toBe('Direct');
  expect(pathLabel('PEER_RELAY', null)).toBe('Peer relay');
  expect(pathLabel('RELAYED', 'sfo')).toBe('Relayed (sfo)');
  expect(pathLabel('RELAYED', null)).toBe('Relayed');
  expect(pathLabel('UNKNOWN', null)).toBe('Not known');
});

it('VPN-009: every site state has its words', () => {
  expect((['SERVING', 'OFFLINE', 'NO_ROUTER'] as const).map(stateLabel)).toEqual(['Serving', 'Router offline', 'No router']);
});

it('VPN-010: each key state reads in words with its date or days', () => {
  expect(keyText({ keyState: 'NEVER', keyExpiry: null }, NOW)).toBe('Does not expire');
  expect(keyText({ keyState: 'VALID', keyExpiry: '2027-05-12T12:00:00Z' }, NOW, 'en-GB')).toBe('Expires on 12 May 2027');
  expect(keyText({ keyState: 'EXPIRING', keyExpiry: at(8.5) }, NOW)).toBe('Expires in 9 days');
  expect(keyText({ keyState: 'EXPIRING', keyExpiry: at(0.2) }, NOW)).toBe('Expires in 1 day');
  expect(keyText({ keyState: 'EXPIRED', keyExpiry: at(-3.2) }, NOW)).toBe('Expired 3 days ago');
  expect(keyText({ keyState: 'EXPIRED', keyExpiry: at(-0.2) }, NOW)).toBe('Expired today');
  expect(keyText({ keyState: 'TAGGED_WITH_EXPIRY', keyExpiry: '2027-05-12T12:00:00Z' }, NOW, 'en-GB')).toBe(
    'Tagged, but expires on 12 May 2027: renew it as a tagged node',
  );
  expect(keyText({ keyState: 'VALID', keyExpiry: null }, NOW)).toBe('Expiry not known');
});

it('VPN-010: a key is up, warning or down, by state', () => {
  expect((['NEVER', 'VALID', 'EXPIRING', 'TAGGED_WITH_EXPIRY', 'EXPIRED'] as const).map(keyTone)).toEqual(['up', 'up', 'warning', 'warning', 'down']);
});

it('VPN-011: nodes not in Anima come first, then keys that need attention, then by name', () => {
  const nodes = [
    node({ name: 'b-fine' }),
    node({ name: 'a-fine' }),
    node({ name: 'z-expired', keyState: 'EXPIRED' }),
    node({ name: 'y-stranger', inAnima: false }),
    node({ name: 'x-stranger-expiring', inAnima: false, keyState: 'EXPIRING' }),
  ];
  expect(sortedNodes(nodes).map((each) => each.name)).toEqual(['x-stranger-expiring', 'y-stranger', 'z-expired', 'a-fine', 'b-fine']);
});

it('SET-003: the age of a read is in seconds, minutes, hours or days', () => {
  const ago = (seconds: number) => readAgo(new Date(NOW.getTime() - seconds * 1000).toISOString(), NOW);
  expect([ago(40), ago(600), ago(7200), ago(3 * 86_400)]).toEqual(['Read 40 s ago', 'Read 10 min ago', 'Read 2 h ago', 'Read 3 days ago']);
  expect(readAgo('not a date', NOW)).toBe('Read at an unknown time');
});
