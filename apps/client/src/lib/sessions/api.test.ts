import { afterEach, describe, expect, it } from 'vitest';
import { createHub, DEFAULT_HUB_ADDRESS, type Hub } from '$lib/hub/hub';
import { FAKE_WEBSOCKET } from '$lib/hub/socket/fake';
import { sessionsApi } from './api';

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
  return { api: sessionsApi(hub), sent };
}

const DISPLAY = { widthPx: 1280, heightPx: 800, dpi: 192, timezone: 'Europe/London' };
const TICKET = { sessionId: 's-1', token: 't'.repeat(43), tunnelPath: '/tunnel', expiresAt: '2026-11-04T10:00:30Z' };

describe('sessionsApi (K1, CQ-045)', () => {
  it('K1: startSession asks for SSH on the device with the display and returns the ticket', async () => {
    const { api, sent } = hubAnswering({ data: { startSession: TICKET } });
    expect(await api.startSession('d-1', DISPLAY)).toEqual({ ok: true, value: TICKET });
    expect(sent[0]?.query).toContain('startSession(input: $input)');
    expect(sent[0]?.variables).toEqual({ input: { deviceId: 'd-1', protocol: 'SSH', display: DISPLAY } });
  });

  it('K1: endSession returns the session with its recorded endReason', async () => {
    const session = {
      id: 's-1', deviceId: 'd-1', deviceName: 'nas', protocol: 'SSH',
      startedAt: '2026-11-04T10:00:00Z', endedAt: '2026-11-04T10:05:00Z', endReason: 'HOST_KEY_MISMATCH',
    };
    const { api, sent } = hubAnswering({ data: { endSession: session } });
    expect(await api.endSession('s-1')).toEqual({ ok: true, value: session });
    expect(sent[0]?.variables).toEqual({ id: 's-1' });
  });

  it('K1: testConnection tests the saved device by ID only', async () => {
    const test = { reachable: true, authenticated: true, hostKey: null, hostKeyChanged: false, message: null };
    const { api, sent } = hubAnswering({ data: { testConnection: test } });
    expect(await api.testConnection({ deviceId: 'd-1' })).toEqual({ ok: true, value: test });
    expect(sent[0]?.query).toContain('testConnection(input: $input)');
    expect(sent[0]?.variables).toEqual({ input: { deviceId: 'd-1' } });
  });

  it('DEV-013: testConnection sends unsaved host, SSH settings and credential without a device ID', async () => {
    const test = { reachable: true, authenticated: false, hostKey: null, hostKeyChanged: false, message: 'Bad password' };
    const { api, sent } = hubAnswering({ data: { testConnection: test } });
    const input = {
      host: 'nas.lan',
      ssh: { enabled: true, port: 2222, username: 'root', auth: 'PASSWORD' as const },
      credential: { password: 'hunter2' },
    };
    expect(await api.testConnection(input)).toEqual({ ok: true, value: test });
    expect(sent[0]?.variables).toEqual({ input });
  });

  it('K1: acceptHostKey sends exactly the fingerprint given', async () => {
    const key = { algorithm: 'ssh-ed25519', fingerprintSha256: 'SHA256:abc', firstSeenAt: '2026-11-04T10:00:00Z' };
    const { api, sent } = hubAnswering({ data: { acceptHostKey: key } });
    expect(await api.acceptHostKey('d-1', 'SHA256:abc')).toEqual({ ok: true, value: key });
    expect(sent[0]?.variables).toEqual({ deviceId: 'd-1', fingerprintSha256: 'SHA256:abc' });
  });

  it('Section 6.7: history asks for one page of the device\'s sessions and returns it', async () => {
    const page = {
      pageInfo: { hasNextPage: true, endCursor: 'c-50' },
      edges: [{ node: { id: 's-1', protocol: 'SSH', operatorName: 'ada', startedAt: '2026-11-04T10:00:00Z', endedAt: null, endReason: null } }],
    };
    const { api, sent } = hubAnswering({ data: { sessions: page } });
    expect(await api.history('d-1', 50, 'c-0')).toEqual({ ok: true, value: page });
    expect(sent[0]?.query).toContain('sessions(deviceId: $deviceId, first: $first, after: $after)');
    expect(sent[0]?.variables).toEqual({ deviceId: 'd-1', first: 50, after: 'c-0' });
  });

  it('K7: a hub error stays typed', async () => {
    const { api } = hubAnswering({ errors: [{ message: 'No host key is accepted yet.', extensions: { code: 'CONFLICT' } }] });
    expect(await api.startSession('d-1', DISPLAY)).toEqual({ ok: false, error: { code: 'CONFLICT', message: 'No host key is accepted yet.' } });
  });
});
