// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { HubOptions } from '$lib/hub/hub';
import { FAKE_WEBSOCKET, FakeSocket } from '$lib/hub/socket/fake';
import { route as current } from '$lib/nav/route.fixture.svelte';
import Layout from '../../../routes/+layout.svelte';
import { forgetListener } from './check';

// The F-10 wiring of the desktop start: on this computer's hub address the root layout's hub and the start guard read
// the key store only after the listener check. These tests fail when either is given `readAccessToken` directly.

const mocks = vi.hoisted(() => ({
  goto: vi.fn(() => Promise.resolve()),
  createHub: vi.fn(),
  readAccessToken: vi.fn(() => Promise.resolve('tok-secret')),
  listener: vi.fn(),
}));

vi.mock('$app/state', () => ({
  page: {
    get url() {
      return new URL(current.pathname);
    },
  },
}));
vi.mock('$app/navigation', () => ({ goto: mocks.goto }));
vi.mock('$app/paths', () => ({ resolve: (path: string) => path, asset: (path: string) => path }));
vi.mock('$lib/platform/platform', async () => {
  const { DESKTOP_MAC } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => ({ ...DESKTOP_MAC, builds: false, updates: false }) };
});
vi.mock('$lib/desktop/token', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/desktop/token')>()),
  readAccessToken: mocks.readAccessToken,
  readBuilderToken: () => Promise.resolve(null),
}));
vi.mock('$lib/desktop/hub-listener/shell', () => ({ checkHubListener: mocks.listener }));
vi.mock('$lib/desktop/link/shell', () => ({ readLink: () => Promise.resolve({ ok: false, problem: 'unavailable', message: '' }) }));
vi.mock('$lib/hub/hub', async (importOriginal) => {
  const real = await importOriginal<typeof import('$lib/hub/hub')>();
  mocks.createHub.mockImplementation(real.createHub);
  return { ...real, createHub: mocks.createHub };
});

let app: ReturnType<typeof mount> | undefined;
const settled = async () => {
  for (let round = 0; round < 3; round += 1) await new Promise((resolve) => setTimeout(resolve, 0));
};

beforeEach(() => {
  mocks.goto.mockClear();
  mocks.createHub.mockClear();
  mocks.readAccessToken.mockClear();
  mocks.listener.mockReset();
  FakeSocket.opened = [];
  vi.stubGlobal('WebSocket', FAKE_WEBSOCKET);
  vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => undefined)));
  Object.defineProperty(Element.prototype, 'animate', { value: () => ({ cancel: vi.fn(), finish: vi.fn() }), configurable: true });
  localStorage.setItem('anima.hubAddress', 'http://127.0.0.1:8443');
  // The page's gate keeps an answer for a few seconds; each test starts with none.
  forgetListener('http://127.0.0.1:8443');
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  localStorage.clear();
  vi.unstubAllGlobals();
  Reflect.deleteProperty(Element.prototype, 'animate');
});

function open(): void {
  current.pathname = 'http://tauri.localhost/devices';
  const target = document.createElement('div');
  document.body.append(target);
  const children = createRawSnippet(() => ({ render: () => '<p>the page</p>' }));
  app = mount(Layout, { target, props: { children } });
  flushSync();
}

it.each(['other', 'none'] as const)('F-10: with %s listening, the start guard never reads the key store and goes to sign-in', async (listener) => {
  mocks.listener.mockResolvedValue({ ok: true, listener });
  open();
  await settled();
  expect(mocks.listener).toHaveBeenCalledWith('http://127.0.0.1:8443');
  expect(mocks.readAccessToken).not.toHaveBeenCalled();
  expect(mocks.goto).toHaveBeenCalledWith('/sign-in');
});

it("F-10: the layout's hub reads its token through the gate: refused, the key store is never read", async () => {
  mocks.listener.mockResolvedValue({ ok: true, listener: 'other' });
  open();
  const options = mocks.createHub.mock.calls[0]?.[0] as HubOptions;
  await expect(options.token(options.address)).rejects.toThrow();
  expect(mocks.readAccessToken).not.toHaveBeenCalled();
});

it("F-10: with this user's own program listening, the token is read", async () => {
  mocks.listener.mockResolvedValue({ ok: true, listener: 'own' });
  open();
  await settled();
  expect(mocks.readAccessToken).toHaveBeenCalled();
  expect(mocks.goto).not.toHaveBeenCalledWith('/sign-in');
});
