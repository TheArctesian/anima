import { DESKTOP_LIMITS, type CopyPlace } from './copy';
import { indexedCopyStore } from './indexed';

/** Changes each time this page removes the copy, so that an answer on its way at that time is not kept. */
let era = 0;

/**
 * The copy of the desktop application (FND-007), or none where the web view has no IndexedDB. A browser keeps no copy:
 * the hub serves its page, so it shows nothing without the hub, and it can be a shared computer.
 */
export function desktopRecentCopy(): CopyPlace | undefined {
  if (!('indexedDB' in globalThis)) return undefined;
  return { store: indexedCopyStore(globalThis.indexedDB), limits: DESKTOP_LIMITS, era: () => era };
}

/** Removes the copy from this computer, for sign-out and for a computer that leaves its hub. */
export async function forgetRecentCopy(): Promise<void> {
  era += 1;
  await desktopRecentCopy()?.store.clear();
}
