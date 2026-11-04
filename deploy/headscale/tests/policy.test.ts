import { describe, expect, test } from 'bun:test';
import { HujsonError, parseHujson } from '../src/hujson.ts';
import { parsePolicy, PolicyError, type Policy } from '../src/policy.ts';
import { allows, runPolicyTests, type Connection, type TailnetNode } from '../src/tailnet.ts';

const policyText = await Bun.file(new URL('../config/policy.hujson', import.meta.url)).text();
const raw = parseHujson(policyText) as Record<string, unknown>;
const policy = parsePolicy(raw);

const SITES = { home: '10.10.0.0/24', office1: '10.20.0.0/24', office2: '10.30.0.0/24' };
const nodes: TailnetNode[] = [
  { name: 'mac', identity: { user: 'operator' }, ip: '100.64.0.10' },
  { name: 'laptop', identity: { user: 'operator' }, ip: '100.64.0.11' },
  { name: 'hub', identity: { tags: ['tag:hub'] }, ip: '100.64.0.1' },
  { name: 'router-home', identity: { tags: ['tag:router', 'tag:site-home'] }, ip: '100.64.0.2', routes: [SITES.home] },
  { name: 'router-office2', identity: { tags: ['tag:router', 'tag:site-office2'] }, ip: '100.64.0.3', routes: [SITES.office2] },
];

const tcp = (from: Connection['from'], to: Connection['to'], port: number): Connection => ({ from, to, port });
const allowed = (connection: Connection, under: Policy = policy, inventory = nodes): boolean => allows(under, inventory, connection);
const variant = (change: (grants: unknown[]) => unknown[]): Policy =>
  parsePolicy({ ...raw, grants: change(structuredClone(raw.grants as unknown[])) });

describe('policy tests block', () => {
  test('vpn_004 every row of the policy tests block passes in the evaluator', () => {
    expect(policy.tests.length).toBeGreaterThan(0);
    expect(runPolicyTests(policy, nodes)).toEqual([]);
  });

  test('vpn_004 the tests block runner reports a row the policy contradicts', () => {
    const wrong = parsePolicy({ ...raw, tests: [{ src: 'tag:hub', accept: ['operator@:22'] }] });
    expect(runPolicyTests(wrong, nodes)).toEqual([
      { src: 'tag:hub', dst: 'operator@:22', expected: 'accept', reason: 'hub is denied to mac:22' },
    ]);
  });

  test('vpn_004 a test row whose source matches no node fails instead of passing vacuously', () => {
    const empty = parsePolicy({ ...raw, tests: [{ src: 'tag:site-office1', deny: ['operator@:22'] }] });
    expect(runPolicyTests(empty, nodes)[0]?.reason).toBe('source matches no node');
  });
});

describe('permitted connections (VPN-004)', () => {
  test('vpn_004 client reaches hub on 443 and 22', () => {
    expect(allowed(tcp({ node: 'mac' }, { node: 'hub' }, 443))).toBe(true);
    expect(allowed(tcp({ node: 'laptop' }, { node: 'hub' }, 22))).toBe(true);
  });

  test('vpn_004 client is denied other hub ports such as 5432', () => {
    expect(allowed(tcp({ node: 'mac' }, { node: 'hub' }, 5432))).toBe(false);
    expect(allowed({ ...tcp({ node: 'mac' }, { node: 'hub' }, 443), proto: 'udp' })).toBe(false);
  });

  test('vpn_004 client reaches devices at every site', () => {
    for (const address of ['10.10.0.5', '10.20.0.5', '10.30.0.5']) {
      expect(allowed(tcp({ node: 'mac' }, { address }, 3389))).toBe(true);
    }
  });

  test('vpn_004 hub reaches devices at every site', () => {
    for (const address of ['10.10.0.5', '10.20.0.254', '10.30.0.1']) {
      expect(allowed(tcp({ node: 'hub' }, { address }, 161))).toBe(true);
    }
  });

  test('vpn_004 home router reaches only its own subnet', () => {
    expect(allowed(tcp({ node: 'router-home' }, { address: '10.10.0.5' }, 22))).toBe(true);
    expect(allowed(tcp({ node: 'router-home' }, { address: '10.30.0.5' }, 22))).toBe(false);
    expect(allowed(tcp({ node: 'router-home' }, { address: '10.20.0.5' }, 22))).toBe(false);
  });

  test('vpn_004 office 2 router is denied the home subnet', () => {
    expect(allowed(tcp({ node: 'router-office2' }, { address: '10.30.0.5' }, 22))).toBe(true);
    expect(allowed(tcp({ node: 'router-office2' }, { address: '10.10.0.5' }, 22))).toBe(false);
  });

  test('vpn_004 an office 1 router would reach only office 1', () => {
    const office1: TailnetNode = { name: 'router-office1', identity: { tags: ['tag:router', 'tag:site-office1'] }, ip: '100.64.0.4', routes: [SITES.office1] };
    const inventory = [...nodes, office1];
    expect(allowed(tcp({ node: 'router-office1' }, { address: '10.20.0.5' }, 22), policy, inventory)).toBe(true);
    expect(allowed(tcp({ node: 'router-office1' }, { address: '10.10.0.5' }, 22), policy, inventory)).toBe(false);
  });
});

describe('denied connections (VPN-004)', () => {
  test('vpn_004 routers cannot start a connection to a client', () => {
    for (const router of ['router-home', 'router-office2']) {
      for (const port of [22, 443, 5900]) expect(allowed(tcp({ node: router }, { node: 'mac' }, port))).toBe(false);
    }
  });

  test('vpn_004 a site device cannot start a connection to a client', () => {
    expect(allowed(tcp({ lan: '10.10.0.5' }, { node: 'mac' }, 22))).toBe(false);
    expect(allowed(tcp({ lan: '10.30.0.5' }, { address: '100.64.0.11' }, 443))).toBe(false);
  });

  test('vpn_004 a site device cannot reach the hub or another site', () => {
    expect(allowed(tcp({ lan: '10.10.0.5' }, { node: 'hub' }, 443))).toBe(false);
    expect(allowed(tcp({ lan: '10.10.0.5' }, { address: '10.30.0.5' }, 22))).toBe(false);
  });

  test('vpn_004 a LAN address with no router never enters the tailnet', () => {
    expect(allowed(tcp({ lan: '10.20.0.5' }, { address: '10.10.0.5' }, 22))).toBe(false);
  });

  test('vpn_004 hub cannot start a connection to a client', () => {
    expect(allowed(tcp({ node: 'hub' }, { node: 'mac' }, 22))).toBe(false);
    expect(allowed(tcp({ node: 'hub' }, { node: 'laptop' }, 443))).toBe(false);
  });

  test('vpn_004 client cannot start a connection to another client', () => {
    expect(allowed(tcp({ node: 'mac' }, { node: 'laptop' }, 22))).toBe(false);
  });

  test('vpn_004 clients cannot reach routers over the tailnet', () => {
    expect(allowed(tcp({ node: 'mac' }, { node: 'router-home' }, 22))).toBe(false);
  });
});

describe('evaluator rules', () => {
  test('vpn_004 the evaluator denies by default what no grant names', () => {
    const narrow = parsePolicy({ ...raw, grants: [{ src: ['tag:site-home'], dst: ['site-home'], ip: ['tcp:22'] }], tests: [] });
    expect(allowed(tcp({ node: 'router-home' }, { address: '10.10.0.5' }, 22), narrow)).toBe(true);
    expect(allowed(tcp({ node: 'mac' }, { node: 'hub' }, 443), narrow)).toBe(false);
    expect(allowed(tcp({ node: 'hub' }, { address: '10.10.0.5' }, 22), narrow)).toBe(false);
    expect(allowed(tcp({ node: 'router-home' }, { address: '10.10.0.5' }, 23), narrow)).toBe(false);
  });

  test('vpn_004 an empty or missing grants list is refused because Headscale would allow everything', () => {
    for (const grants of [[], undefined]) {
      expect(() => parsePolicy({ ...raw, grants })).toThrow(PolicyError);
    }
  });

  test('vpn_004 wildcard and autogroup selectors are refused', () => {
    for (const selector of ['*', 'autogroup:danger-all', 'autogroup:member', '0.0.0.0/0']) {
      const error = catchPolicyError(() => variant(() => [{ src: [selector], dst: ['site-home'], ip: ['*'] }]));
      expect(error.code).toBe('unsupported-selector');
    }
  });

  test('vpn_004 legacy acls, autoApprovers and unknown grant fields are refused', () => {
    expect(catchPolicyError(() => parsePolicy({ ...raw, acls: [] })).code).toBe('unsupported-section');
    expect(catchPolicyError(() => parsePolicy({ ...raw, autoApprovers: {} })).code).toBe('unsupported-section');
    const via = () => variant(() => [{ src: ['operator@'], dst: ['site-home'], ip: ['*'], via: ['tag:router'] }]);
    expect(catchPolicyError(via).code).toBe('unsupported-section');
  });

  test('vpn_004 a tag missing from tagOwners is refused', () => {
    expect(catchPolicyError(() => variant(() => [{ src: ['tag:client'], dst: ['site-home'], ip: ['*'] }])).code).toBe('unknown-tag');
  });

  test('vpn_004 malformed ports and addresses are refused', () => {
    for (const ip of ['tcp:70000', 'tcp:22-21', 'gre:1', 'tcp:22,443', '', 'tcp:']) {
      expect(catchPolicyError(() => variant(() => [{ src: ['operator@'], dst: ['site-home'], ip: [ip] }])).code).toBe('invalid-port');
    }
    for (const address of ['10.10.0.5/24', '10.10.0.256', '010.1.1.1', '10.10.0.0/33', 'fd7a::1']) {
      expect(catchPolicyError(() => parsePolicy({ ...raw, hosts: { lab: address } })).code).toBe('invalid-address');
    }
  });
});

describe('policy structure', () => {
  const grants = raw.grants as { src: string[]; dst: string[]; ip: string[] }[];
  const tests = raw.tests as { src: string; accept?: string[]; deny?: string[] }[];
  const tagOwners = raw.tagOwners as Record<string, string[]>;
  const selectors = [
    ...grants.flatMap((grant) => [...grant.src, ...grant.dst]),
    ...tests.flatMap((row) => [row.src, ...[...(row.accept ?? []), ...(row.deny ?? [])].map((dst) => dst.slice(0, dst.lastIndexOf(':')))]),
  ];

  test('vpn_004 policy uses grants only, with no acls and no autoApprovers', () => {
    expect(Object.keys(raw).sort()).toEqual(['grants', 'hosts', 'tagOwners', 'tests']);
  });

  test('vpn_004 no wildcard or autogroup selector appears in src or dst', () => {
    expect(selectors.filter((selector) => selector === '*' || selector.startsWith('autogroup:'))).toEqual([]);
    expect(policyText).not.toContain('autogroup:danger-all');
  });

  test('vpn_004 every tag used is in tagOwners and every owner is operator@', () => {
    const used = selectors.filter((selector) => selector.startsWith('tag:'));
    expect(used.filter((tag) => !(tag in tagOwners))).toEqual([]);
    expect(Object.values(tagOwners).flat().filter((owner) => owner !== 'operator@')).toEqual([]);
    expect(Object.keys(tagOwners).sort()).toEqual(['tag:hub', 'tag:router', 'tag:site-home', 'tag:site-office1', 'tag:site-office2']);
  });

  test('vpn_003 no tag:client exists, so client keys keep their expiry', () => {
    expect(policyText).not.toContain('tag:client');
  });

  test('vpn_001 site subnets are the planned ones and do not overlap', () => {
    const hosts = raw.hosts as Record<string, string>;
    expect(hosts).toEqual({ 'site-home': SITES.home, 'site-office1': SITES.office1, 'site-office2': SITES.office2 });
    const ranges = Object.values(hosts).map(addressRange);
    for (const [index, a] of ranges.entries()) {
      for (const b of ranges.slice(index + 1)) expect(a.first > b.last || b.first > a.last).toBe(true);
    }
  });

  test('vpn_004 no grant targets a client', () => {
    expect(grants.filter((grant) => grant.dst.some((selector) => selector.includes('@')))).toEqual([]);
  });
});

describe('hujson', () => {
  test('vpn_004 comments inside strings are kept and trailing commas are dropped', () => {
    const text = '{"a": "// not a comment", /* c */ "b": ["/* kept */", 1,], // end\n}';
    expect(parseHujson(text)).toEqual({ a: '// not a comment', b: ['/* kept */', 1] });
  });

  test('vpn_004 unterminated strings and comments are refused', () => {
    for (const text of ['{"a": "x}', '{"a": 1 /* open', '[1,,]', '{,}', '[,]']) {
      expect(() => parseHujson(text)).toThrow(HujsonError);
    }
  });

  test('vpn_004 random JSON with random comments and trailing commas reads back unchanged', () => {
    const rand = seeded(7);
    for (let round = 0; round < 500; round += 1) {
      const value = randomValue(rand, 3);
      expect(parseHujson(render(value, rand))).toEqual(value);
    }
  });

  test('vpn_004 random text either parses or raises HujsonError', () => {
    const rand = seeded(11);
    const alphabet = '{}[],:"/*\\\n 1a';
    for (let round = 0; round < 2000; round += 1) {
      const text = Array.from({ length: Math.floor(rand() * 24) }, () => alphabet.charAt(Math.floor(rand() * alphabet.length))).join('');
      try {
        parseHujson(text);
      } catch (error) {
        expect(error).toBeInstanceOf(HujsonError);
      }
    }
  });
});

function catchPolicyError(run: () => unknown): PolicyError {
  try {
    run();
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

function seeded(seed: number): () => number {
  let state = seed;
  return () => {
    state = (state * 1_103_515_245 + 12_345) % 2_147_483_648;
    return state / 2_147_483_648;
  };
}

const TRICKY = ['', 'a', '// x', '/* y */', '*/', '"', '\\', ',]', '},', 'tag:hub:443'];

function randomValue(rand: () => number, depth: number): unknown {
  const pick = Math.floor(rand() * (depth > 0 ? 6 : 4));
  if (pick === 0) return TRICKY[Math.floor(rand() * TRICKY.length)];
  if (pick === 1) return Math.floor(rand() * 1000) - 500;
  if (pick === 2) return rand() < 0.5;
  if (pick === 3) return null;
  const size = Math.floor(rand() * 4);
  if (pick === 4) return Array.from({ length: size }, () => randomValue(rand, depth - 1));
  return Object.fromEntries(Array.from({ length: size }, (_, index) => [`${TRICKY[index] ?? ''}${String(index)}`, randomValue(rand, depth - 1)]));
}

function render(value: unknown, rand: () => number): string {
  const noise = (): string => ['', ' ', '\n', '// c, ]\n', '/* , } */', '\t'][Math.floor(rand() * 6)] ?? '';
  const close = (parts: string[]): string => parts.join(',') + (parts.length > 0 && rand() < 0.5 ? `,${noise()}` : '') + noise();
  if (Array.isArray(value)) return `[${close(value.map((entry) => noise() + render(entry, rand) + noise()))}]`;
  if (typeof value === 'object' && value !== null) {
    const parts = Object.entries(value).map(([key, entry]) => `${noise()}${JSON.stringify(key)}${noise()}:${noise()}${render(entry, rand)}`);
    return `{${close(parts)}}`;
  }
  return JSON.stringify(value);
}
