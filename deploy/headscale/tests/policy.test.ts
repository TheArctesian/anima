import { describe, expect, test } from 'bun:test';
import { parseHujson } from '../src/hujson.ts';
import { parsePolicy, PolicyError, type Policy, type PolicyErrorCode } from '../src/policy.ts';
import { allows, runPolicyTests, type Connection, type TailnetNode } from '../src/tailnet.ts';

const policyText = await Bun.file(new URL('../config/policy.hujson', import.meta.url)).text();
const raw = parseHujson(policyText) as Record<string, unknown>;
const policy = parsePolicy(raw);

const SITES = { home: '10.10.0.0/24', office1: '10.20.0.0/24', office2: '10.30.0.0/24' };
const TAILNET_IPV4 = '100.64.0.0/10';
const nodes: TailnetNode[] = [
  // The Mac the operator added as a device (R-061): `device-stephens-mac` in the policy.
  { name: 'mac', identity: { user: 'operator' }, ip: '100.64.0.2' },
  { name: 'laptop', identity: { user: 'operator' }, ip: '100.64.0.11' },
  { name: 'hub', identity: { tags: ['tag:hub'] }, ip: '100.64.0.1' },
  // An agent-only device (MET-020): tagged, not a personal node.
  { name: 'agent', identity: { tags: ['tag:agent'] }, ip: '100.64.0.30' },
  { name: 'router-home', identity: { tags: ['tag:router', 'tag:site-home'] }, ip: '100.64.0.20', routes: [SITES.home] },
  { name: 'router-office2', identity: { tags: ['tag:router', 'tag:site-office2'] }, ip: '100.64.0.21', routes: [SITES.office2] },
];

type From = Connection['from'];
type To = Connection['to'];
const tcp = (from: From, to: To, port: number): Connection => ({ from, to, port });
const allowed = (connection: Connection, under: Policy = policy, inventory = nodes): boolean => allows(under, inventory, connection);
const withGrants = (grants: unknown[]): Policy => parsePolicy({ ...raw, grants });
const withTests = (tests: unknown[]): Policy => parsePolicy({ ...raw, tests });

/**
 * Test rows that need a router node. Headscale refuses the policy while a row names a tag no node
 * has, so each group moves into the policy's tests block once its router is enrolled.
 */
const ROUTER_ROWS = [
  // The home router (tag:router, tag:site-home).
  { src: 'tag:site-home', proto: 'tcp', accept: ['10.10.0.5:22'], deny: ['10.20.0.5:22', '10.30.0.5:22', 'operator@:22', 'tag:hub:443'] },
  { src: '10.10.0.5', deny: ['operator@:22', 'tag:hub:443', '10.30.0.5:22'] },
  { src: 'tag:router', proto: 'tcp', deny: ['operator@:22', 'operator@:443', 'tag:hub:443', 'tag:hub:22'] },
  { src: 'operator@', proto: 'tcp', deny: ['tag:router:22'] },
  { src: 'tag:hub', proto: 'tcp', deny: ['tag:router:22'] },
  // The office 2 router (tag:router, tag:site-office2).
  { src: 'tag:site-office2', proto: 'tcp', accept: ['10.30.0.5:22'], deny: ['10.10.0.5:22', '10.20.0.5:22', 'operator@:22', 'tag:hub:443'] },
  { src: '10.30.0.5', deny: ['operator@:22', 'tag:hub:443', '10.10.0.5:22'] },
];

/** Rows for the agent tag. Headscale refuses the policy while no node carries tag:agent, so they move into the policy's tests block once one is enrolled. */
const AGENT_ROWS = [
  { src: 'tag:agent', proto: 'tcp', accept: ['tag:hub:443'], deny: ['tag:hub:22', 'tag:hub:80', 'tag:hub:5432', '10.10.0.5:22', '10.20.0.5:443', '10.30.0.5:3389', '100.64.0.2:22', '100.64.0.3:5900', 'operator@:443'] },
  { src: 'tag:agent', proto: 'udp', deny: ['tag:hub:443', 'tag:hub:53'] },
  { src: 'tag:hub', proto: 'tcp', deny: ['tag:agent:22', 'tag:agent:443'] },
  { src: 'operator@', proto: 'tcp', deny: ['tag:agent:22', 'tag:agent:443'] },
];

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
  const more: TailnetNode = { name: 'agent2', identity: { tags: ['tag:agent'] }, ip: '100.64.0.31' };
  const everyone = [...nodes, more];
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

  test('met_020 exactly one grant names tag:agent, and only as a source to the hub on tcp 443', () => {
    const named = (raw.grants as { src: string[]; dst: string[]; ip: string[] }[]).filter((entry) => [...entry.src, ...entry.dst].includes('tag:agent'));
    expect(named).toEqual([{ src: ['tag:agent'], dst: ['tag:hub'], ip: ['tcp:443'] }]);
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

describe('policy structure', () => {
  const grants = raw.grants as { src: string[]; dst: string[]; ip: string[] }[];
  const rows = [...(raw.tests as unknown[]), ...ROUTER_ROWS, ...AGENT_ROWS] as { src: string; accept?: string[]; deny?: string[] }[];
  const tagOwners = raw.tagOwners as Record<string, string[]>;
  const hosts = raw.hosts as Record<string, string>;
  const selectors = [
    ...grants.flatMap((grant) => [...grant.src, ...grant.dst]),
    ...rows.flatMap((row) => [row.src, ...[...(row.accept ?? []), ...(row.deny ?? [])].map((dst) => dst.slice(0, dst.lastIndexOf(':')))]),
  ];

  test('vpn_004 policy uses grants only, with no acls and no autoApprovers', () => {
    expect(Object.keys(raw).sort()).toEqual(['grants', 'hosts', 'tagOwners', 'tests']);
  });

  test('vpn_004 no wildcard or autogroup selector appears in src or dst', () => {
    expect(selectors.filter((selector) => selector === '*' || selector.startsWith('autogroup:'))).toEqual([]);
    expect(policyText).not.toContain('autogroup:danger-all');
  });

  test('vpn_004 every tag used is in tagOwners', () => {
    expect(selectors.filter((selector) => selector.startsWith('tag:') && !(selector in tagOwners))).toEqual([]);
    expect(Object.keys(tagOwners).sort()).toEqual(['tag:agent', 'tag:hub', 'tag:router', 'tag:site-home', 'tag:site-office1', 'tag:site-office2']);
  });

  test('vpn_003 every tag owner list is empty, so no node can claim a tag when it registers', () => {
    expect(Object.entries(tagOwners).filter(([, owners]) => owners.length > 0)).toEqual([]);
  });

  test('vpn_003 no tag:client exists, so client keys keep their expiry', () => {
    expect(policyText).not.toContain('tag:client');
  });

  test('vpn_001 site subnets are the planned ones and do not overlap', () => {
    const sites = Object.fromEntries(Object.entries(hosts).filter(([name]) => name.startsWith('site-')));
    expect(sites).toEqual({ 'site-home': SITES.home, 'site-office1': SITES.office1, 'site-office2': SITES.office2 });
    const ranges = Object.values(sites).map(addressRange);
    for (const [index, a] of ranges.entries()) {
      for (const b of ranges.slice(index + 1)) expect(overlaps(a, b)).toBe(false);
    }
  });

  test('vpn_004 a client added as a device is one /32, reached only by tag:hub on named tcp ports (R-061)', () => {
    const devices = Object.entries(hosts).filter(([name]) => name.startsWith('device-'));
    expect(devices.length).toBeGreaterThan(0);
    for (const [name, cidr] of devices) {
      expect(cidr).toMatch(/^100\.(6[4-9]|[7-9]\d|1[01]\d|12[0-7])\.\d{1,3}\.\d{1,3}\/32$/);
      const reaching = grants.filter((grant) => grant.dst.includes(name));
      expect(reaching.length).toBeGreaterThan(0);
      for (const grant of reaching) {
        expect(grant.src).toEqual(['tag:hub']);
        expect(grant.dst).toEqual([name]);
        for (const range of grant.ip) expect(range).toMatch(/^tcp:\d+$/);
      }
    }
  });

  test('vpn_004 no grant dst reaches a tailnet address except through tag:hub', () => {
    const reachesTailnet = (selector: string): boolean => {
      if (selector === 'tag:hub') return false;
      // A client added as a device; the test above limits its grants (R-061).
      if (selector.startsWith('device-')) return false;
      if (selector.startsWith('tag:') || selector.includes('@')) return true;
      const address = hosts[selector] ?? selector;
      // Every IPv6 target counts, which covers the tailnet's fd7a:115c:a1e0::/48.
      return address.includes(':') || overlaps(addressRange(address), addressRange(TAILNET_IPV4));
    };
    expect(grants.flatMap((grant) => grant.dst).filter(reachesTailnet)).toEqual([]);
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

function addressRange(cidr: string): { first: number; last: number } {
  const [address = '', bits = '32'] = cidr.split('/');
  const first = address.split('.').reduce((sum, octet) => sum * 256 + Number(octet), 0);
  return { first, last: first + 2 ** (32 - Number(bits)) - 1 };
}

function overlaps(a: { first: number; last: number }, b: { first: number; last: number }): boolean {
  return a.first <= b.last && b.first <= a.last;
}
