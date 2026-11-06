import { fingerprintFrom, type Fingerprint, type WebViewEvent } from '$lib/desktop/webView/webView';
import type { HubResult } from '$lib/hub/hub';
import type { WebCertificatePin } from './api';

/** Where a web interface opens (D-6): a web view in the tab, or the system browser beside a pinned fingerprint. */
export type WebMode = 'EMBEDDED' | 'SYSTEM_BROWSER';

/**
 * What the tab body shows. `untrusted` is the first-visit prompt (WEB-008); `mismatch` is the stop of SEC-007, and
 * accepting there needs a second, explicit step (`confirming`); `failed` and `down` are error pages with Reload
 * (WEB-010); `refused` has no Reload, since the interface itself cannot be opened. `reading` and `browser` belong
 * to the system-browser mode. A `reason` is a short code, never text from the hub or a page (LOG-007).
 */
export type WebStep =
  | { readonly step: 'loading' }
  | { readonly step: 'loaded'; readonly url: string }
  | { readonly step: 'untrusted'; readonly presented: Fingerprint; readonly problem?: string }
  | { readonly step: 'accepting'; readonly presented: Fingerprint }
  | { readonly step: 'mismatch'; readonly expected: Fingerprint; readonly presented: Fingerprint; readonly confirming: boolean; readonly problem?: string }
  | { readonly step: 'failed'; readonly reason: string }
  | { readonly step: 'down' }
  | { readonly step: 'refused'; readonly reason: 'scheme' | 'pin' }
  | { readonly step: 'reading' }
  | { readonly step: 'browser' };

/** One web tab: its mode and the pin it holds, which an accept replaces, and the step it is at. */
export interface WebTab {
  readonly mode: WebMode;
  readonly pin: Fingerprint | null;
  readonly current: WebStep;
}

export type WebTabEvent =
  | { readonly event: 'shell'; readonly shell: WebViewEvent }
  | { readonly event: 'read'; readonly presented: Fingerprint }
  | { readonly event: 'mismatch'; readonly expected: Fingerprint; readonly presented: Fingerprint }
  | { readonly event: 'failed'; readonly reason: string }
  | { readonly event: 'down' }
  | { readonly event: 'reload' }
  | { readonly event: 'confirm' }
  | { readonly event: 'back' }
  | { readonly event: 'accept' }
  | { readonly event: 'accepted'; readonly result: HubResult<WebCertificatePin> };

/** Fixed texts; the hub's and the shell's own messages are not shown (R-035, LOG-007). */
export const PROBLEM = {
  accept: 'The hub did not accept the certificate. Try again.',
  unreachable: 'The hub did not answer, so the certificate is not accepted. Try again.',
} as const;

function schemeOf(url: string): string | null {
  return URL.canParse(url) ? new URL(url).protocol : null;
}

/** D-6: `http` opens in the tab; `https` in the tab only where a web view can pin, else in the system browser. */
export function modeFor(url: string, pinning: boolean): WebMode | null {
  const scheme = schemeOf(url);
  if (scheme === 'http:') return 'EMBEDDED';
  if (scheme === 'https:') return pinning ? 'EMBEDDED' : 'SYSTEM_BROWSER';
  return null;
}

/** What a tab starts as. */
export interface WebTabStart {
  readonly url: string;
  /** The hub's pin for the interface, as K1 sends it. */
  readonly pinned: string | null;
  readonly pinning: boolean;
  readonly down: boolean;
}

function entry(mode: WebMode, pin: Fingerprint | null): WebStep {
  if (mode === 'EMBEDDED') return { step: 'loading' };
  return pin === null ? { step: 'reading' } : { step: 'browser' };
}

/** The tab for an interface at `url`; a URL that is not `http` or `https`, or an unreadable pin, opens nothing. */
export function startTab({ url, pinned, pinning, down }: WebTabStart): WebTab {
  const chosen = modeFor(url, pinning);
  const mode = chosen ?? 'SYSTEM_BROWSER';
  const pin = pinned === null ? null : fingerprintFrom(pinned);
  if (chosen === null) return { mode, pin, current: { step: 'refused', reason: 'scheme' } };
  if (pinned !== null && pin === null) return { mode, pin, current: { step: 'refused', reason: 'pin' } };
  return { mode, pin, current: down ? { step: 'down' } : entry(mode, pin) };
}

/** A presented fingerprint against the pin: none to compare, the pin, or a stop (WEB-008). */
function compared(pin: Fingerprint | null, presented: Fingerprint, unpinned: WebStep, same: WebStep | undefined): WebStep | undefined {
  if (pin === null) return unpinned;
  return pin === presented ? same : { step: 'mismatch', expected: pin, presented, confirming: false };
}

function heard(tab: WebTab, event: WebViewEvent): WebStep | undefined {
  const { current, pin } = tab;
  // A stop for a changed certificate wins over every other step (SEC-007).
  if (event.kind === 'MISMATCH') {
    return current.step === 'mismatch' ? undefined : { step: 'mismatch', expected: event.expected, presented: event.presented, confirming: false };
  }
  if (tab.mode !== 'EMBEDDED' || (current.step !== 'loading' && current.step !== 'loaded')) return undefined;
  switch (event.kind) {
    case 'LOADING':
      return { step: 'loading' };
    case 'LOADED':
      return { step: 'loaded', url: event.url };
    case 'UNTRUSTED':
      return compared(pin, event.fingerprintSha256, { step: 'untrusted', presented: event.fingerprintSha256 }, undefined);
    case 'FAILED':
      return { step: 'failed', reason: event.reason };
  }
}

function acceptProblem(result: HubResult<WebCertificatePin>): string {
  return !result.ok && result.error.code === 'unreachable' ? PROBLEM.unreachable : PROBLEM.accept;
}

/** The tab after the hub's answer to an accept of `presented`. */
function accepted(tab: WebTab, presented: Fingerprint, result: HubResult<WebCertificatePin>): WebTab {
  // The hub answers with the pin it now holds; anything but the fingerprint shown is not this accept.
  if (result.ok && fingerprintFrom(result.value.pinnedFingerprintSha256) === presented) {
    return { mode: tab.mode, pin: presented, current: tab.mode === 'EMBEDDED' ? { step: 'loading' } : { step: 'browser' } };
  }
  const problem = acceptProblem(result);
  if (tab.pin === null) return { ...tab, current: { step: 'untrusted', presented, problem } };
  return { ...tab, current: { step: 'mismatch', expected: tab.pin, presented, confirming: false, problem } };
}

function accepting(current: WebStep): WebStep | undefined {
  if (current.step === 'untrusted') return { step: 'accepting', presented: current.presented };
  return current.step === 'mismatch' && current.confirming ? { step: 'accepting', presented: current.presented } : undefined;
}

type StepEvent = Exclude<WebTabEvent, { event: 'accepted' }>;

/** The step after a confirm, a back or an accept: the operator's way through a prompt. */
function prompted(current: WebStep, event: Extract<StepEvent, { event: 'confirm' | 'back' | 'accept' }>): WebStep | undefined {
  if (event.event === 'accept') return accepting(current);
  if (current.step !== 'mismatch' || current.confirming === (event.event === 'confirm')) return undefined;
  return { ...current, confirming: event.event === 'confirm' };
}

/** The step after a failure, the device going Down, or a reload; a stop or a refusal stays. */
function interrupted(tab: WebTab, event: Extract<StepEvent, { event: 'failed' | 'down' | 'reload' }>): WebStep | undefined {
  const { step } = tab.current;
  switch (event.event) {
    case 'failed':
      return step === 'mismatch' || step === 'refused' ? undefined : { step: 'failed', reason: event.reason };
    case 'down':
      return ['mismatch', 'accepting', 'refused', 'down'].includes(step) ? undefined : { step: 'down' };
    case 'reload':
      return step === 'failed' || step === 'down' ? entry(tab.mode, tab.pin) : undefined;
  }
}

function stepFor(tab: WebTab, event: StepEvent): WebStep | undefined {
  const { current } = tab;
  switch (event.event) {
    case 'shell':
      return heard(tab, event.shell);
    case 'read':
      return current.step === 'reading' ? compared(tab.pin, event.presented, { step: 'untrusted', presented: event.presented }, { step: 'browser' }) : undefined;
    case 'mismatch':
      return current.step === 'mismatch' ? undefined : { step: 'mismatch', expected: event.expected, presented: event.presented, confirming: false };
    case 'failed':
    case 'down':
    case 'reload':
      return interrupted(tab, event);
    case 'confirm':
    case 'back':
    case 'accept':
      return prompted(current, event);
  }
}

/**
 * The tab `event` leads to, or undefined when it does not apply now. Answers apply only in the step that asked for
 * them, so a late answer or a second click does nothing; the pin changes only on the hub's accept.
 */
export function transition(tab: WebTab, event: WebTabEvent): WebTab | undefined {
  if (event.event === 'accepted') return tab.current.step === 'accepting' ? accepted(tab, tab.current.presented, event.result) : undefined;
  const current = stepFor(tab, event);
  return current === undefined ? undefined : { ...tab, current };
}

/** Whether the native web view may be shown: only while a page loads or is loaded, never over a prompt or a stop. */
export function viewShows(current: WebStep): boolean {
  return current.step === 'loading' || current.step === 'loaded';
}

/** The TLS status in the toolbar: no TLS, the system's trust, or the pinned fingerprint (WEB-005, WEB-008). */
export type TlsStatus = { readonly tls: 'plain' } | { readonly tls: 'trusted' } | { readonly tls: 'pinned'; readonly pin: Fingerprint };

/** The TLS status of `shown` for an interface at `url`; the pin counts only on the interface's own origin (D-4). */
export function tlsStatus(url: string, shown: string, pin: Fingerprint | null): TlsStatus {
  if (schemeOf(shown) !== 'https:') return { tls: 'plain' };
  const sameOrigin = URL.canParse(url) && new URL(url).origin === new URL(shown).origin;
  return pin !== null && sameOrigin ? { tls: 'pinned', pin } : { tls: 'trusted' };
}

/**
 * Whether opening `url` in the system browser needs the certificate read and compared first: an `https` URL on
 * the pinned origin. A different fingerprint stops the open (WEB-008).
 */
export function checksBeforeBrowser(interfaceUrl: string, url: string, pin: Fingerprint | null): boolean {
  return tlsStatus(interfaceUrl, url, pin).tls === 'pinned';
}

/** Whether "Open in system browser" may be used now: not while a certificate is unread, unaccepted or changed. */
export function browserAllowed(current: WebStep): boolean {
  return !['reading', 'untrusted', 'accepting', 'mismatch', 'refused'].includes(current.step);
}
