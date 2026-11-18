import { DESKTOP_LIMITS, type CopyLimits, type CopyStore } from './copy';
import { indexedCopyStore } from './indexed';

/**
 * The copy of the desktop application (FND-007), or none where the web view has no IndexedDB. A browser keeps no copy:
 * the hub serves its page, so it shows nothing without the hub, and it can be a shared computer.
 */
export function desktopRecentCopy(): { store: CopyStore; limits: CopyLimits } | undefined {
  return 'indexedDB' in globalThis ? { store: indexedCopyStore(globalThis.indexedDB), limits: DESKTOP_LIMITS } : undefined;
}

/** Removes the copy from this computer, for sign-out. */
export async function forgetRecentCopy(): Promise<void> {
  await desktopRecentCopy()?.store.clear();
}
