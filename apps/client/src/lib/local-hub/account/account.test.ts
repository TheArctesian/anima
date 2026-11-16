import { describe, expect, it } from 'vitest';
import { accountFailure, checkAccount, firstProblem, groupedSecret, PASSWORD_MIN, secondsToNextCode } from './account';

const GOOD = { name: 'ann', password: 'correct horse', repeat: 'correct horse' };

describe('the account rules (FRN-003)', () => {
  it.each(['a', 'ann', 'Ann.Ops_2-x', '9lives', 'a'.repeat(64)])('FRN-003: %j is a good name', (name) => {
    expect(checkAccount({ ...GOOD, name }).name).toBeUndefined();
  });

  it.each(['', '.ann', '-ann', '_ann', 'ann ops', 'ann@hub', 'a'.repeat(65), 'änn'])('FRN-003: %j is refused', (name) => {
    expect(checkAccount({ ...GOOD, name }).name).toContain('1 to 64');
  });

  it('FRN-003: the password has at least 12 characters, and the repeat must match', () => {
    expect(checkAccount(GOOD)).toEqual({});
    expect(checkAccount({ ...GOOD, password: 'x'.repeat(PASSWORD_MIN - 1), repeat: 'x'.repeat(PASSWORD_MIN - 1) }).password).toBe('Use at least 12 characters.');
    expect(checkAccount({ ...GOOD, repeat: 'correct horsf' })).toEqual({ repeat: 'The two passwords are not the same.' });
  });

  it('LOG-007: no problem text repeats the password', () => {
    expect(JSON.stringify(checkAccount({ name: '', password: 'short-pw', repeat: 'other' }))).not.toContain('short-pw');
  });

  it('focus goes to the first field in page order with a problem', () => {
    expect(firstProblem({ repeat: 'x', name: 'y' })).toBe('name');
    expect(firstProblem({})).toBeUndefined();
  });
});

describe('the authenticator (FRN-004)', () => {
  it('FRN-004: the secret is shown in groups of four', () => {
    expect(groupedSecret('JBSWY3DPEHPK3PXPJB')).toBe('JBSW Y3DP EHPK 3PXP JB');
    expect(groupedSecret('')).toBe('');
  });

  it('FRN-004: the next code comes at the next 30-second step', () => {
    expect(secondsToNextCode(new Date('2026-11-15T10:00:00.000Z'))).toBe(30);
    expect(secondsToNextCode(new Date('2026-11-15T10:00:00.999Z'))).toBe(30);
    expect(secondsToNextCode(new Date('2026-11-15T10:00:29.500Z'))).toBe(1);
    expect(secondsToNextCode(new Date('2026-11-15T10:00:47.000Z'))).toBe(13);
  });

  it('FRN-004, FRN-007: a wrong code and a cancelled recovery have their own fixed text', () => {
    expect(accountFailure('refused')).toBe('The code did not match. Wait for a new code and try again.');
    expect(accountFailure('cancelled')).toBe('Recovery was cancelled. Nothing changed.');
    expect(accountFailure('busy')).toBe('Another change to the hub is running. Wait for it to finish.');
  });
});
