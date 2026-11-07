import { describe, expect, it } from 'vitest';
import {
  formatBytes,
  formatBytesPerSecond,
  formatLoad,
  formatPercent,
  formatPower,
  formatTemperature,
  formatUptime,
} from './units';

describe('units (MET-007)', () => {
  it('writes percent as whole numbers', () => {
    expect(formatPercent(41.6)).toBe('42 %');
  });
  it('writes IEC bytes', () => {
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(1.5 * 1024 ** 3)).toBe('1.5 GiB');
    expect(formatBytesPerSecond(1024 * 1024)).toBe('1.0 MiB/s');
  });
  it('writes uptime with its two largest units', () => {
    expect(formatUptime(30)).toBe('30 s');
    expect(formatUptime(42 * 60)).toBe('42 min');
    expect(formatUptime(5 * 3600 + 12 * 60)).toBe('5 h 12 min');
    expect(formatUptime(3 * 86400 + 4 * 3600)).toBe('3 d 4 h');
  });
  it('writes load, temperature and power', () => {
    expect(formatLoad(0.4)).toBe('0.40');
    expect(formatTemperature(61.6)).toBe('62 °C');
    expect(formatPower(45.2)).toBe('45 W');
  });
  it('shows a dash for a missing value, never 0, in every unit', () => {
    for (const format of [formatPercent, formatBytes, formatBytesPerSecond, formatUptime, formatLoad, formatTemperature, formatPower]) {
      expect(format(null)).toBe('—');
      expect(format(Number.NaN)).toBe('—');
    }
  });
  it('keeps a real zero as 0', () => {
    expect(formatPercent(0)).toBe('0 %');
  });
});
