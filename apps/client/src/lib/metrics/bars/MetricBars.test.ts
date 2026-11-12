// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import type { MetricsSource } from '$lib/hub/generated/graphql';
import { endTransitionsAtOnce, GPU, readings } from './bars.fixture';
import type { BarReadings } from './bars';
import MetricBars from './MetricBars.svelte';

const preference = vi.hoisted(() => ({ reduced: false }));
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => preference }));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

function bars(source: MetricsSource | null, values?: BarReadings): void {
  app = mount(MetricBars, { target: document.body, props: { readings: values ?? null, source } });
  flushSync();
}

it('6.1 MET-007: a device without readings shows the empty state and no meter', () => {
  bars('NONE');
  expect(document.querySelectorAll('[role="meter"]')).toHaveLength(0);
  expect(document.body.textContent).toContain('No metrics');
});

it('6.1: an SSH device without readings yet says it is waiting', () => {
  bars('SSH');
  expect(document.body.textContent).toContain('Waiting for metrics');
});

it('MET-004 MET-008: a GPU device shows four meters with their values, VRAM last', () => {
  bars('SSH', readings({ cpuPercent: 90, vramPercent: 40.4, vramGpu: GPU }));
  const meters = [...document.querySelectorAll('[role="meter"]')];
  expect(meters.map((meter) => meter.getAttribute('aria-label'))).toEqual(['CPU', 'Memory', 'Disk /', 'VRAM']);
  expect(meters.map((meter) => meter.getAttribute('aria-valuenow'))).toEqual(['90', '20', '30', '40.4']);
  expect(meters.every((m) => m.getAttribute('aria-valuemin') === '0' && m.getAttribute('aria-valuemax') === '100')).toBe(true);
  expect(meters[0]?.textContent).toContain('90 %');
  expect(meters[3]?.textContent).toContain('40 %');
  expect(meters[0]?.classList.contains('degraded')).toBe(true);
});

it('MET-004: a degraded bar has a marker as well as a colour, and a null reading is no meter', () => {
  bars('SSH', readings({ cpuPercent: 90, diskPercent: null }));
  expect(document.querySelector('.degraded .warn')).not.toBeNull();
  expect(document.querySelectorAll('[role="meter"]')).toHaveLength(2);
  expect(document.body.textContent).toContain('—');
});

it('MOT-040: the row takes the reduced-motion form when the operator asks for it', () => {
  preference.reduced = true;
  bars('SSH', readings());
  expect(document.querySelector('.row')?.classList.contains('reduced')).toBe(true);
  preference.reduced = false;
});

it('MET-030: the disk bar shows its mount and "+N drives" as text', () => {
  bars('AGENT', readings({ diskPercent: 81, diskMount: '/', otherDrives: 3 }));
  const disk = document.querySelector('[aria-label="Disk /"]');
  expect(disk?.textContent).toContain('Disk / +3 drives');
  expect(disk?.textContent).toContain('81 %');
  expect(disk?.getAttribute('title')).toBe('Disk /, 3 drives more on this device');
});
