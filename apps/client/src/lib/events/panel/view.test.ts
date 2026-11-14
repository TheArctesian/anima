import { describe, expect, it } from 'vitest';
import { DEFAULT_LOG_FILTER } from '../log';
import { SEVERITY_CHOICES, clockLabel, dayLabel, deviceLabel, labelOf, switched, timeLabel, withDevice, withSince } from './view';

describe('time labels', () => {
  const now = new Date(2026, 10, 4, 15, 0, 0);

  it('LOG-009: a time with no day heading shows the date when it is not today', () => {
    expect(timeLabel(new Date(2026, 10, 4, 9, 5, 7).toISOString(), now)).toBe('09:05:07');
    expect(timeLabel(new Date(2026, 9, 31, 23, 59, 1).toISOString(), now)).toBe('10-31 23:59:01');
    expect(timeLabel('not a time', now)).toBe('not a time');
  });

  it('LOG-009: a row shows the clock time in the local zone', () => {
    expect(clockLabel(new Date(2026, 9, 31, 9, 5, 7).toISOString())).toBe('09:05:07');
  });

  it('LOG-009: rows are grouped under Today, Yesterday, then the date', () => {
    expect(dayLabel(new Date(2026, 10, 4, 0, 1).toISOString(), now)).toBe('Today');
    expect(dayLabel(new Date(2026, 10, 3, 23, 59).toISOString(), now)).toBe('Yesterday');
    expect(dayLabel(new Date(2026, 9, 31, 12).toISOString(), now)).toBe('Sat 31 Oct');
  });

  it('LOG-009: a time that does not parse shows as the hub sent it, under no day', () => {
    expect(clockLabel('not a time')).toBe('not a time');
    expect(dayLabel('not a time', now)).toBe('');
  });
});

describe('labels', () => {
  it('LOG-009: severity shows as a word', () => {
    expect(labelOf(SEVERITY_CHOICES, 'WARNING')).toBe('Warning');
  });

  it('LOG-009: an event with no device shows a dash', () => {
    expect(deviceLabel(null)).toBe('—');
    expect(deviceLabel('NAS')).toBe('NAS');
  });
});

describe('filter changes', () => {
  it('LOG-009: a choice switches on and off', () => {
    const on = switched(SEVERITY_CHOICES, [], 'ERROR');
    expect(on).toEqual(['ERROR']);
    expect(switched(SEVERITY_CHOICES, on, 'ERROR')).toEqual([]);
  });

  it('LOG-009: a value that is no choice changes nothing', () => {
    expect(switched(SEVERITY_CHOICES, ['INFO'], 'LOUD')).toEqual(['INFO']);
    expect(withSince(DEFAULT_LOG_FILTER, '2d')).toBe(DEFAULT_LOG_FILTER);
  });

  it('LOG-009: the time window and the device are set and cleared', () => {
    expect(withSince(DEFAULT_LOG_FILTER, '7d').since).toBe('7d');
    expect(withDevice(DEFAULT_LOG_FILTER, 'd2').deviceId).toBe('d2');
    expect(withDevice({ ...DEFAULT_LOG_FILTER, deviceId: 'd2' }, '').deviceId).toBeNull();
  });
});
