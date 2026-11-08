import type { UpdaterPort } from '$lib/desktop/updater';
import { warnToProcessLog } from '$lib/desktop/process-log';
import { capabilities } from '$lib/platform/platform';
import { provideUpdates, Updates } from './updates.svelte';

/** The shell's updater, loaded by the first call that needs it: a client that never calls it never loads it (UPD-001). */
const load = () => import('$lib/desktop/updater');
const lazyUpdater: UpdaterPort = {
  check: async () => (await load()).desktopUpdater.check(),
  restart: async () => (await load()).desktopUpdater.restart(),
  currentVersion: async () => (await load()).desktopUpdater.currentVersion(),
};

/**
 * Builds the app's `Updates` over the desktop shell and provides it to the components below. Checking starts only where
 * the platform updates itself, in a production build: dev shells have no updater plugin, and the web and iOS clients
 * never check, so they never load the updater (UPD-001). Call once in the root layout, during component setup.
 */
export function provideDesktopUpdates(): void {
  const updates = new Updates(lazyUpdater, { warn: warnToProcessLog });
  provideUpdates(updates);
  $effect(() => (capabilities().updates && !import.meta.env.DEV ? updates.start() : undefined));
}
