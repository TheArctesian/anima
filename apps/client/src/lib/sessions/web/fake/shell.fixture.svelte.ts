// Test support: a fake desktop shell and hub API for mounting `WebTab` in jsdom, and a ResizeObserver a test drives.
import { flushSync, mount, unmount } from 'svelte';
import type { Bounds, Fingerprint, WebViewEvent, WebViewOpening, WebViewShell } from '$lib/desktop/webView/webView';
import { WebViewError } from '$lib/desktop/webView/webView';
import type { Status } from '$lib/hub/generated/graphql';
import type { HubResult } from '$lib/hub/hub';
import type { WebCertificatePin, WebTabApi } from '../api';
import WebTab from '../WebTab.svelte';

export const BOUNDS = { x: 0, y: 48, width: 800, height: 552 };

/** One shell command as the fake heard it. */
export type ShellCall =
  | { readonly command: 'open'; readonly opening: WebViewOpening }
  | { readonly command: 'place'; readonly bounds: Bounds }
  | { readonly command: 'show'; readonly shown: boolean }
  | { readonly command: 'go'; readonly to: string }
  | { readonly command: 'pin'; readonly pin: Fingerprint }
  | { readonly command: 'popOut' | 'close' }
  | { readonly command: 'browser' | 'read'; readonly url: string };

/** A shell that records every command; `answers` sets what it says (`refuse`: command to code), `emit` plays an event. */
export function fakeShell() {
  const calls: ShellCall[] = [];
  const answers = {
    pinning: true,
    certificate: null as Fingerprint | null,
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- the fake's answers are read by the shell calls, never rendered
    refuse: new Map<ShellCall['command'], string>(),
    /** When set, a certificate read waits for it before answering. */
    gate: null as Promise<void> | null,
  };
  let listener: ((event: WebViewEvent) => void) | undefined;
  const heard = (call: ShellCall): Promise<void> => {
    calls.push(call);
    const code = answers.refuse.get(call.command);
    return code === undefined ? Promise.resolve() : Promise.reject(new WebViewError(code));
  };
  const shell: WebViewShell = {
    pinning: () => Promise.resolve(answers.pinning),
    open: (opening) => heard({ command: 'open', opening }),
    place: (_tabId, bounds) => heard({ command: 'place', bounds }),
    show: (_tabId, shown) => heard({ command: 'show', shown }),
    go: (_tabId, to) => heard({ command: 'go', to }),
    pin: (_tabId, pin) => heard({ command: 'pin', pin }),
    popOut: () => heard({ command: 'popOut' }),
    close: () => heard({ command: 'close' }),
    openSystemBrowser: (url) => heard({ command: 'browser', url }),
    readCertificate: async (url) => {
      await heard({ command: 'read', url });
      await answers.gate;
      if (answers.certificate === null) throw new WebViewError('unreachable');
      return answers.certificate;
    },
    listen: (onevent) => {
      listener = onevent;
      return Promise.resolve(() => (listener = undefined));
    },
  };
  return {
    shell,
    calls,
    answers,
    emit: (event: WebViewEvent) => listener?.(event),
    /** The commands heard, by name only. */
    names: () => calls.map(({ command }) => command),
  };
}

/** A hub API that pins what it is given, unless `answer` says otherwise. */
export function fakeApi() {
  const accepts: [string, string][] = [];
  const control = {
    answer: (fingerprint: string): HubResult<WebCertificatePin> => ({
      ok: true,
      value: { id: 'web-1', pinnedFingerprintSha256: fingerprint, pinnedAt: '2026-11-06T10:00:00Z' },
    }),
  };
  const api: WebTabApi = {
    acceptWebCertificate: (id, fingerprint) => {
      accepts.push([id, fingerprint]);
      return Promise.resolve(control.answer(fingerprint));
    },
  };
  return { api, accepts, control };
}

/** A ResizeObserver that reports only when a test calls `resize`; the view area then measures `rect`. */
export class FakeResizeObserver {
  static observers: FakeResizeObserver[] = [];
  static rect = { x: 0, y: 48, width: 800, height: 552 };

  static resize(rect: Bounds): void {
    FakeResizeObserver.rect = rect;
    for (const observer of FakeResizeObserver.observers) observer.callback();
  }

  constructor(private readonly callback: () => void) {
    FakeResizeObserver.observers.push(this);
  }

  observe(): void {
    // The test calls `resize`.
  }

  disconnect(): void {
    FakeResizeObserver.observers = FakeResizeObserver.observers.filter((observer) => observer !== this);
  }
}

/** Install the fake observer and the view area's measure; returns the function that removes them. */
export function installLayout(): () => void {
  const saved = globalThis.ResizeObserver;
  const measure = Object.getOwnPropertyDescriptor(Element.prototype, 'getBoundingClientRect');
  FakeResizeObserver.observers = [];
  globalThis.ResizeObserver = FakeResizeObserver as unknown as typeof ResizeObserver;
  Object.defineProperty(Element.prototype, 'getBoundingClientRect', {
    configurable: true,
    value(this: Element) {
      const { x, y, width, height } = this.hasAttribute('data-view-area') ? FakeResizeObserver.rect : { x: 0, y: 0, width: 0, height: 0 };
      return { x, y, width, height, top: y, left: x, right: x + width, bottom: y + height, toJSON: () => ({}) };
    },
  });
  return () => {
    globalThis.ResizeObserver = saved;
    if (measure !== undefined) Object.defineProperty(Element.prototype, 'getBoundingClientRect', measure);
  };
}

/** Let pending promises settle, then flush Svelte's updates. */
export async function settle(): Promise<void> {
  for (let turn = 0; turn < 5; turn += 1) await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

/** What `showTab` mounts: the interface URL and pin, the device status, the platform and the device certificate. */
export interface Shown {
  url?: string;
  pinned?: string | null;
  status?: Status;
  pinning?: boolean;
  certificate?: Fingerprint;
}

/** A web tab over a fake shell and hub; `props` changes visibility, overlays and the device status. */
export async function showTab({ url = 'https://nas.lan:5001/', pinned = null, status = 'UP', pinning = true, certificate }: Shown, cleanups: (() => unknown)[]) {
  const fake = fakeShell();
  fake.answers.pinning = pinning;
  fake.answers.certificate = certificate ?? null;
  const hub = fakeApi();
  const props = $state({
    webInterface: { id: 'web-1', deviceId: 'device-1', url, label: 'DSM', pinnedFingerprintSha256: pinned },
    device: { id: 'device-1', name: 'nas', status },
    tabId: 'tab-1',
    visible: true,
    overlayOpen: false,
    shell: fake.shell,
    api: hub.api,
  });
  const component = mount(WebTab, { target: document.body, props });
  cleanups.push(() => unmount(component));
  await settle();
  FakeResizeObserver.resize(BOUNDS);
  await settle();
  return { ...fake, ...hub, props, component };
}

/** The first button named `name`, by its accessible name or its text without icon glyphs (UI-016). */
export const button = (name: string) =>
  [...document.querySelectorAll('button')].find(
    (candidate) => (candidate.getAttribute('aria-label') ?? candidate.textContent.replace(/[\u{F0000}-\u{FFFFD}]/gu, '').trim()) === name,
  );
export const field = (name: string) => document.querySelector(`[data-${name}]`)?.textContent.trim();
/** A field of the page over the view area, not the toolbar's. */
export const pageField = (name: string) => document.querySelector(`[data-view-area] [data-${name}]`)?.textContent.trim();
export const lastShow = (calls: { command: string; shown?: boolean }[]) => calls.filter(({ command }) => command === 'show').at(-1)?.shown;
