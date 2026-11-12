import { describe, expect, it } from 'vitest';
import type { HubError } from '$lib/hub/hub';
import { fakeHub } from '$lib/devices/hub.fixture';
import { metricsApi, type NewMetrics } from './api';
import { added, CPU_ONLY, INTEGRATED, NVIDIA } from './metrics.fixture';

const DENIED: HubError = { code: 'UNAUTHENTICATED', message: 'Sign in again.' };

describe('metricsApi', () => {
  it('series asks DeviceMetrics with the id and range and returns the device view', async () => {
    const fake = fakeHub({
      DeviceMetrics: { device: { id: 'd1', metricsSource: 'SSH', gpus: [NVIDIA, INTEGRATED], metrics: CPU_ONLY, storage: { drives: [] }, readings: null } },
    });
    const result = await metricsApi(fake.hub).series('d1', 'DAY');
    expect(fake.requests).toEqual([{ operation: 'DeviceMetrics', variables: { id: 'd1', range: 'DAY' } }]);
    expect(result).toEqual({ ok: true, value: { metricsSource: 'SSH', gpus: [NVIDIA, INTEGRATED], series: CPU_ONLY, drives: [], readings: null } });
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

  it('enrolAgent asks for the device and returns the code and its expiry', async () => {
    const enrolment = { code: 'anima_enrol_AbCdEfGhIjKlMnOpQrStUv', expiresAt: '2026-11-09T10:15:00Z' };
    const fake = fakeHub({ EnrolAgent: { enrolAgent: enrolment }, });
    expect(await metricsApi(fake.hub).enrolAgent('d1')).toEqual({ ok: true, value: enrolment });
    expect(fake.requests).toEqual([{ operation: 'EnrolAgent', variables: { deviceId: 'd1' } }]);
  });

  it('enrolAgent and revokeAgentToken pass a hub error through', async () => {
    const fake = fakeHub({ EnrolAgent: DENIED, RevokeAgentToken: DENIED });
    expect(await metricsApi(fake.hub).enrolAgent('d1')).toEqual({ ok: false, error: DENIED });
    expect(await metricsApi(fake.hub).revokeAgentToken('d1')).toEqual({ ok: false, error: DENIED });
  });

  it('revokeAgentToken returns the source and stale flag the hub answers', async () => {
    const fake = fakeHub({ RevokeAgentToken: { revokeAgentToken: { id: 'd1', metricsSource: 'AGENT', metricsStale: true } } });
    expect(await metricsApi(fake.hub).revokeAgentToken('d1')).toEqual({ ok: true, value: { metricsSource: 'AGENT', metricsStale: true } });
  });

  it('agentStatus returns the state, or null for an unknown device', async () => {
    const known = fakeHub({ AgentStatus: { device: { id: 'd1', metricsSource: 'AGENT', metricsStale: false } } });
    expect(await metricsApi(known.hub).agentStatus('d1')).toEqual({ ok: true, value: { metricsSource: 'AGENT', metricsStale: false } });
    expect(await metricsApi(fakeHub({ AgentStatus: { device: null } }).hub).agentStatus('x')).toEqual({ ok: true, value: null });
  });
});
