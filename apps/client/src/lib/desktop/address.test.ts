import { describe, expect, it } from 'vitest';
import { DEFAULT_HUB_ADDRESS } from '$lib/hub/hub';
import { storedHubAddress, storeHubAddress } from './address';

function memoryStorage(initial: Record<string, string> = {}): Storage {
  const items = new Map(Object.entries(initial));
  return {
    get length() {
      return items.size;
    },
    clear: () => {
      items.clear();
    },
    getItem: (key) => items.get(key) ?? null,
    key: (index) => [...items.keys()][index] ?? null,
    removeItem: (key) => {
      items.delete(key);
    },
    setItem: (key, value) => {
      items.set(key, value);
    },
  };
}

describe('hub address setting', () => {
  it('CLI-002: is the default hub when nothing is stored', () => {
    expect(storedHubAddress(memoryStorage())).toEqual(DEFAULT_HUB_ADDRESS);
  });

  it('CLI-002: a stored valid address is used', () => {
    const storage = memoryStorage();
    expect(storeHubAddress(storage, ' my-hub.example:8443 ').ok).toBe(true);
    expect(storedHubAddress(storage).origin).toBe('https://my-hub.example:8443');
  });

  it('CLI-002: a stored address that no longer parses falls back to the default', () => {
    const storage = memoryStorage({
      'anima.hubAddress': 'http://insecure.example',
    });
    expect(storedHubAddress(storage)).toEqual(DEFAULT_HUB_ADDRESS);
  });

  it('CLI-002: the canonical origin is stored, not the typed text', () => {
    const storage = memoryStorage();
    storeHubAddress(storage, ' HTTPS://My-Hub.example:443 ');
    expect(storage.getItem('anima.hubAddress')).toBe('https://my-hub.example');
  });

  it('CLI-002: storage that throws on read gives the default hub', () => {
    const storage = memoryStorage();
    storage.getItem = () => {
      throw new DOMException('denied', 'SecurityError');
    };
    expect(storedHubAddress(storage)).toEqual(DEFAULT_HUB_ADDRESS);
  });

  it('CLI-002: a refused address is returned as the error and the old address stays', () => {
    const storage = memoryStorage();
    storeHubAddress(storage, 'first.example');
    const result = storeHubAddress(storage, 'http://second.example');
    expect(result.ok).toBe(false);
    expect(storedHubAddress(storage).origin).toBe('https://first.example');
  });
});
