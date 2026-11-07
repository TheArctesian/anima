import { describe, expect, it } from 'vitest';
import type { CardDevice, DevicePage } from './api';
import { appendPage, applyStatus, applyVncEnabled, compareDevices, fromPage } from './list';
import { DEFAULT_QUERY, type ListQuery } from './query';

function device(name: string, status: CardDevice['status'] = 'UP', uptime: number | null = 1): CardDevice {
  return {
    id: name,
    name,
    host: `${name}.lan`,
    ip: null,
    os: 'OTHER',
    tags: [],
    maintenance: false,
    status,
    uptime,
    macAddress: null,
    metricsSource: 'NONE',
    readings: null,
    accessWarnings: [],
    rdp: { enabled: false },
    vnc: { enabled: false, username: null },
    ssh: { enabled: false, hostKey: null },
    checks: [],
    webInterfaces: [],
  };
}

function page(devices: CardDevice[], next: string | null = null, totalCount = devices.length): DevicePage {
  return {
    totalCount,
    pageInfo: { hasNextPage: next !== null, endCursor: next },
    edges: devices.map((node) => ({ node })),
  };
}

const names = (devices: readonly CardDevice[]): string[] => devices.map((entry) => entry.name);
const change = (deviceId: string, status: CardDevice['status']) => ({ deviceId, status, at: '2026-01-01T00:00:00Z' });

describe('compareDevices (E-1, DEV-004)', () => {
  it('DEV-004 orders STATUS worst first by severity, then by name', () => {
    const devices = [
      device('e', 'UP'),
      device('d', 'MAINTENANCE'),
      device('c', 'UNKNOWN'),
      device('b2', 'DEGRADED'),
      device('b1', 'DEGRADED'),
      device('a', 'DOWN'),
    ];
    expect(names(devices.toSorted(compareDevices('STATUS')))).toEqual(['a', 'b1', 'b2', 'c', 'd', 'e']);
  });

  it('DEV-004 orders UPTIME lowest first, no uptime last, ties by name', () => {
    const devices = [
      device('n', 'UP', null),
      device('m', 'UP', 0.99),
      device('z', 'UP', 0.5),
      device('a', 'UP', 0.5),
      device('b', 'UP', null),
    ];
    expect(names(devices.toSorted(compareDevices('UPTIME')))).toEqual(['a', 'z', 'm', 'b', 'n']);
  });

  it('DEV-004 orders NAME by UTF-8 bytes, not by UTF-16 code units', () => {
    // U+FF21 is one UTF-16 unit (0xFF21) but three bytes; U+1F600 is a surrogate pair (0xD83D) but four bytes.
    // Bytes put U+FF21 (EF BC A1) before U+1F600 (F0 9F 98 80); code units would put the emoji first.
    const devices = [device('\u{1F600}'), device('Ａ'), device('B'), device('a')];
    expect(names(devices.toSorted(compareDevices('NAME')))).toEqual(['B', 'a', 'Ａ', '\u{1F600}']);
  });

  it('DEV-004 orders a name before a longer name that starts with it', () => {
    const devices = [device('ab'), device('a')];
    expect(names(devices.toSorted(compareDevices('NAME')))).toEqual(['a', 'ab']);
  });
});

describe('device list pages (DEV-001)', () => {
  it('DEV-001 starts from a page and keeps the cursor', () => {
    const list = fromPage(page([device('a')], 'c1', 3));
    expect(list).toMatchObject({ totalCount: 3, endCursor: 'c1', hasNextPage: true });
    expect(names(list.devices)).toEqual(['a']);
  });

  it('DEV-001 appends a page without listing a device twice', () => {
    const first = fromPage(page([device('a'), device('b', 'DOWN')], 'c1', 3));
    const next = appendPage(first, page([device('b', 'UP'), device('c')], null, 3));
    expect(names(next.devices)).toEqual(['a', 'b', 'c']);
    expect(next.devices[1]?.status).toBe('DOWN');
    expect(next).toMatchObject({ endCursor: null, hasNextPage: false });
  });
});

describe('live status (DEV-003, DEV-004)', () => {
  const list = fromPage(page([device('a'), device('b'), device('c')], null, 3));

  it('DEV-003 changes the status in place under the name order', () => {
    const next = applyStatus(list, change('b', 'DOWN'), DEFAULT_QUERY);
    expect(names(next.devices)).toEqual(['a', 'b', 'c']);
    expect(next.devices[1]?.status).toBe('DOWN');
    expect(list.devices[1]?.status).toBe('UP');
  });

  it('DEV-003 removes a device that no longer matches the status filter', () => {
    const query: ListQuery = { ...DEFAULT_QUERY, statuses: ['UP'] };
    const next = applyStatus(list, change('b', 'DOWN'), query);
    expect(names(next.devices)).toEqual(['a', 'c']);
    expect(next.totalCount).toBe(2);
  });

  it('DEV-003 keeps a device whose new status the filter admits', () => {
    const query: ListQuery = { ...DEFAULT_QUERY, statuses: ['UP', 'DOWN'] };
    const next = applyStatus(list, change('b', 'DOWN'), query);
    expect(names(next.devices)).toEqual(['a', 'b', 'c']);
    expect(next.totalCount).toBe(3);
  });

  it('DEV-004 moves a device that turns DOWN to the front under the STATUS sort', () => {
    const next = applyStatus(list, change('c', 'DOWN'), { ...DEFAULT_QUERY, sort: 'STATUS' });
    expect(names(next.devices)).toEqual(['c', 'a', 'b']);
  });

  it('DEV-003 ignores a change for a device that is not listed', () => {
    expect(applyStatus(list, change('zzz', 'DOWN'), DEFAULT_QUERY)).toBe(list);
  });
});

describe('applyVncEnabled (R-079)', () => {
  it('R-079: only the named device shows VNC on', () => {
    const list = fromPage({
      totalCount: 2,
      pageInfo: { hasNextPage: false, endCursor: null },
      edges: [{ node: device('a') }, { node: device('b') }],
    });
    const next = applyVncEnabled(list, 'b');
    expect(next.devices.map((shown) => shown.vnc.enabled)).toEqual([false, true]);
    expect(next.devices[0]).toBe(list.devices[0]);
  });
});
