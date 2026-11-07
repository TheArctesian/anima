// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { Hub } from '$lib/hub/hub';
import { runs } from '$lib/motion/engine.fixture';
import { route } from '$lib/nav/route.fixture.svelte';
import Layout from '../../../routes/+layout.svelte';
import type { PopOutPort, SessionPort } from '../sessions.svelte';
import { cleanup, fake, openStarted, prepare, settle, show as mountLayout, tabs } from './layout.fixture';

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

it('keyboard: the selected tab is the one Tab stop, the arrows move focus, Delete closes', async () => {
  const sessions = show();
  await openStarted(sessions);
  await openStarted(sessions);
  expect(tabs().map((tab) => tab.tabIndex)).toEqual([-1, 0]);
  tabs()[1]?.focus();
  tabs()[1]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
  expect(document.activeElement).toBe(tabs()[0]);
  tabs()[0]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Delete', bubbles: true }));
  await settle();
  expect(fake.port.end).toHaveBeenCalledExactlyOnceWith('session-1');
  expect(tabs()).toHaveLength(1);
});

it('S5 / ARIA: the tablist owns only tabs; Close and More are hidden from it and are not Tab stops', async () => {
  const sessions = show(vi.fn(() => Promise.resolve()));
  await openStarted(sessions);
  const strip = document.querySelector('[role="tablist"]');
  const exposed = [...(strip?.querySelectorAll('button') ?? [])].filter((button) => button.closest('[aria-hidden="true"]') === null);
  expect(exposed.map((button) => button.getAttribute('role'))).toEqual(['tab']);
  const others = [...(strip?.querySelectorAll<HTMLElement>('button[aria-hidden="true"]') ?? [])];
  expect(others.map((button) => button.tabIndex)).toEqual([-1, -1]);
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

it('UI-015: focus never rests inside the aria-hidden Close or More, on a press or a toggle-close', async () => {
  const sessions = show(vi.fn(() => Promise.resolve()));
  await openStarted(sessions);
  await openStarted(sessions);
  const hidden = () => document.activeElement?.closest('[aria-hidden="true"]') ?? null;
  const more = document.querySelector<HTMLElement>('[role="tablist"] [aria-haspopup="menu"]');
  more?.click();
  await settle();
  more?.focus();
  more?.click();
  await settle();
  expect(document.querySelector('[role="menu"]')).toBeNull();
  expect(hidden()).toBeNull();
  expect(document.activeElement?.getAttribute('role')).toBe('tab');
  const close = document.querySelector<HTMLElement>('button[aria-label="Close web-1 SSH session"]');
  const down = new PointerEvent('pointerdown', { bubbles: true, cancelable: true, button: 0 });
  runs.length = 0;
  close?.dispatchEvent(down);
  expect(down.defaultPrevented).toBe(true);
  // The press feedback still runs (MOT-001), and the drag ignores the press (data-no-drag).
  expect(runs.map((run) => run.keyframes.at(-1))).toEqual([0.97, 0.97]);
  expect(document.querySelector('[data-dragging]')).toBeNull();
  close?.focus();
  expect(hidden()).toBeNull();
  close?.click();
  await settle();
  expect(hidden()).toBeNull();
  expect(tabs()).toHaveLength(1);
});
