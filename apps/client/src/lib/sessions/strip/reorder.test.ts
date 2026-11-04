import { describe, expect, it } from 'vitest';
import { project } from '$lib/motion/gesture';
import { VELOCITY_WINDOW_MS, isOutside, shifts, slotIndex, velocityOf } from './reorder';
import { nextFocus } from './tablist';

// Four 100 px tabs with a 10 px gap: centres at 50, 160, 270, 380.
const slots = [0, 110, 220, 330].map((left) => ({ left, width: 100 }));

describe('slotIndex', () => {
  it.each([
    ['over its own place', 50, 0, 0],
    ['past the middle of the next tab', 170, 0, 1],
    ['before the strip', -500, 0, 0],
    ['after the strip', 9000, 0, 3],
  ] as const)('with no velocity, a tab %s goes to the place it is over', (_name, centre, velocity, index) => {
    expect(slotIndex(slots, centre, velocity)).toBe(index);
  });

  it('MOT-032: the place comes from the projected position, not the release position', () => {
    // Released over place 0 (centre 60), moving right at 120 px/s: projected 60 + 0.12 × 499 ≈ 120 → place 1.
    expect(slotIndex(slots, 60, 0)).toBe(0);
    expect(project(60, 120)).toBeGreaterThan(105);
    expect(slotIndex(slots, 60, 120)).toBe(1);
    expect(slotIndex(slots, 270, -300)).toBe(1);
  });

  it('an empty strip gives place 0', () => {
    expect(slotIndex([], 10, 0)).toBe(0);
  });
});

describe('shifts', () => {
  it.each([
    ['nothing moves while the tab is over its own place', 1, 1, [0, 0, 0, 0]],
    ['dragged right, the passed tabs move left by its width and a gap', 0, 2, [0, -110, -110, 0]],
    ['dragged left, the passed tabs move right', 3, 1, [0, 110, 110, 0]],
  ] as const)('%s', (_name, from, over, expected) => {
    expect(shifts(slots, from, over)).toEqual(expected);
  });

  it('an unknown tab moves nothing', () => {
    expect(shifts(slots, 7, 0)).toEqual([0, 0, 0, 0]);
  });
});

describe('velocityOf', () => {
  it('MOT-031: px/s over the recent samples', () => {
    const samples = [
      { x: 0, y: 0, t: 0 },
      { x: 10, y: 5, t: 50 },
      { x: 30, y: 10, t: 100 },
    ];
    expect(velocityOf(samples)).toEqual({ x: 300, y: 100 });
  });

  it('samples older than the window do not count', () => {
    const samples = [
      { x: -1000, y: 0, t: 0 },
      { x: 0, y: 0, t: 1000 },
      { x: 20, y: 0, t: 1000 + VELOCITY_WINDOW_MS },
    ];
    expect(velocityOf(samples).x).toBe(200);
  });

  it.each([
    ['no samples', []],
    ['one sample', [{ x: 5, y: 5, t: 1 }]],
    ['a pointer that stopped', [{ x: 5, y: 5, t: 1 }, { x: 50, y: 5, t: 1 + VELOCITY_WINDOW_MS * 3 }]],
  ])('%s gives no velocity', (_name, samples) => {
    expect(velocityOf(samples)).toEqual({ x: 0, y: 0 });
  });
});

describe('isOutside', () => {
  const bounds = { left: 0, top: 0, right: 400, bottom: 40 };
  it.each([
    [200, 20, false],
    [200, 45, false],
    [200, 51, true],
    [-11, 20, true],
    [411, 20, true],
    [200, -11, true],
  ])('(%d, %d) with a 10 px margin is outside: %s', (x, y, outside) => {
    expect(isOutside(x, y, bounds, 10)).toBe(outside);
  });
});

describe('nextFocus', () => {
  it.each([
    ['ArrowRight', 0, 1],
    ['ArrowRight', 2, 0],
    ['ArrowLeft', 0, 2],
    ['Home', 2, 0],
    ['End', 0, 2],
    ['Enter', 1, null],
  ] as const)('%s from %d goes to %s', (key, current, next) => {
    expect(nextFocus(key, current, 3)).toBe(next);
  });

  it('no tabs, no focus move', () => {
    expect(nextFocus('ArrowRight', 0, 0)).toBeNull();
  });
});
