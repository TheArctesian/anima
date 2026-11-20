// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, onTestFinished, vi } from 'vitest';
import Nav from './Nav.svelte';

vi.mock('$lib/platform/platform', async () => {
  const { BROWSER } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => BROWSER };
});
vi.mock('$lib/hub/context', () => ({
  useHub: () => ({
    address: { origin: 'https://hub.example' },
    status: { current: { connection: 'connected', lastContactAt: null } },
    request: () => new Promise(() => undefined),
    subscribe: () => () => undefined,
  }),
}));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function hrefs(): (string | null)[] {
  app = mount(Nav, { target: document.body, props: { pathname: '/devices', now: new Date('2026-11-12T12:00:00Z') } });
  flushSync();
  return [...document.querySelectorAll('a')].map((link) => link.getAttribute('href'));
}

it('AIS-020: a wide window has Agent usage between Devices and Settings', () => {
  expect(hrefs()).toEqual(['/devices', '/agents/usage', '/extensions', '/settings']);
});

it('AIS-020, MOB-010: the phone bar keeps its four destinations, so Agent usage is not in it', () => {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    value: (query: string) => ({ matches: query.includes('max-width'), addEventListener: () => undefined, removeEventListener: () => undefined }),
  });
  onTestFinished(() => {
    Reflect.deleteProperty(window, 'matchMedia');
    document.documentElement.removeAttribute('data-nav');
  });
  expect(hrefs()).toEqual(['/devices', '/logs', '/settings']);
});
