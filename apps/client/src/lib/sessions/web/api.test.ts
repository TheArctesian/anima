import { afterEach, describe, expect, it } from 'vitest';
import { createHub, DEFAULT_HUB_ADDRESS, type Hub } from '$lib/hub/hub';
import { FAKE_WEBSOCKET } from '$lib/hub/socket/fake';
import { webTabApi } from './api';

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
  return { api: webTabApi(hub), sent };
}

const PIN = { id: 'web-1', pinnedFingerprintSha256: 'ab'.repeat(32), pinnedAt: '2026-11-06T10:00:00Z' };

describe('webTabApi (K1, CQ-045)', () => {
  it('WEB-008: acceptWebCertificate sends the interface and the fingerprint and returns the pin', async () => {
    const { api, sent } = hubAnswering({ data: { acceptWebCertificate: PIN } });
    expect(await api.acceptWebCertificate('web-1', 'ab'.repeat(32))).toEqual({ ok: true, value: PIN });
    expect(sent[0]?.query).toContain('acceptWebCertificate(id: $id, fingerprintSha256: $fingerprintSha256)');
    expect(sent[0]?.variables).toEqual({ id: 'web-1', fingerprintSha256: 'ab'.repeat(32) });
  });

  it('K7: a refusal stays the hub error', async () => {
    const { api } = hubAnswering({ errors: [{ message: 'Only https interfaces take a pin.', extensions: { code: 'VALIDATION', fields: [{ field: 'fingerprintSha256', message: 'x' }] } }] });
    const result = await api.acceptWebCertificate('web-1', 'ab'.repeat(32));
    expect(result).toMatchObject({ ok: false, error: { code: 'VALIDATION', fields: [{ field: 'fingerprintSha256' }] } });
  });
});
