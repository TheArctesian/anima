import type { AccessApi } from '$lib/access/api';
import { listenerProblem, verdictOf } from '$lib/access/listener/listener';
import type { ListenerResult } from '$lib/desktop/hub-listener/shell';
import type { LocalHubResult, LocalHubStatus } from '$lib/desktop/local-hub/shell';
import { parseHubAddress, type HubAddress } from '$lib/hub/hub';
import { POLL_MS } from '$lib/local-hub/state';

/** What connecting to the hub on this computer needs; each part is injected (CQ-021). */
export interface ConnectParts {
  readonly read: () => Promise<LocalHubResult<LocalHubStatus>>;
  readonly check: (origin: string) => Promise<ListenerResult>;
  readonly saveAddress: (origin: string) => void;
  /** Waits between reads while the hub is not ready yet. */
  readonly wait: (ms: number) => Promise<void>;
}

export type Connected = { readonly ok: true; readonly address: HubAddress } | { readonly ok: false; readonly message: string };

const NOT_READY = 'The hub on this computer is not ready. Retry shortly.';

/** How many times a hub that is not ready yet is read, one second apart, before the step says so. */
export const READY_READS = 15;

/** The local hub's address, read again each second while the hub is not ready, for up to `READY_READS` reads. */
async function readyAddress(parts: ConnectParts): Promise<Connected> {
  for (let read = 1; ; read += 1) {
    const status = await parts.read();
    if (!status.ok) return { ok: false, message: status.message };
    const address = localHubAddress(status.value);
    if (address !== null) return { ok: true, address };
    if (read >= READY_READS) return { ok: false, message: NOT_READY };
    await parts.wait(POLL_MS);
  }
}

/**
 * The hub address of the local hub: `http://127.0.0.1:<port>` only (F-1), or `null` for anything else. The address
 * rules permit plain `http` for the host `127.0.0.1` alone, so any other host fails to parse.
 */
export function localHubAddress(status: LocalHubStatus): HubAddress | null {
  if (status.state !== 'ready' || status.hub === undefined) return null;
  const parsed = parseHubAddress(`http://${status.hub}`);
  return parsed.ok ? parsed.value : null;
}

/**
 * Finds the hub on this computer, checks that a program of this user listens at its address (F-10), and only then saves
 * the address. On "other" or "none" nothing is saved and nothing is sent.
 */
export async function connectLocalHub(parts: ConnectParts): Promise<Connected> {
  const found = await readyAddress(parts);
  if (!found.ok) return found;
  const { address } = found;
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

const KEPT_NOT = 'Signed in, but the key store could not keep the token.';

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
        return { ok: false, message: 'The hub refused the code. Wait for the next one.' };
      case 'unreachable':
        return { ok: false, message: 'The hub did not answer. Retry shortly.' };
      default:
        return { ok: false, message: 'Sign-in failed. Retry.' };
    }
  }
  try {
    await keep(result.value);
  } catch {
    return { ok: false, message: KEPT_NOT };
  }
  return { ok: true };
}
