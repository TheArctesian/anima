import { describe, expect, it } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import type { Hub, HubResult } from '$lib/hub/hub';
import { CPU_ONLY, added, point, series } from '$lib/metrics/metrics.fixture';
import { DeviceMetrics } from './metrics.svelte';

const ID = '0190a000-0000-7000-8012-000000000004';
const LOADED = new Date('2026-11-05T09:05:00Z');
const answer = (metrics = CPU_ONLY) => ({ device: { id: ID, metricsSource: 'SSH', gpus: [], metrics } });

function metricsOf(fake: ReturnType<typeof fakeHub>, now = LOADED) {
  return new DeviceMetrics(fake.hub, ID, () => now);
}

describe('DeviceMetrics (D-5, D-6, D-7)', () => {
  it('D-7: a device without SSH metrics sends no request and no subscription', async () => {
    const fake = fakeHub({ DeviceMetrics: answer() });
    const metrics = metricsOf(fake);
    await metrics.load('HOUR', 'NONE');
    expect(fake.requests).toEqual([]);
    expect(fake.subscriptions).toEqual([]);
    expect(metrics.view).toBeNull();
  });

  it('MOT-007, 5.7.3 Loading: the chart count of the last answer is kept for the skeleton, and is null before one', async () => {
    const fake = fakeHub({ DeviceMetrics: answer() });
    const metrics = metricsOf(fake);
    expect(metrics.chartCount).toBeNull();
    await metrics.load('HOUR', 'SSH');
    expect(metrics.chartCount).toBe(2);
  });

  it('MET-003: the request carries the range, and the window ends when the load began', async () => {
    const fake = fakeHub({ DeviceMetrics: answer() });
    const metrics = metricsOf(fake);
    await metrics.load('DAY', 'SSH');
    expect(fake.requests).toEqual([{ operation: 'DeviceMetrics', variables: { id: ID, range: 'DAY' } }]);
    expect(metrics.view?.series).toEqual(CPU_ONLY);
    expect(metrics.span).toEqual({ start: LOADED.getTime() - 24 * 3_600_000, end: LOADED.getTime() });
  });

  it('D-7: the hub error is kept, and the next load clears it', async () => {
    const failing = fakeHub({ DeviceMetrics: { code: 'internal', message: 'metrics are not served' } });
    const metrics = metricsOf(failing);
    await metrics.load('HOUR', 'SSH');
    expect(metrics.error?.message).toBe('metrics are not served');
    expect(metrics.view).toBeNull();
    const pending = metrics.load('HOUR', 'SSH');
    expect(metrics.error).toBeNull();
    await pending;
  });

});

describe('DeviceMetrics answers (D-6)', () => {
  it('D-6: a newer load drops an older answer', async () => {
    const waiting: ((result: HubResult<unknown>) => void)[] = [];
    const base = fakeHub();
    const hub: Hub = {
      ...base.hub,
      request: () => new Promise((resolve) => waiting.push(resolve as never)),
    };
    const metrics = new DeviceMetrics(hub, ID, () => LOADED);
    const first = metrics.load('HOUR', 'SSH');
    const second = metrics.load('DAY', 'SSH');
    waiting[1]?.({ ok: true, value: answer([series('CPU_PERCENT', 'cpu', [point('2026-11-05T09:00:00Z', 50)])]) });
    await second;
    waiting[0]?.({ ok: true, value: answer() });
    await first;
    expect(metrics.view?.series).toHaveLength(1);
    expect(metrics.span?.end).toBe(LOADED.getTime());
  });

  it('D-6: after stop an answer is dropped and the subscription ends', async () => {
    const waiting: ((result: HubResult<unknown>) => void)[] = [];
    const base = fakeHub();
    const hub: Hub = { ...base.hub, request: () => new Promise((resolve) => waiting.push(resolve as never)) };
    const metrics = new DeviceMetrics(hub, ID, () => LOADED);
    const pending = metrics.load('HOUR', 'SSH');
    expect(base.listeners('MetricsAdded')).toBe(1);
    metrics.stop();
    expect(base.listeners('MetricsAdded')).toBe(0);
    waiting[0]?.({ ok: true, value: answer() });
    await pending;
    expect(metrics.view).toBeNull();
  });

});

describe('DeviceMetrics live points (D-5)', () => {
  it('MET-003: the shown range stays the old one until the new answer lands', async () => {
    const waiting: ((result: HubResult<unknown>) => void)[] = [];
    const base = fakeHub({ DeviceMetrics: answer() });
    let held = false;
    const hub: Hub = {
      ...base.hub,
      request: (document, variables) => (held ? new Promise((resolve) => waiting.push(resolve as never)) : base.hub.request(document, variables)),
    };
    const metrics = new DeviceMetrics(hub, ID, () => LOADED);
    await metrics.load('HOUR', 'SSH');
    held = true;
    const pending = metrics.load('DAY', 'SSH');
    expect(metrics.shownRange).toBe('HOUR');
    expect(metrics.view).not.toBeNull();
    waiting[0]?.({ ok: true, value: answer() });
    await pending;
    expect(metrics.shownRange).toBe('DAY');
  });

  it('D-5: the 1-hour range merges a live point and ends the window at its time', async () => {
    const fake = fakeHub({ DeviceMetrics: answer() });
    const metrics = metricsOf(fake);
    await metrics.load('HOUR', 'SSH');
    const sample = [{ kind: 'CPU_PERCENT' as const, label: 'cpu', value: 40 }];
    fake.push('MetricsAdded', { metricsAdded: added('2026-11-05T09:06:00Z', sample) });
    const cpu = metrics.view?.series.find((one) => one.kind === 'CPU_PERCENT');
    expect(cpu?.points.at(-1)).toMatchObject({ at: '2026-11-05T09:06:00Z', value: 40 });
    const end = Date.parse('2026-11-05T09:06:00Z');
    expect(metrics.span).toEqual({ start: end - 3_600_000, end });
    expect(metrics.readings?.cpuPercent).toBeNull();
  });

  it('D-5: the window moves only when a point arrives, and never backwards', async () => {
    let time = LOADED;
    const fake = fakeHub({ DeviceMetrics: answer() });
    const metrics = new DeviceMetrics(fake.hub, ID, () => time);
    await metrics.load('HOUR', 'SSH');
    const before = metrics.span;
    time = new Date(LOADED.getTime() + 600_000);
    expect(metrics.span).toBe(before);
    fake.push('MetricsAdded', { metricsAdded: added('2026-11-05T09:01:00Z', [{ kind: 'CPU_PERCENT', label: 'cpu', value: 1 }]) });
    expect(metrics.span).toBe(before);
  });

});

describe('DeviceMetrics clock and readings (D-5)', () => {
  it('D-5: a client clock ahead of the hub still gets the live point, and the window does not move back', async () => {
    const ahead = new Date('2026-11-05T09:30:00Z');
    const fake = fakeHub({ DeviceMetrics: answer() });
    const metrics = new DeviceMetrics(fake.hub, ID, () => ahead);
    await metrics.load('HOUR', 'SSH');
    fake.push('MetricsAdded', { metricsAdded: added('2026-11-05T09:05:00Z', [{ kind: 'CPU_PERCENT', label: 'cpu', value: 40 }]) });
    expect(metrics.view?.series.find((one) => one.kind === 'CPU_PERCENT')?.points.at(-1)?.value).toBe(40);
    expect(metrics.span?.end).toBe(ahead.getTime());
  });

  it('MET-004: the readings come from the hub at load and a live batch replaces them', async () => {
    const readings = { at: '2026-11-05T09:04:00Z', cpuPercent: 12, memoryPercent: 45, diskPercent: 61, vramPercent: null, vramGpu: null };
    const fake = fakeHub({ DeviceMetrics: { device: { ...answer().device, readings } } });
    const metrics = metricsOf(fake);
    await metrics.load('HOUR', 'SSH');
    expect(metrics.readings?.cpuPercent).toBe(12);
    fake.push('MetricsAdded', { metricsAdded: added('2026-11-05T09:06:00Z', []) });
    expect(metrics.readings?.cpuPercent).toBeNull();
  });

  it('D-7: an error clears the old view', async () => {
    const fake = fakeHub({ DeviceMetrics: answer() });
    const metrics = metricsOf(fake);
    await metrics.load('HOUR', 'SSH');
    expect(metrics.view).not.toBeNull();
    const failing = fakeHub({ DeviceMetrics: { code: 'INTERNAL', message: 'down' } });
    const again = metricsOf(failing);
    await again.load('DAY', 'SSH');
    expect(again.view).toBeNull();
    expect(again.span).toBeNull();
  });
});

describe('DeviceMetrics ranges (D-5)', () => {
  it('D-5: DAY and WEEK do not subscribe and take no live points', async () => {
    const fake = fakeHub({ DeviceMetrics: answer() });
    const metrics = metricsOf(fake);
    await metrics.load('WEEK', 'SSH');
    expect(fake.subscriptions).toEqual([]);
    fake.push('MetricsAdded', { metricsAdded: added('2026-11-05T09:06:00Z', []) });
    expect(metrics.view?.series).toEqual(CPU_ONLY);
  });

  it('D-5: a live point does not join a DAY view still shown after a switch to HOUR', async () => {
    const waiting: ((result: HubResult<unknown>) => void)[] = [];
    const base = fakeHub({ DeviceMetrics: answer() });
    let held = false;
    const hub: Hub = {
      ...base.hub,
      request: (document, variables) => {
        if (!held) return base.hub.request(document, variables);
        return new Promise((resolve) => waiting.push(resolve as never));
      },
    };
    const metrics = new DeviceMetrics(hub, ID, () => LOADED);
    await metrics.load('DAY', 'SSH');
    held = true;
    const pending = metrics.load('HOUR', 'SSH');
    const sample = [{ kind: 'CPU_PERCENT' as const, label: 'cpu', value: 40 }];
    base.push('MetricsAdded', { metricsAdded: added('2026-11-05T09:06:00Z', sample) });
    expect(metrics.view?.series).toEqual(CPU_ONLY);
    waiting[0]?.({ ok: true, value: answer() });
    await pending;
  });

  it('D-5: leaving HOUR ends the subscription, and a live failure is kept', async () => {
    const fake = fakeHub({ DeviceMetrics: answer() });
    const metrics = metricsOf(fake);
    await metrics.load('HOUR', 'SSH');
    fake.fail('MetricsAdded', { code: 'unreachable', message: 'The hub did not answer.' });
    expect(metrics.liveError?.message).toBe('The hub did not answer.');
    await metrics.load('DAY', 'SSH');
    expect(fake.listeners('MetricsAdded')).toBe(0);
    expect(metrics.liveError).toBeNull();
  });
});
