import { expect, it } from 'vitest';
import { DETAIL } from '../containers.fixture';
import { healthRows, mountLine, networkLine, summaryRows } from './detail';

const value = (rows: readonly { label: string; value: string }[], label: string) => rows.find((row) => row.label === label)?.value;

it('HOST-017: the rows give the command, the restart policy and the last run', () => {
  const rows = summaryRows(DETAIL);
  expect(value(rows, 'Command')).toBe('web-entrypoint.sh bin/rails server');
  expect(value(rows, 'Restart policy')).toBe('on-failure:3');
  expect(value(rows, 'Restarts')).toBe('2');
  expect(value(rows, 'User')).toBe('The default of the image');
  // Docker gives no time for a container that never ended.
  expect(value(rows, 'Finished')).toBe('Never');
  expect(value(rows, 'Exit status')).toBe('0');
});

it('HOST-017: the health check has its test, its interval and its last result, and a container with none has no row', () => {
  const rows = healthRows(DETAIL);
  expect(value(rows, 'Test')).toBe('CMD-SHELL curl -f localhost:3000');
  expect(value(rows, 'Interval')).toBe('10 s');
  expect(value(rows, 'Health')).toBe('Healthy');
  expect(value(rows, 'Last run')).toContain('Exit status 0');
  expect(healthRows({ ...DETAIL, healthCheck: null })).toEqual([]);
  const stopped = healthRows({ ...DETAIL, health: 'NONE', lastHealthRun: null, healthCheck: { test: ['CMD', 'true'], intervalSeconds: null } });
  expect([value(stopped, 'Health'), value(stopped, 'Last run'), value(stopped, 'Interval')]).toEqual(['Not running', 'None', 'The default of Docker (30 s)']);
});

it('HOST-017: a mount names its volume or its path on the device, and a network gives the addresses', () => {
  expect(DETAIL.mounts.map(mountLine)).toEqual(['volume tracks-data → /data (read-write)', 'bind /srv/config → /config (read-only)']);
  expect(DETAIL.networks.map(networkLine)).toEqual(['tracks_default  172.18.0.3']);
  expect(networkLine({ network: 'host', ipv4: null, ipv6: null, gateway: null, macAddress: null })).toBe('host');
});
