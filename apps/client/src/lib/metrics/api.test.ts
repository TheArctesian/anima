import { describe, expect, it } from 'vitest';
import type { HubError } from '$lib/hub/hub';
import { fakeHub } from '$lib/devices/hub.fixture';
import { metricsApi, type NewMetrics } from './api';
import { added, CPU_ONLY, INTEGRATED, NVIDIA } from './metrics.fixture';

const DENIED: HubError = { code: 'UNAUTHENTICATED', message: 'Sign in again.' };

describe('metricsApi', () => {
  it('series asks DeviceMetrics with the id and range and returns the device view', async () => {
    const fake = fakeHub({
      DeviceMetrics: { device: { id: 'd1', metricsSource: 'SSH', gpus: [NVIDIA, INTEGRATED], metrics: CPU_ONLY } },
    });
    const result = await metricsApi(fake.hub).series('d1', 'DAY');
    expect(fake.requests).toEqual([{ operation: 'DeviceMetrics', variables: { id: 'd1', range: 'DAY' } }]);
    expect(result).toEqual({ ok: true, value: { metricsSource: 'SSH', gpus: [NVIDIA, INTEGRATED], series: CPU_ONLY } });
  });

  it('series returns null for an unknown device', async () => {
    const fake = fakeHub({ DeviceMetrics: { device: null } });
    expect(await metricsApi(fake.hub).series('gone', 'HOUR')).toEqual({ ok: true, value: null });
  });

  it('series passes a hub error through', async () => {
    const fake = fakeHub({ DeviceMetrics: DENIED });
    expect(await metricsApi(fake.hub).series('d1', 'HOUR')).toEqual({ ok: false, error: DENIED });
  });

  it('watch subscribes for the device, unwraps each batch, and stops', () => {
    const fake = fakeHub();
    const heard: (NewMetrics | HubError | 'complete')[] = [];
    const stop = metricsApi(fake.hub).watch('d1', {
      next: (value) => heard.push(value),
      error: (error) => heard.push(error),
      complete: () => heard.push('complete'),
    });
    const batch = added('2026-11-05T09:05:00Z', [{ kind: 'CPU_PERCENT', label: 'cpu', value: 20 }]);
    fake.push('MetricsAdded', { metricsAdded: batch });
    fake.fail('MetricsAdded', DENIED);
    stop();
    fake.push('MetricsAdded', { metricsAdded: batch });
    expect(fake.subscriptions).toEqual([{ operation: 'MetricsAdded', variables: { deviceId: 'd1' } }]);
    expect(heard).toEqual([batch, DENIED]);
    expect(fake.listeners('MetricsAdded')).toBe(0);
  });
});
