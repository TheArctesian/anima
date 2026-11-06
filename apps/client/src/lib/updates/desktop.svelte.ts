import { keyStoreAvailable } from '$lib/desktop/token';
import { warnToProcessLog } from '$lib/desktop/processLog';
import { desktopUpdater } from '$lib/desktop/updater';
import { provideUpdates, Updates } from './updates.svelte';

/**
 * Builds the app's `Updates` over the desktop shell and provides it to the components below. Checking starts only in the
 * desktop production app: dev shells have no updater plugin, and the web and iOS clients never check (UPD-001).
 * Call once in the root layout, during component setup.
 */
export function provideDesktopUpdates(): void {
  const updates = new Updates(desktopUpdater, { warn: warnToProcessLog });
  provideUpdates(updates);
  $effect(() => (keyStoreAvailable() && !import.meta.env.DEV ? updates.start() : undefined));
}
