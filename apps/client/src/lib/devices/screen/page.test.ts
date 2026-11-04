// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import Page from '../../../routes/devices/+page.svelte';
import { fakeHub } from '../hub.fixture';
import { endTransitionsAtOnce } from '../transitions.fixture';
import { device, devicesAnswer, url } from './screen.fixture.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const current = vi.hoisted(() => ({ hub: undefined as unknown, goto: [] as [string, unknown][] }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));

vi.mock('$app/navigation', () => ({
  goto: (href: string, options: unknown) => {
    current.goto.push([href, options]);
    return Promise.resolve();
  },
}));

vi.mock('$app/state', async () => {
  const { url: state } = await import('./screen.fixture.svelte');
  const { writeListQuery } = await import('../query');
  return {
    page: {
      get url() {
        return new URL(`/devices?${writeListQuery(state.query).toString()}`, 'http://localhost');
      },
    },
  };
});

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

async function show(devices = [device('a')]): Promise<ReturnType<typeof fakeHub>> {
  const fake = fakeHub({ Devices: devicesAnswer(devices), Tags: { tags: [] } });
  current.hub = fake.hub;
  app = mount(Page, { target: document.body });
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
  flushSync();
  return fake;
}

beforeEach(() => {
  current.goto = [];
  url.query = { view: 'grid', name: '', statuses: [], tags: [], sort: 'NAME' };
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  vi.useRealTimers();
});

it('DEV-001: the page has one h1 and the list in a labelled list', async () => {
  await show();
  expect(document.querySelectorAll('h1')).toHaveLength(1);
  expect(document.querySelector('h1')?.textContent).toBe('Devices');
  expect(document.querySelectorAll('article')).toHaveLength(1);
});

it('DEV-003: a filter writes the URL query with replaceState, keeping focus and scroll', async () => {
  await show();
  const toggle = document.querySelectorAll<HTMLButtonElement>('.views button')[1];
  toggle?.click();
  expect(current.goto).toEqual([['/devices?view=table', { replaceState: true, keepFocus: true, noScroll: true }]]);
});

it('DEV-003: the URL query is what the page shows (Back and Forward restore the view)', async () => {
  await show();
  url.query = { ...url.query, view: 'table' };
  flushSync();
  expect(document.querySelector('table')).not.toBeNull();
  url.query = { ...url.query, view: 'grid' };
  flushSync();
  expect(document.querySelector('table')).toBeNull();
});

it('H-1: Add device goes to /devices/new', async () => {
  await show();
  [...document.querySelectorAll('button')].find((node) => node.textContent.includes('Add device'))?.click();
  expect(current.goto[0]?.[0]).toBe('/devices/new');
});

it('H-2: SSH does nothing until WP-14 supplies the session tab', async () => {
  const fake = await show([device('a', { ssh: { enabled: true } })]);
  const sent = fake.requests.length;
  const ssh = [...document.querySelectorAll('button')].find((node) => node.textContent.trim() === 'SSH');
  expect(ssh).toBeDefined();
  ssh?.click();
  await Promise.resolve();
  expect(current.goto).toEqual([]);
  expect(fake.requests).toHaveLength(sent);
});
