import { afterEach, describe, expect, it } from 'vitest';
import { createHub, DEFAULT_HUB_ADDRESS, type Hub } from '$lib/hub/hub';
import { FAKE_WEBSOCKET } from '$lib/hub/socket/fake';
import { wakeApi } from './api';

const hubs: Hub[] = [];

afterEach(async () => {
  await Promise.all(hubs.splice(0).map((hub) => hub.close()));
});

function hubAnswering(body: unknown) {
  const sent: { query: string; variables: Record<string, unknown> }[] = [];
  const fakeFetch: typeof fetch = (_input, init) => {
    sent.push(JSON.parse(String(init?.body)) as (typeof sent)[number]);
    return Promise.resolve(new Response(JSON.stringify(body)));
  };
  const hub = createHub({ address: DEFAULT_HUB_ADDRESS, token: () => Promise.resolve('operator-token'), fetch: fakeFetch, WebSocket: FAKE_WEBSOCKET });
  hubs.push(hub);
  return { api: wakeApi(hub), sent };
}

describe('wakeApi (RA-031)', () => {
  it('RA-031: wakeDevice sends the device and returns when and through which relay', async () => {
    const value = { sentAt: '2026-11-06T10:00:00Z', relayName: 'Pi' };
    const { api, sent } = hubAnswering({ data: { wakeDevice: value } });
    expect(await api.wakeDevice('device-1')).toEqual({ ok: true, value });
    expect(sent[0]?.variables).toEqual({ deviceId: 'device-1' });
  });

  it('K7: a refusal stays the hub error', async () => {
    const { api } = hubAnswering({ errors: [{ message: 'no relay', extensions: { code: 'CONFLICT' } }] });
    expect(await api.wakeDevice('device-1')).toMatchObject({ ok: false, error: { code: 'CONFLICT' } });
  });
});
