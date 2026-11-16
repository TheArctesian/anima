import type { Joined, LinkResult } from '$lib/desktop/link/shell';
import { parseHubAddress, type HubAddress } from '$lib/hub/hub';

/** What a join needs; each part is injected, so that tests need no shell or key store. */
export interface JoinParts {
  readonly join: (invite: string) => Promise<LinkResult<Joined>>;
  /** Removes the access token kept for `address`; rejects when the key store refuses. */
  readonly clearToken: (address: HubAddress) => Promise<void>;
  /** Saves the hub address of the app. */
  readonly saveAddress: (origin: string) => void;
}

export type JoinOutcome = { readonly ok: true; readonly fingerprint: string; readonly origin: string } | { readonly ok: false; readonly message: string };

/** The hub address of the device link at `port`: always `127.0.0.1`, never `localhost` (F-1, F-2). */
export function deviceLinkAddress(port: number): HubAddress | null {
  const parsed = parseHubAddress(`http://127.0.0.1:${String(port)}`);
  return parsed.ok ? parsed.value : null;
}

const PASTE = 'Paste the invite from the hub.';
const NOT_CLEARED = 'This device joined, but the key store kept an old access token for the link address. Try again.';

/**
 * Joins this device with `invite` (FRN-005). On success the token kept for the link's address is removed first, so a
 * token of an earlier hub is never sent to this one (security review S3); only then is the address saved. The invite
 * is never logged or kept in an outcome (LOG-007).
 */
export async function joinHub(invite: string, parts: JoinParts): Promise<JoinOutcome> {
  const text = invite.trim();
  if (text === '') return { ok: false, message: PASTE };
  const joined = await parts.join(text);
  if (!joined.ok) return { ok: false, message: joined.message };
  const address = deviceLinkAddress(joined.value.port);
  if (address === null) return { ok: false, message: NOT_CLEARED };
  try {
    await parts.clearToken(address);
  } catch {
    return { ok: false, message: NOT_CLEARED };
  }
  parts.saveAddress(address.origin);
  return { ok: true, fingerprint: joined.value.fingerprint, origin: address.origin };
}
