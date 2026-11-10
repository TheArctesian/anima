// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CODE, showPending, ENROLMENT, click, command, setUp, show, tearDown, text } from './panel.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

beforeEach(setUp);
afterEach(tearDown);

describe('Install agent: an installed agent (MET-018, MET-013)', () => {
  const status = (metricsStale: boolean) => ({ AgentStatus: { device: { id: 'd1', metricsSource: 'AGENT', metricsStale } } });

  it('MET-018: shows the source as set by the agent and stale as a plain status, not an alert', async () => {
    const { fake } = await show(status(true), { source: 'AGENT' });
    expect(fake.requests).toEqual([{ operation: 'AgentStatus', variables: { id: 'd1' } }]);
    expect(text()).toContain('The source is set by the agent.');
    expect(document.querySelector('[role="status"]')?.textContent).toContain('Metrics are stale');
    expect(document.querySelector('[role="alert"]')).toBeNull();
  });

  it('MET-018: the "Reading the agent state." line is plain text, not a live region', async () => {
    const answer = await showPending('agentStatus', 'AGENT');
    const line = [...document.querySelectorAll('p')].find((each) => each.textContent.includes('Reading the agent state'));
    expect(line?.getAttribute('role')).toBeNull();
    await answer({ ok: true, value: { metricsSource: 'AGENT', metricsStale: false } });
  });

  it('MET-018: a current agent reads as current', async () => {
    await show(status(false), { source: 'AGENT' });
    expect(text()).toContain('Metrics are current.');
  });

  it('MET-018: when the state cannot be read the panel says so and still offers Revoke', async () => {
    await show({ AgentStatus: { code: 'INTERNAL', message: 'down' } }, { source: 'AGENT' });
    expect(text()).toContain('The agent state could not be read.');
    expect(text()).toContain('Revoke agent');
  });

  it('MET-013, D-7: confirming revokes the token and the panel then shows stale', async () => {
    const { fake } = await show(
      { ...status(false), RevokeAgentToken: { revokeAgentToken: { id: 'd1', metricsSource: 'AGENT', metricsStale: true } } },
      { source: 'AGENT' },
    );
    await click('Revoke agent');
    await click('Revoke agent', '[role="dialog"]');
    expect(fake.requests.at(-1)).toEqual({ operation: 'RevokeAgentToken', variables: { deviceId: 'd1' } });
    expect(text()).toContain('Agent revoked. Metrics stay stale until it enrols again or you change the source.');
  });
});

describe('Install agent: revoke and install again (MET-013, MET-019, D-7)', () => {
  const status = (metricsStale: boolean) => ({ AgentStatus: { device: { id: 'd1', metricsSource: 'AGENT', metricsStale } } });

  it('MET-013: a refused revoke keeps the dialog open and shows the hub message', async () => {
    await show({ ...status(false), RevokeAgentToken: { code: 'FORBIDDEN', message: 'Not allowed.' } }, { source: 'AGENT' });
    await click('Revoke agent');
    await click('Revoke agent', '[role="dialog"]');
    expect(document.querySelector('[role="dialog"] [role="alert"]')?.textContent).toBe('Not allowed.');
    expect(text()).not.toContain('Agent revoked.');
  });

  it('MET-019, D-7: an AGENT device can get a new install code, and Close returns to its state', async () => {
    const { fake } = await show({ ...status(true), ...ENROLMENT }, { source: 'AGENT' });
    await click('Install agent again');
    expect(fake.requests.at(-1)).toEqual({ operation: 'EnrolAgent', variables: { deviceId: 'd1' } });
    expect(command()).toContain(CODE);
    expect(text()).not.toContain('Revoke agent');
    await click('Close');
    expect(command()).toBe('');
    expect(text()).toContain('Revoke agent');
    expect(text()).toContain('Metrics are stale');
  });
});

