import { describe, expect, it } from 'vitest';
import { nearestSnap, project, rubberband } from './gesture';

// Golden values worked by hand from the spec formulas with d = 0.998 and c = 0.55:
// d ÷ (1 − d) = 0.998 ÷ 0.002 = 499.
describe('project', () => {
  it.each([
    [100, 2000, 100 + 2 * 499],
    [0, -500, -249.5],
    [40, 0, 40],
  ])('MOT-032: position %d with velocity %d px/s projects to %d', (position, velocity, expected) => {
    expect(project(position, velocity)).toBeCloseTo(expected, 6);
  });
});

describe('rubberband', () => {
  it.each([
    // 100 × 300 × 0.55 ÷ (300 + 55) = 16500 ÷ 355
    [100, 300, 46.478873],
    [-100, 300, -46.478873],
    // 1000 × 200 × 0.55 ÷ (200 + 550) = 110000 ÷ 750
    [1000, 200, 146.666667],
    [0, 300, 0],
  ])('MOT-033: overshoot %d on size %d shows %d', (overshoot, size, expected) => {
    expect(rubberband(overshoot, size)).toBeCloseTo(expected, 5);
  });

  it('MOT-033: resistance never lets the shown distance reach the overshoot', () => {
    for (const overshoot of [1, 10, 100, 1000]) {
      expect(rubberband(overshoot, 300)).toBeLessThan(overshoot);
    }
  });
});

describe('nearestSnap', () => {
  const snaps = [0, 330, 660] as const;

  it('MOT-032: a flick goes to the snap point nearest the projected position', () => {
    // 100 + 1 × 499 = 599, nearest 660; the release position alone would pick 0.
    expect(nearestSnap(100, 1000, snaps)).toBe(660);
  });

  it('MOT-032: without velocity the nearest snap point to the release wins', () => {
    expect(nearestSnap(100, 0, snaps)).toBe(0);
    expect(nearestSnap(500, 0, snaps)).toBe(660);
  });

  it('MOT-034: velocity against the position decides the direction', () => {
    expect(nearestSnap(600, -800, snaps)).toBe(330);
  });
});
