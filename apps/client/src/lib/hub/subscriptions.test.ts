import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { parse } from 'graphql';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createHub, DEFAULT_HUB_ADDRESS, type HubError, type Sink, type TokenProvider } from './hub';
import { retryDelayMs } from './subscriptions';

const STATUS_CHANGED = parse('subscription S($deviceId: ID) { deviceStatusChanged(deviceId: $deviceId) { status } }') as TypedDocumentNode<
  { deviceStatusChanged: { status: string } },
  { deviceId?: string }
>;
const NOW = new Date('2026-11-04T10:00:00Z');

/** The hub side of one fake graphql-transport-ws connection. */
class FakeSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;
  static opened: FakeSocket[] = [];

  readyState = FakeSocket.CONNECTING;
  readonly sent: Record<string, unknown>[] = [];
  onopen: ((event: unknown) => void) | null = null;
  onclose: ((event: unknown) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;

  constructor(
    readonly url: string,
    readonly protocol: string,
  ) {
    FakeSocket.opened.push(this);
  }

  send(text: string): void {
    this.sent.push(JSON.parse(text) as Record<string, unknown>);
  }

  close(code = 1000, reason = ''): void {
    if (this.readyState === FakeSocket.CLOSED) return;
    this.readyState = FakeSocket.CLOSED;
    this.onclose?.({ code, reason, wasClean: code === 1000 });
  }

  accept(): void {
    this.readyState = FakeSocket.OPEN;
    this.onopen?.({});
  }

  receive(message: Record<string, unknown>): void {
    this.onmessage?.({ data: JSON.stringify(message) });
  }

  subscribeId(): string {
    const message = this.sent.find((sent) => sent.type === 'subscribe');
    return typeof message?.id === 'string' ? message.id : '';
  }
}

function recordingSink() {
  const events: { next: unknown[]; errors: HubError[]; completed: number } = { next: [], errors: [], completed: 0 };
  const sink: Sink<unknown> = {
    next: (value) => events.next.push(value),
    error: (error) => events.errors.push(error),
    complete: () => {
      events.completed += 1;
    },
  };
  return { events, sink };
}

function newHub(token: TokenProvider = () => Promise.resolve('s3cret-token')) {
  return createHub({
    address: DEFAULT_HUB_ADDRESS,
    token,
    WebSocket: FakeSocket as unknown as typeof WebSocket,
    now: () => NOW,
  });
}

/** Run pending promise callbacks and timers due by `ms`. */
async function settle(ms = 0): Promise<void> {
  await vi.advanceTimersByTimeAsync(ms);
}

function onlySocket(): FakeSocket {
  const [socket] = FakeSocket.opened;
  if (socket === undefined) throw new Error('no socket was opened');
  return socket;
}

/** Open a socket, subscribe and acknowledge; returns the socket. */
async function liveSubscription(hub: ReturnType<typeof newHub>, sink: Sink<unknown>) {
  const stop = hub.subscribe(STATUS_CHANGED, { deviceId: 'd1' }, sink);
  await settle();
  const socket = onlySocket();
  socket.accept();
  await settle();
  socket.receive({ type: 'connection_ack' });
  await settle();
  return { socket, stop };
}

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
    hub.close();
  });

  it('LOG-007: the token is never in the socket address', async () => {
    const hub = newHub();
    const { socket } = await liveSubscription(hub, recordingSink().sink);
    expect(socket.url).not.toContain('s3cret-token');
    hub.close();
  });

  it('API-002: all subscriptions of one hub share one socket', async () => {
    const hub = newHub();
    await liveSubscription(hub, recordingSink().sink);
    hub.subscribe(STATUS_CHANGED, {}, recordingSink().sink);
    await settle();
    expect(FakeSocket.opened).toHaveLength(1);
    expect(onlySocket().sent.filter((sent) => sent.type === 'subscribe')).toHaveLength(2);
    hub.close();
  });
});

describe('Hub.subscribe and the connection status', () => {
  it('NAV-003: an open socket means connected', async () => {
    const hub = newHub();
    await liveSubscription(hub, recordingSink().sink);
    expect(hub.status.current).toEqual({ connection: 'connected', lastContactAt: NOW });
    hub.close();
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
    await settle(retryDelayMs(0));
    expect(FakeSocket.opened).toHaveLength(2);
    expect(events.errors).toEqual([]);
    hub.close();
  });
});

describe('Hub.subscribe and the token', () => {
  it('API-002: each new connection reads the token again', async () => {
    const tokens = ['first-token', 'second-token'];
    const hub = newHub(() => Promise.resolve(tokens.shift() ?? null));
    const { socket } = await liveSubscription(hub, recordingSink().sink);
    socket.close(1006);
    await settle(retryDelayMs(0));
    const [, second] = FakeSocket.opened;
    second?.accept();
    await settle();
    expect(second?.sent[0]).toEqual({ type: 'connection_init', payload: { authorization: 'Bearer second-token' } });
    hub.close();
  });

  it('API-003: before login connection_init carries no authorization', async () => {
    const hub = newHub(() => Promise.resolve(null));
    const { socket } = await liveSubscription(hub, recordingSink().sink);
    expect(socket.sent[0]).toEqual({ type: 'connection_init', payload: {} });
    hub.close();
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
    hub.close();
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
    hub.close();
  });
});

describe('retryDelayMs', () => {
  it.each([
    [0, 1000],
    [1, 2000],
    [4, 16_000],
    [5, 30_000],
    [1000, 30_000],
  ])('NET-004: retry %i waits %i ms and never gives up', (retries, ms) => {
    expect(retryDelayMs(retries)).toBe(ms);
  });
});
