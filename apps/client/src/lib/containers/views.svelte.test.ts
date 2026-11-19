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
let store: ContainersStore;

beforeEach(async () => {
  installAnimations();
  calls = fakeCalls();
  store = new ContainersStore(calls);
  const reading = store.load();
  app = mount(ContainersPanel, { target: document.body, props: { store } });
  await reading;
  flushSync();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function settle(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  flushSync();
}

const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find((each) => (each.getAttribute('aria-label') ?? each.textContent.trim()) === label);
const view = (value: string) => document.querySelector<HTMLButtonElement>(`[role="radio"][data-value="${value}"]`);
const rowTexts = () => [...document.querySelectorAll('tbody tr')].map((row) => [...row.children].map((cell) => cell.textContent.trim()));

it('HOST-017: the details of a container are read when the operator asks, and show names of the environment only', async () => {
  expect(calls.detailsAsked).toEqual([]);
  button('Details of tracks-app-1')?.click();
  await settle();
  expect(calls.detailsAsked).toEqual([APP_ID]);
  expect(document.querySelector('.detail h3')?.textContent).toBe('Details of tracks-app-1');
  const text = document.querySelector('.detail')?.textContent ?? '';
  for (const part of ['web-entrypoint.sh bin/rails server', 'on-failure:3', 'CMD-SHELL curl -f localhost:3000', 'volume tracks-data → /data (read-write)', 'tracks_default  172.18.0.3', 'DATABASE_PASSWORD', '805  root  bin/rails server', 'Anima never reads a value']) {
    expect(text).toContain(part);
  }
  // A label of a container is text: it never becomes an element of the page.
  expect(text).toContain('<i>tracks</i>');
  expect(document.querySelector('.detail i')).toBeNull();
  expect(button('Details of tracks-app-1')?.getAttribute('aria-pressed')).toBe('true');
  expect(button('Log of tracks-app-1')?.getAttribute('aria-pressed')).toBe('false');
});

it('HOST-017: the log replaces the details, and closing drops the details', async () => {
  button('Details of tracks-app-1')?.click();
  await settle();
  button('Log of tracks-app-1')?.click();
  await settle();
  expect(document.querySelector('.detail')).toBeNull();
  expect(document.querySelector('.log h3')?.textContent).toBe('Log of tracks-app-1');
  button('Details of tracks-app-1')?.click();
  await settle();
  button('Close the details')?.click();
  flushSync();
  expect(document.querySelector('.below')).toBeNull();
  expect(store.detail).toBeNull();
});

it('HOST-017: details the hub refuses show the reason of the hub and keep the table', async () => {
  calls.fail = { code: 'FORBIDDEN', message: 'An API key cannot use containerDetail.' };
  button('Details of tracks-app-1')?.click();
  await settle();
  expect(document.querySelector('.detail [role="alert"]')?.textContent).toContain('An API key cannot use containerDetail.');
  expect(document.querySelectorAll('tbody tr:not(.group)')).toHaveLength(4);
});

it('HOST-021: the hub looks for newer images only when the operator asks, and each row then has its mark', async () => {
  expect(calls.updatesAsked).toBe(0);
  expect(document.querySelector('.mark')).toBeNull();
  button('Look for newer images')?.click();
  await settle();
  expect(calls.updatesAsked).toBe(1);
  expect([...document.querySelectorAll('.mark')].map((mark) => [mark.textContent, mark.classList.contains('newer')])).toEqual([
    ['Newer image', true],
    ['Up to date', false],
    ['Built here, or the tag moved', false],
  ]);
  expect(document.querySelector('.count')?.textContent.replace(/\s+/g, ' ').trim()).toBe('4 containers, 1 with a newer image');
});

it('HOST-021: when the hub cannot look, the reason is shown and the table stays', async () => {
  calls.fail = { code: 'UNAVAILABLE', message: 'Docker was not read: the device cannot be reached over SSH.' };
  button('Look for newer images')?.click();
  await settle();
  expect(document.querySelector('.problem')?.textContent).toContain('cannot be reached over SSH');
  expect(document.querySelectorAll('tbody tr:not(.group)')).toHaveLength(4);
});

it('HOST-020: the images and the volumes are read once, when the operator first opens one of them', async () => {
  expect(calls.storageAsked).toBe(0);
  view('images')?.click();
  await settle();
  expect(calls.storageAsked).toBe(1);
  expect(rowTexts()[0]).toEqual(['example/tracks:0.36', '470MB', '49.49MB', 'tracks-app-1 and 2 more']);
  expect(document.querySelector('.count')?.textContent).toBe('2 images. 483.6MB in use, 13.6MB (2%) can be freed by a clean-up');
  view('volumes')?.click();
  await settle();
  expect(calls.storageAsked).toBe(1);
  expect(rowTexts()[1]).toEqual(['old-data', '1.2GB', 'local', 'Not used']);
  button('Refresh')?.click();
  await settle();
  expect(calls.storageAsked).toBe(2);
});

it('HOST-020: the networks have their own read, and a failed read offers a retry', async () => {
  calls.fail = { code: 'UNAVAILABLE', message: 'Docker was not read: docker did not succeed on the device.' };
  view('networks')?.click();
  await settle();
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('docker did not succeed');
  calls.fail = null;
  button('Retry')?.click();
  await settle();
  expect(calls.networksAsked).toBe(2);
  expect(calls.storageAsked).toBe(0);
  expect(rowTexts()[1]).toEqual(['backend', 'bridge', 'local', 'Inside the device only', 'None']);
  view('containers')?.click();
  flushSync();
  expect(document.querySelectorAll('tbody tr:not(.group)')).toHaveLength(4);
});
