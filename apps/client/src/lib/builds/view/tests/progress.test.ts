import { describe, expect, it } from 'vitest';
import type { JobView } from '../api';
import { jobStages, releaseSummary } from '../card/progress';

function job(overrides: Partial<JobView> = {}): JobView {
  return {
    id: 'job-1',
    releaseId: 'rel-1',
    target: 'LINUX_X86_64',
    state: 'QUEUED',
    failureReason: null,
    attempts: 0,
    builderName: null,
    stateChangedAt: '2026-11-12T11:59:00Z',
    ...overrides,
  };
}

const marks = (overrides: Partial<JobView>) => jobStages(job(overrides)).map((stage) => `${stage.name}:${stage.mark}`);
const states = (...list: JobView['state'][]) => list.map((state) => ({ state }));

describe('job stages', () => {
  it.each([
    ['QUEUED', ['claim:current', 'build:pending', 'upload:pending', 'publish:pending']],
    ['CLAIMED', ['claim:done', 'build:current', 'upload:pending', 'publish:pending']],
    ['BUILDING', ['claim:done', 'build:current', 'upload:pending', 'publish:pending']],
    ['UPLOADING', ['claim:done', 'build:done', 'upload:current', 'publish:pending']],
    ['PUBLISHED', ['claim:done', 'build:done', 'upload:done', 'publish:done']],
    ['CANCELLED', ['claim:skipped', 'build:skipped', 'upload:skipped', 'publish:skipped']],
  ] as const)('BLD-080: a %s job is marked stage by stage', (state, expected) => {
    expect(marks({ state })).toEqual(expected);
  });

  it('BLD-080: a building job names its steps, with notarizing on a Mac', () => {
    expect(jobStages(job({ state: 'BUILDING' }))[1]?.detail).toBe('checkout, gates, bundle');
    expect(jobStages(job({ state: 'BUILDING', target: 'DARWIN_UNIVERSAL' }))[1]?.detail).toBe('checkout, gates, bundle, notarize');
  });

  it('BLD-080: a failure reason that names a build step fails the build stage at that step', () => {
    const stages = jobStages(job({ state: 'FAILED', failureReason: 'The gates failed. The job log says why.' }));
    expect(stages.map((stage) => stage.mark)).toEqual(['done', 'failed', 'skipped', 'skipped']);
    expect(stages[1]?.detail).toBe('the gates failed');
  });

  it('BLD-080: a failed upload leaves the build done', () => {
    expect(marks({ state: 'FAILED', failureReason: 'The upload could not run on this computer.' })).toEqual([
      'claim:done',
      'build:done',
      'upload:failed',
      'publish:skipped',
    ]);
  });

  it('BLD-080: a failure that names no step claims only that a builder held the job', () => {
    expect(marks({ state: 'FAILED', failureReason: 'builder stopped 3 times', builderName: 'mac' })[0]).toBe('claim:done');
    expect(marks({ state: 'FAILED', failureReason: 'builder stopped 3 times' })).toEqual([
      'claim:skipped',
      'build:skipped',
      'upload:skipped',
      'publish:skipped',
    ]);
  });
});

describe('release summary', () => {
  it.each([
    [states('BUILDING', 'FAILED'), 'Building', 'active'],
    [states('QUEUED', 'PUBLISHED'), 'Queued', 'waiting'],
    [states('FAILED', 'CANCELLED'), 'Failed', 'failed'],
    [states('PUBLISHED', 'PUBLISHED'), 'Published', 'done'],
    [states('PUBLISHED', 'CANCELLED'), 'Partly published', 'done'],
    [states('CANCELLED'), 'Cancelled', 'stopped'],
  ] as const)('BLD-080: jobs %j read as %s', (jobs, word, tone) => {
    expect(releaseSummary(jobs)).toMatchObject({ word, tone });
  });

  it('BLD-080: the summary counts the published targets', () => {
    expect(releaseSummary(states('PUBLISHED', 'BUILDING')).published).toBe('1 of 2 published');
  });
});
