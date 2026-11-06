import { describe, expect, it } from 'vitest';
import type { Hub, HubError, HubResult, Variables } from '$lib/hub/hub';
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
          ssh: { enabled: true, hostKey: { fingerprintSha256: 'SHA256:abc' } },
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

describe('devicesApi hosts', () => {
  function hostPage(hosts: readonly string[], next: string | null) {
    return { devices: { pageInfo: { hasNextPage: next !== null, endCursor: next }, edges: hosts.map((host) => ({ node: { host } })) } };
  }

  /** A hub that answers `DeviceHosts` by its `after` cursor and records the cursors asked for. */
  function pagedHub(pageAt: (after: string | null) => HubResult<unknown>) {
    const asked: (string | null)[] = [];
    const request = (_document: unknown, variables: Variables) => {
      const after = (variables['after'] as string | null | undefined) ?? null;
      asked.push(after);
      return Promise.resolve(pageAt(after));
    };
    return { hub: { ...fakeHub().hub, request: request as Hub['request'] }, asked };
  }

  it('R-060 reads every page of hosts, following the cursor', async () => {
    const pages: Record<string, ReturnType<typeof hostPage>> = { start: hostPage(['nas.lan'], 'c1'), c1: hostPage(['100.64.0.7'], null) };
    const { hub, asked } = pagedHub((after) => ({ ok: true, value: pages[after ?? 'start'] }));
    expect(await devicesApi(hub).hosts()).toEqual({ ok: true, value: ['nas.lan', '100.64.0.7'] });
    expect(asked).toEqual([null, 'c1']);
  });

  it('R-060 fails after a bounded number of pages when the hub always says there is more', async () => {
    const { hub, asked } = pagedHub(() => ({ ok: true, value: hostPage(['loop.lan'], 'same') }));
    const result = await devicesApi(hub).hosts();
    expect(asked).toHaveLength(50);
    expect(result).toMatchObject({ ok: false, error: { code: 'malformed' } });
  });

  it('R-060 fails at once when a page says there is more but gives no cursor', async () => {
    const page = { devices: { pageInfo: { hasNextPage: true, endCursor: null }, edges: [{ node: { host: 'nas.lan' } }] } };
    const { hub, asked } = pagedHub(() => ({ ok: true, value: page }));
    const result = await devicesApi(hub).hosts();
    expect(asked).toEqual([null]);
    expect(result).toMatchObject({ ok: false, error: { code: 'malformed' } });
  });

  it('R-060 passes a hub error through as a result', async () => {
    const { hub } = pagedHub(() => ({ ok: false, error: DENIED }));
    expect(await devicesApi(hub).hosts()).toEqual({ ok: false, error: DENIED });
  });
});
