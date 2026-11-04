import type { HubAddress } from '$lib/hub/hub';
import type { AccessApi } from './api';

/** What sign-out needs; every part is injected so that tests need no desktop shell (CLI-003). */
export interface SignOut {
  readonly api: Pick<AccessApi, 'logout'>;
  readonly address: HubAddress;
  readonly clearToken: (address: HubAddress) => Promise<void>;
}

export const TOKEN_KEPT_MESSAGE = 'Signed out of the hub, but the access token could not be removed from the key store.';

/** Whether a key-store failure only means this client has no key store (a browser): then there is nothing to remove. */
function nothingToClear(reason: unknown): boolean {
  return typeof reason === 'object' && reason !== null && 'problem' in reason && reason.problem === 'not-desktop';
}

/**
 * Ends the token on the hub, then removes it from this machine (SET-002, CLI-003). The local token is cleared even when
 * `logout` fails: an unreachable hub must not keep the operator signed in here. Returns the message to show when the
 * local token could not be removed, or `null` when the client may go to sign-in.
 */
export async function signOut({ api, address, clearToken }: SignOut): Promise<string | null> {
  // The result is ignored on purpose: whatever the hub answers, this machine signs out.
  await api.logout();
  try {
    await clearToken(address);
  } catch (reason) {
    return nothingToClear(reason) ? null : TOKEN_KEPT_MESSAGE;
  }
  return null;
}
