import { expect, it } from 'vitest';
import { revokeWarning } from '../revoke';

it('MET-013, R-105: on a hub with a VPN the warning says that the VPN access stays and names the Headscale node', () => {
  expect(revokeWarning(false)).toContain('VPN access stays');
  expect(revokeWarning(false)).toContain('also delete its node in Headscale');
});

it('R-211: on a hub that Anima.ing hosts the warning names no VPN and no Headscale', () => {
  expect(revokeWarning(true)).toBe('The daemon stops sending metrics. The device keeps the Daemon source, shown as stale, until you change it.');
});
