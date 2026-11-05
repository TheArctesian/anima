// Test support: a fake sessions API, a hand-driven ResizeObserver, a fake system clipboard and Web Animations
// stand-in for mounting `SessionView` in jsdom with the real tunnel on `FakeTunnelSocket`. No vitest import here.
import { flushSync, mount, unmount } from 'svelte';
import type { HubResult } from '$lib/hub/hub';
import type { AcceptedHostKey, ConnectionTest, EndedSession, SessionDisplay, SessionsApi, SessionTicket } from '../api';
import type { SessionEndReason, SessionEnding } from '../ending';
import { FAKE_TUNNEL_SOCKET, FakeTunnelSocket } from '../tunnel/fake/socket.fixture';
import SessionView from './SessionView.svelte';
import { sessionStart } from './start';

/** Every call the fake API heard, in order per method. */
export interface ApiCalls {
  startSession: [string, SessionDisplay][];
  endSession: string[];
  testConnection: string[];
  acceptHostKey: [string, string][];
}

/** What the fake hub answers; tests change it between steps. */
export interface HubAnswers {
  endReason: SessionEndReason | null;
  test: HubResult<ConnectionTest>;
  accept: (fingerprint: string) => HubResult<AcceptedHostKey>;
}

export const NEW_KEY = { algorithm: 'ssh-ed25519', fingerprintSha256: 'SHA256:n3wK3yF1ngerpr1ntAAAAAAAAAAAAAAAAAAAAAAAAAA', firstSeenAt: '2026-11-04T10:00:00Z' };

/** A token marker: it must never appear in the DOM, a log or an error (SEC-002, LOG-007). */
export const TOKEN_MARKER = 'TOKEN-MARKER-7f3a';

export function fakeApi() {
  const calls: ApiCalls = { startSession: [], endSession: [], testConnection: [], acceptHostKey: [] };
  const answers: HubAnswers = {
    endReason: null,
    test: { ok: true, value: { reachable: true, authenticated: true, hostKey: NEW_KEY, hostKeyChanged: true, message: null } },
    accept: (fingerprint) => ({ ok: true, value: { ...NEW_KEY, fingerprintSha256: fingerprint } }),
  };
  let issued = 0;
  const api: SessionsApi = {
    startSession: (deviceId, display) => {
      calls.startSession.push([deviceId, display]);
      issued += 1;
      const n = String(issued);
      const ticket: SessionTicket = { sessionId: `session-${n}`, token: `${TOKEN_MARKER}-${n}`, tunnelPath: '/tunnel', expiresAt: '2026-11-04T10:00:30Z' };
      return Promise.resolve({ ok: true, value: ticket });
    },
    endSession: (id) => {
      calls.endSession.push(id);
      const ended: EndedSession = {
        id,
        deviceId: 'device-1',
        deviceName: 'web-01',
        protocol: 'SSH',
        startedAt: '2026-11-04T10:00:00Z',
        endedAt: '2026-11-04T10:05:00Z',
        endReason: answers.endReason,
      };
      return Promise.resolve({ ok: true, value: ended });
    },
    testConnection: ({ deviceId }) => {
      calls.testConnection.push(deviceId ?? '');
      return Promise.resolve(answers.test);
    },
    acceptHostKey: (deviceId, fingerprint) => {
      calls.acceptHostKey.push([deviceId, fingerprint]);
      return Promise.resolve(answers.accept(fingerprint));
    },
  };
  return { api, calls, answers };
}

/** The flags the parent passes, as state so that a test can change them on a mounted view. */
export const flags = $state({ visible: true, focused: true });

type Callback = (entries: { contentRect: { width: number; height: number } }[]) => void;

/** A ResizeObserver that reports only when a test calls `resize`. */
export class FakeResizeObserver {
  static observers: FakeResizeObserver[] = [];

  /** Report a new content size to every observer. */
  static resize(width: number, height: number): void {
    for (const observer of FakeResizeObserver.observers) observer.callback([{ contentRect: { width, height } }]);
  }

  constructor(private readonly callback: Callback) {}

  observe(): void {
    FakeResizeObserver.observers.push(this);
  }

  disconnect(): void {
    FakeResizeObserver.observers = FakeResizeObserver.observers.filter((observer) => observer !== this);
  }
}

/** The system clipboard as `navigator.clipboard` sees it. */
export const systemClipboard = {
  written: [] as string[],
  text: '',
  refuse: false,
  writeText(text: string): Promise<void> {
    if (this.refuse) return Promise.reject(new Error('denied'));
    this.written.push(text);
    return Promise.resolve();
  },
  readText(): Promise<string> {
    return this.refuse ? Promise.reject(new Error('denied')) : Promise.resolve(this.text);
  },
};

/** Install the fakes jsdom lacks; returns the function that removes them. */
export function installBrowserFakes(): () => void {
  const saved = globalThis.ResizeObserver;
  FakeResizeObserver.observers = [];
  systemClipboard.written = [];
  systemClipboard.text = '';
  systemClipboard.refuse = false;
  globalThis.ResizeObserver = FakeResizeObserver as unknown as typeof ResizeObserver;
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: systemClipboard });
  // Svelte's transitions call Web Animations; this one finishes on the next microtask.
  Object.defineProperty(Element.prototype, 'animate', {
    configurable: true,
    value: () => {
      const animation = { onfinish: null as (() => void) | null, cancel: () => undefined, currentTime: 0 };
      queueMicrotask(() => animation.onfinish?.());
      return animation;
    },
  });
  return () => {
    globalThis.ResizeObserver = saved;
    Reflect.deleteProperty(navigator, 'clipboard');
    Reflect.deleteProperty(Element.prototype, 'animate');
  };
}

export const DEVICE = { id: 'device-1', name: 'web-01' };

/** Let pending promises (the fake hub's answers) settle, then flush Svelte's updates. */
export async function settle(): Promise<void> {
  for (let turn = 0; turn < 10; turn += 1) await Promise.resolve();
  flushSync();
}

/** Mount a `SessionView` for `DEVICE` on the real tunnel over `FakeTunnelSocket`, with `flags` as its flags. */
export function showView() {
  const { api, calls, answers } = fakeApi();
  const ended: SessionEnding[] = [];
  const target = document.createElement('div');
  document.body.append(target);
  const component = mount(SessionView, {
    target,
    props: {
      device: DEVICE,
      start: sessionStart(api, DEVICE.id, 'https://hub.anima.ing', FAKE_TUNNEL_SOCKET),
      api,
      get visible() {
        return flags.visible;
      },
      get focused() {
        return flags.focused;
      },
      onended: (ending: SessionEnding) => {
        ended.push(ending);
      },
    },
  });
  flushSync();
  return { calls, answers, ended, target, close: () => unmount(component) };
}

/** Measure the pane (which starts the session), let the hub answer, open the tunnel and draw the first frame. */
export async function connectView(width = 800, height = 600): Promise<FakeTunnelSocket> {
  FakeResizeObserver.resize(width, height);
  await settle();
  const socket = FakeTunnelSocket.latest();
  socket.open();
  socket.sync();
  flushSync();
  return socket;
}

/** The pane the keyboard listens to. */
export function paneOf(target: HTMLElement): HTMLElement {
  const pane = target.querySelector<HTMLElement>('[role="application"]');
  if (pane === null) throw new Error('no session pane');
  return pane;
}

/** The instructions the client sent with `opcode`, without their opcode. */
export function sent(socket: FakeTunnelSocket, opcode: string): string[][] {
  return socket.sentInstructions().filter(([name]) => name === opcode).map(([, ...args]) => args);
}
