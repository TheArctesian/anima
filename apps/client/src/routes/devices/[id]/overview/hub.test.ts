// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import type { HubKept } from '$lib/hosted/kept/kept';
import { point, series } from '$lib/metrics/metrics.fixture';
import { manualClock } from '$lib/motion/loading/clock.fixture';
import { device } from '../page.fixture.svelte';
import type { DeviceMetrics } from './metrics.svelte';
import OverviewPanel from './OverviewPanel.svelte';

vi.mock('$lib/hosted/usage/read', () => ({ readAccountUsage: () => Promise.resolve({ state: 'unavailable' }) }));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const GIB = 1024 ** 3;
const NOW = new Date('2026-11-05T09:05:00Z');
const AT = '2026-11-05T09:04:00Z';
const READINGS = { at: AT, cpuPercent: 12, memoryPercent: 40, diskPercent: null, diskMount: null, otherDrives: 0, vramPercent: null, vramGpu: null };
// The hub's own daemon sends only figures of its container: no disk percent, the bytes of its data (K8 Section 4).
const SERIES = [series('CPU_PERCENT', '', [point(AT, 12)]), series('MEMORY_PERCENT', '', [point(AT, 40)]), series('DISK_BYTES', '/var/lib/anima', [point(AT, 3 * GIB)])];
const METRICS: Pick<DeviceMetrics, 'view' | 'span' | 'shownRange' | 'chartShape' | 'readings' | 'battery' | 'error' | 'liveError' | 'memoryBytes'> = {
  view: { metricsSource: 'DAEMON', gpus: [], series: SERIES, drives: [], readings: READINGS, batteries: [] },
  span: { start: NOW.getTime() - 3_600_000, end: NOW.getTime() },
  shownRange: 'HOUR',
  chartShape: null,
  readings: READINGS,
  battery: null,
  error: null,
  liveError: null,
  memoryBytes: 0.8 * GIB,
};
const KEPT: HubKept = { devices: 4, checks: 9, periods: [{ subject: 'METRICS', days: 30 }, { subject: 'HEARTBEATS', days: 90 }] };

async function show(kind: string) {
  app = mount(OverviewPanel, {
    target: document.body,
    props: {
      device: device({ kind, metricsSource: 'DAEMON', ssh: { ...device().ssh, enabled: false }, vnc: { ...device().vnc, enabled: false } }),
      checks: [],
      metrics: METRICS,
      range: 'HOUR',
      now: () => NOW,
      clock: manualClock().clock,
      onRange: () => undefined,
      onRetry: () => undefined,
      onSettings: () => undefined,
      onConnect: () => undefined,
      onSection: () => undefined,
      readKept: () => Promise.resolve(KEPT),
    },
  });
  await tick();
  await tick();
  flushSync();
}

const card = (title: string) => [...document.querySelectorAll('section')].find((one) => one.querySelector('h2')?.textContent.trim() === title);
const rows = (title: string) => [...(card(title)?.querySelectorAll('dt') ?? [])].map((term) => `${term.textContent.trim()}: ${term.nextElementSibling?.textContent.trim() ?? ''}`);
const bar = (label: string) => document.querySelector(`[role="meter"][aria-label="${label}"]`);

it('HMD-073: the page of the hub shows CPU and its memory as used of its limit, what it keeps and the data of the hub', async () => {
  await show('hub');
  expect(bar('Memory')?.getAttribute('aria-valuetext')).toBe('0.8 of 2.0 GiB, 40 %');
  expect(bar('CPU')?.getAttribute('title')).toBe('CPU: the share of the processors that the hub may use');
  expect(document.querySelector('[role="meter"][aria-label^="Disk"]')).toBeNull();
  expect(rows('What the hub keeps')).toEqual(['Devices: 4', 'Checks: 9', 'History: 30 days of metrics, 90 days of check results']);
  expect(rows('Plan and usage')[0]).toBe('Data of your hub: 3.0 GiB');
  expect(card('Connect')?.textContent).toContain('No session reaches the container of the hub');
  expect(card('Storage')).toBeUndefined();
  expect(bar('CPU')?.textContent).toContain('of its processors');
  expect(document.body.textContent).not.toContain('SSH is off');
});

it('HMD-073: another device has the bars of a computer and no card of what the hub keeps', async () => {
  await show('server');
  expect(card('What the hub keeps')).toBeUndefined();
  expect(card('Storage')).toBeDefined();
  expect(bar('Memory')?.getAttribute('aria-valuetext')).toBe('40 %');
});
