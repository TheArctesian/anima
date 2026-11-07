import { beforeEach, describe, expect, it } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import type { EndedSession, SessionDisplay, SessionsApi, SessionTicket } from '../../api';
import { startTerminal } from '../start';
import { FAKE_TERMINAL_SOCKET, FakeTerminalSocket } from './fake/socket.fixture';

const DISPLAY: SessionDisplay = { widthPx: 1280, heightPx: 800, dpi: 96, timezone: 'Europe/London' };
const TARGET = { deviceId: 'device-1', display: DISPLAY };
const SETUP = { origin: 'https://hub.anima.ing', WebSocket: FAKE_TERMINAL_SOCKET, size: () => ({ cols: 80, rows: 24 }), onbytes: () => undefined };

/** A sessions API that issues a new ticket per call and records every call. */
function fakeApi(tunnelPath = '/terminal') {
  const calls: { startSession: string[]; endSession: string[] } = { startSession: [], endSession: [] };
  let issued = 0;
  const api: Pick<SessionsApi, 'startSession' | 'endSession'> = {
    startSession: (deviceId, protocol) => {
      calls.startSession.push(`${deviceId} ${protocol}`);
      issued += 1;
      const ticket: SessionTicket = { sessionId: `session-${String(issued)}`, token: `token-${String(issued)}`, tunnelPath, expiresAt: '2026-11-04T10:00:30Z', wakeSent: false };
      return Promise.resolve({ ok: true, value: ticket });
    },
    endSession: (id) => {
      calls.endSession.push(id);
      return Promise.resolve({ ok: false, error: { code: 'NOT_FOUND', message: 'No such session.' } } as HubResult<EndedSession>);
    },
  };
  return { api, calls };
}

beforeEach(() => {
  FakeTerminalSocket.reset();
});

describe('start and reconnect of an SSH session (D-3, D-5, RA-007)', () => {
  it('RA-007: a start asks for a ticket and opens the terminal tunnel on its path', async () => {
    const { api, calls } = fakeApi();
    const result = await startTerminal(api, TARGET, SETUP);
    expect(calls.startSession).toEqual(['device-1 SSH']);
    expect(result.ok && result.value.sessionId).toBe('session-1');
    expect(FakeTerminalSocket.latest().url).toBe('wss://hub.anima.ing/terminal?token=token-1');
  });

  it('RA-007: Reconnect is another start, with a new ticket and a new socket', async () => {
    const { api, calls } = fakeApi();
    await startTerminal(api, TARGET, SETUP);
    await startTerminal(api, TARGET, SETUP);
    expect(calls.startSession).toHaveLength(2);
    const tokens = FakeTerminalSocket.made.map((socket) => new URL(socket.url).searchParams.get('token'));
    expect(tokens).toEqual(['token-1', 'token-2']);
  });

  it('API-009: the started session holds the session ID and the link, never the ticket', async () => {
    const { api } = fakeApi();
    const result = await startTerminal(api, TARGET, SETUP);
    expect(result.ok && Object.keys(result.value).sort()).toEqual(['sessionId', 'tunnel', 'wakeSent']);
    expect(JSON.stringify(result.ok && result.value.tunnel.state())).not.toContain('token-1');
  });

  it('K7: a hub refusal stays the typed hub error and opens nothing', async () => {
    const { api } = fakeApi();
    const conflict = { code: 'CONFLICT', message: 'No host key is accepted yet.' } as const;
    api.startSession = () => Promise.resolve({ ok: false, error: conflict });
    expect(await startTerminal(api, TARGET, SETUP)).toEqual({ ok: false, error: conflict });
    expect(FakeTerminalSocket.made).toEqual([]);
  });

  it('SEC-002: a ticket path the client refuses ends the new session and reports the refusal', async () => {
    const { api, calls } = fakeApi('//evil.example/terminal');
    const result = await startTerminal(api, TARGET, SETUP);
    expect(result).toEqual({
      ok: false,
      error: { problem: 'bad-address', message: 'The hub gave a tunnel address that Anima does not accept.' },
      wakeSent: false,
    });
    expect(calls.endSession).toEqual(['session-1']);
    expect(FakeTerminalSocket.made).toEqual([]);
  });
});
