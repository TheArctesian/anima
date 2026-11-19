import { expect, it } from 'vitest';
import { indexOf } from './index';

const DESKTOP = { keyStore: true, stayAwake: true, desktopRows: true, mcpPermissions: true };

it('R-211: the index has VPN for a hub with a VPN, and leaves it out for a hub that Anima.ing hosts', () => {
  expect(indexOf({ ...DESKTOP, vpn: true }).map((entry) => entry.title)).toContain('VPN');
  expect(indexOf({ ...DESKTOP, vpn: false }).map((entry) => entry.title)).not.toContain('VPN');
});
