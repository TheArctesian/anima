// `headscale nodes list -o json` and `nodes list-routes -o json` (0.29.7) with invented names.
// Keys use the placeholders of vpn-build.md rule 0.6.
import { parseNodes, parseRoutes, type Tailnet } from '../src/inventory.ts';

export const NOW = new Date('2026-11-10T00:00:00Z');
const DAY_SECONDS = 86_400;
const inDays = (days: number): { seconds: number; nanos: number } => ({ seconds: NOW.getTime() / 1000 + days * DAY_SECONDS, nanos: 0 });
const key = (id: number): { id: number; key: string } => ({ id, key: 'REDACTED-PREAUTH-KEY' });
const keys = (n: number): Record<string, string> => ({
  machine_key: `mkey:${n.toString(16).padStart(64, '0')}`,
  node_key: `nodekey:${n.toString(16).padStart(64, '0')}`,
  disco_key: `discokey:${n.toString(16).padStart(64, '0')}`,
});

function node(id: number, name: string, user: string, extra: Record<string, unknown>): Record<string, unknown> {
  return { id, name, user: { id: 1, name: user }, ip_addresses: [`100.64.0.${String(id)}`, `fd7a:115c:a1e0::${String(id)}`], ...keys(id), ...extra };
}

export const rawNodes = [
  node(1, 'hub', 'tagged-devices', { tags: ['tag:hub'], online: true, pre_auth_key: key(1) }),
  node(2, 'mac', 'operator', { online: true, expiry: inDays(170) }),
  node(3, 'router-home', 'tagged-devices', { tags: ['tag:router', 'tag:site-home'], online: true, pre_auth_key: key(3) }),
  node(4, 'stranger', 'intruder', { expiry: inDays(30), pre_auth_key: key(4) }),
  node(5, 'odd', 'tagged-devices', { tags: ['tag:unknown'], pre_auth_key: key(5) }),
  node(6, 'router-office1', 'tagged-devices', { tags: ['tag:router', 'tag:site-office1'], pre_auth_key: key(6) }),
  node(7, 'agent-nas', 'tagged-devices', { tags: ['tag:agent'], online: true, expiry: inDays(90), pre_auth_key: key(7) }),
  node(8, 'mac2', 'operator', { online: true }),
];

export const rawRoutes = [
  { id: 3, approved_routes: ['10.10.0.0/24'], available_routes: ['10.10.0.0/24'], subnet_routes: ['10.10.0.0/24'] },
  { id: 6, available_routes: ['10.20.0.0/24'] },
];

export const tailnet: Tailnet = { nodes: parseNodes(rawNodes), routes: parseRoutes(rawRoutes) };
export const expected = ['hub', 'mac', 'router-home', 'router-office1', 'agent-nas', 'mac2'];

/** The tailnet without its bad nodes: hub, a client within 180 days, the home router. */
export const clean: Tailnet = {
  nodes: tailnet.nodes.filter((entry) => ['hub', 'mac', 'router-home'].includes(entry.name)),
  routes: tailnet.routes.filter((entry) => entry.id === 3),
};
export const cleanExpected = ['hub', 'mac', 'router-home'];
