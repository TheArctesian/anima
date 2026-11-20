// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { CardDevice } from '../api';
import { fakeHub } from '../hub.fixture';
import { DEFAULT_QUERY, type ListQuery } from '../query';
import DeviceScreen from '../screen/DeviceScreen.svelte';
import { device, devicesAnswer, url } from '../screen/screen.fixture.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';

// How the rows of the Devices list come, go and move (5.7.3, Reorder and Loading).

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const preference = vi.hoisted(() => ({ reduced: false }));
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => preference }));

const current = vi.hoisted(() => ({ hub: undefined as unknown }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;
let fake: ReturnType<typeof fakeHub>;
const LIST: ListQuery = { ...DEFAULT_QUERY, view: 'table', sort: 'STATUS' };

async function settle(): Promise<void> {
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
  flushSync();
}

async function show(devices: readonly CardDevice[], query: ListQuery = LIST): Promise<void> {
  fake = fakeHub({ Devices: devicesAnswer(devices), Tags: { tags: [] } });
  current.hub = fake.hub;
  url.query = query;
  app = mount(DeviceScreen, {
    target: document.body,
    props: {
      get query() {
        return url.query;
      },
      onQueryChange: (next: ListQuery) => {
        url.query = next;
      },
      onAddDevice: () => undefined,
      onConnect: () => undefined,
    },
  });
  await settle();
}

const rowNames = () => [...document.querySelectorAll('tbody tr.device a[data-device-id]')].map((node) => node.textContent.trim());
const list = () => document.querySelector('.device-list') as HTMLElement;
const goesDown = (id: string) => fake.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: id, status: 'DOWN', at: 'now' } });

beforeEach(() => {
  preference.reduced = false;
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

const groupTitles = () => [...document.querySelectorAll('tr.group')].map((node) => node.textContent.trim());

it('5.7.3 Reorder: a live status change waits while the pointer is on the list, then the rows move', async () => {
  await show([device('a'), device('b'), device('c')]);
  list().dispatchEvent(new Event('pointerenter'));
  goesDown('b');
  await settle();
  expect(rowNames()).toEqual(['Device a', 'Device b', 'Device c']);
  expect(groupTitles()).toEqual(['Up2']);
  const rowB = [...document.querySelectorAll('tr.device')].find((row) => row.textContent.includes('Device b'));
  expect(rowB?.querySelector('.pill')?.textContent.trim()).toBe('Down');
  list().dispatchEvent(new Event('pointerleave'));
  await settle();
  expect(rowNames()).toEqual(['Device b', 'Device a', 'Device c']);
  expect(groupTitles()).toEqual(['Needs a look1', 'Up2']);
});

it('5.7.3 Reorder: with no pointer on the list, a live status change moves the rows at once', async () => {
  await show([device('c', { status: 'DOWN' }), device('a'), device('b')]);
  goesDown('b');
  await settle();
  expect(rowNames()).toEqual(['Device b', 'Device c', 'Device a']);
});

it('5.7.3 Reorder: a row that leaves the list plays its exit in place', async () => {
  await show([device('a'), device('b')], { ...LIST, statuses: ['UP'] });
  const leaving = [...document.querySelectorAll('tr.device')].find((row) => row.textContent.includes('Device a'));
  const animate = vi.spyOn(Element.prototype, 'animate');
  goesDown('a');
  await settle();
  expect(animate.mock.contexts).toContain(leaving);
});

it('5.7.3 Reorder: a group title that appears plays its entrance', async () => {
  await show([device('a'), device('b')]);
  const animate = vi.spyOn(Element.prototype, 'animate');
  goesDown('a');
  await settle();
  const title = [...document.querySelectorAll('tr.group')].find((row) => row.textContent.includes('Needs a look'));
  expect(title).toBeDefined();
  expect(animate.mock.contexts).toContain(title);
});

it('5.7.3 Page change: the change from List to Grid cross-fades the two faces', async () => {
  await show([device('a')]);
  const table = document.querySelector('.layer');
  const animate = vi.spyOn(Element.prototype, 'animate');
  url.query = { ...LIST, view: 'grid' };
  await settle();
  const grid = document.querySelector('ul.grid')?.closest('.layer');
  expect(animate.mock.contexts).toEqual(expect.arrayContaining([table, grid]));
});
