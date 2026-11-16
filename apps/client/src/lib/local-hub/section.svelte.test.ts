// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { LocalHubResult, LocalHubStatus } from '$lib/desktop/local-hub/shell';
import { installAnimations } from '$lib/forms/animations.fixture';
import { LocalHubWatch, type LocalHubShell } from './local-hub.svelte';
import LocalHubSection from './LocalHubSection.svelte';

const mocks = vi.hoisted(() => ({ saveHubKey: vi.fn() }));
vi.mock('$lib/hub/context', () => ({ useHub: () => ({ address: { origin: 'http://127.0.0.1:8443' } }) }));
vi.mock('$lib/desktop/local-hub/shell', async (importOriginal) => ({ ...(await importOriginal<typeof import('$lib/desktop/local-hub/shell')>()), saveHubKey: mocks.saveHubKey }));
vi.mock('$lib/access/navigation', () => ({ loadClientAt: vi.fn() }));
vi.mock('$lib/desktop/token', () => ({ clearAccessToken: vi.fn() }));

const SECTION = { id: 'settings-local-hub', title: "This computer's hub", icon: 'carbon:bare-metal-server' as const, description: 'The hub.' };
const READY: LocalHubStatus = { installed: true, running: true, state: 'ready', hub: '127.0.0.1:8443', link: { state: 'up', since: '2026-11-15T10:00:00Z' } };
const STOPPED: LocalHubStatus = { installed: true, running: false, state: 'stopped' };
let app: ReturnType<typeof mount> | undefined;

function shell(...reads: LocalHubStatus[]): LocalHubShell & { start: ReturnType<typeof vi.fn>; stop: ReturnType<typeof vi.fn> } {
  const read = vi.fn<() => Promise<LocalHubResult<LocalHubStatus>>>();
  for (const status of reads) read.mockResolvedValueOnce({ ok: true, value: status });
  read.mockResolvedValue({ ok: true, value: reads.at(-1) ?? READY });
  return { read, start: vi.fn(() => Promise.resolve({ ok: true as const, value: null })), stop: vi.fn(() => Promise.resolve({ ok: true as const, value: null })), wait: () => Promise.resolve() };
}

async function settle(): Promise<void> {
  for (let round = 0; round < 4; round += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
    flushSync();
  }
}

async function open(watch: LocalHubWatch): Promise<HTMLElement> {
  const target = document.createElement('div');
  document.body.append(target);
  watch.watch();
  app = mount(LocalHubSection, { target, props: { section: SECTION, hubState: watch, mac: true } });
  await settle();
  return target;
}

function click(label: string): void {
  [...document.querySelectorAll('button')].find((button) => button.textContent.trim() === label)?.click();
  flushSync();
}

beforeEach(() => {
  installAnimations();
  mocks.saveHubKey.mockReset();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('FRN-006: with no hub installed the section is not shown', async () => {
  const target = await open(new LocalHubWatch(shell({ installed: false, running: false })));
  expect(target.querySelector('section')).toBeNull();
});

it('FRN-006, LHB-010, F-6: a ready hub shows its state and its link, and Stop', async () => {
  const watch = new LocalHubWatch(shell(READY));
  const target = await open(watch);
  expect(target.querySelector('h2')?.textContent).toBe("This computer's hub");
  expect(target.querySelector('.hint')?.textContent).toBe('Ready. The link is up: devices can join.');
  expect(watch.origin).toBe('http://127.0.0.1:8443');
});

it('FRN-006: Stop stops the hub and shows the stopped state; Start starts it and waits until it is ready', async () => {
  const hub = shell(READY, STOPPED, { ...READY, state: 'starting' }, READY);
  const target = await open(new LocalHubWatch(hub));
  click('Stop');
  await settle();
  expect(hub.stop).toHaveBeenCalledOnce();
  expect(target.querySelector('.hint')?.textContent).toContain('Stopped.');
  click('Start');
  await settle();
  expect(hub.start).toHaveBeenCalledOnce();
  expect(target.querySelector('.hint')?.textContent).toContain('Ready.');
});

it('2.8: "Save a copy of the hub key" opens the shell dialog and says where to keep it', async () => {
  mocks.saveHubKey.mockResolvedValue({ ok: true, value: true });
  const target = await open(new LocalHubWatch(shell(READY)));
  click('Save a copy of the hub key');
  await settle();
  expect(mocks.saveHubKey).toHaveBeenCalledOnce();
  expect(target.textContent).toContain('Saved. Keep it with your backups.');
});

it('FRN-007: Recover the account opens the account step of the first run', async () => {
  await open(new LocalHubWatch(shell(READY)));
  click('Recover the account');
  await settle();
  const headings = [...document.querySelectorAll('[role="dialog"] h2')].map((heading) => heading.textContent);
  expect(headings).toEqual(['Recover the account', 'A new name and password']);
});
