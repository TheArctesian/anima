import type { AccessApi } from '$lib/access/api';
import { listenerProblem, verdictOf } from '$lib/access/listener/listener';
import type { ListenerResult } from '$lib/desktop/hub-listener/shell';
import type { LocalHubResult, LocalHubStatus } from '$lib/desktop/local-hub/shell';
import { parseHubAddress, type HubAddress } from '$lib/hub/hub';

/** What connecting to the hub on this computer needs; each part is injected (CQ-021). */
export interface ConnectParts {
  readonly read: () => Promise<LocalHubResult<LocalHubStatus>>;
  readonly check: (origin: string) => Promise<ListenerResult>;
  readonly saveAddress: (origin: string) => void;
}

export type Connected = { readonly ok: true; readonly address: HubAddress } | { readonly ok: false; readonly message: string };

const NOT_READY = 'The hub on this computer is not ready. Wait for it, then try again.';

/** The hub address of the local hub: `http://127.0.0.1:<port>` only (F-1), or `null` for anything else. */
export function localHubAddress(status: LocalHubStatus): HubAddress | null {
  if (status.state !== 'ready' || status.hub === undefined) return null;
  const parsed = parseHubAddress(`http://${status.hub}`);
  if (!parsed.ok || new URL(parsed.value.origin).hostname !== '127.0.0.1') return null;
  return parsed.value;
}

/**
 * Finds the hub on this computer, checks that a program of this user listens at its address (F-10), and only then saves
 * the address. On "other" or "none" nothing is saved and nothing is sent.
 */
export async function connectLocalHub(parts: ConnectParts): Promise<Connected> {
  const status = await parts.read();
  if (!status.ok) return { ok: false, message: status.message };
  const address = localHubAddress(status.value);
  if (address === null) return { ok: false, message: NOT_READY };
  const verdict = verdictOf(await parts.check(address.origin));
  const problem = listenerProblem(verdict, address.origin, null);
  if (problem !== null) return { ok: false, message: `${problem.title}. ${problem.advice}` };
  parts.saveAddress(address.origin);
  return { ok: true, address };
}

/** The account's name, password and the next code. All but the name are secrets (LOG-007). */
export interface FirstSignIn {
  readonly name: string;
  readonly password: string;
  readonly code: string;
}

export type SignedIn = { readonly ok: true } | { readonly ok: false; readonly message: string };

const KEPT_NOT = 'Signed in, but the access token could not be kept in the key store.';

/** Signs in to the new hub with the next code and keeps the token in the key store (FRN-004, CLI-003). */
export async function signInFirst(
  api: Pick<AccessApi, 'login'>,
  keep: (token: string) => Promise<void>,
  form: FirstSignIn,
): Promise<SignedIn> {
  const result = await api.login({ username: form.name, password: form.password, totpCode: form.code });
  if (!result.ok) {
    switch (result.error.code) {
      case 'UNAUTHENTICATED':
        return { ok: false, message: 'The hub refused the code. Wait for the next code: the code that made the account cannot sign in.' };
      case 'unreachable':
        return { ok: false, message: 'The hub did not answer. Wait a moment, then try again.' };
      default:
        return { ok: false, message: 'The hub could not sign you in. Try again.' };
    }
  }
  try {
    await keep(result.value);
  } catch {
    return { ok: false, message: KEPT_NOT };
  }
  return { ok: true };
}
