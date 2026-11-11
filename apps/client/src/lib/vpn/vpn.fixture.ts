// Test support: a VPN picture with one routed site, a router, a client and a node Anima does not know.
import { flushSync } from 'svelte';
import type { VpnNode, VpnPicture } from './api';

export { fakeHub } from '$lib/devices/hub.fixture';

/** The time the fixture's keys are measured from. */
export const NOW = new Date('2026-11-10T12:00:00Z');

/** One node; `change` overrides any field. */
export function node(change: Partial<VpnNode> = {}): VpnNode {
  return {
    name: 'mac',
    address: '100.64.0.4',
    os: 'macOS',
    role: 'CLIENT',
    tags: [],
    routes: [],
    waitingRoutes: [],
    online: true,
    lastSeen: '2026-11-10T11:59:00Z',
    keyExpiry: '2027-05-12T00:00:00Z',
    keyState: 'VALID',
    path: 'DIRECT',
    relayRegion: null,
    inAnima: true,
    device: { id: '00000000-0000-4000-8000-000000000004', name: 'mac' },
    ...change,
  };
}

export const ROUTER = node({
  name: 'router-home',
  address: '100.64.0.2',
  os: 'linux',
  role: 'ROUTER',
  routes: ['192.168.1.0/24'],
  keyState: 'NEVER',
  keyExpiry: null,
  device: { id: '00000000-0000-4000-8000-000000000002', name: 'Home router' },
});

export const STRANGER = node({
  name: 'stranger',
  address: '100.64.0.5',
  keyState: 'EXPIRING',
  keyExpiry: '2026-11-19T12:00:00Z',
  inAnima: false,
  device: null,
});

/** A fresh picture; `change` overrides any field. */
export function picture(change: Partial<VpnPicture> = {}): VpnPicture {
  return {
    readAt: '2026-11-10T11:59:20Z',
    fresh: true,
    hub: node({ name: 'hub', address: '100.64.0.1', role: 'HUB', os: 'linux', keyState: 'NEVER', keyExpiry: null, device: null, inAnima: false }),
    sites: [
      {
        site: { id: 'site-home', name: 'Home', subnet: '192.168.1.0/24' },
        router: ROUTER,
        state: 'SERVING',
        path: 'DIRECT',
        relayRegion: null,
        hasPingCheck: false,
        waitingRouter: null,
      },
    ],
    nodes: [node(), ROUTER, STRANGER],
    broker: 'READY',
    ...change,
  };
}

export async function settled(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}
