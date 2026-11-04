import { describe, expect, it } from 'vitest';
import { dataAge } from './age';

const NOW = new Date('2026-11-04T10:00:00Z');

function secondsBefore(seconds: number): Date {
  return new Date(NOW.getTime() - seconds * 1000);
}

describe('dataAge', () => {
  it.each([
    [0, 'just now'],
    [59, 'just now'],
    [60, '1 minute ago'],
    [119, '1 minute ago'],
    [120, '2 minutes ago'],
    [5 * 60 + 30, '5 minutes ago'],
    [59 * 60 + 59, '59 minutes ago'],
    [60 * 60, '1 hour ago'],
    [2 * 60 * 60 + 59 * 60, '2 hours ago'],
    [23 * 60 * 60 + 59 * 60, '23 hours ago'],
    [24 * 60 * 60, '1 day ago'],
    [3 * 24 * 60 * 60 + 600, '3 days ago'],
    [-30, 'just now'],
  ])('NET-004: data %i seconds old reads "%s"', (seconds, text) => {
    expect(dataAge(secondsBefore(seconds), NOW)).toBe(text);
  });
});
