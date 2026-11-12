// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { runs } from '$lib/motion/engine.fixture';
import { spring } from '$lib/motion/tokens';
import Tabs from './Tabs.svelte';
import { scrolledToShow, tabIds, type TabItem } from './tabs';

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

const ITEMS: readonly TabItem[] = [
  { id: 'overview', label: 'Overview' },
  { id: 'checks', label: 'Checks', count: 3 },
  { id: 'sessions', label: 'Sessions', count: 0 },
  { id: 'logs', label: 'Logs' },
  { id: 'settings', label: 'Settings' },
];

function tabs(selected: string, reduced = false) {
  const state = $state({ selected });
  const onSelect = vi.fn<(id: string) => void>();
  app = mount(Tabs, {
    target: document.body,
    props: {
      items: ITEMS,
      get selected() {
        return state.selected;
      },
      onSelect,
      label: 'Device',
      idPrefix: 'device',
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

describe('Tabs (UI redesign, DEV-020)', () => {
  it('DEV-020: a tablist named by its label, one tab per item, only the selected tab in the tab order', () => {
    tabs('checks');
    expect(document.querySelector('[role="tablist"]')?.getAttribute('aria-label')).toBe('Device');
    expect(tabButtons().map((tab) => tab.querySelector('.word')?.textContent)).toEqual(['Overview', 'Checks', 'Sessions', 'Logs', 'Settings']);
    expect(tabButtons().map((tab) => tab.tabIndex)).toEqual([-1, 0, -1, -1, -1]);
    expect(tabButtons()[1]?.getAttribute('aria-selected')).toBe('true');
  });

  it('DEV-020: each tab names its panel and a count shows beside the word, a zero included', () => {
    tabs('overview');
    expect(tabButtons()[1]?.getAttribute('aria-controls')).toBe(tabIds('device', 'checks').panel);
    expect(tabButtons().map((tab) => tab.querySelector('.count')?.textContent ?? null)).toEqual([null, '3', '0', null, null]);
  });

  it('APG tabs: the arrow keys move focus and wrap, Home and End go to the ends, and only a click selects', () => {
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

it('NAV-005: a focused tab past either edge of the strip is scrolled into view clear of the fade, and one inside it is left alone', () => {
  expect(scrolledToShow({ start: 0, size: 200 }, { start: 300, size: 80 }, 32)).toBe(212);
  expect(scrolledToShow({ start: 150, size: 200 }, { start: 100, size: 80 }, 32)).toBe(68);
  expect(scrolledToShow({ start: 150, size: 200 }, { start: 10, size: 80 }, 32)).toBe(0);
  expect(scrolledToShow({ start: 50, size: 300 }, { start: 100, size: 80 }, 32)).toBe(50);
});
