// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import type { ContainerLog } from '../api';
import ContainerLogView from './ContainerLogView.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const LOG: ContainerLog = {
  lines: [
    { at: null, text: 'GET /health 200' },
    { at: '2026-11-18T18:00:00+00:00', text: 'INFO listening on :3000' },
    { at: '2026-11-18T18:00:01+00:00', text: 'ERROR <b>connection refused</b>' },
  ],
  cut: false,
};

let app: ReturnType<typeof mount> | undefined;
const saved: [string, Blob][] = [];

beforeEach(() => {
  installAnimations();
  saved.length = 0;
  app = mount(ContainerLogView, { target: document.body, props: { log: LOG, container: 'tracks app/1', save: (name: string, blob: Blob) => saved.push([name, blob]) } });
  flushSync();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const texts = () => [...document.querySelectorAll('.lines li > span')].map((each) => each.textContent);
const control = (label: string) => document.querySelector<HTMLElement>(`[aria-label="${label}"]`);
const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find((each) => each.textContent.trim() === label);

function type(text: string): void {
  const input = control('Search the log') as HTMLInputElement;
  input.value = text;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}

it('HOST-015: the search and the level show only the lines that agree, and say how many', () => {
  expect(texts()).toHaveLength(3);
  type('refused');
  expect(texts()).toEqual(['ERROR <b>connection refused</b>']);
  // A line of a container is text: it never becomes an element of the page.
  expect(document.querySelector('.lines b')).toBeNull();
  expect(document.querySelector('.shown')?.textContent).toBe('1 of 3 lines');
  type('');
  document.querySelector<HTMLButtonElement>('[role="radio"][data-value="info"]')?.click();
  flushSync();
  expect(texts()).toEqual(['INFO listening on :3000']);
  type('nothing has this');
  expect(document.querySelector('.lines')).toBeNull();
  expect(document.body.textContent).toContain('No line has this text and this level.');
});

it('HOST-015: each switch turns only its own thing off and on', () => {
  const wrapped = () => document.querySelector('.lines')?.classList.contains('nowrap') === false;
  const times = () => document.querySelectorAll('.lines time').length;
  expect([wrapped(), times()]).toEqual([true, 3]);
  control('Wrap lines')?.click();
  flushSync();
  expect([wrapped(), times()]).toEqual([false, 3]);
  control('Show timestamps')?.click();
  flushSync();
  expect([wrapped(), times()]).toEqual([false, 0]);
  control('Wrap lines')?.click();
  flushSync();
  expect([wrapped(), times()]).toEqual([true, 0]);
});

it('HOST-015: the download is the lines that are shown, in a file named for the container', async () => {
  document.querySelector<HTMLButtonElement>('[role="radio"][data-value="error"]')?.click();
  flushSync();
  button('Download')?.click();
  expect(saved).toHaveLength(1);
  expect(saved[0]?.[0]).toBe('tracks-app-1.log');
  expect(await saved[0]?.[1].text()).toBe('2026-11-18T18:00:01+00:00 ERROR <b>connection refused</b>\n');
  // With the timestamps off, the file has no time.
  control('Show timestamps')?.click();
  flushSync();
  button('Download')?.click();
  expect(await saved[1]?.[1].text()).toBe('ERROR <b>connection refused</b>\n');
  // With no line shown there is nothing to download.
  type('nothing has this');
  button('Download')?.click();
  expect(saved).toHaveLength(2);
});
