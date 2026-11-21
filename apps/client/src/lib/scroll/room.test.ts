import { expect, it } from 'vitest';
import { roomBelow } from './room';

it('THR-013: a page that scrolls in parts takes the height from its top to the foot of the window', () => {
  expect(roomBelow(120, 900, 24, 320)).toBe(756);
  expect(roomBelow(700, 900, 24, 320), 'a short window keeps a least height').toBe(320);
});
