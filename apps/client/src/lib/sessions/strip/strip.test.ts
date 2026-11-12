// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Hub } from '$lib/hub/hub';
import { glyph } from '$lib/icons/icons';
import { runs } from '$lib/motion/engine.fixture';
import { route } from '$lib/nav/route.fixture.svelte';
import Layout from '../../../routes/+layout.svelte';
import type { PopOutPort, SessionPort } from '../sessions.svelte';
import { cleanup, fake, keyframes, openStarted, prepare, request, settle, show as mountLayout, tabs, visiblePanels } from './layout.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

vi.mock('$app/state', async () => {
  const { route } = await import('$lib/nav/route.fixture.svelte');
  return {
    page: {
      get url() {
        return new URL(route.pathname, 'http://localhost');
      },
    },
  };
});

vi.mock('$lib/hub/hub', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/hub/hub')>()),
  createHub: (): Hub =>
    ({
      address: { origin: 'https://hub.anima.ing', http: '', socket: '' },
      status: { current: { connection: 'connected', lastContactAt: null } },
      // The nav's live-session item asks the hub; nothing answers here.
      request: () => new Promise(() => undefined),
      subscribe: () => () => undefined,
      close: () => Promise.resolve(),
    }) as unknown as Hub,
}));

// The layout's sessions API is the fixture's fake hub; its port is spied on, and `popOut` is the test's port.
vi.mock('$lib/sessions/api', async (importOriginal) => {
  const { fake: state } = await import('./layout.fixture');
  return { ...(await importOriginal<typeof import('$lib/sessions/api')>()), sessionsApi: () => state.api };
});

vi.mock('$lib/sessions/sessions.svelte', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$lib/sessions/sessions.svelte')>();
  const { spiedSessions } = await import('./layout.fixture');
  return { ...actual, createSessions: (port: SessionPort) => spiedSessions(actual.createSessions, port) };
});

const show = (popOut?: PopOutPort) => mountLayout(Layout, popOut);

beforeEach(() => {
  prepare();
  runs.length = 0;
  route.pathname = '/devices';
});

afterEach(cleanup);

it('no tabs: the strip is hidden and the pages show', () => {
  show();
  expect(document.querySelector('[role="tablist"]')?.hasAttribute('hidden')).toBe(true);
  expect(visiblePanels()).toEqual([]);
});

it('RA-001: two opens of one device give two tabs with name and protocol; the newest is selected', async () => {
  const sessions = show();
  await openStarted(sessions);
  await openStarted(sessions);
  const label = (tab: HTMLElement) => [tab.querySelector('.name'), tab.querySelector('.protocol')].map((part) => part?.textContent);
  expect(tabs().map(label)).toEqual([
    ['web-1', 'SSH'],
    ['web-1', 'SSH'],
  ]);
  expect(tabs().map((tab) => tab.getAttribute('aria-selected'))).toEqual(['false', 'true']);
  expect(visiblePanels().map((panel) => panel.getAttribute('aria-labelledby'))).toEqual([tabs()[1]?.id]);
});

it('RA-001: switching tabs keeps both sessions mounted and shows only the selected one', async () => {
  const sessions = show();
  await openStarted(sessions);
  await openStarted(sessions);
  tabs()[0]?.click();
  flushSync();
  expect(tabs().map((tab) => tab.getAttribute('aria-selected'))).toEqual(['true', 'false']);
  expect(document.querySelectorAll('[role="tabpanel"] [aria-label="SSH session on web-1"]')).toHaveLength(2);
  expect(visiblePanels()).toHaveLength(1);
  expect(fake.port.end).not.toHaveBeenCalled();
});

it('RA-001 / UI-015: the labelled close button ends that session once and keeps the other tab', async () => {
  const sessions = show();
  await openStarted(sessions);
  await openStarted(sessions);
  const close = document.querySelector<HTMLButtonElement>('button[aria-label="Close web-1 SSH session"]');
  expect(close?.getAttribute('title')).toBe('Close web-1 SSH session');
  close?.click();
  await settle();
  expect(fake.port.end).toHaveBeenCalledExactlyOnceWith('session-1');
  expect(tabs()).toHaveLength(1);
  expect(tabs()[0]?.getAttribute('aria-selected')).toBe('true');
});

it('LOG-007: a failed end warns with the error name only, never its message', async () => {
  // `using` restores the spy when the test ends, even when an assertion throws.
  using warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
  const sessions = show();
  await openStarted(sessions);
  fake.port.end.mockRejectedValueOnce(new Error('secret-marker'));
  document.querySelector<HTMLButtonElement>('button[aria-label="Close web-1 SSH session"]')?.click();
  await settle();
  expect(warn).toHaveBeenCalledExactlyOnceWith('Ending the session failed:', 'Error');
  expect(JSON.stringify(warn.mock.calls)).not.toContain('secret-marker');
});

it('MOT-005: a new tab grows with transform-origin at the control that opened it', async () => {
  const sessions = show();
  await openStarted(sessions, new DOMRect(400, 300, 50, 20));
  const growing = keyframes.find((frames) => frames.some((frame) => 'transformOrigin' in frame));
  expect(growing?.[0]?.transformOrigin).toBe('425px 310px');
});

describe('layout mount (WP-14 E-2)', () => {
  it('the pages stay mounted, covered and inert, while a session tab is selected', async () => {
    const sessions = show();
    const page = document.querySelector('#page');
    await openStarted(sessions);
    expect(document.querySelector('#page')).toBe(page);
    // Svelte sets the `inert` property, which a browser reflects to the attribute (jsdom does not).
    const pages = page?.closest<HTMLElement>('.pages');
    expect(pages?.inert).toBe(true);
    expect(pages?.classList.contains('covered')).toBe(true);
    expect(visiblePanels()).toHaveLength(1);
  });

  it('D-11: the nav Devices item leaves the session tab open and shows the pages', async () => {
    const sessions = show();
    await openStarted(sessions);
    const devices = document.querySelector<HTMLElement>('nav a[href="/devices"]');
    // jsdom cannot follow the link; the router is not under test here.
    document.addEventListener(
      'click',
      (event) => {
        event.preventDefault();
      },
      { once: true },
    );
    devices?.click();
    flushSync();
    expect(sessions.active).toBeNull();
    expect(tabs()).toHaveLength(1);
    expect(visiblePanels()).toEqual([]);
    expect(document.querySelector<HTMLElement>('.pages')?.inert).toBe(false);
    expect(fake.port.end).not.toHaveBeenCalled();
  });
});

describe('pop out (RA-002, D-13)', () => {
  it('with no port, no menu and no "Pop out" are offered', async () => {
    const sessions = show();
    await openStarted(sessions);
    expect(document.querySelector('[role="tablist"] [aria-haspopup="menu"]')).toBeNull();
    expect(document.querySelector('[role="tablist"]')?.textContent).not.toContain('Pop out');
  });

  it('with a port, "Pop out" in the tab menu opens the device in a window once, the tab leaves and its session ends', async () => {
    const popOut = vi.fn(() => Promise.resolve());
    const sessions = show(popOut);
    await openStarted(sessions);
    const more = document.querySelector<HTMLButtonElement>('[role="tablist"] [aria-haspopup="menu"]');
    expect(more?.getAttribute('aria-label')).toBe('More for web-1 SSH session');
    more?.click();
    await settle();
    document.querySelector<HTMLButtonElement>('[role="menu"] [role="menuitem"]')?.click();
    await settle();
    expect(popOut).toHaveBeenCalledExactlyOnceWith({ deviceId: 'web-1', deviceName: 'web-1', protocol: 'SSH' });
    expect(tabs()).toEqual([]);
    expect(fake.port.end).toHaveBeenCalledExactlyOnceWith('session-1');
  });

  it('with a port, dragging a tab out of the strip calls it once with the device', async () => {
    const popOut = vi.fn(() => Promise.resolve());
    const sessions = show(popOut);
    await openStarted(sessions);
    const slot = document.querySelector<HTMLElement>('[data-slot]');
    if (slot === null) throw new Error('no tab');
    const at = (type: string, y: number) =>
      new PointerEvent(type, { bubbles: true, button: 0, pointerId: 3, clientX: 20, clientY: y });
    slot.querySelector('[role="tab"]')?.dispatchEvent(at('pointerdown', 10));
    slot.dispatchEvent(at('pointermove', 120));
    slot.dispatchEvent(at('pointerup', 200));
    await settle();
    expect(popOut).toHaveBeenCalledExactlyOnceWith({ deviceId: 'web-1', deviceName: 'web-1', protocol: 'SSH' });
  });

  it('Shift+F10 on a tab opens its menu, so the menu needs no Tab stop', async () => {
    const sessions = show(vi.fn(() => Promise.resolve()));
    await openStarted(sessions);
    tabs()[0]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'F10', shiftKey: true, bubbles: true }));
    await settle();
    expect(document.activeElement?.textContent).toBe('Pop out');
    expect(tabs()[0]?.getAttribute('aria-keyshortcuts')).toBe('Delete Shift+F10');
  });
});

// jsdom has no layout, so this shows the markers the CSS acts on, not pixels: the strip is hidden in the same update
// in which the pages show, so the page has no 32 px step to take and no tab paints over it (M-1, R-075).
it('MOT-007, R-075, 5.7.3: after the last tab closes the strip is hidden in the update in which the pages show', async () => {
  const sessions = show();
  const id = await openStarted(sessions);
  const strip = document.querySelector('[role="tablist"]');
  await sessions.close(id);
  await settle();
  expect(document.querySelector('.pages')?.hasAttribute('inert')).toBe(false);
  expect(strip?.hasAttribute('hidden')).toBe(true);
});

it('MOT-007, MOT-040: a tab opened again after the last one closed shows the strip', async () => {
  const sessions = show();
  const id = await openStarted(sessions);
  await sessions.close(id);
  await settle();
  await openStarted(sessions);
  expect(document.querySelector('[role="tablist"]')?.hasAttribute('hidden')).toBe(false);
});

it('MOT-007: the strip, not the page, carries the exit: no translate is written on an ancestor of the pages', async () => {
  const sessions = show();
  const id = await openStarted(sessions);
  await sessions.close(id);
  await settle();
  const pages = document.querySelector<HTMLElement>('.pages');
  for (let node: HTMLElement | null = pages; node !== null; node = node.parentElement) expect(node.style.translate).toBe('');
});

it('S4: closing a tab with none selected sends focus to the tab in its place; the last one to the nav', async () => {
  const sessions = show();
  await openStarted(sessions);
  await openStarted(sessions);
  sessions.deselect();
  flushSync();
  tabs()[0]?.focus();
  tabs()[0]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Delete', bubbles: true }));
  await settle();
  expect(document.activeElement).toBe(tabs()[0]);
  tabs()[0]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Delete', bubbles: true }));
  await settle();
  expect(document.activeElement).toBe(document.querySelector('nav a[aria-current="page"]'));
});

it('D-11: a route change (back, goto) shows the pages and keeps the tabs', async () => {
  const sessions = show();
  await openStarted(sessions);
  route.pathname = '/settings';
  flushSync();
  expect(sessions.active).toBeNull();
  expect(tabs()).toHaveLength(1);
});

it('K4 §8.7: a title the shell sets shows in the tab next to the device name, which stays; the full title is its tooltip', async () => {
  const sessions = show();
  const id = await openStarted(sessions);
  sessions.retitle(id, 'user@web-1: ~/src');
  await settle();
  const tab = tabs()[0];
  expect(tab?.querySelector('.name')?.textContent).toBe('web-1');
  expect(tab?.querySelector('.title')?.textContent).toBe('user@web-1: ~/src');
  expect(tab?.querySelector('.title')?.getAttribute('title')).toBe('user@web-1: ~/src');
  sessions.retitle(id, '');
  await settle();
  expect(tab?.querySelector('.title')?.textContent).toBe('');
});

it('UI-015: a tab opened with the device\'s OS shows its logo in place of the console icon, and its name stays the device and protocol', () => {
  const sessions = show();
  sessions.openSession({ ...request, os: 'MACOS', origin: null });
  sessions.openSession({ ...request, origin: null });
  flushSync();
  const glyphs = tabs().map((tab) => tab.querySelector('.icon')?.textContent);
  expect(glyphs).toEqual([glyph('nf-md-apple'), glyph('nf-md-console')]);
  expect(tabs()[0]?.querySelector('[role="img"]')).toBeNull();
});

it('MOT-007, MV-004: the tab has one width whatever the title, so a title moves neither the close button nor the protocol', () => {
  const css = readFileSync(`${import.meta.dirname}/SessionTabs.svelte`, 'utf8');
  expect(css).toMatch(/\.slot \{[^}]*\n\s+inline-size: min\(/);
  expect(css).not.toMatch(/\.slot \{[^}]*max-inline-size/);
  const tab = readFileSync(`${import.meta.dirname}/SessionTab.svelte`, 'utf8');
  expect(tab).toMatch(/\.title \{[^}]*flex: 1 1 0;/);
});

describe('RDP and VNC tabs (RA-001, Section 5.7 "Session tab open")', () => {
  it('MOT-007: an RDP or VNC tab names its protocol in the fixed-width label; an SSH tab keeps its plain text', () => {
    const sessions = show();
    for (const protocol of ['SSH', 'RDP', 'VNC'] as const) sessions.openSession({ ...request, protocol, origin: null });
    flushSync();
    const labels = tabs().map((tab) => tab.querySelector<HTMLElement>('.protocol'));
    expect(labels.map((label) => label?.textContent)).toEqual(['SSH', 'RDP', 'VNC']);
    expect(labels.map((label) => label?.hasAttribute('data-remote'))).toEqual([false, true, true]);
  });

  it.each(['RDP', 'VNC'] as const)('MOT-005: a new %s tab grows from the control that opened it, as an SSH tab does', (protocol) => {
    const sessions = show();
    sessions.openSession({ ...request, protocol, origin: new DOMRect(400, 300, 50, 20) });
    flushSync();
    const growing = keyframes.find((frames) => frames.some((frame) => 'transformOrigin' in frame));
    expect(growing?.[0]?.transformOrigin).toBe('425px 310px');
  });
});
