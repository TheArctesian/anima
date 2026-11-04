import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { isVpnAddress } from './vpn-address.ts';

interface CaaProperty {
  flags: number;
  tag: string;
  value: string;
}

interface DnsRecord {
  type: string;
  name: string;
  content?: string;
  data?: CaaProperty;
  proxied?: boolean;
  ttl: number;
  comment?: string;
}

interface Zone {
  zone: string;
  records: DnsRecord[];
}

const dnsDir = join(import.meta.dir, '..');
const recordsText = readFileSync(join(dnsDir, 'anima.ing.records.json'), 'utf8');
const readme = readFileSync(join(dnsDir, 'README.md'), 'utf8');
const zone = JSON.parse(recordsText) as Zone;

function only(name: string, type: string): DnsRecord {
  const matches = zone.records.filter((record) => record.name === name && record.type === type);
  expect(matches).toHaveLength(1);
  return matches[0] as DnsRecord;
}

const caa = zone.records.filter((record) => record.type === 'CAA');

describe('anima.ing public records', () => {
  test('DNS-004 hs.anima.ing is an A record that is not proxied', () => {
    const hs = only('hs.anima.ing', 'A');
    expect(hs.proxied).toBe(false);
    expect(hs.content).toBe('${VPS_PUBLIC_IPV4}');
  });

  test('DNS-004 hub.anima.ing is an A record that is not proxied', () => {
    expect(only('hub.anima.ing', 'A').proxied).toBe(false);
  });

  test('DNS-005 NET-005 hub.anima.ing content is the HUB_VPN_IPV4 placeholder, filled in after the VPS joins', () => {
    expect(only('hub.anima.ing', 'A').content).toBe('${HUB_VPN_IPV4}');
  });

  test('DNS-005 a hub address outside 100.64.0.0/10 is rejected', () => {
    for (const address of ['10.10.0.5', '100.63.255.255', '100.128.0.1', '192.168.1.10', '8.8.8.8', '100.64.0', '100.64.0.1.2', '100.064.0.1', '100.64.0.256', '']) {
      expect(isVpnAddress(address)).toBe(false);
    }
  });

  test('DNS-005 a hub address inside 100.64.0.0/10 is accepted', () => {
    for (const address of ['100.64.0.1', '100.100.7.3', '100.127.255.254']) {
      expect(isVpnAddress(address)).toBe(true);
    }
  });

  test('DNS-005 every A record content is a placeholder, never a literal address', () => {
    for (const record of zone.records.filter((r) => r.type === 'A')) {
      expect(record.content).toMatch(/^\$\{[A-Z0-9_]+\}$/);
    }
  });

  test('DNS-006 no record exists under net.anima.ing', () => {
    for (const record of zone.records) {
      expect(record.name === 'net.anima.ing' || record.name.endsWith('.net.anima.ing')).toBe(false);
    }
  });

  test('DNS-007 handoff updates.anima.ing is a proxied placeholder origin', () => {
    const updates = only('updates.anima.ing', 'AAAA');
    expect(updates.proxied).toBe(true);
    expect(updates.content).toBe('100::');
  });

  test('DNS-001 the record names are exactly the apex, hs, hub and updates', () => {
    const names = new Set(zone.records.map((record) => record.name));
    expect([...names].sort()).toEqual(['anima.ing', 'hs.anima.ing', 'hub.anima.ing', 'updates.anima.ing']);
  });

  test('DNS-001 every record name is a fully qualified name inside the zone', () => {
    expect(zone.zone).toBe('anima.ing');
    for (const record of zone.records) {
      expect(record.name === zone.zone || record.name.endsWith(`.${zone.zone}`)).toBe(true);
    }
  });

  test('DNS-002 CAA allows only letsencrypt.org to issue', () => {
    const issue = caa.filter((record) => record.data?.tag === 'issue');
    expect(issue.map((record) => record.data?.value)).toEqual(['letsencrypt.org']);
    for (const record of caa) {
      expect(record.name).toBe('anima.ing');
      expect(record.data?.flags).toBe(0);
      expect(['issue', 'issuewild']).toContain(record.data?.tag ?? '');
    }
  });

  test('DNS-002 CAA forbids wildcard certificates', () => {
    const wild = caa.filter((record) => record.data?.tag === 'issuewild');
    expect(wild.map((record) => record.data?.value)).toEqual([';']);
  });

  test('DNS-003 the records file holds only the two documented placeholders', () => {
    const placeholders = new Set(recordsText.match(/\$\{[^}]*\}/g) ?? []);
    expect([...placeholders].sort()).toEqual(['${HUB_VPN_IPV4}', '${VPS_PUBLIC_IPV4}']);
    expect(recordsText.replace(/\$\{(HUB_VPN_IPV4|VPS_PUBLIC_IPV4)\}/g, '')).not.toContain('$');
  });

  test('DNS-003 the README names the scoped token permission and never the Global API Key as usable', () => {
    expect(readme).toContain('Zone → DNS → Edit');
    expect(readme).toContain('Include → Specific zone → anima.ing');
    expect(readme).toContain('Never use the Global API Key');
    expect(readme).toContain('-H @"$HDR"');
  });

  test('DNS-003 LOG-007 the README contains no token-shaped string', () => {
    expect(readme).not.toMatch(/[A-Za-z0-9_-]{32,}/);
    for (const bearer of readme.matchAll(/Bearer\s+(\S+)/g)) {
      expect(bearer[1]).toBe('%s\\n\'');
    }
  });

  test('DNS-008 the README requires the operator confirmation in chat for each change', () => {
    expect(readme).toContain('DNS-008');
    expect(readme).toMatch(/confirmation in chat/);
  });
});
