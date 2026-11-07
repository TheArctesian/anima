// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { runs } from '$lib/motion/engine.fixture';
import { spring } from '$lib/motion/tokens';
import ChartSkeleton from './ChartSkeleton.svelte';
import LineChart from './LineChart.svelte';
import type { ChartSeries } from './plot';
import { PLOT, type TimeWindow } from './scales';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

endTransitionsAtOnce();

const HOUR = 3_600_000;
const percent = (value: number) => `${String(value)} %`;
let apps: ReturnType<typeof mount>[] = [];

beforeEach(() => {
  runs.length = 0;
});

afterEach(async () => {
  await Promise.all(apps.map((app) => unmount(app)));
  apps = [];
  document.body.replaceChildren();
});

function cpu(span: TimeWindow): ChartSeries {
  return { label: 'CPU', colour: 1, points: [{ at: span.start, value: 20 }, { at: span.end, value: 40 }] };
}

function chart(reduced = false) {
  const span = { start: 0, end: HOUR };
  const props = $state({ title: 'CPU, last hour', series: [cpu(span)], span, axis: 'percent' as const, format: percent, threshold: undefined as number | undefined, preference: { reduced } });
  apps.push(mount(LineChart, { target: document.body, props }));
  flushSync();
  return props;
}

/** Moves the chart's window forward by `minutes`, with a new point at the right edge. */
function moveOn(props: ReturnType<typeof chart>, minutes: number): void {
  const step = minutes * 60_000;
  props.span = { start: props.span.start + step, end: props.span.end + step };
  props.series = [cpu(props.span)];
  flushSync();
}

const lines = () => document.querySelector<SVGGElement>('.lines > g');

describe('LineChart: Chart update (5.7.3, D-2)', () => {
  it('MOT-002: the lines start right by the distance the axis moved, in viewBox units, and spring to rest', () => {
    const props = chart();
    moveOn(props, 15);
    expect(runs.map((run) => run.keyframes)).toEqual([[PLOT.width / 4, 0]]);
    expect(runs[0]?.options).toMatchObject({ type: 'spring', stiffness: spring.default.stiffness, damping: spring.default.damping });
    runs[0]?.finish();
    expect(lines()?.style.translate).toBe('0px 0px');
  });

  it('MOT-002: a point that comes in mid-slide starts from where the lines are on screen', () => {
    const props = chart();
    moveOn(props, 15);
    runs[0]?.frame(60);
    moveOn(props, 6);
    expect(runs[1]?.keyframes).toEqual([60 + PLOT.width / 10, 0]);
    expect(runs[0]?.stops).toBe(1);
  });

  it('MOT-006: only translate moves, and the grid and axes stay still', () => {
    const props = chart();
    moveOn(props, 15);
    const moved = [...document.querySelectorAll<HTMLElement | SVGElement>('[style]')].filter((node) => node.style.translate !== '');
    expect(moved).toEqual([lines()]);
    expect([...(lines()?.style ?? [])]).toEqual(['translate']);
  });

  it('MOT-041: under reduced motion the lines are at rest at once', () => {
    const props = chart(true);
    moveOn(props, 15);
    expect(runs).toEqual([]);
    expect(lines()?.style.translate).toBe('0px 0px');
  });

  it('MOT-011: a first draw, or new values in the same window, does not slide', () => {
    const props = chart();
    props.series = [{ ...cpu(props.span), points: [{ at: 10, value: 90 }] }];
    flushSync();
    expect(runs).toEqual([]);
    expect(lines()?.style.translate).toBe('');
  });

  it('D-2: a range change does not slide; the old chart cross-fades out in the same cell', async () => {
    const props = chart();
    props.span = { start: 0, end: 24 * HOUR };
    flushSync();
    expect(runs).toEqual([]);
    expect(document.querySelectorAll('.chart > .layer')).toHaveLength(2);
    await tick();
    await Promise.resolve();
    flushSync();
    expect(document.querySelectorAll('.chart > .layer')).toHaveLength(1);
  });
});

describe('LineChart: text and legend (D-1)', () => {
  it('D-1: the chart is an image whose label says the title, the latest value and the top of the axis', () => {
    chart();
    expect(document.querySelector('[role="img"]')?.getAttribute('aria-label')).toBe('CPU, last hour, latest 40 %, top of the chart 100 %');
  });

  it('D-1: one series has no legend; two series name each line in a legend, so colour is not the only cue', () => {
    const props = chart();
    expect(document.querySelector('.legend')).toBeNull();
    props.series = [
      { label: 'In', colour: 1, points: [] },
      { label: 'Out', colour: 2, points: [] },
    ];
    flushSync();
    expect([...document.querySelectorAll('.legend li')].map((item) => item.textContent.trim())).toEqual(['In', 'Out']);
    expect([...document.querySelectorAll('.legend li')].map((item) => item.classList.contains('dash'))).toEqual([false, true]);
    expect(document.querySelectorAll('.lines .dash')).toHaveLength(1);
    expect(document.querySelectorAll('.lines polyline')).toHaveLength(0);
  });

  it('MET-004: a threshold draws one line across the plot at its value', () => {
    const props = chart();
    expect(document.querySelector('.threshold')).toBeNull();
    props.threshold = 85;
    flushSync();
    expect(Number(document.querySelector('.threshold')?.getAttribute('y1'))).toBeCloseTo(PLOT.height * 0.15);
  });
});

/** The frame's markup without its lines and its text alternative: what sets the box, grid, axes and legend. */
function geometry(target: Element): string {
  const frame = target.querySelector('.frame')?.cloneNode(true) as Element;
  frame.querySelector('.lines')?.remove();
  frame.removeAttribute('aria-hidden');
  for (const image of frame.querySelectorAll('[role="img"]')) {
    image.removeAttribute('role');
    image.removeAttribute('aria-label');
  }
  return frame.outerHTML;
}

describe('ChartSkeleton (D-3, MOT-007)', () => {
  it('MOT-007: the skeleton is the chart box with the same grid, axes and legend, and no line', () => {
    const span = { start: Date.UTC(2026, 10, 5, 9), end: Date.UTC(2026, 10, 5, 10) };
    const legend = [
      { label: 'In', colour: 1 as const },
      { label: 'Out', colour: 2 as const },
    ];
    const shared = { span, axis: 'percent' as const, format: percent, threshold: 85 };
    const chartTarget = document.body.appendChild(document.createElement('div'));
    const skeletonTarget = document.body.appendChild(document.createElement('div'));
    const series = legend.map((entry) => ({ ...entry, points: [{ at: span.start, value: 10 }, { at: span.end, value: 30 }] }));
    apps.push(mount(LineChart, { target: chartTarget, props: { ...shared, title: 'Network', series } }));
    apps.push(mount(ChartSkeleton, { target: skeletonTarget, props: { ...shared, legend } }));
    flushSync();
    expect(chartTarget.querySelectorAll('polyline')).toHaveLength(2);
    expect(skeletonTarget.querySelector('polyline')).toBeNull();
    expect(skeletonTarget.querySelector('[role="img"]')).toBeNull();
    expect(skeletonTarget.firstElementChild?.getAttribute('aria-hidden')).toBe('true');
    expect(geometry(skeletonTarget)).toBe(geometry(chartTarget));
  });

  it('D-3: an open axis skeleton keeps its value label places but shows no values it does not know', () => {
    apps.push(mount(ChartSkeleton, { target: document.body, props: { span: { start: 0, end: HOUR }, axis: 'open', format: percent } }));
    flushSync();
    const labels = [...document.querySelectorAll('.values span')];
    expect(labels).toHaveLength(5);
    expect(labels.every((label) => label.textContent === '')).toBe(true);
  });
});
