import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  acknowledge,
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

describe('Hub connects when it is created', () => {
  it('NAV-003: the status is connected after connection_ack, with no request or subscription', async () => {
    const hub = newHub();
    await settle();
    expect(FakeSocket.opened).toHaveLength(1);
    await acknowledge(onlySocket());
    expect(hub.status.current).toEqual({ connection: 'connected', lastContactAt: NOW });
    void hub.close();
  });

  it('NET-004: a socket that fails reads unreachable', async () => {
    const hub = newHub();
    await settle();
    onlySocket().close(1006);
    await settle();
    expect(hub.status.current).toEqual({ connection: 'unreachable', lastContactAt: null });
    void hub.close();
  });

  it('NET-004: no connection_ack within the timeout reads unreachable', async () => {
    const hub = newHub();
    await settle(DEFAULT_TIMEOUT_MS - 1);
    expect(hub.status.current.connection).toBe('connecting');
    await settle(1);
    expect(hub.status.current.connection).toBe('unreachable');
    void hub.close();
  });

  it('NET-004: a later connection_ack recovers the status', async () => {
    const hub = newHub();
    await settle();
    onlySocket().close(1006);
    await settle(FIRST_RETRY_MS);
    const retry = FakeSocket.opened.at(-1);
    if (retry === undefined) throw new Error('no retry socket');
    await acknowledge(retry);
    expect(FakeSocket.opened).toHaveLength(2);
    expect(hub.status.current).toEqual({ connection: 'connected', lastContactAt: NOW });
    void hub.close();
  });

  it('NET-004: an acknowledged socket stays connected past the timeout', async () => {
    const hub = newHub();
    await settle();
    await acknowledge(onlySocket());
    await settle(DEFAULT_TIMEOUT_MS * 3);
    expect(hub.status.current.connection).toBe('connected');
    void hub.close();
  });
});

describe('Hub connects when it is created, and the hub ends the connection', () => {
  it('NET-004: a fatal close (4500) stops reconnecting and reads unreachable', async () => {
    const hub = newHub();
    await settle();
    await acknowledge(onlySocket());
    onlySocket().close(4500);
    await settle(120_000);
    expect(hub.status.current).toEqual({ connection: 'unreachable', lastContactAt: NOW });
    expect(FakeSocket.opened).toHaveLength(1);
    void hub.close();
  });
});

describe('Hub connects when it is created, and the token is refused', () => {
  it('NET-004: a 4403 at start is contact, and the socket does not reconnect by itself', async () => {
    const hub = newHub();
    await settle();
    onlySocket().accept();
    await settle();
    onlySocket().close(4403, 'Forbidden');
    await settle(120_000);
    expect(hub.status.current).toEqual({ connection: 'connected', lastContactAt: NOW });
    expect(FakeSocket.opened).toHaveLength(1);
    void hub.close();
  });

  it('API-002: after a 4403 at start, the next subscription connects again with the token read anew', async () => {
    const hub = newHub();
    await settle();
    onlySocket().close(4403, 'Forbidden');
    await settle();
    hub.subscribe(STATUS_CHANGED, {}, recordingSink().sink);
    await settle();
    const again = FakeSocket.opened.at(-1);
    again?.accept();
    await settle();
    expect(FakeSocket.opened).toHaveLength(2);
    expect(again?.sent[0]).toEqual({ type: 'connection_init', payload: { authorization: 'Bearer s3cret-token' } });
    void hub.close();
  });
});

describe('Hub.close', () => {
  it('API-002: close ends every subscription with complete and closes the socket with 1000', async () => {
    const hub = newHub();
    const { events, sink } = recordingSink();
    const { socket } = await liveSubscription(hub, sink);
    const closing = hub.close();
    expect(events).toEqual({ next: [], errors: [], completed: 1 });
    await settle();
    expect(socket.closedWith).toBe(1000);
    await expect(closing).resolves.toBeUndefined();
  });

  it('API-002: a subscription after close completes at once and never connects', async () => {
    const hub = newHub();
    await liveSubscription(hub, recordingSink().sink);
    void hub.close();
    const { events, sink } = recordingSink();
    hub.subscribe(STATUS_CHANGED, {}, sink);
    expect(events).toEqual({ next: [], errors: [], completed: 1 });
    await settle(120_000);
    expect(FakeSocket.opened).toHaveLength(1);
  });

  it.each([
    ['right after creation', async () => undefined, 4499],
    ['while acknowledged', (socket: FakeSocket) => acknowledge(socket), 1000],
    ['during a retry wait', async (socket: FakeSocket) => {
      socket.close(1006);
      await settle();
    }, 1006],
  ])('NET-004: close %s resolves, opens no socket and leaves no timer', async (_label, before, closedWith) => {
    const hub = newHub();
    await settle();
    await before(onlySocket());
    const status = hub.status.current;
    await expect(hub.close()).resolves.toBeUndefined();
    await settle(120_000);
    expect(vi.getTimerCount()).toBe(0);
    expect(FakeSocket.opened).toHaveLength(1);
    expect(onlySocket().closedWith).toBe(closedWith);
    expect(hub.status.current).toEqual(status);
  });

  it.each([
    ['an abnormal close', (socket: FakeSocket) => socket.close(1006)],
    ['a socket error', (socket: FakeSocket) => socket.onerror?.({ type: 'error' })],
  ])('NET-004: after close, a pending attempt that ends with %s changes nothing', async (_label, fail) => {
    const hub = newHub();
    await settle();
    const socket = onlySocket();
    await expect(hub.close()).resolves.toBeUndefined();
    fail(socket);
    await settle(120_000);
    expect(hub.status.current.connection).toBe('connecting');
    expect(FakeSocket.opened).toHaveLength(1);
  });
});
