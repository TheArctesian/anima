import { afterEach, describe, expect, it } from 'vitest';
import { createHub, DEFAULT_HUB_ADDRESS, type Hub } from '$lib/hub/hub';
import { FAKE_WEBSOCKET } from '$lib/hub/socket/fake';
import { filesApi } from './api';

interface Sent {
  readonly query: string;
  readonly variables: Record<string, unknown>;
}

const hubs: Hub[] = [];

afterEach(async () => {
  await Promise.all(hubs.splice(0).map((hub) => hub.close()));
});

/** A hub whose HTTP endpoint answers every request with `body`, recording what was sent. */
function hubAnswering(body: unknown) {
  const sent: Sent[] = [];
  const fakeFetch: typeof fetch = (_input, init) => {
    sent.push(JSON.parse(String(init?.body)) as Sent);
    return Promise.resolve(new Response(JSON.stringify(body)));
  };
  const hub = createHub({ address: DEFAULT_HUB_ADDRESS, token: () => Promise.resolve('operator-token'), fetch: fakeFetch, WebSocket: FAKE_WEBSOCKET });
  hubs.push(hub);
  return { api: filesApi(hub), sent };
}

const TICKET = { sessionId: 'session-1', token: 'files-token', tunnelPath: '/files', expiresAt: '2026-11-06T10:00:30Z' };

describe('filesApi (K1, CQ-045)', () => {
  it('RA-032: openFiles sends the session and returns the ticket, without asking for wakeSent', async () => {
    const { api, sent } = hubAnswering({ data: { openFiles: TICKET } });
    expect(await api.openFiles('session-1')).toEqual({ ok: true, value: TICKET });
    expect(sent[0]?.query).toContain('openFiles(sessionId: $sessionId)');
    expect(sent[0]?.query).not.toContain('wakeSent');
    expect(sent[0]?.variables).toEqual({ sessionId: 'session-1' });
  });

  it('K7: a refusal stays the hub error', async () => {
    const { api } = hubAnswering({ errors: [{ message: 'The session is not a live SSH session.', extensions: { code: 'CONFLICT' } }] });
    expect(await api.openFiles('session-1')).toMatchObject({ ok: false, error: { code: 'CONFLICT' } });
  });
});
