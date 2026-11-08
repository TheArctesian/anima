// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { applyTheme, DEFAULT_THEME, storedTheme, storeTheme, THEMES } from './theme';

function memoryStorage(entries: Record<string, string> = {}): Storage {
  const values = new Map(Object.entries(entries));
  return {
    get length() {
      return values.size;
    },
    clear: () => {
      values.clear();
    },
    getItem: (key) => values.get(key) ?? null,
    key: (index) => [...values.keys()][index] ?? null,
    removeItem: (key) => {
      values.delete(key);
    },
    setItem: (key, value) => {
      values.set(key, value);
    },
  };
}

describe('storedTheme (UI-005)', () => {
  it('UI-005: is dark when nothing is stored', () => {
    expect(DEFAULT_THEME).toBe('dark');
    expect(storedTheme(memoryStorage())).toBe('dark');
  });

  it('UI-005: gives back each stored choice, saved under anima.theme', () => {
    for (const { id } of THEMES) {
      const storage = memoryStorage();
      storeTheme(storage, id);
      expect(storage.getItem('anima.theme')).toBe(id);
      expect(storedTheme(storage)).toBe(id);
    }
  });

  it('UI-005: an unknown stored value gives dark, and is never passed on', () => {
    for (const text of ['sepia', 'LIGHT', '', 'light"] body{', ' light']) {
      expect(storedTheme(memoryStorage({ 'anima.theme': text }))).toBe('dark');
    }
  });

  it('UI-005: a storage that refuses to be read gives dark', () => {
    const refusing: Storage = {
      ...memoryStorage(),
      getItem: () => {
        throw new Error('refused');
      },
    };
    expect(storedTheme(refusing)).toBe('dark');
  });
});

describe('applyTheme (UI-005)', () => {
  it('UI-005: marks the root with the theme, for the CSS to follow', () => {
    const root = document.createElement('html');
    for (const { id } of THEMES) {
      applyTheme(root, id);
      expect(root.getAttribute('data-theme')).toBe(id);
    }
  });
});
