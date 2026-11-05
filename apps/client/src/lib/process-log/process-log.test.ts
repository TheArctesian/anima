// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { LogFolderError } from '$lib/desktop/processLog';
import LogFolderRow from './LogFolderRow.svelte';

const log = vi.hoisted(() => ({ open: vi.fn() }));
vi.mock('$lib/desktop/processLog', async (importOriginal) => ({
  ...(await importOriginal<typeof import('$lib/desktop/processLog')>()),
  openLogFolder: log.open,
}));

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  log.open.mockReset();
});

const settle = async () => {
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
  flushSync();
};

it('LOG-008: the button opens the log folder', async () => {
  log.open.mockResolvedValue(undefined);
  app = mount(LogFolderRow, { target: document.body });
  document.querySelector('button')?.click();
  await settle();
  expect(log.open).toHaveBeenCalledOnce();
  expect(document.querySelector('[role="alert"]')).toBeNull();
});

it('LOG-008: a failure shows a short inline message and no dialog', async () => {
  log.open.mockRejectedValue(new LogFolderError('failed'));
  app = mount(LogFolderRow, { target: document.body });
  document.querySelector('button')?.click();
  await settle();
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('Anima could not open the log folder.');
  expect(document.querySelector('dialog, [role="alertdialog"]')).toBeNull();
});
