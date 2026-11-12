import { expect, it } from 'vitest';
import { placeInside } from './placement';

const viewport = { width: 800, height: 600 };
const panel = { width: 288, height: 120 };

it('UI-028: the panel goes under the button, level with its left edge', () => {
  expect(placeInside({ left: 100, top: 100, width: 24, height: 24 }, panel, viewport)).toEqual({ left: 100, top: 124 });
});

it('UI-028: a panel that would pass the right edge moves left until it fits', () => {
  expect(placeInside({ left: 700, top: 100, width: 24, height: 24 }, panel, viewport).left).toBe(512);
});

it('UI-028: with no room below and room above, the panel goes above the button', () => {
  expect(placeInside({ left: 100, top: 560, width: 24, height: 24 }, panel, viewport).top).toBe(440);
});

it('UI-028: a panel wider than the window keeps its left edge on screen', () => {
  expect(placeInside({ left: 100, top: 100, width: 24, height: 24 }, { width: 900, height: 120 }, viewport).left).toBe(0);
});
