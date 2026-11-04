import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

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
const caa = zone.records.filter((record) => record.type === 'CAA');

function singleRecord(name: string, type: string): DnsRecord {
  const matches = zone.records.filter((record) => record.name === name && record.type === type);
  expect(matches).toHaveLength(1);
  return matches[0] as DnsRecord;
}

describe('anima.ing public records', () => {
  test('DNS-001 the records are exactly A hs, A hub, AAAA updates and CAA on the apex', () => {
    const pairs = zone.records.map((record) => `${record.type} ${record.name}`).sort();
    expect(pairs).toEqual(['A hs.anima.ing', 'A hub.anima.ing', 'AAAA updates.anima.ing', 'CAA anima.ing']);
  });

  test('DNS-001 every record name is a fully qualified name inside the zone', () => {
    expect(zone.zone).toBe('anima.ing');
    for (const record of zone.records) {
      expect(record.name === zone.zone || record.name.endsWith(`.${zone.zone}`)).toBe(true);
    }
  });

  test('DNS-004 every record named hs or hub is not proxied', () => {
    const named = zone.records.filter((record) => ['hs.anima.ing', 'hub.anima.ing'].includes(record.name));
    expect(named.length).toBeGreaterThanOrEqual(2);
    for (const record of named) expect(record.proxied).toBe(false);
  });

  test('DNS-004 hs.anima.ing points to the VPS_PUBLIC_IPV4 placeholder', () => {
    expect(singleRecord('hs.anima.ing', 'A').content).toBe('${VPS_PUBLIC_IPV4}');
  });

  test('DNS-005 NET-005 hub.anima.ing content is the HUB_VPN_IPV4 placeholder, filled in after the VPS joins', () => {
    expect(singleRecord('hub.anima.ing', 'A').content).toBe('${HUB_VPN_IPV4}');
  });

  test('DNS-005 every A record content is a placeholder, never a literal address', () => {
    for (const record of zone.records.filter((r) => r.type === 'A')) {
      expect(record.content).toMatch(/^\$\{[A-Z0-9_]+\}$/);
    }
  });

  test('DNS-005 the README states HUB_VPN_IPV4 is a 100.64.0.0/10 address', () => {
    expect(readme).toMatch(/`HUB_VPN_IPV4` is a 100\.64\.0\.0\/10 address/);
  });

  test('DNS-006 no record exists under net.anima.ing', () => {
    for (const record of zone.records) {
      expect(record.name === 'net.anima.ing' || record.name.endsWith('.net.anima.ing')).toBe(false);
    }
  });
});

describe('anima.ing proxied and CAA records, and the README', () => {
  test('DNS-007 handoff updates.anima.ing is the only proxied record', () => {
    const proxied = zone.records.filter((record) => record.proxied === true);
    expect(proxied.map((record) => `${record.type} ${record.name}`)).toEqual(['AAAA updates.anima.ing']);
    expect(singleRecord('updates.anima.ing', 'AAAA').content).toBe('100::');
  });

  test('DNS-007 handoff a proxied record has ttl 1 (automatic)', () => {
    for (const record of zone.records.filter((r) => r.proxied === true)) expect(record.ttl).toBe(1);
  });

  test('DNS-002 the only CAA property is issue letsencrypt.org on the apex', () => {
    expect(caa.map((record) => [record.name, record.data])).toEqual([
      ['anima.ing', { flags: 0, tag: 'issue', value: 'letsencrypt.org' }],
    ]);
  });

  test('DNS-003 the records file holds only the two documented placeholders', () => {
    const placeholders = new Set(recordsText.match(/\$\{[^}]*\}/g) ?? []);
    expect([...placeholders].sort()).toEqual(['${HUB_VPN_IPV4}', '${VPS_PUBLIC_IPV4}']);
    expect(recordsText.replace(/\$\{(HUB_VPN_IPV4|VPS_PUBLIC_IPV4)\}/g, '')).not.toContain('$');
  });

  test('DNS-003 the README names the scoped token permission and forbids the Global API Key', () => {
    expect(readme).toContain('Zone → DNS → Edit');
    expect(readme).toContain('Include → Specific zone → anima.ing');
    expect(readme).toContain('Never use the Global API Key');
  });

  test('DNS-003 LOG-007 the README contains no token-shaped string', () => {
    expect(readme).not.toMatch(/[A-Za-z0-9_-]{32,}/);
  });

  test('DNS-003 LOG-007 every curl in the README takes -q first so ~/.curlrc cannot turn on verbose', () => {
    const calls = [...readme.matchAll(/\bcurl -\S*/g)].map((match) => match[0]);
    expect(calls.length).toBeGreaterThan(0);
    for (const call of calls) expect(call).toBe('curl -q');
  });

  test('DNS-008 the README requires the operator confirmation in chat for each change', () => {
    expect(readme).toContain('DNS-008');
    expect(readme).toMatch(/confirmation in chat/);
  });
});
