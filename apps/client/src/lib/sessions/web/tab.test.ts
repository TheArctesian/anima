import { describe, expect, it } from 'vitest';
import { fingerprintFrom, type Fingerprint } from '$lib/desktop/webView/webView';
import type { HubResult } from '$lib/hub/hub';
import type { WebCertificatePin } from './api';
import { browserAllowed, checksBeforeBrowser, LOAD_TIMEOUT_MS, modeFor, PROBLEM, startTab, tlsStatus, transition, viewShows, type WebTab, type WebTabEvent } from './tab';

const PIN = fingerprintFrom('ab'.repeat(32)) as Fingerprint;
const NEW = fingerprintFrom('cd'.repeat(32)) as Fingerprint;
const HTTPS = 'https://nas.lan:5001/';

const start = (overrides: Partial<Parameters<typeof startTab>[0]> = {}) =>
  startTab({ url: HTTPS, pinned: null, pinning: true, down: false, now: 0, ...overrides });

/** The tab after `events` in order; an event that does not apply leaves it as it was. */
function after(tab: WebTab, ...events: WebTabEvent[]): WebTab {
  return events.reduce((current, event) => transition(current, event, 0) ?? current, tab);
}

const shell = (event: Record<string, unknown>): WebTabEvent => ({ event: 'shell', shell: { tabId: 't', ...event } as never });
const pinned = (fingerprint: string): HubResult<WebCertificatePin> => ({ ok: true, value: { id: 'w-1', pinnedFingerprintSha256: fingerprint, pinnedAt: '2026-11-06T10:00:00Z' } });

describe('mode choice (D-6)', () => {
  it('D-6: http opens in the tab; https in the tab only where a web view can pin', () => {
    expect(modeFor('http://printer.lan/', false)).toBe('EMBEDDED');
    expect(modeFor('http://printer.lan/', true)).toBe('EMBEDDED');
    expect(modeFor(HTTPS, true)).toBe('EMBEDDED');
    expect(modeFor(HTTPS, false)).toBe('SYSTEM_BROWSER');
    expect(modeFor('ftp://nas.lan/', true)).toBeNull();
  });

  it('D-6: a tab starts loading in the tab, or in system-browser mode reading or showing the pin', () => {
    expect(start().current).toEqual({ step: 'loading', since: 0 });
    expect(start({ pinning: false }).current).toEqual({ step: 'reading' });
    expect(start({ pinning: false, pinned: PIN })).toEqual({ mode: 'SYSTEM_BROWSER', pin: PIN, current: { step: 'browser' } });
  });

  it('opens nothing for a URL that is not http or https, or a pin it cannot read', () => {
    expect(start({ url: 'file:///etc/passwd' }).current).toEqual({ step: 'refused', reason: 'scheme' });
    expect(start({ pinned: PIN.toUpperCase() }).current).toEqual({ step: 'refused', reason: 'pin' });
    expect(transition(start({ url: 'file:///x' }), { event: 'reload' }, 0)).toBeUndefined();
    expect(transition(start({ url: 'file:///x' }), { event: 'down' }, 0)).toBeUndefined();
  });
});

describe('embedded tab (WEB-004, WEB-008)', () => {
  it('follows the page as it loads', () => {
    const tab = after(start(), shell({ kind: 'LOADED', url: `${HTTPS}home` }));
    expect(tab.current).toEqual({ step: 'loaded', url: `${HTTPS}home` });
    expect(after(tab, shell({ kind: 'LOADING', url: HTTPS })).current).toEqual({ step: 'loading', since: 0 });
  });

  it('WEB-008: a first visit to an untrusted certificate asks, and an accept the hub confirms pins it and loads', () => {
    const asked = after(start(), shell({ kind: 'UNTRUSTED', fingerprintSha256: NEW }));
    expect(asked.current).toEqual({ step: 'untrusted', presented: NEW });
    const sending = after(asked, { event: 'accept' });
    expect(sending.current).toEqual({ step: 'accepting', presented: NEW });
    expect(transition(sending, { event: 'accept' }, 0)).toBeUndefined();
    expect(after(sending, { event: 'accepted', result: pinned(NEW) })).toEqual({ mode: 'EMBEDDED', pin: NEW, current: { step: 'loading', since: 0 } });
  });

  it('WEB-008: a hub error, or a pin other than the one shown, keeps the prompt and pins nothing', () => {
    const sending = after(start(), shell({ kind: 'UNTRUSTED', fingerprintSha256: NEW }), { event: 'accept' });
    const refused = after(sending, { event: 'accepted', result: { ok: false, error: { code: 'INTERNAL', message: 'hub text' } } });
    expect(refused).toEqual({ mode: 'EMBEDDED', pin: null, current: { step: 'untrusted', presented: NEW, problem: PROBLEM.accept } });
    const unreachable = after(sending, { event: 'accepted', result: { ok: false, error: { code: 'unreachable', message: 'x' } } });
    expect(unreachable.current).toMatchObject({ step: 'untrusted', problem: PROBLEM.unreachable });
    expect(after(sending, { event: 'accepted', result: pinned(PIN) }).pin).toBeNull();
    expect(after(start(), { event: 'accepted', result: pinned(NEW) }).pin).toBeNull();
  });

  it('SEC-007: a changed certificate stops the page, from the shell or against the pin', () => {
    const tab = start({ pinned: PIN });
    expect(after(tab, shell({ kind: 'MISMATCH', expected: PIN, presented: NEW })).current).toEqual({ step: 'mismatch', expected: PIN, presented: NEW, confirming: false });
    expect(after(tab, shell({ kind: 'UNTRUSTED', fingerprintSha256: NEW })).current).toEqual({ step: 'mismatch', expected: PIN, presented: NEW, confirming: false });
    expect(transition(tab, shell({ kind: 'UNTRUSTED', fingerprintSha256: PIN }), 0)).toBeUndefined();
  });

  it('SEC-007: the stop holds: no load, failure, Down or reload leaves it, and accepting needs a second step', () => {
    const stopped = after(start({ pinned: PIN }), shell({ kind: 'MISMATCH', expected: PIN, presented: NEW }));
    for (const event of [shell({ kind: 'LOADED', url: HTTPS }), { event: 'failed', reason: 'x' }, { event: 'down' }, { event: 'reload' }, { event: 'accept' }] as const) {
      expect(transition(stopped, event, 0)).toBeUndefined();
    }
    const confirming = after(stopped, { event: 'confirm' });
    expect(confirming.current).toMatchObject({ step: 'mismatch', confirming: true });
    expect(after(confirming, { event: 'back' }).current).toMatchObject({ confirming: false });
    const replaced = after(confirming, { event: 'accept' }, { event: 'accepted', result: pinned(NEW) });
    expect(replaced).toEqual({ mode: 'EMBEDDED', pin: NEW, current: { step: 'loading', since: 0 } });
    const kept = after(confirming, { event: 'accept' }, { event: 'accepted', result: { ok: false, error: { code: 'CONFLICT', message: 'x' } } });
    expect(kept).toEqual({ mode: 'EMBEDDED', pin: PIN, current: { step: 'mismatch', expected: PIN, presented: NEW, confirming: false, problem: PROBLEM.accept } });
  });

  it('WEB-010: a failed load or a Down device shows the error page; Reload loads again', () => {
    const failed = after(start(), shell({ kind: 'FAILED', reason: 'unreachable' }));
    expect(failed.current).toEqual({ step: 'failed', reason: 'unreachable' });
    expect(after(failed, { event: 'reload' }).current).toEqual({ step: 'loading', since: 0 });
    const down = after(start(), shell({ kind: 'LOADED', url: HTTPS }), { event: 'down' });
    expect(down.current).toEqual({ step: 'down' });
    expect(after(down, { event: 'reload' }).current).toEqual({ step: 'loading', since: 0 });
    expect(start({ down: true }).current).toEqual({ step: 'down' });
    expect(after(start({ down: true, pinning: false }), { event: 'reload' }).current).toEqual({ step: 'reading' });
  });

});

describe('embedded tab errors (WEB-010)', () => {
  it('WEB-010: a page still loading after the timeout fails as timeout; a load that ends in time does not', () => {
    const loading = start({ now: 1_000 });
    expect(transition(loading, { event: 'tick' }, 1_000 + LOAD_TIMEOUT_MS - 1)).toBeUndefined();
    expect(transition(loading, { event: 'tick' }, 1_000 + LOAD_TIMEOUT_MS)?.current).toEqual({ step: 'failed', reason: 'timeout' });
    const loaded = transition(loading, shell({ kind: 'LOADED', url: HTTPS }), 2_000) as WebTab;
    expect(transition(loaded, { event: 'tick' }, 1_000_000)).toBeUndefined();
    const again = transition(loaded, shell({ kind: 'LOADING', url: HTTPS }), 50_000) as WebTab;
    expect(again.current).toEqual({ step: 'loading', since: 50_000 });
    expect(transition(again, shell({ kind: 'LOADING', url: `${HTTPS}next` }), 60_000)).toBeUndefined();
    const reloaded = transition(transition(loading, { event: 'tick' }, 40_000) as WebTab, { event: 'reload' }, 41_000);
    expect(reloaded?.current).toEqual({ step: 'loading', since: 41_000 });
  });

  it('ignores page events once a prompt or an error page is up', () => {
    const asked = after(start(), shell({ kind: 'UNTRUSTED', fingerprintSha256: NEW }));
    expect(transition(asked, shell({ kind: 'LOADED', url: HTTPS }), 0)).toBeUndefined();
    expect(transition(after(start(), { event: 'down' }), shell({ kind: 'LOADING', url: HTTPS }), 0)).toBeUndefined();
  });
});

describe('system-browser tab (D-9)', () => {
  const browser = (pin: Fingerprint | null) => start({ pinning: false, pinned: pin });

  it('D-9: with no pin, the read fingerprint is shown and an accept pins it beside the button', () => {
    const asked = after(browser(null), { event: 'read', presented: NEW });
    expect(asked.current).toEqual({ step: 'untrusted', presented: NEW });
    expect(after(asked, { event: 'accept' }, { event: 'accepted', result: pinned(NEW) })).toEqual({ mode: 'SYSTEM_BROWSER', pin: NEW, current: { step: 'browser' } });
  });

  it('D-9: a read fingerprint other than the pin stops; the same one goes on', () => {
    const tab = { ...browser(PIN), current: { step: 'reading' } } as const;
    expect(after(tab, { event: 'read', presented: NEW }).current).toEqual({ step: 'mismatch', expected: PIN, presented: NEW, confirming: false });
    expect(after(tab, { event: 'read', presented: PIN }).current).toEqual({ step: 'browser' });
    expect(after(browser(PIN), { event: 'mismatch', expected: PIN, presented: NEW }).current).toMatchObject({ step: 'mismatch' });
  });

  it('a read only applies while reading, and a failed read shows the error page', () => {
    expect(transition(browser(PIN), { event: 'read', presented: NEW }, 0)).toBeUndefined();
    expect(after(browser(null), { event: 'failed', reason: 'unreachable' }).current).toEqual({ step: 'failed', reason: 'unreachable' });
  });

  it('hears no page events: there is no web view', () => {
    expect(transition(browser(PIN), shell({ kind: 'LOADED', url: HTTPS }), 0)).toBeUndefined();
  });
});

describe('view and toolbar rules (MAT-008, WEB-005)', () => {
  it('MAT-008: the native view shows only while a page loads or is loaded', () => {
    expect(viewShows({ step: 'loading', since: 0 })).toBe(true);
    expect(viewShows({ step: 'loaded', url: HTTPS })).toBe(true);
    for (const step of ['untrusted', 'accepting', 'mismatch', 'failed', 'down', 'refused', 'reading', 'browser'] as const) {
      expect(viewShows({ step } as never)).toBe(false);
    }
  });

  it('WEB-005: the TLS status is no TLS, the pin on its origin, or system trust only for a loaded page', () => {
    const loaded = { step: 'loaded', url: HTTPS } as const;
    expect(tlsStatus('http://printer.lan/', 'http://printer.lan/x', null, loaded)).toEqual({ tls: 'plain' });
    expect(tlsStatus(HTTPS, `${HTTPS}x`, null, loaded)).toEqual({ tls: 'trusted' });
    expect(tlsStatus(HTTPS, `${HTTPS}x`, PIN, loaded)).toEqual({ tls: 'pinned', pin: PIN });
    expect(tlsStatus(HTTPS, 'https://other.lan/', PIN, loaded)).toEqual({ tls: 'trusted' });
    expect(tlsStatus(HTTPS, 'http://nas.lan:5001/', PIN, loaded)).toEqual({ tls: 'plain' });
  });

  it('SEC-007: the TLS status never claims trust the tab has not seen', () => {
    for (const step of [{ step: 'loading', since: 0 }, { step: 'failed', reason: 'x' }, { step: 'down' }, { step: 'reading' }] as const) {
      expect(tlsStatus(HTTPS, HTTPS, null, step)).toEqual({ tls: 'unknown' });
    }
    expect(tlsStatus(HTTPS, HTTPS, null, { step: 'untrusted', presented: NEW })).toEqual({ tls: 'untrusted' });
    expect(tlsStatus(HTTPS, HTTPS, null, { step: 'accepting', presented: NEW })).toEqual({ tls: 'untrusted' });
    expect(tlsStatus(HTTPS, HTTPS, PIN, { step: 'mismatch', expected: PIN, presented: NEW, confirming: false })).toEqual({ tls: 'changed' });
  });

  it('WEB-008: the system browser opens only after a check on the pinned origin, and never past a prompt or a stop', () => {
    expect(checksBeforeBrowser(HTTPS, `${HTTPS}x`, PIN)).toBe(true);
    expect(checksBeforeBrowser(HTTPS, `${HTTPS}x`, null)).toBe(false);
    expect(checksBeforeBrowser('http://printer.lan/', 'http://printer.lan/', null)).toBe(false);
    expect(browserAllowed({ step: 'loaded', url: HTTPS })).toBe(true);
    expect(browserAllowed({ step: 'browser' })).toBe(true);
    for (const step of ['reading', 'untrusted', 'accepting', 'mismatch', 'refused'] as const) {
      expect(browserAllowed({ step } as never)).toBe(false);
    }
  });
});
