import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { vi } from 'vitest';
import { builderPass, buildJob, POLL_MS, PUBLISHING_LATER, startBuilderLoop, type BuilderStatus, type LoopPorts } from '../loop';
import { BUILDER_TOKEN, FakeBuildsApi, FakeShell, JOB, refused, sleep } from './builds.fixture';

/** The hub's limit between two reports of a running job (BLD-005). */
const REPORT_LIMIT_MS = 15_000;

let shell: FakeShell;
let api: FakeBuildsApi;
let statuses: BuilderStatus[];
let ports: LoopPorts;

beforeEach(() => {
  vi.useFakeTimers({ now: 0 });
  shell = new FakeShell(sleep);
  api = new FakeBuildsApi();
  statuses = [];
  ports = { api, shell, secrets: () => [BUILDER_TOKEN], sleep, onStatus: (status) => statuses.push(status) };
});

afterEach(() => {
  vi.useRealTimers();
});

/** Runs `work` to its end while the fake clock moves on. */
async function settle<T>(work: Promise<T>): Promise<T> {
  let done = false;
  void work.finally(() => (done = true));
  while (!done) await vi.advanceTimersByTimeAsync(1_000);
  return work;
}

describe('a pass of the builder loop', () => {
  it('BLD-028: a builder with a missing tool names it and does not claim', async () => {
    shell.missing(['postgres', 'updater-key']);
    await expect(settle(builderPass(ports))).resolves.toBe(POLL_MS);
    expect(api.claims).toBe(0);
    expect(statuses).toEqual([{ kind: 'missing', tools: ['postgres', 'updater-key'] }]);
  });

  it('BLD-028, D-19: a locked updater key is its own state, and the builder claims nothing until it is unlocked', async () => {
    shell.missing(['updater-key-locked']);
    await expect(settle(builderPass(ports))).resolves.toBe(POLL_MS);
    expect(api.claims).toBe(0);
    shell.missing(['git', 'updater-key-locked']);
    await settle(builderPass(ports));
    expect(api.claims).toBe(0);
    expect(statuses).toEqual([{ kind: 'locked' }, { kind: 'missing', tools: ['git', 'updater-key-locked'] }]);
    shell.missing([]);
    await settle(builderPass(ports));
    expect(api.claims).toBe(1);
  });

  it('BLD-028: a computer that cannot build, or whose tool check fails, does not claim', async () => {
    shell.toolsFail = 'platform';
    await settle(builderPass(ports));
    shell.toolsFail = 'failed';
    await settle(builderPass(ports));
    expect(api.claims).toBe(0);
    expect(statuses.map((status) => status.kind)).toEqual(['unsupported', 'tools-unknown']);
  });

  it('BLD-004: with no job waiting the builder waits; a refused token shows as refused', async () => {
    await expect(settle(builderPass(ports))).resolves.toBe(POLL_MS);
    api.claimAnswer = refused('UNAUTHENTICATED');
    await settle(builderPass(ports));
    api.claimAnswer = refused('unreachable');
    await settle(builderPass(ports));
    expect(statuses.map((status) => status.kind)).toEqual(['waiting', 'refused', 'unreachable']);
  });

  it('BLD-029: a claimed job is built, then the next pass follows at once', async () => {
    api.jobs = [JOB];
    await expect(settle(builderPass(ports))).resolves.toBe(0);
    expect(shell.calls).toEqual(['checkout', 'gates', 'bundle', 'sign', 'verify']);
  });
});

describe('building one job', () => {
  it('BLD-010, D-10: reports BUILDING, runs every step in order and stops at the upload point with a fixed reason', async () => {
    await settle(buildJob(JOB, ports));
    expect(api.reports[0]).toMatchObject({ id: JOB.id, state: 'BUILDING' });
    expect(shell.calls).toEqual(['checkout', 'gates', 'bundle', 'sign', 'verify']);
    expect(api.reports.at(-1)).toMatchObject({ state: 'FAILED', failureReason: PUBLISHING_LATER });
    expect(statuses.map((status) => (status.kind === 'building' ? status.step : status.kind))).toEqual(['checkout', 'gates', 'bundle', 'sign', 'verify']);
  });

  it('BLD-005: a long step reports at least every 15 s', async () => {
    shell.scripts.gates = { ms: 95_000 };
    await settle(buildJob(JOB, ports));
    const times = api.reports.map((report) => report.at);
    const gaps = times.slice(1).map((at, index) => at - (times[index] ?? 0));
    expect(Math.max(...gaps)).toBeLessThanOrEqual(REPORT_LIMIT_MS);
    expect(times.at(-1)).toBeGreaterThanOrEqual(95_000);
  });

  it('BLD-048, BLD-049: each output line reaches the hub with the builder token and key material removed', async () => {
    shell.scripts.gates = { lines: [`token=${BUILDER_TOKEN} ok`, 'cargo test: 182 passed', 'TAURI_SIGNING_PRIVATE_KEY=dW50cnVzdGVkIGNvbW1lbnQ6abc'] };
    await settle(buildJob(JOB, ports));
    const lines = api.lines();
    expect(lines).toContain('cargo test: 182 passed');
    expect(lines.join('\n')).not.toContain(BUILDER_TOKEN);
    expect(lines.join('\n')).not.toContain('dW50cnVzdGVkIGNvbW1lbnQ6abc');
  });

});

describe('a job that does not finish', () => {
  it('BLD-042, BLD-010: a failed step ends the job as failed with a fixed reason, and no later step runs', async () => {
    shell.scripts.gates = { lines: ['clippy: error'], fails: 'step-failed' };
    await settle(buildJob(JOB, ports));
    expect(shell.calls).toEqual(['checkout', 'gates']);
    expect(api.reports.at(-1)).toMatchObject({ state: 'FAILED', failureReason: 'The gates failed. The job log says why.' });
    expect(api.lines()).toContain('clippy: error');
  });

  it('BLD-041: a commit that is not on main fails the job before the gates', async () => {
    shell.scripts.checkout = { fails: 'not-on-main' };
    await settle(buildJob(JOB, ports));
    expect(shell.calls).toEqual(['checkout']);
    expect(api.reports.at(-1)).toMatchObject({ failureReason: 'The commit is not on main of the Anima repository.' });
  });

  it('BLD-010: when the hub refuses a report the job is no longer this builder’s: the step is cancelled and nothing more is sent', async () => {
    shell.scripts.gates = { ms: 60_000 };
    api.reportAnswer = (report) => (report.state === 'BUILDING' ? { ok: true, value: 'BUILDING' } : refused('CONFLICT'));
    await settle(buildJob(JOB, ports));
    expect(shell.cancels).toBe(1);
    expect(shell.calls).toEqual(['checkout', 'gates']);
    expect(api.reports).toHaveLength(1);
  });

  it('BLD-005: when the hub stays unreachable past the lease the step is cancelled', async () => {
    shell.scripts.gates = { ms: 600_000 };
    api.reportAnswer = (report) => (report.state === 'BUILDING' ? { ok: true, value: 'BUILDING' } : refused('unreachable'));
    await settle(buildJob(JOB, ports));
    expect(shell.cancels).toBe(1);
    expect(Date.now()).toBeLessThan(120_000);
  });

  it('BLD-026: once the builder is stopping no further step starts, and the job ends as cancelled', async () => {
    let stopping = false;
    shell.scripts.checkout = { ms: 1_000 };
    const work = buildJob(JOB, ports, () => stopping);
    await vi.advanceTimersByTimeAsync(500);
    stopping = true;
    await settle(work);
    expect(shell.calls).toEqual(['checkout']);
    expect(api.reports.at(-1)).toMatchObject({ state: 'FAILED', failureReason: 'The build was cancelled on this computer.' });
  });

  it('BLD-010: a job whose BUILDING report is refused runs no step', async () => {
    api.reportAnswer = () => refused('CONFLICT');
    await settle(buildJob(JOB, ports));
    expect(shell.calls).toEqual([]);
  });
});

describe('startBuilderLoop', () => {
  it('BLD-026: stopping cancels the running step, ends the loop and shows stopped', async () => {
    api.jobs = [JOB];
    shell.scripts.gates = { ms: 600_000 };
    const stop = startBuilderLoop(ports);
    await vi.advanceTimersByTimeAsync(5_000);
    expect(shell.calls).toEqual(['checkout', 'gates']);
    await settle(stop());
    expect(shell.cancels).toBeGreaterThanOrEqual(1);
    expect(api.reports.at(-1)).toMatchObject({ state: 'FAILED', failureReason: 'The build was cancelled on this computer.' });
    expect(statuses.at(-1)).toEqual({ kind: 'stopped' });
  });

  it('BLD-007: an idle builder claims again after the poll interval, and stops while it waits', async () => {
    const stop = startBuilderLoop(ports);
    await vi.advanceTimersByTimeAsync(POLL_MS + 1_000);
    expect(api.claims).toBe(2);
    await settle(stop());
    await vi.advanceTimersByTimeAsync(POLL_MS * 3);
    expect(api.claims).toBe(2);
  });
});
