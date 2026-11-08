// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { BROWSER, DESKTOP_MAC } from '$lib/platform/answers.fixture';
import type { Capabilities } from '$lib/platform/platform';
import { openWebInterface, type WebOpening } from './open';

const platform = vi.hoisted(() => ({ answers: undefined as unknown as Capabilities }));
vi.mock('$lib/platform/platform', () => ({ capabilities: () => platform.answers }));

beforeEach(() => {
  platform.answers = DESKTOP_MAC;
});

const device = { id: 'd1', name: 'nas', status: 'DOWN' } as const;
const origin = { left: 1, top: 2, width: 3, height: 4 };

function opening(url: string, openIn: WebOpening['webInterface']['openIn']): WebOpening {
  return { device, origin, webInterface: { id: 'w1', deviceId: 'd1', url, label: 'Admin', openIn, pinnedFingerprintSha256: 'ab'.repeat(32) } };
}

function ports(browser: () => Promise<void> = () => Promise.resolve()) {
  return { sessions: { openWeb: vi.fn(() => 'tab-1') }, shell: { openSystemBrowser: vi.fn(browser) } };
}

describe('openWebInterface (WEB-003, WEB-011)', () => {
  it('WEB-003 / MOT-005: ANIMA_TAB opens a web tab for the device with its status, from the button', async () => {
    const { sessions, shell } = ports();
    await openWebInterface(sessions, shell, opening('http://nas.lan/', 'ANIMA_TAB'));
    expect(shell.openSystemBrowser).not.toHaveBeenCalled();
    expect(sessions.openWeb).toHaveBeenCalledExactlyOnceWith({
      deviceId: 'd1',
      deviceName: 'nas',
      status: 'DOWN',
      web: { id: 'w1', deviceId: 'd1', url: 'http://nas.lan/', label: 'Admin', pinnedFingerprintSha256: 'ab'.repeat(32) },
      origin,
    });
  });

  it('WEB-011: SYSTEM_BROWSER over http opens the system browser and no tab', async () => {
    const { sessions, shell } = ports();
    await openWebInterface(sessions, shell, opening('http://nas.lan/', 'SYSTEM_BROWSER'));
    expect(shell.openSystemBrowser).toHaveBeenCalledExactlyOnceWith('http://nas.lan/');
    expect(sessions.openWeb).not.toHaveBeenCalled();
  });

  it('WEB-008 / WEB-011: SYSTEM_BROWSER over https opens a web tab, which checks the pin', async () => {
    const { sessions, shell } = ports();
    await openWebInterface(sessions, shell, opening('https://nas.lan/', 'SYSTEM_BROWSER'));
    expect(shell.openSystemBrowser).not.toHaveBeenCalled();
    expect(sessions.openWeb).toHaveBeenCalledOnce();
  });

  it('WEB-011: a system browser that cannot be opened leaves the web tab to say why', async () => {
    const { sessions, shell } = ports(() => Promise.reject(new Error('no browser')));
    await openWebInterface(sessions, shell, opening('http://nas.lan/', 'SYSTEM_BROWSER'));
    expect(sessions.openWeb).toHaveBeenCalledOnce();
  });
});

/** Runs the tests of a describe with browser answers and a `window.open` that records its calls. */
function inBrowser(): typeof window.open {
  const opened = vi.fn<typeof window.open>(() => null);
  beforeEach(() => {
    platform.answers = BROWSER;
    vi.stubGlobal('open', opened);
  });
  afterEach(() => {
    opened.mockClear();
    vi.unstubAllGlobals();
  });
  return opened;
}

describe('openWebInterface in a browser (WEB-011, E-5)', () => {
  const opened = inBrowser();

  it.each([
    ['http://nas.example.net/', 'ANIMA_TAB'],
    ['https://nas.example.net/', 'ANIMA_TAB'],
    ['http://nas.example.net/', 'SYSTEM_BROWSER'],
    ['https://nas.example.net/', 'SYSTEM_BROWSER'],
  ] as const)('WEB-011: %s with %s opens one new tab with noopener,noreferrer and no Anima tab', async (url, openIn) => {
    const { sessions, shell } = ports();
    await openWebInterface(sessions, shell, opening(url, openIn));
    expect(opened).toHaveBeenCalledExactlyOnceWith(url, '_blank', 'noopener,noreferrer');
    expect(sessions.openWeb).not.toHaveBeenCalled();
    expect(shell.openSystemBrowser).not.toHaveBeenCalled();
  });

  it('WEB-011: window.open is called before the first await, inside the click', () => {
    const { sessions, shell } = ports();
    void openWebInterface(sessions, shell, opening('https://nas.example.net/', 'ANIMA_TAB'));
    expect(opened).toHaveBeenCalledOnce();
  });

  it.each(['javascript:alert(1)', 'data:text/html,x', 'file:///etc/hosts', 'not a url', ''])('WEB-011: %j opens nothing and does not throw', async (url) => {
    const { sessions, shell } = ports();
    await expect(openWebInterface(sessions, shell, opening(url, 'ANIMA_TAB'))).resolves.toBeUndefined();
    expect(opened).not.toHaveBeenCalled();
    expect(sessions.openWeb).not.toHaveBeenCalled();
  });

});

describe('openWebInterface addresses in a browser (WEB-011, LOG-007)', () => {
  const opened = inBrowser();

  it('WEB-011: https:evil opens the address that was checked, https://evil/, not a path on the page', async () => {
    const { sessions, shell } = ports();
    await openWebInterface(sessions, shell, opening('https:evil', 'ANIMA_TAB'));
    expect(opened).toHaveBeenCalledExactlyOnceWith('https://evil/', '_blank', 'noopener,noreferrer');
  });

  it.each(['httpx://nas/', 'JAVASCRIPT:alert(1)', 'ftp://nas.example.net/'])('WEB-011: %j opens nothing', async (url) => {
    const { sessions, shell } = ports();
    await openWebInterface(sessions, shell, opening(url, 'ANIMA_TAB'));
    expect(opened).not.toHaveBeenCalled();
  });

  it.each([
    [' https://nas.example.net/', 'https://nas.example.net/'],
    ['HTTPS://NAS.example.net/', 'https://nas.example.net/'],
  ])('WEB-011: %j opens as the normalised %j', async (url, href) => {
    const { sessions, shell } = ports();
    await openWebInterface(sessions, shell, opening(url, 'ANIMA_TAB'));
    expect(opened).toHaveBeenCalledExactlyOnceWith(href, '_blank', 'noopener,noreferrer');
  });

  it('LOG-007: nothing of the address reaches the console, opened or refused', async () => {
    const spies = (['log', 'info', 'warn', 'error', 'debug'] as const).map((level) => vi.spyOn(console, level).mockImplementation(() => undefined));
    const { sessions, shell } = ports();
    for (const url of ['https://nas.example.net/?token=secret', 'javascript:secret', 'not a url secret']) {
      await openWebInterface(sessions, shell, opening(url, 'ANIMA_TAB'));
    }
    const printed = spies.flatMap((spy) => spy.mock.calls).map((args) => args.map(String).join(' '));
    expect(printed.filter((line) => line.includes('secret'))).toEqual([]);
    for (const spy of spies) spy.mockRestore();
  });

  it('WEB-011: a null answer from window.open (noopener) is not a failure', async () => {
    const { sessions, shell } = ports();
    await openWebInterface(sessions, shell, opening('https://nas.example.net/', 'ANIMA_TAB'));
    expect(sessions.openWeb).not.toHaveBeenCalled();
  });
});

describe('openWebInterface on the desktop (WEB-011)', () => {
  it('WEB-011: the desktop never calls window.open', async () => {
    const opened = vi.fn<typeof window.open>(() => null);
    vi.stubGlobal('open', opened);
    const { sessions, shell } = ports();
    await openWebInterface(sessions, shell, opening('https://nas.lan/', 'ANIMA_TAB'));
    await openWebInterface(sessions, shell, opening('http://nas.lan/', 'SYSTEM_BROWSER'));
    expect(opened).not.toHaveBeenCalled();
    vi.unstubAllGlobals();
  });
});
