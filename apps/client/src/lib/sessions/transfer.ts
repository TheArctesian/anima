import type { SessionTransfer, SessionsApi } from './api';

/** The copy and paste flags one session obeys (RA-033), and whether the hub answered them. */
export interface SessionPolicy extends SessionTransfer {
  /** False when the hub did not answer; copy and paste are then off and the view says why. */
  readonly read: boolean;
}

/** What a session obeys when its policy could not be read: copy and paste off (fail closed). */
export const UNREAD_POLICY: SessionPolicy = { copy: false, paste: false, read: false };

/**
 * The device's policy for a session starting now. Every failure gives `UNREAD_POLICY`; a flag that is not exactly
 * `true` counts as off, so a malformed answer cannot turn copy or paste on.
 */
export async function readPolicy(api: Pick<SessionsApi, 'transferPolicy'>, deviceId: string): Promise<SessionPolicy> {
  try {
    const result = await api.transferPolicy(deviceId);
    return result.ok ? { copy: result.value.copy === true, paste: result.value.paste === true, read: true } : UNREAD_POLICY;
  } catch {
    // The hub's calls resolve with typed errors; a rejection here means a broken API, which reads as unread too.
    return UNREAD_POLICY;
  }
}
