// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FAKE_WEBSOCKET, FakeSocket } from '$lib/hub/socket/fake';
import Layout from '../../routes/+layout.svelte';
import { ORIGIN_REFUSED } from './start';

// The root layout's start on a bare route (E-3): it renders only the page and creates no operator hub, so a share
// viewer opens no socket and sends no request (SEC-002, D-7). The browser's transports are spies, so any attempt shows.

const mocks = vi.hoisted(() => ({
  page: { url: new URL('https://hub.example.org/') },
  goto: vi.fn(() => Promise.resolve()),
  beforeNavigate: vi.fn(),
  createHub: vi.fn(),
}));

vi.mock('$app/state', () => ({ page: mocks.page }));
vi.mock('$app/navigation', () => ({ goto: mocks.goto, beforeNavigate: mocks.beforeNavigate }));
vi.mock('$app/paths', () => ({ resolve: (path: string) => path, asset: (path: string) => path }));
vi.mock('$lib/hub/hub', async (importOriginal) => {
  const real = await importOriginal<typeof import('$lib/hub/hub')>();
  mocks.createHub.mockImplementation(real.createHub);
  return { ...real, createHub: mocks.createHub };
});

const fetches: string[] = [];
let app: ReturnType<typeof mount> | undefined;

/** A navigation as `beforeNavigate` hands it to its callback. */
interface Crossing {
  readonly to: { readonly url: URL } | null;
  readonly willUnload: boolean;
  readonly cancel: () => void;
}

function open(href: string): HTMLElement {
  mocks.page.url = new URL(href);
  const target = document.createElement('div');
  document.body.append(target);
  const children = createRawSnippet(() => ({ render: () => '<p data-page>the page</p>' }));
  app = mount(Layout, { target, props: { children } });
  flushSync();
  return target;
}

/** The layout's `beforeNavigate` callback, run for a navigation to `href`; answers whether it was cancelled. */
function navigateTo(href: string, willUnload = false): { cancelled: boolean } {
  const callback = mocks.beforeNavigate.mock.calls[0]?.[0] as ((navigation: Crossing) => void) | undefined;
  if (callback === undefined) throw new Error('the layout registered no beforeNavigate');
  const outcome = { cancelled: false };
  callback({
    to: { url: new URL(href) },
    willUnload,
    cancel: () => {
      outcome.cancelled = true;
    },
  });
  return outcome;
}

beforeEach(() => {
  fetches.length = 0;
  mocks.createHub.mockClear();
  mocks.beforeNavigate.mockClear();
  mocks.goto.mockClear();
  FakeSocket.opened = [];
  vi.stubGlobal('WebSocket', FAKE_WEBSOCKET);
  vi.stubGlobal('fetch', (url: string, init: RequestInit) => {
    const query = (JSON.parse(String(init.body)) as { query: string }).query;
    fetches.push(`${url} ${/^\s*(?:query|mutation) (\w+)/.exec(query)?.[1] ?? 'anonymous'}`);
    return new Promise<Response>(() => undefined);
  });
  vi.stubGlobal('location', { assign: vi.fn() });
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  await new Promise((resolve) => setTimeout(resolve, 0));
  vi.unstubAllGlobals();
});

describe('the root layout on a bare route', () => {
  it.each(['/shared', '/shared/', '/shared/abc'])('SEC-002: %s renders only the page and creates no hub', async (path) => {
    const target = open(`https://hub.example.org${path}#share-token`);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(target.querySelector('[data-page]')).not.toBeNull();
    expect(target.querySelector('nav, .shell, [role="alert"]')).toBeNull();
    expect(mocks.createHub).not.toHaveBeenCalled();
    expect(FakeSocket.opened).toEqual([]);
    expect(fetches).toEqual([]);
    expect(mocks.goto).not.toHaveBeenCalled();
  });

  it('E-3: a navigation from /shared to an operator page loads the whole client there', () => {
    open('https://hub.example.org/shared#share-token');
    expect(navigateTo('https://hub.example.org/devices')).toEqual({ cancelled: true });
    expect(location.assign).toHaveBeenCalledWith('https://hub.example.org/devices');
  });

  it('E-3: a navigation within the bare routes stays in the page', () => {
    open('https://hub.example.org/shared#share-token');
    expect(navigateTo('https://hub.example.org/shared/other')).toEqual({ cancelled: false });
    expect(location.assign).not.toHaveBeenCalled();
  });
});

describe('the root layout on an operator route in a browser', () => {
  it('CLI-002: it creates one hub at the served origin and asks me at the start (API-003)', async () => {
    const target = open('https://hub.example.org/devices');
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(target.querySelector('.shell')).not.toBeNull();
    expect(mocks.createHub).toHaveBeenCalledOnce();
    expect(FakeSocket.opened.map((socket) => socket.url)).toEqual(['wss://hub.example.org/graphql']);
    expect(fetches).toContain('https://hub.example.org/graphql Me');
    expect(fetches.every((sent) => sent.startsWith('https://hub.example.org/graphql '))).toBe(true);
  });

  it('E-3: a navigation from an operator page to /shared loads the whole client there', () => {
    open('https://hub.example.org/devices');
    expect(navigateTo('https://hub.example.org/shared')).toEqual({ cancelled: true });
    expect(location.assign).toHaveBeenCalledWith('https://hub.example.org/shared');
    expect(navigateTo('https://hub.example.org/settings')).toEqual({ cancelled: false });
    expect(navigateTo('https://hub.example.org/shared', true)).toEqual({ cancelled: false });
    expect(location.assign).toHaveBeenCalledOnce();
  });
});

describe('the root layout on a page no hub served', () => {
  it('SEC-001: a plain http page from another host shows the fixed refusal and sends nothing', async () => {
    const target = open('http://hub.example.org/sign-in');
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(target.querySelector('[role="alert"]')?.textContent).toBe(ORIGIN_REFUSED);
    expect(target.querySelector('[data-page], nav, .shell')).toBeNull();
    expect(mocks.createHub).not.toHaveBeenCalled();
    expect(FakeSocket.opened).toEqual([]);
    expect(fetches).toEqual([]);
  });
});
