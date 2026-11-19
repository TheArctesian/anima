// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { isLocalBuild } from '../local-build';
import DesktopProbe from './DesktopProbe.svelte';

// A file of its own, as browser-client.test.ts: the updater mock counts imports of the module.

const shell = vi.hoisted(() => ({ loads: 0, checks: 0 }));
vi.mock('$lib/platform/platform', async () => {
  const { DESKTOP_MAC } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => DESKTOP_MAC };
});
vi.mock('$lib/desktop/updater', () => {
  shell.loads += 1;
  return {
    desktopUpdater: {
      check: () => {
        shell.checks += 1;
        return Promise.resolve(null);
      },
      restart: () => Promise.resolve(),
      currentVersion: () => Promise.resolve('0.2.2'),
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

const settled = () => new Promise((resolve) => setTimeout(resolve, 0));

it('DEF-001: only the value that trial-bundle.sh sets makes a local build', () => {
  expect(isLocalBuild({ VITE_ANIMA_LOCAL_BUILD: '1' })).toBe(true);
  expect(isLocalBuild({})).toBe(false);
  expect(isLocalBuild({ VITE_ANIMA_LOCAL_BUILD: '' })).toBe(false);
  expect(isLocalBuild({ VITE_ANIMA_LOCAL_BUILD: '0' })).toBe(false);
});

it('UPD-001, DEF-001: a local build of the desktop app never checks the feed and never loads the updater module', async () => {
  vi.stubEnv('DEV', false);
  vi.stubEnv('VITE_ANIMA_LOCAL_BUILD', '1');
  app = mount(DesktopProbe, { target: document.body });
  flushSync();
  await settled();
  expect(shell.checks).toBe(0);
  expect(shell.loads).toBe(0);
  expect(document.querySelector('[data-update-state]')?.textContent).toBe('unchecked');
});

it('UPD-001: a released build of the desktop app still checks the feed at start', async () => {
  vi.stubEnv('DEV', false);
  app = mount(DesktopProbe, { target: document.body });
  flushSync();
  await settled();
  expect(shell.checks).toBe(1);
});
