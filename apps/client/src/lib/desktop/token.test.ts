import { parse } from 'graphql';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createHub, DEFAULT_HUB_ADDRESS, parseHubAddress, type HubAddress } from '$lib/hub/hub';
import { FakeSocket, newHub, onlySocket, recordingSink, settle, STATUS_CHANGED } from '$lib/hub/socket/fake';
import { storedHubAddress, storeHubAddress } from './address';
import { AccessTokenError, clearAccessToken, readAccessToken, writeAccessToken } from './token';

const core = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => core);

const SECRET = 's3cret-token';
const HUB = DEFAULT_HUB_ADDRESS;

function addressOf(host: string): HubAddress {
  const parsed = parseHubAddress(host);
  if (!parsed.ok) throw new Error('bad test address');
  return parsed.value;
}

/** A key store that keeps one token per origin. */
function keyStoreHolding(entries: [string, string][]): void {
  const stored = new Map(entries);
  core.invoke.mockImplementation((_command: string, args: { origin: string }) => Promise.resolve(stored.get(args.origin) ?? null));
}

beforeEach(() => {
  core.invoke.mockReset();
  core.isTauri.mockReset().mockReturnValue(true);
  FakeSocket.opened = [];
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
});

afterEach(() => {
  vi.useRealTimers();
});

describe('readAccessToken', () => {
  it('CLI-003: returns the token the shell holds', async () => {
    core.invoke.mockResolvedValue(SECRET);
    await expect(readAccessToken(HUB)).resolves.toBe(SECRET);
    expect(core.invoke).toHaveBeenCalledWith('access_token_read', { origin: HUB.origin });
  });

  it('CLI-003: returns null when the shell holds no token', async () => {
    core.invoke.mockResolvedValue(null);
    await expect(readAccessToken(HUB)).resolves.toBeNull();
  });

  it('CLI-003: rejects with the shell code when the key store fails', async () => {
    core.invoke.mockRejectedValue({ code: 'denied', message: `refused ${SECRET}` });
    await expect(readAccessToken(HUB)).rejects.toMatchObject({ problem: 'denied', message: 'The key store refused access.' });
  });

  it('CLI-003: an unrecognised failure shape becomes failed', async () => {
    core.invoke.mockRejectedValue('boom');
    await expect(readAccessToken(HUB)).rejects.toMatchObject({ problem: 'failed' });
    core.invoke.mockRejectedValue({ code: 'other' });
    await expect(readAccessToken(HUB)).rejects.toMatchObject({ problem: 'failed' });
    core.invoke.mockResolvedValue(42);
    await expect(readAccessToken(HUB)).rejects.toMatchObject({ problem: 'failed' });
  });

  it('CLI-003: outside the desktop app it returns null and never calls the shell', async () => {
    core.isTauri.mockReturnValue(false);
    await expect(readAccessToken(HUB)).resolves.toBeNull();
    expect(core.invoke).not.toHaveBeenCalled();
  });
});

describe('readAccessToken origin and errors', () => {
  it('LOG-007: the error has no cause and its stack does not hold the token', async () => {
    core.invoke.mockRejectedValue(Object.assign(new Error(`item ${SECRET} locked`), { code: 'failed' }));
    const error = (await readAccessToken(HUB).catch((reason: unknown) => reason)) as Error;
    expect(error.cause).toBeUndefined();
    expect(error.stack).not.toContain(SECRET);
  });

  it('LOG-007: no error text holds the token', async () => {
    core.invoke.mockRejectedValue({ code: 'failed', message: `item ${SECRET} locked` });
    const error: unknown = await readAccessToken(HUB).catch((reason: unknown) => reason);
    expect(error).toBeInstanceOf(AccessTokenError);
    expect(JSON.stringify(error) + (error as Error).message).not.toContain(SECRET);
  });

  it('CLI-002: a token stored for hub A is not read for hub B', async () => {
    const [a, b] = [addressOf('a.example'), addressOf('b.example')];
    keyStoreHolding([[a.origin, SECRET]]);
    await expect(readAccessToken(a)).resolves.toBe(SECRET);
    await expect(readAccessToken(b)).resolves.toBeNull();
    expect(core.invoke).toHaveBeenLastCalledWith('access_token_read', { origin: b.origin });
  });

  it('CLI-002: the shell code origin is a typed problem', async () => {
    core.invoke.mockRejectedValue({ code: 'origin', message: 'bad' });
    await expect(readAccessToken(HUB)).rejects.toMatchObject({ problem: 'origin' });
  });
});

describe('the origin guard', () => {
  const handBuilt = (origin: string): HubAddress => ({ ...HUB, origin });
  const hostileHttp: HubAddress = { ...HUB, http: 'https://evil.example/graphql' };
  const hostileSocket: HubAddress = { ...HUB, socket: 'wss://evil.example/graphql' };

  it.each([
    ['a canonical origin with a hostile http endpoint', hostileHttp],
    ['a canonical origin with a hostile socket endpoint', hostileSocket],
  ])('CLI-003: read, write and clear refuse %s without calling the shell', async (_name, address) => {
    await expect(readAccessToken(address)).rejects.toMatchObject({ problem: 'origin' });
    await expect(writeAccessToken(address, SECRET)).rejects.toMatchObject({ problem: 'origin' });
    await expect(clearAccessToken(address)).rejects.toMatchObject({ problem: 'origin' });
    expect(core.invoke).not.toHaveBeenCalled();
  });

  it.each([
    ['an upper-case host', 'https://HUB.anima.ing'],
    ['a path', 'https://hub.anima.ing/graphql'],
    ['a trailing slash', 'https://hub.anima.ing/'],
    ['http', 'http://hub.anima.ing'],
    ['text that is not a URL', 'not a url'],
    ['the http endpoint', HUB.http],
  ])('CLI-002: read, write and clear refuse %s without calling the shell', async (_name, origin) => {
    const address = handBuilt(origin);
    await expect(readAccessToken(address)).rejects.toMatchObject({ problem: 'origin' });
    await expect(writeAccessToken(address, SECRET)).rejects.toMatchObject({ problem: 'origin' });
    await expect(clearAccessToken(address)).rejects.toMatchObject({ problem: 'origin' });
    expect(core.invoke).not.toHaveBeenCalled();
  });
});

describe('writeAccessToken and clearAccessToken', () => {
  const other = addressOf('my-hub.example:8443');

  it('CLI-003: write passes the origin and the token', async () => {
    core.invoke.mockResolvedValue(undefined);
    await writeAccessToken(other, SECRET);
    expect(core.invoke).toHaveBeenCalledWith('access_token_write', { origin: 'https://my-hub.example:8443', token: SECRET });
  });

  it('CLI-003: clear passes the origin to access_token_clear', async () => {
    core.invoke.mockResolvedValue(undefined);
    await clearAccessToken(other);
    expect(core.invoke).toHaveBeenCalledWith('access_token_clear', { origin: 'https://my-hub.example:8443' });
  });

  it('CLI-003: a refused write rejects with the shell code', async () => {
    core.invoke.mockRejectedValue({ code: 'empty', message: 'The access token is empty.' });
    await expect(writeAccessToken(HUB, '')).rejects.toMatchObject({ problem: 'empty' });
  });

  it('CLI-003: outside the desktop app write and clear reject with not-desktop', async () => {
    core.isTauri.mockReturnValue(false);
    await expect(writeAccessToken(HUB, SECRET)).rejects.toMatchObject({ problem: 'not-desktop' });
    await expect(clearAccessToken(HUB)).rejects.toMatchObject({ problem: 'not-desktop' });
    expect(core.invoke).not.toHaveBeenCalled();
  });
});

describe('createHub with readAccessToken', () => {
  it('CLI-003: a key-store failure ends the subscription as UNAUTHENTICATED "could not be read", with no retry', async () => {
    core.invoke.mockRejectedValue({ code: 'unavailable', message: 'no key store' });
    const hub = newHub(() => readAccessToken(HUB));
    const { events, sink } = recordingSink();
    hub.subscribe(STATUS_CHANGED, {}, sink);
    await settle();
    onlySocket().accept();
    await settle(120_000);
    expect(events.errors).toEqual([{ code: 'UNAUTHENTICATED', message: 'The access token could not be read.' }]);
    expect(FakeSocket.opened).toHaveLength(1);
    expect(core.invoke).toHaveBeenCalledTimes(1);
  });
});

describe('changing the hub address', () => {
  it('CLI-002: after storing hub B, a request to B never carries the token stored for hub A', async () => {
    keyStoreHolding([[HUB.origin, SECRET]]);
    const items = new Map<string, string>();
    const storage = {
      getItem: (key: string) => items.get(key) ?? null,
      setItem: (key: string, value: string) => void items.set(key, value),
    } as Storage;
    expect(storeHubAddress(storage, 'hub-anima.example').ok).toBe(true);
    const address = storedHubAddress(storage);
    const sent: { url: string; authorization: unknown }[] = [];
    const fakeFetch: typeof fetch = (input, init) => {
      sent.push({ url: String(input), authorization: (init?.headers as Record<string, string>).authorization });
      return Promise.resolve(new Response(JSON.stringify({ data: {} }), { headers: { 'content-type': 'application/json' } }));
    };
    const hub = createHub({ address, token: readAccessToken, fetch: fakeFetch });
    await hub.request(parse('query Me { me { username } }') as never, {});
    expect(sent.map((request) => request.url)).toEqual(['https://hub-anima.example/graphql']);
    expect(sent[0]?.authorization).toBeUndefined();
    expect(JSON.stringify(sent)).not.toContain(SECRET);
    void hub.close();
  });
});
