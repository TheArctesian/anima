import { describe, expect, test } from 'bun:test';
import { parsePrefix } from '../src/policy.ts';
import { AGENT_ROWS, everyone, policyText, raw, ROUTER_ROWS, SITES, TAILNET_IPV4 } from './policy-fixture.ts';

describe('agent structure (MET-020, R-084)', () => {
  test('met_020 no host /32 is the address of a tagged node in the test inventory, so a device grant cannot reach it', () => {
    const taggedAddresses = everyone.filter((node) => 'tags' in node.identity).map((node) => parsePrefix(`${node.ip}/32`).base);
    const hostPrefixes = Object.values(raw.hosts as Record<string, string>).map((cidr) => parsePrefix(cidr));
    const hostAddresses = hostPrefixes.filter((prefix) => prefix.bits === 32).map((prefix) => prefix.base);
    expect(hostAddresses.filter((address) => taggedAddresses.includes(address))).toEqual([]);
  });

  test('met_020 exactly one grant names tag:agent, and only as a source to the hub on tcp 443', () => {
    const named = (raw.grants as { src: string[]; dst: string[]; ip: string[] }[]).filter((entry) => [...entry.src, ...entry.dst].includes('tag:agent'));
    expect(named).toEqual([{ src: ['tag:agent'], dst: ['tag:hub'], ip: ['tcp:443'] }]);
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

function addressRange(cidr: string): { first: number; last: number } {
  const [address = '', bits = '32'] = cidr.split('/');
  const first = address.split('.').reduce((sum, octet) => sum * 256 + Number(octet), 0);
  return { first, last: first + 2 ** (32 - Number(bits)) - 1 };
}

function overlaps(a: { first: number; last: number }, b: { first: number; last: number }): boolean {
  return a.first <= b.last && b.first <= a.last;
}
