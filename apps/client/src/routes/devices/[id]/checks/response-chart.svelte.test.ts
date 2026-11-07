// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { PLOT } from '$lib/charts/scales';
import { runs } from '$lib/motion/engine.fixture';
import { RESPONSE_POINTS, type ChartBeat } from './response';
import ResponseChart from './ResponseChart.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  runs.length = 0;
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const beat = (minute: number, responseTimeMs: number | null = 20): ChartBeat => ({
  at: `2026-11-04T10:${String(minute).padStart(2, '0')}:00Z`,
  responseTimeMs,
});

function chart(beats: ChartBeat[]) {
  const props = $state({ beats, preference: { reduced: false } });
  app = mount(ResponseChart, { target: document.body, props });
  flushSync();
  return props;
}

/** The x and y of each point of each drawn line. */
function drawn(): number[][][] {
  return [...document.querySelectorAll('polyline')].map((line) =>
    (line.getAttribute('points') ?? '').split(' ').map((pair) => pair.split(',').map(Number)),
  );
}

it('D-10: the newest beat is at the right edge, and a beat without a response time breaks the line', () => {
  chart([beat(4), beat(3), beat(2, null), beat(1)]);
  const lines = drawn();
  expect(lines).toHaveLength(2);
  expect(lines[1]?.at(-1)?.[0]).toBe(PLOT.width);
  expect(lines[0]?.[0]?.[0]).toBeLessThan(lines[1]?.[0]?.[0] ?? 0);
});

it('D-10: the axis scales to the slowest beat, which the label states with the latest one', () => {
  chart([beat(3, 10), beat(2, 40), beat(1, 5)]);
  const highest = Math.min(...drawn().flat().map(([, y]) => y ?? PLOT.height));
  expect(highest).toBe(0);
  expect(document.querySelector('[role="img"]')?.getAttribute('aria-label')).toBe(
    'Response time of the last 3 heartbeats, latest 10 ms, top of the chart 40 ms',
  );
});

it('D-10: no beats yet says so', () => {
  chart([]);
  expect(document.querySelector('[role="img"]')?.getAttribute('aria-label')).toBe('No response times yet');
});

it('Chart update: a new beat starts the line one step right, in viewBox units, and springs it to rest', () => {
  const props = chart([beat(2), beat(1)]);
  props.beats = [beat(3), beat(2), beat(1)];
  flushSync();
  expect(runs).toHaveLength(1);
  expect(runs[0]?.keyframes[0]).toBeCloseTo(PLOT.width / (RESPONSE_POINTS - 1));
  expect(runs[0]?.keyframes[1]).toBe(0);
});

it('Chart update: a first load, or older beats arriving, does not move the line', () => {
  const props = chart([]);
  props.beats = [beat(3), beat(2)];
  flushSync();
  props.beats = [beat(3), beat(2), beat(1)];
  flushSync();
  expect(runs).toEqual([]);
});
