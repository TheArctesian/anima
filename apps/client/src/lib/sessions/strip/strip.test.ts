// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Hub } from '$lib/hub/hub';
import { runs } from '$lib/motion/engine.fixture';
import { route } from '$lib/nav/route.fixture.svelte';
import Layout from '../../../routes/+layout.svelte';
import type { PopOutPort, SessionPort } from '../sessions.svelte';
import { cleanup, fake, keyframes, openStarted, prepare, settle, show as mountLayout, tabs, visiblePanels } from './layout.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

vi.mock('$app/state', async () => {
  const { route } = await import('$lib/nav/route.fixture.svelte');
  return {
    navigating: { to: null },
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

it('MV-004: the strip keeps its space until it has left after the last tab closes', async () => {
  const sessions = show();
  const id = await openStarted(sessions);
  const strip = document.querySelector('[role="tablist"]');
  expect(strip?.hasAttribute('hidden')).toBe(false);
  await sessions.close(id);
  await settle();
  expect(strip?.hasAttribute('hidden')).toBe(false);
  for (const run of [...runs]) run.finish();
  await settle();
  expect(strip?.hasAttribute('hidden')).toBe(true);
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
