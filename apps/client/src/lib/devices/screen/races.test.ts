// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { threshold } from '$lib/motion/tokens';
import { DEFAULT_QUERY, type ListQuery } from '../query';
import { endTransitionsAtOnce } from '../transitions.fixture';
import { manualClock } from '$lib/motion/loading/clock.fixture';
import DeviceScreen from './DeviceScreen.svelte';
import { heldHub } from './held.fixture';
import { device, url } from './screen.fixture.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const current = vi.hoisted(() => ({ hub: undefined as unknown }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;
let held: ReturnType<typeof heldHub>;
let time: ReturnType<typeof manualClock>;

async function settle(): Promise<void> {
  for (let round = 0; round < 6; round += 1) await Promise.resolve();
  flushSync();
}

async function show(query: ListQuery = DEFAULT_QUERY): Promise<void> {
  held = heldHub();
  current.hub = held.hub;
  time = manualClock();
  url.query = query;
  app = mount(DeviceScreen, {
    target: document.body,
    props: {
      get query() {
        return url.query;
      },
      onQueryChange: () => undefined,
      onAddDevice: () => undefined,
      onConnect: () => undefined,
      clock: time.clock,
    },
  });
  await settle();
}

const names = () => [...document.querySelectorAll('article h2')].map((node) => node.textContent.trim());
const requests = () => held.held.length;

beforeEach(() => {
  url.writes = [];
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('DEV-003: an answer for filter A that arrives after filter B was answered does not show', async () => {
  await show({ ...DEFAULT_QUERY, name: 'a' });
  url.query = { ...DEFAULT_QUERY, name: 'b' };
  await settle();
  expect(requests()).toBe(2);
  held.held[1]?.release([device('b')]);
  await settle();
  held.held[0]?.release([device('a')]);
  await settle();
  expect(names()).toEqual(['Device b']);
});

it('DEV-003: a Load more in flight when the filter changes does not add its page', async () => {
  await show();
  held.held[0]?.release([device('a')], 'c1');
  await settle();
  document.querySelectorAll<HTMLButtonElement>('button').forEach((node) => {
    if (node.textContent.includes('Load more')) node.click();
  });
  await settle();
  expect(requests()).toBe(2);
  url.query = { ...DEFAULT_QUERY, name: 'b' };
  await settle();
  held.held[2]?.release([device('b')]);
  await settle();
  held.held[1]?.release([device('old')]);
  await settle();
  expect(names()).toEqual(['Device b']);
});

it('DEV-003: unmounting during a load leaves no timer, and a late answer does nothing', async () => {
  await show();
  expect(time.pending()).toBe(1);
  await unmount(app as object);
  app = undefined;
  expect(time.pending()).toBe(0);
  time.advance(threshold.loading * 4);
  expect(time.fired()).toBe(0);
  held.held[0]?.release([device('a')]);
  await settle();
  expect(document.querySelectorAll('article')).toHaveLength(0);
});

it('Loading: unmounting while the skeleton is held leaves no timer, and nothing fires afterwards', async () => {
  await show();
  time.advance(threshold.loading);
  held.held[0]?.release([device('a')]);
  await settle();
  expect(time.pending()).toBe(1);
  const before = time.fired();
  await unmount(app as object);
  app = undefined;
  expect(time.pending()).toBe(0);
  time.advance(threshold.loadingMin * 3);
  expect(time.fired()).toBe(before);
});

it('DEV-003: two pushes for unknown devices in flight give exactly one more fetch after the first', async () => {
  await show({ ...DEFAULT_QUERY, statuses: ['DOWN'] });
  held.held[0]?.release([device('a', { status: 'DOWN' })]);
  await settle();
  held.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'x', status: 'DOWN', at: 'now' } });
  held.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'y', status: 'DOWN', at: 'now' } });
  held.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'z', status: 'DOWN', at: 'now' } });
  await settle();
  expect(requests()).toBe(2);
  held.held[1]?.release([device('a', { status: 'DOWN' }), device('x', { status: 'DOWN' })]);
  await settle();
  expect(requests()).toBe(3);
  held.held[2]?.release([device('a', { status: 'DOWN' }), device('x', { status: 'DOWN' }), device('y', { status: 'DOWN' })]);
  await settle();
  expect(requests()).toBe(3);
  expect(names()).toEqual(['Device a', 'Device x', 'Device y']);
});

it('DEV-003: the follow-up fetch is not sent after the page went away', async () => {
  await show({ ...DEFAULT_QUERY, statuses: ['DOWN'] });
  held.held[0]?.release([device('a', { status: 'DOWN' })]);
  await settle();
  held.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'x', status: 'DOWN', at: 'now' } });
  held.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'y', status: 'DOWN', at: 'now' } });
  await settle();
  await unmount(app as object);
  app = undefined;
  held.held[1]?.release([device('a', { status: 'DOWN' })]);
  await settle();
  expect(requests()).toBe(2);
});

it('DEV-003: a refresh drops a Load more that is in flight', async () => {
  await show({ ...DEFAULT_QUERY, statuses: ['DOWN'] });
  held.held[0]?.release([device('a', { status: 'DOWN' })], 'c1');
  await settle();
  [...document.querySelectorAll('button')].find((node) => node.textContent.includes('Load more'))?.click();
  await settle();
  held.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'x', status: 'DOWN', at: 'now' } });
  await settle();
  held.held[2]?.release([device('a', { status: 'DOWN' }), device('x', { status: 'DOWN' })]);
  await settle();
  held.held[1]?.release([device('late', { status: 'DOWN' })]);
  await settle();
  expect(names()).toEqual(['Device a', 'Device x']);
});

it('Loading: a list on screen stays while a slow reload runs, and no skeleton replaces it', async () => {
  await show();
  held.held[0]?.release([device('a')]);
  await settle();
  url.query = { ...DEFAULT_QUERY, sort: 'STATUS' };
  await settle();
  time.advance(threshold.loading * 2);
  flushSync();
  expect(document.querySelectorAll('li.place')).toHaveLength(0);
  expect(names()).toEqual(['Device a']);
});

it('DEV-003: a refresh answer that arrives after the filter changed does not show', async () => {
  await show({ ...DEFAULT_QUERY, statuses: ['DOWN'] });
  held.held[0]?.release([device('a', { status: 'DOWN' })]);
  await settle();
  held.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: 'x', status: 'DOWN', at: 'now' } });
  await settle();
  expect(requests()).toBe(2);
  url.query = { ...DEFAULT_QUERY, statuses: ['DOWN'], name: 'b' };
  await settle();
  held.held[2]?.release([device('b', { status: 'DOWN' })]);
  await settle();
  held.held[1]?.release([device('late', { status: 'DOWN' })]);
  await settle();
  expect(names()).toEqual(['Device b']);
});
