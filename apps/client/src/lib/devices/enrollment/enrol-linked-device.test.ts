import { describe, expect, it, vi } from 'vitest';
import { enrolLinkedDevice, type LinkedDeviceParts } from './enrol-linked-device';

const origin = 'http://127.0.0.1:47100';
const host = 'fd41:4e49:4d41::7';
const ipad = { name: 'iPad', kind: 'tablet' };
const ok = <T,>(value: T) => ({ ok: true as const, value });

function setup(savedHosts: readonly string[] = [], port = 47100) {
  return {
    link: async () => ok({ joined: true, state: 'up' as const, port, publicKey: 'nodekey:own' }),
    linkDevices: async () => ok([{ key: 'nodekey:other', address: 'fd41:4e49:4d41::6' }, { key: 'nodekey:own', address: host }]),
    hosts: async () => ok(savedHosts),
    sites: async () => ok([{ id: 'link', name: 'Link', subnet: 'fd41:4e49:4d41::/48' }]),
    createSite: async () => ok({ id: 'new-site' }),
    createDevice: vi.fn<LinkedDeviceParts['createDevice']>().mockResolvedValue(ok({ id: 'ipad' })),
  } satisfies LinkedDeviceParts;
}

describe('enrolling this iPad', () => {
  it('uses the active link identity and makes one tablet entry for simultaneous requests', async () => {
    const parts = setup();
    expect(await Promise.all([enrolLinkedDevice(origin, parts, ipad), enrolLinkedDevice(origin, parts, ipad)])).toEqual(['added', 'added']);
    expect(parts.createDevice).toHaveBeenCalledExactlyOnceWith({
      siteId: 'link', name: 'iPad', host, kind: 'tablet', os: 'OTHER', tags: [], metricsSource: 'NONE', addDefaultCheck: false,
    });
  });

  it('keeps an existing entry regardless of its IPv6 spelling', async () => {
    const parts = setup(['FD41:4E49:4D41:0:0:0:0:7']);
    expect(await enrolLinkedDevice(origin, parts, ipad)).toBe('already-added');
    expect(parts.createDevice).not.toHaveBeenCalled();
  });

  it('refuses a hub reached through a different local port', async () => {
    const parts = setup([], 47101);
    expect(await enrolLinkedDevice(origin, parts, ipad)).toBe('not-linked');
    expect(parts.createDevice).not.toHaveBeenCalled();
  });
});
