import { expect, it, vi } from 'vitest';
import { SessionEndError, createSessions, type SessionPort, type Sessions } from './sessions.svelte';
import type { TunnelStart } from './tunnel/start';
import type { TunnelState } from './tunnel/state';
import type { TerminalLink } from './tunnel/terminal/link';
import type { SessionStart, TunnelEvents } from './view/start';

const display = { widthPx: 800, heightPx: 600 };
const request = { deviceId: 'web-1', deviceName: 'web-1', protocol: 'SSH', origin: null } as const;
const ENDED = { ok: true, value: undefined } as const;
const UNAVAILABLE = { ok: false, error: { code: 'UNAVAILABLE', message: 'The hub is not ready.' } } as const;
const ENDED_STATE: TunnelState = { phase: 'disconnected', opened: true, gatewayError: undefined, closeCode: 1000 };

function quiet(): TunnelEvents {
  return { size: () => ({ cols: 80, rows: 24 }), onbytes: vi.fn(), onchange: vi.fn(), onend: vi.fn() };
}

/**
 * A hub whose starts give `session-1`, `session-2`, ... with a stand-in tunnel. `hold()` makes the next start wait
 * until the function it returns is called; `events` holds what each start was told to report to.
 */
function fakeHub() {
  let count = 0;
  const held: ((answer: () => void) => void)[] = [];
  const events: TunnelEvents[] = [];
  const open = vi.fn<SessionStart>((_display, heard) => {
    events.push(heard);
    count += 1;
    const sessionId = `session-${String(count)}`;
    const answer: TunnelStart = { ok: true, value: { sessionId, tunnel: { close: vi.fn() } as unknown as TerminalLink } };
    const wait = held.shift();
    return wait === undefined ? Promise.resolve(answer) : new Promise((resolve) => wait(() => resolve(answer)));
  });
  const port = {
    start: vi.fn<SessionPort['start']>(() => open),
    end: vi.fn<SessionPort['end']>(() => Promise.resolve(ENDED)),
  };
  const hold = () => {
    let release: () => void = () => undefined;
    held.push((answer) => (release = answer));
    return () => release();
  };
  return { port, open, events, hold };
}

const sessionIds = (sessions: Sessions) => sessions.tabs.map((open) => open.sessionId);

it('RA-001: two opens give two tabs; switching keeps both; closing one ends it once and keeps the other', async () => {
  const hub = fakeHub();
  const sessions = createSessions(hub.port);
  const first = sessions.openSession(request);
  const second = sessions.openSession(request);
  expect(sessions.tabs.map((open) => open.id)).toEqual([first, second]);
  await sessions.startIn(first)(display, quiet());
  await sessions.startIn(second)(display, quiet());
  sessions.select(first);
  sessions.select(second);
  expect(hub.port.end).not.toHaveBeenCalled();
  await sessions.close(first);
  expect(hub.port.end).toHaveBeenCalledExactlyOnceWith('session-1');
  expect(sessionIds(sessions)).toEqual(['session-2']);
  expect(sessions.active?.id).toBe(second);
});

it('RA-001: opening selects the new tab and records its origin; deselect shows the pages', () => {
  const sessions = createSessions(fakeHub().port);
  const origin = { left: 10, top: 20, width: 30, height: 40 };
  const id = sessions.openSession({ ...request, origin });
  expect(sessions.active).toMatchObject({ id, origin, sessionId: null });
  sessions.deselect();
  expect(sessions.active).toBeNull();
  expect(sessions.tabs).toHaveLength(1);
});

it('D-7: a view start goes to the tab device with the view display, and the tab learns the session id', async () => {
  const hub = fakeHub();
  const sessions = createSessions(hub.port);
  const id = sessions.openSession({ ...request, deviceId: 'db-2' });
  const heard = quiet();
  const started = await sessions.startIn(id)(display, heard);
  expect(started).toMatchObject({ ok: true, value: { sessionId: 'session-1' } });
  expect(hub.port.start).toHaveBeenCalledExactlyOnceWith('db-2');
  expect(hub.open).toHaveBeenCalledExactlyOnceWith(display, expect.anything());
  expect(sessionIds(sessions)).toEqual(['session-1']);
  const state: TunnelState = { ...ENDED_STATE, phase: 'connected', closeCode: undefined };
  hub.events[0]?.onchange(state);
  expect(heard.onchange).toHaveBeenCalledExactlyOnceWith(state);
});

it('a start in a tab that has closed asks the hub for nothing', async () => {
  const hub = fakeHub();
  const sessions = createSessions(hub.port);
  const id = sessions.openSession(request);
  await sessions.close(id);
  await expect(sessions.startIn(id)(display, quiet())).resolves.toMatchObject({ ok: false });
  expect(hub.open).not.toHaveBeenCalled();
});

it('a tab closed while its start is in flight ends the late session once', async () => {
  const hub = fakeHub();
  const release = hub.hold();
  const sessions = createSessions(hub.port);
  const id = sessions.openSession(request);
  const starting = sessions.startIn(id)(display, quiet());
  await sessions.close(id);
  expect(hub.port.end).not.toHaveBeenCalled();
  release();
  // The view has gone and closes the late tunnel it is handed (T2); the store ends the session.
  await expect(starting).resolves.toMatchObject({ ok: true, value: { sessionId: 'session-1' } });
  expect(hub.port.end).toHaveBeenCalledExactlyOnceWith('session-1');
});

it('D-5: a second start replaces the first; one that finishes after it is ended', async () => {
  const hub = fakeHub();
  const release = hub.hold();
  const sessions = createSessions(hub.port);
  const id = sessions.openSession(request);
  const first = sessions.startIn(id)(display, quiet());
  await sessions.startIn(id)(display, quiet());
  release();
  await first;
  expect(hub.port.end).toHaveBeenCalledExactlyOnceWith('session-1');
  expect(sessionIds(sessions)).toEqual(['session-2']);
});

it('D-5 / N2: a start that replaces a live session ends it once before asking for the new one', async () => {
  const hub = fakeHub();
  const sessions = createSessions(hub.port);
  const id = sessions.openSession(request);
  await sessions.startIn(id)(display, quiet());
  hub.port.end.mockImplementationOnce(() => {
    expect(hub.open).toHaveBeenCalledOnce();
    return Promise.resolve(ENDED);
  });
  await sessions.startIn(id)(display, quiet());
  expect(hub.port.end).toHaveBeenCalledExactlyOnceWith('session-1');
  expect(hub.open).toHaveBeenCalledTimes(2);
  expect(sessionIds(sessions)).toEqual(['session-2']);
  await sessions.close(id);
  expect(hub.port.end.mock.calls).toEqual([['session-1'], ['session-2']]);
});

it('D-5: a tab closed while its old session is being ended starts nothing and ends nothing more', async () => {
  const hub = fakeHub();
  const sessions = createSessions(hub.port);
  const id = sessions.openSession(request);
  await sessions.startIn(id)(display, quiet());
  const restarting = sessions.startIn(id)(display, quiet());
  await sessions.close(id);
  await expect(restarting).resolves.toMatchObject({ ok: false });
  expect(hub.port.end).toHaveBeenCalledExactlyOnceWith('session-1');
  expect(hub.open).toHaveBeenCalledOnce();
});

it('D-5: when the old session cannot be ended, no new one starts and the tab keeps the old one', async () => {
  const hub = fakeHub();
  const sessions = createSessions(hub.port);
  const id = sessions.openSession(request);
  await sessions.startIn(id)(display, quiet());
  hub.port.end.mockResolvedValueOnce(UNAVAILABLE);
  await expect(sessions.startIn(id)(display, quiet())).resolves.toEqual(UNAVAILABLE);
  expect(hub.open).toHaveBeenCalledOnce();
  expect(sessionIds(sessions)).toEqual(['session-1']);
});

it('D-4 / D-5: once a tunnel ends the view owns its end; a close or a reconnect never ends it again', async () => {
  const hub = fakeHub();
  const sessions = createSessions(hub.port);
  const id = sessions.openSession(request);
  const heard = quiet();
  await sessions.startIn(id)(display, heard);
  hub.events[0]?.onend(ENDED_STATE);
  expect(heard.onend).toHaveBeenCalledExactlyOnceWith(ENDED_STATE);
  expect(sessionIds(sessions)).toEqual([null]);
  await sessions.startIn(id)(display, quiet());
  expect(hub.port.end).not.toHaveBeenCalled();
  await sessions.close(id);
  expect(hub.port.end).toHaveBeenCalledExactlyOnceWith('session-2');
});

it('D-4: a tunnel that ends before its start answers is not recorded on the tab', async () => {
  const hub = fakeHub();
  const release = hub.hold();
  const sessions = createSessions(hub.port);
  const id = sessions.openSession(request);
  const starting = sessions.startIn(id)(display, quiet());
  await Promise.resolve();
  hub.events[0]?.onend(ENDED_STATE);
  release();
  await starting;
  expect(sessionIds(sessions)).toEqual([null]);
  await sessions.close(id);
  expect(hub.port.end).not.toHaveBeenCalled();
});

it('a hub failure to end reaches the caller as a typed error; closing the tab again ends nothing more', async () => {
  const hub = fakeHub();
  hub.port.end.mockResolvedValueOnce(UNAVAILABLE);
  const sessions = createSessions(hub.port);
  const id = sessions.openSession(request);
  await sessions.startIn(id)(display, quiet());
  const closing = sessions.close(id);
  await expect(closing).rejects.toBeInstanceOf(SessionEndError);
  await expect(closing).rejects.toMatchObject({ code: 'UNAVAILABLE', message: 'The hub could not end the session' });
  await sessions.close(id);
  expect(hub.port.end).toHaveBeenCalledOnce();
  expect(sessions.tabs).toEqual([]);
});

const DEVICE = { deviceId: 'web-1', deviceName: 'web-1', protocol: 'SSH' };

it('RA-002, D-13: popOut opens the device in a window, then removes the tab and ends its session once', async () => {
  const hub = fakeHub();
  const popOut = vi.fn(() => Promise.resolve());
  const sessions = createSessions(hub.port, popOut);
  const id = sessions.openSession(request);
  await sessions.startIn(id)(display, quiet());
  await sessions.popOut(id);
  expect(popOut).toHaveBeenCalledExactlyOnceWith(DEVICE);
  expect(JSON.stringify(popOut.mock.calls)).not.toContain('session-1');
  expect(sessions.tabs).toEqual([]);
  expect(hub.port.end).toHaveBeenCalledExactlyOnceWith('session-1');
});

it('D-13: popping out a tab whose tunnel already ended ends nothing more', async () => {
  const hub = fakeHub();
  const sessions = createSessions(hub.port, () => Promise.resolve());
  const id = sessions.openSession(request);
  await sessions.startIn(id)(display, quiet());
  hub.events[0]?.onend(ENDED_STATE);
  await sessions.popOut(id);
  expect(sessions.tabs).toEqual([]);
  expect(hub.port.end).not.toHaveBeenCalled();
});

it('RA-002: without a port nothing is popped out and the tab stays', async () => {
  const sessions = createSessions(fakeHub().port);
  const id = sessions.openSession(request);
  await sessions.startIn(id)(display, quiet());
  expect(sessions.canPopOut).toBe(false);
  await sessions.popOut(id);
  expect(sessions.tabs).toHaveLength(1);
});

it('a pop out the window refuses keeps the tab and its session', async () => {
  const hub = fakeHub();
  const sessions = createSessions(hub.port, () => Promise.reject(new Error('no window')));
  const id = sessions.openSession(request);
  await sessions.startIn(id)(display, quiet());
  await expect(sessions.popOut(id)).rejects.toThrow('no window');
  expect(sessions.tabs).toHaveLength(1);
  expect(hub.port.end).not.toHaveBeenCalled();
});

it('a hub failure to end a popped-out session reaches the caller; the tab is gone', async () => {
  const hub = fakeHub();
  hub.port.end.mockResolvedValueOnce(UNAVAILABLE);
  const sessions = createSessions(hub.port, () => Promise.resolve());
  const id = sessions.openSession(request);
  await sessions.startIn(id)(display, quiet());
  await expect(sessions.popOut(id)).rejects.toBeInstanceOf(SessionEndError);
  expect(sessions.tabs).toEqual([]);
});

it('moving a tab changes the order the strip shows', () => {
  const sessions = createSessions(fakeHub().port);
  const a = sessions.openSession(request);
  const b = sessions.openSession(request);
  sessions.move(0, 1);
  expect(sessions.tabs.map((open) => open.id)).toEqual([b, a]);
});

function slowWindow() {
  let settle: (opened: boolean) => void = () => undefined;
  const popOut = vi.fn(
    () =>
      new Promise<void>((resolve, reject) => {
        settle = (opened) => {
          if (opened) resolve();
          else reject(new Error('no window'));
        };
      }),
  );
  return { popOut, settle: (opened: boolean) => settle(opened) };
}

for (const opened of [true, false]) {
  it(`RA-002: a tab closed while its window opens ends its session once (window ${opened ? 'opens' : 'fails'})`, async () => {
    const hub = fakeHub();
    const window = slowWindow();
    const sessions = createSessions(hub.port, window.popOut);
    const id = sessions.openSession(request);
    await sessions.startIn(id)(display, quiet());
    const moving = sessions.popOut(id);
    await sessions.close(id);
    window.settle(opened);
    if (opened) await moving;
    else await expect(moving).rejects.toThrow('no window');
    expect(hub.port.end).toHaveBeenCalledExactlyOnceWith('session-1');
    expect(sessions.tabs).toEqual([]);
  });
}

it('K4 §8.7: a title reaches its tab only, next to the device name that stays', () => {
  const sessions = createSessions(fakeHub().port);
  const first = sessions.openSession(request);
  const second = sessions.openSession({ ...request, deviceName: 'db-1' });
  sessions.retitle(second, 'psql');
  expect(sessions.tabs.map((tab) => [tab.id, tab.deviceName, tab.title])).toEqual([
    [first, 'web-1', undefined],
    [second, 'db-1', 'psql'],
  ]);
});
