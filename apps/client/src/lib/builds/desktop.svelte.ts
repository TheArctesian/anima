import { desktopBuilderShell } from '$lib/desktop/builder/shell';
import { readThisMachine } from '$lib/desktop/thisMachine';
import { clearBuilderToken, keyStoreAvailable, readBuilderToken, writeBuilderToken } from '$lib/desktop/token';
import { createRequestHub, type Hub, type HubAddress } from '$lib/hub/hub';
import { buildsApi } from './api';
import { provideThisBuilder, ThisBuilder, type BuilderIdStore } from './builder.svelte';

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
  // The builder's calls carry the builder token and open no socket (D-1, BLD-022).
  const builderHub = createRequestHub({ address, token: readBuilderToken });
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
    computerName,
    sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
  });
  provideThisBuilder(builder);
  $effect(() => {
    if (!keyStoreAvailable()) return undefined;
    void builder.resume();
    return () => void builder.stop();
  });
}
