import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { readAccessToken } from '$lib/desktop/token';
import { createHub, DEFAULT_HUB_ADDRESS, type HubAddress } from '$lib/hub/hub';
import { FAKE_WEBSOCKET, FakeSocket, recordingSink, settle, STATUS_CHANGED } from '$lib/hub/socket/fake';
import { accessApi } from './api';
import { clientStart, ORIGIN_REFUSED, type ClientStart } from './start';

/** A storage holding `stored` as the desktop app's hub address setting. */
function storage(stored: string | null = null): Storage {
  return { getItem: (key: string) => (key === 'anima.hubAddress' ? stored : null) } as Storage;
}

function browserAt(origin: string, stored: string | null = null): ClientStart {
  return clientStart({ desktop: false, storage: storage(stored), origin });
}

function addressOf(start: ClientStart): HubAddress {
  if (start.kind !== 'shell') throw new Error(`expected a shell start, got ${start.kind}`);
  return start.address;
}

describe('the hub address a client starts with', () => {
  it('CLI-002: in a browser the hub is the https origin that served the page', () => {
    expect(addressOf(browserAt('https://hub.example.org'))).toEqual({
      origin: 'https://hub.example.org',
      http: 'https://hub.example.org/graphql',
      socket: 'wss://hub.example.org/graphql',
    });
    expect(addressOf(browserAt('https://hub.example.org:8443')).socket).toBe('wss://hub.example.org:8443/graphql');
  });

  it('CLI-002: a browser ignores a stored hub address, so script on the page cannot point it at another hub', () => {
    expect(addressOf(browserAt('https://hub.example.org', 'https://other.example.net')).origin).toBe('https://hub.example.org');
  });

  it.each([
    ['http://localhost:5173', 'ws://localhost:5173/graphql'],
    ['http://127.0.0.1:4173', 'ws://127.0.0.1:4173/graphql'],
    ['http://[::1]:4173', 'ws://[::1]:4173/graphql'],
  ])('CLI-002: a loopback http origin %s uses http and ws on the same origin', (origin, socket) => {
    expect(addressOf(browserAt(origin))).toEqual({ origin, http: `${origin}/graphql`, socket });
  });

  it.each(['http://hub.example.org', 'http://192.0.2.10:8080', 'http://localhost.example.net', 'null', 'file://', 'tauri://localhost', ''])(
    'SEC-001: a browser page served from %j is refused with fixed text and no hub',
    (origin) => {
      const start = browserAt(origin);
      expect(start).toEqual({ kind: 'refused', message: ORIGIN_REFUSED });
      if (origin !== '') expect(ORIGIN_REFUSED).not.toContain(origin);
    },
  );

  it('CLI-002: the desktop app still talks to its stored hub, whatever the page origin', () => {
    const stored = clientStart({ desktop: true, storage: storage('https://my-hub.example.org'), origin: 'tauri://localhost' });
    expect(addressOf(stored).origin).toBe('https://my-hub.example.org');
    const unset = clientStart({ desktop: true, storage: storage(), origin: 'http://tauri.localhost' });
    expect(addressOf(unset)).toEqual(DEFAULT_HUB_ADDRESS);
  });
});

describe('a browser start sends no token: the cookie travels by itself', () => {
  beforeEach(() => {
    FakeSocket.opened = [];
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it('CLI-003: outside the desktop app the token provider answers null for the served address', async () => {
    expect(await readAccessToken(addressOf(browserAt('https://hub.example.org')))).toBeNull();
  });

  it('CLI-003: me goes to the served origin with no Authorization header, and the socket sends an empty connection_init', async () => {
    const sent: { url: string; headers: Record<string, string> }[] = [];
    const fetch = (url: string, init: RequestInit) => {
      sent.push({ url, headers: init.headers as Record<string, string> });
      return Promise.resolve(new Response('{"data":{"me":{"username":"ann"}}}'));
    };
    const address = addressOf(browserAt('https://hub.example.org'));
    const hub = createHub({ address, token: readAccessToken, fetch: fetch as typeof globalThis.fetch, WebSocket: FAKE_WEBSOCKET });
    expect(await accessApi(hub).me()).toEqual({ ok: true, value: { username: 'ann' } });
    hub.subscribe(STATUS_CHANGED, { deviceId: 'd1' }, recordingSink().sink);
    await settle();
    const [socket] = FakeSocket.opened;
    socket?.accept();
    await settle();
    expect(sent).toHaveLength(1);
    expect(sent[0]?.url).toBe('https://hub.example.org/graphql');
    expect(Object.keys(sent[0]?.headers ?? {}).map((name) => name.toLowerCase())).not.toContain('authorization');
    expect(socket?.url).toBe('wss://hub.example.org/graphql');
    expect(socket?.sent[0]).toEqual({ type: 'connection_init', payload: {} });
    void hub.close();
  });
});
