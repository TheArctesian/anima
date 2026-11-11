import { expect, it } from 'vitest';
import { draggedWidth, restsCollapsed } from './resize';

const widths = { icons: 84, full: 208 };

it('5.7.4: between the two widths the edge follows the pointer 1:1', () => {
  expect(draggedWidth(150, widths)).toBe(150);
  expect(draggedWidth(84, widths)).toBe(84);
  expect(draggedWidth(208, widths)).toBe(208);
});

it('MOT-033: past either width the edge resists and never reaches the pointer', () => {
  const under = draggedWidth(84 - 60, widths);
  expect(under).toBeLessThan(84);
  expect(under).toBeGreaterThan(84 - 60);
  const over = draggedWidth(208 + 60, widths);
  expect(over).toBeGreaterThan(208);
  expect(over).toBeLessThan(208 + 60);
});

it('MOT-032: a slow release rests at the nearer width', () => {
  expect(restsCollapsed(120, 0, widths)).toBe(true);
  expect(restsCollapsed(170, 0, widths)).toBe(false);
});

it('MOT-034: a flick carries the bar past the middle in the direction it was thrown', () => {
  expect(restsCollapsed(180, -400, widths)).toBe(true);
  expect(restsCollapsed(110, 400, widths)).toBe(false);
});
