// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import type { HubError } from '$lib/hub/hub';
import type { MetricRange } from '$lib/metrics/api';
import { INTEGRATED, NVIDIA, TWO_GPUS, added, point, series } from '$lib/metrics/metrics.fixture';
import { manualClock } from '$lib/motion/loading/clock.fixture';
import { threshold } from '$lib/motion/tokens';
import { device } from '../page.fixture.svelte';
import type { DeviceMetrics } from './metrics.svelte';
import OverviewPanel from './OverviewPanel.svelte';
import OverviewSkeleton from './OverviewSkeleton.svelte';
import { checkCounts, hostKeyText, uptimeReadings } from './overview';

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

it('D-10: uptime for 24 hours, 7 days and 30 days; no value is "—"', () => {
  expect(uptimeReadings(device({ day: 0.9995, week: 0.5, month: null }))).toEqual([
    { label: '24 hours', text: '99.95 %' },
    { label: '7 days', text: '50.00 %' },
    { label: '30 days', text: '—' },
  ]);
});

it('D-10: checks are counted by status, worst first, without empty statuses', () => {
  expect(checkCounts([{ status: 'UP' }, { status: 'DOWN' }, { status: 'UP' }])).toEqual([
    { status: 'DOWN', label: 'Down', count: 1 },
    { status: 'UP', label: 'Up', count: 2 },
  ]);
  expect(checkCounts([])).toEqual([]);
});

it.each([
  [{ enabled: true, hostKey: device().ssh.hostKey }, 'ssh-ed25519 SHA256:abc'],
  [{ enabled: true, hostKey: null }, 'No host key accepted yet'],
  [{ enabled: false, hostKey: null }, 'SSH is off'],
])('D-10: the host key line for %j is %j', (ssh, text) => {
  expect(hostKeyText(ssh)).toBe(text);
});

type Shown = Pick<DeviceMetrics, 'view' | 'span' | 'shownRange' | 'chartShape' | 'readings' | 'error' | 'liveError'>;

const NOW = new Date('2026-11-05T09:05:00Z');
const SPAN = { start: NOW.getTime() - 3_600_000, end: NOW.getTime() };
const LOADED: Shown = {
  view: { metricsSource: 'SSH', gpus: [NVIDIA, INTEGRATED], series: TWO_GPUS, readings: null },
  span: SPAN,
  shownRange: 'HOUR',
  chartShape: null,
  readings: { ...added('2026-11-05T09:04:00Z', []).readings, cpuPercent: 20, memoryPercent: 45, diskPercent: 61 },
  error: null,
  liveError: null,
};
const WAITING: Shown = { view: null, span: null, shownRange: null, chartShape: null, readings: null, error: null, liveError: null };

function show(metrics: Shown, source: 'SSH' | 'NONE' = 'SSH', range: MetricRange = 'HOUR') {
  const time = manualClock();
  const calls = { range: vi.fn<(range: MetricRange) => void>(), retry: vi.fn<() => void>(), settings: vi.fn<() => void>() };
  app = mount(OverviewPanel, {
    target: document.body,
    props: {
      device: device({ maintenance: true, metricsSource: source }),
      checks: [{ status: 'UP' }],
      metrics,
      range,
      now: () => NOW,
      clock: time.clock,
      onRange: calls.range,
      onRetry: calls.retry,
      onSettings: calls.settings,
    },
  });
  flushSync();
  return { time, calls };
}

const button = (name: string) => [...document.querySelectorAll('button')].find((one) => one.textContent.trim() === name);

it('D-10: the panel shows status, maintenance, uptime, check counts and the key', () => {
  show(LOADED);
  const text = document.body.textContent;
  for (const part of ['Status', 'Up', 'Maintenance', 'On', 'Uptime, 30 days', '—', '1 Up', 'SHA256:abc']) expect(text).toContain(part);
});

it('MET-003: the data shows one chart per metric group with its title, and the current bars', () => {
  show(LOADED);
  const titles = [...document.querySelectorAll('h3')].map((one) => one.textContent);
  expect(titles).toEqual([
    'CPU',
    'Memory',
    'Disk /',
    'Disk /data',
    'Network eth0',
    'Load',
    `GPU 0 ${NVIDIA.name} usage`,
    `GPU 0 ${NVIDIA.name} VRAM`,
    `GPU 1 ${INTEGRATED.name} usage`,
  ]);
  expect(document.querySelectorAll('svg[role="img"]')).toHaveLength(titles.length);
  expect(document.querySelectorAll('[role="meter"]').length).toBeGreaterThan(0);
});

it('MET-003: the range control marks the current choice and reports a change', () => {
  const { calls } = show(LOADED, 'SSH', 'DAY');
  const choices = [...document.querySelectorAll('[aria-label="Time range"] button')];
  expect(choices.map((one) => [one.textContent.trim(), one.getAttribute('aria-pressed')])).toEqual([
    ['1 hour', 'false'],
    ['24 hours', 'true'],
    ['7 days', 'false'],
  ]);
  button('7 days')?.click();
  expect(calls.range).toHaveBeenCalledExactlyOnceWith('WEEK');
});

it('D-7: with no metrics source the panel says metrics are off, links to Settings and shows no chart', () => {
  const { calls } = show(WAITING, 'NONE');
  expect(document.body.textContent).toContain('Metrics are off for this device.');
  expect(document.querySelector('[aria-label="Time range"]')).toBeNull();
  expect(document.querySelector('svg')).toBeNull();
  button('Open Settings')?.click();
  expect(calls.settings).toHaveBeenCalledOnce();
});

it('D-3: a slow load shows the chart grid skeleton after threshold-loading, with no line', () => {
  const { time } = show(WAITING);
  expect(document.querySelector('.chart-grid')).toBeNull();
  time.advance(threshold.loading);
  flushSync();
  expect(document.querySelectorAll('.chart-grid > .cell')).toHaveLength(4);
  expect(document.querySelector('polyline')).toBeNull();
});

it('MOT-007, 5.7.3 Loading: the skeleton draws as many cells as the last answer had, and four before any', () => {
  const { time } = show({ ...WAITING, chartShape: [1, 2, 1, 1, 1, 1, 2] });
  time.advance(threshold.loading);
  flushSync();
  expect(document.querySelectorAll('.chart-grid > .cell').length).toBe(7);
  // A chart with several series has a legend row, so its placeholder reserves it.
  expect(document.querySelectorAll('.chart-grid > .cell .legend').length).toBe(2);
});

it('D-7: an error shows the hub message and Retry in the chart area', () => {
  const error: HubError = { code: 'INTERNAL', message: 'metrics are not served' };
  const { calls } = show({ ...WAITING, error });
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('metrics are not served');
  button('Retry')?.click();
  expect(calls.retry).toHaveBeenCalledOnce();
});

it('MET-007: data with no series says it is waiting for metrics', () => {
  show({ ...LOADED, view: { metricsSource: 'SSH', gpus: [], series: [], readings: null } });
  expect(document.body.textContent).toContain('Waiting for metrics');
  expect(document.querySelector('h3')).toBeNull();
});

it.each([
  ['NONE', 0, 1],
  ['SSH', 4, 0],
] as const)('D-9: the Overview skeleton for %s draws %i chart cells and %i "off" lines, as the loaded layout does', (source, cells, off) => {
  app = mount(OverviewSkeleton, { target: document.body, props: { range: 'HOUR', source, ssh: null, now: () => NOW } });
  flushSync();
  expect(document.querySelectorAll('.chart-grid > .cell')).toHaveLength(cells);
  expect(document.querySelectorAll('.metrics-off')).toHaveLength(off);
  expect(document.querySelectorAll('.overview-facts > div')).toHaveLength(7);
});

it.each([
  ['a host key', { enabled: true, hostKey: device().ssh.hostKey }],
  ['no key yet', { enabled: true, hostKey: null }],
  ['SSH off', { enabled: false, hostKey: null }],
])('MOT-007: the skeleton host key row for %s has the loaded key row\'s text and class', (_case, ssh) => {
  app = mount(OverviewSkeleton, { target: document.body, props: { range: 'HOUR', source: 'SSH', ssh, now: () => NOW } });
  flushSync();
  const row = document.querySelector('.overview-facts > div:last-child dd');
  expect(row?.textContent).toBe(hostKeyText(ssh));
  expect(row?.classList.contains('key')).toBe(true);
});

it('MOT-007: with no device known the skeleton key row holds a full-length key', () => {
  app = mount(OverviewSkeleton, { target: document.body, props: { range: 'HOUR', source: 'SSH', ssh: null, now: () => NOW } });
  flushSync();
  expect(document.querySelector('.overview-facts .key')?.textContent).toMatch(/^ssh-ed25519 SHA256:x{43}$/);
});

it('MET-003: while a switch is pending the old chart keeps its own range in the label and its own gap', () => {
  const at = (minute: number) => point(`2026-11-05T09:0${String(minute)}:00Z`, 20);
  // Minutes 0, 1, 4, 5: a three-minute hole that breaks an HOUR line but not a DAY one.
  const cpu = series('CPU_PERCENT', 'cpu', [at(0), at(1), at(4), at(5)]);
  show({ ...LOADED, view: { metricsSource: 'SSH', gpus: [], series: [cpu], readings: null }, shownRange: 'HOUR' }, 'SSH', 'DAY');
  expect(document.querySelector('svg[role="img"]')?.getAttribute('aria-label')).toContain('CPU, last hour');
  expect(document.querySelectorAll('.lines polyline')).toHaveLength(2);
  expect(document.querySelector('[aria-pressed="true"]')?.textContent.trim()).toBe('24 hours');
});
