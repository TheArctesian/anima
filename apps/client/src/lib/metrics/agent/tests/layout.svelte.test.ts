// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { AgentCalls } from '../install.svelte';
import { CODE, ENROLMENT, click, pass, settle, setUp, show, showPending, tearDown } from './panel.fixture';

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

/** How many times what lies below moved while `step` ran. */
async function flipsDuring(step: () => Promise<unknown>): Promise<number> {
  const before = flips.count;
  await step();
  return flips.count - before;
}

describe('Install agent: what lies below moves with every height change (MOT-002, 5.7.3)', () => {
  it('MOT-002: "Copied." appearing and going away each move what lies below once', async () => {
    await show(ENROLMENT);
    await click('Install agent');
    expect(await flipsDuring(() => click('Copy command'))).toBe(1);
    expect(await flipsDuring(() => pass(5))).toBe(1);
  });

  it('MOT-002: an error alert moves what lies below once', async () => {
    await show({ EnrolAgent: { code: 'FORBIDDEN', message: 'Not allowed.' } });
    expect(await flipsDuring(() => click('Install agent'))).toBe(1);
  });

  it('MOT-002: the revoked sentence replacing "Metrics are current." moves what lies below once', async () => {
    await show({ ...status(false), ...REVOKED }, { source: 'AGENT' });
    await click('Revoke agent');
    expect(await flipsDuring(() => click('Revoke agent', '[role="dialog"]'))).toBe(1);
  });

  it('MOT-002: "The code expired." going while the next code is made moves what lies below once', async () => {
    const enrolAgent = vi
      .fn<AgentCalls['enrolAgent']>()
      .mockResolvedValueOnce({ ok: true, value: { code: CODE, expiresAt: '2026-11-09T10:15:00Z' } })
      .mockReturnValueOnce(new Promise(() => undefined));
    await show({ AgentStatus: { device: null } }, { enrolAgent });
    await click('Install agent');
    await pass(15 * 60);
    expect(document.body.textContent).toContain('The code expired.');
    expect(await flipsDuring(() => click('Install agent'))).toBe(1);
    expect(document.body.textContent).not.toContain('The code expired.');
  });

  it('MOT-002: a failed agent state read moves what lies below once', async () => {
    const answer = await showPending('agentStatus', 'AGENT');
    expect(await flipsDuring(() => answer({ ok: false, error: { code: 'INTERNAL', message: 'down' } }))).toBe(1);
    expect(document.body.textContent).toContain('The agent state could not be read.');
  });

  it('MOT-002: choosing another computer, which changes the command, moves what lies below once', async () => {
    await show(ENROLMENT);
    await click('Install agent');
    const select = document.querySelector('select');
    if (select === null) throw new Error('no computer choice');
    const moved = await flipsDuring(async () => {
      select.value = 'macos-aarch64';
      select.dispatchEvent(new Event('change'));
      await settle();
    });
    expect(moved).toBe(1);
  });
});
