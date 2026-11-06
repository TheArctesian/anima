// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { displayFingerprint, fingerprintFrom, type Fingerprint } from '$lib/desktop/webView/webView';
import type { Status } from '$lib/hub/generated/graphql';
import { fakeApi, fakeShell, FakeResizeObserver, installLayout, settle } from './fake.fixture';
import { PROBLEM } from './tab';
import WebTab from './WebTab.svelte';

const PIN = fingerprintFrom('ab'.repeat(32)) as Fingerprint;
const NEW = fingerprintFrom('cd'.repeat(32)) as Fingerprint;
const BOUNDS = { x: 0, y: 48, width: 800, height: 552 };

let cleanups: (() => unknown)[] = [];

beforeEach(() => {
  cleanups = [installLayout()];
});

afterEach(async () => {
  for (const cleanup of cleanups.reverse()) await cleanup();
  document.body.innerHTML = '';
});

interface Shown {
  url?: string;
  pinned?: string | null;
  status?: Status;
  pinning?: boolean;
  certificate?: Fingerprint;
}

/** A web tab over a fake shell and hub; `props` changes visibility, overlays and the device status. */
async function showTab({ url = 'https://nas.lan:5001/', pinned = null, status = 'UP', pinning = true, certificate }: Shown = {}) {
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
const button = (name: string) =>
  [...document.querySelectorAll('button')].find(
    (candidate) => (candidate.getAttribute('aria-label') ?? candidate.textContent.replace(/[\u{F0000}-\u{FFFFD}]/gu, '').trim()) === name,
  );
const field = (name: string) => document.querySelector(`[data-${name}]`)?.textContent.trim();
const lastShow = (calls: { command: string; shown?: boolean }[]) => calls.filter(({ command }) => command === 'show').at(-1)?.shown;

describe('web tab view (WEB-004, MAT-008)', () => {
  it('WEB-004: opens the web view once at the measured area, places it on change, and closes it on destroy', async () => {
    const { calls, names, component } = await showTab({ url: 'http://printer.lan/' });
    expect(calls[0]).toEqual({ command: 'open', opening: { tabId: 'tab-1', deviceId: 'device-1', url: 'http://printer.lan/', pin: null, bounds: BOUNDS } });
    FakeResizeObserver.resize(BOUNDS);
    FakeResizeObserver.resize({ ...BOUNDS, width: 640 });
    FakeResizeObserver.resize({ ...BOUNDS, width: 600 });
    await settle();
    expect(calls.filter(({ command }) => command === 'place')).toEqual([{ command: 'place', bounds: { ...BOUNDS, width: 600 } }]);
    await unmount(component);
    await settle();
    expect(names().filter((name) => name === 'open')).toHaveLength(1);
    expect(names().at(-1)).toBe('close');
  });

  it('MAT-008: a hidden tab or an open overlay hides the view, and showing the tab again shows it', async () => {
    const { calls, props } = await showTab();
    expect(lastShow(calls)).toBe(true);
    props.visible = false;
    await settle();
    expect(lastShow(calls)).toBe(false);
    props.visible = true;
    props.overlayOpen = true;
    await settle();
    expect(lastShow(calls)).toBe(false);
    props.overlayOpen = false;
    await settle();
    expect(lastShow(calls)).toBe(true);
  });

});

describe('web tab toolbar and layout (WEB-005)', () => {
  it('Pop out moves the view to its own window, which closing the tab leaves open', async () => {
    const { names, component } = await showTab();
    button('Pop out')?.click();
    await settle();
    expect(names()).toContain('popOut');
    expect(document.body.textContent).toContain('DSM is open in its own window.');
    expect(button('Back')?.disabled).toBe(true);
    await unmount(component);
    await settle();
    expect(names()).not.toContain('close');
  });

  it('opens nothing while the tab has no area', async () => {
    const fake = fakeShell();
    FakeResizeObserver.rect = { ...BOUNDS, width: 0 };
    const component = mount(WebTab, {
      target: document.body,
      props: {
        webInterface: { id: 'web-1', deviceId: 'device-1', url: 'http://printer.lan/', label: 'Printer', pinnedFingerprintSha256: null },
        device: { id: 'device-1', name: 'printer', status: 'UP' },
        tabId: 'tab-1', visible: false, overlayOpen: false, shell: fake.shell, api: fakeApi().api,
      },
    });
    cleanups.push(() => unmount(component));
    await settle();
    expect(fake.names()).not.toContain('open');
  });

  it('WEB-005: the toolbar steps the history and shows the page and its TLS status', async () => {
    const { calls, emit } = await showTab({ url: 'http://printer.lan/' });
    emit({ tabId: 'tab-1', kind: 'LOADED', url: 'http://printer.lan/status' });
    emit({ tabId: 'tab-2', kind: 'LOADED', url: 'http://other.lan/' });
    flushSync();
    expect(field('url')).toBe('http://printer.lan/status');
    expect(field('tls')).toContain('Not encrypted');
    button('Back')?.click();
    button('Forward')?.click();
    button('Reload')?.click();
    await settle();
    expect(calls.filter(({ command }) => command === 'go')).toEqual([
      { command: 'go', to: 'BACK' },
      { command: 'go', to: 'FORWARD' },
      { command: 'go', to: 'RELOAD' },
    ]);
  });
});

describe('web tab certificates (WEB-008, SEC-007)', () => {
  it('WEB-008: a first visit shows the fingerprint; Accept sends it to the hub once, then pins it in the view', async () => {
    const { emit, calls, accepts } = await showTab();
    emit({ tabId: 'tab-1', kind: 'UNTRUSTED', fingerprintSha256: NEW });
    await settle();
    expect(lastShow(calls)).toBe(false);
    expect(field('presented')).toBe(displayFingerprint(NEW));
    expect(document.activeElement?.matches('section[aria-labelledby]')).toBe(true);
    const accept = button('Accept certificate');
    accept?.click();
    accept?.click();
    await settle();
    expect(accepts).toEqual([['web-1', NEW]]);
    expect(calls.filter(({ command }) => command === 'pin')).toEqual([{ command: 'pin', pin: NEW }]);
    expect(lastShow(calls)).toBe(true);
    expect(field('tls')).toContain('Pinned certificate');
  });

  it('R-035: a hub error keeps the prompt with fixed text, and nothing is pinned', async () => {
    const { emit, calls, control } = await showTab();
    control.answer = () => ({ ok: false, error: { code: 'VALIDATION', message: 'HUBTEXT', fields: [] } });
    emit({ tabId: 'tab-1', kind: 'UNTRUSTED', fingerprintSha256: NEW });
    await settle();
    button('Accept certificate')?.click();
    await settle();
    expect(document.body.textContent).toContain(PROBLEM.accept);
    expect(document.body.textContent).not.toContain('HUBTEXT');
    expect(button('Accept certificate')).toBeDefined();
    expect(calls.map(({ command }) => command)).not.toContain('pin');
  });

  it('SEC-007: a changed certificate stops the page with both fingerprints and pins nothing without two steps', async () => {
    const { emit, calls, accepts, names } = await showTab({ pinned: PIN });
    expect(calls[0]).toMatchObject({ opening: { pin: PIN } });
    emit({ tabId: 'tab-1', kind: 'MISMATCH', expected: PIN, presented: NEW });
    emit({ tabId: 'tab-1', kind: 'LOADED', url: 'https://nas.lan:5001/' });
    await settle();
    expect(lastShow(calls)).toBe(false);
    expect(field('expected')).toBe(displayFingerprint(PIN));
    expect(field('presented')).toBe(displayFingerprint(NEW));
    expect(document.body.textContent).toContain('has changed');
    button('Accept new certificate')?.click();
    button('Open in system browser')?.click();
    await settle();
    expect(accepts).toEqual([]);
    expect(names().filter((name) => name === 'open')).toHaveLength(1);
    expect(names()).not.toContain('pin');
    expect(names()).not.toContain('browser');
    expect(button('Yes, accept this certificate')).toBeDefined();
  });
});

describe('web tab Down page (WEB-010)', () => {
  it('WEB-010: a Down device shows the reason and Reload, and opens the page only on Reload', async () => {
    const { names } = await showTab({ status: 'DOWN' });
    expect(field('reason')).toContain('nas is Down');
    expect(names()).not.toContain('open');
    button('Reload')?.click();
    await settle();
    FakeResizeObserver.resize(BOUNDS);
    await settle();
    expect(names()).toContain('open');
  });

  it('WEB-010: a device that turns Down hides the view behind the Down page; Reload reloads it', async () => {
    const { props, calls } = await showTab();
    props.device = { ...props.device, status: 'DOWN' };
    await settle();
    expect(lastShow(calls)).toBe(false);
    expect(field('reason')).toContain('nas is Down');
    const reload = [...document.querySelectorAll('section button')].find((candidate) => candidate.textContent.includes('Reload'));
    (reload as HTMLButtonElement | undefined)?.click();
    await settle();
    expect(calls.at(-2)).toEqual({ command: 'go', to: 'RELOAD' });
    expect(lastShow(calls)).toBe(true);
  });

  it('a web view that does not open shows the error page once, and Reload tries again', async () => {
    const fake = fakeShell();
    fake.answers.refuse.add('open');
    const component = mount(WebTab, {
      target: document.body,
      props: {
        webInterface: { id: 'web-1', deviceId: 'device-1', url: 'http://printer.lan/', label: 'Printer', pinnedFingerprintSha256: null },
        device: { id: 'device-1', name: 'printer', status: 'UP' },
        tabId: 'tab-1', visible: true, overlayOpen: false, shell: fake.shell, api: fakeApi().api,
      },
    });
    cleanups.push(() => unmount(component));
    await settle();
    FakeResizeObserver.resize(BOUNDS);
    await settle();
    expect(fake.names()).toEqual(['open']);
    expect(field('code')).toBe('refused');
    fake.answers.refuse.clear();
    button('Reload')?.click();
    await settle();
    expect(fake.names()).toEqual(['open', 'open', 'show']);
  });

  it('a failed load shows the shell code and Reload', async () => {
    const { emit } = await showTab();
    emit({ tabId: 'tab-1', kind: 'FAILED', reason: 'unreachable' });
    await settle();
    expect(field('code')).toBe('unreachable');
  });
});

describe('web tab system-browser mode (D-6, D-9)', () => {
  it('D-9: shows the pinned fingerprint beside the button and opens the browser after the certificate matches', async () => {
    const { names, calls } = await showTab({ pinning: false, pinned: PIN, certificate: PIN });
    expect(field('fingerprint')).toBe(displayFingerprint(PIN));
    expect(names()).not.toContain('open');
    const open = [...document.querySelectorAll('section button')].find((candidate) => candidate.textContent.includes('Open in system browser'));
    (open as HTMLButtonElement | undefined)?.click();
    await settle();
    expect(calls.slice(-2)).toEqual([
      { command: 'read', url: 'https://nas.lan:5001/' },
      { command: 'browser', url: 'https://nas.lan:5001/' },
    ]);
  });

  it('D-9: a certificate other than the pin stops, and the browser never opens', async () => {
    const { names } = await showTab({ pinning: false, pinned: PIN, certificate: NEW });
    button('Open in system browser')?.click();
    await settle();
    expect(field('presented')).toBe(displayFingerprint(NEW));
    expect(document.body.textContent).toContain('has changed');
    button('Open in system browser')?.click();
    await settle();
    expect(names()).not.toContain('browser');
    expect(names()).not.toContain('open');
  });

  it('D-9: with no pin, the read fingerprint is shown; an accept pins it at the hub only, and the button opens', async () => {
    const { names, accepts } = await showTab({ pinning: false, certificate: NEW });
    expect(field('presented')).toBe(displayFingerprint(NEW));
    expect(button('Open in system browser')?.disabled).toBe(true);
    button('Accept certificate')?.click();
    await settle();
    expect(accepts).toEqual([['web-1', NEW]]);
    expect(field('fingerprint')).toBe(displayFingerprint(NEW));
    expect(names()).not.toContain('pin');
    expect(names()).not.toContain('open');
    button('Open in system browser')?.click();
    await settle();
    expect(names().slice(-2)).toEqual(['read', 'browser']);
  });
});
