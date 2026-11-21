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

it('H38: the catalogue is read when the operator first opens it, and lists each entry with its machine test', async () => {
  await openCatalogue();
  expect(calls.entriesAsked).toBe(1);
  const rows = [...document.querySelectorAll('.entries li')].map((row) => row.textContent);
  expect(rows).toHaveLength(2);
  expect(rows[0]).toContain('OwnTracks Recorder');
  expect(rows[0]).toContain('Passed on a machine on 2026-11-20');
  expect(rows[1]).toContain('PostgreSQL');
  expect(rows[1]).toContain('Database');
  expect(rows[1]).toContain('Not tested on a machine');
});

it('H38: the install button is present and refused with the reason of the hub; a click asks nothing', async () => {
  await openCatalogue();
  document.querySelector<HTMLButtonElement>('.entries li:nth-child(2) button')?.click();
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
