import { describe, expect, it } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import type { AcceptedHostKey, ConnectionTest } from '../../api';
import { PROBLEM, WARNED, transition, type HostKeyEvent, type HostKeyStep } from './hostkey';

const KEY = { algorithm: 'ssh-ed25519', fingerprintSha256: 'SHA256:new', firstSeenAt: '2026-11-04T10:00:00Z' };
const tested = (hostKey: typeof KEY | null, reachable = true): HubResult<ConnectionTest> => ({
  ok: true,
  value: { reachable, authenticated: true, hostKey, hostKeyChanged: hostKey !== null, message: 'hub text' },
});
const acceptedAs = (fingerprintSha256: string): HubResult<AcceptedHostKey> => ({ ok: true, value: { ...KEY, fingerprintSha256 } });
const refused: HubResult<never> = { ok: false, error: { code: 'CONFLICT', message: 'hub text' } };

/** The step after `events`; an event that does not apply leaves the step as it was. */
function run(...events: HostKeyEvent[]): HostKeyStep {
  return events.reduce((step, event) => transition(step, event) ?? step, WARNED);
}

const REVIEWED: HostKeyEvent[] = [{ event: 'review' }, { event: 'reviewed', result: tested(KEY) }];

describe('host key warning (SEC-007, D-6)', () => {
  it('SEC-007: there is no accept without a review first', () => {
    expect(run({ event: 'confirm' }, { event: 'accept' })).toEqual(WARNED);
  });

  it('SEC-007: accepting needs the review, then a second, explicit confirmation', () => {
    expect(run(...REVIEWED, { event: 'accept' }).step).toBe('reviewed');
    expect(run(...REVIEWED, { event: 'confirm' }, { event: 'accept' })).toEqual({
      step: 'accepting',
      key: { algorithm: KEY.algorithm, fingerprintSha256: KEY.fingerprintSha256 },
    });
  });

  it('a second accept while one is under way changes nothing (no double accept)', () => {
    const accepting = run(...REVIEWED, { event: 'confirm' }, { event: 'accept' });
    expect(transition(accepting, { event: 'accept' })).toBeUndefined();
  });

  it('SEC-007: the accept succeeds only when the hub holds exactly the reviewed fingerprint', () => {
    const accepting: HostKeyEvent[] = [...REVIEWED, { event: 'confirm' }, { event: 'accept' }];
    expect(run(...accepting, { event: 'accepted', result: acceptedAs('SHA256:new') }).step).toBe('accepted');
    expect(run(...accepting, { event: 'accepted', result: acceptedAs('SHA256:other') })).toEqual({ step: 'warned', problem: PROBLEM.accept });
    expect(run(...accepting, { event: 'accepted', result: refused })).toEqual({ step: 'warned', problem: PROBLEM.accept });
  });

  it('a stale fingerprint: no review mid-confirmation, and a new review replaces the reviewed key', () => {
    const confirming = run(...REVIEWED, { event: 'confirm' });
    expect(transition(confirming, { event: 'review' })).toBeUndefined();
    const again = run(...REVIEWED, { event: 'review' }, { event: 'reviewed', result: tested({ ...KEY, fingerprintSha256: 'SHA256:newer' }) });
    expect(again).toEqual({ step: 'reviewed', key: { algorithm: KEY.algorithm, fingerprintSha256: 'SHA256:newer' } });
  });

  it('answers that arrive in another step are ignored', () => {
    expect(transition(WARNED, { event: 'reviewed', result: tested(KEY) })).toBeUndefined();
    expect(transition(WARNED, { event: 'accepted', result: acceptedAs('SHA256:new') })).toBeUndefined();
  });

  it.each([
    [refused, PROBLEM.review],
    [tested(null), PROBLEM.noKey],
    [tested(null, false), PROBLEM.unreachable],
  ])('a review that reads no key goes back to the warning with fixed text', (result, problem) => {
    expect(run({ event: 'review' }, { event: 'reviewed', result })).toEqual({ step: 'warned', problem });
  });

  it('Back from the confirmation returns to the reviewed key', () => {
    expect(run(...REVIEWED, { event: 'confirm' }, { event: 'back' }).step).toBe('reviewed');
  });
});
