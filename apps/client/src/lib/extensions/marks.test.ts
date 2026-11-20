import { expect, it } from 'vitest';
import { markOf, storedView, storeView, VIEW_KEY } from './marks';

it('EXT-001: each extension has its mark, and one that the client does not know has the mark of the page', () => {
  expect(markOf('projects')).toBe('carbon:folder');
  expect(markOf('not_known_yet')).toBe('carbon:grid');
});

it('EXT-001: the view of the Extensions page is kept on the device, and is the list when nothing is kept', () => {
  const kept = new Map<string, string>();
  const storage = { getItem: (key: string) => kept.get(key) ?? null, setItem: (key: string, value: string) => void kept.set(key, value) };
  expect(storedView(storage)).toBe('list');
  storeView(storage, 'grid');
  expect(kept.get(VIEW_KEY)).toBe('grid');
  expect(storedView(storage)).toBe('grid');
  expect(storedView(undefined)).toBe('list');
});

it('EXT-001: a store that refuses loses only the memory of the choice', () => {
  const refuses = {
    getItem: (): string => {
      throw new Error('refused');
    },
    setItem: (): void => {
      throw new Error('refused');
    },
  };
  expect(storedView(refuses)).toBe('list');
  expect(() => { storeView(refuses, 'grid'); }).not.toThrow();
});
