// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { APPLE, CPU_ONLY, added, point, series } from '$lib/metrics/metrics.fixture';
import { manualClock } from '$lib/motion/loading/clock.fixture';
import { device } from '../page.fixture.svelte';
import OverviewPanel from './OverviewPanel.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const NOW = new Date('2026-11-05T09:05:00Z');
const AT = '2026-11-05T09:04:00Z';
const THREE_GIB = 3 * 1024 ** 3;

/** The detail view of an Apple silicon Mac with the agent: usage and shared memory, and no VRAM series. */
function showApple(): void {
  const metrics = {
    view: {
      metricsSource: 'AGENT' as const,
      gpus: [APPLE],
      series: [...CPU_ONLY, series('GPU_PERCENT', 'GPU 0', [point(AT, 35)], 0), series('GPU_MEMORY_BYTES', 'GPU 0', [point(AT, THREE_GIB)], 0)],
      readings: null,
    },
    span: { start: NOW.getTime() - 3_600_000, end: NOW.getTime() },
    shownRange: 'HOUR' as const,
    chartShape: null,
    // The card's readings, even with a VRAM figure for a GPU without its own memory (MET-007).
    readings: { ...added(AT, []).readings, cpuPercent: 20, memoryPercent: 45, diskPercent: 61, vramPercent: 5, vramGpu: APPLE },
    error: null,
    liveError: null,
  };
  app = mount(OverviewPanel, {
    target: document.body,
    props: {
      device: device({ metricsSource: 'AGENT' }),
      checks: [],
      metrics,
      range: 'HOUR',
      now: () => NOW,
      clock: manualClock().clock,
      onRange: vi.fn(),
      onRetry: vi.fn(),
      onSettings: vi.fn(),
    },
  });
  flushSync();
}

const labels = () => [...document.querySelectorAll('svg[role="img"]')].map((chart) => chart.getAttribute('aria-label') ?? '');

it('MET-009, D-30: the detail view of an Apple silicon Mac shows GPU usage and the shared memory in use, in bytes', () => {
  showApple();
  const titles = [...document.querySelectorAll('h3')].map((one) => one.textContent);
  expect(titles).toContain(`GPU 0 ${APPLE.name} usage`);
  expect(titles).toContain(`GPU 0 ${APPLE.name} memory in use (shared)`);
  expect(labels().find((label) => label.startsWith(`GPU 0 ${APPLE.name} usage`))).toContain('latest 35 %');
  expect(labels().find((label) => label.startsWith(`GPU 0 ${APPLE.name} memory in use (shared)`))).toContain('latest 3.0 GiB');
});

it('MET-007, D-30: an Apple silicon Mac shows no VRAM chart and the card draws no VRAM bar', () => {
  showApple();
  expect([...document.querySelectorAll('h3')].map((one) => one.textContent).filter((title) => /VRAM/.test(title))).toEqual([]);
  const meters = [...document.querySelectorAll('[role="meter"]')].map((meter) => meter.getAttribute('aria-label'));
  expect(meters).toEqual(['CPU', 'Memory', 'Disk']);
});
