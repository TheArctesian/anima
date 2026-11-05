import { describe, expect, it } from 'vitest';
import type { UpdateState } from './schedule';
import { canCheck, describeUpdate } from './status';

const time = () => '14:30';
const snap = (state: UpdateState, nextCheckAt: number | null = null) => ({ state, nextCheckAt });

describe('describeUpdate', () => {
  it('UPD-006: says each state in words', () => {
    expect(describeUpdate(snap({ kind: 'idle' }), time)).toBe('Up to date');
    expect(describeUpdate(snap({ kind: 'checking' }), time)).toBe('Checking…');
    expect(describeUpdate(snap({ kind: 'downloading', version: '1.2.0' }), time)).toBe('Downloading Anima 1.2.0…');
    expect(describeUpdate(snap({ kind: 'installed', version: '1.2.0' }), time)).toBe('Anima 1.2.0 is installed — restart to update');
  });

  it('UPD-006: a failure names the time of the next check', () => {
    expect(describeUpdate(snap({ kind: 'failed' }, 5), time)).toBe('The last check failed; the next check is at 14:30.');
    expect(describeUpdate(snap({ kind: 'failed' }), time)).toBe('The last check failed.');
  });
});

describe('canCheck', () => {
  it('UPD-006: the button is off while checking, downloading and once installed', () => {
    expect(canCheck(snap({ kind: 'idle' }))).toBe(true);
    expect(canCheck(snap({ kind: 'failed' }))).toBe(true);
    expect(canCheck(snap({ kind: 'checking' }))).toBe(false);
    expect(canCheck(snap({ kind: 'downloading', version: '1' }))).toBe(false);
    expect(canCheck(snap({ kind: 'installed', version: '1' }))).toBe(false);
  });
});
