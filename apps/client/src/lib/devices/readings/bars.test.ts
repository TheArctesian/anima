// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import DeviceCard from '../card/DeviceCard.svelte';
import { device } from '../screen/screen.fixture.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';
import { GPU, readings } from './readings.test';

const preference = vi.hoisted(() => ({ reduced: false }));
vi.mock('$lib/motion/preference', () => ({ motionPreference: () => preference }));

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

function card(over: Parameters<typeof device>[1]): void {
  app = mount(DeviceCard, { target: document.body, props: { device: device('a', over), beats: undefined, onConnect: () => undefined } });
  flushSync();
}

it('6.1 MET-007: a device without readings shows the empty state and no meter', () => {
  card({ metricsSource: 'NONE' });
  expect(document.querySelectorAll('[role="meter"]')).toHaveLength(0);
  expect(document.body.textContent).toContain('No metrics');
});

it('6.1: an SSH device without readings yet says it is waiting', () => {
  card({ metricsSource: 'SSH' });
  expect(document.body.textContent).toContain('Waiting for metrics');
});

it('MET-004 MET-008: a GPU device shows four meters with their values, VRAM last', () => {
  card({ metricsSource: 'SSH', readings: readings({ cpuPercent: 90, vramPercent: 40.4, vramGpu: GPU }) });
  const meters = [...document.querySelectorAll('[role="meter"]')];
  expect(meters.map((meter) => meter.getAttribute('aria-label'))).toEqual(['CPU', 'Memory', 'Disk', 'VRAM']);
  expect(meters.map((meter) => meter.getAttribute('aria-valuenow'))).toEqual(['90', '20', '30', '40.4']);
  expect(meters.every((m) => m.getAttribute('aria-valuemin') === '0' && m.getAttribute('aria-valuemax') === '100')).toBe(true);
  expect(meters[0]?.textContent).toContain('90 %');
  expect(meters[3]?.textContent).toContain('40 %');
  expect(meters[0]?.classList.contains('degraded')).toBe(true);
});
