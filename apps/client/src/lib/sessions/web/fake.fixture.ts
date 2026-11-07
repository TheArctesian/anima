// Test support: a fake desktop shell and hub API for mounting `WebTab` in jsdom, and a ResizeObserver a test drives.
import { flushSync } from 'svelte';
import type { Bounds, Fingerprint, WebViewEvent, WebViewOpening, WebViewShell } from '$lib/desktop/webView/webView';
import { WebViewError } from '$lib/desktop/webView/webView';
import type { HubResult } from '$lib/hub/hub';
import type { WebCertificatePin, WebTabApi } from './api';

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
  const answers = { pinning: true, certificate: null as Fingerprint | null, refuse: new Map<ShellCall['command'], string>() };
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
