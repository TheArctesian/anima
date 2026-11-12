import { describe, expect, it } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import type { AgentEnrolment, AgentStatusView } from '$lib/metrics/api';
import { AgentInstall, type AgentCalls, type PanelEnv } from './install.svelte';

const CODE = 'anima_enrol_AbCdEfGhIjKlMnOpQrStUv';
const ENV: PanelEnv = {
  now: () => new Date('2026-11-09T10:00:00Z'),
  clipboard: { writeText: () => Promise.resolve() },
  later: () => () => undefined,
};
const NEVER = () => Promise.reject(new Error('not called'));

describe('AgentInstall (MET-012)', () => {
  it('MET-012: an answer that arrives after the panel cleared the code does not bring it back', async () => {
    let answer: (result: HubResult<AgentEnrolment>) => void = () => undefined;
    const calls: AgentCalls = {
      enrolAgent: () => new Promise((resolve) => { answer = resolve; }),
      agentStatus: NEVER,
      revokeAgentToken: NEVER,
    };
    const install = new AgentInstall(calls, 'd1', ENV);
    const pending = install.makeCode();
    install.clear();
    answer({ ok: true, value: { code: CODE, expiresAt: '2026-11-09T10:15:00Z' } });
    await pending;
    expect(install.code).toBeNull();
  });

  it('MET-018: a failed read leaves no earlier state standing behind the error', async () => {
    const answers: HubResult<AgentStatusView | null>[] = [
      { ok: true, value: { metricsSource: 'AGENT', metricsStale: false } },
      { ok: false, error: { code: 'INTERNAL', message: 'down' } },
    ];
    const calls: AgentCalls = { enrolAgent: NEVER, agentStatus: () => Promise.resolve(answers.shift() ?? answers[0]), revokeAgentToken: NEVER };
    const install = new AgentInstall(calls, 'd1', ENV);
    await install.loadStatus();
    expect(install.status).toEqual({ metricsSource: 'AGENT', metricsStale: false });
    await install.loadStatus();
    expect(install.status).toBeNull();
    expect(install.statusError?.message).toBe('down');
  });

  it('MET-018: an older status answer does not replace a newer one', async () => {
    const waiting: ((result: HubResult<AgentStatusView | null>) => void)[] = [];
    const calls: AgentCalls = {
      enrolAgent: NEVER,
      agentStatus: () => new Promise((resolve) => { waiting.push(resolve); }),
      revokeAgentToken: NEVER,
    };
    const install = new AgentInstall(calls, 'd1', ENV);
    const first = install.loadStatus();
    const second = install.loadStatus();
    waiting[1]?.({ ok: true, value: { metricsSource: 'AGENT', metricsStale: true } });
    waiting[0]?.({ ok: true, value: { metricsSource: 'AGENT', metricsStale: false } });
    await Promise.all([first, second]);
    expect(install.status?.metricsStale).toBe(true);
  });

  it('MET-018: a status read that was pending when the token was revoked does not undo the revoke', async () => {
    let oldStatus: (result: HubResult<AgentStatusView | null>) => void = () => undefined;
    const calls: AgentCalls = {
      enrolAgent: NEVER,
      agentStatus: () => new Promise((resolve) => { oldStatus = resolve; }),
      revokeAgentToken: () => Promise.resolve({ ok: true, value: { metricsSource: 'AGENT', metricsStale: true } }),
    };
    const install = new AgentInstall(calls, 'd1', ENV);
    const loading = install.loadStatus();
    await install.revoke();
    oldStatus({ ok: true, value: { metricsSource: 'AGENT', metricsStale: false } });
    await loading;
    expect(install.revoked).toBe(true);
    expect(install.status?.metricsStale).toBe(true);
  });
});
