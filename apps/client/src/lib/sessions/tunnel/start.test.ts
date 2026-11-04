// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import type { EndedSession, SessionDisplay, SessionsApi, SessionTicket } from '../api';
import { installBlankCanvas } from './fake/canvas.fixture';
import { FAKE_TUNNEL_SOCKET, FakeTunnelSocket } from './fake/socket.fixture';
import { startTunnel, type StartedTunnel } from './start';

const DISPLAY: SessionDisplay = { widthPx: 1280, heightPx: 800, dpi: 192, timezone: 'Europe/London' };
const TARGET = { deviceId: 'device-1', display: DISPLAY };
const SETUP = { origin: 'https://hub.anima.ing', WebSocket: FAKE_TUNNEL_SOCKET };

/** A sessions API that issues a new ticket per call and records every call. */
function fakeApi(tunnelPath = '/tunnel') {
  const calls: { startSession: [string, SessionDisplay][]; endSession: string[] } = { startSession: [], endSession: [] };
  let issued = 0;
  const api: Pick<SessionsApi, 'startSession' | 'endSession'> = {
    startSession: (deviceId, display) => {
      calls.startSession.push([deviceId, display]);
      issued += 1;
      const ticket: SessionTicket = {
        sessionId: `session-${String(issued)}`,
        token: `token-${String(issued)}-${'x'.repeat(34)}`,
        tunnelPath,
        expiresAt: '2026-11-04T10:00:30Z',
      };
      return Promise.resolve({ ok: true, value: ticket });
    },
    endSession: (id) => {
      calls.endSession.push(id);
      return Promise.resolve({ ok: false, error: { code: 'NOT_FOUND', message: 'No such session.' } } as HubResult<EndedSession>);
    },
  };
  return { api, calls };
}

const started: StartedTunnel[] = [];
let restoreCanvas: () => void = () => undefined;

beforeEach(() => {
  vi.useFakeTimers();
  FakeTunnelSocket.reset();
  restoreCanvas = installBlankCanvas();
});

afterEach(() => {
  for (const { tunnel } of started.splice(0)) tunnel.close();
  restoreCanvas();
  vi.useRealTimers();
});

async function start(api: Pick<SessionsApi, 'startSession' | 'endSession'>) {
  const result = await startTunnel(api, TARGET, SETUP);
  if (result.ok) started.push(result.value);
  return result;
}

describe('start and reconnect (D-3, D-5, RA-007)', () => {
  it('RA-007: a start calls startSession once for the device and display and opens the ticket', async () => {
    const { api, calls } = fakeApi();
    const result = await start(api);
    expect(calls.startSession).toEqual([['device-1', DISPLAY]]);
    expect(result.ok && result.value.sessionId).toBe('session-1');
    expect(FakeTunnelSocket.latest().url).toBe(`wss://hub.anima.ing/tunnel?token=token-1-${'x'.repeat(34)}`);
  });

  it('RA-007: Reconnect calls startSession once more and opens a new tunnel with a new token', async () => {
    const { api, calls } = fakeApi();
    const first = await start(api);
    if (first.ok) {
      FakeTunnelSocket.latest().open();
      FakeTunnelSocket.latest().closeWith(1000);
    }
    await start(api);
    expect(calls.startSession).toHaveLength(2);
    const [old, fresh] = FakeTunnelSocket.made.map((socket) => new URL(socket.url).searchParams.get('token'));
    expect(FakeTunnelSocket.made).toHaveLength(2);
    expect(fresh).not.toBe(old);
  });

  it('API-009: the started session holds the session ID and tunnel, never the ticket', async () => {
    const { api } = fakeApi();
    const result = await start(api);
    expect(result.ok && Object.keys(result.value).sort()).toEqual(['sessionId', 'tunnel']);
    expect(JSON.stringify(result.ok && result.value.tunnel.state())).not.toContain('token-1');
  });

  it('K7: a hub refusal stays the typed hub error and opens nothing', async () => {
    const { api } = fakeApi();
    const conflict = { code: 'CONFLICT', message: 'No host key is accepted yet.' } as const;
    api.startSession = () => Promise.resolve({ ok: false, error: conflict });
    expect(await start(api)).toEqual({ ok: false, error: conflict });
    expect(FakeTunnelSocket.made).toEqual([]);
  });

  it('SEC-002: a ticket path the client refuses ends the new session and reports the refusal', async () => {
    const { api, calls } = fakeApi('//evil.example/tunnel');
    const result = await start(api);
    expect(result).toEqual({ ok: false, error: { problem: 'bad-address', message: 'The hub gave a tunnel address that Anima does not accept.' } });
    expect(calls.endSession).toEqual(['session-1']);
  });
});
