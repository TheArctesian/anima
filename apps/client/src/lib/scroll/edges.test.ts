// @vitest-environment jsdom
import { afterEach, expect, it } from 'vitest';
import { edgeFade, hiddenEdges } from './edges';

afterEach(() => {
  document.body.replaceChildren();
});

it('NAV-005: a strip shows a fade at each end that has content past it', () => {
  expect(hiddenEdges(0, 200, 400)).toEqual({ start: false, end: true });
  expect(hiddenEdges(100, 200, 400)).toEqual({ start: true, end: true });
  expect(hiddenEdges(200, 200, 400)).toEqual({ start: true, end: false });
  expect(hiddenEdges(0, 400, 400)).toEqual({ start: false, end: false });
});

it('MAT-003: a scrolling body is marked at the edge where its content scrolls, and unmarked when scrolled back', () => {
  const body = document.createElement('div');
  Object.defineProperty(body, 'clientHeight', { value: 100 });
  Object.defineProperty(body, 'scrollHeight', { value: 300 });
  document.body.append(body);
  const stop = edgeFade(body);
  expect(body.hasAttribute('data-more-below')).toBe(true);
  expect(body.hasAttribute('data-more-above')).toBe(false);
  body.scrollTop = 100;
  body.dispatchEvent(new Event('scroll'));
  expect(body.hasAttribute('data-more-above')).toBe(true);
  body.scrollTop = 200;
  body.dispatchEvent(new Event('scroll'));
  expect(body.hasAttribute('data-more-below')).toBe(false);
  stop();
});
