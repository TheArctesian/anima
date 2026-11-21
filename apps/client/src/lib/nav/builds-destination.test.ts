// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { BROWSER, DESKTOP_MAC } from '$lib/platform/answers.fixture';
import type { Capabilities } from '$lib/platform/platform';
import Nav from './Nav.svelte';

const mocks = vi.hoisted(() => ({ platform: undefined as Capabilities | undefined }));

vi.mock('$lib/platform/platform', () => ({ capabilities: () => mocks.platform }));
vi.mock('$lib/hub/context', () => ({
  useHub: () => ({
    address: { origin: 'https://hub.example' },
    status: { current: { connection: 'connected', lastContactAt: null } },
    // The live-session item asks the hub; nothing answers here.
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

function links(platform: Capabilities, buildsOn = true): HTMLAnchorElement[] {
  mocks.platform = platform;
  app = mount(Nav, { target: document.body, props: { pathname: '/devices', now: new Date('2026-11-12T12:00:00Z'), buildsOn } });
  flushSync();
  return [...document.querySelectorAll('a')];
}

it('BLD-080: the desktop app has a Builds item between Devices and Settings', () => {
  const found = links(DESKTOP_MAC);
  expect(found.map((link) => link.getAttribute('href'))).toEqual(['/devices', '/threads', '/agents/usage', '/builds', '/extensions', '/settings']);
  expect(found[3]?.querySelector('.words')?.textContent).toBe('Builds');
});

it('PRO-038: the desktop app shows no Builds item for a hub that cannot publish', () => {
  expect(links(DESKTOP_MAC, false).map((link) => link.getAttribute('href'))).toEqual(['/devices', '/threads', '/agents/usage', '/extensions', '/settings']);
});

it('THR-017: the desktop application has the Threads item, above Agent usage', () => {
  expect(links(DESKTOP_MAC, false)[1]?.querySelector('.words')?.textContent).toBe('Threads');
});

it('THR-017: a browser has no Threads item', () => {
  expect(links(BROWSER).map((link) => link.getAttribute('href'))).not.toContain('/threads');
});

it('BLD-080: a browser has no Builds item', () => {
  expect(links(BROWSER).map((link) => link.getAttribute('href'))).toEqual(['/devices', '/agents/usage', '/extensions', '/settings']);
});
