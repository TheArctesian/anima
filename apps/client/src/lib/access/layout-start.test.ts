// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FAKE_WEBSOCKET, FakeSocket } from '$lib/hub/socket/fake';
import { route as current } from '$lib/nav/route.fixture.svelte';
import Layout from '../../routes/+layout.svelte';
import { ORIGIN_REFUSED } from './start';

// The root layout's start (E-2, E-3): every route starts the operator shell and its start guard, and a page no hub
// served is refused with nothing sent (R-111, SEC-001). The browser's transports are spies, so any attempt shows. The
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
/** Whether the hub answers `me` with UNAUTHENTICATED; otherwise no request is ever answered. */
let signedOut = false;
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

const UNAUTHENTICATED = { errors: [{ message: 'Sign in.', extensions: { code: 'UNAUTHENTICATED' } }] };

const settled = () => new Promise((resolve) => setTimeout(resolve, 0));

beforeEach(() => {
  fetches.length = 0;
  signedOut = false;
  mocks.createHub.mockClear();
  mocks.goto.mockClear();
  FakeSocket.opened = [];
  vi.stubGlobal('WebSocket', FAKE_WEBSOCKET);
  vi.stubGlobal('fetch', (url: string, init: RequestInit) => {
    const query = (JSON.parse(String(init.body)) as { query: string }).query;
    const operation = /^\s*(?:query|mutation) (\w+)/.exec(query)?.[1] ?? 'anonymous';
    fetches.push(`${url} ${operation}`);
    if (signedOut && operation === 'Me') return Promise.resolve(new Response(JSON.stringify(UNAUTHENTICATED)));
    return new Promise<Response>(() => undefined);
  });
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

  it.each(['/shared', '/shared/x'])(
    'R-111, API-003: a signed-out visitor at %s gets the shell and is sent to sign-in',
    async (path) => {
      signedOut = true;
      const target = open(`https://hub.example.org${path}`);
      await settled();
      expect(target.querySelector('.shell [data-page]')).not.toBeNull();
      expect(mocks.createHub).toHaveBeenCalledOnce();
      expect(fetches).toContain('https://hub.example.org/graphql Me');
      expect(mocks.goto).toHaveBeenCalledExactlyOnceWith('/sign-in');
    },
  );

  it('E-3: a move between operator pages stays in the shell', async () => {
    const target = open('https://hub.example.org/devices');
    routerShows('https://hub.example.org/settings');
    await settled();
    expect(pagesShown(target)).toContain('/settings');
  });

  it.each(['/sign-in', '/sign-%69n'])('E-3: %s starts the shell, since its mutation needs the hub, and the guard does not ask me', async (path) => {
    open(`https://hub.example.org${path}`);
    await settled();
    expect(mocks.createHub).toHaveBeenCalledOnce();
    expect(fetches.filter((sent) => sent.endsWith(' Me'))).toEqual([]);
  });
});

describe('the root layout before sign-in', () => {
  it('NAV-009: the sign-in page shows alone: no bar, no live sessions, no hub status, no Settings, and a drag strip', async () => {
    const target = open('https://hub.example.org/sign-in');
    await settled();
    expect(target.querySelector('[data-page]')).not.toBeNull();
    expect(target.querySelector('nav, #main-nav, .shell')).toBeNull();
    expect(target.textContent).not.toMatch(/Devices|Live sessions|Settings|Connected/);
    expect(target.querySelector('.drag[data-tauri-drag-region]')).not.toBeNull();
  });

  it('NAV-009: the bar comes back once the router moves to an operator page', async () => {
    const target = open('https://hub.example.org/sign-in');
    routerShows('https://hub.example.org/devices');
    await settled();
    expect(target.querySelector('nav#main-nav')).not.toBeNull();
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
