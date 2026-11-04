import { describe, expect, it } from 'vitest';
import { COLLAPSED, THIRD, TWO_THIRDS, draggedHeight, keyStop, releasedStop, snapsOf } from './resize';

// A 900 px page with a 40 px collapsed panel: snaps at 40, 297 and 594.
const snaps = snapsOf(40, 900);

describe('snapsOf', () => {
  it('MOT-032: the stops are collapsed, 33 % and 66 % of the page', () => {
    expect(snaps).toEqual([40, 297, 594]);
  });

  it('MOT-032: a page shorter than the header never puts a stop below the collapsed height', () => {
    expect(snapsOf(40, 50)).toEqual([40, 40, 40]);
  });
});

describe('draggedHeight', () => {
  it('MOT-030: between the limits the height follows the pointer 1:1', () => {
    expect(draggedHeight(333, snaps)).toBe(333);
  });

  // 100 × 554 × 0.55 ÷ (554 + 0.55 × 100) = 30470 ÷ 609
  it('MOT-033: past 66 % the panel resists with the rubber-band curve', () => {
    expect(draggedHeight(694, snaps)).toBeCloseTo(594 + 50.0328, 3);
  });

  it('MOT-033: below collapsed the panel resists too, and does not stop hard', () => {
    expect(draggedHeight(-60, snaps)).toBeCloseTo(40 - 50.0328, 3);
    expect(draggedHeight(-60, snaps)).toBeLessThan(40);
  });

  it('MOT-033: resistance grows with the distance but stays below it', () => {
    const far = draggedHeight(594 + 400, snaps) - 594;
    expect(far).toBeGreaterThan(draggedHeight(694, snaps) - 594);
    expect(far).toBeLessThan(400);
  });
});

describe('releasedStop', () => {
  it('MOT-032: a slow release goes to the nearest stop', () => {
    expect(releasedStop(310, 0, snaps)).toBe(THIRD);
    expect(releasedStop(560, 0, snaps)).toBe(TWO_THIRDS);
  });

  it('MOT-032: a fast downward flick from near 33 % goes to collapsed, not to the stop nearest the release', () => {
    expect(releasedStop(307, 0, snaps)).toBe(THIRD);
    expect(releasedStop(307, -1500, snaps)).toBe(COLLAPSED);
  });

  it('MOT-034: an upward flick opens the panel although the release is nearer collapsed', () => {
    expect(releasedStop(100, 0, snaps)).toBe(COLLAPSED);
    expect(releasedStop(100, 600, snaps)).toBe(THIRD);
  });

  it('MOT-034: the same release closes or opens by the direction of the velocity alone', () => {
    expect(releasedStop(297, 800, snaps)).toBe(TWO_THIRDS);
    expect(releasedStop(297, -800, snaps)).toBe(COLLAPSED);
  });
});

describe('keyStop', () => {
  it.each([
    ['ArrowUp', COLLAPSED, THIRD],
    ['ArrowUp', THIRD, TWO_THIRDS],
    ['ArrowUp', TWO_THIRDS, TWO_THIRDS],
    ['ArrowDown', TWO_THIRDS, THIRD],
    ['ArrowDown', THIRD, COLLAPSED],
    ['ArrowDown', COLLAPSED, COLLAPSED],
    ['Home', TWO_THIRDS, COLLAPSED],
    ['End', COLLAPSED, TWO_THIRDS],
  ] as const)('MOT-030: %s from stop %d goes to stop %d', (key, from, to) => {
    expect(keyStop(key, from)).toBe(to);
  });

  it('MOT-030: other keys do nothing', () => {
    expect(keyStop('a', THIRD)).toBeNull();
  });
});
