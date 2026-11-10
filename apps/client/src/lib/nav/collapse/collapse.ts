const STORAGE_KEY = 'anima.nav.collapsed';

/** Whether the operator collapsed the navigation bar on this device; open when nothing is stored or storage refuses. */
export function storedCollapsed(storage: Storage): boolean {
  try {
    return storage.getItem(STORAGE_KEY) === 'true';
  } catch {
    return false;
  }
}

/** Save the operator's choice on this device; a refusing storage (quota, privacy modes) only loses it for next time. */
export function storeCollapsed(storage: Storage, collapsed: boolean): void {
  try {
    storage.setItem(STORAGE_KEY, String(collapsed));
  } catch {
    // The bar still collapses for this run; only the next start forgets the choice.
  }
}
