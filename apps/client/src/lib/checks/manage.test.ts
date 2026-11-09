import { describe, expect, it } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import type { HubError } from '$lib/hub/hub';
import { checksApi, type DeviceCheck } from './api';

const CHECK: DeviceCheck = {
  id: 'c1',
  parent: null,
  type: 'TCP',
  name: 'ssh port',
  intervalSeconds: 60,
  retries: 3,
  retryIntervalSeconds: 20,
  degradedThresholdMs: 1000,
  invert: false,
  paused: false,
  status: 'UP',
  certificateDaysLeft: null,
  domainDaysLeft: null,
  lastHeartbeat: { at: '2026-11-04T10:00:00Z', status: 'UP', responseTimeMs: 4, message: null },
  config: { __typename: 'TcpConfig', port: 22, timeoutMs: 5000 },
};

describe('checksApi: checks of a device (API-008, CQ-045, CHK-005)', () => {
  it('Section 6.4: forDevice sends the device ID and returns each check with its config', async () => {
    const fake = fakeHub({ DeviceChecks: { device: { id: 'd1', checks: [CHECK] } } });
    expect(await checksApi(fake.hub).forDevice('d1')).toEqual({ ok: true, value: [CHECK] });
    expect(fake.requests).toEqual([{ operation: 'DeviceChecks', variables: { deviceId: 'd1' } }]);
  });

  it('Section 6.4: forDevice returns null when the hub has no such device', async () => {
    const fake = fakeHub({ DeviceChecks: { device: null } });
    expect(await checksApi(fake.hub).forDevice('gone')).toEqual({ ok: true, value: null });
  });

  it('D-10: heartbeats asks for a page after a cursor and returns the page', async () => {
    const heartbeats = {
      pageInfo: { hasNextPage: true, endCursor: 'k2' },
      edges: [{ node: { at: '2026-11-04T10:00:00Z', status: 'UP', responseTimeMs: 4, message: null } }],
    };
    const fake = fakeHub({ CheckHeartbeats: { check: { id: 'c1', heartbeats } } });
    expect(await checksApi(fake.hub).heartbeats('c1', 100, 'k1')).toEqual({ ok: true, value: heartbeats });
    expect(fake.requests).toEqual([{ operation: 'CheckHeartbeats', variables: { id: 'c1', first: 100, after: 'k1' } }]);
  });

  it('D-10: heartbeats of a check that is gone is null', async () => {
    const fake = fakeHub({ CheckHeartbeats: { check: null } });
    expect(await checksApi(fake.hub).heartbeats('c1', 100, null)).toEqual({ ok: true, value: null });
  });

  it('Section 6.4: create sends the input and returns the new check', async () => {
    const fake = fakeHub({ CreateCheck: { createCheck: CHECK } });
    const input = { deviceId: 'd1', name: 'ssh port', config: { tcp: { port: 22 } } };
    expect(await checksApi(fake.hub).create(input)).toEqual({ ok: true, value: CHECK });
    expect(fake.requests).toEqual([{ operation: 'CreateCheck', variables: { input } }]);
  });

  it('Section 6.4, K7: update sends ID and input; a VALIDATION error keeps its fields', async () => {
    const fake = fakeHub({ UpdateCheck: { updateCheck: CHECK } });
    expect(await checksApi(fake.hub).update('c1', { retries: 5 })).toEqual({ ok: true, value: CHECK });
    expect(fake.requests).toEqual([{ operation: 'UpdateCheck', variables: { id: 'c1', input: { retries: 5 } } }]);
    const denied: HubError = { code: 'VALIDATION', message: 'Check the form.', fields: [{ field: 'name', message: 'Required.' }] };
    expect(await checksApi(fakeHub({ UpdateCheck: denied }).hub).update('c1', { name: '' })).toEqual({ ok: false, error: denied });
  });

  it('Section 6.4: delete sends the ID and returns the deleted ID', async () => {
    const fake = fakeHub({ DeleteCheck: { deleteCheck: 'c1' } });
    expect(await checksApi(fake.hub).delete('c1')).toEqual({ ok: true, value: 'c1' });
    expect(fake.requests).toEqual([{ operation: 'DeleteCheck', variables: { id: 'c1' } }]);
  });

  it('CHK-005: setPaused sends the flag and returns the check as the hub now has it', async () => {
    const paused = { ...CHECK, paused: true };
    const fake = fakeHub({ SetCheckPaused: { setCheckPaused: paused } });
    expect(await checksApi(fake.hub).setPaused('c1', true)).toEqual({ ok: true, value: paused });
    expect(fake.requests).toEqual([{ operation: 'SetCheckPaused', variables: { id: 'c1', paused: true } }]);
  });
});
