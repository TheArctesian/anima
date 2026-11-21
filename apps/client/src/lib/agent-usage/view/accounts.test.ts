import { describe, expect, it } from 'vitest';
import { sums } from '../agent-usage.fixture';
import { deviceShares, limitFill, limitLevel, limitText, resetWords, windowWords } from './accounts';

const NOW = Date.parse('2026-11-17T10:00:00Z');

describe('windowWords', () => {
  it.each([
    ['five_hour', '5 hours'],
    ['seven_day', '7 days'],
    ['weekly', '7 days'],
    ['300 min', '5 hours'],
    ['90 min', '90 minutes'],
    ['1_hour', '1 hour'],
    ['opus_week', 'Opus week'],
  ])('%s is %s', (window, words) => {
    expect(windowWords(window)).toBe(words);
  });
});

it('a window is high from 80 % used, and a reached one is full whatever its percent', () => {
  expect(limitLevel({ usedPercent: 79, reached: false })).toBe('normal');
  expect(limitLevel({ usedPercent: 80, reached: false })).toBe('high');
  expect(limitLevel({ usedPercent: null, reached: true })).toBe('reached');
  expect(limitFill({ usedPercent: 40, reached: true })).toBe(100);
  expect(limitFill({ usedPercent: null, reached: false })).toBe(0);
});

it('the time a window starts again is the clock today, the weekday this week, else the day', () => {
  expect(resetWords('2026-11-17T14:00:00Z', NOW, 'UTC')).toBe('14:00');
  expect(resetWords('2026-11-19T09:00:00Z', NOW, 'UTC')).toBe('Thu 09:00');
  expect(resetWords('2026-11-28T09:00:00Z', NOW, 'UTC')).toBe('28 Nov');
});

it('the line of a window says what is used, or that the limit is reached, and when it starts again if the program said', () => {
  expect(limitText({ window: 'five_hour', usedPercent: 35, resetsAt: '2026-11-17T14:00:00Z', reached: false }, NOW, 'UTC')).toBe('35 % used · starts again 14:00');
  expect(limitText({ window: 'five_hour', usedPercent: 100, resetsAt: null, reached: true }, NOW, 'UTC')).toBe('Limit reached');
  expect(limitText({ window: 'five_hour', usedPercent: null, resetsAt: null, reached: false }, NOW, 'UTC')).toBe('Not reached');
});

it('each device of an account has its share of the use of that program only, and none when no device has use', () => {
  const devices = [{ id: 'a', name: 'mac' }, { id: 'b', name: 'box' }];
  const rows = [
    { ...sums({ estimatedCostUsd: 3 }), device: devices[0] ?? { id: '', name: '' }, source: 'CODEX' as const },
    { ...sums({ estimatedCostUsd: 1 }), device: devices[1] ?? { id: '', name: '' }, source: 'CODEX' as const },
    { ...sums({ estimatedCostUsd: 50 }), device: devices[1] ?? { id: '', name: '' }, source: 'CLAUDE_CODE' as const },
  ];
  expect(deviceShares({ source: 'CODEX', devices }, rows, 'cost').map((one) => one.percent)).toEqual([75, 25]);
  expect(deviceShares({ source: 'CODEX', devices }, [], 'cost').map((one) => one.percent)).toEqual([null, null]);
});
