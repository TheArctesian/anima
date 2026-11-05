// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import Page from './+page.svelte';

const desktop = vi.hoisted(() => ({ available: false }));
vi.mock('$lib/desktop/token', () => ({ keyStoreAvailable: () => desktop.available }));
vi.mock('$lib/access/SignOutRow.svelte', async () => ({ default: (await vi.importActual<{ default: unknown }>('./RowStub.svelte')).default }));
vi.mock('$lib/updates/UpdatesRow.svelte', async () => ({ default: (await vi.importActual<{ default: unknown }>('./RowStub.svelte')).default }));
vi.mock('$lib/process-log/LogFolderRow.svelte', async () => ({ default: (await vi.importActual<{ default: unknown }>('./RowStub.svelte')).default }));

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('UPD-006: the browser shows no Updates or log folder row', () => {
  desktop.available = false;
  app = mount(Page, { target: document.body });
  flushSync();
  expect(document.querySelectorAll('[data-stub]')).toHaveLength(1);
});

it('UPD-006, LOG-008: the desktop app shows the Updates and log folder rows after Sign out', () => {
  desktop.available = true;
  app = mount(Page, { target: document.body });
  flushSync();
  expect(document.querySelectorAll('[data-stub]')).toHaveLength(3);
});
