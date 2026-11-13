// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import Guard from '../../routes/projects/+layout.svelte';
import { extensionsApi } from './api';
import { fakeHub } from './extensions.fixture';
import { ExtensionsStore } from './store.svelte';

const mocks = vi.hoisted(() => ({ goto: vi.fn(() => Promise.resolve()), store: undefined as unknown }));

vi.mock('$app/navigation', () => ({ goto: mocks.goto }));
vi.mock('$app/paths', () => ({ resolve: (path: string) => path }));
vi.mock('./store.svelte', async (importOriginal) => ({ ...(await importOriginal<typeof import('./store.svelte')>()), useExtensions: () => mocks.store }));

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  mocks.goto.mockClear();
  document.body.replaceChildren();
});

async function open(on: boolean): Promise<void> {
  const hub = fakeHub();
  const store = new ExtensionsStore(extensionsApi(hub));
  await store.load();
  if (on) await store.set('projects', true);
  mocks.store = store;
  const children = createRawSnippet(() => ({ render: () => '<p>The Projects page</p>' }));
  app = mount(Guard, { target: document.body, props: { children } });
  flushSync();
}

it('EXT-002: /projects goes to /devices while Projects is off', async () => {
  await open(false);
  expect(mocks.goto).toHaveBeenCalledWith('/devices', { replaceState: true });
  expect(document.body.textContent).not.toContain('The Projects page');
});

it('EXT-002: /projects shows its page while Projects is on', async () => {
  await open(true);
  expect(mocks.goto).not.toHaveBeenCalled();
  expect(document.body.textContent).toContain('The Projects page');
});
