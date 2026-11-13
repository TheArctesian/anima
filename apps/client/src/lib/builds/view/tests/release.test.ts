import { describe, expect, it } from 'vitest';
import type { Builder } from '../../api';
import type { JobChange, JobView, ReleaseView } from '../api';
import { attemptsByVersion, canCancel, newestFirst, shortCommit, stateAge, timeInState, waitingText, withChange } from '../release';

const NOW = new Date('2026-11-12T12:00:00Z');

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

function release(id: string, createdAt: string, jobs: JobView[] = []): ReleaseView {
  return { id, version: `0.1.${id}`, commit: 'a'.repeat(40), createdAt, createdBy: 'op', jobs };
}

const builder = (platform: Builder['platform'], online: boolean): Builder => ({ id: 'b', name: 'b', platform, online });

describe('Builds view logic', () => {
  it('BLD-080: releases are listed newest first', () => {
    const list = [release('1', '2026-11-01T00:00:00Z'), release('3', '2026-11-03T00:00:00Z'), release('2', '2026-11-02T00:00:00Z')];
    expect(newestFirst(list).map((item) => item.id)).toEqual(['3', '2', '1']);
  });

  it('BLD-080: the commit is shown short', () => {
    expect(shortCommit('0123456789abcdef0123456789abcdef01234567')).toBe('0123456');
  });

  it.each([
    ['2026-11-12T11:59:40Z', 'under a minute'],
    ['2026-11-12T11:59:00Z', '1 minute'],
    ['2026-11-12T11:15:30Z', '44 minutes'],
    ['2026-11-12T10:00:00Z', '2 hours'],
    ['2026-11-10T11:00:00Z', '2 days'],
    ['2026-11-12T12:00:05Z', 'under a minute'],
  ])('BLD-080: a job changed at %s has been in its state for %s', (changed, text) => {
    expect(timeInState({ stateChangedAt: changed }, NOW)).toBe(text);
  });

  it('BLD-007: a queued job with no online builder for its target waits for one', () => {
    expect(waitingText(job(), [])).toBe('Waiting for a linux-x86_64 builder');
    expect(waitingText(job({ target: 'DARWIN_UNIVERSAL' }), [])).toBe('Waiting for a darwin-universal builder');
  });

  it('BLD-007: an offline builder, or one of another platform, does not stop the wait', () => {
    const builders = [builder({ os: 'LINUX', arch: 'X86_64' }, false), builder({ os: 'MACOS', arch: 'AARCH64' }, true)];
    expect(waitingText(job(), builders)).toBe('Waiting for a linux-x86_64 builder');
  });

  it('BLD-007: an online builder that can build the target ends the wait', () => {
    expect(waitingText(job(), [builder({ os: 'LINUX', arch: 'X86_64' }, true)])).toBeNull();
    expect(waitingText(job({ target: 'DARWIN_UNIVERSAL' }), [builder({ os: 'MACOS', arch: 'AARCH64' }, true)])).toBeNull();
  });

  it('BLD-007: only a queued job waits', () => {
    expect(waitingText(job({ state: 'BUILDING' }), [])).toBeNull();
  });

  it('BLD-008: only a queued job can be cancelled', () => {
    expect(canCancel(job())).toBe(true);
    for (const state of ['CLAIMED', 'BUILDING', 'UPLOADING', 'PUBLISHED', 'FAILED', 'CANCELLED'] as const) {
      expect(canCancel(job({ state }))).toBe(false);
    }
  });

  it('BLD-007: nothing is said about waiting before the builders have loaded', () => {
    expect(waitingText(job(), null)).toBeNull();
  });

  it('BLD-081: a job change replaces that job in its release and leaves the rest', () => {
    const first = release('rel-1', '2026-11-01T00:00:00Z', [job(), job({ id: 'job-2' })]);
    const second = release('rel-2', '2026-11-02T00:00:00Z', [job({ id: 'job-3', releaseId: 'rel-2' })]);
    const change: JobChange = { job: job({ state: 'BUILDING' }), logLines: [] };
    const next = withChange([first, second], change);
    expect(next[0]?.jobs.map((item) => item.state)).toEqual(['BUILDING', 'QUEUED']);
    expect(next[1]).toBe(second);
  });

});

describe('release attempts and job ages', () => {
  it('BLD-080: releases of one version are grouped under the newest, in the place of the newest', () => {
    const withVersion = (id: string, version: string) => ({ ...release(id, '2026-11-01T00:00:00Z'), version });
    const groups = attemptsByVersion([withVersion('4', '0.2.0'), withVersion('3', '0.1.1'), withVersion('2', '0.1.0'), withVersion('1', '0.1.1')]);
    expect(groups.map((group) => [group.latest.id, group.earlier.map((earlier) => earlier.id)])).toEqual([
      ['4', []],
      ['3', ['1']],
      ['2', []],
    ]);
  });

  it('BLD-080: a running job says how long it has run, an ended one how long ago it ended', () => {
    expect(stateAge(job({ state: 'BUILDING', stateChangedAt: '2026-11-12T11:54:00Z' }), NOW)).toBe('for 6 minutes');
    expect(stateAge(job({ state: 'FAILED', stateChangedAt: '2026-11-12T11:42:00Z' }), NOW)).toBe('18 minutes ago');
    expect(stateAge(job({ state: 'PUBLISHED', stateChangedAt: '2026-11-12T11:59:30Z' }), NOW)).toBe('just now');
  });
});
