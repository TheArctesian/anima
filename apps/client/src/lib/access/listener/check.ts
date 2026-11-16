import { readAccessToken } from '$lib/desktop/token';
import { checkHubListener } from '$lib/desktop/hub-listener/shell';
import { readLink } from '$lib/desktop/link/shell';
import type { HubAddress, TokenProvider } from '$lib/hub/hub';
import { listenerGate, listenerProblem, needsListenerCheck, type ListenerProblem, type TokenRead } from './listener';

// The shell side of the loopback check (F-10): one gate for the page, shared by every token reader and the sign-in page.

/** The page's checks; each answer stands for a few seconds. */
export const hubListeners = listenerGate({ check: checkHubListener, now: () => Date.now() });

/** `reader` for the hub at `address`: behind the listener check when the address is this computer's (F-10, S-1). */
export function gatedReader(reader: TokenProvider, address: HubAddress, desktop: boolean, read?: TokenRead): TokenProvider {
  return needsListenerCheck(address.origin, desktop) ? hubListeners.token(reader, read) : reader;
}

/** The key store's access token reader for the hub at `address`, behind the listener check (F-10). */
export function tokenFor(address: HubAddress, desktop: boolean, read?: TokenRead): TokenProvider {
  return gatedReader(readAccessToken, address, desktop, read);
}

/** Drops the answer for `origin`, so that the next read asks the shell again (M-C). */
export function forgetListener(origin: string): void {
  hubListeners.forget(origin);
}

/**
 * Why the client sends nothing to `origin`, with the device link's state on a joined device, or `null` when it may (F-10).
 * Always a fresh answer: it is asked right before a password would be sent.
 */
export async function loopbackProblem(origin: string, desktop: boolean): Promise<ListenerProblem | null> {
  if (!needsListenerCheck(origin, desktop)) return null;
  hubListeners.forget(origin);
  const verdict = await hubListeners.verdict(origin);
  if (verdict.safe) return null;
  const link = await readLink();
  return listenerProblem(verdict, origin, link.ok ? link.value : null);
}
