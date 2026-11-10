import { describe, expect, test } from 'bun:test';
import { parsePolicy, PolicyError, type PolicyErrorCode } from '../src/policy.ts';
import { allows, runPolicyTests, type TailnetNode } from '../src/tailnet.ts';
import { AGENT_ROWS, allowed, everyone, nodes, policy, raw, ROUTER_ROWS, SITES, tcp, withGrants, withTests, type From, type To } from './policy-fixture.ts';

describe('policy tests block', () => {
  test('vpn_004 every row of the policy tests block passes in the evaluator', () => {
    expect(policy.tests.length).toBeGreaterThan(0);
    expect(runPolicyTests(policy, nodes)).toEqual([]);
  });

  test('vpn_004 the tag:hub rows name the Mac on SSH and VNC and deny RDP (R-061)', () => {
    const hubRows = policy.tests.filter((row) => row.src === 'tag:hub');
    const tcpRow = hubRows.find((row) => row.protos.length === 1 && row.protos[0] === 'tcp');
    const anyProtoRow = hubRows.find((row) => row.protos.length > 1);
    expect(tcpRow?.accept).toEqual(expect.arrayContaining(['100.64.0.2:22', '100.64.0.2:5900']));
    expect(tcpRow?.deny).toContain('100.64.0.2:3389');
    expect(anyProtoRow?.deny).toContain('100.64.0.2:3389');
  });

  test('vpn_004 every router row waiting for its router passes in the evaluator', () => {
    expect(runPolicyTests(withTests(ROUTER_ROWS), nodes)).toEqual([]);
  });

  test('vpn_004 the policy tests block names only identities that exist before any router enrols', () => {
    const named = policy.tests.flatMap((row) => [row.src, ...[...(row.accept ?? []), ...(row.deny ?? [])]]);
    expect(named.filter((selector) => selector.startsWith('tag:') && !selector.startsWith('tag:hub'))).toEqual([]);
    expect(named.filter((selector) => /^10\./.test(selector) && !selector.includes(':'))).toEqual([]);
  });

  test('vpn_004 the tests block runner reports a row the policy contradicts', () => {
    expect(runPolicyTests(withTests([{ src: 'tag:hub', proto: 'tcp', accept: ['operator@:443'] }]), nodes)).toEqual([
      { src: 'tag:hub', dst: 'operator@:443', expected: 'accept', reason: 'hub is denied tcp to mac:443' },
    ]);
  });

  test('vpn_004 a row without proto fails when a reverse grant opens only udp', () => {
    const reverse = { src: ['tag:hub', 'site-home'], dst: ['100.64.0.11'], ip: ['udp:*'] };
    const failures = runPolicyTests(withGrants([...(raw.grants as unknown[]), reverse]), nodes);
    expect(failures.map((failure) => failure.reason)).toContain('hub is allowed udp to laptop:53');
  });

  test('vpn_004 a test row whose source matches no node fails instead of passing vacuously', () => {
    expect(runPolicyTests(withTests([{ src: 'tag:site-office1', deny: ['operator@:22'] }]), nodes)[0]?.reason).toBe('source matches no node');
  });
});

describe('agent devices (MET-020, R-084)', () => {
  const probe = (from: From, to: To, port: number, proto: string): boolean => allows(policy, everyone, { from, to, port, proto });

  test('met_020 every agent-tag row passes in the evaluator', () => {
    expect(runPolicyTests(withTests(AGENT_ROWS), nodes)).toEqual([]);
  });

  test('met_020 an agent reaches tcp 443 of the hub', () => {
    expect(probe({ node: 'agent' }, { node: 'hub' }, 443, 'tcp')).toBe(true);
    expect(probe({ node: 'agent2' }, { node: 'hub' }, 443, 'tcp')).toBe(true);
  });

  test('met_020 an agent is refused every other hub port and protocol', () => {
    for (const port of [22, 80, 5432, 8080, 5900]) expect(probe({ node: 'agent' }, { node: 'hub' }, port, 'tcp')).toBe(false);
    for (const proto of ['udp', 'icmp']) expect(probe({ node: 'agent' }, { node: 'hub' }, 443, proto)).toBe(false);
  });

  test('met_020 an agent reaches no site, device, client, router or other agent', () => {
    for (const port of [22, 443, 3389, 5900]) {
      for (const address of ['10.10.0.5', '10.20.0.5', '10.30.0.5']) expect(probe({ node: 'agent' }, { address }, port, 'tcp')).toBe(false);
      for (const node of ['mac', 'laptop', 'router-home', 'router-office2', 'agent2']) expect(probe({ node: 'agent' }, { node }, port, 'tcp')).toBe(false);
    }
  });

  test('met_020 nothing reaches an agent device: the agent listens on no port', () => {
    for (const from of ['hub', 'mac', 'laptop', 'router-home', 'agent2']) {
      for (const port of [22, 443, 5900]) {
        for (const proto of ['tcp', 'udp', 'icmp']) expect(probe({ node: from }, { node: 'agent' }, port, proto)).toBe(false);
      }
    }
    expect(probe({ lan: '10.10.0.5' }, { node: 'agent' }, 443, 'tcp')).toBe(false);
  });
});

describe('permitted connections (VPN-004)', () => {
  test('vpn_004 client reaches hub on 443 and 22', () => {
    expect(allowed(tcp({ node: 'mac' }, { node: 'hub' }, 443))).toBe(true);
    expect(allowed(tcp({ node: 'laptop' }, { node: 'hub' }, 22))).toBe(true);
  });

  test('vpn_004 client reaches devices at every site', () => {
    for (const address of ['10.10.0.5', '10.20.0.5', '10.30.0.5']) expect(allowed(tcp({ node: 'mac' }, { address }, 3389))).toBe(true);
  });

  test('vpn_004 hub reaches the Mac added as a device on SSH and VNC only (R-061)', () => {
    for (const port of [22, 5900]) expect(allowed(tcp({ node: 'hub' }, { node: 'mac' }, port))).toBe(true);
    for (const port of [443, 3389, 5901, 5432]) expect(allowed(tcp({ node: 'hub' }, { node: 'mac' }, port))).toBe(false);
    for (const port of [22, 5900]) {
      expect(allowed(tcp({ node: 'hub' }, { node: 'laptop' }, port))).toBe(false);
      expect(allowed(tcp({ node: 'router-home' }, { node: 'mac' }, port))).toBe(false);
      expect(allowed(tcp({ lan: '10.10.0.5' }, { node: 'mac' }, port))).toBe(false);
    }
    for (const proto of ['udp', 'icmp']) {
      expect(allows(policy, nodes, { from: { node: 'hub' }, to: { node: 'mac' }, port: 5900, proto })).toBe(false);
    }
  });

  test('vpn_004 hub reaches devices at every site', () => {
    for (const address of ['10.10.0.5', '10.20.0.254', '10.30.0.1']) expect(allowed(tcp({ node: 'hub' }, { address }, 161))).toBe(true);
  });

  test('vpn_004 each router reaches its own subnet', () => {
    expect(allowed(tcp({ node: 'router-home' }, { address: '10.10.0.5' }, 22))).toBe(true);
    expect(allowed(tcp({ node: 'router-office2' }, { address: '10.30.0.5' }, 22))).toBe(true);
  });

  test('vpn_004 an office 1 router would reach only office 1', () => {
    const office1: TailnetNode = { name: 'router-office1', identity: { tags: ['tag:router', 'tag:site-office1'] }, ip: '100.64.0.4', routes: [SITES.office1] };
    const inventory = [...nodes, office1];
    expect(allowed(tcp({ node: 'router-office1' }, { address: '10.20.0.5' }, 22), policy, inventory)).toBe(true);
    expect(allowed(tcp({ node: 'router-office1' }, { address: '10.10.0.5' }, 22), policy, inventory)).toBe(false);
  });

  test('vpn_004 a site device to its own subnet is allowed through the router identity (never a tailnet path)', () => {
    expect(allowed(tcp({ lan: '10.10.0.5' }, { address: '10.10.0.6' }, 22))).toBe(true);
  });
});

for (const proto of ['tcp', 'udp', 'icmp']) {
  describe(`denied connections over ${proto} (VPN-004)`, () => {
    const denied = (from: From, to: To, port = 22): boolean => !allows(policy, nodes, { from, to, port, proto });

    test(`vpn_004 client is denied other hub ports such as 5432 (${proto})`, () => {
      expect(denied({ node: 'mac' }, { node: 'hub' }, 5432)).toBe(true);
      if (proto !== 'tcp') expect(denied({ node: 'mac' }, { node: 'hub' }, 443)).toBe(true);
    });

    test(`vpn_004 a router is denied the other sites (${proto})`, () => {
      expect(denied({ node: 'router-home' }, { address: '10.30.0.5' })).toBe(true);
      expect(denied({ node: 'router-home' }, { address: '10.20.0.5' })).toBe(true);
      expect(denied({ node: 'router-office2' }, { address: '10.10.0.5' })).toBe(true);
    });

    test(`vpn_004 routers cannot start a connection to a client (${proto})`, () => {
      for (const router of ['router-home', 'router-office2']) {
        for (const port of [22, 53, 443]) expect(denied({ node: router }, { node: 'mac' }, port)).toBe(true);
      }
    });

    test(`vpn_004 a site device cannot start a connection to a client (${proto})`, () => {
      expect(denied({ lan: '10.10.0.5' }, { node: 'mac' })).toBe(true);
      expect(denied({ lan: '10.30.0.5' }, { address: '100.64.0.11' }, 443)).toBe(true);
    });

    test(`vpn_004 a site device cannot reach the hub or another site (${proto})`, () => {
      expect(denied({ lan: '10.10.0.5' }, { node: 'hub' }, 443)).toBe(true);
      expect(denied({ lan: '10.10.0.5' }, { address: '10.30.0.5' })).toBe(true);
      expect(denied({ lan: '10.30.0.5' }, { address: '10.10.0.5' })).toBe(true);
    });

    test(`vpn_004 a LAN address with no router never enters the tailnet (${proto})`, () => {
      expect(denied({ lan: '10.20.0.5' }, { address: '10.10.0.5' })).toBe(true);
    });

    test(`vpn_004 hub cannot start a connection to a client or a router (${proto})`, () => {
      expect(denied({ node: 'hub' }, { node: 'laptop' })).toBe(true);
      expect(denied({ node: 'hub' }, { node: 'mac' }, 443)).toBe(true);
      if (proto !== 'tcp') expect(denied({ node: 'hub' }, { node: 'mac' })).toBe(true);
      expect(denied({ node: 'hub' }, { node: 'laptop' }, 443)).toBe(true);
      expect(denied({ node: 'hub' }, { node: 'router-home' })).toBe(true);
    });

    test(`vpn_004 clients cannot reach other clients or routers (${proto})`, () => {
      expect(denied({ node: 'mac' }, { node: 'laptop' })).toBe(true);
      expect(denied({ node: 'mac' }, { node: 'router-home' })).toBe(true);
    });
  });
}

// Malformed input the parser and evaluator must refuse, with the error code each raises.
const valid = { src: ['operator@'], dst: ['site-home'], ip: ['*'] };
const grant = (change: Record<string, unknown>): (() => unknown) => () => withGrants([{ ...valid, ...change }]);
const tests = (row: Record<string, unknown>): (() => unknown) => () => withTests([row]);
const run = (row: Record<string, unknown>): (() => unknown) => () => runPolicyTests(withTests([row]), nodes);
const at = (from: From, to: To, port = 22, inventory = nodes): (() => unknown) => () => allows(policy, inventory, { from, to, port });
const hubLike = (name: string): TailnetNode => ({ name, identity: { tags: ['tag:hub'] }, ip: '100.64.0.9' });
const refused: [string, () => unknown, PolicyErrorCode][] = [
  ['a policy that is not an object', () => parsePolicy([]), 'invalid-shape'],
  ['a missing grants list (Headscale would allow everything)', () => parsePolicy({ ...raw, grants: undefined }), 'invalid-shape'],
  ['an empty grants list', () => withGrants([]), 'invalid-shape'],
  ['legacy acls', () => parsePolicy({ ...raw, acls: [] }), 'unsupported-section'],
  ['autoApprovers', () => parsePolicy({ ...raw, autoApprovers: {} }), 'unsupported-section'],
  ['groups', () => parsePolicy({ ...raw, groups: {} }), 'unsupported-section'],
  ['a grant with via', grant({ via: ['tag:router'] }), 'unsupported-section'],
  ['a * source', grant({ src: ['*'] }), 'unsupported-selector'],
  ['autogroup:danger-all', grant({ dst: ['autogroup:danger-all'] }), 'unsupported-selector'],
  ['autogroup:member', grant({ src: ['autogroup:member'] }), 'unsupported-selector'],
  ['a group selector', grant({ src: ['group:admins'] }), 'unsupported-selector'],
  ['a /0 prefix', grant({ dst: ['0.0.0.0/0'] }), 'unsupported-selector'],
  ['a tag missing from tagOwners', grant({ src: ['tag:client'] }), 'unknown-tag'],
  ['an unknown name', grant({ dst: ['printer'] }), 'unknown-selector'],
  ['an IPv6 address', grant({ dst: ['fd7a:115c:a1e0::1'] }), 'invalid-address'],
  ['an empty src', grant({ src: [] }), 'invalid-shape'],
  ['an empty dst', grant({ dst: [] }), 'invalid-shape'],
  ['an empty ip', grant({ ip: [] }), 'invalid-shape'],
  ['a non-string selector', grant({ src: [7] }), 'invalid-shape'],
  ['a port above 65535', grant({ ip: ['tcp:70000'] }), 'invalid-port'],
  ['a backwards port range', grant({ ip: ['tcp:22-21'] }), 'invalid-port'],
  ['a three-part port range', grant({ ip: ['tcp:1-2-3'] }), 'invalid-port'],
  ['a port list', grant({ ip: ['tcp:22,443'] }), 'invalid-port'],
  ['an empty port', grant({ ip: ['tcp:'] }), 'invalid-port'],
  ['an unknown protocol', grant({ ip: ['gre:1'] }), 'invalid-protocol'],
  ['an owner that is an address', () => parsePolicy({ ...raw, tagOwners: { ...(raw.tagOwners as object), 'tag:hub': ['10.0.0.1'] } }), 'invalid-shape'],
  ['a tag name without tag:', () => parsePolicy({ ...raw, tagOwners: { ...(raw.tagOwners as object), hub: [] } }), 'invalid-shape'],
  ['an empty tag name', () => parsePolicy({ ...raw, tagOwners: { ...(raw.tagOwners as object), 'tag:': [] } }), 'invalid-shape'],
  ['a host alias that looks like a tag', () => parsePolicy({ ...raw, hosts: { 'tag:x': '10.1.0.0/24' } }), 'invalid-shape'],
  ['a host alias that is not a string', () => parsePolicy({ ...raw, hosts: { lab: 5 } }), 'invalid-shape'],
  ['a prefix with host bits set', () => parsePolicy({ ...raw, hosts: { lab: '10.10.0.5/24' } }), 'invalid-address'],
  ['an octet above 255', () => parsePolicy({ ...raw, hosts: { lab: '10.10.0.256' } }), 'invalid-address'],
  ['an octet with a leading zero', () => parsePolicy({ ...raw, hosts: { lab: '010.1.1.1' } }), 'invalid-address'],
  ['a prefix longer than 32 bits', () => parsePolicy({ ...raw, hosts: { lab: '10.10.0.0/33' } }), 'invalid-address'],
  ['a prefix with two slashes', () => parsePolicy({ ...raw, hosts: { lab: '10.0.0.0/8/8' } }), 'invalid-address'],
  ['a prefix with no bits', () => parsePolicy({ ...raw, hosts: { lab: '10.0.0.0/' } }), 'invalid-address'],
  ['a test row with an unknown key', tests({ src: 'operator@', deny: ['tag:hub:80'], note: 'x' }), 'unsupported-section'],
  ['a test row without a string src', tests({ src: ['operator@'], deny: ['tag:hub:80'] }), 'invalid-shape'],
  ['a test row that checks nothing', tests({ src: 'operator@' }), 'invalid-shape'],
  ['a test destination without a port', tests({ src: 'operator@', deny: ['taghub'] }), 'invalid-shape'],
  ['a test destination without a host', tests({ src: 'operator@', deny: [':80'] }), 'invalid-shape'],
  ['a test row with an unknown protocol', tests({ src: 'operator@', proto: 'gre', deny: ['tag:hub:80'] }), 'invalid-protocol'],
  ['a test destination that is a subnet', run({ src: 'operator@', deny: ['10.10.0.0/24:22'] }), 'invalid-shape'],
  ['an unknown node', at({ node: 'ghost' }, { node: 'hub' }), 'unknown-node'],
  ['a connection port above 65535', at({ node: 'mac' }, { node: 'hub' }, 70_000), 'invalid-port'],
  ['two nodes with one name', at({ node: 'mac' }, { node: 'hub' }, 22, [...nodes, { ...hubLike('mac') }]), 'invalid-node'],
  ['two nodes with one address', at({ node: 'mac' }, { node: 'hub' }, 22, [...nodes, { ...hubLike('x'), ip: '100.64.0.1' }]), 'invalid-node'],
  ['a node with neither user nor tags', at({ node: 'mac' }, { node: 'hub' }, 22, [...nodes, { ...hubLike('x'), identity: { tags: [] } }]), 'invalid-node'],
];

describe('evaluator rules', () => {
  test('vpn_004 the evaluator denies by default what no grant names', () => {
    const narrow = parsePolicy({ ...raw, grants: [{ src: ['tag:site-home'], dst: ['site-home'], ip: ['tcp:22'] }], tests: [] });
    expect(allowed(tcp({ node: 'router-home' }, { address: '10.10.0.5' }, 22), narrow)).toBe(true);
    expect(allowed(tcp({ node: 'mac' }, { node: 'hub' }, 443), narrow)).toBe(false);
    expect(allowed(tcp({ node: 'hub' }, { address: '10.10.0.5' }, 22), narrow)).toBe(false);
    expect(allowed(tcp({ node: 'router-home' }, { address: '10.10.0.5' }, 23), narrow)).toBe(false);
  });

  test.each(refused)('vpn_004 refuses %s', (_name, attempt, code) => {
    expect(catchPolicyError(attempt).code).toBe(code);
  });
});

function catchPolicyError(attempt: () => unknown): PolicyError {
  try {
    attempt();
  } catch (error) {
    if (error instanceof PolicyError) return error;
    throw error;
  }
  throw new Error('expected a PolicyError');
}

