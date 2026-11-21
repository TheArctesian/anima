import { expect, it } from 'vitest';
import { enumWords, machineTest, markWords } from './entry';

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
