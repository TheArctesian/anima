// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { applyTypefaces, storedTypefaces, storeTypeface } from './typeface';

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
  const defaults = { ui: 'ibm-plex-sans', mono: 'fira-code' };

  it('is IBM Plex Sans for the interface and Fira Code for monospace when nothing is stored', () => {
    expect(storedTypefaces(memoryStorage())).toEqual(defaults);
  });

  it('gives back each role’s stored choice on its own', () => {
    const storage = memoryStorage();
    storeTypeface(storage, 'mono', 'opendyslexic');
    expect(storedTypefaces(storage)).toEqual({ ui: 'ibm-plex-sans', mono: 'opendyslexic' });
    storeTypeface(storage, 'ui', 'opendyslexic');
    expect(storedTypefaces(storage)).toEqual({ ui: 'opendyslexic', mono: 'opendyslexic' });
  });

  it('falls back to the default for unknown text and for a storage that refuses to be read', () => {
    expect(storedTypefaces(memoryStorage({ 'anima.typeface.ui': 'comic-sans' }))).toEqual(defaults);
    expect(storedTypefaces(refusing)).toEqual(defaults);
  });

  it('migrates a stored interface choice of fira-code, or atkinson-hyperlegible-next, to the new default', () => {
    expect(storedTypefaces(memoryStorage({ 'anima.typeface.ui': 'fira-code' })).ui).toBe('ibm-plex-sans');
    expect(storedTypefaces(memoryStorage({ 'anima.typeface.ui': 'atkinson-hyperlegible-next' })).ui).toBe('ibm-plex-sans');
  });

  it('keeps a stored opendyslexic for the interface and fira-code or opendyslexic for monospace', () => {
    const stored = storedTypefaces(memoryStorage({ 'anima.typeface.ui': 'opendyslexic', 'anima.typeface.mono': 'fira-code' }));
    expect(stored).toEqual({ ui: 'opendyslexic', mono: 'fira-code' });
    expect(storedTypefaces(memoryStorage({ 'anima.typeface.mono': 'opendyslexic' })).mono).toBe('opendyslexic');
  });

  it('does not offer ibm-plex-sans as a monospace font', () => {
    expect(storedTypefaces(memoryStorage({ 'anima.typeface.mono': 'ibm-plex-sans' })).mono).toBe('fira-code');
  });
});

describe('applyTypefaces (UI-020)', () => {
  it('marks the root with each role’s typeface', () => {
    const root = document.createElement('html');
    applyTypefaces(root, { ui: 'opendyslexic', mono: 'fira-code' });
    expect(root.dataset).toMatchObject({ fontUi: 'opendyslexic', fontMono: 'fira-code' });
  });
});
