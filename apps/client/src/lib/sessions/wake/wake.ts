import type { HubResult } from '$lib/hub/hub';
import type { WakeSent } from './api';

/** One sentence for each way a wake fails. Fixed text: the hub's own text may hold relay addresses (LOG-007). */
const FAILURE_TEXT: Readonly<Record<string, string>> = {
  NOT_FOUND: 'This device no longer exists.',
  VALIDATION: 'The hub did not accept this wake request.',
  CONFLICT: 'Cannot wake this device. Check its MAC address and its site wake relay in settings.',
  UNAVAILABLE: 'Wake relay unreachable, or its wake program failed.',
  INTERNAL: 'The hub failed to send the wake packet.',
  unreachable: 'The hub did not answer. Check the VPN client.',
  malformed: 'The hub did not answer. Check the VPN client.',
};

const OTHER_FAILURE_TEXT = 'Wake packet not sent.';

/** What the operator reads after a wake attempt (RA-031). */
export function wakeText(result: HubResult<WakeSent>): string {
  if (result.ok) return `Wake sent via ${result.value.relayName}.`;
  return FAILURE_TEXT[result.error.code] ?? OTHER_FAILURE_TEXT;
}
