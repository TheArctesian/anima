import { describe, expect, it } from 'vitest';
import { dayCount } from './days';

describe('dayCount (CHK-007, D-29)', () => {
  it('uses "day" for one and minus one, "days" for everything else', () => {
    expect(dayCount(1)).toBe('1 day');
    expect(dayCount(-1)).toBe('-1 day');
    expect(dayCount(0)).toBe('0 days');
    expect(dayCount(12)).toBe('12 days');
    expect(dayCount(-5)).toBe('-5 days');
  });
});
