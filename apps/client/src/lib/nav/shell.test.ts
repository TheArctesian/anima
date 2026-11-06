// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, onTestFinished, vi } from 'vitest';
import type { Hub, HubStatus } from '$lib/hub/hub';
import Layout from '../../routes/+layout.svelte';
import Icon from '../icons/Icon.svelte';
import { runs } from '../motion/engine.fixture';
import { route } from './route.fixture.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../motion/engine.fixture')).fakeRun,
}));

const keyStore = vi.hoisted(() => ({ read: vi.fn(), available: vi.fn(() => false) }));
vi.mock('$lib/desktop/token', () => ({ readAccessToken: keyStore.read, keyStoreAvailable: keyStore.available }));

const router = vi.hoisted(() => ({ goto: vi.fn(() => Promise.resolve()) }));
vi.mock('$app/navigation', () => router);

const fake = vi.hoisted(() => ({
  options: undefined as { address: { origin: string }; token: (address: never) => Promise<string | null> } | undefined,
  status: { connection: 'connected', lastContactAt: null } as HubStatus,
  close: (): Promise<void> => Promise.resolve(),
}));

vi.mock('$app/state', async () => {
  const { route: current } = await import('./route.fixture.svelte');
  return {
    page: {
      get url() {
        return new URL(current.pathname, 'http://localhost');
      },
    },
  };
});

vi.mock('$lib/hub/hub', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/hub/hub')>()),
  createHub: (options: NonNullable<typeof fake.options>): Hub => {
    fake.options = options;
    return {
      address: options.address,
      status: {
        get current() {
          return fake.status;
        },
      },
      close: () => fake.close(),
    } as unknown as Hub;
  },
}));

const NOW = new Date('2026-11-04T10:00:00Z');
const CONNECTED: HubStatus = { connection: 'connected', lastContactAt: null };
let app: ReturnType<typeof mount> | undefined;

function show(pathname = '/devices', status: HubStatus = CONNECTED): HTMLElement {
  route.pathname = pathname;
  fake.status = status;
  const children = createRawSnippet(() => ({ render: () => '<p id="page">page</p>' }));
  app = mount(Layout, { target: document.body, props: { children } });
  flushSync();
  const nav = document.querySelector('nav');
  if (nav === null) throw new Error('no nav');
  return nav;
}

beforeEach(() => {
  keyStore.available.mockReset().mockReturnValue(false);
  keyStore.read.mockReset();
  router.goto.mockClear();
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

it('NAV-001: Devices comes first, then the divider, then the status, then Settings last', () => {
  const nav = show();
  const destinations = [...nav.querySelectorAll('a, hr, .status')];
  expect(destinations.map((node) => node.getAttribute('href') ?? node.textContent?.trim())).toEqual([
    '/devices', '', 'Connected', '/settings',
  ]);
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

it('UI-015: each item keeps its words as its name; the tooltip is set only when the bar shows icons', () => {
  const wide = show();
  expect([...wide.querySelectorAll('a')].map((link) => [link.querySelector('.words')?.textContent, link.getAttribute('title')])).toEqual([
    ['Devices', null],
    ['Settings', null],
  ]);
});

it('UI-015: below bp-compact each item has its words as a tooltip, and the status too', () => {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    value: (query: string) => ({ matches: query.includes('max-width'), addEventListener: () => undefined, removeEventListener: () => undefined }),
  });
  onTestFinished(() => {
    Reflect.deleteProperty(window, 'matchMedia');
    document.documentElement.removeAttribute('data-nav');
  });
  const nav = show();
  expect([...nav.querySelectorAll('a')].map((link) => [link.querySelector('.words')?.textContent, link.getAttribute('title')])).toEqual([
    ['Devices', 'Devices'],
    ['Settings', 'Settings'],
  ]);
  expect(nav.querySelector('.status')?.getAttribute('title')).toBe('Connected');
});

it('NET-004: when unreachable the banner has an icon besides its text, so the state is not colour alone', () => {
  const nav = show('/devices', { connection: 'unreachable', lastContactAt: null });
  expect(nav.querySelector('.banner .alert .icon')).not.toBeNull();
});

it('NAV-005: every link is focusable by Tab (tabindex 0 for WebKit)', () => {
  const links = [...show().querySelectorAll('a')];
  expect(links.map((link) => link.getAttribute('tabindex'))).toEqual(['0', '0']);
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

it('NET-004: no banner text while connected, but the status container is always there to announce one', () => {
  const notice = show().querySelector('[role="status"]');
  expect(notice).not.toBeNull();
  expect(notice?.textContent.trim()).toBe('');
});

it('MOT-007: the banner sits before the divider, so the divider, label and Settings keep their places', () => {
  const nav = show('/devices', { connection: 'unreachable', lastContactAt: null });
  const children = [...nav.children];
  const notice = children.findIndex((child) => child.getAttribute('role') === 'status');
  expect(notice).toBeGreaterThan(0);
  expect(notice).toBeLessThan(children.findIndex((child) => child.tagName === 'HR'));
  expect(children.at(-2)?.textContent).toBe('Disconnected');
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

it('MOT-042: the clock stops its timer when the layout is unmounted', async () => {
  show();
  expect(vi.getTimerCount()).toBeGreaterThan(0);
  await unmount(app as ReturnType<typeof mount>);
  app = undefined;
  expect(vi.getTimerCount()).toBe(0);
});

it('5.5: a route change makes a new page and the old one stays for the cross-fade', () => {
  // jsdom has no Web Animations; Svelte's transition runtime needs `animate` to exist.
  const animate = vi.fn(() => ({ cancel: vi.fn(), finish: vi.fn(), pause: vi.fn(), play: vi.fn(), currentTime: 0 }));
  Object.defineProperty(Element.prototype, 'animate', { value: animate, configurable: true });
  onTestFinished(() => {
    Reflect.deleteProperty(Element.prototype, 'animate');
  });
  show('/devices');
  const first = document.querySelector('.page');
  route.pathname = '/settings';
  flushSync();
  const pages = document.querySelectorAll('.page');
  expect(pages).toHaveLength(2);
  expect(pages[0]).toBe(first);
  expect(animate).toHaveBeenCalled();
});

it('CQ-045: unmounting the layout closes the hub, and a rejected close is handled without logging its message', async () => {
  const unhandled = vi.fn();
  process.on('unhandledRejection', unhandled);
  const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
  const close = vi.fn(() => Promise.reject(new Error('connect failed token=s3cret')));
  fake.close = close;
  show();
  await unmount(app as ReturnType<typeof mount>);
  app = undefined;
  await vi.advanceTimersByTimeAsync(10);
  expect(close).toHaveBeenCalledTimes(1);
  expect(warn).toHaveBeenCalledExactlyOnceWith(expect.any(String), 'Error');
  expect(unhandled).not.toHaveBeenCalled();
  process.off('unhandledRejection', unhandled);
  warn.mockRestore();
});

it('H-2: the hub is created with the stored address and the key store as its token provider', () => {
  localStorage.setItem('anima.hubAddress', 'https://my-hub.example');
  onTestFinished(() => {
    localStorage.clear();
  });
  show();
  expect(fake.options?.address.origin).toBe('https://my-hub.example');
  expect(fake.options?.token).toBe(keyStore.read);
});

it('H-2: with nothing stored the hub is the default one', () => {
  show();
  expect(fake.options?.address.origin).toBe('https://hub.anima.ing');
});

it('API-003, H-5: in the desktop app with no stored token the layout goes to /sign-in', async () => {
  keyStore.available.mockReturnValue(true);
  keyStore.read.mockResolvedValue(null);
  localStorage.setItem('anima.hubAddress', 'https://hub.example.org');
  onTestFinished(() => {
    localStorage.clear();
  });
  show('/devices');
  await vi.advanceTimersByTimeAsync(0);
  expect(keyStore.read).toHaveBeenCalledWith(expect.objectContaining({ origin: 'https://hub.example.org' }));
  expect(router.goto).toHaveBeenCalledWith('/sign-in');
});

it('H-5: on /sign-in the layout does not go anywhere', async () => {
  keyStore.available.mockReturnValue(true);
  keyStore.read.mockResolvedValue(null);
  show('/sign-in');
  await vi.advanceTimersByTimeAsync(0);
  expect(router.goto).not.toHaveBeenCalled();
});

it('D-4: outside the desktop app the layout does not go to /sign-in', async () => {
  keyStore.available.mockReturnValue(false);
  keyStore.read.mockResolvedValue(null);
  show('/devices');
  await vi.advanceTimersByTimeAsync(0);
  expect(router.goto).not.toHaveBeenCalled();
});

it('UI-020: the layout applies the stored fonts before the page shows, and Fira Code when nothing is stored', () => {
  localStorage.setItem('anima.typeface.ui', 'opendyslexic');
  onTestFinished(() => {
    localStorage.clear();
    delete document.documentElement.dataset['fontUi'];
    delete document.documentElement.dataset['fontMono'];
  });
  show();
  expect(document.documentElement.dataset['fontUi']).toBe('opendyslexic');
  expect(document.documentElement.dataset['fontMono']).toBe('fira-code');
});
