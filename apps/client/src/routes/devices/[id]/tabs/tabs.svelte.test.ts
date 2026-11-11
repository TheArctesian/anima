// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { manualClock } from '$lib/motion/loading/clock.fixture';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { runs } from '$lib/motion/engine.fixture';
import { spring, threshold } from '$lib/motion/tokens';
import TabPanel from './TabPanel.svelte';
import Tabs from './Tabs.svelte';
import { hiddenEdges, scrolledToShow } from './tabs';
import { placeSearch, readPlace, type TabName } from './tabs';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  runs.length = 0;
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

describe('readPlace and placeSearch (D-9)', () => {
  it.each([
    ['', { tab: 'overview', section: null, range: 'HOUR' }],
    ['?tab=checks', { tab: 'checks', section: null, range: 'HOUR' }],
    ['?tab=settings&section=ssh', { tab: 'settings', section: 'ssh', range: 'HOUR' }],
    ['?tab=settings&section=web', { tab: 'settings', section: null, range: 'HOUR' }],
    ['?tab=checks&section=ssh', { tab: 'checks', section: null, range: 'HOUR' }],
    ['?tab=metrics', { tab: 'overview', section: null, range: 'HOUR' }],
    ['?range=day', { tab: 'overview', section: null, range: 'DAY' }],
    ['?range=week', { tab: 'overview', section: null, range: 'WEEK' }],
    ['?range=hour', { tab: 'overview', section: null, range: 'HOUR' }],
    ['?range=year', { tab: 'overview', section: null, range: 'HOUR' }],
    ['?tab=overview&range=day', { tab: 'overview', section: null, range: 'DAY' }],
    ['?tab=checks&range=day', { tab: 'checks', section: null, range: 'HOUR' }],
  ])('D-9: %j is %j', (search, place) => {
    expect(readPlace(new URLSearchParams(search))).toEqual(place);
  });

  it('D-9: each place written to the URL reads back the same', () => {
    for (const place of [
      { tab: 'overview', section: null, range: 'HOUR' },
      { tab: 'overview', section: null, range: 'DAY' },
      { tab: 'overview', section: null, range: 'WEEK' },
      { tab: 'logs', section: null, range: 'HOUR' },
      { tab: 'settings', section: 'rdp', range: 'HOUR' },
    ] as const) {
      expect(readPlace(new URLSearchParams(placeSearch(place)))).toEqual(place);
    }
    expect(placeSearch({ tab: 'overview', section: null, range: 'HOUR' })).toBe('');
  });

  it('D-5: the range is in the URL for Overview only, and the 1-hour default leaves no query', () => {
    expect(placeSearch({ tab: 'overview', section: null, range: 'DAY' })).toBe('?range=day');
    expect(placeSearch({ tab: 'overview', section: null, range: 'WEEK' })).toBe('?range=week');
    expect(placeSearch({ tab: 'checks', section: null, range: 'WEEK' })).toBe('?tab=checks');
  });
});

function tabs(selected: TabName, reduced = false) {
  const state = $state({ selected });
  const onSelect = vi.fn<(name: TabName) => void>();
  app = mount(Tabs, {
    target: document.body,
    props: {
      get selected() {
        return state.selected;
      },
      onSelect,
      preference: { reduced },
    },
  });
  flushSync();
  return { state, onSelect };
}

function tabButtons(): HTMLButtonElement[] {
  return [...document.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
}

function key(name: string): void {
  document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { key: name, bubbles: true }));
}

describe('Tabs (D-9, Section 6.7)', () => {
  it('6.7: five tabs; only the selected one is in the tab order', () => {
    tabs('checks');
    expect(tabButtons().map((tab) => tab.textContent.replace(/\p{Co}/gu, '').trim())).toEqual(['Overview', 'Checks', 'Sessions', 'Logs', 'Settings']);
    expect(tabButtons().map((tab) => tab.tabIndex)).toEqual([-1, 0, -1, -1, -1]);
    expect(tabButtons()[1]?.getAttribute('aria-selected')).toBe('true');
  });

  it('6.7: the arrow keys move focus between tabs and wrap; Home and End go to the ends', () => {
    const { onSelect } = tabs('overview');
    tabButtons()[0]?.focus();
    key('ArrowRight');
    expect(document.activeElement).toBe(tabButtons()[1]);
    key('ArrowLeft');
    key('ArrowLeft');
    expect(document.activeElement).toBe(tabButtons()[4]);
    key('Home');
    expect(document.activeElement).toBe(tabButtons()[0]);
    key('End');
    expect(document.activeElement).toBe(tabButtons()[4]);
    expect(onSelect).not.toHaveBeenCalled();
    tabButtons()[4]?.click();
    expect(onSelect).toHaveBeenCalledWith('settings');
  });

  it('Tab selection: the indicator is placed at once, then moves on spring-snappy', () => {
    const { state } = tabs('overview');
    expect(runs).toEqual([]);
    state.selected = 'logs';
    flushSync();
    expect(runs.length).toBeGreaterThan(0);
    expect(runs[0]?.options).toMatchObject({ stiffness: spring.snappy.stiffness, damping: spring.snappy.damping });
  });

  it('MOT-040: under reduced motion the indicator moves at once', () => {
    const { state } = tabs('overview', true);
    state.selected = 'logs';
    flushSync();
    expect(runs).toEqual([]);
  });
});

function loadingPanel() {
  const time = manualClock();
  const state = $state({ loading: true });
  app = mount(TabPanel, {
    target: document.body,
    props: {
      name: 'checks',
      get loading() {
        return state.loading;
      },
      clock: time.clock,
      children: createRawSnippet(() => ({ render: () => '<p class="loaded">Loaded</p>' })),
    },
  });
  flushSync();
  return { time, state };
}

const skeleton = () => document.querySelector('.skeleton');

it.each([
  ['the list rows', undefined],
  ['a given skeleton', createRawSnippet(() => ({ render: () => '<p class="mine">Mine</p>' }))],
])('MOT-007: %s cross-fades out when the data arrives', async (_name, custom) => {
  const time = manualClock();
  const state = $state({ loading: true });
  app = mount(TabPanel, {
    target: document.body,
    props: {
      name: 'overview',
      get loading() {
        return state.loading;
      },
      clock: time.clock,
      skeleton: custom,
      children: createRawSnippet(() => ({ render: () => '<p class="loaded">Loaded</p>' })),
    },
  });
  flushSync();
  time.advance(threshold.loading);
  flushSync();
  const shown = document.querySelector('.skeleton');
  expect(shown).not.toBeNull();
  const animated: unknown[] = [];
  const animate = Element.prototype.animate;
  Object.defineProperty(Element.prototype, 'animate', {
    configurable: true,
    value: function spy(this: Element, ...args: Parameters<Element['animate']>) {
      animated.push(this);
      return animate.apply(this, args);
    },
  });
  try {
    state.loading = false;
    flushSync();
    time.advance(threshold.loadingMin);
    flushSync();
    expect(animated).toContain(shown);
  } finally {
    Object.defineProperty(Element.prototype, 'animate', { configurable: true, value: animate });
  }
});

it('D-9: a given skeleton replaces the list rows while loading', () => {
  const time = manualClock();
  app = mount(TabPanel, {
    target: document.body,
    props: {
      name: 'overview',
      loading: true,
      clock: time.clock,
      skeleton: createRawSnippet(() => ({ render: () => '<p class="mine">Mine</p>' })),
      children: createRawSnippet(() => ({ render: () => '<p class="loaded">Loaded</p>' })),
    },
  });
  flushSync();
  time.advance(threshold.loading);
  flushSync();
  expect(document.querySelector('.skeleton .mine')).not.toBeNull();
  expect(document.querySelector('.skeleton li')).toBeNull();
});
const loaded = () => document.querySelector('.loaded');

describe('TabPanel (D-9, Loading role)', () => {
  it('Loading: a quick answer shows no skeleton at all', async () => {
    const { time, state } = loadingPanel();
    time.advance(threshold.loading - 1);
    state.loading = false;
    flushSync();
    await tick();
    expect(loaded()).not.toBeNull();
    time.advance(threshold.loading);
    flushSync();
    expect(skeleton()).toBeNull();
  });

  it('Loading: the skeleton shows after threshold-loading and stays threshold-loading-min before the data', async () => {
    const { time, state } = loadingPanel();
    time.advance(threshold.loading);
    flushSync();
    expect(skeleton()).not.toBeNull();
    expect(document.querySelector('[role="tabpanel"]')?.getAttribute('aria-labelledby')).toBe('device-tab-checks');
    state.loading = false;
    flushSync();
    time.advance(threshold.loadingMin - 1);
    flushSync();
    expect(loaded()).toBeNull();
    time.advance(1);
    flushSync();
    await tick();
    expect(loaded()).not.toBeNull();
  });
});

it('NAV-005: a strip shows a fade at each end that has tabs past it', () => {
  expect(hiddenEdges(0, 200, 400)).toEqual({ start: false, end: true });
  expect(hiddenEdges(100, 200, 400)).toEqual({ start: true, end: true });
  expect(hiddenEdges(200, 200, 400)).toEqual({ start: true, end: false });
  expect(hiddenEdges(0, 400, 400)).toEqual({ start: false, end: false });
});

it('NAV-005: a focused tab past either edge of the strip is scrolled into view clear of the fade, and one inside it is left alone', () => {
  expect(scrolledToShow({ start: 0, size: 200 }, { start: 300, size: 80 }, 32)).toBe(212);
  expect(scrolledToShow({ start: 150, size: 200 }, { start: 100, size: 80 }, 32)).toBe(68);
  expect(scrolledToShow({ start: 150, size: 200 }, { start: 10, size: 80 }, 32)).toBe(0);
  expect(scrolledToShow({ start: 50, size: 300 }, { start: 100, size: 80 }, 32)).toBe(50);
});
