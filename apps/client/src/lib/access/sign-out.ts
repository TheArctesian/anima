import type { HubAddress, HubResult } from '$lib/hub/hub';
import type { AccessApi } from './api';

/** What sign-out needs; every part is injected so that tests need no desktop shell (CLI-003). */
export interface SignOut {
  readonly api: Pick<AccessApi, 'logout'>;
  readonly address: HubAddress;
  /**
   * Whether the key store exists. The desktop app removes its own token; a browser's token is in the `HttpOnly` cookie,
   * which script cannot touch, so only the hub's `logout` clears it (CLI-003, E-4).
   */
  readonly desktop: boolean;
  readonly clearToken: (address: HubAddress) => Promise<void>;
}

export const TOKEN_KEPT_AFTER_LOGOUT = 'Signed out of the hub, but the access token could not be removed from the key store.';
export const TOKEN_KEPT_UNCONFIRMED =
  'The hub did not confirm the sign-out, and the access token could not be removed from the key store.';
export const COOKIE_KEPT = 'The hub did not confirm the sign-out, so this browser may still be signed in. Try again.';

/** The hub's answer to `logout`; a rejected call is no answer (`null`). */
async function logoutAnswer(api: Pick<AccessApi, 'logout'>): Promise<HubResult<boolean> | null> {
  try {
    return await api.logout();
  } catch {
    // Whatever went wrong, the caller words its message by the missing answer.
    return null;
  }
}

/**
 * Whether the browser holds no valid cookie now: the hub confirmed the logout (and cleared the cookie), or refused the
 * cookie as no longer valid. Any other answer may leave a valid cookie that script cannot remove (E-4).
 */
function cookieGone(answer: HubResult<boolean> | null): boolean {
  return answer !== null && (answer.ok || answer.error.code === 'UNAUTHENTICATED');
}

/**
 * Ends the token on the hub, then, on desktop, removes it from this machine (SET-002, CLI-003). The desktop app clears
 * its token whatever `logout` does: an unreachable hub must not keep the operator signed in here. A browser is signed
 * out only once the hub says so, since its cookie is out of script's reach (E-4). Returns the message to show when a
 * token may remain, or `null` when the client may go to sign-in.
 */
export async function signOut({ api, address, desktop, clearToken }: SignOut): Promise<string | null> {
  const answer = await logoutAnswer(api);
  if (!desktop) return cookieGone(answer) ? null : COOKIE_KEPT;
  try {
    await clearToken(address);
  } catch {
    return answer?.ok === true ? TOKEN_KEPT_AFTER_LOGOUT : TOKEN_KEPT_UNCONFIRMED;
  }
  return null;
}
