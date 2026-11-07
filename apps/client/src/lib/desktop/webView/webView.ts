import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { webUrlFrom, webViewEventFrom, type WebViewEvent } from './events';
import { fingerprintFrom, type Fingerprint } from './fingerprint';

export type { WebViewEvent } from './events';
export { displayFingerprint, fingerprintFrom, type Fingerprint } from './fingerprint';

/** Where a web view sits: CSS pixels in the main window. */
export interface Bounds {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** A history step or a reload of a web view. */
export type WebViewStep = 'BACK' | 'FORWARD' | 'RELOAD';

/** What `web_view_open` is given; `pin` is the fingerprint the view must accept, or `null` for system trust. */
export interface WebViewOpening {
  readonly tabId: string;
  readonly deviceId: string;
  readonly url: string;
  readonly pin: Fingerprint | null;
  readonly bounds: Bounds;
}

/** A shell command that failed. The message is fixed text: the shell's error carries a code only (LOG-007). */
export class WebViewError extends Error {
  constructor(readonly code: string) {
    super('The desktop app could not do this for the web view.');
    this.name = 'WebViewError';
  }
}

/** The shell's web view commands and event (WP-31 "Shell interface"). Every answer is checked. */
export interface WebViewShell {
  /** Whether a web view can pin a certificate here (D-6); `false` outside the desktop app or on failure. */
  pinning(): Promise<boolean>;
  open(opening: WebViewOpening): Promise<void>;
  place(tabId: string, bounds: Bounds): Promise<void>;
  show(tabId: string, shown: boolean): Promise<void>;
  go(tabId: string, to: WebViewStep): Promise<void>;
  /** Pin `pin` for the tab's web view; the shell reloads it. */
  pin(tabId: string, pin: Fingerprint): Promise<void>;
  /** Move the web view to its own window with the same store. */
  popOut(tabId: string): Promise<void>;
  close(tabId: string): Promise<void>;
  /** Open an `http` or `https` URL in the system browser. */
  openSystemBrowser(url: string): Promise<void>;
  /** The SHA-256 fingerprint of the leaf certificate an `https` URL presents now. */
  readCertificate(url: string): Promise<Fingerprint>;
  /** Hear `web-view` events; unknown shapes are dropped. Resolves to the function that stops listening. */
  listen(onevent: (event: WebViewEvent) => void): Promise<() => void>;
}

const CODE = /^[a-z][a-z0-9_]{0,39}$/;

/** The shell's error code, or `failed` for anything that is not one; no other part of the error is kept. */
export function codeOf(reason: unknown): string {
  const code = typeof reason === 'object' && reason !== null && 'code' in reason ? reason.code : undefined;
  return typeof code === 'string' && CODE.test(code) ? code : 'failed';
}

async function command(name: string, args: Record<string, unknown>): Promise<unknown> {
  try {
    return await invoke(name, args);
  } catch (reason) {
    throw new WebViewError(codeOf(reason));
  }
}

/** The `{ fingerprintSha256 }` answer of `web_certificate_read`, checked field by field. */
export function certificateFrom(answer: unknown): Fingerprint {
  const fingerprint =
    typeof answer === 'object' && answer !== null && 'fingerprintSha256' in answer ? fingerprintFrom(answer.fingerprintSha256) : null;
  if (fingerprint === null) throw new WebViewError('malformed');
  return fingerprint;
}

async function pinning(): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    // Anything but `true` keeps the system browser, which never trusts a certificate the operator did not see.
    return (await invoke('web_view_pinning')) === true;
  } catch {
    return false;
  }
}

/**
 * The web view commands of the desktop shell. The event is heard on the main web view only: the shell emits it to
 * `main`, and device web views have no IPC to emit with (D-3).
 */
export const desktopWebView: WebViewShell = {
  pinning,
  open: async ({ tabId, deviceId, url, pin, bounds }) => {
    await command('web_view_open', { tabId, deviceId, url, pin, bounds });
  },
  place: async (tabId, bounds) => {
    await command('web_view_place', { tabId, bounds });
  },
  show: async (tabId, shown) => {
    await command('web_view_show', { tabId, shown });
  },
  go: async (tabId, to) => {
    await command('web_view_go', { tabId, to });
  },
  pin: async (tabId, pin) => {
    await command('web_view_pin', { tabId, pin });
  },
  popOut: async (tabId) => {
    await command('web_view_pop_out', { tabId });
  },
  close: async (tabId) => {
    await command('web_view_close', { tabId });
  },
  openSystemBrowser: async (url) => {
    // The shell checks the scheme too; this keeps a wrong URL from ever reaching it.
    if (webUrlFrom(url) === null) throw new WebViewError('invalid_url');
    await command('system_browser_open', { url });
  },
  readCertificate: async (url) => {
    if (webUrlFrom(url) === null || new URL(url).protocol !== 'https:') throw new WebViewError('invalid_url');
    return certificateFrom(await command('web_certificate_read', { url }));
  },
  listen: (onevent) =>
    listen<unknown>(
      'web-view',
      ({ event, payload }) => {
        if (event !== 'web-view') return;
        const parsed = webViewEventFrom(payload);
        if (parsed !== null) onevent(parsed);
      },
      { target: { kind: 'WebviewWindow', label: 'main' } },
    ),
};
