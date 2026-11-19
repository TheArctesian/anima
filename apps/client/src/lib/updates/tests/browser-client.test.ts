// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { BROWSER, DESKTOP_MAC } from '$lib/platform/answers.fixture';
import type { Capabilities } from '$lib/platform/platform';
import { isLocalBuild } from '../local-build';
import DesktopProbe from './DesktopProbe.svelte';

// A file of its own: the updater mock below counts imports of the module, and the module registry is per file, so
// no other test can have loaded it first.

const shell = vi.hoisted(() => ({ loads: 0, checks: 0 }));
const platform = vi.hoisted(() => ({ answers: undefined as Capabilities | undefined }));
vi.mock('$lib/platform/platform', () => ({ capabilities: () => platform.answers }));
vi.mock('$lib/desktop/updater', () => {
  shell.loads += 1;
  return {
    desktopUpdater: {
      check: () => {
        shell.checks += 1;
        return Promise.resolve(null);
      },
      restart: () => Promise.resolve(),
      currentVersion: () => Promise.resolve('1.0.0'),
    },
  };
});
vi.mock('$lib/desktop/process-log', () => ({ warnToProcessLog: () => undefined }));

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  vi.unstubAllEnvs();
});

it('UPD-001, CLI-001: with the browser answers a production build never checks and never loads the updater module', async () => {
  platform.answers = BROWSER;
  vi.stubEnv('DEV', false);
  app = mount(DesktopProbe, { target: document.body });
  flushSync();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(shell.checks).toBe(0);
  expect(shell.loads).toBe(0);
});

it('DEF-001: only the value that trial-bundle.sh sets makes a local build', () => {
  expect(isLocalBuild({ VITE_ANIMA_LOCAL_BUILD: '1' })).toBe(true);
  expect(isLocalBuild({})).toBe(false);
  expect(isLocalBuild({ VITE_ANIMA_LOCAL_BUILD: '' })).toBe(false);
  expect(isLocalBuild({ VITE_ANIMA_LOCAL_BUILD: '0' })).toBe(false);
});

it('UPD-001, DEF-001: a local build of the desktop app never checks the feed and never loads the updater module', async () => {
  platform.answers = DESKTOP_MAC;
  vi.stubEnv('DEV', false);
  vi.stubEnv('VITE_ANIMA_LOCAL_BUILD', '1');
  app = mount(DesktopProbe, { target: document.body });
  flushSync();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(shell.checks).toBe(0);
  expect(shell.loads).toBe(0);
  expect(document.querySelector('[data-update-state]')?.textContent).toBe('unchecked');
});

// Last: this test loads the updater module, which the tests above count.
it('UPD-001: a released build of the desktop app still checks the feed at start', async () => {
  platform.answers = DESKTOP_MAC;
  vi.stubEnv('DEV', false);
  app = mount(DesktopProbe, { target: document.body });
  flushSync();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(shell.checks).toBe(1);
});
