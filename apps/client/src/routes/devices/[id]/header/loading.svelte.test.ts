// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { manualClock } from '$lib/motion/loading/clock.fixture';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { threshold } from '$lib/motion/tokens';
import { device } from '../page.fixture.svelte';
import Header from './Header.svelte';
import headerSource from './Header.svelte?raw';

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function open(initial: { loading: boolean; device: ReturnType<typeof device> | null }) {
  const time = manualClock();
  const props = $state({ ...initial, onConnect: () => undefined, onSettings: () => undefined, clock: time.clock });
  app = mount(Header, { target: document.body, props });
  flushSync();
  return { time, props };
}

const skeleton = () => document.querySelector('header .skeleton');

it('Loading: the header line shows no skeleton before threshold-loading, a skeleton of the line after it', () => {
  const { time } = open({ loading: true, device: null });
  time.advance(threshold.loading - 1);
  flushSync();
  expect(skeleton()).toBeNull();
  time.advance(1);
  flushSync();
  expect(skeleton()?.getAttribute('aria-hidden')).toBe('true');
  // The name, the status, the details and the buttons each have a place.
  expect(skeleton()?.querySelectorAll('.name, .pill, .details-place, .actions-place')).toHaveLength(4);
  // The back link stays a real link: the skeleton holds only a hidden ghost of it for the width.
  expect(document.querySelectorAll('header a.back')).toHaveLength(1);
});

it('Loading: a device that arrives before threshold-loading shows at once and the skeleton never does', () => {
  const { time, props } = open({ loading: true, device: null });
  time.advance(threshold.loading - 1);
  props.device = device();
  props.loading = false;
  flushSync();
  time.advance(threshold.loading * 2);
  flushSync();
  expect(skeleton()).toBeNull();
  expect(document.querySelector('h1')?.textContent).toBe('nas');
});

it('Loading: the skeleton stays threshold-loading-min after the device arrives, then leaves', async () => {
  const { time, props } = open({ loading: true, device: null });
  time.advance(threshold.loading);
  flushSync();
  props.device = device();
  props.loading = false;
  flushSync();
  time.advance(threshold.loadingMin - 1);
  flushSync();
  expect(skeleton()).not.toBeNull();
  time.advance(1);
  await vi.waitFor(() => {
    expect(skeleton()).toBeNull();
  });
});

it('Loading: a header that is not loading, such as a missing device, never shows a skeleton', () => {
  const { time } = open({ loading: false, device: null });
  time.advance(threshold.loading * 10);
  flushSync();
  expect(skeleton()).toBeNull();
});

/** jsdom applies no component styles, so the icon-only rules are read from the source: the area each element takes there. */
function iconOnlyArea(selector: string): string | undefined {
  const rule = new RegExp(String.raw`:root\[data-nav='icons'\]\) ${selector.replace('.', String.raw`\.`)} \{[^}]*?grid-area: ([^;]+);`);
  return rule.exec(headerSource)?.[1];
}

it('MOT-007: below bp-compact the skeleton has the rows of the loaded header, each placeholder over its row', () => {
  const rows = { back: iconOnlyArea('.back'), status: iconOnlyArea('.status'), name: iconOnlyArea('h1') };
  expect(rows).toEqual({ back: '1 / 1', status: '1 / 3', name: '2 / 1 / 3 / -1' });
  // Back link, pill and name share the loaded line's cells; the details and buttons take the full width of rows 3 and 4.
  expect([
    iconOnlyArea('.skeleton > .ghost'),
    iconOnlyArea('.skeleton > .pill'),
    iconOnlyArea('.skeleton > .name'),
    iconOnlyArea('.skeleton > .details-place'),
    iconOnlyArea('.skeleton > .actions-place'),
  ]).toEqual(['1 / 1', '1 / 3', '2 / 1 / 3 / -1', '3 / 1 / 4 / -1', '4 / 1 / 5 / -1']);
  expect(headerSource).toMatch(/\.line,\s*:global\(:root\[data-nav='icons'\]\) \.skeleton \{\s*display: grid;/);
});
