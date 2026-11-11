// The inventory check for OD-10 nodes, keys and routes (VPN-013).
import { describe, expect, it } from 'bun:test';
import { findings, InventoryError, parseNodes, tailnetNodes, type Finding } from '../src/inventory.ts';
import { clean, cleanExpected, expected, NOW, rawNodes, tailnet } from './inventory-fixture.ts';
import { policy } from './policy-fixture.ts';

const kinds = (list: readonly Finding[]): string[] => list.map((finding) => `${finding.kind} ${'node' in finding ? finding.node : finding.test}`);

describe('inventory findings', () => {
  it('VPN-013: each finding kind is reported from the fixture', () => {
    const list = findings(policy, tailnet, [...expected, 'ghost'], NOW);
    expect(kinds(list)).toEqual([
      'foreign-user stranger',
      'unexpected-node stranger',
      'unknown-tag odd',
      'unexpected-node odd',
      'unapproved-route router-office1',
      'tagged-with-expiry agent-nas',
      'long-expiry mac2',
      'missing-node ghost',
    ]);
    expect(list[0]).toEqual({ kind: 'foreign-user', node: 'stranger', user: 'intruder' });
    expect(list).toContainEqual({ kind: 'unapproved-route', node: 'router-office1', route: '10.20.0.0/24' });
  });

  it('VPN-013: a tagged node under tagged-devices is not a foreign user', () => {
    const foreign = findings(policy, tailnet, expected, NOW).filter((finding) => finding.kind === 'foreign-user');
    expect(foreign).toEqual([{ kind: 'foreign-user', node: 'stranger', user: 'intruder' }]);
  });

  it('VPN-013: an agent node with an expiry is reported (R-105)', () => {
    expect(findings(policy, tailnet, expected, NOW)).toContainEqual({ kind: 'tagged-with-expiry', node: 'agent-nas' });
  });

  it('VPN-013: a client with no expiry is reported, one at 170 days is not', () => {
    const long = findings(policy, tailnet, expected, NOW).filter((finding) => finding.kind === 'long-expiry');
    expect(long.map((finding) => finding.node)).toEqual(['mac2']);
  });

  it('VPN-013: a client at exactly 180 days is not reported, one at 181 days is, 180 days and an hour is not', () => {
    const MS_PER_HOUR = 3_600_000;
    const longExpiry = (afterMs: number): Finding[] => {
      const nodes = clean.nodes.map((node) => (node.name === 'mac' ? { ...node, expiry: new Date(NOW.getTime() + afterMs) } : node));
      return findings(policy, { ...clean, nodes }, cleanExpected, NOW).filter((finding) => finding.kind === 'long-expiry');
    };
    expect(longExpiry(180 * 24 * MS_PER_HOUR)).toEqual([]);
    // The days are floored: 180 days and an hour counts as 180.
    expect(longExpiry(180 * 24 * MS_PER_HOUR + MS_PER_HOUR)).toEqual([]);
    expect(longExpiry(181 * 24 * MS_PER_HOUR)).toEqual([{ kind: 'long-expiry', node: 'mac', days: 181 }]);
  });

  it('VPN-013: a node with no IPv4 address is reported and left out of the policy test', () => {
    const v6only = { ...(clean.nodes.find((node) => node.name === 'mac') as object), name: 'v6', ips: ['fd7a:115c:a1e0::9'] } as (typeof clean.nodes)[number];
    const crowded = { ...clean, nodes: [...clean.nodes, v6only] };
    expect(tailnetNodes(crowded).map((node) => node.name)).toEqual(['hub', 'mac', 'router-home']);
    expect(findings(policy, crowded, [...cleanExpected, 'v6'], NOW)).toEqual([{ kind: 'no-ipv4', node: 'v6' }]);
  });

  it('VPN-013: a clean tailnet prints ok', () => {
    expect(findings(policy, clean, cleanExpected, NOW)).toEqual([]);
  });

  it('VPN-013: a failed policy test is reported', () => {
    const noHub = { ...clean, nodes: clean.nodes.filter((node) => node.name !== 'hub') };
    const list = findings(policy, noHub, ['mac', 'router-home'], NOW);
    expect(list.some((finding) => finding.kind === 'policy-test-failed')).toBe(true);
  });
});

describe('inventory parsing', () => {
  it('VPN-013: no key text survives parsing', () => {
    const text = JSON.stringify(parseNodes(rawNodes));
    for (const secret of ['REDACTED-PREAUTH-KEY', 'nodekey:', 'mkey:', 'discokey:']) expect(text).not.toContain(secret);
    expect(JSON.stringify(rawNodes)).toContain('REDACTED-PREAUTH-KEY');
    expect(parseNodes(rawNodes).find((node) => node.name === 'hub')?.preAuthKeyId).toBe(1);
  });

  it('VPN-013: an unknown shape is refused', () => {
    const broken = { ...(rawNodes[0] as Record<string, unknown>), ip_addresses: undefined };
    expect(() => parseNodes([broken])).toThrow(InventoryError);
    expect(() => parseNodes({})).toThrow(InventoryError);
  });

  it('VPN-013: absent online, tags, expiry and approved routes read as false, empty and null', () => {
    expect(tailnet.nodes.find((node) => node.name === 'stranger')).toMatchObject({ online: false, tags: [] });
    expect(tailnet.nodes.find((node) => node.name === 'hub')?.expiry).toBeNull();
    expect(tailnet.routes.find((entry) => entry.id === 6)).toEqual({ id: 6, approved: [], available: ['10.20.0.0/24'] });
  });

  it('VPN-013: a numeric id as a string is read, as protojson writes it', () => {
    expect(parseNodes([{ ...(rawNodes[0] as object), id: '7' }])[0]?.id).toBe(7);
  });
});
