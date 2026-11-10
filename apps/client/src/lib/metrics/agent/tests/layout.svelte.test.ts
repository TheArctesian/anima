// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ENROLMENT, click, pass, setUp, show, tearDown } from './panel.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const flips = vi.hoisted(() => ({ count: 0 }));
vi.mock('$lib/motion/roles/flip', async (importOriginal) => {
  const real = await importOriginal<typeof import('$lib/motion/roles/flip')>();
  return {
    ...real,
    flip: (...args: Parameters<typeof real.flip>) => {
      flips.count += 1;
      return real.flip(...args);
    },
  };
});

beforeEach(() => {
  setUp();
  flips.count = 0;
});
afterEach(tearDown);

const status = (metricsStale: boolean) => ({ AgentStatus: { device: { id: 'd1', metricsSource: 'AGENT', metricsStale } } });
const REVOKED = { RevokeAgentToken: { revokeAgentToken: { id: 'd1', metricsSource: 'AGENT', metricsStale: true } } };

describe('Install agent: what lies below moves with every height change (MOT-002, 5.7.3)', () => {
  it('MOT-002: "Copied." appearing and going away each move what lies below', async () => {
    await show(ENROLMENT);
    await click('Install agent');
    await click('Copy command');
    const afterCopy = flips.count;
    await pass(5);
    expect(flips.count).toBeGreaterThan(afterCopy);
    expect(afterCopy).toBeGreaterThan(1);
  });

  it('MOT-002: an error alert and the revoked sentence each move what lies below', async () => {
    await show({ EnrolAgent: { code: 'FORBIDDEN', message: 'Not allowed.' } });
    const before = flips.count;
    await click('Install agent');
    expect(flips.count).toBeGreaterThan(before);
    document.body.replaceChildren();
  });

  it('MOT-002: the revoked sentence replacing "Metrics are current." moves what lies below', async () => {
    await show({ ...status(false), ...REVOKED }, { source: 'AGENT' });
    await click('Revoke agent');
    const before = flips.count;
    await click('Revoke agent', '[role="dialog"]');
    expect(flips.count).toBeGreaterThan(before);
  });
});

describe('Install agent: revoked and used codes (MET-013, MET-012)', () => {
  it('MET-013: after a new code the revoked sentence is gone, and Close reads the state again', async () => {
    const { fake } = await show({ ...status(true), ...REVOKED, ...ENROLMENT }, { source: 'AGENT' });
    await click('Revoke agent');
    await click('Revoke agent', '[role="dialog"]');
    expect(document.body.textContent).toContain('Agent revoked.');
    await click('Install agent again');
    await click('Close');
    expect(document.body.textContent).not.toContain('Agent revoked.');
    expect(fake.requests.filter((request) => request.operation === 'AgentStatus')).toHaveLength(2);
  });

  it('MET-012: when the code expires the state is read again', async () => {
    const { fake } = await show({ ...status(false), ...ENROLMENT }, { source: 'AGENT' });
    await click('Install agent again');
    await pass(15 * 60);
    expect(fake.requests.filter((request) => request.operation === 'AgentStatus')).toHaveLength(2);
  });

  it('MET-011: a device that was not an agent device shows the agent state once the hub says it enrolled', async () => {
    await show({ ...ENROLMENT, ...status(false) });
    await click('Install agent');
    await click('Close');
    expect(document.body.textContent).toContain('Metrics are current.');
  });
});
