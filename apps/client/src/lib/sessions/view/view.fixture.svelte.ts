// Test support: a fake sessions API, a hand-driven ResizeObserver, a fake system clipboard and Web Animations
// stand-in for mounting `SessionView` in jsdom with the real terminal link on `FakeTerminalSocket` and the real
// xterm.js (its DOM renderer runs in jsdom).
import { flushSync, mount, unmount } from 'svelte';
import { vi } from 'vitest';
import { UNREACHABLE } from '$lib/hub/error';
import type { HubResult } from '$lib/hub/hub';
import type { AcceptedHostKey, ConnectionTest, ConnectionTestInput, EndedSession, SessionDisplay, SessionsApi, SessionTicket, SessionTransfer } from '../api';
import type { SessionEndReason, SessionEnding } from '../ending';
import { openScreen, type ScreenHooks, type ScreenOpener } from '../screen/screen';
import { FAKE_TERMINAL_SOCKET, FakeTerminalSocket } from '../tunnel/terminal/fake/socket.fixture';
import type { Hub } from '$lib/hub/hub';
import Hubbed from './hubbed.fixture.svelte';
import SessionView from './SessionView.svelte';
import { sessionStart } from './start';

/** Every call the fake API heard, in order per method. */
export interface ApiCalls {
  startSession: [string, SessionDisplay][];
  endSession: string[];
  testConnection: ConnectionTestInput[];
  acceptHostKey: [string, string][];
}

/** What the fake hub answers; tests change it between steps. */
export interface HubAnswers {
  endReason: SessionEndReason | null;
  test: HubResult<ConnectionTest>;
  accept: (fingerprint: string) => HubResult<AcceptedHostKey>;
  /** The device's copy and paste flags, or `'unanswered'` when the hub does not answer them. */
  transfer: SessionTransfer | 'unanswered';
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
    transfer: { copy: true, paste: true },
  };
  let issued = 0;
  const api: SessionsApi = {
    startSession: (deviceId, _protocol, display) => {
      calls.startSession.push([deviceId, display]);
      issued += 1;
      const n = String(issued);
      const ticket: SessionTicket = { sessionId: `session-${n}`, token: `${TOKEN_MARKER}-${n}`, tunnelPath: '/terminal', expiresAt: '2026-11-04T10:00:30Z' };
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
    testConnection: (input) => {
      calls.testConnection.push(input);
      return Promise.resolve(answers.test);
    },
    acceptHostKey: (deviceId, fingerprint) => {
      calls.acceptHostKey.push([deviceId, fingerprint]);
      return Promise.resolve(answers.accept(fingerprint));
    },
    acceptServerCertificate: () => Promise.resolve({ ok: false, error: { code: 'NOT_FOUND', message: 'No RDP certificate.' } }),
    transferPolicy: () =>
      Promise.resolve(answers.transfer === 'unanswered' ? { ok: false, error: UNREACHABLE } : { ok: true, value: answers.transfer }),
    setCredential: () => Promise.resolve({ ok: true, value: { credentialStored: true } }),
    history: () => Promise.resolve({ ok: true, value: { pageInfo: { hasNextPage: false, endCursor: null }, edges: [] } }),
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
  /** How many times the clipboard was read. */
  reads: 0,
  text: '',
  refuse: false,
  writeText(text: string): Promise<void> {
    if (this.refuse) return Promise.reject(new Error('denied'));
    this.written.push(text);
    return Promise.resolve();
  },
  readText(): Promise<string> {
    this.reads += 1;
    return this.refuse ? Promise.reject(new Error('denied')) : Promise.resolve(this.text);
  },
};

/** Install the fakes jsdom lacks; returns the function that removes them. */
export function installBrowserFakes(): () => void {
  const saved = globalThis.ResizeObserver;
  FakeResizeObserver.observers = [];
  systemClipboard.written = [];
  systemClipboard.reads = 0;
  systemClipboard.text = '';
  systemClipboard.refuse = false;
  globalThis.ResizeObserver = FakeResizeObserver as unknown as typeof ResizeObserver;
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: systemClipboard });
  // xterm.js asks for the colour scheme and the pixel ratio; jsdom has no `matchMedia`.
  const savedMatchMedia = window.matchMedia;
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
    addListener: () => undefined,
    removeListener: () => undefined,
  })) as unknown as typeof window.matchMedia;
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
    window.matchMedia = savedMatchMedia;
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

/** What a mounted view told its parent, and the hooks the terminal gave it (for events a test cannot make by hand). */
export interface ViewHeard {
  ended: SessionEnding[];
  titles: string[];
  hooks: ScreenHooks | undefined;
}

/** Mount a `SessionView` for `DEVICE` on the real link over `FakeTerminalSocket`, with `flags` as its flags. */
export function showView(wrap: (open: ScreenOpener) => ScreenOpener = (open) => open, hub?: Hub) {
  const { api, calls, answers } = fakeApi();
  const heard: ViewHeard = { ended: [], titles: [], hooks: undefined };
  const opener = wrap((host, hooks) => {
    heard.hooks = hooks;
    return openScreen(host, hooks);
  });
  const target = document.createElement('div');
  document.body.append(target);
  const props = {
    device: DEVICE,
    start: sessionStart(api, DEVICE.id, 'https://hub.anima.ing', FAKE_TERMINAL_SOCKET).terminal,
    api,
    openScreen: opener,
    get visible() {
      return flags.visible;
    },
    get focused() {
      return flags.focused;
    },
    onended: (ending: SessionEnding) => {
      heard.ended.push(ending);
    },
    onretitle: (title: string) => {
      heard.titles.push(title);
    },
  };
  const component = hub === undefined ? mount(SessionView, { target, props }) : mount(Hubbed, { target, props: Object.defineProperty(props, 'hub', { value: hub, enumerable: true }) });
  flushSync();
  return { calls, answers, heard, ended: heard.ended, target, close: () => unmount(component) };
}

/** Measure the pane (which opens the terminal and starts the session), let the hub answer and open the socket. */
export async function connectView(width = 800, height = 600): Promise<FakeTerminalSocket> {
  FakeResizeObserver.resize(width, height);
  await settle();
  const socket = FakeTerminalSocket.latest();
  socket.open();
  flushSync();
  return socket;
}

/** xterm.js's hidden text field, which takes the keyboard. */
export function textareaOf(target: HTMLElement): HTMLTextAreaElement {
  const field = target.querySelector<HTMLTextAreaElement>('textarea.xterm-helper-textarea');
  if (field === null) throw new Error('no terminal text field');
  return field;
}

/** The text the DOM renderer shows; xterm.js draws on animation frames, so this advances the fake timers first. */
export function screenText(target: HTMLElement): string {
  vi.advanceTimersByTime(100);
  return target.querySelector('.xterm-rows')?.textContent ?? '';
}

/** A key press on `target` (xterm.js reads `keyCode`, which the event constructor does not take). */
export function key(target: Element, init: KeyboardEventInit & { keyCode: number }) {
  const event = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init });
  Object.defineProperty(event, 'keyCode', { value: init.keyCode });
  target.dispatchEvent(event);
  return event;
}
