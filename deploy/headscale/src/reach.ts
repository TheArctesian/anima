/**
 * The reachability table (VPN-004, VPN-012): for each pair of parties, each probed port and
 * each protocol, whether the policy allows the connection. Pure. It adds no evaluator: every
 * row is one `allows` call. A diff of two tables shows what a policy edit changed.
 */

import { contains, MAX_PORT, parseIpv4, type Policy } from './policy.ts';
import { allows, type Connection, type TailnetNode } from './tailnet.ts';

/** One class of node or LAN address that VPN-004 names. */
export type Party =
  | { readonly kind: 'client'; readonly node: string }
  | { readonly kind: 'hub'; readonly node: string }
  | { readonly kind: 'agent'; readonly node: string }
  | { readonly kind: 'router'; readonly node: string; readonly site: string }
  | { readonly kind: 'lan'; readonly site: string; readonly address: string }
  | { readonly kind: 'device'; readonly node: string };

export interface Reach {
  readonly from: Party;
  readonly to: Party;
  readonly port: number;
  readonly proto: 'tcp' | 'udp';
  readonly allowed: boolean;
}

const PROTOCOLS = ['tcp', 'udp'] as const;
const LAN_HOST_OFFSET = 5;

/** The ports that the table probes: 1, 65535, and for each grant `PortRule` its `first`, `last`,
 *  `first − 1` and `last + 1` (kept inside 1–65535), without repeats, sorted. */
export function probedPorts(policy: Policy): readonly number[] {
  const edges = policy.grants.flatMap((grant) => grant.ip.flatMap((rule) => [rule.first - 1, rule.first, rule.last, rule.last + 1]));
  const inside = [1, MAX_PORT, ...edges].filter((port) => port >= 1 && port <= MAX_PORT);
  return [...new Set(inside)].sort((a, b) => a - b);
}

/** Every party, from the nodes and the policy's site hosts. */
export function parties(policy: Policy, nodes: readonly TailnetNode[]): readonly Party[] {
  const hosts = [...policy.scope.hosts];
  const deviceNets = hosts.filter(([name]) => name.startsWith('device-')).map(([, prefix]) => prefix);
  const fromNodes = nodes.flatMap((node) => {
    const party = deviceNets.some((net) => contains(net, parseIpv4(node.ip))) ? ({ kind: 'device', node: node.name } as const) : roleOf(node);
    return party === undefined ? [] : [party];
  });
  const lans = hosts
    .filter(([name]) => name.startsWith('site-'))
    .map(([name, prefix]): Party => ({ kind: 'lan', site: name.slice('site-'.length), address: dotted(prefix.base + LAN_HOST_OFFSET) }));
  return [...fromNodes, ...lans];
}

/** The role of a node by its tags (BD-5); undefined for a tagged node of no known role. */
function roleOf(node: TailnetNode): Party | undefined {
  if ('user' in node.identity) return { kind: 'client', node: node.name };
  const tags = node.identity.tags;
  if (tags.includes('tag:hub')) return { kind: 'hub', node: node.name };
  if (tags.includes('tag:router')) {
    const site = tags.find((tag) => tag.startsWith('tag:site-'))?.slice('tag:site-'.length);
    return site === undefined ? undefined : { kind: 'router', node: node.name, site };
  }
  return tags.includes('tag:agent') ? { kind: 'agent', node: node.name } : undefined;
}

/** The full table: each pair of different parties, each probed port, tcp and udp. */
export function reachTable(policy: Policy, nodes: readonly TailnetNode[]): readonly Reach[] {
  const everyone = parties(policy, nodes);
  const ports = probedPorts(policy);
  return everyone.flatMap((from) =>
    everyone
      .filter((to) => label(to) !== label(from))
      .flatMap((to) => ports.flatMap((port) => PROTOCOLS.map((proto) => ({ from, to, port, proto, allowed: allows(policy, nodes, connection(from, to, port, proto)) })))),
  );
}

function connection(from: Party, to: Party, port: number, proto: string): Connection {
  return {
    from: from.kind === 'lan' ? { lan: from.address } : { node: from.node },
    to: to.kind === 'lan' ? { address: to.address } : { node: to.node },
    port,
    proto,
  };
}

/** The table as Markdown rows, sorted, one row for each allowed pair and protocol; for a diff. */
export function reachMarkdown(table: readonly Reach[]): string {
  const ports = new Map<string, number[]>();
  for (const row of table.filter((entry) => entry.allowed)) {
    const key = `| ${label(row.from)} | ${label(row.to)} | ${row.proto} |`;
    ports.set(key, [...(ports.get(key) ?? []), row.port]);
  }
  const rows = [...ports].map(([key, list]) => `${key} ${list.join(', ')} |`).sort();
  return ['| from | to | proto | ports |', '| --- | --- | --- | --- |', ...rows].join('\n') + '\n';
}

function label(party: Party): string {
  if (party.kind === 'lan') return `lan ${party.site} ${party.address}`;
  return party.kind === 'router' ? `router ${party.node} (${party.site})` : `${party.kind} ${party.node}`;
}

function dotted(ip: number): string {
  return [24, 16, 8, 0].map((shift) => String((ip >>> shift) & 0xff)).join('.');
}

const allowedRows = (table: readonly Reach[]): readonly Reach[] => table.filter((row) => row.allowed);

/** Allowed rows from a router of one site to a LAN of another site (VPN-012). */
export function crossSiteRows(table: readonly Reach[]): readonly Reach[] {
  return allowedRows(table).filter((row) => row.from.kind === 'router' && row.to.kind === 'lan' && row.from.site !== row.to.site);
}

/** Allowed rows from a LAN to a client (VPN-012). */
export function lanToClientRows(table: readonly Reach[]): readonly Reach[] {
  return allowedRows(table).filter((row) => row.from.kind === 'lan' && row.to.kind === 'client');
}

/** Allowed rows from an agent to anything but the hub on tcp 443 (VPN-012, MET-020). */
export function agentOverreachRows(table: readonly Reach[]): readonly Reach[] {
  return allowedRows(table).filter((row) => row.from.kind === 'agent' && !(row.to.kind === 'hub' && row.proto === 'tcp' && row.port === 443));
}

/** Allowed rows to a device /32 from anything but the hub, or on a protocol but tcp (VPN-012, R-061). */
export function deviceOverreachRows(table: readonly Reach[]): readonly Reach[] {
  return allowedRows(table).filter((row) => row.to.kind === 'device' && !(row.from.kind === 'hub' && row.proto === 'tcp'));
}
