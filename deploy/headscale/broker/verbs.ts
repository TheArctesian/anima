/**
 * Each broker request as the Headscale command lines it runs (R-055, BD-9, VP-10). Pure.
 * Each flag and its value are one argv element (`--auth-id=<id>`), so a value can never be
 * read as a flag; no step makes a reusable or ephemeral key.
 */

import { prefixText, type Policy } from '../src/policy.ts';
import { OPERATOR, type BrokerNode } from './nodes.ts';
import { expiryValue, staggered } from './outcome.ts';
import type { Refusal, Request } from './protocol.ts';

export interface PolicyView {
  /** Site name `x` → the subnet of the policy host `site-<x>`. */
  readonly sites: ReadonlyMap<string, string>;
  readonly tags: ReadonlySet<string>;
}

/** The argv after `headscale`; one element for each flag=value. */
export type Step = readonly string[];

/** What a plan needs besides the request, the policy and the nodes. */
export interface Moment {
  readonly now: Date;
  /** The `operator` user's id (`users list`); `preauthkeys create --user` takes an id. */
  readonly operatorId: number | null;
}

const JSON_OUTPUT = '--output=json';
const SITE_HOST = /^site-([a-z0-9-]{1,32})$/;
const ROUTER_TAG = 'tag:router';

export const LIST_STEPS: readonly Step[] = [
  ['nodes', 'list', JSON_OUTPUT],
  ['nodes', 'list-routes', JSON_OUTPUT],
];
export const USERS_STEP: Step = ['users', 'list', JSON_OUTPUT];

export function siteTag(site: string): string {
  return `tag:site-${site}`;
}

/** The sites and tags of the policy (`policy.scope.hosts` and `policy.scope.tags`). */
export function policyView(policy: Policy): PolicyView {
  const sites = new Map<string, string>();
  for (const [host, prefix] of policy.scope.hosts) {
    const site = SITE_HOST.exec(host)?.[1];
    if (site !== undefined) sites.set(site, prefixText(prefix));
  }
  return { sites, tags: policy.scope.tags };
}

function routerKey(site: string, policy: PolicyView): readonly Step[] | Refusal {
  // BD-9: Headscale accepts a tag the policy does not know, so the broker checks it first.
  if (!policy.sites.has(site) || !policy.tags.has(siteTag(site)) || !policy.tags.has(ROUTER_TAG)) return 'unknown-site';
  return [['preauthkeys', 'create', `--tags=${ROUTER_TAG},${siteTag(site)}`, '--expiration=1h', JSON_OUTPUT]];
}

function approveRoute(request: { node: number; site: string }, policy: PolicyView, nodes: readonly BrokerNode[]): readonly Step[] | Refusal {
  const subnet = policy.sites.get(request.site);
  if (subnet === undefined) return 'unknown-site';
  const node = nodes.find((candidate) => candidate.id === request.node);
  if (node === undefined || !node.tags.includes(ROUTER_TAG) || !node.tags.includes(siteTag(request.site))) return 'not-allowed';
  if (node.available.length !== 1 || node.available[0] !== subnet) return 'not-allowed';
  return [['nodes', 'approve-routes', `--identifier=${String(node.id)}`, `--routes=${subnet}`]];
}

function renew(nodeId: number, nodes: readonly BrokerNode[], now: Date): readonly Step[] | Refusal {
  const node = nodes.find((candidate) => candidate.id === nodeId);
  if (node === undefined || node.tags.length > 0 || node.user !== OPERATOR) return 'not-allowed';
  const others = nodes
    .filter((other) => other.id !== nodeId)
    .map((other) => ({ ...other, expiry: other.expiry === null ? null : new Date(other.expiry) }));
  return [['nodes', 'expire', `--identifier=${String(nodeId)}`, `--expiry=${expiryValue(staggered(now, others))}`]];
}

/** The steps for a request, or the refusal; checks every value against the policy and the nodes. */
export function plan(request: Request, policy: PolicyView, nodes: readonly BrokerNode[], moment: Moment): readonly Step[] | Refusal {
  switch (request.verb) {
    case 'nodes':
      return LIST_STEPS;
    case 'personal-key':
      if (moment.operatorId === null) return 'headscale-failed';
      return [['preauthkeys', 'create', `--user=${String(moment.operatorId)}`, '--expiration=1h', JSON_OUTPUT]];
    case 'router-key':
      return routerKey(request.site, policy);
    case 'approve-route':
      return approveRoute(request, policy, nodes);
    case 'register':
      // `auth register --user` takes the user's name (unlike `preauthkeys create`; Build notes).
      return [['auth', 'register', `--user=${OPERATOR}`, `--auth-id=${request.authId}`, JSON_OUTPUT]];
    case 'renew':
      return renew(request.node, nodes, moment.now);
  }
}
