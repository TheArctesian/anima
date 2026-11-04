import { describe, expect, it } from 'vitest';
import type { HubStatus } from '$lib/hub/hub';
import { DEVICES, SETTINGS, isActive, statusLabel, unreachableBanner } from './nav';

const NOW = new Date('2026-11-04T10:00:00Z');
const status = (connection: HubStatus['connection'], lastContactAt: Date | null = null): HubStatus => ({
  connection,
  lastContactAt,
});

describe('navigation view logic', () => {
  it.each([
    ['connected', 'Connected'],
    ['connecting', 'Disconnected'],
    ['unreachable', 'Disconnected'],
  ] as const)('NAV-003: status %s reads %s', (connection, label) => {
    expect(statusLabel(status(connection))).toBe(label);
  });

  it('NAV-002: an item is active on its page and below it, not on a prefix of another name', () => {
    expect(isActive(DEVICES, '/devices')).toBe(true);
    expect(isActive(DEVICES, '/devices/d1')).toBe(true);
    expect(isActive(DEVICES, '/devices-old')).toBe(false);
    expect(isActive(SETTINGS, '/devices')).toBe(false);
  });

  it('NET-004: the banner names the problem, the hub, the advice and the data age', () => {
    const lastContactAt = new Date(NOW.getTime() - 2 * 3_600_000);
    expect(unreachableBanner(status('unreachable', lastContactAt), 'https://hub.anima.ing', NOW)).toEqual({
      title: 'Not on the network',
      origin: 'https://hub.anima.ing',
      advice: 'Check the VPN client',
      age: 'Data from 2 hours ago',
    });
  });

  it('NET-004: with no contact yet the banner says no data was received', () => {
    expect(unreachableBanner(status('unreachable'), 'https://h', NOW)?.age).toBe('No data received yet');
  });

  it.each(['connected', 'connecting'] as const)('NET-004: no banner while %s', (connection) => {
    expect(unreachableBanner(status(connection), 'https://h', NOW)).toBeNull();
  });
});
