// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { APP_ID, fakeCalls, type FakeCalls } from './containers.fixture';
import ContainersPanel from './ContainersPanel.svelte';
import { ContainersStore } from './store.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let calls: FakeCalls;

beforeEach(() => {
  installAnimations();
  calls = fakeCalls();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function open(): Promise<ContainersStore> {
  const store = new ContainersStore(calls);
  const reading = store.load();
  app = mount(ContainersPanel, { target: document.body, props: { store } });
  await reading;
  flushSync();
  return store;
}

async function settle(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  flushSync();
}

const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find((each) => (each.getAttribute('aria-label') ?? each.textContent.trim()) === label);
const groupHeaders = () => [...document.querySelectorAll('tr.group th')].map((header) => header.textContent);

it('HOST-013: the table has a group for each Compose project with its running count, and a row for each container', async () => {
  await open();
  expect(groupHeaders()).toEqual(['ai1 of 1 running', 'tracks1 of 2 running', 'No Compose project1 of 1 running']);
  const rows = [...document.querySelectorAll('tbody tr:not(.group)')];
  expect(rows).toHaveLength(4);
  expect(rows[0]?.textContent).toContain('ollama');
  expect(rows[0]?.textContent).toContain('Running, unhealthy');
  expect(rows[2]?.textContent).toContain('Exited (1) 2 hours ago');
  expect(document.querySelector('.count')?.textContent).toBe('4 containers');
});

it('HOST-013: the log of a container is read only when the operator asks, with 200 lines', async () => {
  const store = await open();
  expect(calls.logsAsked).toEqual([]);
  button('Log of tracks-app-1')?.click();
  await settle();
  expect(calls.logsAsked).toEqual([[APP_ID, 200]]);
  expect(store.selected).toBe(APP_ID);
  expect(document.querySelector('.log h3')?.textContent).toBe('Log of tracks-app-1');
  expect([...document.querySelectorAll('.lines li span')].map((line) => line.textContent)).toEqual(['listening on :3000', '<b>ready</b>']);
  // A line of a container is text: it never becomes an element of the page.
  expect(document.querySelector('.lines b')).toBeNull();
  expect(button('Log of tracks-app-1')?.getAttribute('aria-pressed')).toBe('true');
});

it('HOST-001: the operator selects the number of lines, and the log is read again', async () => {
  await open();
  button('Log of tracks-app-1')?.click();
  await settle();
  document.querySelector<HTMLButtonElement>('[role="radio"][data-value="1000"]')?.click();
  await settle();
  expect(calls.logsAsked).toEqual([[APP_ID, 200], [APP_ID, 1000]]);
});

it('HOST-001: a log that was cut says so, and closing the log drops its lines', async () => {
  calls.logAnswer = { lines: [{ at: null, text: 'first' }], cut: true };
  const store = await open();
  button('Log of tracks-app-1')?.click();
  await settle();
  expect(document.querySelector('.cut')?.textContent).toContain('the newest lines are missing');
  button('Close the log')?.click();
  flushSync();
  expect(document.querySelector('.log')).toBeNull();
  expect(store.log).toBeNull();
});

it('HOST-001: a device the hub cannot read shows the reason of the hub and a way to try again', async () => {
  calls.fail = { code: 'CONFLICT', message: 'Docker was not read: the device has no SSH credential in Anima.' };
  await open();
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('the device has no SSH credential in Anima');
  expect(document.querySelector('table')).toBeNull();
  calls.fail = null;
  button('Retry')?.click();
  await settle();
  expect(calls.listsAsked).toBe(2);
  expect(document.querySelectorAll('tbody tr:not(.group)')).toHaveLength(4);
});

it('HOST-013: a device with no container says so', async () => {
  calls = fakeCalls([]);
  await open();
  expect(document.body.textContent).toContain('No containers');
  expect(document.querySelector('table')).toBeNull();
});

it('HOST-013: a failed log read keeps the table and offers a retry', async () => {
  await open();
  calls.fail = { code: 'UNAVAILABLE', message: 'Docker was not read: the device cannot be reached over SSH.' };
  button('Log of tracks-app-1')?.click();
  await settle();
  expect(document.querySelector('.log [role="alert"]')?.textContent).toContain('cannot be reached over SSH');
  expect(document.querySelectorAll('tbody tr:not(.group)')).toHaveLength(4);
});

it('HOST-013: when the selected container is gone after a refresh, its log closes', async () => {
  const store = await open();
  button('Log of tracks-app-1')?.click();
  await settle();
  calls.containers = calls.containers.filter((each) => each.id !== APP_ID);
  await store.load();
  flushSync();
  expect(store.selected).toBeNull();
  expect(document.querySelector('.log')).toBeNull();
});
