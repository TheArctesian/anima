import type { HubResult } from '$lib/hub/hub';
import type { AcceptedHostKey, ConnectionTest } from '../../api';

/** The key the operator reviewed: exactly what the hub's connection test returned. */
export interface ReviewedKey {
  readonly algorithm: string;
  readonly fingerprintSha256: string;
}

/**
 * Where the host key warning stands (D-6, SEC-007). Accepting needs a review first (`reviewed`), then a second,
 * explicit step (`confirming`); `accepting` sends the reviewed fingerprint once.
 */
export type HostKeyStep =
  | { readonly step: 'warned'; readonly problem?: string }
  | { readonly step: 'reviewing' }
  | { readonly step: 'reviewed'; readonly key: ReviewedKey }
  | { readonly step: 'confirming'; readonly key: ReviewedKey }
  | { readonly step: 'accepting'; readonly key: ReviewedKey }
  | { readonly step: 'accepted'; readonly key: ReviewedKey };

export type HostKeyEvent =
  | { readonly event: 'review' }
  | { readonly event: 'reviewed'; readonly result: HubResult<ConnectionTest> }
  | { readonly event: 'confirm' }
  | { readonly event: 'back' }
  | { readonly event: 'accept' }
  | { readonly event: 'accepted'; readonly result: HubResult<AcceptedHostKey> };

export const WARNED: HostKeyStep = { step: 'warned' };

/** Fixed texts; the hub's own messages are not shown (R-035). */
export const PROBLEM = {
  review: 'The connection test failed, so no new key could be read. Try the review again.',
  unreachable: 'The device could not be reached, so no new key could be read. Try the review again.',
  noKey: 'The connection test did not report a new host key. Close this warning and reconnect.',
  accept: 'The hub did not accept the key. It may have changed again since the review: review it again.',
} as const;

function reviewed(result: HubResult<ConnectionTest>): HostKeyStep {
  if (!result.ok) return { step: 'warned', problem: PROBLEM.review };
  const { hostKey, reachable } = result.value;
  if (hostKey === null) return { step: 'warned', problem: reachable ? PROBLEM.noKey : PROBLEM.unreachable };
  return { step: 'reviewed', key: { algorithm: hostKey.algorithm, fingerprintSha256: hostKey.fingerprintSha256 } };
}

function accepted(key: ReviewedKey, result: HubResult<AcceptedHostKey>): HostKeyStep {
  // The hub answers with the key it now holds; anything but the reviewed fingerprint is not this accept.
  if (result.ok && result.value.fingerprintSha256 === key.fingerprintSha256) return { step: 'accepted', key };
  return { step: 'warned', problem: PROBLEM.accept };
}

/**
 * The step `event` leads to, or undefined when it does not apply to `current`. Results apply only in the step that
 * asked for them, so a late or repeated answer, or a second click, does nothing; a new review clears an older key.
 */
export function transition(current: HostKeyStep, event: HostKeyEvent): HostKeyStep | undefined {
  switch (event.event) {
    case 'review':
      return current.step === 'warned' || current.step === 'reviewed' ? { step: 'reviewing' } : undefined;
    case 'reviewed':
      return current.step === 'reviewing' ? reviewed(event.result) : undefined;
    case 'confirm':
      return current.step === 'reviewed' ? { step: 'confirming', key: current.key } : undefined;
    case 'back':
      return current.step === 'confirming' ? { step: 'reviewed', key: current.key } : undefined;
    case 'accept':
      return current.step === 'confirming' ? { step: 'accepting', key: current.key } : undefined;
    case 'accepted':
      return current.step === 'accepting' ? accepted(current.key, event.result) : undefined;
  }
}
