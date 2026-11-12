import { describe, expect, it } from 'vitest';
import type { UpdateState } from '../schedule';
import { canCheck, describeUpdate } from '../status';

const time = () => '14:30';
const snap = (state: UpdateState, nextCheckAt: number | null = null) => ({ state, nextCheckAt });

describe('describeUpdate', () => {
  it('UPD-006: says each state in words', () => {
    expect(describeUpdate(snap({ kind: 'unchecked' }), time)).toBe('Not checked yet');
    expect(describeUpdate(snap({ kind: 'idle' }), time)).toBe('Up to date');
    expect(describeUpdate(snap({ kind: 'checking' }), time)).toBe('Checking…');
    expect(describeUpdate(snap({ kind: 'downloading', version: '1.2.0' }), time)).toBe('Downloading Anima 1.2.0…');
    expect(describeUpdate(snap({ kind: 'installed', version: '1.2.0' }), time)).toBe('Anima 1.2.0 is installed — restart to update');
  });

  it('UPD-006: a failure names the time of the next check', () => {
    expect(describeUpdate(snap({ kind: 'failed', reason: 'other' }, 5), time)).toBe('The last update attempt failed. The next check is at 14:30.');
    expect(describeUpdate(snap({ kind: 'failed', reason: 'other' }), time)).toBe('The last update attempt failed.');
  });

  it('UPD-006, BLD-068: a missing feed for the target says so, the same on a Mac as on Linux', () => {
    const failed = snap({ kind: 'failed', reason: 'no-feed' });
    expect(describeUpdate(failed, time)).toBe('No update is published for this computer yet.');
  });

  it('UPD-002: a refused signature says the update was refused and the version kept', () => {
    expect(describeUpdate(snap({ kind: 'failed', reason: 'signature' }, 5), time)).toBe(
      'An update was refused because its signature did not verify. Anima kept the version it has. The next check is at 14:30.',
    );
  });
});

describe('canCheck', () => {
  it('UPD-006: the button is off while checking, downloading and once installed', () => {
    expect(canCheck(snap({ kind: 'unchecked' }))).toBe(true);
    expect(canCheck(snap({ kind: 'idle' }))).toBe(true);
    expect(canCheck(snap({ kind: 'failed', reason: 'other' }))).toBe(true);
    expect(canCheck(snap({ kind: 'checking' }))).toBe(false);
    expect(canCheck(snap({ kind: 'downloading', version: '1' }))).toBe(false);
    expect(canCheck(snap({ kind: 'installed', version: '1' }))).toBe(false);
  });
});
