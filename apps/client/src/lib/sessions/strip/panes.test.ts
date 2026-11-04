// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { Hub } from '$lib/hub/hub';
import { route } from '$lib/nav/route.fixture.svelte';
import Layout from '../../../routes/+layout.svelte';
import type { SessionPort } from '../sessions.svelte';
import { FakeTunnelSocket } from '../tunnel/fake/socket.fixture';
import { FakeResizeObserver, TOKEN_MARKER, sent } from '../view/view.fixture.svelte';
import { cleanup, fake, openStarted, prepare, request, settle, show as mountLayout, tabs } from './layout.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

vi.mock('$app/state', async () => {
  const { route } = await import('$lib/nav/route.fixture.svelte');
  return {
    navigating: { to: null },
    page: {
      get url() {
        return new URL(route.pathname, 'http://localhost');
      },
    },
  };
});

vi.mock('$lib/hub/hub', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/hub/hub')>()),
  createHub: (): Hub =>
    ({
      address: { origin: 'https://hub.anima.ing', http: '', socket: '' },
      status: { current: { connection: 'connected', lastContactAt: null } },
      close: () => Promise.resolve(),
    }) as unknown as Hub,
}));

// The layout's sessions API is the fixture's fake hub; its port is spied on.
vi.mock('$lib/sessions/api', async (importOriginal) => {
  const { fake: state } = await import('./layout.fixture');
  return { ...(await importOriginal<typeof import('$lib/sessions/api')>()), sessionsApi: () => state.api };
});

vi.mock('$lib/sessions/sessions.svelte', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$lib/sessions/sessions.svelte')>();
  const { spiedSessions } = await import('./layout.fixture');
  return { ...actual, createSessions: (port: SessionPort) => spiedSessions(actual.createSessions, port) };
});

/** The layout in a window that has the focus (jsdom's document never has it). */
function show() {
  const sessions = mountLayout(Layout);
  window.dispatchEvent(new Event('focus'));
  flushSync();
  return sessions;
}
const ENTER = 0xff0d;

beforeEach(() => {
  prepare();
  route.pathname = '/devices';
});

afterEach(cleanup);

/** The session pane (the keyboard's element) of the `index`th tab. */
function paneAt(index: number): HTMLElement {
  const pane = document.querySelectorAll<HTMLElement>('[role="tabpanel"] [role="application"]')[index];
  if (pane === undefined) throw new Error(`no session pane ${String(index)}`);
  return pane;
}

function typeEnter(target: EventTarget) {
  for (const type of ['keydown', 'keyup']) {
    const event = new KeyboardEvent(type, { key: 'Enter', code: 'Enter', bubbles: true, cancelable: true });
    Object.defineProperty(event, 'keyCode', { value: 13 });
    target.dispatchEvent(event);
  }
}

/** The gateway's first frame on `socket`, so its pane is ready and may take the keyboard. */
async function ready(socket: FakeTunnelSocket): Promise<FakeTunnelSocket> {
  socket.open();
  socket.sync();
  await settle();
  return socket;
}

/** The next `startSession` waits until the function this returns is called. */
function holdStart(): () => void {
  const answer = fake.api.startSession.bind(fake.api);
  let release: () => void = () => undefined;
  vi.spyOn(fake.api, 'startSession').mockImplementationOnce(
    (deviceId, display) =>
      new Promise((resolve) => {
        release = () => {
          resolve(answer(deviceId, display));
        };
      }),
  );
  return () => {
    release();
  };
}

it('RA-001 / D-7: openSession gives a tab whose view starts the session once with the measured display', async () => {
  const sessions = show();
  const id = await openStarted(sessions);
  expect(tabs()).toHaveLength(1);
  const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  expect(fake.calls.startSession).toEqual([[request.deviceId, { widthPx: 800, heightPx: 600, dpi: 96, timezone: zone }]]);
  expect(fake.port.start).toHaveBeenCalledExactlyOnceWith(request.deviceId);
  expect(sessions.tabs.find((tab) => tab.id === id)?.sessionId).toBe('session-1');
  expect(FakeTunnelSocket.made).toHaveLength(1);
});

it('SEC-002 / LOG-007: the token reaches the tunnel only, never the store or the page', async () => {
  const sessions = show();
  await openStarted(sessions);
  await ready(FakeTunnelSocket.latest());
  expect(FakeTunnelSocket.latest().url).toContain(TOKEN_MARKER);
  expect(JSON.stringify(sessions.tabs)).not.toContain(TOKEN_MARKER);
  expect(document.body.innerHTML).not.toContain(TOKEN_MARKER);
});

it('RA-001: closing a tab ends its session once and closes its tunnel', async () => {
  const sessions = show();
  const id = await openStarted(sessions);
  const socket = await ready(FakeTunnelSocket.latest());
  await sessions.close(id);
  await settle();
  expect(fake.calls.endSession).toEqual(['session-1']);
  expect(socket.closedByClient).toBeDefined();
  expect(tabs()).toEqual([]);
});

it('D-4 / D-5: Reconnect after an ended tunnel starts a new session; the old one is never ended twice', async () => {
  const sessions = show();
  const id = await openStarted(sessions);
  const first = await ready(FakeTunnelSocket.latest());
  first.disconnect();
  first.closeWith(1000);
  await settle();
  expect(fake.calls.endSession).toEqual(['session-1']);
  const reconnect = [...document.querySelectorAll('[role="tabpanel"] button')].find((button) =>
    button.textContent.includes('Reconnect'),
  );
  (reconnect as HTMLButtonElement | undefined)?.click();
  await settle();
  expect(fake.calls.startSession).toHaveLength(2);
  expect(sessions.tabs.find((tab) => tab.id === id)?.sessionId).toBe('session-2');
  expect(fake.calls.endSession).toEqual(['session-1']);
  await sessions.close(id);
  await settle();
  expect(fake.calls.endSession).toEqual(['session-1', 'session-2']);
});

it('a tab closed while its start is in flight has the late session ended and its tunnel closed', async () => {
  const sessions = show();
  const release = holdStart();
  const id = sessions.openSession({ ...request, origin: null });
  flushSync();
  FakeResizeObserver.resize(800, 600);
  await settle();
  await sessions.close(id);
  await settle();
  expect(fake.calls.endSession).toEqual([]);
  release();
  await settle();
  expect(fake.calls.endSession).toEqual(['session-1']);
  expect(FakeTunnelSocket.latest().closedByClient).toBeDefined();
});

it('D-8 / D-11: a hidden tab keeps its session running, and only the selected pane takes keys', async () => {
  const sessions = show();
  const first = await openStarted(sessions);
  const hiddenSocket = await ready(FakeTunnelSocket.latest());
  await openStarted(sessions);
  const shownSocket = await ready(FakeTunnelSocket.latest());
  expect(sessions.active?.id).not.toBe(first);
  const hiddenPanel = paneAt(0).closest<HTMLElement>('[role="tabpanel"]');
  expect(hiddenPanel?.hasAttribute('hidden')).toBe(true);
  expect(hiddenPanel?.inert).toBe(true);
  typeEnter(paneAt(0));
  expect(sent(hiddenSocket, 'key')).toEqual([]);
  expect(hiddenSocket.closedByClient).toBeUndefined();
  expect(fake.calls.endSession).toEqual([]);
  typeEnter(paneAt(1));
  expect(sent(shownSocket, 'key')).toEqual([
    [String(ENTER), '1'],
    [String(ENTER), '0'],
  ]);
});

it('D-8: the keyboard is unbound while the app window has no focus, and bound again when it returns', async () => {
  const sessions = show();
  await openStarted(sessions);
  const socket = await ready(FakeTunnelSocket.latest());
  window.dispatchEvent(new Event('blur'));
  flushSync();
  typeEnter(paneAt(0));
  expect(sent(socket, 'key')).toEqual([]);
  window.dispatchEvent(new Event('focus'));
  await settle();
  typeEnter(paneAt(0));
  expect(sent(socket, 'key')).toHaveLength(2);
});

it('D-8: the keyboard is unbound while the page is hidden', async () => {
  const sessions = show();
  await openStarted(sessions);
  const socket = await ready(FakeTunnelSocket.latest());
  Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' });
  document.dispatchEvent(new Event('visibilitychange'));
  flushSync();
  Reflect.deleteProperty(document, 'visibilityState');
  typeEnter(paneAt(0));
  expect(sent(socket, 'key')).toEqual([]);
});

it('no temporary adapter or placeholder pane remains', async () => {
  const layout = readFileSync(`${import.meta.dirname}/../../../routes/+layout.svelte`, 'utf8');
  expect(layout).not.toMatch(/not wired|TEMPORARY|notWired/);
  const sessions = show();
  await openStarted(sessions);
  expect(document.querySelector('[data-placeholder]')).toBeNull();
  expect(document.body.textContent).not.toContain('not available yet');
});

