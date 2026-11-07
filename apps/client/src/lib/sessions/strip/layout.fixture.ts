// Test support: the root layout with a fake hub behind the session store and the session views, in jsdom.
// The test files mock `$lib/sessions/api` so the layout's `sessionsApi` is `fake.api`, and
// `$lib/sessions/sessions.svelte` so `createSessions` records the layout's port in `fake.port` and the store in
// `fake.sessions`. Tunnels run on `FakeTerminalSocket` with the real xterm.js, and panes measure through
// `FakeResizeObserver`.
import { createRawSnippet, flushSync, mount, unmount, type Component } from 'svelte';
import { vi } from 'vitest';
import type { SessionsApi } from '../api';
import type { PopOutPort, SessionPort, Sessions } from '../sessions.svelte';
import { FakeTerminalSocket } from '../tunnel/terminal/fake/socket.fixture';
import { FakeResizeObserver, fakeApi, type ApiCalls, type HubAnswers } from '../view/view.fixture.svelte';

/** A spy on each of the port's calls. */
interface SpiedPort extends SessionPort {
  readonly start: ReturnType<typeof vi.fn<SessionPort['start']>>;
  readonly end: ReturnType<typeof vi.fn<SessionPort['end']>>;
}

export const fake = {
  api: undefined as unknown as SessionsApi,
  /** Every hub call, from the store and the views alike. */
  calls: undefined as unknown as ApiCalls,
  answers: undefined as unknown as HubAnswers,
  port: undefined as unknown as SpiedPort,
  popOut: undefined as PopOutPort | undefined,
  sessions: undefined as unknown as Sessions,
};

/** The store over the layout's own `port`, with its calls spied on; the mocked `createSessions` calls it. */
export function spiedSessions(create: (port: SessionPort, popOut?: PopOutPort) => Sessions, port: SessionPort): Sessions {
  fake.port = { start: vi.fn((deviceId: string) => port.start(deviceId)), end: vi.fn((id: string) => port.end(id)) };
  fake.sessions = create(fake.port, fake.popOut);
  return fake.sessions;
}

/** Every keyframe list Svelte's transitions asked for, in order. */
export const keyframes: Keyframe[][] = [];

let app: ReturnType<typeof mount> | undefined;

/** jsdom has no Web Animations; this one finishes at once so outros complete. */
function finishingAnimate(frames: Keyframe[]) {
  keyframes.push(frames);
  return {
    cancel: () => undefined,
    currentTime: 0,
    playState: 'finished',
    effect: null,
    set onfinish(done: () => void) {
      queueMicrotask(done);
    },
  };
}

const stubs: Record<string, unknown> = {
  animate: finishingAnimate,
  setPointerCapture: () => undefined,
  releasePointerCapture: () => undefined,
  hasPointerCapture: () => true,
};

let restore: (() => void)[] = [];

/** A fresh fake hub whose starts give `session-1`, `session-2`, ...; and the jsdom stubs. */
export function prepare(): void {
  ({ api: fake.api, calls: fake.calls, answers: fake.answers } = fakeApi());
  keyframes.length = 0;
  for (const [name, value] of Object.entries(stubs)) {
    Object.defineProperty(Element.prototype, name, { value, configurable: true });
  }
  FakeTerminalSocket.reset();
  FakeResizeObserver.observers = [];
  const saved = { WebSocket: globalThis.WebSocket, ResizeObserver: globalThis.ResizeObserver };
  globalThis.WebSocket = FakeTerminalSocket as unknown as typeof WebSocket;
  globalThis.ResizeObserver = FakeResizeObserver as unknown as typeof ResizeObserver;
  restore = [
    () => {
      Object.assign(globalThis, saved);
    },
  ];
}

export async function cleanup(): Promise<void> {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  for (const name of Object.keys(stubs)) Reflect.deleteProperty(Element.prototype, name);
  for (const undo of restore.reverse()) undo();
  restore = [];
}

export async function settle(): Promise<void> {
  for (let tick = 0; tick < 10; tick += 1) {
    await Promise.resolve();
    flushSync();
  }
}

/**
 * xterm.js asks for the colour scheme and the pixel ratio, and jsdom has no `matchMedia`. The stand-in comes after the
 * layout is mounted, because the layout's motion preference reads `matchMedia` once, when it mounts, and the tests
 * of the strip rely on the answer for a window without one.
 */
function giveMatchMedia(): void {
  const saved = window.matchMedia;
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
    addListener: () => undefined,
    removeListener: () => undefined,
  })) as unknown as typeof window.matchMedia;
  restore.push(() => {
    window.matchMedia = saved;
  });
}

/** Mounts `layout` with a stand-in page; `popOut` is the window port, if any. */
export function show(layout: Component<{ children: ReturnType<typeof createRawSnippet> }>, popOut?: PopOutPort): Sessions {
  fake.popOut = popOut;
  const children = createRawSnippet(() => ({ render: () => '<p id="page">page</p>' }));
  app = mount(layout, { target: document.body, props: { children } });
  flushSync();
  giveMatchMedia();
  return fake.sessions;
}

export const request = { deviceId: 'web-1', deviceName: 'web-1', protocol: 'SSH', origin: null } as const;

export const tabs = () => [...document.querySelectorAll<HTMLElement>('[role="tab"]')];

export const visiblePanels = () =>
  [...document.querySelectorAll('[role="tabpanel"]')].filter((panel) => !panel.closest('[hidden]'));

/** Opens a tab for `request`'s device; its pane's first measure starts the session, as a browser's would. */
export async function openStarted(sessions: Sessions, origin: DOMRect | null = null): Promise<string> {
  const id = sessions.openSession({ ...request, origin });
  flushSync();
  FakeResizeObserver.resize(800, 600);
  await settle();
  return id;
}
