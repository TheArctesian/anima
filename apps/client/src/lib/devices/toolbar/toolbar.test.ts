import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { DEFAULT_QUERY } from '../query';
import { NAME_DEBOUNCE_MS, debounced, isFiltered, menuLabel, shiftInsideViewport, toggled, withoutFilters } from './toolbar';

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

it('DEV-003: toggled adds a value, and removes it when it is there', () => {
  expect(toggled(['UP'], 'DOWN')).toEqual(['UP', 'DOWN']);
  expect(toggled(['UP', 'DOWN'], 'UP')).toEqual(['DOWN']);
});

it('DEV-005: sort and view are not filters; name, status and tags are', () => {
  expect(isFiltered({ ...DEFAULT_QUERY, sort: 'STATUS', view: 'table' })).toBe(false);
  expect(isFiltered({ ...DEFAULT_QUERY, name: 'x' })).toBe(true);
  expect(isFiltered({ ...DEFAULT_QUERY, statuses: ['UP'] })).toBe(true);
  expect(isFiltered({ ...DEFAULT_QUERY, tags: ['lab'] })).toBe(true);
});

it('DEV-005: Clear filters keeps the sort and the view', () => {
  const query = { view: 'table', name: 'x', statuses: ['UP'], tags: ['lab'], sort: 'UPTIME' } as const;
  expect(withoutFilters(query)).toEqual({ view: 'table', name: '', statuses: [], tags: [], sort: 'UPTIME' });
});

it('DEV-003: a menu label counts the chosen values', () => {
  expect(menuLabel('Status', 0)).toBe('Status');
  expect(menuLabel('Status', 2)).toBe('Status (2)');
});

it('D-4: debounced runs once, 200 ms after the last call', () => {
  const run = vi.fn();
  const call = debounced(run, NAME_DEBOUNCE_MS);
  call();
  vi.advanceTimersByTime(NAME_DEBOUNCE_MS - 1);
  call();
  vi.advanceTimersByTime(NAME_DEBOUNCE_MS - 1);
  expect(run).not.toHaveBeenCalled();
  vi.advanceTimersByTime(1);
  expect(run).toHaveBeenCalledTimes(1);
});

it('D-4: a cancelled debounce never runs', () => {
  const run = vi.fn();
  const call = debounced(run, NAME_DEBOUNCE_MS);
  call();
  call.cancel();
  vi.advanceTimersByTime(NAME_DEBOUNCE_MS * 2);
  expect(run).not.toHaveBeenCalled();
});

it('UI-013: a menu panel that runs past the right edge moves left to end inside the window', () => {
  expect(shiftInsideViewport(300, 192, 390, 8)).toBe(-110);
  expect(shiftInsideViewport(100, 192, 390, 8)).toBe(0);
});

it('UI-013: a menu panel wider than the window keeps its left edge in view', () => {
  expect(shiftInsideViewport(40, 500, 390, 8)).toBe(-32);
});
