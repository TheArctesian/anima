// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { applyMotion, DEFAULT_MOTION, MOTION_CHOICES, storedMotion, storeMotion } from './motion-choice';

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

describe('storedMotion (MOT-043, D-28)', () => {
  it('MOT-043: is System when nothing is stored', () => {
    expect(DEFAULT_MOTION).toBe('system');
    expect(storedMotion(memoryStorage())).toBe('system');
  });

  it('MOT-043: gives back each stored choice, saved under anima.motion', () => {
    for (const { id } of MOTION_CHOICES) {
      const storage = memoryStorage();
      storeMotion(storage, id);
      expect(storage.getItem('anima.motion')).toBe(id);
      expect(storedMotion(storage)).toBe(id);
    }
  });

  it('MOT-043: an unknown stored value gives System, and is never passed on', () => {
    for (const text of ['on', 'REDUCE', '', 'reduce"] body{', ' full', 'true']) {
      expect(storedMotion(memoryStorage({ 'anima.motion': text }))).toBe('system');
    }
  });

  it('MOT-043: a storage that refuses to be read gives System', () => {
    const refusing: Storage = {
      ...memoryStorage(),
      getItem: () => {
        throw new Error('refused');
      },
    };
    expect(storedMotion(refusing)).toBe('system');
  });
});

describe('applyMotion (MOT-043, F-7)', () => {
  it('MOT-043: On and Off mark the root with reduce and full; System takes the mark away', () => {
    const root = document.createElement('html');
    applyMotion(root, 'reduce');
    expect(root.getAttribute('data-motion')).toBe('reduce');
    applyMotion(root, 'full');
    expect(root.getAttribute('data-motion')).toBe('full');
    applyMotion(root, 'system');
    expect(root.hasAttribute('data-motion')).toBe(false);
  });
});
