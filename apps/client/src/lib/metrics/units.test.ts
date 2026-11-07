import { describe, expect, it } from 'vitest';
import { formatBytesPerSecond, formatLoad, formatPercent } from './units';

describe('units (MET-007)', () => {
  it('writes percent as whole numbers', () => {
    expect(formatPercent(41.6)).toBe('42 %');
  });
  it('writes IEC bytes', () => {
    expect(formatBytesPerSecond(512)).toBe('512 B/s');
    expect(formatBytesPerSecond(1.5 * 1024 ** 3)).toBe('1.5 GiB/s');
    expect(formatBytesPerSecond(1024 * 1024)).toBe('1.0 MiB/s');
  });
  it('writes load with two decimals', () => {
    expect(formatLoad(0.4)).toBe('0.40');
  });
  it('shows a dash for a missing value, never 0, in every unit', () => {
    for (const format of [formatPercent, formatBytesPerSecond, formatLoad]) {
      expect(format(null)).toBe('—');
      expect(format(Number.NaN)).toBe('—');
    }
  });
  it('keeps a real zero as 0', () => {
    expect(formatPercent(0)).toBe('0 %');
  });
});
