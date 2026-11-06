import { expect, it } from 'vitest';
import { wrapTarget } from './focus';

const ELEMENTS = ['a', 'b', 'c'];

it('Modal: Tab from the last element goes to the first', () => {
  expect(wrapTarget(ELEMENTS, 'c', false)).toBe('a');
});

it('Modal: Shift+Tab from the first element goes to the last', () => {
  expect(wrapTarget(ELEMENTS, 'a', true)).toBe('c');
});

it('Modal: inside the dialog the browser steps by itself', () => {
  expect(wrapTarget(ELEMENTS, 'b', false)).toBeNull();
  expect(wrapTarget(ELEMENTS, 'b', true)).toBeNull();
});

it('Modal: focus outside the dialog is pulled back in', () => {
  expect(wrapTarget(ELEMENTS, 'outside', false)).toBe('a');
  expect(wrapTarget(ELEMENTS, 'outside', true)).toBe('c');
});

it('Modal: a dialog with nothing to focus leaves Tab alone', () => {
  expect(wrapTarget([], null, false)).toBeNull();
});
