import { describe, expect, it } from 'vitest';
import { failureReason, JobReporter, LINES_PER_REPORT, PENDING_LINES_MAX } from '../report';
import { FakeBuildsApi, refused } from './builds.fixture';

function numbered(count: number, from = 0): string[] {
  return Array.from({ length: count }, (_, index) => `line ${String(from + index)}`);
}

describe('JobReporter (BLD-005, BLD-048)', () => {
  it('BLD-048: sends the lines in parts the hub accepts, with the state on the last part only', async () => {
    const api = new FakeBuildsApi();
    const reporter = new JobReporter(api, 'job-1');
    numbered(LINES_PER_REPORT * 2 + 5).forEach((line) => reporter.add(line));
    await expect(reporter.send('FAILED', 'why')).resolves.toBe('sent');
    expect(api.reports.map((report) => report.logLines.length)).toEqual([LINES_PER_REPORT, LINES_PER_REPORT, 5]);
    expect(api.reports.map((report) => report.state)).toEqual([null, null, 'FAILED']);
    expect(api.reports.map((report) => report.failureReason)).toEqual([null, null, 'why']);
    expect(api.lines()).toEqual(numbered(LINES_PER_REPORT * 2 + 5));
  });

  it('BLD-005: a report with no lines still goes, so the lease is extended', async () => {
    const api = new FakeBuildsApi();
    await new JobReporter(api, 'job-1').send(null);
    expect(api.reports).toEqual([expect.objectContaining({ state: null, logLines: [] })]);
  });

  it('BLD-048: lines of a report that did not arrive are sent with the next one', async () => {
    const api = new FakeBuildsApi();
    const reporter = new JobReporter(api, 'job-1');
    reporter.add('first');
    api.reportAnswer = () => refused('unreachable');
    await expect(reporter.send(null)).resolves.toBe('unreachable');
    api.reportAnswer = () => ({ ok: true, value: 'BUILDING' });
    reporter.add('second');
    await reporter.send(null);
    expect(api.lines()).toEqual(['first', 'second']);
  });

  it.each(['CONFLICT', 'NOT_FOUND', 'UNAUTHENTICATED', 'FORBIDDEN', 'VALIDATION'] as const)('BLD-010: a %s refusal means the job is lost', async (code) => {
    const api = new FakeBuildsApi();
    api.reportAnswer = () => refused(code);
    await expect(new JobReporter(api, 'job-1').send(null)).resolves.toBe('lost');
  });

  it('BLD-048: the queue is bounded; the oldest lines go and the next report says how many', async () => {
    const api = new FakeBuildsApi();
    const reporter = new JobReporter(api, 'job-1');
    numbered(PENDING_LINES_MAX + 3).forEach((line) => reporter.add(line));
    await reporter.send(null);
    const lines = api.lines();
    expect(lines[0]).toBe('[3 earlier lines were not sent]');
    expect(lines[1]).toBe('line 3');
    expect(lines.at(-1)).toBe(`line ${String(PENDING_LINES_MAX + 2)}`);
    expect(api.reports.every((report) => report.logLines.length <= LINES_PER_REPORT)).toBe(true);
  });
});

describe('failureReason (BLD-010)', () => {
  it('BLD-010, LOG-007: is fixed text for each code and never process output', () => {
    expect(failureReason('The gates', 'step-failed')).toBe('The gates failed. The job log says why.');
    expect(failureReason('Signing', 'key-missing')).toBe('No updater key is stored on this computer.');
    expect(failureReason('The bundle', 'not-desktop')).toBe('The bundle could not run on this computer.');
  });

  it('BLD-010: every reason fits the hub limit of 200 characters', () => {
    const codes = ['busy', 'commit', 'version', 'target', 'platform', 'not-on-main', 'dirty', 'tool-missing', 'step-failed', 'cancelled', 'bundle', 'key-missing', 'signed-version', 'signing', 'failed', 'not-desktop'] as const;
    for (const code of codes) expect(failureReason('The signature check', code).length).toBeLessThanOrEqual(200);
  });
});
