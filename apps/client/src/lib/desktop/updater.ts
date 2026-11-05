import type { AvailableUpdate, UpdaterPort } from '$lib/updates/port';

/** Which shell call failed. */
export type UpdaterStep = 'check' | 'install' | 'restart' | 'version';

const MESSAGES: Record<UpdaterStep, string> = {
  check: 'The update feed could not be checked.',
  install: 'The update could not be installed.',
  restart: 'The app could not restart.',
  version: 'The app version could not be read.',
};

/** An updater failure. The message is fixed text chosen here; nothing from the plugin's error is kept (LOG-007). */
export class UpdaterError extends Error {
  constructor(readonly step: UpdaterStep) {
    super(MESSAGES[step]);
    this.name = 'UpdaterError';
  }
}

/** Runs `call`, and turns any rejection into an `UpdaterError` with no `cause`. */
async function inShell<T>(step: UpdaterStep, call: () => Promise<T>): Promise<T> {
  try {
    return await call();
  } catch {
    throw new UpdaterError(step);
  }
}

/**
 * The updater and process plugins behind the update schedule's port. The plugins load on first use, so the web and iOS
 * clients, which never call this, never load them (UPD-001). The signature is verified by the plugin before it
 * installs (UPD-002); `allowDowngrades` is set in the shell's configuration, not here (R-006).
 */
export const desktopUpdater: UpdaterPort = {
  async check(): Promise<AvailableUpdate | null> {
    const { check } = await import('@tauri-apps/plugin-updater');
    const found = await inShell('check', () => check());
    if (found === null) return null;
    return {
      version: found.version,
      install: () =>
        inShell('install', async () => {
          try {
            await found.downloadAndInstall();
          } finally {
            // Frees the shell's handle; a failure to free it does not change whether the install worked.
            await found.close().catch(() => {
              console.warn('The update handle could not be closed.');
            });
          }
        }),
    };
  },

  async restart(): Promise<void> {
    const { relaunch } = await import('@tauri-apps/plugin-process');
    await inShell('restart', () => relaunch());
  },

  async currentVersion(): Promise<string> {
    const { getVersion } = await import('@tauri-apps/api/app');
    return inShell('version', () => getVersion());
  },
};
