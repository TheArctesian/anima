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
  it.each([4400, 4500])('NET-004: a fatal close %i from the hub stops reconnecting and counts as contact', async (code) => {
    const hub = newHub();
    await settle();
    onlySocket().accept();
    await settle();
    onlySocket().close(code);
    await settle(120_000);
    expect(hub.status.current).toEqual({ connection: 'connected', lastContactAt: NOW });
    expect(FakeSocket.opened).toHaveLength(1);
    void hub.close();
  });

  it('NET-004: a fatal close that is not from graphql-transport-ws (1002) reads unreachable', async () => {
    const hub = newHub();
    await settle();
    onlySocket().accept();
    await settle();
    onlySocket().close(1002);
    await settle(120_000);
    expect(hub.status.current).toEqual({ connection: 'unreachable', lastContactAt: null });
    void hub.close();
  });
});

/** Open the newest socket and refuse its token, as the hub does. */
async function refuseNewest(): Promise<void> {
  const socket = FakeSocket.opened.at(-1);
  socket?.accept();
  await settle();
  socket?.close(4403, 'Forbidden');
  await settle();
}

describe('Hub connects when it is created, and the token is refused', () => {
  it('NET-004: a 4403 at start is contact, then probes on the backoff (wait grows to the cap)', async () => {
    const hub = newHub();
    await settle();
    await refuseNewest();
    for (const waitMs of [1000, 2000, 4000, 8000, 16_000, 30_000, 30_000]) {
      const before = FakeSocket.opened.length;
      await settle(waitMs - 1);
      expect(FakeSocket.opened).toHaveLength(before);
      await settle(1);
      expect(FakeSocket.opened).toHaveLength(before + 1);
      await refuseNewest();
      expect(hub.status.current).toEqual({ connection: 'connected', lastContactAt: NOW });
    }
    void hub.close();
  });

  it('NET-004: a drop after a 4403 reaches unreachable', async () => {
    const hub = newHub();
    await settle();
    await refuseNewest();
    await settle(FIRST_RETRY_MS);
    expect(FakeSocket.opened).toHaveLength(2);
    await settle(DEFAULT_TIMEOUT_MS);
    expect(hub.status.current.connection).toBe('unreachable');
    void hub.close();
  });

  it('API-002: each probe after a 4403 reads the token again, so a new login connects', async () => {
    const tokens: (string | null)[] = [null, 's3cret-token'];
    const hub = newHub(() => Promise.resolve(tokens.shift() ?? null));
    await settle();
    await refuseNewest();
    expect(onlySocket().sent[0]).toEqual({ type: 'connection_init', payload: {} });
    await settle(FIRST_RETRY_MS);
    const probe = FakeSocket.opened.at(-1);
    if (probe === undefined) throw new Error('no probe socket');
    await acknowledge(probe);
    expect(probe.sent[0]).toEqual({ type: 'connection_init', payload: { authorization: 'Bearer s3cret-token' } });
    expect(hub.status.current.connection).toBe('connected');
    void hub.close();
  });
});

describe('Hub.close failure', () => {
  it('CQ-024: close rejects with the reason when the open socket cannot be closed', async () => {
    const hub = newHub();
    await settle();
    await acknowledge(onlySocket());
    onlySocket().closeFailure = new Error('the socket is broken');
    await expect(hub.close()).rejects.toThrow('the socket is broken');
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
