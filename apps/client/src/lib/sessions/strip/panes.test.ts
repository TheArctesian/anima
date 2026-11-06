// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Hub } from '$lib/hub/hub';
import { route } from '$lib/nav/route.fixture.svelte';
import Layout from '../../../routes/+layout.svelte';
import type { SessionPort } from '../sessions.svelte';
import { FakeTerminalSocket } from '../tunnel/terminal/fake/socket.fixture';
import { FakeResizeObserver, TOKEN_MARKER } from '../view/view.fixture.svelte';
import { cleanup, fake, openStarted, prepare, request, settle, show as mountLayout, tabs } from './layout.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

vi.mock('$app/state', async () => {
  const { route } = await import('$lib/nav/route.fixture.svelte');
  return {
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

beforeEach(() => {
  prepare();
  route.pathname = '/devices';
});

afterEach(cleanup);

/** The session pane of the `index`th tab. */
function paneAt(index: number): HTMLElement {
  const pane = document.querySelectorAll<HTMLElement>('[role="tabpanel"] [role="group"]')[index];
  if (pane === undefined) throw new Error(`no session pane ${String(index)}`);
  return pane;
}

/** The terminal's text field in the `index`th pane. */
function fieldAt(index: number): HTMLTextAreaElement {
  const field = paneAt(index).querySelector<HTMLTextAreaElement>('textarea');
  if (field === null) throw new Error(`no terminal in pane ${String(index)}`);
  return field;
}

function typeEnter(target: EventTarget) {
  const event = new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', bubbles: true, cancelable: true });
  Object.defineProperty(event, 'keyCode', { value: 13 });
  target.dispatchEvent(event);
}

/** The socket opens, so its pane is ready and may take the keyboard. */
async function ready(socket: FakeTerminalSocket): Promise<FakeTerminalSocket> {
  socket.open();
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
  expect(FakeTerminalSocket.made).toHaveLength(1);
});

it('SEC-002 / LOG-007: the token reaches the tunnel only, never the store or the page', async () => {
  const sessions = show();
  await openStarted(sessions);
  await ready(FakeTerminalSocket.latest());
  expect(FakeTerminalSocket.latest().url).toContain(TOKEN_MARKER);
  expect(JSON.stringify(sessions.tabs)).not.toContain(TOKEN_MARKER);
  expect(document.body.innerHTML).not.toContain(TOKEN_MARKER);
});

it('RA-001: closing a tab ends its session once and closes its tunnel', async () => {
  const sessions = show();
  const id = await openStarted(sessions);
  const socket = await ready(FakeTerminalSocket.latest());
  await sessions.close(id);
  await settle();
  expect(fake.calls.endSession).toEqual(['session-1']);
  expect(socket.closedByClient).toBeDefined();
  expect(tabs()).toEqual([]);
});

it('D-4 / D-5: Reconnect after an ended tunnel starts a new session; the old one is never ended twice', async () => {
  const sessions = show();
  const id = await openStarted(sessions);
  const first = await ready(FakeTerminalSocket.latest());
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
  expect(FakeTerminalSocket.latest().closedByClient).toBeDefined();
});

it('D-8 / D-11: a hidden tab keeps its session running and its output, and only the selected pane takes the focus', async () => {
  const sessions = show();
  const first = await openStarted(sessions);
  const hiddenSocket = await ready(FakeTerminalSocket.latest());
  await openStarted(sessions);
  const shownSocket = await ready(FakeTerminalSocket.latest());
  expect(sessions.active?.id).not.toBe(first);
  const hiddenPanel = paneAt(0).closest<HTMLElement>('[role="tabpanel"]');
  expect(hiddenPanel?.hasAttribute('hidden')).toBe(true);
  expect(hiddenPanel?.inert).toBe(true);
  expect(hiddenSocket.closedByClient).toBeUndefined();
  expect(fake.calls.endSession).toEqual([]);
  hiddenSocket.output('output while hidden');
  expect(document.activeElement).toBe(fieldAt(1));
  typeEnter(fieldAt(1));
  expect(shownSocket.typed()).toEqual([13]);
  await vi.waitFor(() => {
    expect(paneAt(0).textContent).toContain('output while hidden');
  });
});

it('D-8: the terminal gives up the focus while the app window has none, and takes it again when it returns', async () => {
  const sessions = show();
  await openStarted(sessions);
  await ready(FakeTerminalSocket.latest());
  expect(document.activeElement).toBe(fieldAt(0));
  window.dispatchEvent(new Event('blur'));
  flushSync();
  expect(document.activeElement).not.toBe(fieldAt(0));
  window.dispatchEvent(new Event('focus'));
  await settle();
  expect(document.activeElement).toBe(fieldAt(0));
});

it('D-8: the terminal gives up the focus while the page is hidden', async () => {
  const sessions = show();
  await openStarted(sessions);
  await ready(FakeTerminalSocket.latest());
  Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' });
  document.dispatchEvent(new Event('visibilitychange'));
  flushSync();
  Reflect.deleteProperty(document, 'visibilityState');
  expect(document.activeElement).not.toBe(fieldAt(0));
});

it('closing the last tab while its pane has the focus tears the view down without a state error', async () => {
  const sessions = show();
  const id = await openStarted(sessions);
  await ready(FakeTerminalSocket.latest());
  expect(document.activeElement).toBe(fieldAt(0));
  // Chromium fires `focusout` on the focused text field while Svelte clears the panes (jsdom does not): do the same.
  const panes = document.querySelector<HTMLElement>('.panes');
  const clear = Object.getOwnPropertyDescriptor(Node.prototype, 'textContent');
  Object.defineProperty(panes, 'textContent', {
    configurable: true,
    set(text: string) {
      document.activeElement?.dispatchEvent(new FocusEvent('focusout', { bubbles: true }));
      clear?.set?.call(this, text);
    },
  });
  const errors: unknown[] = [];
  const caught = (event: ErrorEvent) => errors.push(event.error);
  window.addEventListener('error', caught);
  await expect(sessions.close(id)).resolves.toBeUndefined();
  await settle();
  window.removeEventListener('error', caught);
  expect(errors).toEqual([]);
  expect(document.querySelector('[role="group"][aria-label^="Terminal"]')).toBeNull();
});

it('no temporary adapter or placeholder pane remains', async () => {
  const layout = readFileSync(`${import.meta.dirname}/../../../routes/+layout.svelte`, 'utf8');
  expect(layout).not.toMatch(/not wired|TEMPORARY|notWired/);
  const sessions = show();
  await openStarted(sessions);
  expect(document.querySelector('[data-placeholder]')).toBeNull();
  expect(document.body.textContent).not.toContain('not available yet');
});

describe('covered pages keep their scroll (WP-14 E-2)', () => {
  /** The pages scrolled to `y` before a session covers them; `scrollTo` is spied on (jsdom has none). */
  function scrolledTo(y: number) {
    Object.defineProperty(window, 'scrollY', { configurable: true, value: y });
    return vi.spyOn(window, 'scrollTo').mockImplementation(() => undefined);
  }

  afterEach(() => {
    Reflect.deleteProperty(window, 'scrollY');
    vi.restoreAllMocks();
  });

  it('showing the same page again with no navigation, by deselect or by closing the last tab, gives it back', async () => {
    const scrollTo = scrolledTo(500);
    const sessions = show();
    const id = await openStarted(sessions);
    sessions.deselect();
    await settle();
    expect(scrollTo).toHaveBeenCalledExactlyOnceWith(0, 500);
    sessions.select(id);
    await settle();
    await sessions.close(id);
    await settle();
    expect(scrollTo.mock.calls).toEqual([
      [0, 500],
      [0, 500],
    ]);
  });

  it('a nav click on the page under the session is a navigation: no old position comes back', async () => {
    const scrollTo = scrolledTo(500);
    const sessions = show();
    await openStarted(sessions);
    // jsdom cannot follow the link; the router is not under test here.
    document.addEventListener(
      'click',
      (event) => {
        event.preventDefault();
      },
      { once: true },
    );
    document.querySelector<HTMLElement>('nav a[href="/devices"]')?.click();
    await settle();
    expect(sessions.active).toBeNull();
    expect(scrollTo).not.toHaveBeenCalled();
  });
});

