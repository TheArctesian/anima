import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  DEFAULT_TIMEOUT_MS,
  FakeSocket,
  FIRST_RETRY_MS,
  liveSubscription,
  newHub,
  NOW,
  onlySocket,
  recordingSink,
  settle,
  STATUS_CHANGED,
} from './fake';

beforeEach(() => {
  FakeSocket.opened = [];
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
});

afterEach(() => {
  vi.useRealTimers();
});

describe('Hub.subscribe', () => {
  it('API-002: sends the token in connection_init and delivers next payloads', async () => {
    const hub = newHub();
    const { events, sink } = recordingSink();
    const { socket } = await liveSubscription(hub, sink);
    expect(socket.url).toBe('wss://hub.anima.ing/graphql');
    expect(socket.protocol).toBe('graphql-transport-ws');
    expect(socket.sent[0]).toEqual({ type: 'connection_init', payload: { authorization: 'Bearer s3cret-token' } });
    expect(socket.sent[1]).toMatchObject({ type: 'subscribe', payload: { variables: { deviceId: 'd1' } } });
    const id = socket.subscribeId();
    socket.receive({ id, type: 'next', payload: { data: { deviceStatusChanged: { status: 'DOWN' } } } });
    socket.receive({ id, type: 'complete' });
    await settle();
    expect(events).toEqual({ next: [{ deviceStatusChanged: { status: 'DOWN' } }], errors: [], completed: 1 });
    void hub.close();
  });

  it('LOG-007: the token is never in the socket address', async () => {
    const hub = newHub();
    const { socket } = await liveSubscription(hub, recordingSink().sink);
    expect(socket.url).not.toContain('s3cret-token');
    void hub.close();
  });

  it('API-002: all subscriptions of one hub share one socket', async () => {
    const hub = newHub();
    await liveSubscription(hub, recordingSink().sink);
    hub.subscribe(STATUS_CHANGED, {}, recordingSink().sink);
    await settle();
    expect(FakeSocket.opened).toHaveLength(1);
    expect(onlySocket().sent.filter((sent) => sent.type === 'subscribe')).toHaveLength(2);
    void hub.close();
  });
});

describe('Hub.subscribe and the connection status', () => {
  it('NAV-003: an open socket means connected', async () => {
    const hub = newHub();
    await liveSubscription(hub, recordingSink().sink);
    expect(hub.status.current).toEqual({ connection: 'connected', lastContactAt: NOW });
    void hub.close();
  });

  it('NET-004: close 4403 is contact, ends the subscription as UNAUTHENTICATED and is not retried', async () => {
    const hub = newHub();
    const { events, sink } = recordingSink();
    const { socket } = await liveSubscription(hub, sink);
    socket.close(4403, 'Forbidden');
    await settle(120_000);
    expect(events.errors.map((error) => error.code)).toEqual(['UNAUTHENTICATED']);
    expect(hub.status.current.connection).toBe('connected');
    expect(FakeSocket.opened).toHaveLength(1);
    expect(JSON.stringify(events.errors)).not.toContain('s3cret-token');
  });

  it('NET-004: a dropped socket reconnects after a backoff and the subscription continues', async () => {
    const hub = newHub();
    const { events, sink } = recordingSink();
    const { socket } = await liveSubscription(hub, sink);
    socket.close(1006);
    await settle();
    expect(hub.status.current.connection).toBe('connecting');
    await settle(FIRST_RETRY_MS);
    expect(FakeSocket.opened).toHaveLength(2);
    expect(events.errors).toEqual([]);
    void hub.close();
  });
});

describe('Hub.subscribe and the token', () => {
  it('API-002: each new connection reads the token again', async () => {
    const tokens = ['first-token', 'second-token'];
    const hub = newHub(() => Promise.resolve(tokens.shift() ?? null));
    const { socket } = await liveSubscription(hub, recordingSink().sink);
    socket.close(1006);
    await settle(FIRST_RETRY_MS);
    const [, second] = FakeSocket.opened;
    second?.accept();
    await settle();
    expect(second?.sent[0]).toEqual({ type: 'connection_init', payload: { authorization: 'Bearer second-token' } });
    void hub.close();
  });

  it('API-003: before login connection_init carries no authorization', async () => {
    const hub = newHub(() => Promise.resolve(null));
    const { socket } = await liveSubscription(hub, recordingSink().sink);
    expect(socket.sent[0]).toEqual({ type: 'connection_init', payload: {} });
    void hub.close();
  });
});

describe('Hub.subscribe endings', () => {
  it('a payload with errors ends the subscription with the typed error', async () => {
    const hub = newHub();
    const { events, sink } = recordingSink();
    const { socket } = await liveSubscription(hub, sink);
    const id = socket.subscribeId();
    socket.receive({ id, type: 'next', payload: { errors: [{ message: 'No device.', extensions: { code: 'NOT_FOUND' } }] } });
    await settle();
    expect(events).toEqual({ next: [], errors: [{ code: 'NOT_FOUND', message: 'No device.' }], completed: 0 });
    expect(socket.sent.at(-1)).toEqual({ id, type: 'complete' });
    void hub.close();
  });

  it('after unsubscribe the hub is told and the sink hears nothing more', async () => {
    const hub = newHub();
    const { events, sink } = recordingSink();
    const { socket, stop } = await liveSubscription(hub, sink);
    const id = socket.subscribeId();
    stop();
    socket.receive({ id, type: 'next', payload: { data: { deviceStatusChanged: { status: 'UP' } } } });
    await settle();
    expect(socket.sent.at(-1)).toEqual({ id, type: 'complete' });
    expect(events).toEqual({ next: [], errors: [], completed: 0 });
    void hub.close();
  });
});

describe('Hub.subscribe reconnect backoff', () => {
  it('NET-004: reconnects wait 1 s, doubling to at most 30 s, and never stop', async () => {
    const hub = newHub();
    const { socket } = await liveSubscription(hub, recordingSink().sink);
    socket.close(1006);
    for (const waitMs of [1000, 2000, 4000, 8000, 16_000, 30_000, 30_000, 30_000]) {
      const before = FakeSocket.opened.length;
      await settle(waitMs - 1);
      expect(FakeSocket.opened).toHaveLength(before);
      await settle(1);
      expect(FakeSocket.opened).toHaveLength(before + 1);
      FakeSocket.opened.at(-1)?.close(1006);
    }
    expect(hub.status.current.connection).toBe('unreachable');
    void hub.close();
  });
});

describe('Hub.subscribe and close', () => {
  it('NET-004: when the last subscription stops, the socket stays open and the status stays connected', async () => {
    const hub = newHub();
    const { socket, stop } = await liveSubscription(hub, recordingSink().sink);
    stop();
    await settle(DEFAULT_TIMEOUT_MS * 3);
    expect(socket.closedWith).toBeUndefined();
    expect(hub.status.current).toEqual({ connection: 'connected', lastContactAt: NOW });
    void void hub.close();
  });

});

describe('Hub.subscribe on a stalled link', () => {
  it('NET-004: no connection_ack within the timeout is unreachable, then the socket retries', async () => {
    const hub = newHub();
    const { events, sink } = recordingSink();
    hub.subscribe(STATUS_CHANGED, {}, sink);
    await settle();
    onlySocket().accept();
    await settle(DEFAULT_TIMEOUT_MS);
    expect(onlySocket().closedWith).toBe(4504);
    expect(hub.status.current.connection).toBe('unreachable');
    await settle(FIRST_RETRY_MS);
    expect(FakeSocket.opened).toHaveLength(2);
    expect(events.errors).toEqual([]);
    void hub.close();
  });
});

describe('Hub.subscribe with an unreadable token', () => {
  it('LOG-007: a rejecting token provider ends the subscription as UNAUTHENTICATED, sends no token and is not retried', async () => {
    const hub = newHub(() => Promise.reject(new Error('keychain item s3cret-token locked')));
    const { events, sink } = recordingSink();
    hub.subscribe(STATUS_CHANGED, {}, sink);
    await settle();
    onlySocket().accept();
    await settle(120_000);
    expect(events.errors).toEqual([{ code: 'UNAUTHENTICATED', message: 'The access token could not be read.' }]);
    expect(onlySocket().sent).toEqual([]);
    expect(JSON.stringify(events.errors)).not.toContain('s3cret-token');
    expect(FakeSocket.opened).toHaveLength(1);
    expect(hub.status.current.connection).toBe('connecting');
  });
});

describe('Hub.subscribe recovery', () => {
  it('NET-004: after one unreadable token the next connection reads it again and a drop is retried', async () => {
    let reads = 0;
    const hub = newHub(() => {
      reads += 1;
      return reads === 1 ? Promise.reject(new Error('locked')) : Promise.resolve('s3cret-token');
    });
    const first = recordingSink();
    hub.subscribe(STATUS_CHANGED, {}, first.sink);
    await settle();
    onlySocket().accept();
    await settle();
    expect(first.events.errors.map((error) => error.code)).toEqual(['UNAUTHENTICATED']);
    const second = recordingSink();
    hub.subscribe(STATUS_CHANGED, {}, second.sink);
    await settle();
    const socket = FakeSocket.opened.at(-1);
    socket?.accept();
    await settle();
    expect(socket?.sent[0]).toEqual({ type: 'connection_init', payload: { authorization: 'Bearer s3cret-token' } });
    socket?.receive({ type: 'connection_ack' });
    await settle();
    const opened = FakeSocket.opened.length;
    socket?.close(1006);
    await settle(FIRST_RETRY_MS);
    expect(FakeSocket.opened).toHaveLength(opened + 1);
    expect(second.events.errors).toEqual([]);
    void hub.close();
  });

  it.each([4500, 4005])('a close %i from the hub ends the subscription as INTERNAL', async (code) => {
    const hub = newHub();
    const { events, sink } = recordingSink();
    const { socket } = await liveSubscription(hub, sink);
    socket.close(code);
    await settle();
    expect(events.errors.map((error) => error.code)).toEqual(['INTERNAL']);
    void hub.close();
  });

});
