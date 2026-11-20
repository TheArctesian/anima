import { describe, expect, it } from 'vitest';
import type { HubStatus } from '$lib/hub/hub';
import { AGENT_USAGE, DEVICES, SETTINGS, hubName, isActive, statusLabel, unreachableBanner } from './nav';

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

  it('AIS-020: Agent usage is active on its page, which is two segments deep', () => {
    expect(isActive(AGENT_USAGE, '/agents/usage')).toBe(true);
    expect(isActive(AGENT_USAGE, '/agents')).toBe(false);
  });

  it('NET-004: the banner names the problem, the hub, the advice and the data age', () => {
    const lastContactAt = new Date(NOW.getTime() - 2 * 3_600_000);
    expect(unreachableBanner(status('unreachable', lastContactAt), 'https://hub.anima.ing', NOW, false)).toEqual({
      title: 'Not on the network',
      origin: 'https://hub.anima.ing',
      advice: 'Check the VPN client',
      age: 'Data from 2 hours ago',
    });
  });

  it('NET-004, R-222: for a hosted hub the banner names the network of the computer, not a VPN', () => {
    expect(unreachableBanner(status('unreachable', null), 'http://127.0.0.1:4810', NOW, true)?.advice).toBe("Check this computer's network");
  });

  it('R-224: the bar names a hosted hub by who hosts it, and each other hub by its host', () => {
    expect(hubName('http://127.0.0.1:4810', true)).toBe('Hosted by Anima.ing');
    expect(hubName('https://hub.example.net', false)).toBe('hub.example.net');
  });

  it('FND-007: while a copy is on the screen the banner gives the age of the copy, also after a start with no hub', () => {
    const copyFrom = new Date(NOW.getTime() - 3 * 24 * 3_600_000);
    const lastContactAt = new Date(NOW.getTime() - 3_600_000);
    expect(unreachableBanner({ connection: 'unreachable', lastContactAt, copyFrom }, 'https://h', NOW, false)?.age).toBe('Data from 3 days ago');
    expect(unreachableBanner({ connection: 'unreachable', lastContactAt: null, copyFrom }, 'https://h', NOW, false)?.age).toBe('Data from 3 days ago');
  });

  it('NET-004: with no contact yet the banner says no data was received', () => {
    expect(unreachableBanner(status('unreachable'), 'https://h', NOW, false)?.age).toBe('No data yet');
  });

  it.each(['connected', 'connecting'] as const)('NET-004: no banner while %s', (connection) => {
    expect(unreachableBanner(status(connection), 'https://h', NOW, false)).toBeNull();
  });
});
