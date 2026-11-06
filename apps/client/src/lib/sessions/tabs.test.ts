import { expect, it } from 'vitest';
import { closed, deselected, moved, noTabs, opened, selected, titled, withSession, type Tab, type Tabs } from './tabs';

function tab(id: string, sessionId: string | null = null): Tab {
  return { id, deviceId: `device-${id}`, deviceName: id, protocol: 'SSH', origin: null, sessionId };
}

function strip(ids: string[], activeId: string | null): Tabs {
  return { tabs: ids.map((id) => tab(id)), activeId };
}

const ids = (state: Tabs) => state.tabs.map((open) => open.id);

it('RA-001: an opened tab goes last and is selected, even for a device that already has one', () => {
  const once = opened(noTabs, tab('a'));
  const twice = opened(once, { ...tab('b'), deviceId: 'device-a' });
  expect(ids(twice)).toEqual(['a', 'b']);
  expect(twice.activeId).toBe('b');
});

it.each([
  { name: 'a selected middle tab passes to its right neighbour', before: ['a', 'b', 'c'], active: 'b', close: 'b', after: ['a', 'c'], next: 'c' },
  { name: 'the selected last tab passes to its left neighbour', before: ['a', 'b', 'c'], active: 'c', close: 'c', after: ['a', 'b'], next: 'b' },
  { name: 'a tab that is not selected leaves the selection alone', before: ['a', 'b', 'c'], active: 'a', close: 'c', after: ['a', 'b'], next: 'a' },
  { name: 'the only tab leaves the pages showing', before: ['a'], active: 'a', close: 'a', after: [], next: null },
  { name: 'closing while the pages show keeps them showing', before: ['a', 'b'], active: null, close: 'a', after: ['b'], next: null },
  { name: 'an unknown tab changes nothing', before: ['a'], active: 'a', close: 'z', after: ['a'], next: 'a' },
])('close: $name', ({ before, active, close, after, next }) => {
  const state = closed(strip(before, active), close);
  expect(ids(state)).toEqual(after);
  expect(state.activeId).toBe(next);
});

it.each([
  ['forward', 0, 2, ['b', 'c', 'a', 'd']],
  ['backward', 3, 1, ['a', 'd', 'b', 'c']],
  ['to the same place', 1, 1, ['a', 'b', 'c', 'd']],
  ['past the end, clamped', 0, 9, ['b', 'c', 'd', 'a']],
  ['before the start, clamped', 2, -3, ['c', 'a', 'b', 'd']],
] as const)('move %s', (_name, from, to, after) => {
  const next = moved(strip(['a', 'b', 'c', 'd'], 'b'), from, to);
  expect(ids(next)).toEqual(after);
  expect(next.activeId).toBe('b');
});

it('moving a tab in an empty strip changes nothing', () => {
  expect(moved(noTabs, 0, 1)).toBe(noTabs);
});

it('selecting picks a known tab only; deselecting shows the pages and keeps every tab', () => {
  const state = strip(['a', 'b'], 'a');
  expect(selected(state, 'b').activeId).toBe('b');
  expect(selected(state, 'z')).toBe(state);
  expect(deselected(state)).toEqual({ tabs: state.tabs, activeId: null });
});

it('a started session is recorded on its own tab only', () => {
  const next = withSession(strip(['a', 'b'], 'a'), 'b', 'session-1');
  expect(next.tabs.map((open) => open.sessionId)).toEqual([null, 'session-1']);
});

it('K4 §8.7: a title is set on its own tab only, and an empty one clears it', () => {
  const state = strip(['a', 'b'], 'a');
  const named = titled(state, 'b', 'user@host: ~');
  expect(named.tabs.map((open) => open.title)).toEqual([undefined, 'user@host: ~']);
  expect(titled(named, 'b', '').tabs[1]?.title).toBe('');
  expect(titled(state, 'missing', 'x')).toEqual(state);
});
