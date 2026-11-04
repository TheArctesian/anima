import { describe, expect, test } from 'bun:test';
import { sampleCurve, settleMs, stepResponse, toLibraryForm } from '../build/spring.ts';

const SPRINGS = {
  default: { dampingRatio: 1, responseS: 0.35 },
  snappy: { dampingRatio: 1, responseS: 0.25 },
  move: { dampingRatio: 1, responseS: 0.4 },
  momentum: { dampingRatio: 0.8, responseS: 0.3 },
};

describe('spring conversion (5.7.2)', () => {
  for (const [name, spring] of Object.entries(SPRINGS)) {
    test(`5.7.2 spring-${name} converts with stiffness = (2π/response)², damping = 4πζ/response`, () => {
      const converted = toLibraryForm(spring);
      expect(converted.stiffness).toBeCloseTo((2 * Math.PI / spring.responseS) ** 2, 9);
      expect(converted.damping).toBeCloseTo((4 * Math.PI * spring.dampingRatio) / spring.responseS, 9);
    });
  }
});

describe('spring step response (D-4)', () => {
  test('MOT-003 every spring reaches 1 and its curve ends exactly at 1', () => {
    for (const spring of Object.values(SPRINGS)) {
      expect(Math.abs(1 - stepResponse(spring, settleMs(spring) / 1000))).toBeLessThan(0.001);
      expect(sampleCurve(spring).at(-1)).toBe(1);
    }
  });

  test('MOT-003 a critically damped spring never overshoots', () => {
    for (const spring of [SPRINGS.default, SPRINGS.snappy, SPRINGS.move]) {
      expect(Math.max(...sampleCurve(spring))).toBeLessThanOrEqual(1);
    }
  });

  test('MOT-003 the momentum spring (damping 0.8) overshoots before it settles', () => {
    expect(Math.max(...sampleCurve(SPRINGS.momentum))).toBeGreaterThan(1);
  });

  test('MOT-003 a slower spring takes longer to settle', () => {
    expect(settleMs(SPRINGS.snappy)).toBeLessThan(settleMs(SPRINGS.default));
    expect(settleMs(SPRINGS.default)).toBeLessThan(settleMs(SPRINGS.move));
  });
});
