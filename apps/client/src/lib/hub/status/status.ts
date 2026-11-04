/** Whether the hub answers: `connected` while it does, `unreachable` after a failure, `connecting` before the first answer (NAV-003, NET-004). */
export type Connection = 'connecting' | 'connected' | 'unreachable';

export interface HubStatus {
  readonly connection: Connection;
  /** Last time the hub answered; the age of the data on the screen (NET-004). `null` until the first answer. */
  readonly lastContactAt: Date | null;
}

/**
 * What the transport saw.
 * Socket: `opened` (the upgrade succeeded), `refused` (closed 4403: the hub answered and refused the token),
 * `closed` (any other abnormal close), `error`.
 * Requests: `answered` (any HTTP answer with a GraphQL body, even `UNAUTHENTICATED`), `failed` (no answer).
 */
export type StatusSignal = 'opened' | 'refused' | 'closed' | 'error' | 'answered' | 'failed';

export const INITIAL_STATUS: HubStatus = { connection: 'connecting', lastContactAt: null };

/** The status after `signal` arrives at `now` (NAV-003, NET-004). */
export function nextStatus(state: HubStatus, signal: StatusSignal, now: Date): HubStatus {
  switch (signal) {
    case 'opened':
    case 'refused':
    case 'answered':
      return { connection: 'connected', lastContactAt: now };
    case 'closed':
      // A drop from a live connection is retried first; a close before any answer means no answer.
      return { ...state, connection: state.connection === 'connected' ? 'connecting' : 'unreachable' };
    case 'error':
    case 'failed':
      return { ...state, connection: 'unreachable' };
  }
}
