// Nodes and the policy view for the broker tests, with invented names (vpn-build.md rule 0.6).
import type { ListedNode, Tailnet } from '../src/inventory.ts';
import { policyView } from '../broker/verbs.ts';
import { policy } from './policy-fixture.ts';

export const NOW = new Date('2026-11-10T00:00:00Z');
export const DAY_MS = 86_400_000;
export const inDays = (days: number, from: Date = NOW): Date => new Date(from.getTime() + days * DAY_MS);

export const view = policyView(policy);

/** A personal node of `operator`, unless `fields` say otherwise. */
export function listed(id: number, fields: Partial<ListedNode> = {}): ListedNode {
  return {
    id,
    name: `node${String(id)}`,
    user: 'operator',
    tags: [],
    ips: [`100.64.0.${String(id)}`, `fd7a:115c:a1e0::${String(id)}`],
    online: true,
    expiry: inDays(170),
    preAuthKeyId: null,
    ...fields,
  };
}

/** A router node of a site, from the pre-auth key `keyId`. */
export function router(id: number, site: string, keyId: number | null = null): ListedNode {
  return listed(id, { user: 'tagged-devices', tags: ['tag:router', `tag:site-${site}`], expiry: null, preAuthKeyId: keyId });
}

export const tailnetOfNodes = (nodes: readonly ListedNode[], routes: Tailnet['routes'] = []): Tailnet => ({ nodes, routes });
