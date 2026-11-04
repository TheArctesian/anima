import { expect, it, vi } from 'vitest';
import { TabClosedError, createSessions, type SessionPort, type StartedSession } from './sessions.svelte';
import { closed, deselected, moved, noTabs, opened, selected, withSession, type Tab, type Tabs } from './tabs';

function tab(id: string, sessionId: string | null = null): Tab {
  return { id, deviceId: `device-${id}`, deviceName: id, protocol: 'SSH', origin: null, sessionId };
}

function strip(ids: string[], activeId: string | null): Tabs {
  return { tabs: ids.map((id) => tab(id)), activeId };
}

const ids = (state: Tabs) => state.tabs.map((open) => open.id);

it('RA-001: an opened tab goes last and is selected, even for a device that already has one', () => {
  const once = opened(noTabs, tab('a'));
  const twice = opened(once, { ...tab('b'), deviceId: 'device-a' });
  expect(ids(twice)).toEqual(['a', 'b']);
  expect(twice.activeId).toBe('b');
});

it.each([
  { name: 'a selected middle tab passes to its right neighbour', before: ['a', 'b', 'c'], active: 'b', close: 'b', after: ['a', 'c'], next: 'c' },
  { name: 'the selected last tab passes to its left neighbour', before: ['a', 'b', 'c'], active: 'c', close: 'c', after: ['a', 'b'], next: 'b' },
  { name: 'a tab that is not selected leaves the selection alone', before: ['a', 'b', 'c'], active: 'a', close: 'c', after: ['a', 'b'], next: 'a' },
  { name: 'the only tab leaves the pages showing', before: ['a'], active: 'a', close: 'a', after: [], next: null },
  { name: 'closing while the pages show keeps them showing', before: ['a', 'b'], active: null, close: 'a', after: ['b'], next: null },
  { name: 'an unknown tab changes nothing', before: ['a'], active: 'a', close: 'z', after: ['a'], next: 'a' },
])('close: $name', ({ before, active, close, after, next }) => {
  const state = closed(strip(before, active), close);
  expect(ids(state)).toEqual(after);
  expect(state.activeId).toBe(next);
});

it.each([
  ['forward', 0, 2, ['b', 'c', 'a', 'd']],
  ['backward', 3, 1, ['a', 'd', 'b', 'c']],
  ['to the same place', 1, 1, ['a', 'b', 'c', 'd']],
  ['past the end, clamped', 0, 9, ['b', 'c', 'd', 'a']],
  ['before the start, clamped', 2, -3, ['c', 'a', 'b', 'd']],
] as const)('move %s', (_name, from, to, after) => {
  const next = moved(strip(['a', 'b', 'c', 'd'], 'b'), from, to);
  expect(ids(next)).toEqual(after);
  expect(next.activeId).toBe('b');
});

it('move on an empty strip changes nothing', () => {
  expect(moved(noTabs, 0, 1)).toBe(noTabs);
});

it('select picks a known tab only; deselect shows the pages and keeps every tab', () => {
  const state = strip(['a', 'b'], 'a');
  expect(selected(state, 'b').activeId).toBe('b');
  expect(selected(state, 'z')).toBe(state);
  expect(deselected(state)).toEqual({ tabs: state.tabs, activeId: null });
});

it('withSession records the session on its tab only', () => {
  const next = withSession(strip(['a', 'b'], 'a'), 'b', 'session-1');
  expect(next.tabs.map((open) => open.sessionId)).toEqual([null, 'session-1']);
});

interface FakeHub extends SessionPort {
  readonly start: ReturnType<typeof vi.fn<SessionPort['start']>>;
  readonly end: ReturnType<typeof vi.fn<SessionPort['end']>>;
}

/** A hub whose starts resolve in order with `session-1`, `session-2`, ... and a token. */
function fakeHub(): FakeHub {
  let count = 0;
  return {
    start: vi.fn(() => {
      count += 1;
      return Promise.resolve({ sessionId: `session-${String(count)}`, token: 'TOKEN-MARKER' } as StartedSession);
    }),
    end: vi.fn(() => Promise.resolve()),
  };
}

const display = { widthPx: 800, heightPx: 600 };
const request = { deviceId: 'web-1', deviceName: 'web-1', protocol: 'SSH', origin: null } as const;

it('RA-001: two opens give two tabs; switching keeps both; closing one ends it once and keeps the other', async () => {
  const hub = fakeHub();
  const sessions = createSessions(hub);
  const first = sessions.openSession(request);
  const second = sessions.openSession(request);
  expect(sessions.tabs.map((open) => open.id)).toEqual([first, second]);
  await sessions.start(first, display);
  await sessions.start(second, display);
  sessions.select(first);
  sessions.select(second);
  expect(hub.end).not.toHaveBeenCalled();
  await sessions.close(first);
  expect(hub.end).toHaveBeenCalledExactlyOnceWith('session-1');
  expect(sessions.tabs.map((open) => open.sessionId)).toEqual(['session-2']);
  expect(sessions.active?.id).toBe(second);
});

it('RA-001: opening selects the new tab and records its origin; deselect shows the pages', () => {
  const sessions = createSessions(fakeHub());
  const origin = { left: 10, top: 20, width: 30, height: 40 };
  const id = sessions.openSession({ ...request, origin });
  expect(sessions.active).toMatchObject({ id, origin, sessionId: null });
  sessions.deselect();
  expect(sessions.active).toBeNull();
  expect(sessions.tabs).toHaveLength(1);
});

it('start asks the hub for the tab device with the display and returns the ticket', async () => {
  const hub = fakeHub();
  const sessions = createSessions(hub);
  const id = sessions.openSession({ ...request, deviceId: 'db-2' });
  await expect(sessions.start(id, display)).resolves.toMatchObject({ sessionId: 'session-1' });
  expect(hub.start).toHaveBeenCalledExactlyOnceWith('db-2', display);
});

it('SEC-002 / LOG-007: the store keeps the session id but never the token', async () => {
  const sessions = createSessions(fakeHub());
  await sessions.start(sessions.openSession(request), display);
  expect(JSON.stringify(sessions.tabs)).not.toContain('TOKEN-MARKER');
});

it('a tab closed before its session started ends that session once, when it starts', async () => {
  const hub = fakeHub();
  let resolve: (started: StartedSession) => void = () => undefined;
  hub.start.mockImplementationOnce(() => new Promise((done) => (resolve = done)));
  const sessions = createSessions(hub);
  const id = sessions.openSession(request);
  const starting = sessions.start(id, display);
  await sessions.close(id);
  expect(hub.end).not.toHaveBeenCalled();
  resolve({ sessionId: 'late' });
  await expect(starting).rejects.toBeInstanceOf(TabClosedError);
  expect(hub.end).toHaveBeenCalledExactlyOnceWith('late');
});

it('D-5: a second start replaces the first; one that finishes after it is ended', async () => {
  const hub = fakeHub();
  let resolveFirst: (started: StartedSession) => void = () => undefined;
  hub.start.mockImplementationOnce(() => new Promise((done) => (resolveFirst = done)));
  const sessions = createSessions(hub);
  const id = sessions.openSession(request);
  const first = sessions.start(id, display);
  await sessions.start(id, display);
  resolveFirst({ sessionId: 'stale' });
  await expect(first).rejects.toBeInstanceOf(TabClosedError);
  expect(hub.end).toHaveBeenCalledExactlyOnceWith('stale');
  expect(sessions.active?.sessionId).toBe('session-1');
});

it('close passes on a hub failure; a closed tab closes once', async () => {
  const hub = fakeHub();
  hub.end.mockRejectedValueOnce(new Error('NOT_FOUND'));
  const sessions = createSessions(hub);
  const id = sessions.openSession(request);
  await sessions.start(id, display);
  await expect(sessions.close(id)).rejects.toThrow('NOT_FOUND');
  await sessions.close(id);
  expect(hub.end).toHaveBeenCalledOnce();
  expect(sessions.tabs).toEqual([]);
});

it('RA-002: popOut hands the session to the port once and removes the tab without ending it', async () => {
  const hub = fakeHub();
  const popOut = vi.fn(() => Promise.resolve());
  const sessions = createSessions(hub, popOut);
  const id = sessions.openSession(request);
  await sessions.popOut(id);
  expect(popOut).not.toHaveBeenCalled();
  await sessions.start(id, display);
  await sessions.popOut(id);
  expect(popOut).toHaveBeenCalledExactlyOnceWith('session-1');
  expect(sessions.tabs).toEqual([]);
  expect(hub.end).not.toHaveBeenCalled();
});

it('RA-002: without a port nothing is popped out and the tab stays', async () => {
  const sessions = createSessions(fakeHub());
  const id = sessions.openSession(request);
  await sessions.start(id, display);
  expect(sessions.canPopOut).toBe(false);
  await sessions.popOut(id);
  expect(sessions.tabs).toHaveLength(1);
});

it('a failed pop out keeps the tab', async () => {
  const sessions = createSessions(fakeHub(), () => Promise.reject(new Error('no window')));
  const id = sessions.openSession(request);
  await sessions.start(id, display);
  await expect(sessions.popOut(id)).rejects.toThrow('no window');
  expect(sessions.tabs).toHaveLength(1);
});

it('move reorders the store', () => {
  const sessions = createSessions(fakeHub());
  const a = sessions.openSession(request);
  const b = sessions.openSession(request);
  sessions.move(0, 1);
  expect(sessions.tabs.map((open) => open.id)).toEqual([b, a]);
});
