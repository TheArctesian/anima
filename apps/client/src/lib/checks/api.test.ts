import { expect, it } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import { checksApi, type NewHeartbeat } from './api';

const added: NewHeartbeat = {
  checkId: 'c1',
  deviceId: 'd1',
  heartbeat: { at: '2026-11-04T10:00:00Z', status: 'UP', responseTimeMs: 4 },
};

it('DEV-002: watchHeartbeats subscribes for every device and hands over each new heartbeat', () => {
  const hub = fakeHub();
  const seen: NewHeartbeat[] = [];
  checksApi(hub.hub).watchHeartbeats({ next: (value) => seen.push(value), error: () => undefined, complete: () => undefined });
  expect(hub.subscriptions).toEqual([{ operation: 'HeartbeatAdded', variables: {} }]);
  hub.push('HeartbeatAdded', { heartbeatAdded: added });
  expect(seen).toEqual([added]);
});

it('DEV-002: the function watchHeartbeats returns stops listening', () => {
  const hub = fakeHub();
  const stop = checksApi(hub.hub).watchHeartbeats({ next: () => undefined, error: () => undefined, complete: () => undefined });
  stop();
  expect(hub.listeners('HeartbeatAdded')).toBe(0);
});
