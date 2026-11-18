// @vitest-environment jsdom
import { expect, it } from 'vitest';
import { followVisualTop } from './visual-top';

it('MOB-011: the root follows where the visible area is and how tall it is, and stops when asked', () => {
  const area = Object.assign(new EventTarget(), { offsetTop: 0, height: 874 });
  const root = document.createElement('div');
  const stop = followVisualTop(root, area);
  expect(root.style.getPropertyValue('--visual-block-start')).toBe('0px');

  area.offsetTop = 72;
  area.height = 520;
  area.dispatchEvent(new Event('resize'));
  expect(root.style.getPropertyValue('--visual-block-start')).toBe('72px');
  expect(root.style.getPropertyValue('--visual-block-size')).toBe('520px');

  stop();
  area.offsetTop = 10;
  area.dispatchEvent(new Event('scroll'));
  expect(root.style.getPropertyValue('--visual-block-start')).toBe('72px');
});
