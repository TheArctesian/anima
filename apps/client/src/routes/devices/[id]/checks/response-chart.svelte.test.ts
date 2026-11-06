// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import type { MotionTarget } from '$lib/motion/animate';
import { CHART_STEP, type ChartBeat } from './chart';
import ResponseChart from './ResponseChart.svelte';

const calls = vi.hoisted(() => [] as { element: Element; target: MotionTarget }[]);
vi.mock('$lib/motion/animate', () => ({
  animate: (element: Element, target: MotionTarget) => {
    calls.push({ element, target });
    return { stop: () => undefined, finished: Promise.resolve() };
  },
}));
vi.mock('$lib/motion/options', () => ({ springMotion: () => ({ type: 'spring' }) }));

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  calls.length = 0;
  document.body.replaceChildren();
});

const beat = (minute: number): ChartBeat => ({ at: `2026-11-04T10:${String(minute).padStart(2, '0')}:00Z`, responseTimeMs: 20 });

it('Chart update: the line starts one step right of where it was, in the chart units the viewBox scales', () => {
  const props = $state({ beats: [beat(2), beat(1)], preference: { reduced: false } });
  app = mount(ResponseChart, { target: document.body, props });
  flushSync();
  // A screen 1.68 times wider than the viewBox must not change the offset: it is in viewBox units.
  const svg = document.querySelector('svg');
  vi.spyOn(svg as SVGSVGElement, 'getBoundingClientRect').mockReturnValue({ width: 1008 } as DOMRect);
  calls.length = 0;
  props.beats = [beat(3), beat(2), beat(1)];
  flushSync();
  expect(calls.map((call) => call.target.translate)).toEqual([{ x: CHART_STEP, y: 0 }, { x: 0, y: 0 }]);
  expect(calls[0]?.element.tagName.toLowerCase()).toBe('g');
});

it('Chart update: a first load, or older beats arriving, does not move the line', () => {
  const props = $state({ beats: [] as ChartBeat[], preference: { reduced: false } });
  app = mount(ResponseChart, { target: document.body, props });
  flushSync();
  props.beats = [beat(2), beat(1)];
  flushSync();
  expect(calls).toEqual([]);
});
