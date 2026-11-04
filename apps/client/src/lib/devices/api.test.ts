import { describe, expect, it } from 'vitest';
import type { HubError } from '$lib/hub/hub';
import { devicesApi, type StatusChange } from './api';
import { fakeHub } from './hub.fixture';
import { DEFAULT_QUERY, listVariables } from './query';

const PAGE = {
  devices: {
    totalCount: 1,
    pageInfo: { hasNextPage: false, endCursor: null },
    edges: [
      {
        node: {
          id: 'd1',
          name: 'nas',
          host: 'nas.lan',
          ip: null,
          tags: ['lab'],
          maintenance: false,
          status: 'UP',
          uptime: 0.99,
          ssh: { enabled: true },
          checks: [],
        },
      },
    ],
  },
};

const DENIED: HubError = { code: 'UNAUTHENTICATED', message: 'Sign in again.' };

function sinkOf() {
  const heard: (StatusChange | HubError | 'complete')[] = [];
  return {
    heard,
    sink: {
      next: (value: StatusChange) => heard.push(value),
      error: (error: HubError) => heard.push(error),
      complete: () => heard.push('complete'),
    },
  };
}

describe('devicesApi', () => {
  it('DEV-001 sends the list variables and returns the typed page', async () => {
    const fake = fakeHub({ Devices: PAGE });
    const variables = listVariables({ ...DEFAULT_QUERY, statuses: ['DOWN'] }, 'c1');
    const result = await devicesApi(fake.hub).list(variables);
    expect(fake.requests).toEqual([{ operation: 'Devices', variables }]);
    expect(result).toEqual({ ok: true, value: PAGE.devices });
  });

  it('DEV-001 passes a hub error through as a result', async () => {
    const fake = fakeHub({ Devices: DENIED });
    expect(await devicesApi(fake.hub).list(listVariables(DEFAULT_QUERY))).toEqual({ ok: false, error: DENIED });
  });

  it('DEV-002 lists the tags', async () => {
    const fake = fakeHub({ Tags: { tags: ['home', 'lab'] } });
    expect(await devicesApi(fake.hub).tags()).toEqual({ ok: true, value: ['home', 'lab'] });
    expect(fake.requests.map((request) => request.operation)).toEqual(['Tags']);
  });

  it('DEV-003 delivers pushed status changes until stopped', () => {
    const fake = fakeHub();
    const { heard, sink } = sinkOf();
    const stop = devicesApi(fake.hub).watchStatus(sink);
    const change: StatusChange = { deviceId: 'd1', status: 'DOWN', at: '2026-01-01T00:00:00Z' };
    fake.push('DeviceStatusChanged', { deviceStatusChanged: change });
    stop();
    fake.push('DeviceStatusChanged', { deviceStatusChanged: { ...change, status: 'UP' } });
    expect(heard).toEqual([change]);
    expect(fake.listeners('DeviceStatusChanged')).toBe(0);
  });

  it('DEV-003 passes a subscription error to the sink', () => {
    const fake = fakeHub();
    const { heard, sink } = sinkOf();
    devicesApi(fake.hub).watchStatus(sink);
    fake.fail('DeviceStatusChanged', DENIED);
    expect(heard).toEqual([DENIED]);
  });

  it('DEV-003 forwards the end of the subscription, also when the hub closes', async () => {
    const fake = fakeHub();
    const { heard, sink } = sinkOf();
    devicesApi(fake.hub).watchStatus(sink);
    await fake.hub.close();
    devicesApi(fake.hub).watchStatus(sink);
    expect(heard).toEqual(['complete', 'complete']);
    expect(fake.listeners('DeviceStatusChanged')).toBe(0);
  });
});
