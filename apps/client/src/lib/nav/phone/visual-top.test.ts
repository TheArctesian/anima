// @vitest-environment jsdom
import { expect, it } from 'vitest';
import { followVisualTop } from './visual-top';

it('MOB-011: the root follows how far the visible area has moved, and stops when asked', () => {
  const area = Object.assign(new EventTarget(), { offsetTop: 0 });
  const root = document.createElement('div');
  const stop = followVisualTop(root, area);
  expect(root.style.getPropertyValue('--visual-block-start')).toBe('0px');

  area.offsetTop = 72;
  area.dispatchEvent(new Event('resize'));
  expect(root.style.getPropertyValue('--visual-block-start')).toBe('72px');

  stop();
  area.offsetTop = 10;
  area.dispatchEvent(new Event('scroll'));
  expect(root.style.getPropertyValue('--visual-block-start')).toBe('72px');
});
