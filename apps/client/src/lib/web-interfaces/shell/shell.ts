import type { WebViewEvent } from './events';
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

/** The `{ fingerprintSha256 }` answer of `web_certificate_read`, checked field by field. */
export function certificateFrom(answer: unknown): Fingerprint {
  const fingerprint =
    typeof answer === 'object' && answer !== null && 'fingerprintSha256' in answer ? fingerprintFrom(answer.fingerprintSha256) : null;
  if (fingerprint === null) throw new WebViewError('malformed');
  return fingerprint;
}
