// @vitest-environment jsdom
import { mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { desktopWebView } from '$lib/desktop/web-view/web-view';
import { BROWSER, DESKTOP_MAC } from '$lib/platform/answers.fixture';
import type { Capabilities } from '$lib/platform/platform';
import { fakeApi, installLayout, settle } from '../tab/fake/shell.fixture.svelte';
import WebTab from '../tab/WebTab.svelte';
import { webViewShell } from './choice';
import { WebViewError, type WebViewShell } from './shell';

const platform = vi.hoisted(() => ({ answers: undefined as Capabilities | undefined, pinnings: 0 }));
vi.mock('$lib/platform/platform', () => ({ capabilities: () => platform.answers }));
// The real desktop adapter, counting its `pinning` calls so a test can see whether a tab used it.
vi.mock('$lib/desktop/web-view/web-view', async (importOriginal) => {
  const { desktopWebView: real } = await importOriginal<typeof import('$lib/desktop/web-view/web-view')>();
  const pinning = () => {
    platform.pinnings += 1;
    return real.pinning();
  };
  return { desktopWebView: { ...real, pinning } };
});

const OPENING = { tabId: 'tab-1', deviceId: 'device-1', url: 'https://nas.lan/', pin: null, bounds: { x: 0, y: 0, width: 1, height: 1 } };

/** What each call of `shell` comes to: `false`/`true` for an answer, the error code for a refusal. */
async function outcomes(shell: WebViewShell): Promise<unknown[]> {
  const calls: (() => Promise<unknown>)[] = [
    () => shell.pinning(),
    () => shell.open(OPENING),
    () => shell.place('tab-1', OPENING.bounds),
    () => shell.show('tab-1', true),
    () => shell.go('tab-1', 'RELOAD'),
    () => shell.pin('tab-1', 'ab'.repeat(32) as never),
    () => shell.popOut('tab-1'),
    () => shell.close('tab-1'),
    () => shell.openSystemBrowser('http://nas.lan/'),
    () => shell.readCertificate('https://nas.lan/'),
  ];
  return Promise.all(calls.map((call) => call().then((answer) => answer ?? 'done', (reason: unknown) => (reason instanceof WebViewError ? reason.code : 'other'))));
}

afterEach(() => {
  platform.pinnings = 0;
  document.body.innerHTML = '';
});

describe('webViewShell (WP-42 D-4)', () => {
  it('WEB-003: with web view tabs the client uses the desktop shell', () => {
    platform.answers = DESKTOP_MAC;
    expect(webViewShell()).toBe(desktopWebView);
  });

  it('CLI-001, WEB-011: without them every answer is what the desktop adapter gives without a shell', async () => {
    platform.answers = BROWSER;
    const absent = webViewShell();
    expect(absent).not.toBe(desktopWebView);
    // No shell is running in this test, so the desktop adapter shows what today's browser build does.
    expect(await outcomes(absent)).toEqual(await outcomes(desktopWebView));
    expect(await outcomes(absent)).toEqual([false, ...Array<string>(9).fill('failed')]);
    await expect(absent.listen(() => undefined)).rejects.toBeDefined();
    await expect(desktopWebView.listen(() => undefined)).rejects.toBeDefined();
  });
});

describe('WebTab default shell (WP-42 D-4)', () => {
  /** Mounts a tab, with `shell` when given, and returns the text of what it shows once it has settled. */
  async function shownBy(shell?: WebViewShell): Promise<string> {
    const cleanup = installLayout();
    const props = {
      webInterface: { id: 'web-1', deviceId: 'device-1', url: 'https://nas.lan/', label: 'DSM', pinnedFingerprintSha256: null },
      device: { id: 'device-1', name: 'nas', status: 'UP' as const },
      tabId: 'tab-1',
      visible: true,
      overlayOpen: false,
      api: fakeApi().api,
      ...(shell === undefined ? {} : { shell }),
    };
    const component = mount(WebTab, { target: document.body, props });
    await settle();
    const text = document.body.textContent;
    await unmount(component);
    cleanup();
    document.body.innerHTML = '';
    return text;
  }

  it('CLI-001: with the browser answers a tab never asks the desktop shell', async () => {
    platform.answers = BROWSER;
    await shownBy();
    expect(platform.pinnings).toBe(0);
  });

  it('WEB-008: with the desktop answers a tab asks the desktop shell whether it can pin', async () => {
    platform.answers = DESKTOP_MAC;
    await shownBy();
    expect(platform.pinnings).toBe(1);
  });

  it('CLI-001, WEB-010: with the browser answers a tab shows the page the desktop adapter shows without a shell', async () => {
    platform.answers = BROWSER;
    const today = await shownBy(desktopWebView);
    expect(await shownBy()).toBe(today);
    expect(today).toContain('could not be loaded');
  });
});
