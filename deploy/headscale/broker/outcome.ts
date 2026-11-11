/**
 * Checks what a broker change left in Headscale and undoes a wrong result (vpn-plan.md
 * Section 5.1, VPN-003): a personal node must have no tag and an expiry of at most 180 days; a
 * router node must have exactly its two tags and no expiry. Only nodes that joined with a key
 * this broker issued, and the node it just registered, are checked. Pure; time is a parameter.
 */

import type { ListedNode, Tailnet } from '../src/inventory.ts';
import type { Step } from './verbs.ts';

export interface Issued {
  readonly keyId: number;
  readonly verb: 'personal-key' | 'router-key';
  readonly site?: string;
  /** ISO time the key was made. */
  readonly at: string;
  readonly passed: boolean;
}

export type Fix =
  | { readonly kind: 'delete'; readonly node: number; readonly reason: string }
  | { readonly kind: 'set-expiry'; readonly node: number; readonly expiry: Date };

export interface Audit {
  readonly fixes: readonly Fix[];
  readonly issued: readonly Issued[];
}

/** The fields of a node that the stagger reads. */
export type Expiring = Pick<ListedNode, 'user' | 'tags' | 'expiry'>;

const DAY_MS = 86_400_000;
const MAX_EXPIRY_DAYS = 180;
const STAGGER_DAYS = [180, 150, 120, 90];
const STAGGER_GAP_DAYS = 30;
const ISSUED_WAIT_MS = 2 * 3_600_000;
const OPERATOR = 'operator';
const ROUTER_TAG = 'tag:router';

/**
 * A key expiry 180, 150, 120 or 90 days after `now`: the first that is not within 30 days of
 * the expiry of another untagged `operator` node, so two computers do not expire together
 * (vpn.md 6B step 5). 90 days if none is free.
 */
export function staggered(now: Date, nodes: readonly Expiring[]): Date {
  const taken = nodes.flatMap((node) => (node.user === OPERATOR && node.tags.length === 0 && node.expiry !== null ? [node.expiry.getTime()] : []));
  const free = STAGGER_DAYS.map((days) => now.getTime() + days * DAY_MS).find((candidate) => taken.every((time) => Math.abs(time - candidate) >= STAGGER_GAP_DAYS * DAY_MS));
  return new Date(free ?? now.getTime() + (STAGGER_DAYS.at(-1) ?? MAX_EXPIRY_DAYS) * DAY_MS);
}

/**
 * The `--expiry` value: RFC 3339 in UTC, whole seconds. Never empty: an empty `--expiry`
 * expires the node at once (vpn.md 6B step 4).
 */
export function expiryValue(expiry: Date): string {
  if (Number.isNaN(expiry.getTime())) throw new RangeError('the expiry is not a time');
  return expiry.toISOString().replace(/\.\d{3}Z$/, 'Z');
}

/** The Headscale step that applies a fix. */
export function fixStep(fix: Fix): Step {
  const node = `--identifier=${String(fix.node)}`;
  return fix.kind === 'delete' ? ['nodes', 'delete', node, '--force'] : ['nodes', 'expire', node, `--expiry=${expiryValue(fix.expiry)}`];
}

interface Check {
  readonly now: Date;
  /** Every node, with the expiries already set by earlier fixes of this audit. */
  readonly nodes: ListedNode[];
}

function personalFix(listed: ListedNode, check: Check): Fix | null {
  const node = check.nodes.find((other) => other.id === listed.id) ?? listed;
  if (node.tags.length > 0) return { kind: 'delete', node: node.id, reason: 'the node had a tag' };
  const limit = check.now.getTime() + MAX_EXPIRY_DAYS * DAY_MS;
  if (node.expiry !== null && node.expiry.getTime() <= limit) return null;
  const expiry = staggered(check.now, check.nodes.filter((other) => other.id !== node.id));
  const index = check.nodes.findIndex((other) => other.id === node.id);
  if (index !== -1) check.nodes[index] = { ...node, expiry };
  return { kind: 'set-expiry', node: node.id, expiry };
}

function routerFix(node: ListedNode, site: string | undefined): Fix | null {
  const tags = [...node.tags].sort();
  const expected = [ROUTER_TAG, `tag:site-${site ?? ''}`].sort();
  if (tags.length !== expected.length || tags.some((tag, index) => tag !== expected[index])) return { kind: 'delete', node: node.id, reason: 'the router had other tags' };
  if (node.expiry !== null) return { kind: 'delete', node: node.id, reason: 'the router had a key expiry' };
  return null;
}

function issuedFix(entry: Issued, node: ListedNode, check: Check): Fix | null {
  return entry.verb === 'personal-key' ? personalFix(node, check) : routerFix(node, entry.site);
}

/** Checks the nodes that joined with keys this broker issued, and the node just registered. */
export function audit(tailnet: Tailnet, issued: readonly Issued[], registered: number | null, now: Date): Audit {
  const check: Check = { now, nodes: [...tailnet.nodes] };
  const fixes = new Map<number, Fix>();
  const add = (fix: Fix | null): void => {
    if (fix !== null && fixes.get(fix.node)?.kind !== 'delete') fixes.set(fix.node, fix);
  };
  const kept = issued.flatMap((entry): Issued[] => {
    const old = now.getTime() - Date.parse(entry.at) > ISSUED_WAIT_MS;
    // A passed entry is never checked again, so a later operator change is never undone.
    if (entry.passed) return old ? [] : [entry];
    const node = tailnet.nodes.find((candidate) => candidate.preAuthKeyId === entry.keyId);
    if (node === undefined) return old ? [] : [entry];
    const fix = issuedFix(entry, node, check);
    add(fix);
    return [{ ...entry, passed: fix?.kind !== 'delete' }];
  });
  const node = tailnet.nodes.find((candidate) => candidate.id === registered);
  if (node !== undefined) add(personalFix(node, check));
  return { fixes: [...fixes.values()], issued: kept };
}
