// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FAKE_WEBSOCKET, FakeSocket } from '$lib/hub/socket/fake';
import { route as current } from '$lib/nav/route.fixture.svelte';
import Layout from '../../routes/+layout.svelte';
import { ORIGIN_REFUSED } from './start';

// The root layout's start (E-3): a bare route renders only the page and creates no operator hub, so a share viewer
// opens no socket and sends no request (SEC-002, D-7). The browser's transports are spies, so any attempt shows. The
// router is a reactive stand-in that moves without any `beforeNavigate`, as SvelteKit does for a navigation started
// while another is in flight or for a `load` redirect.

const mocks = vi.hoisted(() => ({
  goto: vi.fn(() => Promise.resolve()),
  createHub: vi.fn(),
}));

// `current.pathname` holds the whole address, so a test can choose the origin too.
vi.mock('$app/state', () => ({
  page: {
    get url() {
      return new URL(current.pathname);
    },
  },
}));
vi.mock('$app/navigation', () => ({ goto: mocks.goto }));
vi.mock('$app/paths', () => ({ resolve: (path: string) => path, asset: (path: string) => path }));
vi.mock('$lib/hub/hub', async (importOriginal) => {
  const real = await importOriginal<typeof import('$lib/hub/hub')>();
  mocks.createHub.mockImplementation(real.createHub);
  return { ...real, createHub: mocks.createHub };
});

const fetches: string[] = [];
const reload = vi.fn();
let app: ReturnType<typeof mount> | undefined;

function open(href: string): HTMLElement {
  current.pathname = href;
  const target = document.createElement('div');
  document.body.append(target);
  // Each rendered page names the path it was rendered for, since an outgoing page stays for the cross-fade.
  const children = createRawSnippet(() => ({ render: () => `<p data-page="${new URL(current.pathname).pathname}">the page</p>` }));
  app = mount(Layout, { target, props: { children } });
  flushSync();
  return target;
}

/** The paths the rendered pages were rendered for. */
function pagesShown(target: HTMLElement): (string | null)[] {
  return [...target.querySelectorAll('[data-page]')].map((page) => page.getAttribute('data-page'));
}

/** The router shows `href` without asking first. */
function routerShows(href: string): void {
  current.pathname = href;
  flushSync();
}

const settled = () => new Promise((resolve) => setTimeout(resolve, 0));

beforeEach(() => {
  fetches.length = 0;
  mocks.createHub.mockClear();
  mocks.goto.mockClear();
  reload.mockClear();
  FakeSocket.opened = [];
  vi.stubGlobal('WebSocket', FAKE_WEBSOCKET);
  vi.stubGlobal('fetch', (url: string, init: RequestInit) => {
    const query = (JSON.parse(String(init.body)) as { query: string }).query;
    fetches.push(`${url} ${/^\s*(?:query|mutation) (\w+)/.exec(query)?.[1] ?? 'anonymous'}`);
    return new Promise<Response>(() => undefined);
  });
  vi.stubGlobal('location', { reload });
  // jsdom has no Web Animations; Svelte's transition runtime needs `animate` for the page cross-fade.
  const animation = { cancel: vi.fn(), finish: vi.fn(), pause: vi.fn(), play: vi.fn(), currentTime: 0 };
  Object.defineProperty(Element.prototype, 'animate', { value: () => animation, configurable: true });
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  await settled();
  vi.unstubAllGlobals();
  Reflect.deleteProperty(Element.prototype, 'animate');
});

describe('the root layout on a bare route', () => {
  it.each(['/shared', '/shared/', '/shared/abc', '/%73hared'])('SEC-002: %s renders only the page and creates no hub', async (path) => {
    const target = open(`https://hub.example.org${path}#share-token`);
    await settled();
    expect(target.querySelector('[data-page]')).not.toBeNull();
    expect(target.querySelector('nav, .shell, [role="alert"]')).toBeNull();
    expect(mocks.createHub).not.toHaveBeenCalled();
    expect(FakeSocket.opened).toEqual([]);
    expect(fetches).toEqual([]);
    expect(mocks.goto).not.toHaveBeenCalled();
  });

  it('E-3: when the router shows an operator page, it never renders bare and the whole client starts there', async () => {
    const target = open('https://hub.example.org/shared#share-token');
    routerShows('https://hub.example.org/devices');
    await settled();
    expect(pagesShown(target)).toEqual([]);
    expect(reload).toHaveBeenCalledOnce();
    expect(mocks.createHub).not.toHaveBeenCalled();
    expect(fetches).toEqual([]);
  });

  it('E-3: a move within the bare routes stays in the page', async () => {
    const target = open('https://hub.example.org/shared#share-token');
    routerShows('https://hub.example.org/shared/abc');
    await settled();
    // Bare, the page area is not keyed by path: the router swaps the page inside it.
    expect(pagesShown(target)).toHaveLength(1);
    expect(reload).not.toHaveBeenCalled();
  });
});

describe('the root layout on an operator route in a browser', () => {
  it('CLI-002: it creates one hub at the served origin and asks me at the start (API-003)', async () => {
    const target = open('https://hub.example.org/devices');
    await settled();
    expect(target.querySelector('.shell [data-page]')).not.toBeNull();
    expect(mocks.createHub).toHaveBeenCalledOnce();
    expect(FakeSocket.opened.map((socket) => socket.url)).toEqual(['wss://hub.example.org/graphql']);
    expect(fetches).toContain('https://hub.example.org/graphql Me');
    expect(fetches.every((sent) => sent.startsWith('https://hub.example.org/graphql '))).toBe(true);
  });

  it.each(['https://hub.example.org/shared#t', 'https://hub.example.org/%73hared#t'])(
    'E-3: when the router shows %s, the share page never renders in the shell and the whole client starts there',
    async (href) => {
      const target = open('https://hub.example.org/devices');
      routerShows(href);
      await settled();
      expect(pagesShown(target)).not.toContain(new URL(href).pathname);
      expect(reload).toHaveBeenCalledOnce();
    },
  );

  it('E-3: a move between operator pages stays in the shell', async () => {
    const target = open('https://hub.example.org/devices');
    routerShows('https://hub.example.org/settings');
    await settled();
    expect(pagesShown(target)).toContain('/settings');
    expect(reload).not.toHaveBeenCalled();
  });

  it('E-3: on a percent-encoded /sign-in the browser guard does not ask me', async () => {
    open('https://hub.example.org/sign-%69n');
    await settled();
    expect(mocks.createHub).toHaveBeenCalledOnce();
    expect(fetches.filter((sent) => sent.endsWith(' Me'))).toEqual([]);
  });
});

describe('the root layout on a page no hub served', () => {
  it('SEC-001: a plain http page from another host shows the fixed refusal and sends nothing', async () => {
    const target = open('http://hub.example.org/sign-in');
    await settled();
    expect(target.querySelector('[role="alert"]')?.textContent).toBe(ORIGIN_REFUSED);
    expect(target.querySelector('[data-page], nav, .shell')).toBeNull();
    expect(mocks.createHub).not.toHaveBeenCalled();
    expect(FakeSocket.opened).toEqual([]);
    expect(fetches).toEqual([]);
  });
});
