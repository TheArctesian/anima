import type { Joined, LinkProblem, LinkResult } from '$lib/desktop/link/shell';
import type { HubAddress } from '$lib/hub/hub';
import { deviceLinkAddress } from '$lib/link/devices';

/** What a join needs; each part is injected, so that tests need no shell or key store. */
export interface JoinParts {
  readonly join: (invite: string) => Promise<LinkResult<Joined>>;
  /** Removes the access token kept for `address`; rejects when the key store refuses. */
  readonly clearToken: (address: HubAddress) => Promise<void>;
  /** Saves the hub address of the app. */
  readonly saveAddress: (origin: string) => void;
}

/** The end of the steps after the join: the address is saved, or why not. */
export type Finished = { readonly ok: true; readonly origin: string } | { readonly ok: false; readonly message: string };

/**
 * A join: refused (`joined: false`), or joined with its device key and the end of the steps after it. A joined device
 * whose address was not saved retries only those steps: the invite is spent.
 */
export type JoinOutcome =
  | { readonly joined: true; readonly device: Joined; readonly finished: Finished }
  | { readonly joined: false; readonly message: string; readonly inviteKept: boolean };

const PASTE = 'Paste the invite from the hub.';
const NOT_CLEARED = 'This device joined, but the key store kept an old access token for the link address. Retry.';
const NO_ADDRESS = 'This device joined, but the link gave no address this app can use. Retry.';

/** Refusals that come before the invite reaches the hub: the pasted invite still works, so the field keeps it. */
const BEFORE_USE: ReadonlySet<LinkProblem> = new Set(['caller', 'not-carried', 'busy', 'no-home', 'bad-invite', 'unavailable']);

/**
 * The steps after a join (FRN-005): the token kept for the link's address is removed first, so a token of an earlier
 * hub is never sent to this one (security review S3); only then is the address saved.
 */
export async function finishJoin(device: Joined, parts: Pick<JoinParts, 'clearToken' | 'saveAddress'>): Promise<Finished> {
  const address = deviceLinkAddress(device.port);
  if (address === null) return { ok: false, message: NO_ADDRESS };
  try {
    await parts.clearToken(address);
  } catch {
    return { ok: false, message: NOT_CLEARED };
  }
  parts.saveAddress(address.origin);
  return { ok: true, origin: address.origin };
}

/** Joins this device with `invite` (FRN-005). The invite is never logged or kept in an outcome (LOG-007). */
export async function joinHub(invite: string, parts: JoinParts): Promise<JoinOutcome> {
  const text = invite.trim();
  if (text === '') return { joined: false, message: PASTE, inviteKept: true };
  const joined = await parts.join(text);
  if (!joined.ok) return { joined: false, message: joined.message, inviteKept: BEFORE_USE.has(joined.problem) };
  return { joined: true, device: joined.value, finished: await finishJoin(joined.value, parts) };
}
