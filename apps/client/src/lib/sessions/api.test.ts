import { afterEach, describe, expect, it } from 'vitest';
import { createHub, DEFAULT_HUB_ADDRESS, type Hub } from '$lib/hub/hub';
import { FAKE_WEBSOCKET } from '$lib/hub/socket/fake';
import { sessionsApi } from './api';
import { CERTIFICATE, fakeHub } from './fake/hub.fixture';

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
    expect(await api.startSession('d-1', 'SSH', DISPLAY)).toEqual({ ok: true, value: TICKET });
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
    expect(await api.startSession('d-1', 'SSH', DISPLAY)).toEqual({ ok: false, error: { code: 'CONFLICT', message: 'No host key is accepted yet.' } });
  });
});

/** A fake hub with the Phase 2 additions, closed after the test. */
function k1Hub(...changes: Parameters<typeof fakeHub>) {
  const fake = fakeHub(...changes);
  hubs.push(fake.hub);
  return fake;
}

describe('sessionsApi, K1 starts and policy', () => {
  it.each(['RDP', 'VNC'] as const)('K1: startSession asks for %s and the ticket names the Guacamole tunnel', async (protocol) => {
    const { api, named } = k1Hub({ certificateExempt: true });
    const ticket = await api.startSession('device-1', protocol, DISPLAY);
    expect(ticket).toMatchObject({ ok: true, value: { sessionId: 'session-1', tunnelPath: '/tunnel' } });
    expect(named('StartSession')[0]?.variables).toEqual({ input: { deviceId: 'device-1', protocol, display: DISPLAY } });
  });

  it('K4 §1: an RDP start without an accepted certificate is CONFLICT', async () => {
    const { api } = k1Hub();
    expect(await api.startSession('device-1', 'RDP', DISPLAY)).toMatchObject({ ok: false, error: { code: 'CONFLICT' } });
  });

  it('RA-033: transferPolicy reads the device\'s copy and paste flags', async () => {
    const { api, named } = k1Hub({ transfer: { copy: false, paste: true } });
    expect(await api.transferPolicy('device-1')).toEqual({ ok: true, value: { copy: false, paste: true } });
    expect(named('DeviceTransfer')[0]?.query).toContain('device(id: $id)');
  });

  it('RA-033: transferPolicy of a device that is gone is NOT_FOUND', async () => {
    const { api } = k1Hub();
    expect(await api.transferPolicy('device-9')).toMatchObject({ ok: false, error: { code: 'NOT_FOUND' } });
  });

});

describe('sessionsApi, K1 credentials and certificates', () => {
  it.each([
    ['RDP', 'SetRdpCredential', 'setRdpCredential'],
    ['VNC', 'SetVncCredential', 'setVncCredential'],
  ] as const)('GW-015: setCredential for %s sends the password to %s only', async (protocol, operation, field) => {
    const { api, named, sent } = k1Hub();
    expect(await api.setCredential(protocol, 'device-1', 'pw-marker-91')).toEqual({ ok: true, value: { credentialStored: true } });
    expect(sent).toHaveLength(1);
    expect(named(operation)[0]?.query).toContain(`${field}(deviceId: $deviceId, credential: $credential)`);
    expect(named(operation)[0]?.variables).toEqual({ deviceId: 'device-1', credential: { password: 'pw-marker-91' } });
  });

  it('LOG-007: a refused credential keeps the code but never the hub\'s text, which may quote the password', async () => {
    const { api } = k1Hub({ refuseCredential: true });
    const result = await api.setCredential('RDP', 'device-1', 'pw-marker-91');
    expect(result).toMatchObject({ ok: false, error: { code: 'VALIDATION', fields: [{ field: 'password' }] } });
    expect(JSON.stringify(result)).not.toContain('pw-marker-91');
  });

  it('LOG-007: any other refused credential also has fixed text, with its code', async () => {
    const { api } = k1Hub({ refuseCredential: true, refusalCode: 'CONFLICT' });
    const result = await api.setCredential('VNC', 'device-1', 'pw-marker-91');
    expect(result).toEqual({ ok: false, error: { code: 'CONFLICT', message: 'The hub did not store the password.' } });
  });

  it.each([
    ['a K1 password', { deviceId: 'device-1', protocol: 'RDP' as const, password: { password: 'pw-marker-91' } }],
    ['an SSH credential', { host: 'nas.lan', credential: { password: 'pw-marker-91' } }],
  ])('LOG-007: a refused connection test with %s keeps the code but never the hub\'s text', async (_case, input) => {
    const { api } = k1Hub({ refuseCredential: true });
    const result = await api.testConnection(input);
    expect(result).toMatchObject({ ok: false, error: { code: 'VALIDATION', message: 'The hub could not run the connection test.' } });
    expect(JSON.stringify(result)).not.toContain('pw-marker-91');
  });

  it('K4 §3: acceptServerCertificate sends exactly the fingerprint; a different one is CONFLICT', async () => {
    const { api, named } = k1Hub();
    expect(await api.acceptServerCertificate('device-1', 'sha256:other')).toMatchObject({ ok: false, error: { code: 'CONFLICT' } });
    expect(await api.acceptServerCertificate('device-1', CERTIFICATE.fingerprintSha256)).toEqual({ ok: true, value: CERTIFICATE });
    expect(named('AcceptServerCertificate')[1]?.variables).toEqual({ deviceId: 'device-1', fingerprintSha256: CERTIFICATE.fingerprintSha256 });
  });

  it('K4 §3: an RDP connection test returns the server certificate and whether it changed', async () => {
    const { api, named, device } = k1Hub({ acceptedFingerprint: 'sha256:old' });
    const result = await api.testConnection({ deviceId: 'device-1', protocol: 'RDP' });
    expect(result).toMatchObject({ ok: true, value: { serverCertificate: CERTIFICATE, certificateChanged: true } });
    expect(named('TestRemoteConnection')[0]?.variables).toEqual({ input: { deviceId: 'device-1', protocol: 'RDP' } });
    device.acceptedFingerprint = CERTIFICATE.fingerprintSha256;
    expect(await api.testConnection({ deviceId: 'device-1', protocol: 'RDP' })).toMatchObject({ ok: true, value: { certificateChanged: false } });
  });

  it('K1: a test without a K1 field is sent as in Phase 1, so a hub without K1 answers it', async () => {
    const { api, named } = k1Hub();
    await api.testConnection({ deviceId: 'device-1' });
    expect(named('TestConnection')).toHaveLength(1);
    expect(named('TestRemoteConnection')).toHaveLength(0);
  });
});
