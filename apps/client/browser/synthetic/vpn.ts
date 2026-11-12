import type { Row } from './fixtures';

// A VPN picture and the broker's answers, so the Settings VPN section and its dialogs have something to show. Every host is an
// example host and every key a placeholder (CON-003, LOG-007).

const ONE_HOUR_MS = 3_600_000;

function node(name: string, address: string, change: Row = {}): Row {
  return {
    name,
    address,
    os: 'linux',
    role: 'CLIENT',
    tags: [],
    routes: [],
    waitingRoutes: [],
    online: true,
    lastSeen: '2026-11-10T11:59:00Z',
    keyExpiry: '2027-03-12T00:00:00Z',
    keyState: 'VALID',
    path: 'DIRECT',
    relayRegion: null,
    inAnima: true,
    device: null,
    ...change,
  };
}

function sitePath(id: string, name: string, subnet: string, change: Row): Row {
  return {
    site: { id, name, subnet },
    router: null,
    state: 'NO_ROUTER',
    path: 'UNKNOWN',
    relayRegion: null,
    hasPingCheck: false,
    waitingRouter: null,
    ...change,
  };
}

/** The hub, a client, a router that serves one site, a router that waits for approval, and a node Anima does not know. */
export function vpnPicture(): Row {
  const unkeyed = { keyState: 'NEVER', keyExpiry: null };
  const router = node('router-lab', '100.64.0.2', { role: 'ROUTER', routes: ['10.40.0.0/24'], ...unkeyed });
  const waiting = node('router-annex', '100.64.0.6', { role: 'ROUTER', waitingRoutes: ['10.50.0.0/24'], ...unkeyed });
  const stranger = node('stranger', '100.64.0.5', { keyState: 'EXPIRING', keyExpiry: '2026-11-19T12:00:00Z', inAnima: false });
  return {
    readAt: '2026-11-10T11:59:20Z',
    fresh: true,
    hub: node('hub', '100.64.0.1', { role: 'HUB', inAnima: false, ...unkeyed }),
    sites: [
      sitePath('lab', 'Lab', '10.40.0.0/24', { router, state: 'SERVING', path: 'DIRECT' }),
      sitePath('annex', 'Annex', '10.50.0.0/24', { waitingRouter: waiting }),
      sitePath('garage', 'Garage', '10.60.0.0/24', {}),
    ],
    nodes: [node('mac', '100.64.0.4', { os: 'macOS' }), router, waiting, stranger],
    broker: 'READY',
  };
}

const KEY_STEPS = ['curl -fsSL https://hub.anima.ing/vpn/join.sh -o join.sh', 'sudo sh join.sh'];

/** The Query and Mutation fields the VPN needs; each key is a placeholder that is no credential. */
export function vpnFields(now: () => Date): Record<string, unknown> {
  const key = (): Row => ({ key: 'tskey-synthetic-not-a-secret', expiresAt: new Date(now().getTime() + ONE_HOUR_MS).toISOString(), steps: KEY_STEPS });
  return {
    vpn: () => vpnPicture(),
    createVpnInvite: key,
    createRouterKey: key,
    registerVpnNode: () => ({ name: 'mac-studio', address: '100.64.0.9', keyExpiry: '2027-03-01T00:00:00Z' }),
  };
}
