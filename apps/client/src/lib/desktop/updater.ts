/** An update the feed offers: its version, and the download and install of it (UPD-003). */
export interface AvailableUpdate {
  readonly version: string;
  /** Downloads, verifies the signature (UPD-002) and installs. Rejects when any of that fails. */
  install(): Promise<void>;
}

/** What the desktop shell offers the update schedule: the updater and process plugins, behind one shape. */
export interface UpdaterPort {
  /** The update the feed offers, or `null` when the app is current. Rejects when the feed cannot be read. */
  check(): Promise<AvailableUpdate | null>;
  /** Restarts the app into the installed update (UPD-003: only when the operator asks). */
  restart(): Promise<void>;
  /** The version of the running app. */
  currentVersion(): Promise<string>;
}

/** Which shell call failed. */
export type UpdaterStep = 'check' | 'install' | 'restart' | 'version';

const MESSAGES: Record<UpdaterStep, string> = {
  check: 'The update feed could not be checked.',
  install: 'The update could not be installed.',
  restart: 'The app could not restart.',
  version: 'The app version could not be read.',
};

/**
 * Why an update failed, in the three cases the operator can act on: the feed has no release for this target (a 404, an
 * absent feed, or no entry for the platform), the signature was refused (UPD-002),
 * or anything else (the network, the disk).
 */
export type UpdateFailure = 'no-feed' | 'signature' | 'other';

const NO_FEED = /valid release JSON|was not found in the response|fallback platforms/i;
const SIGNATURE = /signature|signed for/i;

/** Sorts the plugin's error text into an `UpdateFailure`. Only the sort is kept, never the text (LOG-007). */
export function failureOf(error: unknown): UpdateFailure {
  const text = typeof error === 'string' ? error : error instanceof Error ? error.message : '';
  if (SIGNATURE.test(text)) return 'signature';
  if (NO_FEED.test(text)) return 'no-feed';
  return 'other';
}

/** An updater failure. The message is fixed text chosen here; nothing from the plugin's error is kept (LOG-007). */
export class UpdaterError extends Error {
  constructor(
    readonly step: UpdaterStep,
    readonly reason: UpdateFailure = 'other',
  ) {
    super(MESSAGES[step]);
    this.name = 'UpdaterError';
  }
}

/** Runs `call`, and turns any rejection into an `UpdaterError` with no `cause`. */
async function inShell<T>(step: UpdaterStep, call: () => Promise<T>): Promise<T> {
  try {
    return await call();
  } catch (error) {
    throw new UpdaterError(step, failureOf(error));
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
