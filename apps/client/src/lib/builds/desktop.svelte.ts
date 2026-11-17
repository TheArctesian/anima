import { gatedReader } from '$lib/access/listener/check';
import { desktopBuilderShell } from '$lib/desktop/builder/shell';
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

/** The name `hub` records for this computer: its own name, or a fixed one when the shell cannot say. */
async function computerName(hub: Hub): Promise<string> {
  return (await readThisMachine(hub))?.name ?? 'Anima.ing builder';
}

/**
 * Builds this computer's `ThisBuilder` over the desktop shell and provides it to the components below. A computer that
 * holds a builder token starts its loop at once and runs it while the app runs (BLD-026); the web and iOS clients never
 * build. Call once in the root layout, during component setup, with the operator's hub.
 */
export function provideDesktopBuilder(hub: Hub): void {
  const { address } = hub;
  // On this computer's hub address the builder token goes on a request only after the listener check (F-10, S-1).
  // The two reads below stay local: they only decide whether the loop runs and what a log line hides.
  const builderToken = gatedReader(readBuilderToken, address, capabilities().keyStore);
  // The builder's calls carry the builder token and open no socket (D-1, BLD-022); a short timeout keeps reports
  // inside the lease's 15 s (BLD-005).
  const builderHub = createRequestHub({ address, token: builderToken, timeoutMs: BUILDER_CALL_TIMEOUT_MS });
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
    computerName: () => computerName(hub),
    sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- a value for the feed's date, never held or rendered
    now: () => new Date(),
  });
  provideThisBuilder(builder);
  $effect(() => {
    if (!capabilities().builds) return undefined;
    void builder.resume();
    return () => void builder.stop();
  });
}
