import type { Joined, LinkProblem, LinkResult } from '$lib/desktop/link/shell';
import type { HubAddress } from '$lib/hub/hub';
import { deviceLinkAddress } from '$lib/link/devices';
import { openAccess, type HubAccess } from './access';
import { answer, refusal, type Answer, type Problem } from './account';

/** What the silent join needs; each part is injected, so that tests need no shell or key store. */
export interface JoinParts {
  readonly join: (link: string) => Promise<LinkResult<Joined>>;
  /** Removes the access token kept for `address`; rejects when the key store refuses. */
  readonly clearToken: (address: HubAddress) => Promise<void>;
  readonly writeToken: (address: HubAddress, token: string) => Promise<void>;
  /** Saves the hub address of the app. */
  readonly saveAddress: (origin: string) => void;
}

/** A link that did not answer is a network problem; any other refusal of the link is a refusal. */
const OFFLINE: ReadonlySet<LinkProblem> = new Set(['no-answer']);

/**
 * Joins this computer to the hub of `access` with no step of the person (hosted-mode.md Section 4.3): the link join,
 * then the old token of the link address goes, the new one is kept, and only then the address is saved, so the app
 * never sends one hub's token to a different hub (security review S3). Nothing of `access` is shown or logged.
 */
export async function joinHostedHub(access: HubAccess, parts: JoinParts): Promise<Answer<void>> {
  const opened = openAccess(access);
  if (opened === null) return refusal('refused');
  const joined = await parts.join(opened.link);
  if (!joined.ok) return refusal(OFFLINE.has(joined.problem) ? 'offline' : 'refused');
  const address = deviceLinkAddress(joined.value.port);
  if (address === null) return refusal('refused');
  const kept = await keep(address, opened.token, parts);
  if (kept !== null) return refusal(kept);
  parts.saveAddress(address.origin);
  return answer(undefined);
}

/** Puts `token` in the key store for `address` in place of any old one; a key store that refuses is a refusal. */
async function keep(address: HubAddress, token: string, parts: JoinParts): Promise<Problem | null> {
  try {
    await parts.clearToken(address);
    await parts.writeToken(address, token);
    return null;
  } catch {
    return 'refused';
  }
}
