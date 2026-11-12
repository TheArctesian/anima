import { describe, expect, it } from 'vitest';
import { clearTime, clockText, secondsLeft } from './countdown';

const AT = '2026-11-09T10:15:00Z';

describe('countdown (MET-012)', () => {
  it('counts whole seconds up to the expiry and never below zero', () => {
    expect(secondsLeft(AT, new Date('2026-11-09T10:00:00Z'))).toBe(900);
    expect(secondsLeft(AT, new Date('2026-11-09T10:14:59.400Z'))).toBe(1);
    expect(secondsLeft(AT, new Date('2026-11-09T10:15:00Z'))).toBe(0);
    expect(secondsLeft(AT, new Date('2026-11-09T11:00:00Z'))).toBe(0);
  });

  it('counts a time it cannot read as expired', () => {
    expect(secondsLeft('soon', new Date())).toBe(0);
  });

  it('shows minutes and padded seconds', () => {
    expect(clockText(845)).toBe('14:05');
    expect(clockText(59)).toBe('0:59');
  });
});

describe('clearTime (MET-012)', () => {
  const SHOWN = new Date('2026-11-09T10:00:00Z');

  it('is the hub expiry when it comes first, and 15 minutes after showing when it comes later', () => {
    expect(clearTime('2026-11-09T10:05:00Z', SHOWN)).toBe('2026-11-09T10:05:00.000Z');
    expect(clearTime('2026-11-10T10:00:00Z', SHOWN)).toBe('2026-11-09T10:15:00.000Z');
  });

  it('is the moment of showing for a time it cannot read', () => {
    expect(clearTime('soon', SHOWN)).toBe('2026-11-09T10:00:00.000Z');
  });
});
