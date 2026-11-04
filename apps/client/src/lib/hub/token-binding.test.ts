import { parse } from 'graphql';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createHub, DEFAULT_HUB_ADDRESS, parseHubAddress, type HubAddress, type TokenProvider } from './hub';
import { FAKE_WEBSOCKET, FakeSocket, recordingSink, settle, STATUS_CHANGED } from './socket/fake';

const ME = parse('query { me { username } }') as never;

function addressOf(text: string): HubAddress {
  const parsed = parseHubAddress(text);
  if (!parsed.ok) throw new Error('test address refused');
  return parsed.value;
}

/** A key store holding the default hub's token only, looked up by the address the hub passes. */
function keyStore(): { read: TokenProvider; reads: HubAddress[] } {
  const reads: HubAddress[] = [];
  const read: TokenProvider = (address) => {
    reads.push(address);
    return Promise.resolve(address.origin === DEFAULT_HUB_ADDRESS.origin ? 'real-token' : null);
  };
  return { read, reads };
}

function answering(): { fetch: typeof fetch; sent: { url: string; authorization: string | undefined }[] } {
  const sent: { url: string; authorization: string | undefined }[] = [];
  const fakeFetch = (url: string, init: RequestInit) => {
    sent.push({ url, authorization: (init.headers as Record<string, string>).authorization });
    return Promise.resolve(new Response('{"data":{}}'));
  };
  return { fetch: fakeFetch as unknown as typeof fetch, sent };
}

beforeEach(() => {
  FakeSocket.opened = [];
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
});
afterEach(() => {
  vi.useRealTimers();
});

describe('a hub reads the token for its own address only', () => {
  it('CLI-003: a request and the socket each hear the hub address, so another hub gets no token', async () => {
    const evil = addressOf('evil.example');
    const { read, reads } = keyStore();
    const { fetch, sent } = answering();
    const hub = createHub({ address: evil, token: read, fetch, WebSocket: FAKE_WEBSOCKET });
    await hub.request(ME, {});
    hub.subscribe(STATUS_CHANGED, { deviceId: 'd1' }, recordingSink().sink);
    await settle();
    FakeSocket.opened[0]?.accept();
    await settle();
    expect(reads).toHaveLength(2);
    expect(reads).toEqual([evil, evil]);
    expect(sent).toEqual([{ url: 'https://evil.example/graphql', authorization: undefined }]);
    expect(FakeSocket.opened[0]?.url).toBe('wss://evil.example/graphql');
    void hub.close();
  });

  it('CLI-002: changing the caller object after createHub changes neither the endpoints (HTTP and socket) nor the address the provider hears', async () => {
    const mine = { ...addressOf('evil.example') };
    const { read, reads } = keyStore();
    const { fetch, sent } = answering();
    const hub = createHub({ address: mine, token: read, fetch, WebSocket: FAKE_WEBSOCKET });
    Object.assign(mine, DEFAULT_HUB_ADDRESS);
    await hub.request(ME, {});
    hub.subscribe(STATUS_CHANGED, { deviceId: 'd1' }, recordingSink().sink);
    await settle();
    FakeSocket.opened[0]?.accept();
    await settle();
    expect(reads).toEqual([addressOf('evil.example'), addressOf('evil.example')]);
    expect(reads.every((read) => Object.isFrozen(read))).toBe(true);
    expect(sent).toEqual([{ url: 'https://evil.example/graphql', authorization: undefined }]);
    expect(FakeSocket.opened[0]?.url).toBe('wss://evil.example/graphql');
    void hub.close();
  });
});
