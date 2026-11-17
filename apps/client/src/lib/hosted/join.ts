import type { DeviceLinkStatus, Joined, LinkProblem, LinkResult } from '$lib/desktop/link/shell';
import type { HubAddress, HubResult } from '$lib/hub/hub';
import { deviceLinkAddress } from '$lib/link/devices';
import { openAccess, type HubAccess } from './access';
import { answer, refusal, type Answer, type Problem } from './account';

/** What the silent join needs; each part is injected, so that tests need no shell, key store or hub. */
export interface JoinParts {
  readonly join: (invite: string) => Promise<LinkResult<Joined>>;
  /** The device link of this computer, when it is already on the hub: its local port. */
  readonly link: () => Promise<LinkResult<DeviceLinkStatus>>;
  /** Trades the claim with the hub at `address` for this device's token; the request carries no token. */
  readonly claim: (address: HubAddress, claim: string) => Promise<HubResult<string>>;
  /** Removes the access token kept for `address`; rejects when the key store refuses. */
  readonly clearToken: (address: HubAddress) => Promise<void>;
  readonly writeToken: (address: HubAddress, token: string) => Promise<void>;
  /** Saves the hub address of the app and marks it hosted. */
  readonly saveAddress: (origin: string) => void;
}

/** A link that did not answer is a network problem; any other refusal of the link is a refusal. */
const OFFLINE_LINK: ReadonlySet<LinkProblem> = new Set(['no-answer']);
const OFFLINE_HUB: ReadonlySet<string> = new Set(['unreachable', 'UNAVAILABLE']);

/**
 * Joins this computer to the hub of `access` and signs it in, with no step of the person (hosted-mode.md Section 4.3,
 * R-129): the link join when there is an invite, then the old token of the link address goes, the claim gives the new
 * token through the link, the key store keeps it, and only then the address is saved, so the app never sends one hub's
 * token to a different hub (security review S3). Nothing of `access` is shown or logged.
 */
export async function joinHostedHub(access: HubAccess, parts: JoinParts): Promise<Answer<void>> {
  const opened = openAccess(access);
  if (opened === null) return refusal('refused');
  const port = await linkPort(opened.invite, parts);
  if (!port.ok) return port;
  const address = deviceLinkAddress(port.value);
  if (address === null) return refusal('refused');
  const token = await signIn(address, opened.claim, parts);
  if (!token.ok) return token;
  parts.saveAddress(address.origin);
  return answer(undefined);
}

/** The local port of the device link: from the join of `invite`, or from the link this computer is already on. */
async function linkPort(invite: string | null, parts: JoinParts): Promise<Answer<number>> {
  if (invite !== null) {
    const joined = await parts.join(invite);
    if (!joined.ok) return refusal(OFFLINE_LINK.has(joined.problem) ? 'offline' : 'refused');
    return answer(joined.value.port);
  }
  const link = await parts.link();
  if (!link.ok || !link.value.joined || link.value.port === undefined) return refusal('refused');
  return answer(link.value.port);
}

/** The old token goes, the claim gives the new one, the key store keeps it. */
async function signIn(address: HubAddress, claim: string, parts: JoinParts): Promise<Answer<void>> {
  const kept = await keyStore(() => parts.clearToken(address));
  if (kept !== null) return refusal(kept);
  const token = await parts.claim(address, claim);
  if (!token.ok) return refusal(OFFLINE_HUB.has(token.error.code) ? 'offline' : 'refused');
  const written = await keyStore(() => parts.writeToken(address, token.value));
  return written === null ? answer(undefined) : refusal(written);
}

/** A key store that refuses is a refusal. */
async function keyStore(change: () => Promise<void>): Promise<Problem | null> {
  try {
    await change();
    return null;
  } catch {
    return 'refused';
  }
}

