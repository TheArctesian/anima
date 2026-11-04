import { describe, expect, it } from 'vitest';
import { INITIAL_STATUS, nextStatus, type Connection, type HubStatus, type StatusSignal } from './status';

const EARLIER = new Date('2026-11-04T09:00:00Z');
const NOW = new Date('2026-11-04T10:00:00Z');

function at(connection: Connection): HubStatus {
  return { connection, lastContactAt: EARLIER };
}

describe('nextStatus', () => {
  it('NAV-003: starts connecting with no contact', () => {
    expect(INITIAL_STATUS).toEqual({ connection: 'connecting', lastContactAt: null });
  });

  it.each<[Connection, StatusSignal, Connection, Date]>([
    ['connecting', 'opened', 'connected', NOW],
    ['unreachable', 'opened', 'connected', NOW],
    ['connecting', 'answered', 'connected', NOW],
    ['unreachable', 'answered', 'connected', NOW],
    ['unreachable', 'refused', 'connected', NOW],
    ['connected', 'closed', 'connecting', EARLIER],
    ['connecting', 'closed', 'unreachable', EARLIER],
    ['unreachable', 'closed', 'unreachable', EARLIER],
    ['connected', 'error', 'unreachable', EARLIER],
    ['connecting', 'error', 'unreachable', EARLIER],
    ['connected', 'failed', 'unreachable', EARLIER],
    ['connecting', 'failed', 'unreachable', EARLIER],
  ])('NAV-003, NET-004: %s + %s = %s', (from, signal, connection, lastContactAt) => {
    expect(nextStatus(at(from), signal, NOW)).toEqual({ connection, lastContactAt });
  });

  it('NET-004: a refused token still counts as contact', () => {
    expect(nextStatus(INITIAL_STATUS, 'refused', NOW)).toEqual({ connection: 'connected', lastContactAt: NOW });
  });

  it('NET-004: a failure before any answer keeps no contact time', () => {
    expect(nextStatus(INITIAL_STATUS, 'failed', NOW)).toEqual({ connection: 'unreachable', lastContactAt: null });
  });
});
