/**
 * The Headscale node list checked against the policy and the operator's expectations
 * (VPN-013, OD-10). Pure: it reads parsed JSON and a clock, and never keeps a key.
 */

import { runPolicyTests, type TailnetNode } from './tailnet.ts';
import type { Policy } from './policy.ts';

/** One node from `headscale nodes list -o json` (0.29.7), without any key. */
export interface ListedNode {
  readonly id: number;
  readonly name: string;
  readonly user: string;
  readonly tags: readonly string[];
  readonly ips: readonly string[];
  readonly online: boolean;
  readonly expiry: Date | null;
  /** The key's id, never the key (used by the broker, B1). */
  readonly preAuthKeyId: number | null;
}

/** One entry of `headscale nodes list-routes -o json`. */
export interface ListedRoutes {
  readonly id: number;
  readonly approved: readonly string[];
  readonly available: readonly string[];
}

export interface Tailnet {
  readonly nodes: readonly ListedNode[];
  readonly routes: readonly ListedRoutes[];
}

export type Finding =
  | { readonly kind: 'foreign-user'; readonly node: string; readonly user: string }
  | { readonly kind: 'unknown-tag'; readonly node: string; readonly tag: string }
  | { readonly kind: 'tagged-with-expiry'; readonly node: string }
  | { readonly kind: 'long-expiry'; readonly node: string; readonly days: number }
  | { readonly kind: 'unapproved-route'; readonly node: string; readonly route: string }
  | { readonly kind: 'unexpected-node'; readonly node: string }
  | { readonly kind: 'missing-node'; readonly node: string }
  | { readonly kind: 'policy-test-failed'; readonly test: string };

export class InventoryError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'InventoryError';
  }
}

const OPERATOR = 'operator';
const TAGGED_USER = 'tagged-devices';
const MAX_EXPIRY_DAYS = 180;
const MS_PER_DAY = 86_400_000;

type Json = Readonly<Record<string, unknown>>;

function object(value: unknown, what: string): Json {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) throw new InventoryError(`${what} is not an object`);
  return value as Json;
}

function array(value: unknown, what: string): readonly unknown[] {
  if (!Array.isArray(value)) throw new InventoryError(`${what} is not a list`);
  return value as readonly unknown[];
}

function strings(value: unknown, what: string): readonly string[] {
  const list = value === undefined ? [] : array(value, what);
  if (!list.every((entry) => typeof entry === 'string')) throw new InventoryError(`${what} holds a non-string`);
  return list as readonly string[];
}

// Headscale's JSON writes a uint64 as a string; accept both forms.
function count(value: unknown, what: string): number {
  const number = typeof value === 'string' && /^\d+$/.test(value) ? Number(value) : value;
  if (typeof number !== 'number' || !Number.isSafeInteger(number) || number < 0) throw new InventoryError(`${what} is not an id`);
  return number;
}

function text(value: unknown, what: string): string {
  if (typeof value !== 'string' || value === '') throw new InventoryError(`${what} is not a name`);
  return value;
}

function expiryOf(value: unknown): Date | null {
  if (value === undefined || value === null) return null;
  const seconds = count(object(value, 'expiry').seconds, 'expiry.seconds');
  return new Date(seconds * 1000);
}

function keyIdOf(value: unknown): number | null {
  if (value === undefined || value === null) return null;
  const key = object(value, 'pre_auth_key');
  return key.id === undefined ? null : count(key.id, 'pre_auth_key.id');
}

/** Parses the node list; drops every key field; throws InventoryError on a shape it does not know. */
export function parseNodes(json: unknown): readonly ListedNode[] {
  return array(json, 'nodes').map((entry) => {
    const node = object(entry, 'node');
    return {
      id: count(node.id, 'id'),
      name: text(node.name, 'name'),
      user: text(object(node.user, 'user').name, 'user.name'),
      tags: strings(node.tags, 'tags'),
      ips: strings(array(node.ip_addresses, 'ip_addresses'), 'ip_addresses'),
      online: node.online === true,
      expiry: expiryOf(node.expiry),
      preAuthKeyId: keyIdOf(node.pre_auth_key),
    };
  });
}

/** Parses `headscale nodes list-routes -o json`. */
export function parseRoutes(json: unknown): readonly ListedRoutes[] {
  return array(json, 'routes').map((entry) => {
    const row = object(entry, 'route entry');
    return {
      id: count(row.id, 'id'),
      approved: strings(row.approved_routes, 'approved_routes'),
      available: strings(row.available_routes, 'available_routes'),
    };
  });
}

/** The nodes as the policy evaluator sees them. */
export function tailnetNodes(tailnet: Tailnet): readonly TailnetNode[] {
  return tailnet.nodes.map((node) => {
    const ip = node.ips.find((address) => !address.includes(':'));
    if (ip === undefined) throw new InventoryError(`node ${node.name} has no IPv4 address`);
    const routes = tailnet.routes.find((entry) => entry.id === node.id)?.approved ?? [];
    return {
      name: node.name,
      identity: node.tags.length > 0 ? { tags: node.tags } : { user: node.user },
      ip,
      ...(routes.length > 0 ? { routes } : {}),
    };
  });
}

function nodeFindings(policy: Policy, node: ListedNode, routes: ListedRoutes | undefined, now: Date): readonly Finding[] {
  const tagged = node.tags.length > 0;
  const user = tagged ? TAGGED_USER : OPERATOR;
  const days = node.expiry === null ? null : Math.floor((node.expiry.getTime() - now.getTime()) / MS_PER_DAY);
  const found: Finding[] = [];
  if (node.user !== user) found.push({ kind: 'foreign-user', node: node.name, user: node.user });
  for (const tag of node.tags.filter((candidate) => !policy.scope.tags.has(candidate))) found.push({ kind: 'unknown-tag', node: node.name, tag });
  if (tagged && node.expiry !== null) found.push({ kind: 'tagged-with-expiry', node: node.name });
  if (!tagged && (days === null || days > MAX_EXPIRY_DAYS)) found.push({ kind: 'long-expiry', node: node.name, days: days ?? Infinity });
  for (const route of (routes?.available ?? []).filter((candidate) => !routes?.approved.includes(candidate))) found.push({ kind: 'unapproved-route', node: node.name, route });
  return found;
}

/** Every VPN-013 finding. `expected` is the list of node names; `now` is a parameter. */
export function findings(policy: Policy, tailnet: Tailnet, expected: readonly string[], now: Date): readonly Finding[] {
  const known = new Set(expected);
  const present = new Set(tailnet.nodes.map((node) => node.name));
  const perNode = tailnet.nodes.flatMap((node) => [
    ...nodeFindings(policy, node, tailnet.routes.find((entry) => entry.id === node.id), now),
    ...(known.has(node.name) ? [] : [{ kind: 'unexpected-node', node: node.name } as const]),
  ]);
  const missing = expected.filter((name) => !present.has(name)).map((node) => ({ kind: 'missing-node', node }) as const);
  const failed = runPolicyTests(policy, tailnetNodes(tailnet)).map((failure) => ({ kind: 'policy-test-failed', test: `${failure.src} ${failure.expected} ${failure.dst}: ${failure.reason}` }) as const);
  return [...perNode, ...missing, ...failed];
}
