import { expect, it } from 'vitest';
import { restoredScroll } from './covered';

it('E-2: the page a session covered gets back its scroll position when it shows again', () => {
  expect(restoredScroll({ path: '/devices', y: 1518 }, '/devices')).toBe(1518);
});

it('E-2: a different page keeps the position navigation gives it', () => {
  expect(restoredScroll({ path: '/devices', y: 1518 }, '/settings')).toBeUndefined();
});

it('E-2: nothing is restored when nothing was covered', () => {
  expect(restoredScroll(null, '/devices')).toBeUndefined();
});
