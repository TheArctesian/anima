// The two duration grammars Headscale v0.29.7 uses (WP-03 T2; VPN-003).
import { describe, expect, test } from 'bun:test';
import { parseGoDuration, seconds } from '../src/go-duration.ts';
import { days, parsePrometheusDuration } from '../src/prometheus-duration.ts';

describe('node.expiry grammar, Prometheus ParseDuration (VPN-003)', () => {
  test('accepts unit sequences in Headscale order', () => {
    expect(parsePrometheusDuration('180d')).toBe(days(180));
    expect(parsePrometheusDuration('1w2d')).toBe(days(9));
    expect(parsePrometheusDuration('1h30m')).toBe(90 * 60 * 1000);
    expect(parsePrometheusDuration('5ms')).toBe(5);
    expect(parsePrometheusDuration('0')).toBe(0);
    expect(parsePrometheusDuration('292y')).toBe(days(292 * 365));
  });

  test('rejects what Headscale rejects (and would then never expire)', () => {
    for (const text of ['180 days', '180', '6mo', '', '1.5d', '2d1w', '-1d', '180D', '300y']) {
      expect(parsePrometheusDuration(text)).toBeNull();
    }
  });
});

describe('Go time.ParseDuration grammar (ephemeral timeout, HA probes)', () => {
  test('accepts Go durations, decimals and signs', () => {
    expect(parseGoDuration('30m')).toBe(seconds(1800));
    expect(parseGoDuration('1h30m')).toBe(seconds(5400));
    expect(parseGoDuration('1.5s')).toBe(seconds(1.5));
    expect(parseGoDuration('.5s')).toBe(seconds(0.5));
    expect(parseGoDuration('10ms')).toBe(10_000_000);
    expect(parseGoDuration('3us')).toBe(3000);
    expect(parseGoDuration('7ns')).toBe(7);
    expect(parseGoDuration('-2s')).toBe(-seconds(2));
    expect(parseGoDuration('0')).toBe(0);
  });

  test('rejects what Go rejects, and unit-less numbers', () => {
    for (const text of ['', '10', '1d', '.s', '10 s', 's', '1h-30m', '180d']) {
      expect(parseGoDuration(text)).toBeNull();
    }
  });
});
