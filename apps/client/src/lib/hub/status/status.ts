/** Whether the hub answers: `connected` while it does, `unreachable` after a failure, `connecting` before the first answer (NAV-003, NET-004). */
export type Connection = 'connecting' | 'connected' | 'unreachable';

export interface HubStatus {
  readonly connection: Connection;
  /** Last time the hub answered; the age of the data on the screen (NET-004). `null` until the first answer. */
  readonly lastContactAt: Date | null;
  /**
   * When the oldest copy on the screen was made: set while the hub does not answer and the client shows its
   * copy of an earlier answer (FND-007). Absent when no copy was shown since the last answer.
   */
  readonly copyFrom?: Date;
}

/**
 * What the transport saw.
 * Socket: `acknowledged` (the hub sent `connection_ack`), `refused` (closed 4403: the hub answered and refused
 * the token), `closed` (any other abnormal close, including a missing `connection_ack`).
 * Requests: `answered` (any HTTP answer with a GraphQL body, even `UNAUTHENTICATED`), `failed` (no answer).
 */
export type StatusSignal = 'acknowledged' | 'refused' | 'closed' | 'answered' | 'failed';

export const INITIAL_STATUS: HubStatus = { connection: 'connecting', lastContactAt: null };

/** The status after the client showed its copy made at `at` in place of an answer: the oldest such copy is kept (FND-007). */
export function withCopyShown(state: HubStatus, at: Date): HubStatus {
  return state.copyFrom !== undefined && state.copyFrom <= at ? state : { ...state, copyFrom: at };
}

/** The status after `signal` arrives at `now` (NAV-003, NET-004). */
export function nextStatus(state: HubStatus, signal: StatusSignal, now: Date): HubStatus {
  switch (signal) {
    case 'acknowledged':
    case 'refused':
    case 'answered':
      return { connection: 'connected', lastContactAt: now };
    case 'closed':
      // A drop from a live connection is retried first; a close before any answer means no answer.
      return { ...state, connection: state.connection === 'connected' ? 'connecting' : 'unreachable' };
    case 'failed':
      return { ...state, connection: 'unreachable' };
  }
}
