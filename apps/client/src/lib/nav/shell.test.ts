// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { Hub, HubStatus } from '$lib/hub/hub';
import Layout from '../../routes/+layout.svelte';
import Icon from '../icons/Icon.svelte';
import { runs } from '../motion/engine.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../motion/engine.fixture')).fakeRun,
}));

const fake = vi.hoisted(() => ({
  pathname: '/devices',
  status: { connection: 'connected', lastContactAt: null } as HubStatus,
  close: (): Promise<void> => Promise.resolve(),
}));

vi.mock('$app/state', () => ({
  page: {
    get url() {
      return new URL(fake.pathname, 'http://localhost');
    },
  },
}));

vi.mock('$lib/hub/hub', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/hub/hub')>()),
  createHub: (): Hub =>
    ({
      address: { origin: 'https://hub.anima.ing', http: '', socket: '' },
      status: {
        get current() {
          return fake.status;
        },
      },
      close: () => fake.close(),
    }) as unknown as Hub,
}));

const NOW = new Date('2026-11-04T10:00:00Z');
const CONNECTED: HubStatus = { connection: 'connected', lastContactAt: null };
let app: ReturnType<typeof mount> | undefined;

function show(pathname = '/devices', status: HubStatus = CONNECTED): HTMLElement {
  fake.pathname = pathname;
  fake.status = status;
  const children = createRawSnippet(() => ({ render: () => '<p id="page">page</p>' }));
  app = mount(Layout, { target: document.body, props: { children } });
  flushSync();
  const nav = document.querySelector('nav');
  if (nav === null) throw new Error('no nav');
  return nav;
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(NOW);
  fake.close = () => Promise.resolve();
  runs.length = 0;
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  vi.useRealTimers();
});

it('NAV-001: Devices comes first, then a divider, then the status, then Settings last', () => {
  const nav = show();
  const children = [...nav.children];
  expect(children.map((child) => child.tagName)).toEqual(['A', 'HR', 'P', 'A']);
  expect(children[0]?.textContent).toContain('Devices');
  expect(children[3]?.textContent).toContain('Settings');
});

it('NAV-002: only the active item has aria-current="page"', () => {
  const nav = show('/settings');
  const current = [...nav.querySelectorAll('[aria-current="page"]')];
  expect(current).toHaveLength(1);
  expect(current[0]?.getAttribute('href')).toBe('/settings');
});

it.each([
  ['connected', 'Connected'],
  ['connecting', 'Disconnected'],
  ['unreachable', 'Disconnected'],
] as const)('NAV-003: status %s shows %s directly above Settings', (connection, label) => {
  const nav = show('/devices', { connection, lastContactAt: null });
  expect(nav.querySelector('a[href="/settings"]')?.previousElementSibling?.textContent).toBe(label);
});

it('NAV-005: the items are real links to their pages, with the server and cog icons', () => {
  const nav = show();
  const links = [...nav.querySelectorAll('a')];
  expect(links.map((link) => link.getAttribute('href'))).toEqual(['/devices', '/settings']);
  expect(links.map((link) => link.querySelector('.icon')?.textContent)).toEqual(['\u{F048B}', '\u{F0493}']);
});

it('NET-004: when unreachable the banner shows the problem, the hub, the advice and the data age', () => {
  const lastContactAt = new Date(NOW.getTime() - 5 * 60_000);
  const nav = show('/devices', { connection: 'unreachable', lastContactAt });
  const text = nav.querySelector('[role="status"]')?.textContent ?? '';
  for (const part of ['Not on the network', 'https://hub.anima.ing', 'Check the VPN client', 'Data from 5 minutes ago']) {
    expect(text).toContain(part);
  }
});

it('NET-004: the data age moves on with the clock', () => {
  const lastContactAt = new Date(NOW.getTime() - 59 * 60_000);
  const nav = show('/devices', { connection: 'unreachable', lastContactAt });
  expect(nav.textContent).toContain('59 minutes ago');
  vi.advanceTimersByTime(120_000);
  flushSync();
  expect(nav.textContent).toContain('1 hour ago');
});

it('NET-004: no banner while connected', () => {
  expect(show().querySelector('[role="status"]')).toBeNull();
});

it('MOT-001: a nav item presses on pointerdown', () => {
  const nav = show();
  nav.querySelector('a')?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0 }));
  expect(runs.length).toBeGreaterThan(0);
});

it('UI-014: Icon renders hidden from assistive technology with one pinned glyph', async () => {
  const target = document.createElement('div');
  document.body.append(target);
  const icon = mount(Icon, { target, props: { name: 'nf-md-cog' } });
  const span = target.querySelector('span');
  expect(span?.getAttribute('aria-hidden')).toBe('true');
  expect(span?.textContent).toBe('\u{F0493}');
  await unmount(icon);
});

it('CQ-044: the layout shows the page inside the shell', () => {
  show();
  expect(document.querySelector('main #page')?.textContent).toBe('page');
});

it('CQ-045: unmounting the layout closes the hub, and a rejected close is handled', async () => {
  const unhandled = vi.fn();
  process.on('unhandledRejection', unhandled);
  const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
  const close = vi.fn(() => Promise.reject(new Error('connect failed')));
  fake.close = close;
  show();
  await unmount(app as ReturnType<typeof mount>);
  app = undefined;
  await vi.advanceTimersByTimeAsync(10);
  expect(close).toHaveBeenCalledTimes(1);
  expect(warn).toHaveBeenCalledOnce();
  expect(unhandled).not.toHaveBeenCalled();
  process.off('unhandledRejection', unhandled);
  warn.mockRestore();
});
