import { describe, expect, it } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import type { HubError } from '$lib/hub/hub';
import { eventsApi, type LogEvent } from './api';
import { DEFAULT_LOG_FILTER, eventVariables } from './log';

const ROW: LogEvent = {
  id: 'e1',
  at: '2026-06-01T11:59:00.000Z',
  severity: 'WARNING',
  source: 'DEVICE',
  deviceId: 'd1',
  deviceName: 'nas',
  message: 'Disk almost full',
};

const PAGE = { events: { pageInfo: { hasNextPage: false, endCursor: null }, edges: [{ node: ROW }] } };

const DENIED: HubError = { code: 'UNAUTHENTICATED', message: 'Sign in again.' };

function sinkOf() {
  const heard: (LogEvent | HubError | 'complete')[] = [];
  return {
    heard,
    sink: {
      next: (value: LogEvent) => heard.push(value),
      error: (error: HubError) => heard.push(error),
      complete: () => heard.push('complete'),
    },
  };
}

describe('eventsApi.list', () => {
  it('LOG-009 sends the filter variables and returns the page', async () => {
    const { hub, requests } = fakeHub({ Events: PAGE });
    const variables = eventVariables(DEFAULT_LOG_FILTER, new Date('2026-06-01T12:00:00.000Z'));
    const result = await eventsApi(hub).list(variables);
    expect(requests).toEqual([{ operation: 'Events', variables }]);
    expect(result).toEqual({ ok: true, value: PAGE.events });
  });

  it('LOG-009 passes a hub error through', async () => {
    const { hub } = fakeHub({ Events: DENIED });
    expect(await eventsApi(hub).list({})).toEqual({ ok: false, error: DENIED });
  });
});

describe('eventsApi.watch', () => {
  it('LOG-010 subscribes with the minimum severity and hands over the pushed event', () => {
    const { hub, subscriptions, push } = fakeHub();
    const { heard, sink } = sinkOf();
    eventsApi(hub).watch('WARNING', sink);
    push('EventAdded', { eventAdded: ROW });
    expect(subscriptions).toEqual([{ operation: 'EventAdded', variables: { minSeverity: 'WARNING' } }]);
    expect(heard).toEqual([ROW]);
  });

  it('LOG-010 passes a subscription error to the sink', () => {
    const { hub, fail } = fakeHub();
    const { heard, sink } = sinkOf();
    eventsApi(hub).watch('INFO', sink);
    fail('EventAdded', DENIED);
    expect(heard).toEqual([DENIED]);
  });

  it('LOG-010 the returned function stops listening', () => {
    const { hub, listeners, push } = fakeHub();
    const { heard, sink } = sinkOf();
    const stop = eventsApi(hub).watch('INFO', sink);
    expect(listeners('EventAdded')).toBe(1);
    stop();
    push('EventAdded', { eventAdded: ROW });
    expect(listeners('EventAdded')).toBe(0);
    expect(heard).toEqual([]);
  });

  it('LOG-010 forwards the end of the subscription when the hub closes, and a later watch ends at once', async () => {
    const fake = fakeHub();
    const { heard, sink } = sinkOf();
    eventsApi(fake.hub).watch('INFO', sink);
    await fake.hub.close();
    eventsApi(fake.hub).watch('INFO', sink);
    expect(heard).toEqual(['complete', 'complete']);
    expect(fake.listeners('EventAdded')).toBe(0);
  });

  it('LOG-010 a stop after the hub closed is harmless and adds nothing', async () => {
    const fake = fakeHub();
    const { heard, sink } = sinkOf();
    const stop = eventsApi(fake.hub).watch('INFO', sink);
    await fake.hub.close();
    stop();
    fake.push('EventAdded', { eventAdded: ROW });
    expect(heard).toEqual(['complete']);
  });
});
