import { readAccessToken } from '$lib/desktop/token';
import { checkHubListener } from '$lib/desktop/hub-listener/shell';
import { readLink } from '$lib/desktop/link/shell';
import type { HubAddress, TokenProvider } from '$lib/hub/hub';
import { listenerGate, listenerProblem, needsListenerCheck, type ListenerProblem } from './listener';

// The shell side of the loopback check (F-10): one gate for the page load, shared by the hub's token and the sign-in page.

/** This page load's checks; a reload asks the shell again. */
export const hubListeners = listenerGate(checkHubListener);

/** The key store's reader for the hub at `address`: behind the listener check when the address is this computer's (F-10). */
export function tokenFor(address: HubAddress, desktop: boolean): TokenProvider {
  return needsListenerCheck(address.origin, desktop) ? hubListeners.token(readAccessToken) : readAccessToken;
}

/** Why the client sends nothing to `origin`, with the device link's state on a joined device, or `null` when it may (F-10). */
export async function loopbackProblem(origin: string, desktop: boolean): Promise<ListenerProblem | null> {
  if (!needsListenerCheck(origin, desktop)) return null;
  const verdict = await hubListeners.verdict(origin);
  if (verdict.safe) return null;
  const link = await readLink();
  return listenerProblem(verdict, origin, link.ok ? link.value : null);
}
