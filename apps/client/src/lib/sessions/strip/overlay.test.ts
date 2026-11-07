// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { Hub } from '$lib/hub/hub';
import { route } from '$lib/nav/route.fixture.svelte';
import Layout from '../../../routes/+layout.svelte';
import { overlayCovers } from '$lib/overlay/overlay.svelte';
import type { SessionPort } from '../sessions.svelte';
import {
  cleanup,
  prepare,
  request,
  settle,
  show as mountLayout,
} from './layout.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

// Records every web tab's props, so a test reads the `overlayOpen` the pane passes now.
const webTabs: { readonly overlayOpen: boolean }[] = [];
vi.mock('../web/WebTab.svelte', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../web/WebTab.svelte')>();
  return {
    default: (anchor: never, props: { readonly overlayOpen: boolean }) => {
      webTabs.push(props);
      return (
        actual.default as unknown as (anchor: never, props: unknown) => unknown
      )(anchor, props);
    },
  };
});

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

// The layout's sessions API is the fixture's fake hub; its port is spied on.
vi.mock('$lib/sessions/api', async (importOriginal) => {
  const { fake: state } = await import('./layout.fixture');
  return {
    ...(await importOriginal<typeof import('$lib/sessions/api')>()),
    sessionsApi: () => state.api,
  };
});

vi.mock('$lib/sessions/sessions.svelte', async (importOriginal) => {
  const actual =
    await importOriginal<typeof import('$lib/sessions/sessions.svelte')>();
  const { spiedSessions } = await import('./layout.fixture');
  return {
    ...actual,
    createSessions: (port: SessionPort) =>
      spiedSessions(actual.createSessions, port),
  };
});

beforeEach(() => {
  webTabs.length = 0;
  prepare();
  route.pathname = '/devices';
});

afterEach(cleanup);

const web = {
  id: 'wi-1',
  deviceId: 'web-1',
  url: 'http://printer.lan/',
  label: 'Printer',
  pinnedFingerprintSha256: null,
};
const popOut = () => Promise.resolve();
const covered = () => webTabs.at(-1)?.overlayOpen;
const liveControl = () => {
  const control = document.querySelector<HTMLElement>('[aria-controls$="-panel"]');
  if (control === null) throw new Error('no live sessions control');
  return control;
};
const menuControl = () => {
  const control = document.querySelector<HTMLElement>('[aria-haspopup="menu"]');
  if (control === null) throw new Error('no tab menu control');
  return control;
};

/** An SSH tab, then a web tab in front of it. */
function withWebTab() {
  const sessions = mountLayout(Layout, popOut);
  sessions.openSession(request);
  sessions.openWeb({
    deviceId: 'web-1',
    deviceName: 'nas',
    status: 'UP',
    web,
    origin: null,
  });
  flushSync();
  return sessions;
}

it('MAT-008, WEB-005: opening the Live sessions popover sets overlayOpen and closing it clears it', async () => {
  withWebTab();
  expect(covered()).toBe(false);
  liveControl().click();
  await settle();
  expect(covered()).toBe(true);
  liveControl().click();
  await settle();
  expect(covered()).toBe(false);
});

it('MAT-008, WEB-005: opening a session tab menu sets overlayOpen and closing it clears it', async () => {
  withWebTab();
  menuControl().click();
  await settle();
  expect(covered()).toBe(true);
  document.dispatchEvent(
    new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }),
  );
  window.dispatchEvent(
    new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }),
  );
  await settle();
  expect(covered()).toBe(false);
});

it('MAT-008, WEB-005: destroying the layout with a popover open lowers the count', async () => {
  withWebTab();
  liveControl().click();
  await settle();
  expect(overlayCovers()).toBe(true);
  await cleanup();
  expect(overlayCovers()).toBe(false);
});

it('MAT-008, WEB-005: opening, closing and opening again in one tick leaves the count at one', async () => {
  withWebTab();
  liveControl().click();
  liveControl().click();
  liveControl().click();
  await settle();
  expect(covered()).toBe(true);
  liveControl().click();
  await settle();
  expect(covered()).toBe(false);
});
