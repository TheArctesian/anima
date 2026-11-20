import { describe, expect, it } from 'vitest';
import { heldOrder } from './held';
import type { ListEntry } from './look';

type Entry = ListEntry<string>;
const row = (key: string, device = key): Entry => ({ kind: 'device', key, device });
const title = (key: string, count: number): Entry => ({ kind: 'group', key, title: key, count });
const keys = (entries: readonly Entry[] | null) => entries?.map((entry) => entry.key) ?? null;

describe('heldOrder', () => {
  it('5.7.3 Reorder: keeps each row in its place on screen, with its new data', () => {
    const next = [row('b', 'B down'), row('a'), row('c')];
    const held = heldOrder([row('a'), row('b'), row('c')], next);
    expect(keys(held)).toEqual(['a', 'b', 'c']);
    expect(held?.[1]).toBe(next[0]);
  });

  it('5.7.3 Reorder: a new group title waits, and an emptied one keeps its place until the hold ends', () => {
    const shown = [title('up', 3), row('a'), row('b'), row('c')];
    expect(keys(heldOrder(shown, [title('look', 1), row('b'), title('up', 2), row('a'), row('c')]))).toEqual(['up', 'a', 'b', 'c']);
    const emptied = [title('look', 1), row('b'), title('up', 2), row('a')];
    expect(keys(heldOrder(emptied, [title('up', 2), row('a'), row('b')]))).toEqual(['look', 'b', 'up', 'a']);
  });

  it('5.7.3 Reorder: gives up when a device came or went, so the new list shows at once', () => {
    expect(heldOrder([row('a'), row('b')], [row('a'), row('b'), row('c')])).toBeNull();
    expect(heldOrder([row('a'), row('b')], [row('a'), row('c')])).toBeNull();
  });
});
