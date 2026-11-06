// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { threshold } from '$lib/motion/tokens';
import { fakeHub } from '../hub.fixture';
import { DEFAULT_QUERY } from '../query';
import { manualClock } from '$lib/motion/loading/clock.fixture';
import tableSource from '../card/DeviceTable.svelte?raw';
import DeviceTable from '../card/DeviceTable.svelte';
import DeviceScreen from './DeviceScreen.svelte';
import Skeleton from './Skeleton.svelte';
import skeletonSource from './Skeleton.svelte?raw';
import { device, devicesAnswer } from './screen.fixture.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const current = vi.hoisted(() => ({ hub: undefined as unknown }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));

const geometry = readFileSync('src/lib/devices/card/geometry.css', 'utf8');

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

async function settle(): Promise<void> {
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
  flushSync();
}

/** A screen whose Devices answer waits until `answer()` is called. */
async function slowScreen(view: 'grid' | 'table' = 'grid') {
  const fake = fakeHub({ Devices: devicesAnswer([device('a')]), Tags: { tags: [] } });
  let release: () => void = () => undefined;
  const open = new Promise<void>((resolve) => {
    release = resolve;
  });
  current.hub = {
    ...fake.hub,
    request: async (...args: Parameters<typeof fake.hub.request>) => {
      await open;
      return fake.hub.request(...args);
    },
  };
  const time = manualClock();
  app = mount(DeviceScreen, {
    target: document.body,
    props: {
      query: { ...DEFAULT_QUERY, view },
      onQueryChange: () => undefined,
      onAddDevice: () => undefined,
      onConnect: () => undefined,
      clock: time.clock,
    },
  });
  await settle();
  return {
    time,
    answer: async () => {
      release();
      await settle();
    },
  };
}

const skeleton = () => document.querySelectorAll('li.place');

it('Loading: no skeleton before threshold-loading, the skeleton after it', async () => {
  const { time } = await slowScreen();
  time.advance(threshold.loading - 1);
  flushSync();
  expect(skeleton()).toHaveLength(0);
  time.advance(1);
  flushSync();
  expect(skeleton().length).toBeGreaterThan(0);
  expect(document.querySelector('[aria-busy="true"]')).not.toBeNull();
});

it('Loading: data before the threshold shows the cards and never the skeleton', async () => {
  const { time, answer } = await slowScreen();
  time.advance(threshold.loading - 1);
  await answer();
  time.advance(threshold.loading * 2);
  flushSync();
  expect(skeleton()).toHaveLength(0);
  expect(document.querySelectorAll('article')).toHaveLength(1);
});

it('Loading: data that arrives while the skeleton shows waits for threshold-loading-min', async () => {
  const { time, answer } = await slowScreen();
  time.advance(threshold.loading);
  flushSync();
  await answer();
  time.advance(threshold.loadingMin - 1);
  flushSync();
  expect(skeleton().length).toBeGreaterThan(0);
  time.advance(1);
  await settle();
  await vi.waitFor(() => {
    expect(skeleton()).toHaveLength(0);
  });
  expect(document.querySelectorAll('article')).toHaveLength(1);
});

it('Loading: in table view the skeleton is a table with a header row and as many rows as the first screen, not the card grid', async () => {
  const { time } = await slowScreen('table');
  time.advance(threshold.loading);
  flushSync();
  const table = document.querySelector('table.places');
  expect(table).not.toBeNull();
  expect(table?.querySelectorAll('thead tr')).toHaveLength(1);
  expect(table?.querySelectorAll('tbody tr.place')).toHaveLength(8);
  expect(document.querySelector('ul.places')).toBeNull();
});

// jsdom does not apply component styles (the document head stays empty), so the shared size can only be read from the sources.
it('Loading: the table skeleton and the table take their row size from the same value', () => {
  expect(geometry).toContain('--device-row-block-size');
  for (const source of [skeletonSource, tableSource]) expect(source).toContain('block-size: var(--device-row-block-size)');
});

it('Loading: the table skeleton renders the data table\'s caption, since a caption moves the rows half a border', () => {
  const rendered = (component: typeof Skeleton | typeof DeviceTable, props: Record<string, unknown>) => {
    const target = document.createElement('div');
    document.body.append(target);
    const view = mount(component as never, { target, props });
    const caption = target.querySelector('table > caption') !== null;
    void unmount(view);
    return caption;
  };
  expect(rendered(DeviceTable, { devices: [], beats: new Map(), onConnect: () => undefined })).toBe(true);
  expect(rendered(Skeleton, { view: 'table' })).toBe(true);
});
