import { expect, it } from 'vitest';
import { tabTarget } from './focus';

const STOPS = ['a', 'b', 'c'];

it('Modal: Tab goes to the next stop, also between the first and the last', () => {
  expect(tabTarget(STOPS, 'a', false)).toBe('b');
  expect(tabTarget(STOPS, 'b', false)).toBe('c');
});

it('Modal: Shift+Tab goes to the previous stop', () => {
  expect(tabTarget(STOPS, 'c', true)).toBe('b');
  expect(tabTarget(STOPS, 'b', true)).toBe('a');
});

it('Modal: Tab from the last stop goes to the first, and Shift+Tab from the first to the last', () => {
  expect(tabTarget(STOPS, 'c', false)).toBe('a');
  expect(tabTarget(STOPS, 'a', true)).toBe('c');
});

it('Modal: focus outside the stops is pulled back in at the near end', () => {
  expect(tabTarget(STOPS, 'outside', false)).toBe('a');
  expect(tabTarget(STOPS, 'outside', true)).toBe('c');
});

it('Modal: a dialog with no stops has no Tab target', () => {
  expect(tabTarget([], null, false)).toBeNull();
});
