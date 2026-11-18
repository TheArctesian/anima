import type { Copy, CopyStore } from './copy';

// Test support: a copy store in memory, with the same oldest-first rule as the real one.

/** A store that holds its copies in a map; `failing` makes every call reject, as a refused IndexedDB does. */
export function memoryCopyStore(failing = false): CopyStore & { readonly copies: Map<string, Copy> } {
  const copies = new Map<string, Copy>();
  const run = <T>(work: () => T): Promise<T> => (failing ? Promise.reject(new Error('refused')) : Promise.resolve(work()));
  return {
    copies,
    read: (key) => run(() => copies.get(key)),
    write: (copy, most) =>
      run(() => {
        copies.set(copy.key, copy);
        const oldestFirst = [...copies.values()].toSorted((a, b) => a.at - b.at);
        for (const extra of oldestFirst.slice(0, Math.max(0, copies.size - most))) copies.delete(extra.key);
      }),
    remove: (key) => run(() => void copies.delete(key)),
    clear: () => run(() => copies.clear()),
  };
}
