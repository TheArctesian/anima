import type { ListEntry } from './look';

function deviceKeys<T>(entries: readonly ListEntry<T>[]): string {
  return entries
    .filter((entry) => entry.kind === 'device')
    .map((entry) => entry.key)
    .sort()
    .join('\n');
}

/**
 * The list as it stands on screen, with the new data of each entry, when the same devices are listed: each row keeps its
 * place, a group title that is new waits, and a group title that emptied stays with its last count. `null` when a device
 * came or went, so the list takes its new order at once. The row under the pointer then stays the row it was.
 */
export function heldOrder<T>(shown: readonly ListEntry<T>[], next: readonly ListEntry<T>[]): readonly ListEntry<T>[] | null {
  if (deviceKeys(shown) !== deviceKeys(next)) return null;
  const byKey = new Map(next.map((entry) => [entry.key, entry]));
  return shown.map((entry) => byKey.get(entry.key) ?? entry);
}
