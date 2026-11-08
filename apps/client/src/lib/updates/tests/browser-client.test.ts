// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import DesktopProbe from './DesktopProbe.svelte';

// A file of its own: the updater mock below counts imports of the module, and the module registry is per file, so
// no other test can have loaded it first.

const shell = vi.hoisted(() => ({ loads: 0, checks: 0 }));
vi.mock('$lib/platform/platform', async () => {
  const { BROWSER } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => BROWSER };
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
  vi.stubEnv('DEV', false);
  app = mount(DesktopProbe, { target: document.body });
  flushSync();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(shell.checks).toBe(0);
  expect(shell.loads).toBe(0);
});
