/**
 * The tailnet as Anima's policy sees it (VPN-004): a node inventory, LAN addresses behind
 * subnet routers, and whether the policy permits a connection between them. Pure.
 */

import {
  checkPort,
  checkProto,
  contains,
  parseIpv4,
  parsePrefix,
  parseSelector,
  parseTarget,
  PolicyError,
  type Policy,
  type PortRule,
  type Prefix,
  type Selector,
} from './policy.ts';

/** A tailnet node: a personal node of a user, or a tagged node (VPN-003, D-1). */
export interface TailnetNode {
  readonly name: string;
  readonly identity: { readonly user: string } | { readonly tags: readonly string[] };
  readonly ip: string;
  /** Approved subnet routes; set only on subnet routers. */
  readonly routes?: readonly string[];
}

/** A node, or a LAN address that reaches the tailnet through its site's subnet router. */
export type Source = { readonly node: string } | { readonly lan: string };
export type Destination = { readonly node: string } | { readonly address: string };

export interface Connection {
  readonly from: Source;
  readonly to: Destination;
  readonly port: number;
  readonly proto?: string;
}

export interface TestFailure {
  readonly src: string;
  readonly dst: string;
  readonly expected: 'accept' | 'deny';
  readonly reason: string;
}

interface Party {
  readonly ip: number;
  readonly node?: TailnetNode;
}

interface Network {
  readonly nodes: readonly TailnetNode[];
  readonly ips: ReadonlyMap<number, TailnetNode>;
  readonly routes: readonly { readonly prefix: Prefix; readonly router: TailnetNode }[];
}

interface Packet {
  readonly proto: string;
  readonly port: number;
}

/** Whether the policy permits the connection; anything no grant names is denied. */
export function allows(policy: Policy, nodes: readonly TailnetNode[], connection: Connection): boolean {
  const network = resolveNetwork(nodes);
  const packet = { proto: checkProto(connection.proto ?? 'tcp'), port: checkPort(connection.port) };
  const { from, to } = connection;
  const sources = 'node' in from ? [partyOf(nodeNamed(network, from.node))] : lanParties(network, parseIpv4(from.lan));
  const target = 'node' in to ? partyOf(nodeNamed(network, to.node)) : partyAt(network, parseIpv4(to.address));
  return permits(policy, sources, target, packet);
}

/** Runs the policy's own `tests` block against an inventory; an empty result means all pass. */
export function runPolicyTests(policy: Policy, nodes: readonly TailnetNode[]): TestFailure[] {
  const network = resolveNetwork(nodes);
  return policy.tests.flatMap((test) => {
    const sources = testSources(parseSelector(test.src, policy.scope), network);
    const cases = [
      ...test.accept.map((dst) => ({ dst, expected: 'accept' as const })),
      ...test.deny.map((dst) => ({ dst, expected: 'deny' as const })),
    ];
    return cases.flatMap(({ dst, expected }) => {
      const reason = testCase(policy, network, { sources, dst, expected, proto: test.proto });
      return reason === undefined ? [] : [{ src: test.src, dst, expected, reason }];
    });
  });
}

interface TestCase {
  readonly sources: readonly (readonly Party[])[];
  readonly dst: string;
  readonly expected: 'accept' | 'deny';
  readonly proto: string;
}

function testCase(policy: Policy, network: Network, test: TestCase): string | undefined {
  const { host, port } = parseTarget(test.dst);
  const targets = testTargets(parseSelector(host, policy.scope), network);
  if (test.sources.length === 0) return 'source matches no node';
  if (targets.length === 0) return 'destination matches no node';
  for (const source of test.sources) {
    for (const target of targets) {
      const allowed = permits(policy, source, target, { proto: test.proto, port });
      if (allowed !== (test.expected === 'accept')) {
        const who = source[0]?.node?.name ?? 'an unroutable address';
        return `${who} is ${allowed ? 'allowed' : 'denied'} to ${target.node?.name ?? host}:${String(port)}`;
      }
    }
  }
  return undefined;
}

function permits(policy: Policy, sources: readonly Party[], target: Party, packet: Packet): boolean {
  return policy.grants.some(
    (grant) =>
      grant.ip.some((rule) => portMatches(rule, packet)) &&
      grant.dst.some((selector) => matches(selector, target)) &&
      sources.some((source) => grant.src.some((selector) => matches(selector, source))),
  );
}

function matches(selector: Selector, party: Party): boolean {
  const identity = party.node?.identity;
  if (selector.kind === 'prefix') return contains(selector.prefix, party.ip);
  if (identity === undefined) return false;
  if (selector.kind === 'tag') return 'tags' in identity && identity.tags.includes(selector.tag);
  return 'user' in identity && selector.names.includes(identity.user);
}

function portMatches(rule: PortRule, packet: Packet): boolean {
  return (rule.proto === '*' || rule.proto === packet.proto) && packet.port >= rule.first && packet.port <= rule.last;
}

// Subnet routers masquerade LAN traffic by default, so a LAN source is checked both as
// its own address and as its router's identity; either one being permitted allows it.
function lanParties(network: Network, ip: number): Party[] {
  const routers = network.routes.filter((route) => contains(route.prefix, ip)).map((route) => route.router);
  return routers.length === 0 ? [] : [{ ip }, ...routers.map(partyOf)];
}

function partyAt(network: Network, ip: number): Party {
  const node = network.ips.get(ip);
  return node === undefined ? { ip } : partyOf(node);
}

function partyOf(node: TailnetNode): Party {
  return { ip: parseIpv4(node.ip), node };
}

function testSources(selector: Selector, network: Network): Party[][] {
  if (selector.kind !== 'prefix') return network.nodes.filter((node) => matches(selector, partyOf(node))).map((node) => [partyOf(node)]);
  const ip = singleAddress(selector.prefix);
  const node = network.ips.get(ip);
  if (node !== undefined) return [[partyOf(node)]];
  const parties = lanParties(network, ip);
  return parties.length === 0 ? [] : [parties];
}

function testTargets(selector: Selector, network: Network): Party[] {
  if (selector.kind !== 'prefix') return network.nodes.map(partyOf).filter((party) => matches(selector, party));
  return [partyAt(network, singleAddress(selector.prefix))];
}

function singleAddress(prefix: Prefix): number {
  if (prefix.bits !== 32) throw new PolicyError('invalid-shape', 'a test address must be a single IP, not a subnet');
  return prefix.base;
}

function resolveNetwork(nodes: readonly TailnetNode[]): Network {
  const ips = new Map<number, TailnetNode>();
  const names = new Set<string>();
  for (const node of nodes) {
    const ip = parseIpv4(node.ip);
    if (names.has(node.name) || ips.has(ip)) throw new PolicyError('invalid-shape', `duplicate node ${node.name}`);
    if ('tags' in node.identity && node.identity.tags.length === 0) throw new PolicyError('invalid-shape', `node ${node.name} has neither a user nor tags`);
    names.add(node.name);
    ips.set(ip, node);
  }
  const routes = nodes.flatMap((router) => (router.routes ?? []).map((route) => ({ prefix: parsePrefix(route), router })));
  return { nodes, ips, routes };
}

function nodeNamed(network: Network, name: string): TailnetNode {
  const node = network.nodes.find((candidate) => candidate.name === name);
  if (node === undefined) throw new PolicyError('unknown-node', name);
  return node;
}

