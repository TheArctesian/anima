import { desktopBuilderShell } from '$lib/desktop/builder/shell';
import { lockUpdaterKey } from '$lib/desktop/builder/updater-key';
import { readThisMachine } from '$lib/desktop/this-machine';
import { clearBuilderToken, readAccessToken, readBuilderToken, writeBuilderToken } from '$lib/desktop/token';
import { createRequestHub, type Hub, type HubAddress } from '$lib/hub/hub';
import { capabilities } from '$lib/platform/platform';
import { buildsApi } from './api';
import { provideThisBuilder, ThisBuilder, type BuilderIdStore } from './builder.svelte';
import { BUILDER_CALL_TIMEOUT_MS } from './loop';

/** The `localStorage` key of this computer's builder ID at a hub; the ID is not secret, the token is (BLD-023). */
export function builderIdKey(address: HubAddress): string {
  return `anima.builderId.${address.origin}`;
}

/** The builder ID in `storage`; a storage that refuses to be read holds none. */
export function builderIdStore(storage: Storage, address: HubAddress): BuilderIdStore {
  const key = builderIdKey(address);
  return {
    read: () => {
      try {
        return storage.getItem(key);
      } catch {
        return null;
      }
    },
    write: (id) => {
      storage.setItem(key, id);
    },
    clear: () => {
      storage.removeItem(key);
    },
  };
}

/** The name the hub records for this computer: its own name, or a fixed one when the shell cannot say. */
async function computerName(): Promise<string> {
  return (await readThisMachine())?.name ?? 'Anima builder';
}

/**
 * Builds this computer's `ThisBuilder` over the desktop shell and provides it to the components below. A computer that
 * holds a builder token starts its loop at once and runs it while the app runs (BLD-026); the web and iOS clients never
 * build. Call once in the root layout, during component setup, with the operator's hub.
 */
export function provideDesktopBuilder(hub: Hub): void {
  const { address } = hub;
  // The builder's calls carry the builder token and open no socket (D-1, BLD-022); a short timeout keeps reports
  // inside the lease's 15 s (BLD-005).
  const builderHub = createRequestHub({ address, token: readBuilderToken, timeoutMs: BUILDER_CALL_TIMEOUT_MS });
  const builder = new ThisBuilder({
    operator: buildsApi(hub),
    builder: buildsApi(builderHub),
    shell: desktopBuilderShell,
    tokens: {
      read: () => readBuilderToken(address),
      write: (token) => writeBuilderToken(address, token),
      clear: () => clearBuilderToken(address),
    },
    ids: builderIdStore(localStorage, address),
    operatorToken: () => readAccessToken(address),
    lockKey: lockUpdaterKey,
    computerName,
    sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
  });
  provideThisBuilder(builder);
  $effect(() => {
    if (!capabilities().builds) return undefined;
    void builder.resume();
    return () => void builder.stop();
  });
}
