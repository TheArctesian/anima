// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { fakeCalls, type FakeCalls } from '$lib/containers/containers.fixture';
import ContainersPanel from '$lib/containers/ContainersPanel.svelte';
import { ContainersStore } from '$lib/containers/store.svelte';
import { installAnimations } from '$lib/forms/animations.fixture';

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

async function settle(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  flushSync();
}

const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find((each) => (each.getAttribute('aria-label') ?? each.textContent.trim()) === label);

async function openCatalogue(): Promise<void> {
  const store = new ContainersStore(calls);
  app = mount(ContainersPanel, { target: document.body, props: { store } });
  flushSync();
  button('Catalogue')?.click();
  await settle();
}

const rows = () => [...document.querySelectorAll<HTMLButtonElement>('.entries button')];
const row = (name: string) => rows().find((each) => each.querySelector('.name')?.textContent.startsWith(name));

it('H39: the catalogue is read when the operator first opens it, in groups by category, each row with its tier', async () => {
  await openCatalogue();
  expect(calls.entriesAsked).toBe(1);
  const heads = [...document.querySelectorAll('.groups h4')].map((head) => head.textContent);
  expect(heads).toEqual(['Database1', 'Location1', 'Media servers1', 'Programming1']);
  expect(row('PostgreSQL')?.textContent).toContain('Tested by Anima');
  expect(row('PostgreSQL')?.textContent).toContain('Database');
  expect(row('jellyfin')?.textContent).toContain('Generated');
  expect(row('jellyfin')?.textContent).toContain('1 stream · 1 optional part');
  const shell = row('code-server')?.textContent ?? '';
  expect(shell).toContain('Shell');
  expect(shell).toContain('Needs a password that the machine makes. It has no public route.');
  expect(shell).toContain('Not reviewed');
  row('code-server')?.click();
  flushSync();
  const about = document.querySelector('section[aria-label="About code-server"]')?.textContent ?? '';
  expect(about).toContain('No person has read the settings of this entry. Read them before you install it.');
  expect(about).toContain('Not reviewed');
  expect(row('jellyfin')?.textContent).not.toContain('Not reviewed');
  expect(row('PostgreSQL')?.textContent).not.toContain('reviewed');
});

it('H39: the choice of the tier and the search keep only the entries that match; no match offers to clear', async () => {
  await openCatalogue();
  [...document.querySelectorAll<HTMLButtonElement>('[role="radio"]')].find((each) => each.textContent.startsWith('Generated'))?.click();
  flushSync();
  expect(rows().map((each) => each.querySelector('.name')?.textContent)).toEqual(['jellyfin', 'code-serverShellNot reviewed']);
  const search = document.querySelector<HTMLInputElement>('input[type="search"]');
  if (search === null) throw new Error('no search field');
  search.value = 'nothing like it';
  search.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  expect(document.body.textContent).toContain('No entry matches the search');
  button('Clear the search')?.click();
  flushSync();
  expect(rows()).toHaveLength(4);
});

it('H39: the description gives the stream with its protocols and the optional part with its state', async () => {
  await openCatalogue();
  row('jellyfin')?.click();
  flushSync();
  const about = document.querySelector('section[aria-label="About jellyfin"]');
  expect(about?.textContent).toContain('From LinuxServer.io, data of 2026-11-17. Anima did not test it.');
  expect(about?.textContent).toContain('Reviewed by Ada Lovelace on 2026-11-21');
  expect(about?.textContent).not.toContain('No person has read');
  expect(about?.textContent).toContain('7359TCP and UDP');
  expect(about?.textContent).toContain('Off by default');
  expect(about?.textContent).toContain('Needs machine setting');
  expect(about?.querySelector('[role="switch"]')?.getAttribute('aria-disabled')).toBe('true');
});

it('H39: the arrow keys move the selection from row to row', async () => {
  await openCatalogue();
  row('PostgreSQL')?.click();
  flushSync();
  row('PostgreSQL')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
  await settle();
  expect(document.querySelector('section[aria-label="About OwnTracks Recorder"]')).not.toBeNull();
  expect(document.activeElement).toBe(row('OwnTracks Recorder'));
});

it('H38: the install button is present and refused with the reason of the hub; a click asks nothing', async () => {
  await openCatalogue();
  row('PostgreSQL')?.click();
  flushSync();
  const install = button('Install on this device');
  expect(install?.getAttribute('aria-disabled')).toBe('true');
  const reason = document.getElementById(install?.getAttribute('aria-describedby') ?? '');
  expect(reason?.textContent).toContain('task H6');
  install?.click();
  await settle();
  expect(calls.entriesAsked).toBe(1);
  const about = document.querySelector('section[aria-label="About PostgreSQL"]');
  expect(about?.textContent).toContain('SECRET_KEY, PORT');
  expect(about?.querySelector('pre')?.textContent).toContain('services:');
});

it('H38: a list that the hub does not give shows its reason and a retry', async () => {
  calls.fail = { code: 'UNAVAILABLE', message: 'The hub did not answer.' };
  await openCatalogue();
  expect(document.body.textContent).toContain('The hub did not answer.');
});
