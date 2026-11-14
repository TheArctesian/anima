import { describe, expect, it } from 'vitest';
import { stepped } from './stepper';

const LIMIT = { min: 1, max: 100, empty: true };

describe('stepper (D17, RA-036)', () => {
  it('RA-036: one more from no limit is the least limit, and one fewer from it is no limit again', () => {
    expect(stepped('', 1, LIMIT)).toBe('1');
    expect(stepped('1', -1, LIMIT)).toBe('');
    expect(stepped('', -1, LIMIT)).toBe('');
  });

  it('RA-036: a step never leaves the bounds, and a value that cannot be empty stops at the least', () => {
    expect(stepped('100', 1, LIMIT)).toBe('100');
    expect(stepped('250', 1, LIMIT)).toBe('100');
    expect(stepped('1', -1, { ...LIMIT, empty: false })).toBe('1');
  });

  it('D17: half-typed text steps from the nearer end rather than from a guess at a number', () => {
    expect(stepped('4x', 1, LIMIT)).toBe('1');
    expect(stepped('4x', -1, LIMIT)).toBe('');
    expect(stepped(' 7 ', 1, LIMIT)).toBe('8');
  });
});
