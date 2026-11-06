// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { applyTypefaces, DEFAULT_TYPEFACE, storedTypefaces, storeTypeface } from './typeface';

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

const refusing: Storage = {
  ...memoryStorage(),
  getItem: () => {
    throw new Error('refused');
  },
};

describe('storedTypefaces (UI-020)', () => {
  it('is Fira Code for every role when nothing is stored', () => {
    expect(DEFAULT_TYPEFACE).toBe('fira-code');
    expect(storedTypefaces(memoryStorage())).toEqual({ ui: 'fira-code', mono: 'fira-code' });
  });

  it('gives back each role’s stored choice on its own', () => {
    const storage = memoryStorage();
    storeTypeface(storage, 'mono', 'opendyslexic');
    expect(storedTypefaces(storage)).toEqual({ ui: 'fira-code', mono: 'opendyslexic' });
    storeTypeface(storage, 'ui', 'opendyslexic');
    expect(storedTypefaces(storage)).toEqual({ ui: 'opendyslexic', mono: 'opendyslexic' });
  });

  it('falls back to the default for unknown text and for a storage that refuses to be read', () => {
    expect(storedTypefaces(memoryStorage({ 'anima.typeface.ui': 'comic-sans' }))).toEqual({ ui: 'fira-code', mono: 'fira-code' });
    expect(storedTypefaces(refusing)).toEqual({ ui: 'fira-code', mono: 'fira-code' });
  });
});

describe('applyTypefaces (UI-020)', () => {
  it('marks the root with each role’s typeface', () => {
    const root = document.createElement('html');
    applyTypefaces(root, { ui: 'opendyslexic', mono: 'fira-code' });
    expect(root.dataset).toMatchObject({ fontUi: 'opendyslexic', fontMono: 'fira-code' });
  });
});
