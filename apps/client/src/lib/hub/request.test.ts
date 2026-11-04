import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { parse } from 'graphql';
import { describe, expect, it } from 'vitest';
import { createHub, DEFAULT_HUB_ADDRESS, type Hub, type TokenProvider } from './hub';

const ME = parse('query Me($first: Int) { me { username } }') as TypedDocumentNode<
  { me: { username: string } },
  { first?: number }
>;
const CONTACT_AT = new Date('2026-11-04T10:00:00Z');

interface Sent {
  readonly url: string;
  readonly headers: Record<string, string>;
  readonly body: unknown;
}

function hubAnswering(answer: () => Promise<Response>, token: TokenProvider = () => Promise.resolve('s3cret-token')) {
  const sent: Sent[] = [];
  const fakeFetch: typeof fetch = (input, init) => {
    const headers = init?.headers;
    const body = init?.body;
    sent.push({
      url: String(input),
      headers: headers !== undefined && !Array.isArray(headers) && !(headers instanceof Headers) ? headers : {},
      body: typeof body === 'string' ? JSON.parse(body) : undefined,
    });
    return answer();
  };
  const hub: Hub = createHub({ address: DEFAULT_HUB_ADDRESS, token, fetch: fakeFetch, now: () => CONTACT_AT });
  return { hub, sent };
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
