// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import type { SessionTransfer } from '../api';
import { fakeHub } from '../fake/hub.fixture';
import { createSessions } from '../sessions.svelte';
import { installBlankCanvas } from '../tunnel/fake/canvas.fixture';
import { FAKE_TUNNEL_SOCKET, FakeTunnelSocket } from '../tunnel/fake/socket.fixture';
import { FAKE_TERMINAL_SOCKET, FakeTerminalSocket } from '../tunnel/terminal/fake/socket.fixture';
import { sessionStart, type RemoteEvents, type TunnelEvents } from './start';

const DISPLAY = { widthPx: 1280, heightPx: 800, dpi: 96, timezone: 'Europe/London' };
const ORIGIN = 'https://hub.anima.ing';
const terminalEvents: TunnelEvents = { size: () => ({ cols: 80, rows: 24 }), onbytes: () => undefined, onchange: () => undefined, onend: () => undefined };
const remoteEvents: RemoteEvents = { onchange: () => undefined, onend: () => undefined };

const fakes: ReturnType<typeof fakeHub>[] = [];
const tunnels: { close(): void }[] = [];
let restoreCanvas: () => void = () => undefined;

function hub(...changes: Parameters<typeof fakeHub>) {
  const fake = fakeHub(...changes);
  fakes.push(fake);
  return fake;
}

/** Remember an open tunnel so the test closes it. */
function kept<Result extends { ok: boolean; value?: { tunnel: { close(): void } } }>(result: Result): Result {
  if (result.value !== undefined) tunnels.push(result.value.tunnel);
  return result;
}

beforeEach(() => {
  FakeTunnelSocket.reset();
  FakeTerminalSocket.reset();
  restoreCanvas = installBlankCanvas();
});

afterEach(async () => {
  for (const tunnel of tunnels.splice(0)) tunnel.close();
  await Promise.all(fakes.splice(0).map((fake) => fake.hub.close()));
  restoreCanvas();
});

describe('sessionStart: one start per tunnel (K4 §1, §8)', () => {
  it('K4 §8: the terminal start asks for SSH and opens the terminal tunnel only', async () => {
    const { api, named } = hub();
    const result = kept(await sessionStart(api, 'device-1', ORIGIN, FAKE_TERMINAL_SOCKET).terminal(DISPLAY, terminalEvents));
    expect(result).toMatchObject({ ok: true, value: { sessionId: 'session-1' } });
    expect(named('StartSession')[0]?.variables).toEqual({ input: { deviceId: 'device-1', protocol: 'SSH', display: DISPLAY } });
    expect(FakeTerminalSocket.latest().url).toBe(`wss://hub.anima.ing/terminal?token=token-1-${'x'.repeat(36)}`);
    expect(FakeTunnelSocket.made).toEqual([]);
  });

  it.each(['RDP', 'VNC'] as const)('K4 §1: the remote start asks for %s and opens the Guacamole tunnel only', async (protocol) => {
    const { api, named } = hub({ certificateExempt: true });
    const result = kept(await sessionStart(api, 'device-1', ORIGIN, FAKE_TUNNEL_SOCKET).remote(protocol)(DISPLAY, remoteEvents));
    expect(result).toMatchObject({ ok: true, value: { sessionId: 'session-1' } });
    expect(named('StartSession')[0]?.variables).toEqual({ input: { deviceId: 'device-1', protocol, display: DISPLAY } });
    expect(FakeTunnelSocket.latest().url).toBe(`wss://hub.anima.ing/tunnel?token=token-1-${'x'.repeat(36)}`);
    expect(FakeTerminalSocket.made).toEqual([]);
  });

  it('SEC-002: a started session holds its ID, tunnel and policy, never the ticket', async () => {
    const { api } = hub({ certificateExempt: true });
    const result = kept(await sessionStart(api, 'device-1', ORIGIN, FAKE_TUNNEL_SOCKET).remote('RDP')(DISPLAY, remoteEvents));
    expect(result.ok && Object.keys(result.value).sort()).toEqual(['policy', 'sessionId', 'tunnel']);
  });

  it('K4 §1: an RDP start the hub refuses is the typed error and opens no tunnel', async () => {
    const { api } = hub();
    const result = await sessionStart(api, 'device-1', ORIGIN, FAKE_TUNNEL_SOCKET).remote('RDP')(DISPLAY, remoteEvents);
    expect(result).toMatchObject({ ok: false, error: { code: 'CONFLICT' } });
    expect(FakeTunnelSocket.made).toEqual([]);
  });
});

describe('the transfer policy read at each start (RA-033, K4 §4)', () => {
  it('RA-033: each start reads the device\'s copy and paste flags', async () => {
    const { api, named } = hub({ transfer: { copy: false, paste: true } });
    const starts = sessionStart(api, 'device-1', ORIGIN, FAKE_TERMINAL_SOCKET);
    const first = kept(await starts.terminal(DISPLAY, terminalEvents));
    const second = kept(await starts.terminal(DISPLAY, terminalEvents));
    expect(first.ok && first.value.policy).toEqual({ copy: false, paste: true, read: true });
    expect(second.ok && second.value.policy).toEqual({ copy: false, paste: true, read: true });
    expect(named('DeviceTransfer')).toHaveLength(2);
  });

  it('RA-033: the read runs beside the ticket request, and the start waits for it', async () => {
    const { api, named } = hub();
    let answer: (result: HubResult<SessionTransfer>) => void = () => undefined;
    const held = { ...api, transferPolicy: () => new Promise<HubResult<SessionTransfer>>((resolve) => (answer = resolve)) };
    let done = false;
    const starting = sessionStart(held, 'device-1', ORIGIN, FAKE_TERMINAL_SOCKET)
      .terminal(DISPLAY, terminalEvents)
      .then((result) => {
        done = true;
        return kept(result);
      });
    await vi.waitFor(() => {
      expect(FakeTerminalSocket.made).toHaveLength(1);
    });
    expect(named('StartSession')).toHaveLength(1);
    expect(done).toBe(false);
    answer({ ok: true, value: { copy: true, paste: false } });
    expect(await starting).toMatchObject({ ok: true, value: { policy: { copy: true, paste: false, read: true } } });
  });

  it.each(['unserved', 'gone'] as const)('RA-033: when the transfer read is %s, copy and paste are off and the start says it was not read', async (transfer) => {
    const { api } = hub({ transfer });
    const result = kept(await sessionStart(api, 'device-1', ORIGIN, FAKE_TERMINAL_SOCKET).terminal(DISPLAY, terminalEvents));
    expect(result).toMatchObject({ ok: true, value: { policy: { copy: false, paste: false, read: false } } });
  });

  it('RA-033: a policy read that rejects reads as not read, and the start keeps its tunnel', async () => {
    const { api } = hub();
    const broken = { ...api, transferPolicy: () => Promise.reject(new Error('broken')) };
    const result = kept(await sessionStart(broken, 'device-1', ORIGIN, FAKE_TERMINAL_SOCKET).terminal(DISPLAY, terminalEvents));
    expect(result).toMatchObject({ ok: true, value: { policy: { copy: false, paste: false, read: false } } });
  });

  it('RA-033: a malformed flag counts as off', async () => {
    const { api } = hub();
    const odd = { ...api, transferPolicy: () => Promise.resolve({ ok: true, value: { copy: null, paste: 'yes' } } as unknown as HubResult<SessionTransfer>) };
    const result = kept(await sessionStart(odd, 'device-1', ORIGIN, FAKE_TERMINAL_SOCKET).terminal(DISPLAY, terminalEvents));
    expect(result).toMatchObject({ ok: true, value: { policy: { copy: false, paste: false, read: true } } });
  });
});

/** A store whose starts run on the fake hub through the real `sessionStart`. */
function store(api: ReturnType<typeof fakeHub>['api']) {
  return createSessions({
    start: (deviceId) => ({
      terminal: sessionStart(api, deviceId, ORIGIN, FAKE_TERMINAL_SOCKET).terminal,
      remote: sessionStart(api, deviceId, ORIGIN, FAKE_TUNNEL_SOCKET).remote,
    }),
    end: (id) => api.endSession(id),
  });
}

const tab = { deviceId: 'device-1', deviceName: 'desk', origin: null } as const;

describe('the store passes the tab\'s protocol (RA-001)', () => {
  it('K4 §1: a VNC tab starts a VNC session over the Guacamole tunnel; an SSH tab the terminal', async () => {
    const { api, named } = hub();
    const sessions = store(api);
    const vnc = sessions.openSession({ ...tab, protocol: 'VNC' });
    const ssh = sessions.openSession({ ...tab, protocol: 'SSH' });
    kept(await sessions.remoteStartIn(vnc)(DISPLAY, remoteEvents));
    kept(await sessions.startIn(ssh)(DISPLAY, terminalEvents));
    expect(named('StartSession').map((sent) => sent.variables)).toEqual([
      { input: { deviceId: 'device-1', protocol: 'VNC', display: DISPLAY } },
      { input: { deviceId: 'device-1', protocol: 'SSH', display: DISPLAY } },
    ]);
    expect([FakeTunnelSocket.made.length, FakeTerminalSocket.made.length]).toEqual([1, 1]);
    expect(sessions.tabs.map((open) => open.sessionId)).toEqual(['session-1', 'session-2']);
  });

  it('a start through the other tunnel\'s view asks the hub for nothing', async () => {
    const { api, sent } = hub();
    const sessions = store(api);
    const rdp = sessions.openSession({ ...tab, protocol: 'RDP' });
    const ssh = sessions.openSession({ ...tab, protocol: 'SSH' });
    expect(await sessions.startIn(rdp)(DISPLAY, terminalEvents)).toMatchObject({ ok: false, error: { problem: 'not-opened' } });
    expect(await sessions.remoteStartIn(ssh)(DISPLAY, remoteEvents)).toMatchObject({ ok: false, error: { problem: 'not-opened' } });
    expect(sent).toEqual([]);
  });

  it('RA-007: a remote Reconnect ends the live session once before it asks for a new ticket', async () => {
    const { api, named } = hub({ certificateExempt: true });
    const sessions = store(api);
    const id = sessions.openSession({ ...tab, protocol: 'RDP' });
    kept(await sessions.remoteStartIn(id)(DISPLAY, remoteEvents));
    kept(await sessions.remoteStartIn(id)(DISPLAY, remoteEvents));
    expect(named('EndSession').map((sent) => sent.variables)).toEqual([{ id: 'session-1' }]);
    expect(sessions.tabs.map((open) => open.sessionId)).toEqual(['session-2']);
  });

  it('K4 §5: once a remote tunnel closes with 4003 the view owns its end; closing the tab ends nothing', async () => {
    const { api, named } = hub({ certificateExempt: true });
    const sessions = store(api);
    const id = sessions.openSession({ ...tab, protocol: 'RDP' });
    const heard: number[] = [];
    kept(await sessions.remoteStartIn(id)(DISPLAY, { onchange: () => undefined, onend: (state) => heard.push(state.closeCode ?? 0) }));
    FakeTunnelSocket.latest().open();
    FakeTunnelSocket.latest().closeWith(4003);
    await vi.waitFor(() => {
      expect(heard).toEqual([4003]);
    });
    expect(sessions.tabs.map((open) => open.sessionId)).toEqual([null]);
    await sessions.close(id);
    expect(named('EndSession')).toEqual([]);
  });
});
