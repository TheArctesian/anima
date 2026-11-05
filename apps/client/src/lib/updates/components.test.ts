// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { endTransitionsAtOnce } from '../devices/transitions.fixture';
import { fakeUpdater, offering, recordingLog, type FakeUpdater } from './updates.fixture';
import { Updates } from './updates.svelte';
import UpdateBanner from './UpdateBanner.svelte';
import UpdatesRow from './UpdatesRow.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../motion/engine.fixture')).fakeRun,
}));

const current = vi.hoisted(() => ({ updates: undefined as unknown }));
vi.mock('./updates.svelte', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./updates.svelte')>()),
  useUpdates: () => current.updates,
}));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

function show(component: typeof UpdateBanner, port: FakeUpdater = fakeUpdater()): { updates: Updates; port: FakeUpdater } {
  const updates = new Updates(port, recordingLog(), () => () => undefined);
  current.updates = updates;
  app = mount(component, { target: document.body });
  flushSync();
  return { updates, port };
}

const settle = async () => {
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
  flushSync();
};

const button = (name: string) => [...document.querySelectorAll('button')].find((b) => b.textContent.trim() === name);

it('UPD-004: nothing shows until an update is installed', () => {
  show(UpdateBanner);
  expect(document.querySelector('[role="status"]')).toBeNull();
});

it('UPD-004: an installed update shows a role=status banner with the version and a Restart button', async () => {
  const port = fakeUpdater();
  port.next = () => Promise.resolve(offering(port, '1.2.0'));
  const { updates } = show(UpdateBanner, port);
  await updates.checkNow();
  flushSync();
  const banner = document.querySelector('[role="status"]');
  expect(banner?.textContent).toContain('Anima 1.2.0 is installed');
  expect(button('Restart to update')).toBeDefined();
});

it('UPD-003: the banner restarts only when the button is pressed', async () => {
  const port = fakeUpdater();
  port.next = () => Promise.resolve(offering(port, '1.2.0'));
  const { updates } = show(UpdateBanner, port);
  await updates.checkNow();
  flushSync();
  expect(port.restarts).toBe(0);
  button('Restart to update')?.click();
  expect(port.restarts).toBe(1);
});

it('UPD-004: a restart that fails shows a short message in the banner, not a dialog', async () => {
  const port = fakeUpdater();
  port.next = () => Promise.resolve(offering(port, '1.2.0'));
  port.restart = () => Promise.reject(new Error('x'));
  const { updates } = show(UpdateBanner, port);
  await updates.checkNow();
  flushSync();
  button('Restart to update')?.click();
  await settle();
  expect(document.querySelector('[role="status"]')?.textContent).toContain('could not restart');
});

it('UPD-006: the row shows the current version, the state and an enabled Check for updates', async () => {
  show(UpdatesRow);
  await settle();
  expect(document.body.textContent).toContain('Anima 1.0.0');
  expect(document.body.textContent).toContain('Up to date');
  expect(button('Check for updates')?.disabled).toBe(false);
});

it('UPD-006: pressing Check for updates runs one check, disabled while it runs', async () => {
  const port = fakeUpdater();
  let finish: (() => void) | undefined;
  port.next = () => new Promise((resolve) => { finish = () => { resolve(null); }; });
  show(UpdatesRow, port);
  button('Check for updates')?.click();
  flushSync();
  expect(port.checks).toBe(1);
  expect(document.body.textContent).toContain('Checking…');
  expect(button('Check for updates')?.disabled).toBe(true);
  finish?.();
  await settle();
  expect(button('Check for updates')?.disabled).toBe(false);
});

it('UPD-006: a failed check is plain text in the row, with no dialog or alert', async () => {
  const port = fakeUpdater();
  port.next = () => Promise.reject(new Error('down'));
  show(UpdatesRow, port);
  button('Check for updates')?.click();
  await settle();
  expect(document.body.textContent).toContain('The last check failed');
  expect(document.querySelector('[role="alert"], [role="alertdialog"], dialog')).toBeNull();
});

it('UPD-006: once installed the row says restart and the button is disabled', async () => {
  const port = fakeUpdater();
  port.next = () => Promise.resolve(offering(port, '1.2.0'));
  const { updates } = show(UpdatesRow, port);
  await updates.checkNow();
  flushSync();
  expect(document.body.textContent).toContain('Anima 1.2.0 is installed — restart to update');
  expect(button('Check for updates')?.disabled).toBe(true);
});
