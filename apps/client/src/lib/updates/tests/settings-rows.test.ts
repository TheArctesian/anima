// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { BROWSER, DESKTOP_MAC } from '$lib/platform/answers.fixture';
import type { Capabilities } from '$lib/platform/platform';
import Page from '../../../routes/settings/+page.svelte';

const platform = vi.hoisted(() => ({ answers: undefined as Capabilities | undefined }));
vi.mock('$lib/platform/platform', () => ({ capabilities: () => platform.answers }));
vi.mock('$lib/access/SignOutRow.svelte', async () => ({ default: (await vi.importActual<{ default: unknown }>('./RowStub.svelte')).default }));
vi.mock('$lib/updates/UpdatesRow.svelte', async () => ({ default: (await vi.importActual<{ default: unknown }>('./RowStub.svelte')).default }));
vi.mock('$lib/process-log/LogFolderRow.svelte', async () => ({ default: (await vi.importActual<{ default: unknown }>('./LogRowStub.svelte')).default }));

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('UPD-006: the browser shows no Updates or log folder row', () => {
  platform.answers = BROWSER;
  app = mount(Page, { target: document.body });
  flushSync();
  expect(document.querySelectorAll('[data-stub]')).toHaveLength(1);
});

it('UPD-006, LOG-008: the desktop app shows the Updates and log folder rows after Sign out', () => {
  platform.answers = DESKTOP_MAC;
  app = mount(Page, { target: document.body });
  flushSync();
  expect(document.querySelectorAll('[data-stub]')).toHaveLength(3);
});

it('UPD-006: a client that updates itself but keeps no process log shows the Updates row alone', () => {
  platform.answers = { ...BROWSER, updates: true };
  app = mount(Page, { target: document.body });
  flushSync();
  expect(document.querySelectorAll('[data-stub]')).toHaveLength(2);
  expect(document.querySelector('[data-log-row]')).toBeNull();
});

it('LOG-008: a client that keeps a process log but does not update itself shows the log folder row alone', () => {
  platform.answers = { ...BROWSER, processLog: true };
  app = mount(Page, { target: document.body });
  flushSync();
  expect(document.querySelectorAll('[data-stub]')).toHaveLength(2);
  expect(document.querySelector('[data-log-row]')).not.toBeNull();
});
