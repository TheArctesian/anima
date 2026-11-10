import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { parse } from 'graphql';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createHub, createRequestHub, DEFAULT_HUB_ADDRESS, TOKEN_UNREADABLE, type Hub, type TokenProvider } from './hub';
import { acknowledge, FAKE_WEBSOCKET, FakeSocket } from './socket/fake';

const ME = parse('query Me($first: Int) { me { username } }') as TypedDocumentNode<
  { me: { username: string } },
  { first?: number }
>;
const CONTACT_AT = new Date('2026-11-04T10:00:00Z');
/** The `createHub` timeout when none is given. */
const DEFAULT_TIMEOUT_MS = 10_000;

interface Sent {
  readonly url: string;
  readonly headers: Record<string, string>;
  readonly body: unknown;
}

function hubAnswering(answer: (signal?: AbortSignal | null) => Promise<Response>, token: TokenProvider = () => Promise.resolve('s3cret-token')) {
  const sent: Sent[] = [];
  const fakeFetch: typeof fetch = (input, init) => {
    const headers = init?.headers;
    const body = init?.body;
    sent.push({
      url: String(input),
      headers: headers !== undefined && !Array.isArray(headers) && !(headers instanceof Headers) ? headers : {},
      body: typeof body === 'string' ? JSON.parse(body) : undefined,
    });
    return answer(init?.signal);
  };
  const hub: Hub = createHub({ address: DEFAULT_HUB_ADDRESS, token, fetch: fakeFetch, WebSocket: FAKE_WEBSOCKET, now: () => CONTACT_AT });
  hubs.push(hub);
  return { hub, sent };
}

/** Every hub a test made; each is closed after the test, so no socket or timer outlives it. */
const hubs: Hub[] = [];

beforeEach(() => {
  FakeSocket.opened = [];
});

afterEach(async () => {
  await Promise.all(hubs.splice(0).map((hub) => hub.close()));
});

/** An answer that never arrives; it rejects like fetch does when the request is aborted. */
function stalled(signal?: AbortSignal | null): Promise<Response> {
  return new Promise((_resolve, reject) => {
    signal?.addEventListener('abort', () => reject(new DOMException('The operation was aborted.', 'AbortError')));
  });
}

function graphQLAnswer(body: unknown, status = 200): () => Promise<Response> {
  return () => Promise.resolve(new Response(JSON.stringify(body), { status }));
}

describe('Hub.request', () => {
  it('CQ-045: posts the document and variables to /graphql with the bearer token', async () => {
    const { hub, sent } = hubAnswering(graphQLAnswer({ data: { me: { username: 'op' } } }));
    const result = await hub.request(ME, { first: 3 });
    expect(result).toEqual({ ok: true, value: { me: { username: 'op' } } });
    expect(sent).toHaveLength(1);
    expect(sent[0]?.url).toBe('https://hub.anima.ing/graphql');
    expect(sent[0]?.headers.authorization).toBe('Bearer s3cret-token');
    expect(sent[0]?.body).toEqual({ query: expect.stringContaining('me {') as unknown, variables: { first: 3 } });
  });

  it('API-003: sends no authorization header before login', async () => {
    const { hub, sent } = hubAnswering(graphQLAnswer({ data: { me: { username: 'op' } } }), () => Promise.resolve(null));
    await hub.request(ME, {});
    expect(sent[0]?.headers).not.toHaveProperty('authorization');
  });

  it('NET-004: a network failure is unreachable and the status follows', async () => {
    const { hub } = hubAnswering(() => Promise.reject(new TypeError('Failed to fetch')));
    const result = await hub.request(ME, {});
    expect(result.ok ? undefined : result.error.code).toBe('unreachable');
    expect(hub.status.current).toEqual({ connection: 'unreachable', lastContactAt: null });
  });

  it('NET-004: a refused token still counts as contact', async () => {
    const refused = { errors: [{ message: 'Log in again.', extensions: { code: 'UNAUTHENTICATED' } }] };
    const { hub } = hubAnswering(graphQLAnswer(refused, 401));
    const result = await hub.request(ME, {});
    expect(result).toEqual({ ok: false, error: { code: 'UNAUTHENTICATED', message: 'Log in again.' } });
    expect(hub.status.current).toEqual({ connection: 'connected', lastContactAt: CONTACT_AT });
  });

  it('an answer that is not GraphQL is malformed and is not contact', async () => {
    const { hub } = hubAnswering(() => Promise.resolve(new Response('<html>Bad Gateway</html>', { status: 502 })));
    const result = await hub.request(ME, {});
    expect(result.ok ? undefined : result.error.code).toBe('malformed');
    expect(hub.status.current.lastContactAt).toBeNull();
  });

  it('LOG-007: no error result holds the token', async () => {
    const answers = [
      () => Promise.reject(new TypeError('Failed to fetch')),
      () => Promise.resolve(new Response('garbled{')),
      graphQLAnswer({ errors: [{ message: 'No.', extensions: { code: 'FORBIDDEN' } }] }),
    ];
    for (const answer of answers) {
      const { hub } = hubAnswering(answer);
      expect(JSON.stringify(await hub.request(ME, {}))).not.toContain('s3cret-token');
    }
  });
});

describe('Hub.request on a stalled link', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('NET-004: no answer within the timeout is unreachable and the status follows', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const { hub } = hubAnswering(stalled);
    const [socket] = FakeSocket.opened;
    if (socket === undefined) throw new Error('no socket was opened');
    await acknowledge(socket);
    expect(hub.status.current.connection).toBe('connected');
    let settled = false;
    const pending = hub.request(ME, {}).then((result) => {
      settled = true;
      return result;
    });
    await vi.advanceTimersByTimeAsync(DEFAULT_TIMEOUT_MS - 1);
    expect(settled).toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    const result = await pending;
    expect(result.ok ? undefined : result.error.code).toBe('unreachable');
    expect(hub.status.current.connection).toBe('unreachable');
  });
});

describe('Hub.request with an unreadable token', () => {
  it('LOG-007: a rejecting token provider gives UNAUTHENTICATED without its reason and sends nothing', async () => {
    const { hub, sent } = hubAnswering(graphQLAnswer({ data: {} }), () => Promise.reject(new Error('keychain item s3cret-token locked')));
    const result = await hub.request(ME, {});
    expect(result).toEqual({ ok: false, error: { code: 'UNAUTHENTICATED', message: 'The access token could not be read.' } });
    expect(result.ok ? undefined : result.error).toBe(TOKEN_UNREADABLE);
    expect(JSON.stringify(result)).not.toContain('s3cret-token');
    expect(sent).toEqual([]);
    expect(hub.status.current.connection).toBe('connecting');
  });
});

describe('createRequestHub', () => {
  it('BLD-022, D-1: sends requests with its own token and opens no socket', async () => {
    const sent: Sent[] = [];
    const fakeFetch: typeof fetch = (input, init) => {
      const headers = init?.headers;
      sent.push({ url: String(input), headers: headers !== undefined && !Array.isArray(headers) && !(headers instanceof Headers) ? headers : {}, body: undefined });
      return Promise.resolve(new Response(JSON.stringify({ data: { me: { username: 'builder' } } })));
    };
    const hub = createRequestHub({ address: DEFAULT_HUB_ADDRESS, token: () => Promise.resolve('builder-token'), fetch: fakeFetch });
    await expect(hub.request(ME, {})).resolves.toEqual({ ok: true, value: { me: { username: 'builder' } } });
    expect(sent).toHaveLength(1);
    expect(sent[0]?.headers.authorization).toBe('Bearer builder-token');
    expect(FakeSocket.opened).toHaveLength(0);
  });
});
